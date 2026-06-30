use crate::cartridge::CartridgeRegistry;
use crate::config::Config;
use crate::exam_state::ExamState;
use crate::progress::AppProgress;
use crate::speech_caps::SpeechCapability;
use crate::theme::apply_theme;
use crate::tts::TtsController;
use crate::ui::git_update::{check_latest_release, UpdateState};
use eframe::egui;
use egui::{RichText, ScrollArea};
use egui_commonmark::CommonMarkCache;
use std::collections::HashMap;
use std::path::PathBuf;

/// Load fonts provided by the active cartridge as fallbacks in egui.
/// Cartridges ship their own font files in `fonts/` — this avoids requiring
/// system-wide language/font pack installation.
pub fn apply_cartridge_fonts(ctx: &egui::Context, registry: &CartridgeRegistry) {
    let cart = match registry.active() {
        Some(c) => c,
        None => return,
    };

    let font_entries = cart.fonts();
    if font_entries.is_empty() {
        return;
    }

    let mut fonts = egui::FontDefinitions::default();

    for (i, (name, data)) in font_entries.into_iter().enumerate() {
        let key = format!("cart_font_{i}_{name}");
        fonts.font_data.insert(
            key.clone(),
            egui::FontData {
                font: std::borrow::Cow::Owned(data),
                index: 0,
                tweak: Default::default(),
            },
        );
        // Add as fallback for both families
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            family.push(key.clone());
        }
        if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            family.push(key);
        }
    }

    ctx.set_fonts(fonts);
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Study,
    ExamSetup,
    ExamInProgress,
    ExamResults,
    ProgressView,
    Settings,
}

pub struct ExamHelperApp {
    // Core
    pub config: Config,
    pub progress: AppProgress,
    #[allow(dead_code)]
    pub app_dir: PathBuf,

    // Cartridge system
    pub registry: CartridgeRegistry,

    // Study state
    pub selected_file: Option<PathBuf>,
    pub current_content: String,
    pub md_cache: CommonMarkCache,

    // Zoom
    pub native_ppp: f32,
    pub drag_zoom: Option<f32>,

    // Exam
    pub exam_state: Option<ExamState>,
    pub exam_category_selection: Vec<bool>,

    // TTS
    pub tts: TtsController,
    pub narration_rate: f32,
    pub voice_applied: bool,
    /// Language codes from the active cartridge that have no TTS voice installed.
    pub missing_voices: Vec<String>,

    // Settings / Voice management
    pub speech_caps: Option<Vec<SpeechCapability>>,
    pub speech_caps_loading: bool,

    // UI state
    pub mode: AppMode,
    pub sidebar_width: f32,
    pub update_state: UpdateState,
    pub update_error: Option<String>,
    pub update_rx:
        Option<std::sync::mpsc::Receiver<Option<crate::ui::git_update::UpdateAvailable>>>,
    pub theme_applied: bool,
}

impl ExamHelperApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let progress = AppProgress::load();
        let native_ppp = cc.egui_ctx.pixels_per_point();
        apply_theme(&cc.egui_ctx, config.dark_mode);
        cc.egui_ctx.set_pixels_per_point(native_ppp * config.zoom);

        let app_dir = find_app_dir();

        let mut registry = CartridgeRegistry::discover(&app_dir);

        // Restore active cartridge from config
        if !config.active_cartridge.is_empty() {
            registry.set_active_by_id(&config.active_cartridge);
        }

        // Load cartridge fonts as fallback glyphs
        apply_cartridge_fonts(&cc.egui_ctx, &registry);

        // Auto-select TTS voice matching cartridge language
        let tts = TtsController::spawn();
        // Defer voice selection slightly — voices load async in the worker thread
        // We'll apply cartridge voice preference on first frame instead (see update())

        let num_categories = registry
            .active()
            .map(|c| c.exam_categories().len())
            .unwrap_or(0);

        // Spawn background update check
        let (update_tx, update_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = update_tx.send(check_latest_release());
        });

        Self {
            config,
            progress,
            app_dir,
            registry,
            selected_file: None,
            current_content: String::new(),
            md_cache: CommonMarkCache::default(),
            native_ppp,
            drag_zoom: None,
            exam_state: None,
            exam_category_selection: vec![true; num_categories],
            tts,
            voice_applied: false,
            missing_voices: Vec::new(),
            narration_rate: 0.5,
            speech_caps: None,
            speech_caps_loading: false,
            mode: AppMode::Study,
            sidebar_width: 260.0,
            update_state: UpdateState::Checking,
            update_error: None,
            update_rx: Some(update_rx),
            theme_applied: false,
        }
    }

    /// Build and apply the language→voice map for the active cartridge.
    /// Returns a list of language codes that have no matching voice installed.
    pub fn apply_cartridge_voices(&self) -> Vec<String> {
        let cart = match self.registry.active() {
            Some(c) => c,
            None => return Vec::new(),
        };

        let manifest = cart.manifest();
        let lang_configs = manifest.all_languages();
        let voices = self.tts.voices();
        let mut voice_map: HashMap<String, String> = HashMap::new();
        let mut missing: Vec<String> = Vec::new();

        // Set the default language for content without explicit tags
        self.tts.set_default_lang(manifest.default_language());

        for lc in &lang_configs {
            // Try each preference prefix in order
            let best = lc.tts_preference.iter().find_map(|pref| {
                let pref_lower = pref.to_lowercase();
                voices
                    .iter()
                    .find(|v| v.language.to_lowercase().starts_with(&pref_lower))
            });

            // Fallback: try the bare language code
            let best = best.or_else(|| {
                let code_lower = lc.code.to_lowercase();
                voices
                    .iter()
                    .find(|v| v.language.to_lowercase().starts_with(&code_lower))
            });

            if let Some(voice) = best {
                voice_map.insert(lc.code.clone(), voice.name.clone());
            } else {
                missing.push(lc.code.clone());
            }
        }

        // Set the primary voice (first language's voice)
        if let Some(first_lang) = lang_configs.first() {
            if let Some(voice_name) = voice_map.get(&first_lang.code) {
                self.tts.set_voice(voice_name);
            }
        }

        // Send the full map to the worker
        self.tts.set_voice_map(voice_map);

        missing
    }

    pub fn load_file(&mut self, path: &PathBuf) {
        if let Some(cart) = self.registry.active() {
            if let Some(content) = cart.load_content_file(path) {
                self.current_content = content.clone();
                self.selected_file = Some(path.clone());
                self.md_cache = CommonMarkCache::default();

                if self.tts.playback_mode.auto_narrate_on_select() {
                    self.speak_current();
                }
            }
        }
    }

    /// Speak current content using cached audio if possible.
    pub fn speak_current(&mut self) {
        if self.current_content.is_empty() {
            return;
        }
        let (cart_id, file_stem) = self.cache_key();
        match (cart_id, file_stem) {
            (Some(cid), Some(stem)) => {
                self.tts.speak_cached(&self.current_content, &cid, &stem);
            }
            _ => {
                self.tts.speak(&self.current_content);
            }
        }
    }

    /// Bake current content without playing.
    pub fn bake_current(&mut self) {
        if self.current_content.is_empty() {
            return;
        }
        if let (Some(cid), Some(stem)) = self.cache_key() {
            self.tts.bake(&self.current_content, &cid, &stem);
        }
    }

    /// Extract cartridge_id and file stem for cache keys.
    fn cache_key(&self) -> (Option<String>, Option<String>) {
        let cart_id = self.registry.active().map(|c| c.manifest().id.clone());
        let file_stem = self
            .selected_file
            .as_ref()
            .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_string()));
        (cart_id, file_stem)
    }

    /// Collect all file paths from the content tree in display order.
    pub fn ordered_files(&self) -> Vec<PathBuf> {
        let cart = match self.registry.active() {
            Some(c) => c,
            None => return Vec::new(),
        };
        let mut files = Vec::new();
        collect_files(cart.content_tree(), &mut files);
        files
    }

    /// Load the next file after the current one. Returns true if advanced.
    pub fn advance_to_next(&mut self) -> bool {
        let files = self.ordered_files();
        if let Some(ref current) = self.selected_file {
            if let Some(idx) = files.iter().position(|f| f == current) {
                if idx + 1 < files.len() {
                    let next = files[idx + 1].clone();
                    self.load_file(&next);
                    return true;
                }
            }
        }
        false
    }
}

fn collect_files(nodes: &[crate::cartridge::ContentNode], out: &mut Vec<PathBuf>) {
    for node in nodes {
        match &node.kind {
            crate::cartridge::ContentKind::File => out.push(node.path.clone()),
            crate::cartridge::ContentKind::Dir(children) => collect_files(children, out),
        }
    }
}

fn find_app_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Check for cartridges/ or content/ at exe dir
    if exe_dir.join("cartridges").is_dir() || exe_dir.join("content").is_dir() {
        return exe_dir;
    }

    // Walk up from exe dir
    let mut candidate = exe_dir.clone();
    loop {
        if candidate.join("cartridges").is_dir() || candidate.join("content").is_dir() {
            return candidate;
        }
        if !candidate.pop() {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            if cwd.join("cartridges").is_dir() || cwd.join("content").is_dir() {
                return cwd;
            }
            return exe_dir;
        }
    }
}

impl eframe::App for ExamHelperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.theme_applied {
            apply_theme(ctx, self.config.dark_mode);
            self.theme_applied = true;
        }

        // Poll background update check
        if let Some(ref rx) = self.update_rx {
            if let Ok(result) = rx.try_recv() {
                self.update_state = match result {
                    Some(avail) => UpdateState::Available(avail),
                    None => UpdateState::Idle,
                };
                self.update_rx = None;
            }
        }
        if matches!(self.update_state, UpdateState::Checking) {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }

        // Auto-select TTS voices for active cartridge (deferred until voices are loaded)
        if !self.voice_applied {
            let voices = self.tts.voices();
            if !voices.is_empty() {
                self.missing_voices = self.apply_cartridge_voices();
                self.voice_applied = true;
            }
        }

        // Top bar
        egui::TopBottomPanel::top("top_bar")
            .min_height(36.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                self.render_top_bar(ui);
                ui.add_space(4.0);
            });

        // Status bar with draggable zoom
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Version label — clickable to check for updates
                self.render_version_button(ui);

                ui.separator();

                self.render_update_status(ui);

                if let Some(ref p) = self.selected_file {
                    ui.label(
                        RichText::new(p.file_name().unwrap_or_default().to_string_lossy().as_ref())
                            .weak()
                            .size(11.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let display_zoom = self.drag_zoom.unwrap_or(self.config.zoom);
                    let pct = (display_zoom * 100.0).round() as i32;

                    let response = ui
                        .add(
                            egui::Label::new(
                                RichText::new(format!(" {pct}% ")).monospace().size(11.0),
                            )
                            .sense(egui::Sense::drag()),
                        )
                        .on_hover_text("Arrastra para zoom");

                    if response.hovered() || response.dragged() {
                        ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }

                    if response.dragged() {
                        let z = self.drag_zoom.get_or_insert(self.config.zoom);
                        *z = (*z + response.drag_delta().x * 0.003).clamp(0.25, 4.0);
                    }

                    if response.drag_stopped() {
                        if let Some(z) = self.drag_zoom.take() {
                            self.config.zoom = z;
                            ctx.set_pixels_per_point(self.native_ppp * z);
                            self.config.save();
                        }
                    }

                    ui.separator();
                    ui.label(RichText::new("zoom").weak().size(11.0));
                });
            });
        });

        // (update status is rendered inline in the status bar)

        match self.mode.clone() {
            AppMode::Study => {
                egui::SidePanel::left("study_sidebar")
                    .min_width(200.0)
                    .default_width(self.sidebar_width)
                    .resizable(true)
                    .show(ctx, |ui| {
                        let file_to_load = self.render_study_sidebar(ui);
                        if let Some(path) = file_to_load {
                            self.load_file(&path);
                        }
                    });

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_study_content(ui);
                });
            }
            AppMode::ExamSetup => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ScrollArea::vertical()
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            self.render_exam_setup(ui);
                        });
                });
            }
            AppMode::ExamInProgress => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_exam_in_progress(ui);
                });
            }
            AppMode::ExamResults => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_exam_results(ui);
                });
            }
            AppMode::ProgressView => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_progress_view(ui);
                });
            }
            AppMode::Settings => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_settings(ui);
                });
            }
        }
    }
}

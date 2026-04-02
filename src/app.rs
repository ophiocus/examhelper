use crate::cartridge::CartridgeRegistry;
use crate::config::Config;
use crate::exam_state::ExamState;
use crate::progress::AppProgress;
use crate::speech_caps::SpeechCapability;
use crate::theme::apply_theme;
use crate::tts::TtsController;
use crate::ui::git_update::GitStatus;
use eframe::egui;
use egui::{RichText, ScrollArea};
use egui_commonmark::CommonMarkCache;
use std::path::PathBuf;

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

    // Settings / Voice management
    pub speech_caps: Option<Vec<SpeechCapability>>,
    pub speech_caps_loading: bool,

    // UI state
    pub mode: AppMode,
    pub sidebar_width: f32,
    pub git_status: GitStatus,
    pub show_git_status: bool,
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

        let num_categories = registry
            .active()
            .map(|c| c.exam_categories().len())
            .unwrap_or(0);

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
            tts: TtsController::spawn(),
            narration_rate: 0.5,
            speech_caps: None,
            speech_caps_loading: false,
            mode: AppMode::Study,
            sidebar_width: 260.0,
            git_status: GitStatus::Idle,
            show_git_status: false,
            theme_applied: false,
        }
    }

    pub fn load_file(&mut self, path: &PathBuf) {
        if let Some(cart) = self.registry.active() {
            if let Some(content) = cart.load_content_file(path) {
                self.current_content = content.clone();
                self.selected_file = Some(path.clone());
                self.md_cache = CommonMarkCache::default();

                if self.tts.autoplay {
                    self.tts.speak(&content);
                }
            }
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
                if let Some(ref p) = self.selected_file {
                    ui.label(
                        RichText::new(
                            p.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .as_ref(),
                        )
                        .weak()
                        .size(11.0),
                    );
                }

                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let display_zoom =
                            self.drag_zoom.unwrap_or(self.config.zoom);
                        let pct = (display_zoom * 100.0).round() as i32;

                        let response = ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!(" {pct}% "))
                                        .monospace()
                                        .size(11.0),
                                )
                                .sense(egui::Sense::drag()),
                            )
                            .on_hover_text("Arrastra para zoom");

                        if response.hovered() || response.dragged() {
                            ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }

                        if response.dragged() {
                            let z = self.drag_zoom.get_or_insert(self.config.zoom);
                            *z = (*z + response.drag_delta().x * 0.003)
                                .clamp(0.25, 4.0);
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
                    },
                );
            });
        });

        // Git status window
        self.render_git_status_window(ctx);

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

use crate::app::ExamHelperApp;
use crate::cartridge::{ContentKind, ContentNode};
use crate::progress::AppProgress;
use crate::tts::{strip_lang_tags, NarrationState};
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use egui_commonmark::CommonMarkViewer;
use std::path::PathBuf;

impl ExamHelperApp {
    pub fn render_study_sidebar(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut file_to_load: Option<PathBuf> = None;

        let cart_name = self
            .registry
            .active()
            .map(|c| c.manifest().name.clone())
            .unwrap_or_else(|| "Contenido de Estudio".to_string());

        let accent = self
            .registry
            .active()
            .map(|c| {
                let [r, g, b] = c.manifest().accent_color;
                Color32::from_rgb(r, g, b)
            })
            .unwrap_or(Color32::from_rgb(255, 205, 0));

        ui.label(
            RichText::new(cart_name)
                .size(15.0)
                .strong()
                .color(accent),
        );
        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if let Some(cart) = self.registry.active() {
                    let tree: Vec<ContentNode> = cart.content_tree().to_vec();
                    let cart_id = cart.manifest().id.clone();
                    for node in &tree {
                        Self::render_content_node(
                            ui,
                            node,
                            &self.selected_file,
                            &self.progress,
                            &cart_id,
                            &mut file_to_load,
                        );
                    }
                }
            });

        file_to_load
    }

    fn render_content_node(
        ui: &mut egui::Ui,
        node: &ContentNode,
        selected: &Option<PathBuf>,
        progress: &AppProgress,
        cartridge_id: &str,
        file_to_load: &mut Option<PathBuf>,
    ) {
        match &node.kind {
            ContentKind::Dir(children) => {
                egui::CollapsingHeader::new(
                    RichText::new(&node.display_name)
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(200, 180, 120)),
                )
                .default_open(true)
                .show(ui, |ui| {
                    for child in children {
                        Self::render_content_node(
                            ui,
                            child,
                            selected,
                            progress,
                            cartridge_id,
                            file_to_load,
                        );
                    }
                });
            }
            ContentKind::File => {
                let is_selected = selected.as_ref() == Some(&node.path);
                let is_read =
                    progress.is_read(cartridge_id, &node.path.to_string_lossy());

                let prefix = if is_read { "[OK] " } else { "" };
                let label_text = format!("{}{}", prefix, node.display_name);

                let color = if is_selected {
                    Color32::from_rgb(80, 170, 255)
                } else if is_read {
                    Color32::from_rgb(80, 200, 120)
                } else {
                    Color32::from_rgb(200, 200, 200)
                };

                let label = RichText::new(label_text).color(color);

                if ui.selectable_label(is_selected, label).clicked() {
                    *file_to_load = Some(node.path.clone());
                }
            }
        }
    }

    pub fn render_study_content(&mut self, ui: &mut egui::Ui) {
        if self.current_content.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(
                        "Selecciona un tema del panel izquierdo para comenzar a estudiar.",
                    )
                    .size(16.0)
                    .color(Color32::from_rgb(150, 150, 170)),
                );
            });
            return;
        }

        // ── Narration control bar ────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let tts_state = self.tts.state();

            if tts_state == NarrationState::Speaking {
                if ui
                    .button(
                        RichText::new("Detener")
                            .size(13.0)
                            .color(Color32::from_rgb(255, 100, 100)),
                    )
                    .clicked()
                {
                    self.tts.stop();
                }

                let (current, total) = self.tts.progress();
                if total > 0 {
                    let pct = (current + 1) as f32 / total as f32;
                    ui.add(
                        egui::ProgressBar::new(pct)
                            .desired_width(120.0)
                            .show_percentage(),
                    );
                }
                ui.ctx().request_repaint();
            } else {
                if ui
                    .button(
                        RichText::new("Narrar")
                            .size(13.0)
                            .color(Color32::from_rgb(80, 200, 120)),
                    )
                    .clicked()
                {
                    self.tts.speak(&self.current_content);
                }
            }

            ui.separator();

            // Rate slider
            ui.label(RichText::new("Velocidad:").size(11.0).weak());
            let old_rate = self.narration_rate;
            ui.add(
                egui::Slider::new(&mut self.narration_rate, 0.0..=1.0).show_value(false),
            );
            if (self.narration_rate - old_rate).abs() > 0.001 {
                self.tts.set_rate(self.narration_rate);
            }

            let speed_x = crate::tts::slider_to_rate(self.narration_rate);
            let speed_word = if speed_x < 0.6 {
                "Lento"
            } else if speed_x < 1.1 {
                "Normal"
            } else {
                "Rápido"
            };
            ui.label(
                RichText::new(format!("{:.1}× {}", speed_x, speed_word))
                    .size(11.0)
                    .weak(),
            );

            ui.separator();

            // Autoplay toggle
            let autoplay_color = if self.tts.autoplay {
                Color32::from_rgb(255, 205, 0)
            } else {
                Color32::from_rgb(150, 150, 150)
            };
            if ui
                .button(
                    RichText::new(if self.tts.autoplay {
                        "Auto: ON"
                    } else {
                        "Auto: OFF"
                    })
                    .size(11.0)
                    .color(autoplay_color),
                )
                .on_hover_text("Narrar automáticamente al seleccionar un tema")
                .clicked()
            {
                self.tts.autoplay = !self.tts.autoplay;
            }

            // Missing voice warning
            if !self.missing_voices.is_empty() {
                ui.separator();
                let missing_str = self.missing_voices.join(", ");
                ui.label(
                    RichText::new(format!("Missing TTS: {missing_str}"))
                        .size(11.0)
                        .color(Color32::from_rgb(255, 100, 100)),
                )
                .on_hover_text(format!(
                    "No TTS voice installed for: {}. Install voice packs in Settings.",
                    missing_str
                ));
            }

            ui.separator();

            // Voice selector dropdown
            let voices = self.tts.voices();
            if !voices.is_empty() {
                let current_voice = self.tts.selected_voice_name();
                let display = voices
                    .iter()
                    .find(|v| v.name == current_voice)
                    .map(|v| {
                        format!(
                            "{} ({})",
                            v.name.replace("Microsoft ", ""),
                            v.language
                        )
                    })
                    .unwrap_or_else(|| current_voice.clone());

                egui::ComboBox::from_id_source("voice_selector")
                    .selected_text(RichText::new(&display).size(11.0))
                    .width(140.0)
                    .show_ui(ui, |ui: &mut egui::Ui| {
                        for voice in &voices {
                            let label = format!(
                                "{} ({})",
                                voice.name.replace("Microsoft ", ""),
                                voice.language
                            );
                            let is_selected = voice.name == current_voice;
                            if ui.selectable_label(is_selected, &label).clicked() {
                                self.tts.set_voice(&voice.name);
                            }
                        }
                    });
            }
        });

        ui.separator();

        // ── Content ──────────────────────────────────────────────────────
        let tts_state = self.tts.state();
        let is_narrating = tts_state == NarrationState::Speaking;

        if is_narrating {
            // Karaoke mode — shows language-annotated chunks
            let chunks = self.tts.narration_chunks();
            let (current_idx, _total) = self.tts.progress();

            ScrollArea::vertical()
                .id_source("karaoke_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    let mut prev_lang = String::new();
                    for (i, chunk) in chunks.iter().enumerate() {
                        let is_current = i == current_idx;
                        let is_past = i < current_idx;

                        // Show language badge when language switches
                        if chunk.lang != prev_lang {
                            ui.label(
                                RichText::new(format!("[{}]", chunk.lang.to_uppercase()))
                                    .size(10.0)
                                    .color(Color32::from_rgb(120, 120, 180)),
                            );
                            prev_lang = chunk.lang.clone();
                        }

                        if is_current {
                            let frame = egui::Frame::none()
                                .fill(Color32::from_rgb(60, 50, 10))
                                .stroke(egui::Stroke::new(
                                    2.0,
                                    Color32::from_rgb(255, 205, 0),
                                ))
                                .inner_margin(egui::Margin::same(10.0))
                                .rounding(egui::Rounding::same(6.0));
                            let r = frame.show(ui, |ui| {
                                ui.label(
                                    RichText::new(&chunk.text)
                                        .size(15.0)
                                        .color(Color32::from_rgb(255, 235, 120))
                                        .strong(),
                                );
                            });
                            r.response.scroll_to_me(Some(egui::Align::Center));
                        } else if is_past {
                            ui.label(
                                RichText::new(&chunk.text)
                                    .size(14.0)
                                    .color(Color32::from_rgb(100, 100, 115)),
                            );
                        } else {
                            ui.label(
                                RichText::new(&chunk.text)
                                    .size(14.0)
                                    .color(Color32::from_rgb(190, 190, 200)),
                            );
                        }
                        ui.add_space(6.0);
                    }
                    ui.add_space(20.0);
                });
        } else {
            // Normal markdown view with multimedia support
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Strip language tags before display, then split on multimedia tags
                    let cleaned = strip_lang_tags(&self.current_content);
                    let segments = split_multimedia(&cleaned);
                    for segment in &segments {
                        match segment {
                            ContentSegment::Markdown(md) => {
                                CommonMarkViewer::new("study_content").show(
                                    ui,
                                    &mut self.md_cache,
                                    md,
                                );
                            }
                            ContentSegment::Video { url, title } => {
                                render_video_embed(ui, url, title);
                            }
                            ContentSegment::Image { url, caption } => {
                                render_image_embed(ui, url, caption);
                            }
                        }
                    }

                    ui.add_space(20.0);

                    if let Some(ref path) = self.selected_file {
                        let cart_id = self.registry.active_id().to_string();
                        let path_str = path.to_string_lossy().to_string();
                        if !self.progress.is_read(&cart_id, &path_str) {
                            ui.separator();
                            if ui
                                .button(
                                    RichText::new("Marcar como leido")
                                        .size(14.0)
                                        .color(Color32::from_rgb(80, 200, 120)),
                                )
                                .clicked()
                            {
                                self.progress.mark_read(&cart_id, &path_str);
                            }
                        } else {
                            ui.separator();
                            ui.label(
                                RichText::new("Tema completado")
                                    .size(12.0)
                                    .color(Color32::from_rgb(80, 200, 120)),
                            );
                        }
                    }
                });
        }
    }
}

// ── Multimedia content support ─────────────────────────────────────────────
//
// Content files can embed multimedia references using these tags:
//   {{video:URL:Title}}    — renders as a clickable video link
//   {{image:URL:Caption}}  — renders as an image placeholder
//
// These are stripped from TTS output (only the title/caption is read).

enum ContentSegment {
    Markdown(String),
    Video { url: String, title: String },
    Image { url: String, caption: String },
}

fn split_multimedia(content: &str) -> Vec<ContentSegment> {
    let mut segments = Vec::new();
    let mut current_md = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(media) = parse_media_tag(trimmed) {
            if !current_md.trim().is_empty() {
                segments.push(ContentSegment::Markdown(current_md.clone()));
                current_md.clear();
            }
            segments.push(media);
        } else {
            current_md.push_str(line);
            current_md.push('\n');
        }
    }

    if !current_md.trim().is_empty() {
        segments.push(ContentSegment::Markdown(current_md));
    }

    // If no multimedia tags found, return the whole content as one segment
    if segments.is_empty() {
        segments.push(ContentSegment::Markdown(content.to_string()));
    }

    segments
}

fn parse_media_tag(line: &str) -> Option<ContentSegment> {
    if line.starts_with("{{video:") && line.ends_with("}}") {
        let inner = &line[8..line.len() - 2];
        let (url, title) = split_media_fields(inner);
        Some(ContentSegment::Video { url, title })
    } else if line.starts_with("{{image:") && line.ends_with("}}") {
        let inner = &line[8..line.len() - 2];
        let (url, caption) = split_media_fields(inner);
        Some(ContentSegment::Image { url, caption })
    } else {
        None
    }
}

fn split_media_fields(inner: &str) -> (String, String) {
    // Split on the last colon to get URL:title
    if let Some(pos) = inner.rfind(':') {
        let url = inner[..pos].trim().to_string();
        let title = inner[pos + 1..].trim().to_string();
        (url, title)
    } else {
        (inner.trim().to_string(), String::new())
    }
}

fn render_video_embed(ui: &mut egui::Ui, url: &str, title: &str) {
    ui.add_space(8.0);
    let frame = egui::Frame::none()
        .fill(Color32::from_rgb(30, 30, 45))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(80, 80, 120)))
        .inner_margin(egui::Margin::same(12.0))
        .rounding(egui::Rounding::same(6.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("▶")
                    .size(20.0)
                    .color(Color32::from_rgb(255, 80, 80)),
            );
            ui.vertical(|ui| {
                let display_title = if title.is_empty() { "Video" } else { title };
                if ui
                    .link(RichText::new(display_title).size(14.0).strong())
                    .clicked()
                {
                    let _ = open::that(url);
                }
                ui.label(
                    RichText::new(url)
                        .size(10.0)
                        .color(Color32::from_rgb(120, 120, 150)),
                );
            });
        });
    });
    ui.add_space(8.0);
}

fn render_image_embed(ui: &mut egui::Ui, url: &str, caption: &str) {
    ui.add_space(8.0);
    let frame = egui::Frame::none()
        .fill(Color32::from_rgb(30, 35, 30))
        .stroke(egui::Stroke::new(1.0, Color32::from_rgb(80, 120, 80)))
        .inner_margin(egui::Margin::same(12.0))
        .rounding(egui::Rounding::same(6.0));
    frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🖼")
                    .size(20.0)
                    .color(Color32::from_rgb(80, 200, 120)),
            );
            ui.vertical(|ui| {
                let display_caption = if caption.is_empty() { "Image" } else { caption };
                if ui
                    .link(RichText::new(display_caption).size(14.0).strong())
                    .clicked()
                {
                    let _ = open::that(url);
                }
                ui.label(
                    RichText::new(url)
                        .size(10.0)
                        .color(Color32::from_rgb(120, 150, 120)),
                );
            });
        });
    });
    ui.add_space(8.0);
}

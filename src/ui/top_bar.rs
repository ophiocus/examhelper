use crate::app::{AppMode, ExamHelperApp};
use crate::theme::apply_theme;
use eframe::egui;
use egui::{Color32, RichText};

impl ExamHelperApp {
    pub fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // App title — use cartridge name if available
            let title = if let Some(cart) = self.registry.active() {
                let m = cart.manifest();
                let [r, g, b] = m.accent_color;
                let color = Color32::from_rgb(r, g, b);
                (m.name.clone(), color)
            } else {
                ("ExamHelper".to_string(), Color32::from_rgb(255, 205, 0))
            };

            ui.label(RichText::new(&title.0).size(18.0).strong().color(title.1));

            // Cartridge selector (only if multiple cartridges)
            if self.registry.count() > 1 {
                ui.separator();
                let current_id = self.registry.active_id().to_string();
                let current_name = self
                    .registry
                    .active()
                    .map(|c| c.manifest().name.clone())
                    .unwrap_or_default();

                // Collect cartridge info to avoid borrow conflict
                let cart_info: Vec<(usize, String, String)> = self
                    .registry
                    .list()
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| {
                        let m = c.manifest();
                        (idx, m.id.clone(), m.name.clone())
                    })
                    .collect();

                let mut switch_to: Option<(usize, String)> = None;

                egui::ComboBox::from_id_source("cartridge_selector")
                    .selected_text(RichText::new(&current_name).size(13.0))
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (idx, id, name) in &cart_info {
                            if ui.selectable_label(*id == current_id, name).clicked() {
                                switch_to = Some((*idx, id.clone()));
                            }
                        }
                    });

                if let Some((idx, id)) = switch_to {
                    self.registry.set_active(idx);
                    self.config.active_cartridge = id;
                    self.config.save();
                    self.tts.stop();
                    self.current_content.clear();
                    self.selected_file = None;
                    self.exam_state = None;
                    let num_cats = self
                        .registry
                        .active()
                        .map(|c| c.exam_categories().len())
                        .unwrap_or(0);
                    self.exam_category_selection = vec![true; num_cats];
                    self.mode = AppMode::Study;

                    // Apply cartridge fonts and build TTS voice map
                    crate::app::apply_cartridge_fonts(ui.ctx(), &self.registry);
                    self.missing_voices = self.apply_cartridge_voices();
                }
            }

            ui.separator();

            // Mode buttons
            let study_color = if self.mode == AppMode::Study {
                Color32::from_rgb(80, 200, 120)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui
                .button(RichText::new("Estudiar").color(study_color).size(14.0))
                .clicked()
            {
                self.mode = AppMode::Study;
            }

            let exam_color = if matches!(
                self.mode,
                AppMode::ExamSetup | AppMode::ExamInProgress | AppMode::ExamResults
            ) {
                Color32::from_rgb(100, 149, 237)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui
                .button(RichText::new("Examen").color(exam_color).size(14.0))
                .clicked()
            {
                self.tts.stop();
                self.mode = AppMode::ExamSetup;
            }

            let progress_color = if self.mode == AppMode::ProgressView {
                Color32::from_rgb(255, 165, 0)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui
                .button(RichText::new("Progreso").color(progress_color).size(14.0))
                .clicked()
            {
                self.mode = AppMode::ProgressView;
            }

            let settings_color = if self.mode == AppMode::Settings {
                Color32::from_rgb(200, 150, 255)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui
                .button(RichText::new("Config").color(settings_color).size(14.0))
                .clicked()
            {
                self.tts.stop();
                self.mode = AppMode::Settings;
                if self.speech_caps.is_none() && !self.speech_caps_loading {
                    self.speech_caps_loading = true;
                }
            }

            // Theme toggle + zoom (right-aligned)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_label = if self.config.dark_mode {
                    "Modo Claro"
                } else {
                    "Modo Oscuro"
                };
                if ui.button(RichText::new(theme_label).size(12.0)).clicked() {
                    self.config.dark_mode = !self.config.dark_mode;
                    apply_theme(ui.ctx(), self.config.dark_mode);
                    self.config.save();
                }

                ui.separator();

                ui.menu_button(RichText::new("Zoom").size(12.0), |ui| {
                    for &(label, factor) in &[
                        ("75%", 0.75_f32),
                        ("100%", 1.00_f32),
                        ("125%", 1.25_f32),
                        ("150%", 1.50_f32),
                        ("200%", 2.00_f32),
                    ] {
                        if ui.button(label).clicked() {
                            self.config.zoom = factor;
                            ui.ctx().set_pixels_per_point(self.native_ppp * factor);
                            self.config.save();
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui.button("Reset zoom").clicked() {
                        self.config.zoom = 1.0;
                        ui.ctx().set_pixels_per_point(self.native_ppp);
                        self.config.save();
                        ui.close_menu();
                    }
                });
            });
        });
    }
}

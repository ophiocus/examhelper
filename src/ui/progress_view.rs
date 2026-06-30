use crate::app::ExamHelperApp;
use crate::cartridge::count_files;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

impl ExamHelperApp {
    pub fn render_progress_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.label(
            RichText::new("Tu Progreso de Estudio")
                .size(22.0)
                .strong()
                .color(Color32::from_rgb(255, 165, 0)),
        );
        ui.add_space(15.0);
        ui.separator();

        let cart_id = self.registry.active_id().to_string();

        // Reading progress
        ui.add_space(10.0);
        ui.label(RichText::new("Lectura:").size(16.0).strong());
        ui.add_space(5.0);

        let total_files = self
            .registry
            .active()
            .map(|c| count_files(c.content_tree()))
            .unwrap_or(0);
        let read_count = self.progress.for_cartridge(&cart_id).read_files.len();

        ui.label(RichText::new(format!("Temas leidos: {}/{}", read_count, total_files)).size(14.0));

        if total_files > 0 {
            let progress = read_count as f32 / total_files as f32;
            ui.add(egui::ProgressBar::new(progress).show_percentage());
        }

        ui.add_space(15.0);
        ui.separator();

        // Exam history
        ui.add_space(10.0);
        ui.label(RichText::new("Historial de Examenes:").size(16.0).strong());
        ui.add_space(5.0);

        let cp = self.progress.for_cartridge(&cart_id);

        if cp.exam_history.is_empty() {
            ui.label(
                RichText::new("No has realizado examenes aun.")
                    .size(14.0)
                    .color(Color32::from_rgb(150, 150, 170)),
            );
        } else {
            // Clone history to avoid borrow issues
            let history = cp.exam_history.clone();

            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    for record in history.iter().rev().take(20) {
                        let pct = if record.total > 0 {
                            (record.score as f64 / record.total as f64 * 100.0) as u32
                        } else {
                            0
                        };
                        let threshold = self
                            .registry
                            .active()
                            .map(|c| c.pass_threshold(&record.category))
                            .unwrap_or(50);
                        let color = if pct >= threshold {
                            Color32::from_rgb(80, 200, 120)
                        } else if pct >= threshold.saturating_sub(15) {
                            Color32::from_rgb(255, 165, 0)
                        } else {
                            Color32::from_rgb(255, 80, 80)
                        };

                        ui.horizontal(|ui| {
                            let status = if pct >= threshold { "✓" } else { "✗" };
                            ui.label(
                                RichText::new(format!("{} {}: ", status, record.category))
                                    .size(13.0),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{}/{} ({}%)",
                                    record.score, record.total, pct
                                ))
                                .size(13.0)
                                .color(color),
                            );
                        });
                    }

                    ui.add_space(15.0);

                    if ui
                        .button(
                            RichText::new("Limpiar Historial")
                                .size(12.0)
                                .color(Color32::from_rgb(255, 100, 100)),
                        )
                        .clicked()
                    {
                        self.progress
                            .for_cartridge_mut(&cart_id)
                            .exam_history
                            .clear();
                        self.progress.save();
                    }

                    if ui
                        .button(
                            RichText::new("Reiniciar Lecturas")
                                .size(12.0)
                                .color(Color32::from_rgb(255, 100, 100)),
                        )
                        .clicked()
                    {
                        self.progress.for_cartridge_mut(&cart_id).read_files.clear();
                        self.progress.save();
                    }
                });
        }
    }
}

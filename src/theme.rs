use eframe::egui;
use egui::Color32;

pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    if dark {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(20, 22, 28);
        v.window_fill = Color32::from_rgb(28, 30, 38);
        v.window_rounding = egui::Rounding::same(6.0);
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from_rgb(50, 55, 70));
        ctx.set_visuals(v);
    } else {
        let mut v = egui::Visuals::light();
        v.window_rounding = egui::Rounding::same(6.0);
        ctx.set_visuals(v);
    }
}

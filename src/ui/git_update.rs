use crate::app::ExamHelperApp;
use eframe::egui;
use egui::{Color32, Vec2};
use std::path::Path;
use std::process::Command;

#[derive(Debug, PartialEq)]
pub enum GitStatus {
    Idle,
    Updating,
    Success(String),
    Error(String),
}

pub fn git_pull(repo_dir: &Path) -> Result<String, String> {
    if !repo_dir.join(".git").exists() {
        return Err(
            "No es un repositorio git. Inicializa con 'git init' primero.".to_string(),
        );
    }

    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(repo_dir)
        .output()
        .map_err(|e| format!("Error ejecutando git: {}", e))?;

    if output.status.success() {
        let msg = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(if msg.trim() == "Already up to date." {
            "Contenido actualizado. No hay cambios nuevos.".to_string()
        } else {
            format!("Contenido actualizado exitosamente.\n{}", msg.trim())
        })
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Error en git pull: {}", err.trim()))
    }
}

impl ExamHelperApp {
    pub fn render_git_status_window(&mut self, ctx: &egui::Context) {
        if !self.show_git_status {
            return;
        }

        egui::Window::new("Estado de Actualización")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                match &self.git_status {
                    GitStatus::Updating => {
                        ui.label("Actualizando contenido...");
                        ui.spinner();
                    }
                    GitStatus::Success(msg) => {
                        ui.colored_label(Color32::from_rgb(80, 200, 120), msg);
                    }
                    GitStatus::Error(msg) => {
                        ui.colored_label(Color32::from_rgb(255, 80, 80), msg);
                    }
                    GitStatus::Idle => {}
                }
                if ui.button("Cerrar").clicked() {
                    self.show_git_status = false;
                    self.git_status = GitStatus::Idle;
                }
            });
    }
}

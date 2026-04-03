use crate::app::ExamHelperApp;
use crate::speech_caps::{install_speech_capability, query_speech_capabilities};
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

impl ExamHelperApp {
    pub fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.label(
            RichText::new("Configuración")
                .size(22.0)
                .strong()
                .color(Color32::from_rgb(200, 150, 255)),
        );
        ui.add_space(15.0);
        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // ── Voice Configuration ──
                ui.add_space(10.0);
                ui.label(RichText::new("Voces de Narración").size(18.0).strong());
                ui.add_space(8.0);

                let voices = self.tts.voices();
                let current = self.tts.selected_voice_name();
                let cart_langs: Vec<String> = self
                    .registry
                    .active()
                    .map(|c| {
                        c.manifest()
                            .all_languages()
                            .iter()
                            .map(|l| l.code.clone())
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["es".to_string()]);

                // Show per-language voice status
                if !cart_langs.is_empty() {
                    ui.label(RichText::new("Cartridge Languages:").size(14.0).strong());
                    ui.add_space(4.0);
                    for lang_code in &cart_langs {
                        let has_voice = voices.iter().any(|v| {
                            v.language.to_lowercase().starts_with(&lang_code.to_lowercase())
                        });
                        let (status, color) = if has_voice {
                            ("OK", Color32::from_rgb(80, 200, 120))
                        } else {
                            ("MISSING", Color32::from_rgb(255, 100, 100))
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("  [{lang_code}]"))
                                    .size(13.0)
                                    .monospace(),
                            );
                            ui.label(RichText::new(status).size(12.0).color(color));
                        });
                    }
                    ui.add_space(8.0);
                }

                ui.label(RichText::new("Voces instaladas:").size(14.0).strong());
                ui.add_space(5.0);

                if voices.is_empty() {
                    ui.label(
                        RichText::new("Cargando voces...")
                            .size(13.0)
                            .color(Color32::from_rgb(150, 150, 170)),
                    );
                } else {
                    egui::Grid::new("voices_grid")
                        .striped(true)
                        .min_col_width(120.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Nombre").size(12.0).strong());
                            ui.label(RichText::new("Idioma").size(12.0).strong());
                            ui.label(RichText::new("Estado").size(12.0).strong());
                            ui.end_row();

                            for voice in &voices {
                                let is_current = voice.name == current;
                                let name_color = if is_current {
                                    Color32::from_rgb(80, 200, 120)
                                } else {
                                    Color32::from_rgb(200, 200, 200)
                                };

                                let is_cart_lang = cart_langs.iter().any(|cl| {
                                    voice.language.to_lowercase().starts_with(&cl.to_lowercase())
                                });
                                let lang_color = if is_cart_lang {
                                    Color32::from_rgb(255, 205, 0)
                                } else {
                                    Color32::from_rgb(150, 150, 170)
                                };

                                ui.label(
                                    RichText::new(&voice.name).size(12.0).color(name_color),
                                );
                                ui.label(
                                    RichText::new(&voice.language)
                                        .size(12.0)
                                        .color(lang_color),
                                );

                                if is_current {
                                    ui.label(
                                        RichText::new("Activa")
                                            .size(12.0)
                                            .color(Color32::from_rgb(80, 200, 120)),
                                    );
                                } else {
                                    if ui
                                        .button(RichText::new("Seleccionar").size(11.0))
                                        .clicked()
                                    {
                                        self.tts.set_voice(&voice.name);
                                    }
                                }
                                ui.end_row();
                            }
                        });

                    if !self.missing_voices.is_empty() {
                        ui.add_space(8.0);
                        let cart_name = self
                            .registry
                            .active()
                            .map(|c| c.manifest().name.clone())
                            .unwrap_or_default();
                        let missing_str = self.missing_voices.join(", ");
                        ui.label(
                            RichText::new(format!(
                                "Missing voices for [{missing_str}] (required by {cart_name}). Install below or via Windows Settings."
                            ))
                            .size(13.0)
                            .color(Color32::from_rgb(255, 100, 100)),
                        );
                    }
                }

                ui.add_space(20.0);
                ui.separator();
                ui.add_space(10.0);

                // ── Speech Pack Installation ──
                ui.label(
                    RichText::new("Paquetes de Voz del Sistema")
                        .size(18.0)
                        .strong(),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new(
                        "Instalar nuevos paquetes de voz para Windows (requiere permisos de administrador).",
                    )
                    .size(12.0)
                    .weak(),
                );
                ui.add_space(8.0);

                if self.speech_caps_loading && self.speech_caps.is_none() {
                    ui.label("Consultando paquetes disponibles...");
                    ui.spinner();
                    let caps = query_speech_capabilities();
                    if caps.is_empty() {
                        self.speech_caps = Some(Vec::new());
                    } else {
                        self.speech_caps = Some(caps);
                    }
                    self.speech_caps_loading = false;
                }

                if let Some(ref caps) = self.speech_caps {
                    if caps.is_empty() {
                        ui.label(
                            RichText::new(
                                "No se pudo consultar los paquetes. Ejecuta la app como administrador o instala manualmente:",
                            )
                            .size(12.0)
                            .color(Color32::from_rgb(255, 165, 0)),
                        );
                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(
                                "Configuración → Hora e idioma → Voz → Agregar voces",
                            )
                            .size(12.0),
                        );
                    } else {
                        egui::Grid::new("speech_caps_grid")
                            .striped(true)
                            .min_col_width(200.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Paquete").size(12.0).strong(),
                                );
                                ui.label(
                                    RichText::new("Estado").size(12.0).strong(),
                                );
                                ui.label(
                                    RichText::new("Acción").size(12.0).strong(),
                                );
                                ui.end_row();

                                let mut sorted_caps = caps.clone();
                                sorted_caps.sort_by(|a, b| {
                                    let a_es = a.name.contains("es-");
                                    let b_es = b.name.contains("es-");
                                    b_es.cmp(&a_es)
                                        .then(b.installed.cmp(&a.installed))
                                        .then(a.name.cmp(&b.name))
                                });

                                for cap in &sorted_caps {
                                    let lang_code = cap
                                        .name
                                        .split("~~~")
                                        .nth(1)
                                        .and_then(|s| s.split('~').next())
                                        .unwrap_or(&cap.name);

                                    let is_spanish = lang_code.starts_with("es");
                                    let name_color = if is_spanish {
                                        Color32::from_rgb(255, 205, 0)
                                    } else {
                                        Color32::from_rgb(180, 180, 180)
                                    };

                                    ui.label(
                                        RichText::new(lang_code)
                                            .size(12.0)
                                            .color(name_color),
                                    );

                                    if cap.installed {
                                        ui.label(
                                            RichText::new("Instalado")
                                                .size(12.0)
                                                .color(Color32::from_rgb(80, 200, 120)),
                                        );
                                        ui.label(
                                            RichText::new("—").size(12.0).weak(),
                                        );
                                    } else {
                                        ui.label(
                                            RichText::new("No instalado")
                                                .size(12.0)
                                                .color(Color32::from_rgb(150, 150, 170)),
                                        );
                                        if ui
                                            .button(
                                                RichText::new("Instalar")
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(80, 200, 120)),
                                            )
                                            .clicked()
                                        {
                                            install_speech_capability(&cap.name);
                                        }
                                    }
                                    ui.end_row();
                                }
                            });

                        ui.add_space(10.0);
                        if ui
                            .button(RichText::new("Refrescar lista").size(12.0))
                            .clicked()
                        {
                            self.speech_caps = None;
                            self.speech_caps_loading = true;
                        }

                        ui.add_space(5.0);
                        ui.label(
                            RichText::new(
                                "Después de instalar un paquete, reinicia la aplicación para que aparezca la nueva voz.",
                            )
                            .size(11.0)
                            .weak(),
                        );
                    }
                } else {
                    if ui
                        .button(
                            RichText::new("Escanear paquetes de voz disponibles")
                                .size(13.0)
                                .color(Color32::from_rgb(100, 149, 237)),
                        )
                        .clicked()
                    {
                        self.speech_caps_loading = true;
                    }
                }
            });
    }
}

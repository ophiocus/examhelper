use crate::app::{AppMode, ExamHelperApp};
use crate::exam_state::ExamState;
use eframe::egui;
use egui::{Color32, RichText, ScrollArea};

impl ExamHelperApp {
    pub fn render_exam_setup(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.label(
            RichText::new("Configurar Examen de Practica")
                .size(22.0)
                .strong()
                .color(Color32::from_rgb(100, 149, 237)),
        );
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(15.0);

        let categories = self
            .registry
            .active()
            .map(|c| c.exam_categories())
            .unwrap_or_default();

        ui.label(
            RichText::new("Selecciona las categorias a evaluar:")
                .size(15.0)
                .strong(),
        );
        ui.add_space(8.0);

        if self.exam_category_selection.len() != categories.len() {
            self.exam_category_selection = vec![true; categories.len()];
        }

        for (idx, (name, total)) in categories.iter().enumerate() {
            ui.checkbox(
                &mut self.exam_category_selection[idx],
                RichText::new(format!("{} ({} preguntas disponibles)", name, total))
                    .size(14.0),
            );
        }

        ui.add_space(15.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Preguntas por categoria: ").size(14.0));
            ui.add(egui::Slider::new(
                &mut self.config.questions_per_category,
                5..=30,
            ));
        });

        ui.add_space(20.0);

        let any_selected = self.exam_category_selection.iter().any(|&s| s);

        if any_selected {
            let btn = ui.button(
                RichText::new("Iniciar Examen")
                    .size(16.0)
                    .strong()
                    .color(Color32::from_rgb(80, 200, 120)),
            );

            if btn.clicked() {
                if let Some(cart) = self.registry.active() {
                    let sections = cart.generate_exam(
                        &self.exam_category_selection,
                        self.config.questions_per_category,
                    );
                    self.exam_state = Some(ExamState::new(sections));
                    self.mode = AppMode::ExamInProgress;
                    self.config.save();
                }
            }
        } else {
            ui.label(
                RichText::new("Selecciona al menos una categoria para continuar.")
                    .size(14.0)
                    .color(Color32::from_rgb(255, 165, 0)),
            );
        }
    }

    pub fn render_exam_in_progress(&mut self, ui: &mut egui::Ui) {
        let exam_data = match &self.exam_state {
            Some(e) => {
                if e.sections.is_empty() {
                    None
                } else {
                    let si = e.current_section;
                    let qi = e.current_question;
                    let (sname, squestions) = &e.sections[si];
                    if squestions.is_empty() {
                        None
                    } else {
                        Some((
                            si,
                            qi,
                            sname.clone(),
                            squestions.len(),
                            squestions[qi].text.clone(),
                            squestions[qi].options.clone(),
                            e.answers[si][qi],
                            e.total_questions(),
                            e.total_answered(),
                            e.sections.len(),
                            e.sections
                                .iter()
                                .enumerate()
                                .map(|(idx, (name, qs))| {
                                    let answered_flags: Vec<bool> =
                                        e.answers[idx].iter().map(|a| a.is_some()).collect();
                                    (name.clone(), qs.len(), answered_flags)
                                })
                                .collect::<Vec<_>>(),
                        ))
                    }
                }
            }
            None => {
                self.mode = AppMode::ExamSetup;
                return;
            }
        };

        let exam_data = match exam_data {
            Some(d) => d,
            None => {
                if let Some(ref mut exam) = self.exam_state {
                    if exam.sections.is_empty() {
                        ui.label(
                            RichText::new("No hay preguntas disponibles.")
                                .size(16.0)
                                .color(Color32::from_rgb(255, 80, 80)),
                        );
                        if ui.button("Volver").clicked() {
                            self.mode = AppMode::ExamSetup;
                        }
                    } else if exam.current_section + 1 < exam.sections.len() {
                        exam.current_section += 1;
                        exam.current_question = 0;
                    }
                }
                return;
            }
        };

        let (
            section_idx,
            question_idx,
            section_name,
            section_len,
            question_text,
            question_options,
            current_answer,
            total_q,
            answered,
            num_sections,
            nav_data,
        ) = exam_data;

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "Progreso: {}/{} preguntas respondidas",
                    answered, total_q
                ))
                .size(13.0),
            );
        });

        let progress = if total_q > 0 {
            answered as f32 / total_q as f32
        } else {
            0.0
        };
        ui.add(
            egui::ProgressBar::new(progress)
                .show_percentage()
                .animate(false),
        );

        ui.add_space(10.0);
        ui.separator();

        ui.label(
            RichText::new(format!(
                "Categoria: {} — Pregunta {}/{}",
                section_name,
                question_idx + 1,
                section_len
            ))
            .size(16.0)
            .strong()
            .color(Color32::from_rgb(100, 149, 237)),
        );

        ui.add_space(15.0);
        ui.label(RichText::new(&question_text).size(17.0).strong());
        ui.add_space(12.0);

        let mut action_select: Option<usize> = None;
        let mut action_prev = false;
        let mut action_next = false;
        let mut action_submit = false;
        let mut action_nav: Option<(usize, usize)> = None;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (opt_idx, option) in question_options.iter().enumerate() {
                    let is_selected = current_answer == Some(opt_idx);
                    let letter = match opt_idx {
                        0 => "A",
                        1 => "B",
                        2 => "C",
                        3 => "D",
                        _ => "?",
                    };

                    let label_text = format!("{}. {}", letter, option);
                    let color = if is_selected {
                        Color32::from_rgb(80, 170, 255)
                    } else {
                        Color32::from_rgb(220, 220, 220)
                    };

                    let response = ui.selectable_label(
                        is_selected,
                        RichText::new(label_text).size(15.0).color(color),
                    );

                    if response.clicked() {
                        action_select = Some(opt_idx);
                    }
                    ui.add_space(4.0);
                }

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    let can_prev = section_idx > 0 || question_idx > 0;
                    if ui
                        .add_enabled(
                            can_prev,
                            egui::Button::new(RichText::new("Anterior").size(14.0)),
                        )
                        .clicked()
                    {
                        action_prev = true;
                    }

                    let is_last =
                        section_idx == num_sections - 1 && question_idx == section_len - 1;

                    if !is_last {
                        if ui
                            .button(RichText::new("Siguiente").size(14.0))
                            .clicked()
                        {
                            action_next = true;
                        }
                    }

                    ui.add_space(20.0);

                    if ui
                        .button(
                            RichText::new("Entregar Examen")
                                .size(14.0)
                                .strong()
                                .color(Color32::from_rgb(255, 100, 100)),
                        )
                        .clicked()
                    {
                        action_submit = true;
                    }
                });

                // Question navigator grid
                ui.add_space(20.0);
                ui.separator();
                ui.label(
                    RichText::new("Navegador de preguntas:")
                        .size(13.0)
                        .strong(),
                );
                ui.add_space(5.0);

                for (s_idx, (s_name, _q_count, answered_flags)) in nav_data.iter().enumerate()
                {
                    ui.label(
                        RichText::new(s_name)
                            .size(12.0)
                            .color(Color32::from_rgb(200, 180, 120)),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (q_idx, is_answered) in answered_flags.iter().enumerate() {
                            let is_current =
                                s_idx == section_idx && q_idx == question_idx;
                            let color = if is_current {
                                Color32::from_rgb(255, 205, 0)
                            } else if *is_answered {
                                Color32::from_rgb(80, 200, 120)
                            } else {
                                Color32::from_rgb(120, 120, 140)
                            };

                            let btn_text = RichText::new(format!("{}", q_idx + 1))
                                .size(11.0)
                                .color(color);
                            if ui.small_button(btn_text).clicked() {
                                action_nav = Some((s_idx, q_idx));
                            }
                        }
                    });
                }
            });

        // Apply deferred actions
        if let Some(opt_idx) = action_select {
            if let Some(ref mut exam) = self.exam_state {
                exam.answers[section_idx][question_idx] = Some(opt_idx);
            }
        }

        if action_prev {
            if let Some(ref mut exam) = self.exam_state {
                if exam.current_question > 0 {
                    exam.current_question -= 1;
                } else if exam.current_section > 0 {
                    exam.current_section -= 1;
                    let prev_len = exam.sections[exam.current_section].1.len();
                    exam.current_question = if prev_len > 0 { prev_len - 1 } else { 0 };
                }
            }
        }

        if action_next {
            if let Some(ref mut exam) = self.exam_state {
                if exam.current_question + 1
                    < exam.sections[exam.current_section].1.len()
                {
                    exam.current_question += 1;
                } else if exam.current_section + 1 < exam.sections.len() {
                    exam.current_section += 1;
                    exam.current_question = 0;
                }
            }
        }

        if let Some((s, q)) = action_nav {
            if let Some(ref mut exam) = self.exam_state {
                exam.current_section = s;
                exam.current_question = q;
            }
        }

        if action_submit {
            if let Some(ref mut exam) = self.exam_state {
                exam.submit();
                let cart_id = self.registry.active_id().to_string();
                for (cat, score, total) in &exam.results {
                    self.progress
                        .add_exam_record(&cart_id, cat, *score, *total);
                }
            }
            self.mode = AppMode::ExamResults;
        }
    }

    pub fn render_exam_results(&mut self, ui: &mut egui::Ui) {
        let display_data = match &self.exam_state {
            Some(exam) => {
                let (total_score, total_questions) = exam.overall_score();
                let results = exam.results.clone();
                let review: Vec<(String, Vec<(String, Vec<String>, usize, Option<usize>)>)> =
                    exam.sections
                        .iter()
                        .enumerate()
                        .map(|(s_idx, (s_name, questions))| {
                            let qs: Vec<_> = questions
                                .iter()
                                .enumerate()
                                .map(|(q_idx, q)| {
                                    (
                                        q.text.clone(),
                                        q.options.clone(),
                                        q.correct_index,
                                        exam.answers[s_idx][q_idx],
                                    )
                                })
                                .collect();
                            (s_name.clone(), qs)
                        })
                        .collect();
                Some((total_score, total_questions, results, review))
            }
            None => {
                self.mode = AppMode::ExamSetup;
                return;
            }
        };

        let (total_score, total_questions, results, review) = display_data.unwrap();

        ui.add_space(20.0);

        let percentage = if total_questions > 0 {
            (total_score as f64 / total_questions as f64 * 100.0) as u32
        } else {
            0
        };

        let all_categories_pass = results.iter().all(|(cat, score, total)| {
            if *total == 0 {
                return true;
            }
            let pct = (*score as f64 / *total as f64 * 100.0) as u32;
            let threshold = self
                .registry
                .active()
                .map(|c| c.pass_threshold(cat))
                .unwrap_or(50);
            pct >= threshold
        });

        let overall_pass = all_categories_pass && total_questions > 0;

        let result_color = if overall_pass {
            Color32::from_rgb(80, 200, 120)
        } else if percentage >= 50 {
            Color32::from_rgb(255, 165, 0)
        } else {
            Color32::from_rgb(255, 80, 80)
        };

        ui.label(
            RichText::new("Resultados del Examen")
                .size(24.0)
                .strong()
                .color(Color32::from_rgb(100, 149, 237)),
        );

        ui.add_space(15.0);

        ui.label(
            RichText::new(format!(
                "Puntaje Total: {}/{} ({}%)",
                total_score, total_questions, percentage
            ))
            .size(20.0)
            .strong()
            .color(result_color),
        );

        let pass_text = if overall_pass {
            "APROBADO — Buen trabajo!"
        } else {
            "REPROBADO — Debes aprobar TODAS las categorias individualmente."
        };
        ui.label(RichText::new(pass_text).size(16.0).color(result_color));

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new("Resultados por Categoria:")
                .size(16.0)
                .strong(),
        );
        ui.add_space(8.0);

        for (cat, score, total) in &results {
            let pct = if *total > 0 {
                (*score as f64 / *total as f64 * 100.0) as u32
            } else {
                0
            };
            let threshold = self
                .registry
                .active()
                .map(|c| c.pass_threshold(cat))
                .unwrap_or(50);
            let cat_passed = pct >= threshold;
            let cat_color = if cat_passed {
                Color32::from_rgb(80, 200, 120)
            } else if pct >= threshold.saturating_sub(15) {
                Color32::from_rgb(255, 165, 0)
            } else {
                Color32::from_rgb(255, 80, 80)
            };

            ui.horizontal(|ui| {
                let status = if cat_passed { "✓" } else { "✗" };
                ui.label(
                    RichText::new(format!("{} {}: ", status, cat)).size(14.0),
                );
                ui.label(
                    RichText::new(format!(
                        "{}/{} ({}%) [min: {}%]",
                        score, total, pct, threshold
                    ))
                    .size(14.0)
                    .color(cat_color),
                );
            });
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        ui.label(
            RichText::new("Revision de Respuestas:")
                .size(16.0)
                .strong(),
        );
        ui.add_space(10.0);

        let mut action_new_exam = false;
        let mut action_study = false;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for (s_name, questions) in &review {
                    ui.label(
                        RichText::new(s_name)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(200, 180, 120)),
                    );
                    ui.add_space(5.0);

                    for (q_idx, (text, options, correct_idx, user_answer)) in
                        questions.iter().enumerate()
                    {
                        let is_correct = *user_answer == Some(*correct_idx);

                        let marker = if is_correct { "[OK]" } else { "[X]" };
                        let marker_color = if is_correct {
                            Color32::from_rgb(80, 200, 120)
                        } else {
                            Color32::from_rgb(255, 80, 80)
                        };

                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} {}. {}",
                                    marker,
                                    q_idx + 1,
                                    text
                                ))
                                .size(13.0)
                                .color(marker_color),
                            );
                        });

                        if !is_correct {
                            if let Some(user_idx) = user_answer {
                                ui.label(
                                    RichText::new(format!(
                                        "   Tu respuesta: {}",
                                        options[*user_idx]
                                    ))
                                    .size(12.0)
                                    .color(Color32::from_rgb(255, 120, 120)),
                                );
                            } else {
                                ui.label(
                                    RichText::new("   Sin respuesta")
                                        .size(12.0)
                                        .color(Color32::from_rgb(180, 180, 180)),
                                );
                            }
                            ui.label(
                                RichText::new(format!(
                                    "   Respuesta correcta: {}",
                                    options[*correct_idx]
                                ))
                                .size(12.0)
                                .color(Color32::from_rgb(80, 200, 120)),
                            );
                        }
                        ui.add_space(3.0);
                    }
                    ui.add_space(10.0);
                }

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("Nuevo Examen")
                                .size(14.0)
                                .color(Color32::from_rgb(100, 149, 237)),
                        )
                        .clicked()
                    {
                        action_new_exam = true;
                    }

                    if ui
                        .button(
                            RichText::new("Volver a Estudiar")
                                .size(14.0)
                                .color(Color32::from_rgb(80, 200, 120)),
                        )
                        .clicked()
                    {
                        action_study = true;
                    }
                });
            });

        if action_new_exam {
            self.exam_state = None;
            self.mode = AppMode::ExamSetup;
        }
        if action_study {
            self.exam_state = None;
            self.mode = AppMode::Study;
        }
    }
}

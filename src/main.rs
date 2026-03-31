#![windows_subsystem = "windows"]

use eframe::egui;
use egui::{Color32, RichText, ScrollArea, Vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// ═══════════════════════════════════════════════════════════════════════════════
// DATA MODELS
// ═══════════════════════════════════════════════════════════════════════════════

/// A fixed multiple-choice question loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
struct FixedQuestion {
    text: String,
    options: Vec<String>,
    answer: usize,
}

/// A key-value pair for template expansion.
#[derive(Debug, Clone, Deserialize)]
struct TemplatePair {
    key: String,
    value: String,
}

/// A template that generates many questions dynamically.
#[derive(Debug, Clone, Deserialize)]
struct QuestionTemplate {
    text: String,
    #[allow(dead_code)]
    variable: String,
    answer_template: String,
    distractors: Vec<String>,
    pairs: Vec<TemplatePair>,
}

/// Root structure of a question TOML file.
#[derive(Debug, Clone, Deserialize, Default)]
struct QuestionBank {
    #[serde(default)]
    questions: Vec<FixedQuestion>,
    #[serde(default)]
    templates: Vec<QuestionTemplate>,
}

/// A ready-to-display question (generated from fixed or template).
#[derive(Debug, Clone)]
struct DisplayQuestion {
    text: String,
    options: Vec<String>,
    correct_index: usize,
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONTENT TREE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
struct ContentNode {
    name: String,
    display_name: String,
    path: PathBuf,
    kind: ContentKind,
}

#[derive(Debug, Clone)]
enum ContentKind {
    File,
    Dir(Vec<ContentNode>),
}

impl ContentNode {
    fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();

        if path.is_dir() {
            let mut children: Vec<ContentNode> = fs::read_dir(path)
                .ok()?
                .filter_map(|e| e.ok())
                .filter_map(|e| ContentNode::from_path(&e.path()))
                .collect();

            if children.is_empty() {
                return None;
            }

            children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            let display_name = slug_to_title(&name);
            Some(ContentNode {
                name,
                display_name,
                path: path.to_path_buf(),
                kind: ContentKind::Dir(children),
            })
        } else if name.to_lowercase().ends_with(".md") {
            let display_name = slug_to_title(
                &path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            );
            Some(ContentNode {
                name,
                display_name,
                path: path.to_path_buf(),
                kind: ContentKind::File,
            })
        } else {
            None
        }
    }

    fn is_dir(&self) -> bool {
        matches!(self.kind, ContentKind::Dir(_))
    }
}

fn slug_to_title(slug: &str) -> String {
    // Strip leading number prefix like "01-" or "03-"
    let slug = if slug.len() > 3
        && slug.chars().take(2).all(|c| c.is_ascii_digit())
        && slug.chars().nth(2) == Some('-')
    {
        &slug[3..]
    } else {
        slug
    };

    slug.split(|c: char| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn build_content_tree(root: &Path) -> Vec<ContentNode> {
    if !root.is_dir() {
        return vec![];
    }

    let mut nodes: Vec<ContentNode> = fs::read_dir(root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| ContentNode::from_path(&e.path()))
                .collect()
        })
        .unwrap_or_default();

    nodes.sort_by(|a, b| match (a.is_dir(), b.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    nodes
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUESTION GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

fn load_question_banks(questions_dir: &Path) -> Vec<(String, QuestionBank)> {
    let mut banks = Vec::new();
    if let Ok(entries) = fs::read_dir(questions_dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                if let Ok(content) = fs::read_to_string(&path) {
                    match toml::from_str::<QuestionBank>(&content) {
                        Ok(bank) => {
                            let name = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            banks.push((slug_to_title(&name), bank));
                        }
                        Err(e) => {
                            eprintln!("Error parsing {}: {}", path.display(), e);
                        }
                    }
                }
            }
        }
    }
    banks
}

fn generate_questions_from_bank(bank: &QuestionBank, count: usize) -> Vec<DisplayQuestion> {
    let mut rng = rand::thread_rng();
    let mut all_questions: Vec<DisplayQuestion> = Vec::new();

    // Add all fixed questions
    for q in &bank.questions {
        if q.options.len() >= 2 && q.answer < q.options.len() {
            all_questions.push(DisplayQuestion {
                text: q.text.clone(),
                options: q.options.clone(),
                correct_index: q.answer,
            });
        }
    }

    // Generate from templates
    for template in &bank.templates {
        for pair in &template.pairs {
            let text = template.text.replace("{key}", &pair.key);
            let correct = template.answer_template.replace("{value}", &pair.value);

            // Pick 3 random distractors that aren't the correct answer
            let mut available_distractors: Vec<&String> = template
                .distractors
                .iter()
                .filter(|d| d.to_lowercase() != correct.to_lowercase())
                .collect();
            available_distractors.shuffle(&mut rng);
            let chosen: Vec<String> = available_distractors
                .iter()
                .take(3)
                .map(|s| s.to_string())
                .collect();

            if chosen.len() < 3 {
                continue;
            }

            // Build options and shuffle
            let mut options = vec![correct.clone()];
            options.extend(chosen);
            let correct_answer = options[0].clone();
            options.shuffle(&mut rng);
            let correct_index = options
                .iter()
                .position(|o| *o == correct_answer)
                .unwrap_or(0);

            all_questions.push(DisplayQuestion {
                text,
                options,
                correct_index,
            });
        }
    }

    // Shuffle and take requested count
    all_questions.shuffle(&mut rng);
    all_questions.truncate(count);
    all_questions
}

fn generate_exam(
    banks: &[(String, QuestionBank)],
    questions_per_category: usize,
) -> Vec<(String, Vec<DisplayQuestion>)> {
    banks
        .iter()
        .map(|(name, bank)| {
            let questions = generate_questions_from_bank(bank, questions_per_category);
            (name.clone(), questions)
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONFIG
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    dark_mode: bool,
    zoom: f32,
    questions_per_category: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dark_mode: true,
            zoom: 1.0,
            questions_per_category: 10,
        }
    }
}

impl Config {
    fn load() -> Self {
        if let Ok(s) = fs::read_to_string(Self::path()) {
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = fs::write(p, s);
        }
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ExamHelper")
            .join("config.json")
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// PROGRESS TRACKER
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Serialize, Deserialize, Default)]
struct Progress {
    /// Which content files have been fully read (scrolled through)
    read_files: Vec<String>,
    /// Exam history: (timestamp_str, score, total, category)
    exam_history: Vec<ExamRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ExamRecord {
    timestamp: String,
    category: String,
    score: usize,
    total: usize,
}

impl Progress {
    fn load() -> Self {
        if let Ok(s) = fs::read_to_string(Self::path()) {
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = fs::write(p, s);
        }
    }

    fn mark_read(&mut self, file_path: &str) {
        let key = file_path.to_string();
        if !self.read_files.contains(&key) {
            self.read_files.push(key);
            self.save();
        }
    }

    fn is_read(&self, file_path: &str) -> bool {
        self.read_files.contains(&file_path.to_string())
    }

    fn add_exam_record(&mut self, category: &str, score: usize, total: usize) {
        self.exam_history.push(ExamRecord {
            timestamp: chrono_now(),
            category: category.to_string(),
            score,
            total,
        });
        self.save();
    }

    fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ExamHelper")
            .join("progress.json")
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple timestamp format
    format!("{}", secs)
}

// ═══════════════════════════════════════════════════════════════════════════════
// THEME
// ═══════════════════════════════════════════════════════════════════════════════

fn apply_theme(ctx: &egui::Context, dark: bool) {
    if dark {
        let mut v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(20, 22, 28);
        v.window_fill = Color32::from_rgb(28, 30, 38);
        v.window_rounding = egui::Rounding::same(6.0);
        v.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, Color32::from_rgb(50, 55, 70));
        ctx.set_visuals(v);
    } else {
        let mut v = egui::Visuals::light();
        v.window_rounding = egui::Rounding::same(6.0);
        ctx.set_visuals(v);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// APP MODE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Study,
    ExamSetup,
    ExamInProgress,
    ExamResults,
    ProgressView,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXAM STATE
// ═══════════════════════════════════════════════════════════════════════════════

struct ExamState {
    /// Each section: (category_name, questions)
    sections: Vec<(String, Vec<DisplayQuestion>)>,
    /// User answers: [section_idx][question_idx] -> Option<selected_option>
    answers: Vec<Vec<Option<usize>>>,
    /// Current section index
    current_section: usize,
    /// Current question index within section
    current_question: usize,
    /// Whether exam has been submitted
    submitted: bool,
    /// Per-category results after submission: (category, score, total)
    results: Vec<(String, usize, usize)>,
    /// Which categories were selected for this exam
    selected_categories: Vec<bool>,
}

impl ExamState {
    fn new(sections: Vec<(String, Vec<DisplayQuestion>)>) -> Self {
        let answers: Vec<Vec<Option<usize>>> = sections
            .iter()
            .map(|(_, qs)| vec![None; qs.len()])
            .collect();
        Self {
            sections,
            answers,
            current_section: 0,
            current_question: 0,
            submitted: false,
            results: Vec::new(),
            selected_categories: Vec::new(),
        }
    }

    fn total_questions(&self) -> usize {
        self.sections.iter().map(|(_, qs)| qs.len()).sum()
    }

    fn total_answered(&self) -> usize {
        self.answers
            .iter()
            .map(|section| section.iter().filter(|a| a.is_some()).count())
            .sum()
    }

    fn submit(&mut self) {
        self.results.clear();
        for (idx, (name, questions)) in self.sections.iter().enumerate() {
            let mut score = 0;
            for (q_idx, q) in questions.iter().enumerate() {
                if self.answers[idx][q_idx] == Some(q.correct_index) {
                    score += 1;
                }
            }
            self.results
                .push((name.clone(), score, questions.len()));
        }
        self.submitted = true;
    }

    fn overall_score(&self) -> (usize, usize) {
        let score: usize = self.results.iter().map(|(_, s, _)| s).sum();
        let total: usize = self.results.iter().map(|(_, _, t)| t).sum();
        (score, total)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GIT UPDATE
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, PartialEq)]
enum GitStatus {
    Idle,
    Updating,
    Success(String),
    Error(String),
}

fn git_pull(repo_dir: &Path) -> Result<String, String> {
    // Check if it's a git repo
    if !repo_dir.join(".git").exists() {
        return Err("No es un repositorio git. Inicializa con 'git init' primero.".to_string());
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

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN APP
// ═══════════════════════════════════════════════════════════════════════════════

struct ExamHelperApp {
    // Core
    config: Config,
    progress: Progress,
    app_dir: PathBuf,

    // Content tree
    content_tree: Vec<ContentNode>,
    selected_file: Option<PathBuf>,
    current_content: String,
    md_cache: CommonMarkCache,

    // Questions
    question_banks: Vec<(String, QuestionBank)>,

    // Exam
    exam_state: Option<ExamState>,
    exam_category_selection: Vec<bool>,

    // UI state
    mode: AppMode,
    sidebar_width: f32,
    git_status: GitStatus,
    show_git_status: bool,
    theme_applied: bool,
}

impl ExamHelperApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = Config::load();
        let progress = Progress::load();

        // Determine app directory (where the exe lives)
        let app_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));

        let content_dir = app_dir.join("content");
        let questions_dir = app_dir.join("questions");

        let content_tree = build_content_tree(&content_dir);
        let question_banks = load_question_banks(&questions_dir);
        let exam_category_selection = vec![true; question_banks.len()];

        Self {
            config,
            progress,
            app_dir,
            content_tree,
            selected_file: None,
            current_content: String::new(),
            md_cache: CommonMarkCache::default(),
            question_banks,
            exam_state: None,
            exam_category_selection,
            mode: AppMode::Study,
            sidebar_width: 260.0,
            git_status: GitStatus::Idle,
            show_git_status: false,
            theme_applied: false,
        }
    }

    fn reload_content(&mut self) {
        let content_dir = self.app_dir.join("content");
        let questions_dir = self.app_dir.join("questions");
        self.content_tree = build_content_tree(&content_dir);
        self.question_banks = load_question_banks(&questions_dir);
        self.exam_category_selection = vec![true; self.question_banks.len()];
    }

    fn load_file(&mut self, path: &PathBuf) {
        if let Ok(content) = fs::read_to_string(path) {
            self.current_content = content;
            self.selected_file = Some(path.clone());
            self.md_cache = CommonMarkCache::default();
        }
    }

    fn render_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // App title
            ui.label(
                RichText::new("ExamHelper Colombia")
                    .size(18.0)
                    .strong()
                    .color(Color32::from_rgb(255, 205, 0)),
            );

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
                self.mode = AppMode::ExamSetup;
            }

            let progress_color = if self.mode == AppMode::ProgressView {
                Color32::from_rgb(255, 165, 0)
            } else {
                Color32::from_rgb(180, 180, 180)
            };
            if ui
                .button(
                    RichText::new("Progreso")
                        .color(progress_color)
                        .size(14.0),
                )
                .clicked()
            {
                self.mode = AppMode::ProgressView;
            }

            ui.separator();

            // Git update button
            if ui
                .button(RichText::new("Actualizar Contenido").size(12.0))
                .clicked()
            {
                self.git_status = GitStatus::Updating;
                match git_pull(&self.app_dir) {
                    Ok(msg) => {
                        self.git_status = GitStatus::Success(msg);
                        self.reload_content();
                    }
                    Err(msg) => {
                        self.git_status = GitStatus::Error(msg);
                    }
                }
                self.show_git_status = true;
            }

            // Theme toggle
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let theme_label = if self.config.dark_mode {
                    "Modo Claro"
                } else {
                    "Modo Oscuro"
                };
                if ui.button(RichText::new(theme_label).size(12.0)).clicked() {
                    self.config.dark_mode = !self.config.dark_mode;
                    self.config.save();
                    self.theme_applied = false;
                }

                // Zoom controls
                ui.label(
                    RichText::new(format!("{}%", (self.config.zoom * 100.0) as u32)).size(12.0),
                );
                if ui.button(RichText::new("+").size(12.0)).clicked() {
                    self.config.zoom = (self.config.zoom + 0.1).min(3.0);
                    self.config.save();
                }
                if ui.button(RichText::new("-").size(12.0)).clicked() {
                    self.config.zoom = (self.config.zoom - 0.1).max(0.5);
                    self.config.save();
                }
            });
        });
    }

    fn render_git_status_window(&mut self, ctx: &egui::Context) {
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

    fn render_study_sidebar(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut file_to_load: Option<PathBuf> = None;

        ui.label(
            RichText::new("Contenido de Estudio")
                .size(15.0)
                .strong()
                .color(Color32::from_rgb(255, 205, 0)),
        );
        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let tree = self.content_tree.clone();
                for node in &tree {
                    Self::render_content_node(
                        ui,
                        node,
                        &self.selected_file,
                        &self.progress,
                        &mut file_to_load,
                    );
                }
            });

        file_to_load
    }

    fn render_content_node(
        ui: &mut egui::Ui,
        node: &ContentNode,
        selected: &Option<PathBuf>,
        progress: &Progress,
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
                        Self::render_content_node(ui, child, selected, progress, file_to_load);
                    }
                });
            }
            ContentKind::File => {
                let is_selected = selected.as_ref() == Some(&node.path);
                let is_read = progress.is_read(&node.path.to_string_lossy());

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

    fn render_study_content(&mut self, ui: &mut egui::Ui) {
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

        let zoom = self.config.zoom;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Scale text via ctx
                let mut style = (*ui.ctx().style()).clone();
                style.override_text_style = Some(egui::TextStyle::Body);
                style.spacing.item_spacing.y = 4.0 * zoom;
                for (_text_style, font_id) in style.text_styles.iter_mut() {
                    font_id.size *= zoom;
                }
                ui.set_style(style);

                CommonMarkViewer::new("study_content")
                    .show(ui, &mut self.md_cache, &self.current_content);

                ui.add_space(20.0);

                // Mark as read button
                if let Some(ref path) = self.selected_file {
                    let path_str = path.to_string_lossy().to_string();
                    if !self.progress.is_read(&path_str) {
                        ui.separator();
                        if ui
                            .button(
                                RichText::new("Marcar como leido")
                                    .size(14.0)
                                    .color(Color32::from_rgb(80, 200, 120)),
                            )
                            .clicked()
                        {
                            self.progress.mark_read(&path_str);
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

    fn render_exam_setup(&mut self, ui: &mut egui::Ui) {
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

        // Category selection
        ui.label(
            RichText::new("Selecciona las categorias a evaluar:")
                .size(15.0)
                .strong(),
        );
        ui.add_space(8.0);

        // Ensure selection vec matches banks
        if self.exam_category_selection.len() != self.question_banks.len() {
            self.exam_category_selection = vec![true; self.question_banks.len()];
        }

        for (idx, (name, bank)) in self.question_banks.iter().enumerate() {
            let total = bank.questions.len()
                + bank
                    .templates
                    .iter()
                    .map(|t| t.pairs.len())
                    .sum::<usize>();
            ui.checkbox(
                &mut self.exam_category_selection[idx],
                RichText::new(format!("{} ({} preguntas disponibles)", name, total)).size(14.0),
            );
        }

        ui.add_space(15.0);

        // Questions per category slider
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
                // Generate exam with selected categories
                let selected_banks: Vec<(String, QuestionBank)> = self
                    .question_banks
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| self.exam_category_selection[*idx])
                    .map(|(_, b)| b.clone())
                    .collect();

                let sections =
                    generate_exam(&selected_banks, self.config.questions_per_category);

                self.exam_state = Some(ExamState::new(sections));
                self.mode = AppMode::ExamInProgress;
                self.config.save();
            }
        } else {
            ui.label(
                RichText::new("Selecciona al menos una categoria para continuar.")
                    .size(14.0)
                    .color(Color32::from_rgb(255, 165, 0)),
            );
        }
    }

    fn render_exam_in_progress(&mut self, ui: &mut egui::Ui) {
        // Extract all needed data upfront to avoid borrow issues
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
                            // Navigator data: Vec<(section_name, q_count, Vec<is_answered>)>
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
                // Handle empty sections: try advancing
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

        let zoom = self.config.zoom;

        // Top progress bar
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

        // Section header
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

        // Question text
        ui.label(RichText::new(&question_text).size(17.0).strong());
        ui.add_space(12.0);

        // Collect UI actions to apply after rendering
        let mut action_select: Option<usize> = None;
        let mut action_prev = false;
        let mut action_next = false;
        let mut action_submit = false;
        let mut action_nav: Option<(usize, usize)> = None;

        ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Options
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
                        RichText::new(label_text).size(15.0 * zoom).color(color),
                    );

                    if response.clicked() {
                        action_select = Some(opt_idx);
                    }
                    ui.add_space(4.0);
                }

                ui.add_space(20.0);

                // Navigation buttons
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

                    let is_last = section_idx == num_sections - 1
                        && question_idx == section_len - 1;

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

                for (s_idx, (s_name, _q_count, answered_flags)) in nav_data.iter().enumerate() {
                    ui.label(
                        RichText::new(s_name)
                            .size(12.0)
                            .color(Color32::from_rgb(200, 180, 120)),
                    );
                    ui.horizontal_wrapped(|ui| {
                        for (q_idx, is_answered) in answered_flags.iter().enumerate() {
                            let is_current = s_idx == section_idx && q_idx == question_idx;
                            let color = if is_current {
                                Color32::from_rgb(255, 205, 0)
                            } else if *is_answered {
                                Color32::from_rgb(80, 200, 120)
                            } else {
                                Color32::from_rgb(120, 120, 140)
                            };

                            let btn_text =
                                RichText::new(format!("{}", q_idx + 1)).size(11.0).color(color);
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
                if exam.current_question + 1 < exam.sections[exam.current_section].1.len() {
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
                for (cat, score, total) in &exam.results {
                    self.progress.add_exam_record(cat, *score, *total);
                }
            }
            self.mode = AppMode::ExamResults;
        }
    }

    fn render_exam_results(&mut self, ui: &mut egui::Ui) {
        // Extract all display data upfront
        let display_data = match &self.exam_state {
            Some(exam) => {
                let (total_score, total_questions) = exam.overall_score();
                let results = exam.results.clone();
                // Build review data
                let review: Vec<(String, Vec<(String, Vec<String>, usize, Option<usize>)>)> = exam
                    .sections
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

        let result_color = if percentage >= 70 {
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

        let pass_text = if percentage >= 70 {
            "APROBADO — Buen trabajo!"
        } else {
            "REPROBADO — Sigue estudiando, tu puedes!"
        };
        ui.label(RichText::new(pass_text).size(16.0).color(result_color));

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);

        // Per-category results
        ui.label(RichText::new("Resultados por Categoria:").size(16.0).strong());
        ui.add_space(8.0);

        for (cat, score, total) in &results {
            let pct = if *total > 0 {
                (*score as f64 / *total as f64 * 100.0) as u32
            } else {
                0
            };
            let cat_color = if pct >= 70 {
                Color32::from_rgb(80, 200, 120)
            } else if pct >= 50 {
                Color32::from_rgb(255, 165, 0)
            } else {
                Color32::from_rgb(255, 80, 80)
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}: ", cat)).size(14.0));
                ui.label(
                    RichText::new(format!("{}/{} ({}%)", score, total, pct))
                        .size(14.0)
                        .color(cat_color),
                );
            });
        }

        ui.add_space(20.0);
        ui.separator();
        ui.add_space(10.0);

        // Review answers
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
                                RichText::new(format!("{} {}. {}", marker, q_idx + 1, text))
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

        // Apply deferred actions
        if action_new_exam {
            self.exam_state = None;
            self.mode = AppMode::ExamSetup;
        }
        if action_study {
            self.exam_state = None;
            self.mode = AppMode::Study;
        }
    }

    fn render_progress_view(&mut self, ui: &mut egui::Ui) {
        ui.add_space(20.0);
        ui.label(
            RichText::new("Tu Progreso de Estudio")
                .size(22.0)
                .strong()
                .color(Color32::from_rgb(255, 165, 0)),
        );
        ui.add_space(15.0);
        ui.separator();

        // Reading progress
        ui.add_space(10.0);
        ui.label(RichText::new("Lectura:").size(16.0).strong());
        ui.add_space(5.0);

        let total_files = count_files(&self.content_tree);
        let read_count = self.progress.read_files.len();

        ui.label(
            RichText::new(format!(
                "Temas leidos: {}/{}",
                read_count, total_files
            ))
            .size(14.0),
        );

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

        if self.progress.exam_history.is_empty() {
            ui.label(
                RichText::new("No has realizado examenes aun.")
                    .size(14.0)
                    .color(Color32::from_rgb(150, 150, 170)),
            );
        } else {
            ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    // Show most recent first
                    for record in self.progress.exam_history.iter().rev().take(20) {
                        let pct = if record.total > 0 {
                            (record.score as f64 / record.total as f64 * 100.0) as u32
                        } else {
                            0
                        };
                        let color = if pct >= 70 {
                            Color32::from_rgb(80, 200, 120)
                        } else if pct >= 50 {
                            Color32::from_rgb(255, 165, 0)
                        } else {
                            Color32::from_rgb(255, 80, 80)
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{}: ", record.category)).size(13.0),
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
                        self.progress.exam_history.clear();
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
                        self.progress.read_files.clear();
                        self.progress.save();
                    }
                });
        }
    }
}

fn count_files(nodes: &[ContentNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        match &node.kind {
            ContentKind::File => count += 1,
            ContentKind::Dir(children) => count += count_files(children),
        }
    }
    count
}

// (helper functions removed — logic inlined into render methods)

// ═══════════════════════════════════════════════════════════════════════════════
// EFRAME IMPL
// ═══════════════════════════════════════════════════════════════════════════════

impl eframe::App for ExamHelperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply theme once or on change
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

        // Git status window
        self.render_git_status_window(ctx);

        match self.mode.clone() {
            AppMode::Study => {
                // Sidebar
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

                // Main content
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
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════════

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("ExamHelper — Naturalización Colombia"),
        ..Default::default()
    };

    eframe::run_native(
        "ExamHelper",
        native_options,
        Box::new(|cc| Ok(Box::new(ExamHelperApp::new(cc)))),
    )
}

use super::{
    slug_to_title, Cartridge, CartridgeManifest, CategoryConfig, ContentKind, ContentNode,
    DisplayQuestion, ExamConfig,
};
use rand::seq::SliceRandom;
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// A cartridge backed by a filesystem directory containing
/// `manifest.toml`, `content/`, and `questions/`.
pub struct FilesystemCartridge {
    manifest: CartridgeManifest,
    root_dir: PathBuf,
    content_tree: Vec<ContentNode>,
    question_banks: Vec<(String, QuestionBank)>,
}

// ── Question data model (TOML format) ──────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct FixedQuestion {
    text: String,
    options: Vec<String>,
    answer: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplatePair {
    key: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct QuestionTemplate {
    text: String,
    #[allow(dead_code)]
    variable: String,
    answer_template: String,
    distractors: Vec<String>,
    pairs: Vec<TemplatePair>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct QuestionBank {
    #[serde(default)]
    questions: Vec<FixedQuestion>,
    #[serde(default)]
    templates: Vec<QuestionTemplate>,
}

// ── Implementation ─────────────────────────────────────────────────────────

impl FilesystemCartridge {
    /// Create a cartridge from a directory with `manifest.toml`.
    pub fn from_dir(dir: &Path) -> Option<Self> {
        let manifest_path = dir.join("manifest.toml");
        let manifest_str = fs::read_to_string(&manifest_path).ok()?;
        let manifest: CartridgeManifest = toml::from_str(&manifest_str).ok()?;

        let content_dir = dir.join("content");
        let questions_dir = dir.join("questions");

        let content_tree = build_content_tree(&content_dir);
        let question_banks = load_question_banks(&questions_dir);

        Some(Self {
            manifest,
            root_dir: dir.to_path_buf(),
            content_tree,
            question_banks,
        })
    }

    /// Create a cartridge from a legacy layout (content/ and questions/ at root).
    /// Synthesizes a manifest with sensible Colombian exam defaults.
    pub fn from_legacy(app_dir: &Path) -> Option<Self> {
        let content_dir = app_dir.join("content");
        let questions_dir = app_dir.join("questions");

        if !content_dir.is_dir() || !questions_dir.is_dir() {
            return None;
        }

        let content_tree = build_content_tree(&content_dir);
        let question_banks = load_question_banks(&questions_dir);

        let manifest = CartridgeManifest {
            id: "colombian-nationality-exam".to_string(),
            name: "Examen de Nacionalidad Colombiana".to_string(),
            description: "Estudio y practica para el examen de naturalización colombiana"
                .to_string(),
            version: "1.0.0".to_string(),
            language: "es".to_string(),
            tts_voice_preference: vec![
                "es-CO".to_string(),
                "es-MX".to_string(),
                "es".to_string(),
            ],
            accent_color: [255, 205, 0],
            exam: ExamConfig {
                default_questions_per_category: 10,
                categories: vec![
                    CategoryConfig {
                        name: "Constitución".to_string(),
                        pass_threshold: 60,
                    },
                    CategoryConfig {
                        name: "Geografía".to_string(),
                        pass_threshold: 55,
                    },
                    CategoryConfig {
                        name: "Historia".to_string(),
                        pass_threshold: 40,
                    },
                    CategoryConfig {
                        name: "Cultura".to_string(),
                        pass_threshold: 40,
                    },
                ],
            },
        };

        Some(Self {
            manifest,
            root_dir: app_dir.to_path_buf(),
            content_tree,
            question_banks,
        })
    }

    fn total_questions_for_bank(bank: &QuestionBank) -> usize {
        bank.questions.len()
            + bank
                .templates
                .iter()
                .map(|t| t.pairs.len())
                .sum::<usize>()
    }
}

impl Cartridge for FilesystemCartridge {
    fn manifest(&self) -> &CartridgeManifest {
        &self.manifest
    }

    fn content_tree(&self) -> &[ContentNode] {
        &self.content_tree
    }

    fn load_content_file(&self, path: &PathBuf) -> Option<String> {
        fs::read_to_string(path).ok()
    }

    fn exam_categories(&self) -> Vec<(String, usize)> {
        self.question_banks
            .iter()
            .map(|(name, bank)| (name.clone(), Self::total_questions_for_bank(bank)))
            .collect()
    }

    fn generate_exam(
        &self,
        selected_categories: &[bool],
        questions_per_category: usize,
    ) -> Vec<(String, Vec<DisplayQuestion>)> {
        self.question_banks
            .iter()
            .enumerate()
            .filter(|(idx, _)| selected_categories.get(*idx).copied().unwrap_or(false))
            .map(|(_, (name, bank))| {
                let questions = generate_questions_from_bank(bank, questions_per_category);
                (name.clone(), questions)
            })
            .collect()
    }

    fn pass_threshold(&self, category: &str) -> u32 {
        // Check manifest config first
        for cat in &self.manifest.exam.categories {
            if cat.name == category {
                return cat.pass_threshold;
            }
        }
        // Legacy hardcoded fallbacks
        match category {
            "Constitución" | "constitucion" => 60,
            "Geografía" | "geografia" => 55,
            "Historia" | "historia" => 40,
            "Cultura" | "cultura" => 40,
            _ => 50,
        }
    }

    fn fonts(&self) -> Vec<(String, Vec<u8>)> {
        let fonts_dir = self.root_dir.join("fonts");
        if !fonts_dir.is_dir() {
            return Vec::new();
        }
        let mut result = Vec::new();
        if let Ok(entries) = fs::read_dir(&fonts_dir) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if matches!(ext.as_str(), "ttf" | "ttc" | "otf") {
                    if let Ok(data) = fs::read(&path) {
                        let name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        result.push((name, data));
                    }
                }
            }
        }
        result
    }

    fn reload(&mut self) {
        let content_dir = self.root_dir.join("content");
        let questions_dir = self.root_dir.join("questions");
        self.content_tree = build_content_tree(&content_dir);
        self.question_banks = load_question_banks(&questions_dir);
    }
}

// ── Filesystem helpers ─────────────────────────────────────────────────────

fn build_content_tree(root: &Path) -> Vec<ContentNode> {
    if !root.is_dir() {
        return vec![];
    }

    let mut nodes: Vec<ContentNode> = fs::read_dir(root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| content_node_from_path(&e.path()))
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

fn content_node_from_path(path: &Path) -> Option<ContentNode> {
    let name = path.file_name()?.to_string_lossy().to_string();

    if path.is_dir() {
        let mut children: Vec<ContentNode> = fs::read_dir(path)
            .ok()?
            .filter_map(|e| e.ok())
            .filter_map(|e| content_node_from_path(&e.path()))
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

    for q in &bank.questions {
        if q.options.len() >= 2 && q.answer < q.options.len() {
            all_questions.push(DisplayQuestion {
                text: q.text.clone(),
                options: q.options.clone(),
                correct_index: q.answer,
            });
        }
    }

    for template in &bank.templates {
        for pair in &template.pairs {
            let text = template.text.replace("{key}", &pair.key);
            let correct = template.answer_template.replace("{value}", &pair.value);

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

    all_questions.shuffle(&mut rng);
    all_questions.truncate(count);
    all_questions
}

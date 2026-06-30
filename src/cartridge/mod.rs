mod filesystem;
mod registry;

pub use filesystem::FilesystemCartridge;
pub use registry::CartridgeRegistry;

use serde::Deserialize;
use std::path::PathBuf;

/// A language used by this cartridge, with TTS voice matching preferences.
#[derive(Debug, Clone, Deserialize)]
pub struct LanguageConfig {
    /// ISO 639-1 language code (e.g. "ja", "en", "es").
    pub code: String,
    /// Ordered prefixes to match against system TTS voice language strings.
    /// e.g. ["ja-JP", "ja"] or ["es-CO", "es-MX", "es"].
    #[serde(default)]
    pub tts_preference: Vec<String>,
}

/// Metadata describing a cartridge.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CartridgeManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    /// Primary language (kept for backward compat / default language for TTS).
    #[serde(default)]
    pub language: String,
    /// Legacy single-language TTS voice preference (deprecated, use `languages`).
    #[serde(default)]
    pub tts_voice_preference: Vec<String>,
    /// All languages used in this cartridge's content, with per-language TTS preferences.
    /// Content switches between these via `{{lang:XX}}` tags.
    #[serde(default)]
    pub languages: Vec<LanguageConfig>,
    /// Accent color for UI theming (RGB).
    #[serde(default = "default_accent")]
    pub accent_color: [u8; 3],
    /// Exam configuration.
    #[serde(default)]
    pub exam: ExamConfig,
}

impl CartridgeManifest {
    /// Return all language configs, falling back to the legacy single-language field.
    pub fn all_languages(&self) -> Vec<LanguageConfig> {
        if !self.languages.is_empty() {
            return self.languages.clone();
        }
        // Fall back to legacy single language + tts_voice_preference
        if !self.language.is_empty() {
            vec![LanguageConfig {
                code: self.language.clone(),
                tts_preference: if self.tts_voice_preference.is_empty() {
                    vec![self.language.clone()]
                } else {
                    self.tts_voice_preference.clone()
                },
            }]
        } else {
            Vec::new()
        }
    }

    /// Return the default/primary language code.
    pub fn default_language(&self) -> &str {
        if let Some(first) = self.languages.first() {
            &first.code
        } else if !self.language.is_empty() {
            &self.language
        } else {
            "en"
        }
    }
}

fn default_accent() -> [u8; 3] {
    [255, 205, 0]
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct ExamConfig {
    #[serde(default = "default_qpc")]
    pub default_questions_per_category: usize,
    #[serde(default)]
    pub categories: Vec<CategoryConfig>,
}

fn default_qpc() -> usize {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct CategoryConfig {
    pub name: String,
    pub pass_threshold: u32,
}

/// A ready-to-display question (common across all cartridges).
#[derive(Debug, Clone)]
pub struct DisplayQuestion {
    pub text: String,
    pub options: Vec<String>,
    pub correct_index: usize,
}

/// Study content tree node.
#[derive(Debug, Clone)]
pub struct ContentNode {
    pub name: String,
    pub display_name: String,
    pub path: PathBuf,
    pub kind: ContentKind,
}

#[derive(Debug, Clone)]
pub enum ContentKind {
    File,
    Dir(Vec<ContentNode>),
}

impl ContentNode {
    pub fn is_dir(&self) -> bool {
        matches!(self.kind, ContentKind::Dir(_))
    }
}

/// An exam category with generated questions and passing threshold.
#[allow(dead_code)]
pub struct ExamCategory {
    pub name: String,
    pub questions: Vec<DisplayQuestion>,
    pub pass_threshold_pct: u32,
}

/// The trait every cartridge must implement.
pub trait Cartridge: Send + Sync {
    /// Return the cartridge manifest (metadata).
    fn manifest(&self) -> &CartridgeManifest;

    /// Return the study content tree.
    fn content_tree(&self) -> &[ContentNode];

    /// Load a content file by path, returning the markdown string.
    #[allow(clippy::ptr_arg)]
    fn load_content_file(&self, path: &PathBuf) -> Option<String>;

    /// Return available exam category names with total question counts.
    fn exam_categories(&self) -> Vec<(String, usize)>;

    /// Generate exam sections: (category_name, questions).
    fn generate_exam(
        &self,
        selected_categories: &[bool],
        questions_per_category: usize,
    ) -> Vec<(String, Vec<DisplayQuestion>)>;

    /// Return the passing threshold percentage for a category name.
    fn pass_threshold(&self, category: &str) -> u32;

    /// Return font data bundled with this cartridge: Vec<(font_name, raw_bytes)>.
    /// Fonts are loaded as fallbacks so the app can render the cartridge's language
    /// without requiring system-wide language pack installation.
    fn fonts(&self) -> Vec<(String, Vec<u8>)> {
        Vec::new() // Default: no custom fonts
    }

    /// Reload content and questions from disk.
    #[allow(dead_code)]
    fn reload(&mut self);
}

pub fn slug_to_title(slug: &str) -> String {
    // Strip leading number prefix like "01-" or "03-"
    let slug = if slug.len() > 3
        && slug.chars().take(2).all(|c| c.is_ascii_digit())
        && slug.chars().nth(2) == Some('-')
    {
        &slug[3..]
    } else {
        slug
    };

    slug.split(['-', '_'])
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

pub fn count_files(nodes: &[ContentNode]) -> usize {
    let mut count = 0;
    for node in nodes {
        match &node.kind {
            ContentKind::File => count += 1,
            ContentKind::Dir(children) => count += count_files(children),
        }
    }
    count
}

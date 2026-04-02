use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{fs, path::PathBuf};

/// Application-wide progress, scoped by cartridge ID.
/// Removing a cartridge directory never loses progress data.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AppProgress {
    /// Per-cartridge progress keyed by cartridge ID.
    pub cartridges: HashMap<String, CartridgeProgress>,
}

/// Progress data for a single cartridge.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CartridgeProgress {
    /// Content files marked as read (path strings relative to cartridge).
    pub read_files: Vec<String>,
    /// Exam history records.
    pub exam_history: Vec<ExamRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExamRecord {
    pub timestamp: String,
    pub category: String,
    pub score: usize,
    pub total: usize,
}

impl AppProgress {
    pub fn load() -> Self {
        let path = Self::path();
        if let Ok(s) = fs::read_to_string(&path) {
            // Try new format first
            if let Ok(p) = serde_json::from_str::<AppProgress>(&s) {
                return p;
            }
            // Try migrating from old flat format
            if let Ok(old) = serde_json::from_str::<OldProgress>(&s) {
                let mut new = AppProgress::default();
                let cp = new.for_cartridge_mut("colombian-nationality-exam");
                cp.read_files = old.read_files;
                cp.exam_history = old
                    .exam_history
                    .into_iter()
                    .map(|r| ExamRecord {
                        timestamp: r.timestamp,
                        category: r.category,
                        score: r.score,
                        total: r.total,
                    })
                    .collect();
                new.save();
                return new;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let p = Self::path();
        if let Some(dir) = p.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let _ = fs::write(p, s);
        }
    }

    pub fn for_cartridge(&self, id: &str) -> &CartridgeProgress {
        static EMPTY: CartridgeProgress = CartridgeProgress {
            read_files: Vec::new(),
            exam_history: Vec::new(),
        };
        self.cartridges.get(id).unwrap_or(&EMPTY)
    }

    pub fn for_cartridge_mut(&mut self, id: &str) -> &mut CartridgeProgress {
        self.cartridges
            .entry(id.to_string())
            .or_insert_with(CartridgeProgress::default)
    }

    pub fn mark_read(&mut self, cartridge_id: &str, file_path: &str) {
        let cp = self.for_cartridge_mut(cartridge_id);
        let key = file_path.to_string();
        if !cp.read_files.contains(&key) {
            cp.read_files.push(key);
            self.save();
        }
    }

    pub fn is_read(&self, cartridge_id: &str, file_path: &str) -> bool {
        self.for_cartridge(cartridge_id)
            .read_files
            .contains(&file_path.to_string())
    }

    pub fn add_exam_record(
        &mut self,
        cartridge_id: &str,
        category: &str,
        score: usize,
        total: usize,
    ) {
        let cp = self.for_cartridge_mut(cartridge_id);
        cp.exam_history.push(ExamRecord {
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
    format!("{}", secs)
}

/// Old flat progress format for migration.
#[derive(Debug, Deserialize)]
struct OldProgress {
    #[serde(default)]
    read_files: Vec<String>,
    #[serde(default)]
    exam_history: Vec<OldExamRecord>,
}

#[derive(Debug, Deserialize)]
struct OldExamRecord {
    timestamp: String,
    category: String,
    score: usize,
    total: usize,
}

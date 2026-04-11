use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::LangRange;

/// Per-chunk metadata stored in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    pub text_hash: String,
    pub lang: String,
    pub file_name: String,
}

/// Manifest for a single content file's baked audio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeManifest {
    /// Hash of voice_map + rate so we rebake if settings change.
    pub voice_config_hash: String,
    pub chunks: Vec<ChunkMeta>,
}

/// Result of checking cache for a content file.
pub struct CacheCheck {
    /// WAV paths for each chunk (None if that chunk needs rebaking).
    pub chunk_paths: Vec<Option<PathBuf>>,
    /// Total chunks.
    pub total: usize,
    /// How many were already cached.
    pub cached: usize,
}

impl CacheCheck {
    pub fn fully_cached(&self) -> bool {
        self.cached == self.total
    }

    /// Efficiency: percentage of chunks served from cache.
    pub fn efficiency_pct(&self) -> f32 {
        if self.total == 0 {
            100.0
        } else {
            (self.cached as f32 / self.total as f32) * 100.0
        }
    }

    /// Indices that need rebaking.
    pub fn missing_indices(&self) -> Vec<usize> {
        self.chunk_paths
            .iter()
            .enumerate()
            .filter_map(|(i, p)| if p.is_none() { Some(i) } else { None })
            .collect()
    }
}

pub struct TtsCache {
    cache_root: PathBuf,
}

impl TtsCache {
    pub fn new() -> Self {
        let root = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ExamHelper")
            .join("cache")
            .join("tts");
        Self { cache_root: root }
    }

    /// Directory for a specific content file's baked audio.
    pub fn content_dir(&self, cartridge_id: &str, file_stem: &str) -> PathBuf {
        self.cache_root.join(cartridge_id).join(file_stem)
    }

    /// Path to the manifest JSON for a content file.
    fn manifest_path(&self, cartridge_id: &str, file_stem: &str) -> PathBuf {
        self.content_dir(cartridge_id, file_stem)
            .join("_manifest.json")
    }

    /// Load manifest if it exists.
    fn load_manifest(&self, cartridge_id: &str, file_stem: &str) -> Option<BakeManifest> {
        let path = self.manifest_path(cartridge_id, file_stem);
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Save manifest.
    pub fn save_manifest(
        &self,
        cartridge_id: &str,
        file_stem: &str,
        manifest: &BakeManifest,
    ) {
        let dir = self.content_dir(cartridge_id, file_stem);
        let _ = fs::create_dir_all(&dir);
        let path = self.manifest_path(cartridge_id, file_stem);
        if let Ok(json) = serde_json::to_string_pretty(manifest) {
            let _ = fs::write(path, json);
        }
    }

    /// Check cache status for a set of ranges.
    /// Returns per-chunk cache status with efficiency info.
    pub fn check(
        &self,
        cartridge_id: &str,
        file_stem: &str,
        ranges: &[LangRange],
        voice_config_hash: &str,
    ) -> CacheCheck {
        let dir = self.content_dir(cartridge_id, file_stem);
        let manifest = self.load_manifest(cartridge_id, file_stem);

        // Flatten ranges into (text, lang) pairs for per-chunk comparison
        let flat: Vec<(&str, &str)> = ranges
            .iter()
            .flat_map(|r| r.chunks.iter().map(move |c| (c.as_str(), r.lang.as_str())))
            .collect();

        let total = flat.len();
        let mut chunk_paths = Vec::with_capacity(total);
        let mut cached = 0;

        match manifest {
            Some(m) if m.voice_config_hash == voice_config_hash && m.chunks.len() == total => {
                for (i, (text, lang)) in flat.iter().enumerate() {
                    let expected_hash = hash_text(text);
                    let meta = &m.chunks[i];
                    let wav_path = dir.join(&meta.file_name);

                    if meta.text_hash == expected_hash
                        && meta.lang == *lang
                        && wav_path.exists()
                    {
                        chunk_paths.push(Some(wav_path));
                        cached += 1;
                    } else {
                        chunk_paths.push(None);
                    }
                }
            }
            _ => {
                // No manifest, wrong config, or chunk count changed — full rebake
                for _ in 0..total {
                    chunk_paths.push(None);
                }
            }
        }

        CacheCheck {
            chunk_paths,
            total,
            cached,
        }
    }

    /// Build a manifest from the current ranges (used after baking).
    pub fn build_manifest(
        ranges: &[LangRange],
        voice_config_hash: &str,
    ) -> BakeManifest {
        let mut chunks = Vec::new();
        let mut idx = 0usize;
        for range in ranges {
            for _chunk in &range.chunks {
                let file_name = format!("chunk_{:04}_{}.wav", idx, range.lang);
                let text_hash = hash_text(_chunk);
                chunks.push(ChunkMeta {
                    text_hash,
                    lang: range.lang.clone(),
                    file_name,
                });
                idx += 1;
            }
        }
        BakeManifest {
            voice_config_hash: voice_config_hash.to_string(),
            chunks,
        }
    }

    /// WAV file path for a specific chunk index.
    pub fn wav_path(&self, cartridge_id: &str, file_stem: &str, idx: usize, lang: &str) -> PathBuf {
        self.content_dir(cartridge_id, file_stem)
            .join(format!("chunk_{:04}_{}.wav", idx, lang))
    }

    /// Total cache size in bytes for a cartridge.
    pub fn cartridge_cache_size(&self, cartridge_id: &str) -> u64 {
        let dir = self.cache_root.join(cartridge_id);
        dir_size(&dir)
    }

    /// Clear all cache for a cartridge.
    pub fn clear_cartridge(&self, cartridge_id: &str) {
        let dir = self.cache_root.join(cartridge_id);
        let _ = fs::remove_dir_all(dir);
    }
}

/// SHA-256 hash of text content, truncated to 16 hex chars.
pub fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

/// Hash voice config (voice map + rate) for cache invalidation.
pub fn hash_voice_config(voice_map: &HashMap<String, String>, rate: f32) -> String {
    let mut hasher = Sha256::new();
    let mut entries: Vec<_> = voice_map.iter().collect();
    entries.sort_by_key(|(k, _)| k.clone());
    for (k, v) in entries {
        hasher.update(k.as_bytes());
        hasher.update(v.as_bytes());
    }
    hasher.update(format!("{:.2}", rate).as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let ft = entry.file_type().unwrap_or_else(|_| unreachable!());
            if ft.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if ft.is_dir() {
                total += dir_size(&entry.path());
            }
        }
    }
    total
}

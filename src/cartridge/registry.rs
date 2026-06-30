use super::{Cartridge, FilesystemCartridge};
use std::{fs, path::Path};

/// Discovers, loads, and manages cartridges.
pub struct CartridgeRegistry {
    cartridges: Vec<Box<dyn Cartridge>>,
    active_index: usize,
}

impl CartridgeRegistry {
    /// Discover cartridges from the app directory.
    /// Checks `cartridges/` subdirectory first, then falls back to legacy layout.
    pub fn discover(app_dir: &Path) -> Self {
        let mut cartridges: Vec<Box<dyn Cartridge>> = Vec::new();

        // 1. Check for cartridges/ directory with manifest.toml subdirs
        let cartridges_dir = app_dir.join("cartridges");
        if cartridges_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&cartridges_dir) {
                let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
                entries.sort_by_key(|e| e.file_name());
                for entry in entries {
                    let path = entry.path();
                    if path.is_dir() && path.join("manifest.toml").exists() {
                        if let Some(cart) = FilesystemCartridge::from_dir(&path) {
                            cartridges.push(Box::new(cart));
                        }
                    }
                }
            }
        }

        // 2. Fall back to legacy layout (content/ + questions/ at app root)
        if cartridges.is_empty() {
            if let Some(cart) = FilesystemCartridge::from_legacy(app_dir) {
                cartridges.push(Box::new(cart));
            }
        }

        Self {
            cartridges,
            active_index: 0,
        }
    }

    pub fn active(&self) -> Option<&dyn Cartridge> {
        self.cartridges.get(self.active_index).map(|c| c.as_ref())
    }

    #[allow(dead_code)]
    pub fn active_mut(&mut self) -> Option<&mut Box<dyn Cartridge>> {
        self.cartridges.get_mut(self.active_index)
    }

    pub fn active_id(&self) -> &str {
        self.active()
            .map(|c| c.manifest().id.as_str())
            .unwrap_or("")
    }

    pub fn list(&self) -> &[Box<dyn Cartridge>] {
        &self.cartridges
    }

    pub fn set_active(&mut self, index: usize) {
        if index < self.cartridges.len() {
            self.active_index = index;
        }
    }

    pub fn set_active_by_id(&mut self, id: &str) {
        if let Some(idx) = self.cartridges.iter().position(|c| c.manifest().id == id) {
            self.active_index = idx;
        }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cartridges.is_empty()
    }

    pub fn count(&self) -> usize {
        self.cartridges.len()
    }
}

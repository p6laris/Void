use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::builtin::BUILTINS;
use super::file::ThemeFile;

const EMBEDDED: &[(&str, &str, &str)] = &[
    (
        "catppuccin-mocha",
        "Catppuccin Mocha",
        include_str!("../../themes/catppuccin-mocha.toml"),
    ),
    (
        "catppuccin-latte",
        "Catppuccin Latte",
        include_str!("../../themes/catppuccin-latte.toml"),
    ),
];

#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub id: String,
    pub label: String,
    pub source: ThemeSource,
}

#[derive(Debug, Clone)]
pub enum ThemeSource {
    Builtin,
    Embedded(&'static str),
    File(PathBuf),
}

#[derive(Debug, Clone, Default)]
pub struct ThemeCatalog {
    entries: Vec<ThemeEntry>,
}

impl ThemeCatalog {
    pub fn load() -> Self {
        let mut catalog = Self::default();
        for (id, label) in BUILTINS {
            catalog.entries.push(ThemeEntry {
                id: (*id).to_string(),
                label: (*label).to_string(),
                source: ThemeSource::Builtin,
            });
        }
        for (id, label, toml) in EMBEDDED {
            catalog.entries.push(ThemeEntry {
                id: (*id).to_string(),
                label: (*label).to_string(),
                source: ThemeSource::Embedded(toml),
            });
        }
        if let Ok(dir) = themes_dir() {
            catalog.scan_dir(&dir);
        }
        if let Ok(extra) = std::env::var("VOID_THEMES_DIR") {
            catalog.scan_dir(Path::new(&extra));
        }
        catalog
    }

    pub fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }

    pub fn label(&self, id: &str) -> String {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.label.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub fn next_id(&self, current: &str) -> String {
        if self.entries.is_empty() {
            return current.to_string();
        }
        let idx = self
            .entries
            .iter()
            .position(|entry| entry.id == current)
            .unwrap_or(0);
        self.entries[(idx + 1) % self.entries.len()].id.clone()
    }

    pub fn resolve_entry(&self, id: &str) -> Result<&ThemeEntry> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .with_context(|| format!("unknown theme `{id}`"))
    }

    fn scan_dir(&mut self, dir: &Path) {
        let Ok(read) = fs::read_dir(dir) else {
            return;
        };
        let mut found = Vec::new();
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "toml") {
                continue;
            }
            let Ok(file) = ThemeFile::from_path(&path) else {
                continue;
            };
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() || self.entries.iter().any(|e| e.id == id) {
                continue;
            }
            found.push(ThemeEntry {
                id,
                label: file.name,
                source: ThemeSource::File(path),
            });
        }
        found.sort_by(|a, b| a.label.cmp(&b.label));
        self.entries.extend(found);
    }
}

pub fn themes_dir() -> Result<PathBuf> {
    dirs::config_dir()
        .map(|dir| dir.join("void").join("themes"))
        .context("resolve config directory")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_catppuccin_embedded() {
        let catalog = ThemeCatalog::load();
        assert!(catalog.entries.iter().any(|e| e.id == "catppuccin-mocha"));
    }

    #[test]
    fn cycles_through_entries() {
        let catalog = ThemeCatalog::load();
        let first = catalog.entries[0].id.clone();
        let second = catalog.next_id(&first);
        assert_ne!(first, second);
        let wrap = catalog.next_id(catalog.entries.last().unwrap().id.as_str());
        assert_eq!(wrap, first);
    }
}

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::color::resolve_color;
use super::tokens::{ThemeTokens, NAMES};

#[derive(Debug, Deserialize)]
pub struct ThemeFile {
    pub name: String,
    #[serde(default)]
    pub palette: HashMap<String, String>,
    pub tokens: HashMap<String, String>,
}

impl ThemeFile {
    pub fn from_str(source: &str) -> Result<Self> {
        toml::from_str(source).context("parse theme toml")
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("read theme file {}", path.display()))?;
        Self::from_str(&source)
    }

    pub fn into_tokens(self) -> Result<ThemeTokens> {
        for required in NAMES {
            if !self.tokens.contains_key(*required) {
                bail!("missing required token `{required}`");
            }
        }

        let get = |name: &str| -> Result<ratatui::style::Color> {
            let raw = self
                .tokens
                .get(name)
                .map(String::as_str)
                .expect("checked above");
            resolve_color(raw, &self.palette).with_context(|| format!("token `{name}`"))
        };

        Ok(ThemeTokens {
            bg: get("bg")?,
            text: get("text")?,
            dim: get("dim")?,
            accent: get("accent")?,
            on_accent: get("on_accent")?,
            success: get("success")?,
            warning: get("warning")?,
            error: get("error")?,
            info: get("info")?,
            progress_dim: get("progress_dim")?,
            task_track: get("task_track")?,
            panel: get("panel")?,
            panel_border: get("panel_border")?,
            select_bg: get("select_bg")?,
            select_fg: get("select_fg")?,
            active_bg: get("active_bg")?,
            active_fg: get("active_fg")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCHA: &str = include_str!("../../themes/catppuccin-mocha.toml");

    #[test]
    fn loads_catppuccin_mocha_tokens() {
        let file = ThemeFile::from_str(MOCHA).unwrap();
        assert_eq!(file.name, "Catppuccin Mocha");
        let tokens = file.into_tokens().unwrap();
        assert_eq!(tokens.bg, ratatui::style::Color::Rgb(30, 30, 46));
        assert_eq!(tokens.accent, ratatui::style::Color::Rgb(137, 182, 250));
    }
}

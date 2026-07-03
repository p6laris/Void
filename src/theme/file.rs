use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use ratatui::style::Color;
use serde::Deserialize;

use super::color::resolve_color;
use super::{Theme, TOKEN_NAMES};

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

    fn resolve_token(&self, name: &str) -> Result<Color> {
        let raw = self
            .tokens
            .get(name)
            .with_context(|| format!("missing token `{name}`"))?;
        resolve_color(raw, &self.palette).with_context(|| format!("token `{name}`"))
    }

    pub fn into_theme(self) -> Result<Theme> {
        for required in TOKEN_NAMES {
            if !self.tokens.contains_key(*required) {
                bail!("missing required token `{required}`");
            }
        }

        Ok(Theme {
            bg: self.resolve_token("bg")?,
            text: self.resolve_token("text")?,
            dim: self.resolve_token("dim")?,
            accent: self.resolve_token("accent")?,
            on_accent: self.resolve_token("on_accent")?,
            success: self.resolve_token("success")?,
            warning: self.resolve_token("warning")?,
            error: self.resolve_token("error")?,
            info: self.resolve_token("info")?,
            progress_dim: self.resolve_token("progress_dim")?,
            task_track: self.resolve_token("task_track")?,
            panel: self.resolve_token("panel")?,
            panel_border: self.resolve_token("panel_border")?,
            select_bg: self.resolve_token("select_bg")?,
            select_fg: self.resolve_token("select_fg")?,
            active_bg: self.resolve_token("active_bg")?,
            active_fg: self.resolve_token("active_fg")?,
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
        let theme = file.into_theme().unwrap();
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
        assert_eq!(theme.accent, Color::Rgb(137, 182, 250));
    }
}

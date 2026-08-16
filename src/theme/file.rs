use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use ratatui::style::Color;
use serde::Deserialize;

use super::color::{is_color_light, resolve_color};
use super::{Theme, ThemeVariant, TOKEN_NAMES};

#[derive(Debug, Deserialize)]
pub struct ThemeFile {
    pub name: String,
    pub variant: Option<String>,
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

    pub fn detect_variant(&self) -> ThemeVariant {
        if let Some(ref v) = self.variant {
            if v.eq_ignore_ascii_case("light") {
                return ThemeVariant::Light;
            } else if v.eq_ignore_ascii_case("dark") {
                return ThemeVariant::Dark;
            }
        }
        if let Ok(bg) = self.resolve_token("bg") {
            if is_color_light(bg) {
                return ThemeVariant::Light;
            }
        }
        ThemeVariant::Dark
    }

    fn resolve_token(&self, name: &str) -> Result<Color> {
        let raw = self
            .tokens
            .get(name)
            .with_context(|| format!("missing token `{name}`"))?;
        resolve_color(raw, &self.palette).with_context(|| format!("token `{name}`"))
    }

    /// Resolves a token only if the theme declares it. Absent optional tokens keep the
    /// value derived from the required palette, so new tokens never break old themes.
    fn optional_token(&self, name: &str) -> Result<Option<Color>> {
        match self.tokens.get(name) {
            None => Ok(None),
            Some(raw) => resolve_color(raw, &self.palette)
                .with_context(|| format!("token `{name}`"))
                .map(Some),
        }
    }

    pub fn into_theme(self) -> Result<Theme> {
        for required in TOKEN_NAMES {
            if !self.tokens.contains_key(*required) {
                bail!("missing required token `{required}`");
            }
        }

        let mut theme = Theme {
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
            ..super::PLACEHOLDER
        }
        .finish();

        // Anything the theme states explicitly overrides the derived value.
        if let Some(c) = self.optional_token("comment")? {
            theme.comment = c;
        }
        if let Some(c) = self.optional_token("surface")? {
            theme.surface = c;
        }
        if let Some(c) = self.optional_token("surface_alt")? {
            theme.surface_alt = c;
        }
        if let Some(c) = self.optional_token("mode_focus")? {
            theme.mode_focus = c;
        }
        if let Some(c) = self.optional_token("mode_short_break")? {
            theme.mode_short_break = c;
        }
        if let Some(c) = self.optional_token("mode_long_break")? {
            theme.mode_long_break = c;
        }
        if let Some(c) = self.optional_token("mode_custom")? {
            theme.mode_custom = c;
        }
        for (idx, name) in super::HEAT_TOKEN_NAMES.iter().enumerate() {
            if let Some(c) = self.optional_token(name)? {
                theme.heat[idx] = c;
            }
        }

        Ok(theme)
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

    /// A theme predating the optional tokens must still load, with everything derived.
    #[test]
    fn theme_without_optional_tokens_derives_them() {
        let mut source = String::from("name = \"Minimal\"\n[tokens]\n");
        for name in TOKEN_NAMES {
            source.push_str(&format!("{name} = \"#808080\"\n"));
        }
        let theme = ThemeFile::from_str(&source).unwrap().into_theme().unwrap();

        // Derived, not left at the placeholder.
        assert_ne!(theme.comment, Color::Reset);
        assert_ne!(theme.surface, Color::Reset);
        assert_ne!(theme.mode_long_break, Color::Reset);
        for step in theme.heat {
            assert_ne!(step, Color::Reset);
        }
    }

    #[test]
    fn explicit_optional_tokens_override_derived() {
        let mut source = String::from("name = \"Override\"\n[tokens]\n");
        for name in TOKEN_NAMES {
            source.push_str(&format!("{name} = \"#808080\"\n"));
        }
        source.push_str("heat_3 = \"#123456\"\n");
        source.push_str("mode_long_break = \"#abcdef\"\n");
        let theme = ThemeFile::from_str(&source).unwrap().into_theme().unwrap();

        assert_eq!(theme.heat[3], Color::Rgb(0x12, 0x34, 0x56));
        assert_eq!(theme.mode_long_break, Color::Rgb(0xab, 0xcd, 0xef));
    }

    #[test]
    fn missing_required_token_still_fails() {
        let source = "name = \"Broken\"\n[tokens]\nbg = \"#000000\"\n";
        assert!(ThemeFile::from_str(source).unwrap().into_theme().is_err());
    }
}

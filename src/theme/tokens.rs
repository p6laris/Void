//! Semantic color tokens used across the Void UI.

/// All required keys for a theme file `[tokens]` table.
pub const NAMES: &[&str] = &[
    "bg",
    "text",
    "dim",
    "accent",
    "on_accent",
    "success",
    "warning",
    "error",
    "info",
    "progress_dim",
    "task_track",
    "panel",
    "panel_border",
    "select_bg",
    "select_fg",
    "active_bg",
    "active_fg",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeTokens {
    pub bg: ratatui::style::Color,
    pub text: ratatui::style::Color,
    pub dim: ratatui::style::Color,
    pub accent: ratatui::style::Color,
    pub on_accent: ratatui::style::Color,
    pub success: ratatui::style::Color,
    pub warning: ratatui::style::Color,
    pub error: ratatui::style::Color,
    pub info: ratatui::style::Color,
    pub progress_dim: ratatui::style::Color,
    pub task_track: ratatui::style::Color,
    pub panel: ratatui::style::Color,
    pub panel_border: ratatui::style::Color,
    pub select_bg: ratatui::style::Color,
    pub select_fg: ratatui::style::Color,
    pub active_bg: ratatui::style::Color,
    pub active_fg: ratatui::style::Color,
}

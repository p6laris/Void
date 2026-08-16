use ratatui::style::Color;

use crate::canvas_timer::SceneStyle;

mod builtin;
mod catalog;
mod color;
mod file;

pub use catalog::{themes_dir, ThemeCatalog, ThemeEntry};

use anyhow::{Context, Result};

use self::builtin::builtin_theme;
use self::catalog::ThemeSource;
use self::file::ThemeFile;

/// Required keys for a theme file `[tokens]` table. Every theme must define these; they
/// carry no sensible default because the rest of the palette is derived from them.
pub const TOKEN_NAMES: &[&str] = &[
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

/// Optional keys a theme *may* define. Each one is derived from the required tokens when
/// absent, so adding to this list never invalidates an existing theme file.
pub const OPTIONAL_TOKEN_NAMES: &[&str] = &[
    "comment",
    "surface",
    "surface_alt",
    "heat_0",
    "heat_1",
    "heat_2",
    "heat_3",
    "heat_4",
    "mode_focus",
    "mode_short_break",
    "mode_long_break",
    "mode_custom",
];

/// Number of steps in the activity heatmap ramp, including the "no activity" step.
pub const HEAT_STEPS: usize = 5;

/// Token names for each heat step, indexed to match `Theme::heat`.
pub const HEAT_TOKEN_NAMES: [&str; HEAT_STEPS] =
    ["heat_0", "heat_1", "heat_2", "heat_3", "heat_4"];

#[derive(Clone)]
pub struct Theme {
    pub bg: Color,
    pub text: Color,
    pub dim: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub progress_dim: Color,
    pub task_track: Color,
    pub panel: Color,
    pub panel_border: Color,
    pub select_bg: Color,
    pub select_fg: Color,
    pub active_bg: Color,
    pub active_fg: Color,

    // ── Derived-by-default tokens ────────────────────────────────────────────
    /// Quietest readable foreground — used for `//` annotation lines. Sits below `dim`.
    pub comment: Color,
    /// Slightly raised background for insets and zebra rows.
    pub surface: Color,
    /// Second elevation, for nested insets.
    pub surface_alt: Color,
    /// Activity ramp, `heat[0]` = no activity through `heat[4]` = most active.
    /// A single hue rising in intensity — never a hue change between steps.
    pub heat: [Color; HEAT_STEPS],
    /// Per-timer-mode accents, so a theme can tint breaks without touching `warning`.
    pub mode_focus: Color,
    pub mode_short_break: Color,
    pub mode_long_break: Color,
    pub mode_custom: Color,
}

/// Fills every derived token from the required ones. Themes that set a token explicitly
/// overwrite the derived value afterwards.
pub(crate) fn derive_defaults(base: &mut Theme) {
    base.comment = mix(base.bg, base.dim, 190);
    base.surface = mix(base.bg, base.panel, 200);
    base.surface_alt = mix(base.panel, base.text, 20);
    base.heat = derive_heat(base.bg, base.accent, base.task_track);
    base.mode_focus = base.accent;
    base.mode_short_break = base.success;
    base.mode_long_break = base.warning;
    base.mode_custom = base.info;
}

/// A monotone single-hue ramp: `bg` blended toward `accent` in rising steps. Works on both
/// dark and light themes because `bg` is always the "no activity" end of the scale.
fn derive_heat(bg: Color, accent: Color, empty: Color) -> [Color; HEAT_STEPS] {
    [
        empty,
        mix(bg, accent, 72),
        mix(bg, accent, 128),
        mix(bg, accent, 190),
        accent,
    ]
}

/// Every field unset. Builtins spread this so they only have to spell out the required
/// tokens; `Theme::finish` then fills the derived ones.
const PLACEHOLDER: Theme = Theme {
    bg: Color::Reset,
    text: Color::Reset,
    dim: Color::Reset,
    accent: Color::Reset,
    on_accent: Color::Reset,
    success: Color::Reset,
    warning: Color::Reset,
    error: Color::Reset,
    info: Color::Reset,
    progress_dim: Color::Reset,
    task_track: Color::Reset,
    panel: Color::Reset,
    panel_border: Color::Reset,
    select_bg: Color::Reset,
    select_fg: Color::Reset,
    active_bg: Color::Reset,
    active_fg: Color::Reset,
    comment: Color::Reset,
    surface: Color::Reset,
    surface_alt: Color::Reset,
    heat: [Color::Reset; HEAT_STEPS],
    mode_focus: Color::Reset,
    mode_short_break: Color::Reset,
    mode_long_break: Color::Reset,
    mode_custom: Color::Reset,
};

impl Theme {
    /// Derives every optional token from the required ones.
    fn finish(mut self) -> Self {
        derive_defaults(&mut self);
        self
    }

    /// Accent for a timer mode. Themeable per mode so tinting a break does not also
    /// recolour every warning in the app.
    pub fn mode_color(&self, mode: crate::model::TimerMode) -> Color {
        use crate::model::TimerMode;
        match mode {
            TimerMode::Focus => self.mode_focus,
            TimerMode::ShortBreak => self.mode_short_break,
            TimerMode::LongBreak => self.mode_long_break,
            TimerMode::Custom => self.mode_custom,
        }
    }

    pub fn dark() -> Self {
        Self {
            bg: Color::Rgb(15, 15, 20),
            text: Color::Rgb(225, 225, 230),
            dim: Color::Rgb(90, 90, 100),
            accent: Color::Rgb(100, 180, 255),
            on_accent: Color::Rgb(10, 10, 15),
            success: Color::Rgb(80, 210, 130),
            warning: Color::Rgb(245, 185, 70),
            error: Color::Rgb(245, 85, 85),
            info: Color::Rgb(170, 140, 250),
            progress_dim: Color::Rgb(45, 45, 55),
            task_track: Color::Rgb(35, 35, 42),
            panel: Color::Rgb(20, 22, 30),
            panel_border: Color::Rgb(55, 60, 75),
            select_bg: Color::Rgb(38, 52, 78),
            select_fg: Color::Rgb(230, 235, 245),
            active_bg: Color::Rgb(32, 48, 72),
            active_fg: Color::Rgb(170, 210, 255),
            ..PLACEHOLDER
        }
        .finish()
    }

    pub fn light() -> Self {
        Self {
            bg: Color::Rgb(250, 250, 252),
            text: Color::Rgb(30, 30, 35),
            dim: Color::Rgb(140, 140, 150),
            accent: Color::Rgb(25, 110, 200),
            on_accent: Color::Rgb(255, 255, 255),
            success: Color::Rgb(30, 150, 80),
            warning: Color::Rgb(200, 130, 20),
            error: Color::Rgb(200, 50, 50),
            info: Color::Rgb(110, 70, 190),
            progress_dim: Color::Rgb(200, 205, 215),
            task_track: Color::Rgb(220, 225, 232),
            panel: Color::Rgb(242, 245, 250),
            panel_border: Color::Rgb(190, 198, 210),
            select_bg: Color::Rgb(210, 225, 245),
            select_fg: Color::Rgb(20, 40, 70),
            active_bg: Color::Rgb(195, 218, 245),
            active_fg: Color::Rgb(15, 60, 120),
            ..PLACEHOLDER
        }
        .finish()
    }

    pub fn polaris() -> Self {
        Self {
            bg: Color::Rgb(10, 14, 30),
            text: Color::Rgb(215, 225, 250),
            dim: Color::Rgb(100, 120, 160),
            accent: Color::Rgb(90, 200, 255),
            on_accent: Color::Rgb(10, 14, 30),
            success: Color::Rgb(80, 235, 180),
            warning: Color::Rgb(255, 160, 75),
            error: Color::Rgb(255, 95, 120),
            info: Color::Rgb(180, 135, 255),
            progress_dim: Color::Rgb(40, 50, 80),
            task_track: Color::Rgb(25, 35, 60),
            panel: Color::Rgb(14, 20, 38),
            panel_border: Color::Rgb(55, 70, 110),
            select_bg: Color::Rgb(28, 42, 78),
            select_fg: Color::Rgb(210, 225, 255),
            active_bg: Color::Rgb(22, 38, 68),
            active_fg: Color::Rgb(140, 210, 255),
            ..PLACEHOLDER
        }
        .finish()
    }

    pub fn matrix() -> Self {
        Self {
            bg: Color::Rgb(3, 12, 5),
            text: Color::Rgb(150, 240, 140),
            dim: Color::Rgb(60, 110, 65),
            accent: Color::Rgb(80, 230, 90),
            on_accent: Color::Rgb(0, 0, 0),
            success: Color::Rgb(70, 220, 110),
            warning: Color::Rgb(220, 200, 60),
            error: Color::Rgb(255, 80, 90),
            info: Color::Rgb(100, 190, 240),
            progress_dim: Color::Rgb(15, 40, 18),
            task_track: Color::Rgb(8, 28, 10),
            panel: Color::Rgb(5, 16, 7),
            panel_border: Color::Rgb(35, 85, 40),
            select_bg: Color::Rgb(10, 32, 14),
            select_fg: Color::Rgb(160, 255, 165),
            active_bg: Color::Rgb(14, 42, 18),
            active_fg: Color::Rgb(110, 255, 120),
            ..PLACEHOLDER
        }
        .finish()
    }

    pub fn scene_style(&self, mode: Color) -> SceneStyle {
        SceneStyle {
            mode,
            track: self.progress_dim,
            task: self.success,
            task_dim: self.task_track,
            bg: self.bg,
            core: mode,
            glow: self.accent,
            particle: self.dim,
            text: self.text,
            session_on: self.accent,
            session_off: self.dim,
        }
    }
}

pub fn resolve(id: &str, catalog: &ThemeCatalog) -> Result<Theme> {
    if let Some(theme) = builtin_theme(id) {
        return Ok(theme);
    }

    let entry = catalog.resolve_entry(id)?;
    match &entry.source {
        ThemeSource::Builtin => builtin_theme(id).context("builtin theme missing"),
        ThemeSource::Embedded(source) => ThemeFile::from_str(source)?.into_theme(),
        ThemeSource::File(path) => ThemeFile::from_path(path)?.into_theme(),
    }
}

pub fn normalize_theme_id(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

fn mix(a: Color, b: Color, t: u8) -> Color {
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    let t = t as u16;
    let inv = 255 - t;
    Color::Rgb(
        ((ar as u16 * inv + br as u16 * t) / 255) as u8,
        ((ag as u16 * inv + bg as u16 * t) / 255) as u8,
        ((ab as u16 * inv + bb as u16 * t) / 255) as u8,
    )
}

fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

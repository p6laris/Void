//! UI icons with Nerd Font glyphs or ASCII fallbacks.
//!
//! Set `VOID_ICONS=nerd|ascii|auto` to override detection. `auto` defaults
//! to Nerd Fonts on all platforms.

use nerd_font_symbols::md;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconMode {
    Nerd,
    Ascii,
}

#[derive(Debug, Clone, Copy)]
pub struct IconSet {
    pub logo: &'static str,
    pub dashboard: &'static str,
    pub tasks: &'static str,
    pub stats: &'static str,
    pub settings: &'static str,
    pub help: &'static str,
    pub about: &'static str,
    pub play: &'static str,
    pub pause: &'static str,
    pub check: &'static str,
    pub idle: &'static str,
    pub timer: &'static str,
    pub fire: &'static str,
    pub target: &'static str,
    pub calendar: &'static str,
    pub chart: &'static str,
    pub cycle: &'static str,
    pub task_active: &'static str,
    pub task_todo: &'static str,
    pub task_progress: &'static str,
    pub task_done: &'static str,
    pub star: &'static str,
    pub alert: &'static str,
    pub plus: &'static str,
    pub delete: &'static str,
    pub edit: &'static str,
    pub export: &'static str,
    pub zen: &'static str,
    pub skip: &'static str,
    pub reset: &'static str,
    pub end: &'static str,
    pub chevron: &'static str,
    pub focus: &'static str,
    pub heart: &'static str,
    pub dot: &'static str,
    pub shield: &'static str,
    /// Heatmap day cell with activity.
    pub heat_cell: &'static str,
    /// Heatmap day cell with no activity (quiet dot/marker).
    pub heat_empty: &'static str,
    /// Heatmap cell for today, so the current day is findable without relying on colour.
    pub heat_today: &'static str,
    /// Heatmap cursor indicator for active keyboard navigation.
    pub heat_cursor: &'static str,
}

const NERD: IconSet = IconSet {
    logo: md::MD_DOTS_CIRCLE,
    dashboard: md::MD_VIEW_DASHBOARD,
    tasks: md::MD_FORMAT_LIST_BULLETED,
    stats: md::MD_CHART_LINE,
    settings: md::MD_COG,
    help: md::MD_HELP_CIRCLE,
    about: md::MD_INFORMATION,
    play: md::MD_PLAY,
    pause: md::MD_PAUSE,
    check: md::MD_CHECK_CIRCLE,
    idle: md::MD_TIMER_OUTLINE,
    timer: md::MD_TIMER,
    fire: md::MD_FIRE,
    target: md::MD_TARGET,
    calendar: md::MD_CALENDAR,
    chart: md::MD_CHART_BAR,
    cycle: md::MD_DOTS_HORIZONTAL,
    task_active: md::MD_PLAY_CIRCLE,
    task_todo: md::MD_CHECKBOX_BLANK_CIRCLE_OUTLINE,
    task_progress: md::MD_PROGRESS_CLOCK,
    task_done: md::MD_CHECK_CIRCLE,
    star: md::MD_STAR,
    alert: md::MD_CALENDAR_ALERT,
    plus: md::MD_PLUS,
    delete: md::MD_DELETE,
    edit: md::MD_PENCIL,
    export: md::MD_EXPORT,
    zen: md::MD_WEATHER_NIGHT,
    skip: md::MD_SKIP_NEXT,
    reset: md::MD_REFRESH,
    end: md::MD_STOP,
    chevron: md::MD_CHEVRON_RIGHT,
    focus: md::MD_CROSSHAIRS,
    heart: md::MD_HEART,
    dot: "·",
    shield: md::MD_SHIELD,
    heat_cell: "■",
    heat_empty: "·",
    heat_today: "▣",
    heat_cursor: "◈",
};

const ASCII: IconSet = IconSet {
    logo: "*",
    dashboard: "#",
    tasks: "T",
    stats: "S",
    settings: "G",
    help: "?",
    about: "i",
    play: ">",
    pause: "||",
    check: "+",
    idle: "-",
    timer: "t",
    fire: "^",
    target: "@",
    calendar: "C",
    chart: "=",
    cycle: "...",
    task_active: ">",
    task_todo: "o",
    task_progress: "~",
    task_done: "x",
    star: "*",
    alert: "!",
    plus: "+",
    delete: "X",
    edit: "E",
    export: "S",
    zen: "z",
    skip: ">>",
    reset: "R",
    end: "#",
    chevron: ">",
    focus: "*",
    heart: "<3",
    dot: ".",
    shield: "S",
    heat_cell: "#",
    heat_empty: ".",
    heat_today: "@",
    heat_cursor: "*",
};

impl IconSet {
    pub fn detect() -> Self {
        match std::env::var("VOID_ICONS")
            .ok()
            .map(|v| v.to_ascii_lowercase())
            .as_deref()
        {
            Some("nerd") | Some("nerd-font") => NERD,
            Some("ascii") | Some("text") => ASCII,
            Some("auto") | None => Self::detect_auto(),
            Some(other) => {
                eprintln!("void: unknown VOID_ICONS={other:?}, using auto");
                Self::detect_auto()
            }
        }
    }

    pub fn mode(self) -> IconMode {
        if self.logo == NERD.logo {
            IconMode::Nerd
        } else {
            IconMode::Ascii
        }
    }

    fn detect_auto() -> Self {
        Self::auto_from(|key| std::env::var(key).ok())
    }

    /// Picks ASCII for terminals that can't draw Nerd Font glyphs, Nerd otherwise.
    fn auto_from(env: impl Fn(&str) -> Option<String>) -> Self {
        if matches!(env("TERM").as_deref(), Some("linux") | Some("dumb")) {
            return ASCII;
        }
        // First set locale variable wins, as in POSIX; none set (usual on Windows) means UTF-8.
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"]
            .into_iter()
            .find_map(|key| env(key).filter(|v| !v.is_empty()));
        match locale {
            Some(value) if !value.to_ascii_lowercase().contains("utf") => ASCII,
            _ => NERD,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_set_uses_plain_characters() {
        assert_eq!(ASCII.play, ">");
        assert_eq!(ASCII.check, "+");
        assert_eq!(ASCII.task_done, "x");
    }

    #[test]
    fn nerd_set_uses_private_use_glyphs() {
        assert!(NERD.play.chars().any(|c| c as u32 >= 0xe000));
    }

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn auto_uses_ascii_on_the_linux_console_and_non_utf8_locales() {
        assert_eq!(
            IconSet::auto_from(env(&[("TERM", "linux")])).play,
            ASCII.play
        );
        assert_eq!(IconSet::auto_from(env(&[("LANG", "C")])).play, ASCII.play);
        assert_eq!(
            IconSet::auto_from(env(&[("LC_ALL", "C"), ("LANG", "en_US.UTF-8")])).play,
            ASCII.play
        );
    }

    #[test]
    fn auto_uses_nerd_for_utf8_or_no_locale() {
        assert_eq!(
            IconSet::auto_from(env(&[("LANG", "en_US.UTF-8")])).play,
            NERD.play
        );
        assert_eq!(IconSet::auto_from(env(&[])).play, NERD.play);
        assert_eq!(
            IconSet::auto_from(env(&[("TERM", "xterm-256color")])).play,
            NERD.play
        );
    }

    #[test]
    fn explicit_env_overrides_auto() {
        std::env::set_var("VOID_ICONS", "ascii");
        assert_eq!(IconSet::detect().play, ASCII.play);
        std::env::set_var("VOID_ICONS", "nerd");
        assert_eq!(IconSet::detect().play, NERD.play);
        std::env::remove_var("VOID_ICONS");
    }
}

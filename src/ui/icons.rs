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
}

const NERD: IconSet = IconSet {
    logo: md::MD_WEATHER_NIGHT,
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
        NERD
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

    #[test]
    fn explicit_env_overrides_auto() {
        std::env::set_var("VOID_ICONS", "ascii");
        assert_eq!(IconSet::detect().play, ASCII.play);
        std::env::set_var("VOID_ICONS", "nerd");
        assert_eq!(IconSet::detect().play, NERD.play);
        std::env::remove_var("VOID_ICONS");
    }
}

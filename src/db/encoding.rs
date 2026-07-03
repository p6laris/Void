use crate::model::TimerMode;

pub(crate) fn encode_timer_mode(m: TimerMode) -> &'static str {
    match m {
        TimerMode::Focus => "focus",
        TimerMode::ShortBreak => "shortbreak",
        TimerMode::LongBreak => "longbreak",
        TimerMode::Custom => "custom",
    }
}

pub(crate) fn decode_timer_mode(s: &str) -> TimerMode {
    match s {
        "shortbreak" | "short_break" => TimerMode::ShortBreak,
        "longbreak" | "long_break" => TimerMode::LongBreak,
        "custom" => TimerMode::Custom,
        _ => TimerMode::Focus,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timer_mode_round_trip() {
        for mode in [
            TimerMode::Focus,
            TimerMode::ShortBreak,
            TimerMode::LongBreak,
            TimerMode::Custom,
        ] {
            assert_eq!(decode_timer_mode(encode_timer_mode(mode)), mode);
        }
    }

    #[test]
    fn decode_timer_mode_accepts_legacy_underscore_forms() {
        assert_eq!(decode_timer_mode("short_break"), TimerMode::ShortBreak);
        assert_eq!(decode_timer_mode("long_break"), TimerMode::LongBreak);
    }
}

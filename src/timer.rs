use std::time::{Duration, Instant};

use crate::model::TimerMode;

#[derive(Debug, Clone, Copy)]
pub struct TimerConfig {
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub long_break_every: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
        }
    }
}

impl TimerConfig {
    pub fn from_app_data(data: &crate::model::AppData) -> Self {
        Self {
            focus_minutes: data.focus_minutes,
            short_break_minutes: data.short_break_minutes,
            long_break_minutes: data.long_break_minutes,
            long_break_every: data.long_break_every.max(1),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub mode: TimerMode,
    pub state: crate::model::TimerState,
    pub total_seconds: u32,
    pub elapsed_seconds: u32,
    pub started_at: Option<Instant>,
    pub completed_focus_sessions: u32,
    pub custom_minutes: u32,
    pub config: TimerConfig,
    pub session_pause_count: u32,
    pub session_pause_seconds: u32,
    pause_started_at: Option<Instant>,
}

impl Timer {
    pub fn new(config: TimerConfig) -> Self {
        let focus_secs = config.focus_minutes * 60;
        Self {
            mode: TimerMode::Focus,
            state: crate::model::TimerState::Idle,
            total_seconds: focus_secs,
            elapsed_seconds: 0,
            started_at: None,
            completed_focus_sessions: 0,
            custom_minutes: config.focus_minutes,
            config,
            session_pause_count: 0,
            session_pause_seconds: 0,
            pause_started_at: None,
        }
    }

    pub fn reset_session_pauses(&mut self) {
        self.session_pause_count = 0;
        self.session_pause_seconds = 0;
        self.pause_started_at = None;
    }

    pub fn session_meta(&self) -> crate::storage::SessionMeta {
        let mut pause_seconds = self.session_pause_seconds;
        if let Some(start) = self.pause_started_at {
            pause_seconds = pause_seconds.saturating_add(start.elapsed().as_secs() as u32);
        }
        crate::storage::SessionMeta {
            note: String::new(),
            tags: Vec::new(),
            pause_count: self.session_pause_count,
            pause_seconds,
        }
    }

    pub fn sync_config(&mut self, config: TimerConfig) {
        self.config = config;
        self.custom_minutes = config.focus_minutes;
        if self.state != crate::model::TimerState::Running {
            self.total_seconds = self.duration_seconds();
            if self.state == crate::model::TimerState::Idle {
                self.elapsed_seconds = 0;
            }
        }
    }

    pub fn duration_seconds(&self) -> u32 {
        match self.mode {
            TimerMode::Focus => self.config.focus_minutes * 60,
            TimerMode::ShortBreak => self.config.short_break_minutes * 60,
            TimerMode::LongBreak => self.config.long_break_minutes * 60,
            TimerMode::Custom => self.custom_minutes * 60,
        }
    }

    pub fn configure(&mut self, mode: TimerMode) {
        self.mode = mode;
        self.total_seconds = self.duration_seconds();
        self.elapsed_seconds = 0;
        self.state = crate::model::TimerState::Idle;
        self.started_at = None;
    }

    pub fn set_custom_minutes(&mut self, minutes: u32) {
        self.custom_minutes = minutes.clamp(1, 240);
        if self.mode == TimerMode::Custom && self.state != crate::model::TimerState::Running {
            self.total_seconds = self.custom_minutes * 60;
            self.elapsed_seconds = 0;
        }
    }

    pub fn set_focus_minutes(&mut self, minutes: u32) {
        let m = minutes.clamp(1, 240);
        self.config.focus_minutes = m;
        self.custom_minutes = m;
        if self.mode == TimerMode::Focus && self.state != crate::model::TimerState::Running {
            self.total_seconds = m * 60;
            self.elapsed_seconds = 0;
        }
    }

    pub fn current_elapsed_secs_f64(&self) -> f64 {
        if self.state == crate::model::TimerState::Running {
            if let Some(start) = self.started_at {
                return start.elapsed().as_secs_f64().min(self.total_seconds as f64);
            }
        }
        self.elapsed_seconds as f64
    }

    pub fn current_elapsed_seconds(&self) -> u32 {
        self.current_elapsed_secs_f64() as u32
    }

    pub fn start(&mut self) {
        if self.state == crate::model::TimerState::Running {
            return;
        }
        if self.state == crate::model::TimerState::Paused {
            if let Some(start) = self.pause_started_at.take() {
                self.session_pause_seconds = self
                    .session_pause_seconds
                    .saturating_add(start.elapsed().as_secs() as u32);
            }
        }
        if self.total_seconds == 0 {
            self.total_seconds = self.duration_seconds();
        }
        match self.state {
            crate::model::TimerState::Paused => {
                self.started_at =
                    Some(Instant::now() - Duration::from_secs(self.elapsed_seconds as u64));
            }
            crate::model::TimerState::Finished | crate::model::TimerState::Idle => {
                if self.state == crate::model::TimerState::Finished {
                    self.elapsed_seconds = 0;
                }
                self.started_at = Some(Instant::now());
            }
            _ => {}
        }
        self.state = crate::model::TimerState::Running;
    }

    pub fn pause(&mut self) {
        if self.state != crate::model::TimerState::Running {
            return;
        }
        self.session_pause_count = self.session_pause_count.saturating_add(1);
        self.pause_started_at = Some(Instant::now());
        self.elapsed_seconds = self.current_elapsed_seconds();
        self.started_at = None;
        self.state = crate::model::TimerState::Paused;
    }

    pub fn commit_pause_duration(&mut self) {
        if let Some(start) = self.pause_started_at.take() {
            self.session_pause_seconds = self
                .session_pause_seconds
                .saturating_add(start.elapsed().as_secs() as u32);
        }
    }

    pub fn reset(&mut self) {
        if let Some(start) = self.pause_started_at.take() {
            self.session_pause_seconds = self
                .session_pause_seconds
                .saturating_add(start.elapsed().as_secs() as u32);
        }
        self.state = crate::model::TimerState::Idle;
        self.elapsed_seconds = 0;
        self.started_at = None;
        self.total_seconds = self.duration_seconds();
        self.reset_session_pauses();
    }

    pub fn tick(&mut self) -> bool {
        if self.state != crate::model::TimerState::Running {
            return false;
        }
        let new_elapsed = self.current_elapsed_seconds();
        let just_finished =
            new_elapsed >= self.total_seconds && self.elapsed_seconds < self.total_seconds;
        self.elapsed_seconds = new_elapsed;
        if just_finished {
            self.state = crate::model::TimerState::Finished;
            if self.mode == TimerMode::Focus {
                self.completed_focus_sessions += 1;
            }
            return true;
        }
        false
    }

    pub fn skip(&mut self) {
        self.elapsed_seconds = self.current_elapsed_seconds().max(1);
        self.state = crate::model::TimerState::Finished;
        self.started_at = None;
    }

    pub fn remaining_seconds(&self) -> i32 {
        let elapsed = self.current_elapsed_seconds();
        self.total_seconds as i32 - elapsed as i32
    }

    pub fn is_one_minute_warning(&self) -> bool {
        self.state == crate::model::TimerState::Running && self.remaining_seconds() <= 60
    }

    pub fn progress(&self) -> f64 {
        if self.total_seconds == 0 {
            return 0.0;
        }
        (self.current_elapsed_secs_f64() / self.total_seconds as f64).clamp(0.0, 1.0)
    }

    pub fn remaining_secs_f64(&self) -> f64 {
        (self.total_seconds as f64 - self.current_elapsed_secs_f64()).max(0.0)
    }

    pub fn format_remaining(&self) -> String {
        self.format_remaining_parts().0
    }

    pub fn format_remaining_parts(&self) -> (String, String) {
        let rem = self.remaining_secs_f64();
        let h = (rem / 3600.0) as u32;
        let m = ((rem % 3600.0) / 60.0) as u32;
        let s = rem % 60.0;
        let main = if h > 0 {
            format!("{:02}:{:02}:{:02}", h, m, s as u32)
        } else {
            format!("{:02}:{:02}", m, s as u32)
        };
        let tenths = format!(".{}", (s * 10.0) as u32 % 10);
        (main, tenths)
    }

    pub fn session_in_cycle(&self) -> u32 {
        if self.config.long_break_every == 0 {
            return 1;
        }
        (self.completed_focus_sessions % self.config.long_break_every) + 1
    }

    pub fn focus_sessions_in_cycle(&self) -> u32 {
        let cycle = self.config.long_break_every.max(1);
        let done = self.completed_focus_sessions;
        if done > 0 && done.is_multiple_of(cycle) {
            cycle
        } else {
            done % cycle
        }
    }

    pub fn cycle_label(&self) -> String {
        let cycle = self.config.long_break_every.max(1);
        match self.mode {
            TimerMode::Focus => format!("Focus {} of {}", self.session_in_cycle(), cycle),
            TimerMode::ShortBreak => format!(
                "Short break · {}/{} focus done",
                self.focus_sessions_in_cycle(),
                cycle
            ),
            TimerMode::LongBreak => format!("Long break · {cycle} focus sessions done"),
            TimerMode::Custom => "Custom session".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TimerState;

    #[test]
    fn test_timer_initialization() {
        let config = TimerConfig {
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
        };
        let timer = Timer::new(config);
        assert_eq!(timer.mode, TimerMode::Focus);
        assert_eq!(timer.state, TimerState::Idle);
        assert_eq!(timer.total_seconds, 25 * 60);
        assert_eq!(timer.elapsed_seconds, 0);
    }

    #[test]
    fn test_timer_start_pause_resume() {
        let config = TimerConfig::default();
        let mut timer = Timer::new(config);

        timer.start();
        assert_eq!(timer.state, TimerState::Running);
        assert!(timer.started_at.is_some());

        timer.pause();
        assert_eq!(timer.state, TimerState::Paused);
        assert!(timer.pause_started_at.is_some());
        assert_eq!(timer.session_pause_count, 1);

        timer.start();
        assert_eq!(timer.state, TimerState::Running);
        assert!(timer.pause_started_at.is_none());
    }

    #[test]
    fn test_timer_tick_completion() {
        let config = TimerConfig {
            focus_minutes: 1,
            ..TimerConfig::default()
        };
        let mut timer = Timer::new(config);

        timer.start();
        // forcefully set started_at in the past
        timer.started_at = Some(Instant::now() - Duration::from_secs(65));

        let finished = timer.tick();
        assert!(finished);
        assert_eq!(timer.state, TimerState::Finished);
        assert_eq!(timer.completed_focus_sessions, 1);
    }

    #[test]
    fn test_timer_skip() {
        let config = TimerConfig::default();
        let mut timer = Timer::new(config);

        timer.start();
        timer.skip();

        assert_eq!(timer.state, TimerState::Finished);
        assert!(timer.elapsed_seconds > 0);
    }
}

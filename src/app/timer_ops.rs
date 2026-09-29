use super::*;
use crate::model::TimerMode;
use std::time::SystemTime;

/// Focus time below this is discarded rather than logged as a session.
const MIN_LOGGED_SECS: u32 = 60;

/// A wall-clock gap between ticks longer than this is treated as system sleep.
const SLEEP_GAP: Duration = Duration::from_secs(60);

impl App {
    /// True while a focus or custom session holds enough time to be worth logging.
    pub(crate) fn has_loggable_focus(&self) -> bool {
        matches!(self.timer.mode, TimerMode::Focus | TimerMode::Custom)
            && matches!(self.timer.state, TimerState::Running | TimerState::Paused)
            && self.timer.current_elapsed_seconds() >= MIN_LOGGED_SECS
    }

    /// Records the in-progress focus session and leaves the timer idle in Focus.
    pub(crate) fn log_partial_focus(&mut self) -> u32 {
        let mode = self.timer.mode;
        let mins = self.elapsed_minutes();
        let task_id = self.task_ui.active_task;
        let meta = self.timer.session_meta();
        self.persist_data(|db, data| {
            storage::record_focus_session_with_meta(db, data, mins, task_id, mode, meta)
        });
        self.maybe_complete_task_estimate(task_id);
        self.bump_data();
        self.timer.configure(TimerMode::Focus);
        self.timer.reset_session_pauses();
        self.persist_timer_state();
        mins
    }

    pub fn skip_session(&mut self) {
        let in_progress = matches!(self.timer.state, TimerState::Running | TimerState::Paused);
        match self.timer.mode {
            TimerMode::Focus | TimerMode::Custom if !in_progress => {
                self.set_status("Nothing to skip — start a session first.", false);
            }
            TimerMode::Focus | TimerMode::Custom if !self.has_loggable_focus() => {
                self.timer.reset();
                if self.timer.mode == TimerMode::Focus {
                    self.advance_to_break();
                } else {
                    self.timer.configure(TimerMode::Focus);
                }
                self.set_status("Skipped — under a minute, not logged.", false);
            }
            TimerMode::ShortBreak | TimerMode::LongBreak if !in_progress => {
                self.set_status("Break skipped.", false);
                self.advance_to_focus();
            }
            _ => {
                self.timer.skip();
                self.on_timer_finished(true);
            }
        }
    }

    /// Asks before quitting when a session with loggable time is in progress.
    pub fn request_quit(&mut self) {
        if self.has_loggable_focus() {
            self.input.popup = Some(Popup::ConfirmQuit);
        } else {
            self.ui.should_quit = true;
        }
    }

    /// Ctrl-C quits immediately, but still saves a session worth logging.
    pub fn force_quit(&mut self) {
        if self.has_loggable_focus() {
            self.log_partial_focus();
        }
        self.ui.should_quit = true;
    }

    /// Pauses at the last pre-sleep position when the wall clock jumps; call before `tick()`.
    pub(crate) fn handle_wall_clock_gap(&mut self, now: SystemTime) {
        let prev = self.last_tick_wall.replace(now);
        let Some(gap) = prev.and_then(|p| now.duration_since(p).ok()) else {
            return;
        };
        if gap > SLEEP_GAP && self.timer.state == TimerState::Running {
            self.timer.pause_after_gap(gap.as_secs() as u32);
            self.set_status(
                format!("Paused — system was asleep for {} min.", gap.as_secs() / 60),
                false,
            );
        }
    }

    pub(crate) fn maybe_complete_task_estimate(&mut self, task_id: Option<u64>) {
        let Some(id) = task_id else {
            return;
        };
        let estimated = self
            .data
            .task(id)
            .map(|t| (t.title.clone(), t.actual_minutes, t.estimated_minutes));
        let Some((title, actual, estimate)) = estimated else {
            return;
        };
        if actual < estimate {
            return;
        }
        match self.data.estimate_complete {
            EstimateCompleteBehavior::Nudge => {
                self.set_status(
                    format!("Estimate reached for \"{title}\" — mark done?"),
                    false,
                );
            }
            EstimateCompleteBehavior::AutoDone => {
                self.persist_data(|db, data| storage::mark_task_done(db, data, id));
                if self.task_ui.active_task == Some(id) {
                    self.task_ui.active_task = None;
                    self.data.active_task_id = None;
                    self.persist(|db| db.persist_active_task(None));
                }
                self.bump_tasks();
                if self.data.sound_enabled {
                    sound::play_task_complete();
                }
                self.set_status(format!("\"{title}\" auto-completed (estimate met)."), false);
                self.check_queue_empty();
            }
            EstimateCompleteBehavior::None => {}
        }
    }

    pub fn end_session(&mut self) {
        let in_progress = matches!(self.timer.state, TimerState::Running | TimerState::Paused);
        let logged = if self.has_loggable_focus() {
            Some(self.log_partial_focus())
        } else {
            if in_progress {
                self.timer.configure(TimerMode::Focus);
                self.timer.reset_session_pauses();
            }
            None
        };
        let logged_note = match logged {
            Some(mins) => format!("+{mins} min logged"),
            None if in_progress => "under a minute, not logged".to_string(),
            None => "nothing running".to_string(),
        };
        let today = storage::today_focus_minutes(&self.data);
        let goal = self.data.daily_goal_minutes;
        let queue_note = if self.queue_empty() {
            "all tasks done"
        } else {
            "tasks remain"
        };
        self.set_status(
            format!(
                "Session ended ({logged_note}) — today {}/{} min · goal streak {} days · {queue_note}",
                today, goal, self.data.goal_streak_days
            ),
            false,
        );
    }

    pub fn on_tick(&mut self) {
        let minute = chrono::Utc::now().timestamp() / 60;
        let check_rollover = self.last_rollover_minute.replace(minute) != Some(minute);
        if check_rollover && storage::ensure_today_reset(&self.db, &mut self.data).unwrap_or(false)
        {
            if !matches!(self.timer.state, TimerState::Running | TimerState::Paused) {
                self.timer.completed_focus_sessions = 0;
            }
            self.persist_timer_state();
            self.stats.chart_dirty = true;
            self.refresh_frame_today_cache();
            self.recompute_task_caches();
        }
        if self.data.auto_pause_idle_minutes > 0
            && self.timer.state == TimerState::Running
            && matches!(self.timer.mode, TimerMode::Focus | TimerMode::Custom)
            && self.last_activity.elapsed()
                > Duration::from_secs(self.data.auto_pause_idle_minutes as u64 * 60)
        {
            self.pause_timer();
            self.set_status("Auto-paused — terminal idle.", false);
        }
        if self.data.warn_one_minute
            && matches!(self.timer.mode, TimerMode::Focus | TimerMode::Custom)
            && self.timer.is_one_minute_warning()
            && self.warned_session != self.timer.session_started_at
        {
            self.warned_session = self.timer.session_started_at;
            if self.data.sound_enabled {
                sound::play_warning();
            }
            self.set_status("1 minute remaining!", false);
        }
        self.handle_wall_clock_gap(SystemTime::now());
        let just_finished = self.timer.tick();
        if just_finished {
            self.on_timer_finished(false);
        }
        if self.ui.status.is_some()
            && !self.ui.status_error
            && self.ui.last_status_set.elapsed() > Duration::from_secs(4)
        {
            self.ui.status = None;
        }
    }

    pub(crate) fn on_timer_finished(&mut self, skipped: bool) {
        let mode = self.timer.mode;
        if mode == TimerMode::Focus {
            let mins = self.elapsed_minutes();
            let task_id = self.task_ui.active_task;
            let meta = self.timer.session_meta();
            self.persist_data(|db, data| {
                storage::record_focus_session_with_meta(db, data, mins, task_id, mode, meta)
            });
            // A skip that covered at least half the session still counts toward the long break.
            if skipped && mins * 2 >= self.timer.config.focus_minutes {
                self.timer.completed_focus_sessions += 1;
            }
            if self.data.sound_enabled {
                if skipped {
                    sound::play_skip();
                } else {
                    sound::play_focus_complete();
                }
            }
            if self.data.notify_on_finish {
                let msg = if skipped {
                    format!("Logged {} min (skipped early)", mins)
                } else {
                    format!("+{} min logged — time for a break", mins)
                };
                let kind = if skipped {
                    sound::NotifyKind::SessionSkipped
                } else {
                    sound::NotifyKind::FocusComplete
                };
                sound::notify_typed(kind, "Void · Focus complete", &msg);
            }
            self.set_status(
                format!(
                    "Focus {}: +{} min",
                    if skipped { "skipped" } else { "complete" },
                    mins
                ),
                false,
            );
            self.maybe_advance_task();
            self.bump_data();
            self.timer.reset_session_pauses();
            self.advance_to_break();
            // Last, so an estimate nudge isn't overwritten by the messages above.
            self.maybe_complete_task_estimate(task_id);
        } else if mode == TimerMode::Custom {
            let mins = self.elapsed_minutes();
            let task_id = self.task_ui.active_task;
            let meta = self.timer.session_meta();
            self.persist_data(|db, data| {
                storage::record_focus_session_with_meta(db, data, mins, task_id, mode, meta)
            });
            if self.data.sound_enabled {
                if skipped {
                    sound::play_skip();
                } else {
                    sound::play_focus_complete();
                }
            }
            self.set_status(format!("Custom session complete: +{} min", mins), false);
            self.bump_data();
            self.timer.configure(TimerMode::Focus);
            self.timer.reset_session_pauses();
            self.persist_timer_state();
        } else {
            let break_mins = self.elapsed_minutes();
            self.persist_data(|db, data| storage::record_break_session(db, data, mode, break_mins));
            if self.data.sound_enabled {
                sound::play_break_complete();
            }
            if self.data.notify_on_finish {
                sound::notify_typed(
                    sound::NotifyKind::BreakComplete,
                    "Void · Break over",
                    "Break finished — ready to focus again",
                );
            }
            self.set_status("Break finished. Ready for focus.", false);
            self.bump_data();
            self.advance_to_focus();
        }
    }

    pub(crate) fn advance_to_break(&mut self) {
        let long_break = self.timer.completed_focus_sessions > 0
            && self
                .timer
                .completed_focus_sessions
                .is_multiple_of(self.timer.config.long_break_every);
        let next = if long_break {
            TimerMode::LongBreak
        } else {
            TimerMode::ShortBreak
        };
        self.timer.configure(next);
        self.persist_timer_state();
        if self.data.auto_start_breaks {
            self.timer.start();
            if self.data.sound_enabled {
                sound::play_start();
            }
            self.set_status(format!("{} started.", next.label()), false);
        }
    }

    pub(crate) fn advance_to_focus(&mut self) {
        self.timer.configure(TimerMode::Focus);
        self.persist_timer_state();
        if self.queue_empty() && self.data.empty_queue_behavior == EmptyQueueBehavior::PauseTimer {
            self.set_status("All tasks done — timer waiting. [E] end session", false);
            return;
        }
        self.auto_pick_task_if_needed();
        if self.data.auto_start_focus {
            self.timer.start();
            if self.data.sound_enabled {
                sound::play_start();
            }
            self.set_status("Focus started.", false);
        }
    }

    pub fn toggle_timer(&mut self) {
        if self.timer.state == TimerState::Running {
            self.pause_timer();
        } else {
            self.start_timer();
        }
    }

    pub fn start_timer(&mut self) {
        if self.timer.state == TimerState::Running {
            return;
        }
        if self.timer.state == TimerState::Finished {
            self.timer.reset();
        }
        if self.timer.mode == TimerMode::Focus {
            self.auto_pick_task_if_needed();
        }
        let is_resume = self.timer.current_elapsed_seconds() > 0;
        self.timer.start();
        if self.data.sound_enabled {
            if is_resume {
                sound::play_resume();
            } else {
                sound::play_start();
            }
        }
        self.set_status("Timer started.", false);
    }

    pub fn pause_timer(&mut self) {
        if self.timer.state != TimerState::Running {
            return;
        }
        let elapsed = self.timer.current_elapsed_seconds();
        self.timer.pause();
        if self.data.sound_enabled {
            sound::play_pause();
        }
        let active_minutes = (elapsed / 60).max(1);
        self.set_status(
            format!(
                "Paused at {} ({} min in).",
                self.timer.format_remaining(),
                active_minutes
            ),
            false,
        );
    }

    pub fn reset_timer(&mut self) {
        self.timer.reset();
        self.set_status("Timer reset.", false);
    }

    pub fn cycle_mode(&mut self) {
        if self.timer.state == TimerState::Running || self.timer.state == TimerState::Paused {
            self.set_status("Stop the timer before changing mode.", true);
            return;
        }
        let next = match self.timer.mode {
            TimerMode::Custom => TimerMode::Focus,
            _ => TimerMode::Custom,
        };
        self.timer.configure(next);
        self.set_status(format!("Mode: {}", next.label()), false);
    }

    pub fn cycle_timer_preset(&mut self) {
        if self.timer.state == TimerState::Running || self.timer.state == TimerState::Paused {
            self.set_status("Stop the timer before switching preset.", true);
            return;
        }
        if let Some(preset) = storage::cycle_timer_preset(&mut self.data) {
            self.timer
                .sync_config(TimerConfig::from_app_data(&self.data));
            if let Err(e) = self.db.persist_timer_settings(&self.data) {
                self.set_status(format!("Save error: {e}"), true);
            }
            self.persist_setting(
                "active_preset",
                self.data.active_preset.clone().unwrap_or_default(),
            );
            self.set_status(format!("Preset: {}", preset.name), false);
        }
    }

    pub fn adjust_minutes(&mut self, delta: i32) {
        if self.timer.state == TimerState::Running || self.timer.state == TimerState::Paused {
            self.set_status("Stop the timer before adjusting duration.", true);
            return;
        }
        match self.timer.mode {
            TimerMode::Focus => {
                let cur = self.timer.config.focus_minutes as i32 + delta;
                let v = cur.clamp(1, 240) as u32;
                self.timer.set_focus_minutes(v);
                self.data.focus_minutes = v;
            }
            TimerMode::ShortBreak => {
                let cur = self.timer.config.short_break_minutes as i32 + delta;
                let v = cur.clamp(1, 60) as u32;
                self.timer.set_short_break_minutes(v);
                self.data.short_break_minutes = v;
            }
            TimerMode::LongBreak => {
                let cur = self.timer.config.long_break_minutes as i32 + delta;
                let v = cur.clamp(1, 120) as u32;
                self.timer.set_long_break_minutes(v);
                self.data.long_break_minutes = v;
            }
            TimerMode::Custom => {
                let cur = self.timer.custom_minutes as i32 + delta;
                let v = cur.clamp(1, 240) as u32;
                self.timer.set_custom_minutes(v);
            }
        }
        if let Err(e) = self.db.persist_timer_settings(&self.data) {
            self.set_status(format!("Save error: {e}"), true);
        }
        self.set_status(
            format!(
                "{} length: {} min",
                self.timer.mode.label(),
                self.timer.duration_seconds() / 60
            ),
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn app() -> App {
        App::with_database(Database::open_in_memory().unwrap()).unwrap()
    }

    /// Puts a focus session `secs` in, paused so elapsed time is exact.
    fn paused_focus(app: &mut App, secs: u32) {
        app.timer.configure(TimerMode::Focus);
        app.timer.elapsed_seconds = secs;
        app.timer.state = TimerState::Paused;
    }

    #[test]
    fn skip_on_an_idle_focus_timer_logs_nothing() {
        let mut app = app();
        app.skip_session();
        assert_eq!(app.data.total_sessions, 0);
        assert_eq!(app.data.streak_days, 0);
        assert_eq!(app.timer.mode, TimerMode::Focus);
    }

    #[test]
    fn skip_under_a_minute_moves_on_without_logging() {
        let mut app = app();
        paused_focus(&mut app, 30);
        app.skip_session();
        assert_eq!(app.data.total_sessions, 0);
        assert_eq!(app.timer.mode, TimerMode::ShortBreak);
    }

    #[test]
    fn skip_after_ten_minutes_logs_ten() {
        let mut app = app();
        paused_focus(&mut app, 10 * 60 + 40);
        app.skip_session();
        assert_eq!(app.data.total_sessions, 1);
        assert_eq!(app.data.total_focus_minutes, 10);
    }

    #[test]
    fn skipping_an_idle_break_returns_to_focus() {
        let mut app = app();
        app.timer.configure(TimerMode::ShortBreak);
        app.skip_session();
        assert_eq!(app.timer.mode, TimerMode::Focus);
    }

    #[test]
    fn end_session_logs_the_partial_session_and_idles() {
        let mut app = app();
        paused_focus(&mut app, 12 * 60);
        app.end_session();
        assert_eq!(app.data.total_sessions, 1);
        assert_eq!(app.data.total_focus_minutes, 12);
        assert_eq!(app.timer.state, TimerState::Idle);
        assert_eq!(app.timer.mode, TimerMode::Focus);
    }

    #[test]
    fn end_session_under_a_minute_discards() {
        let mut app = app();
        paused_focus(&mut app, 20);
        app.end_session();
        assert_eq!(app.data.total_sessions, 0);
        assert_eq!(app.timer.state, TimerState::Idle);
    }

    #[test]
    fn quitting_mid_session_asks_first() {
        let mut app = app();
        paused_focus(&mut app, 5 * 60);
        app.request_quit();
        assert!(!app.ui.should_quit);
        assert!(matches!(app.input.popup, Some(Popup::ConfirmQuit)));
    }

    #[test]
    fn quitting_with_nothing_to_log_is_immediate() {
        let mut app = app();
        app.request_quit();
        assert!(app.ui.should_quit);
        assert!(app.input.popup.is_none());
    }

    #[test]
    fn force_quit_saves_the_session() {
        let mut app = app();
        paused_focus(&mut app, 8 * 60);
        app.force_quit();
        assert!(app.ui.should_quit);
        assert_eq!(app.data.total_focus_minutes, 8);
    }

    #[test]
    fn a_wall_clock_jump_pauses_without_crediting_the_gap() {
        let mut app = app();
        app.timer.configure(TimerMode::Focus);
        app.timer.start();
        app.timer.elapsed_seconds = 300;
        let now = SystemTime::now();
        app.last_tick_wall = Some(now - Duration::from_secs(2 * 3600));

        app.handle_wall_clock_gap(now);

        assert_eq!(app.timer.state, TimerState::Paused);
        assert_eq!(app.timer.elapsed_seconds, 300);
        assert!(app.timer.session_pause_seconds >= 2 * 3600);
    }

    #[test]
    fn a_normal_tick_interval_does_not_pause() {
        let mut app = app();
        app.timer.configure(TimerMode::Focus);
        app.timer.start();
        let now = SystemTime::now();
        app.last_tick_wall = Some(now - Duration::from_millis(150));
        app.handle_wall_clock_gap(now);
        assert_eq!(app.timer.state, TimerState::Running);
    }

    #[test]
    fn a_long_skip_counts_toward_the_long_break() {
        let mut app = app();
        paused_focus(&mut app, 13 * 60);
        app.skip_session();
        assert_eq!(app.timer.completed_focus_sessions, 1);
    }

    #[test]
    fn a_short_skip_does_not_count_toward_the_long_break() {
        let mut app = app();
        paused_focus(&mut app, 5 * 60);
        app.skip_session();
        assert_eq!(app.timer.completed_focus_sessions, 0);
    }

    #[test]
    fn a_session_that_crosses_midnight_is_dated_by_its_start() {
        let mut app = app();
        let today = crate::date::today_naive();
        let yesterday = crate::date::format_date(today - chrono::Duration::days(1));
        paused_focus(&mut app, 25 * 60);
        app.timer.session_started_at = Some(chrono::Utc::now() - chrono::Duration::days(1));

        app.end_session();

        assert_eq!(
            app.data.last_session_date.as_deref(),
            Some(yesterday.as_str())
        );
        assert_eq!(app.data.today_focus_minutes, 0);
        assert_eq!(app.data.total_focus_minutes, 25);
    }

    #[test]
    fn midnight_keeps_the_cycle_count_of_a_running_session() {
        let mut app = app();
        app.data.today_date = Some("2020-01-01".into());
        app.timer.completed_focus_sessions = 3;
        app.timer.configure(TimerMode::Focus);
        app.timer.start();
        app.on_tick();
        assert_eq!(app.timer.completed_focus_sessions, 3);
    }

    #[test]
    fn a_clock_set_back_keeps_todays_minutes() {
        let mut app = app();
        let tomorrow = crate::date::today_naive() + chrono::Duration::days(1);
        app.data.today_date = Some(crate::date::format_date(tomorrow));
        app.data.today_focus_minutes = 90;
        let new_day = storage::ensure_today_reset(&app.db, &mut app.data).unwrap();
        assert!(!new_day);
        assert_eq!(app.data.today_focus_minutes, 90);
    }

    #[test]
    fn the_estimate_nudge_is_the_last_status_after_a_session() {
        let mut app = app();
        app.data.auto_start_breaks = true;
        let id = storage::add_task_full(
            &app.db,
            &mut app.data,
            storage::TaskPayload {
                title: "Tiny".into(),
                notes: String::new(),
                estimated_minutes: 10,
                priority: crate::model::Priority::Medium,
                tags: Vec::new(),
                due_date: None,
            },
        )
        .unwrap();
        app.set_active_task(Some(id));
        paused_focus(&mut app, 12 * 60);
        app.skip_session();
        assert!(app
            .ui
            .status
            .as_deref()
            .unwrap_or("")
            .contains("Estimate reached"));
    }

    #[test]
    fn the_one_minute_warning_fires_once_per_session() {
        let mut app = app();
        app.data.warn_one_minute = true;
        app.data.sound_enabled = false;
        app.timer.configure(TimerMode::Focus);
        app.timer.start();
        let near_end = app.timer.total_seconds - 30;
        app.timer.started_at = Some(Instant::now() - Duration::from_secs(near_end as u64));
        app.on_tick();
        assert_eq!(app.ui.status.as_deref(), Some("1 minute remaining!"));

        app.timer.pause();
        app.timer.start();
        app.set_status("resumed", false);
        app.on_tick();
        assert_eq!(app.ui.status.as_deref(), Some("resumed"));
    }

    #[test]
    fn breaks_get_no_warning_and_are_not_idle_paused() {
        let mut app = app();
        app.data.warn_one_minute = true;
        app.data.sound_enabled = false;
        app.data.auto_pause_idle_minutes = 1;
        app.last_activity = Instant::now() - Duration::from_secs(10 * 60);
        app.timer.configure(TimerMode::ShortBreak);
        app.timer.start();
        let near_end = app.timer.total_seconds - 30;
        app.timer.started_at = Some(Instant::now() - Duration::from_secs(near_end as u64));
        app.on_tick();
        assert_eq!(app.timer.state, TimerState::Running);
        assert_ne!(app.ui.status.as_deref(), Some("1 minute remaining!"));
    }

    #[test]
    fn ticks_are_slow_unless_something_on_screen_moves() {
        let mut app = app();
        app.data.canvas_mode = crate::model::CanvasMode::Off;
        app.ui.tab = FocusTab::Settings;
        assert_eq!(app.tick_rate(), Duration::from_secs(1));

        app.timer.start();
        assert_eq!(app.tick_rate(), Duration::from_millis(250));
        app.ui.tab = FocusTab::Dashboard;
        assert_eq!(app.tick_rate(), Duration::from_millis(100));

        app.ui.focused = false;
        assert_eq!(app.tick_rate(), Duration::from_millis(500));
    }

    #[test]
    fn the_frame_signature_only_changes_with_what_is_shown() {
        let mut app = app();
        app.ui.tab = FocusTab::Settings;
        app.timer.start();
        app.timer.started_at = Some(Instant::now() - Duration::from_millis(200));
        let before = app.frame_signature();
        app.timer.started_at = Some(Instant::now() - Duration::from_millis(600));
        assert_eq!(
            app.frame_signature(),
            before,
            "sub-second change off the dashboard"
        );

        app.set_status("hello", false);
        assert_ne!(app.frame_signature(), before);
    }

    #[test]
    fn the_day_rollover_is_checked_once_a_minute() {
        let mut app = app();
        app.on_tick();
        app.data.today_date = Some("2020-01-01".into());
        app.data.today_focus_minutes = 50;
        app.on_tick();
        assert_eq!(
            app.data.today_focus_minutes, 50,
            "checked again within the minute"
        );

        app.last_rollover_minute = None;
        app.on_tick();
        assert_eq!(app.data.today_focus_minutes, 0);
    }

    #[test]
    fn on_tick_resets_metrics_and_marks_dirty_on_midnight_rollover() {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::with_database(db).unwrap();

        // Simulate yesterday's state
        app.data.today_date = Some("2020-01-01".into());
        app.data.today_focus_minutes = 120;
        app.timer.completed_focus_sessions = 4;
        app.stats.chart_dirty = false;

        // Run tick (which detects today != 2020-01-01)
        app.on_tick();

        assert_eq!(app.data.today_focus_minutes, 0);
        assert_eq!(app.today_focus_mins(), 0);
        assert_eq!(app.timer.completed_focus_sessions, 0);
        assert!(app.stats.chart_dirty);
        assert_eq!(
            app.data.today_date.as_deref(),
            Some(crate::date::today_str().as_str())
        );
    }
}

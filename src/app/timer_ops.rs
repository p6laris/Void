use super::*;
use crate::model::TimerMode;

impl App {
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
                if self.active_task == Some(id) {
                    self.active_task = None;
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
        if self.timer.state == TimerState::Running {
            self.pause_timer();
        }
        let today = storage::today_focus_minutes(&self.data);
        let goal = self.data.daily_goal_minutes;
        let queue_note = if self.queue_empty() {
            "all tasks done"
        } else {
            "tasks remain"
        };
        self.set_status(
            format!(
                "Session ended — today {}/{} min · goal streak {} days · {queue_note}",
                today, goal, self.data.goal_streak_days
            ),
            false,
        );
    }

    pub fn on_tick(&mut self) {
        if let Ok(true) = storage::ensure_today_reset(&self.db, &mut self.data) {
            self.timer.completed_focus_sessions = 0;
            self.persist_timer_state();
        }
        if self.data.auto_pause_idle_minutes > 0
            && self.timer.state == TimerState::Running
            && self.last_activity.elapsed()
                > Duration::from_secs(self.data.auto_pause_idle_minutes as u64 * 60)
        {
            self.pause_timer();
            self.set_status("Auto-paused — terminal idle.", false);
        }
        if self.data.warn_one_minute
            && self.timer.is_one_minute_warning()
            && !self.end_warning_shown
        {
            self.end_warning_shown = true;
            if self.data.sound_enabled {
                sound::play_warning();
            }
            self.set_status("1 minute remaining!", false);
        }
        if !self.timer.is_one_minute_warning() {
            self.end_warning_shown = false;
        }
        let just_finished = self.timer.tick();
        if just_finished {
            self.on_timer_finished(false);
        }
        if self.status.is_some()
            && !self.status_error
            && self.last_status_set.elapsed() > Duration::from_secs(4)
        {
            self.status = None;
        }
    }

    pub(crate) fn on_timer_finished(&mut self, skipped: bool) {
        let mode = self.timer.mode;
        if mode == TimerMode::Focus {
            let mins = self.elapsed_minutes(skipped);
            let task_id = self.active_task;
            let meta = self.timer.session_meta();
            self.persist_data(|db, data| {
                storage::record_focus_session_with_meta(db, data, mins, task_id, mode, meta)
            });
            self.maybe_complete_task_estimate(task_id);
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
            if !skipped {
                self.persist_timer_state();
            }
            self.timer.reset_session_pauses();
            self.advance_to_break();
        } else if mode == TimerMode::Custom {
            let mins = self.elapsed_minutes(skipped);
            let task_id = self.active_task;
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
            let break_mins = self.elapsed_minutes(false);
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
        self.end_warning_shown = false;
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
            TimerMode::Focus => TimerMode::ShortBreak,
            TimerMode::ShortBreak => TimerMode::LongBreak,
            TimerMode::LongBreak => TimerMode::Custom,
            TimerMode::Custom => TimerMode::Focus,
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
                self.timer.config.short_break_minutes = v;
                self.data.short_break_minutes = v;
                self.timer.total_seconds = self.timer.duration_seconds();
            }
            TimerMode::LongBreak => {
                let cur = self.timer.config.long_break_minutes as i32 + delta;
                let v = cur.clamp(1, 120) as u32;
                self.timer.config.long_break_minutes = v;
                self.data.long_break_minutes = v;
                self.timer.total_seconds = self.timer.duration_seconds();
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

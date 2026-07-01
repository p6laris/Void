use super::*;
use crate::model::{EmptyQueueBehavior, EstimateCompleteBehavior};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    FocusMinutes,
    ShortBreak,
    LongBreak,
    LongBreakEvery,
    DailyGoal,
    Sound,
    Notifications,
    AutoStartBreaks,
    AutoStartFocus,
    ActiveTaskCycle,
    Theme,
    CustomMinutes,
    AutoPickTask,
    AutoAdvanceTask,
    EmptyQueueBehavior,
    LogBreaks,
    EstimateComplete,
    ExportBackup,
    TerminalTitle,
    WarnOneMinute,
    AutoPauseIdle,
    ArchiveAfterDays,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct NumericSettingSpec<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub min: u32,
    pub max: u32,
    pub step: i32,
}

#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    pub selected: usize,
    pub scroll_offset: usize,
    /// Visible rows in the settings table (updated each draw).
    pub page_size: usize,
    pub items: Vec<SettingsItem>,
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            selected: 0,
            scroll_offset: 0,
            page_size: 12,
            items: vec![
                SettingsItem::FocusMinutes,
                SettingsItem::ShortBreak,
                SettingsItem::LongBreak,
                SettingsItem::LongBreakEvery,
                SettingsItem::DailyGoal,
                SettingsItem::Sound,
                SettingsItem::Notifications,
                SettingsItem::AutoStartBreaks,
                SettingsItem::AutoStartFocus,
                SettingsItem::ActiveTaskCycle,
                SettingsItem::Theme,
                SettingsItem::CustomMinutes,
                SettingsItem::AutoPickTask,
                SettingsItem::AutoAdvanceTask,
                SettingsItem::EmptyQueueBehavior,
                SettingsItem::LogBreaks,
                SettingsItem::EstimateComplete,
                SettingsItem::TerminalTitle,
                SettingsItem::WarnOneMinute,
                SettingsItem::AutoPauseIdle,
                SettingsItem::ArchiveAfterDays,
                SettingsItem::ExportBackup,
            ],
        }
    }
}

impl App {
    pub(crate) fn adjust_numeric_setting<F>(
        &mut self,
        dir: i32,
        spec: NumericSettingSpec<'_>,
        mut getter: F,
    ) where
        F: FnMut(&mut crate::model::AppData) -> &mut u32,
    {
        let val = getter(&mut self.data);
        let cur = *val as i32;
        let new_val = (cur + dir * spec.step).clamp(spec.min as i32, spec.max as i32) as u32;
        *val = new_val;
        if !spec.key.is_empty() {
            self.persist_setting(spec.key, new_val.to_string());
        }
        self.set_status(format!("{}: {}", spec.label, new_val), false);
    }

    pub(crate) fn toggle_bool_setting<F>(&mut self, key: &str, label: &str, mut getter: F)
    where
        F: FnMut(&mut crate::model::AppData) -> &mut bool,
    {
        let val = getter(&mut self.data);
        *val = !*val;
        let is_on = *val;
        self.persist_setting(key, if is_on { "1" } else { "0" });
        self.set_status(
            format!("{}: {}", label, if is_on { "on" } else { "off" }),
            false,
        );
    }

    pub(crate) fn handle_settings_key(&mut self, key: KeyEvent) {
        let n = self.settings_state.items.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.settings_state.selected = (self.settings_state.selected + 1) % n;
                self.sync_settings_scroll();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.settings_state.selected == 0 {
                    self.settings_state.selected = n - 1;
                } else {
                    self.settings_state.selected -= 1;
                }
                self.sync_settings_scroll();
            }
            KeyCode::Enter => {
                let item = self.settings_state.items[self.settings_state.selected];
                if item == SettingsItem::ExportBackup {
                    self.export_backup();
                } else {
                    self.adjust_setting(1);
                }
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_setting(1);
            }
            KeyCode::Left | KeyCode::Char('-') => {
                self.adjust_setting(-1);
            }
            KeyCode::Char('e') => {
                self.export_backup();
            }
            _ => {}
        }
    }

    pub fn settings_visual_row(selected: usize) -> usize {
        const HEADERS: [usize; 6] = [0, 5, 9, 10, 14, 17];
        selected + HEADERS.iter().filter(|h| **h <= selected).count()
    }

    pub fn sync_settings_scroll(&mut self) {
        let visible = self.settings_state.page_size.max(4);
        let visual = Self::settings_visual_row(self.settings_state.selected);
        if visual < self.settings_state.scroll_offset {
            self.settings_state.scroll_offset = visual;
        } else if visual >= self.settings_state.scroll_offset + visible {
            self.settings_state.scroll_offset = visual.saturating_sub(visible - 1);
        }
    }

    pub(crate) fn adjust_setting(&mut self, dir: i32) {
        let item = self.settings_state.items[self.settings_state.selected];
        match item {
            SettingsItem::FocusMinutes => {
                let cur = self.timer.config.focus_minutes as i32;
                let v = (cur + dir).clamp(1, 240) as u32;
                self.timer.set_focus_minutes(v);
                self.data.focus_minutes = v;
                if let Err(e) = self.db.persist_timer_settings(&self.data) {
                    self.set_status(format!("Save error: {e}"), true);
                }
                self.set_status(format!("Focus: {} min", v), false);
            }
            SettingsItem::ShortBreak => {
                let cur = self.timer.config.short_break_minutes as i32;
                let v = (cur + dir).clamp(1, 60) as u32;
                self.timer.config.short_break_minutes = v;
                self.data.short_break_minutes = v;
                if let Err(e) = self.db.persist_timer_settings(&self.data) {
                    self.set_status(format!("Save error: {e}"), true);
                }
                self.set_status(format!("Short break: {} min", v), false);
            }
            SettingsItem::LongBreak => {
                let cur = self.timer.config.long_break_minutes as i32;
                let v = (cur + dir).clamp(1, 120) as u32;
                self.timer.config.long_break_minutes = v;
                self.data.long_break_minutes = v;
                if let Err(e) = self.db.persist_timer_settings(&self.data) {
                    self.set_status(format!("Save error: {e}"), true);
                }
                self.set_status(format!("Long break: {} min", v), false);
            }
            SettingsItem::LongBreakEvery => {
                let cur = self.timer.config.long_break_every as i32;
                let v = (cur + dir).clamp(1, 12) as u32;
                self.timer.config.long_break_every = v;
                self.data.long_break_every = v;
                if let Err(e) = self.db.persist_timer_settings(&self.data) {
                    self.set_status(format!("Save error: {e}"), true);
                }
                self.set_status(format!("Long break every: {} sessions", v), false);
            }
            SettingsItem::DailyGoal => {
                self.adjust_numeric_setting(
                    dir,
                    NumericSettingSpec {
                        key: "daily_goal_minutes",
                        label: "Daily goal (min)",
                        min: 15,
                        max: 1440,
                        step: 15,
                    },
                    |d| &mut d.daily_goal_minutes,
                );
            }
            SettingsItem::Sound => {
                self.toggle_bool_setting("sound_enabled", "Sound", |d| &mut d.sound_enabled);
            }
            SettingsItem::Notifications => {
                self.toggle_bool_setting("notify_on_finish", "Notifications", |d| {
                    &mut d.notify_on_finish
                });
            }
            SettingsItem::AutoStartBreaks => {
                self.toggle_bool_setting("auto_start_breaks", "Auto-start breaks", |d| {
                    &mut d.auto_start_breaks
                });
            }
            SettingsItem::AutoStartFocus => {
                self.toggle_bool_setting("auto_start_focus", "Auto-start focus", |d| {
                    &mut d.auto_start_focus
                });
            }
            SettingsItem::ActiveTaskCycle => {
                if self.data.tasks.is_empty() {
                    self.set_active_task(None);
                    self.set_status("No tasks to activate.", true);
                    return;
                }
                let ids: Vec<u64> = self.data.tasks.iter().map(|t| t.id).collect();
                let cur = self
                    .active_task
                    .and_then(|id| ids.iter().position(|x| *x == id));
                let next_idx = match (cur, dir) {
                    (Some(i), d) if d > 0 => (i + 1) % ids.len(),
                    (Some(i), d) if d < 0 => {
                        if i == 0 {
                            ids.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    (None, _) => 0,
                    _ => 0,
                };
                self.set_active_task(Some(ids[next_idx]));
                if let Some(task) = self.data.tasks.iter().find(|t| t.id == ids[next_idx]) {
                    self.set_status(format!("Active task: {}", task.title), false);
                }
            }
            SettingsItem::Theme => {
                let next = self.theme_catalog.next_id(&self.data.theme);
                let label = self.theme_catalog.label(&next);
                self.apply_theme(&next);
                self.set_status(format!("Theme: {label}"), false);
            }
            SettingsItem::CustomMinutes => {
                let cur = self.timer.custom_minutes as i32;
                let v = (cur + dir).clamp(1, 240) as u32;
                self.timer.set_custom_minutes(v);
                self.set_status(format!("Custom timer: {} min", v), false);
            }
            SettingsItem::AutoPickTask => {
                self.toggle_bool_setting("auto_pick_task", "Auto-pick task", |d| {
                    &mut d.auto_pick_task
                });
            }
            SettingsItem::AutoAdvanceTask => {
                self.toggle_bool_setting("auto_advance_task", "Auto-advance task", |d| {
                    &mut d.auto_advance_task
                });
            }
            SettingsItem::EmptyQueueBehavior => {
                self.data.empty_queue_behavior = self.data.empty_queue_behavior.next();
                let key = match self.data.empty_queue_behavior {
                    EmptyQueueBehavior::FreeFocus => "free-focus",
                    EmptyQueueBehavior::PauseTimer => "pause-timer",
                    EmptyQueueBehavior::AskEachTime => "ask",
                };
                self.persist_setting("empty_queue_behavior", key);
                self.set_status(
                    format!(
                        "When queue empty: {}",
                        self.data.empty_queue_behavior.label()
                    ),
                    false,
                );
            }
            SettingsItem::LogBreaks => {
                self.toggle_bool_setting("log_breaks", "Log breaks", |d| &mut d.log_breaks);
            }
            SettingsItem::EstimateComplete => {
                self.data.estimate_complete = self.data.estimate_complete.next();
                let key = match self.data.estimate_complete {
                    EstimateCompleteBehavior::Nudge => "nudge",
                    EstimateCompleteBehavior::None => "none",
                    EstimateCompleteBehavior::AutoDone => "auto-done",
                };
                self.persist_setting("estimate_complete", key);
                self.set_status(
                    format!("Estimate reached: {}", self.data.estimate_complete.label()),
                    false,
                );
            }
            SettingsItem::TerminalTitle => {
                self.toggle_bool_setting("show_terminal_title", "Terminal title", |d| {
                    &mut d.show_terminal_title
                });
            }
            SettingsItem::WarnOneMinute => {
                self.toggle_bool_setting("warn_one_minute", "1-min warning", |d| {
                    &mut d.warn_one_minute
                });
            }
            SettingsItem::AutoPauseIdle => {
                self.adjust_numeric_setting(
                    dir,
                    NumericSettingSpec {
                        key: "auto_pause_idle_minutes",
                        label: "Auto-pause idle (min, 0=off)",
                        min: 0,
                        max: 120,
                        step: 5,
                    },
                    |d| &mut d.auto_pause_idle_minutes,
                );
            }
            SettingsItem::ArchiveAfterDays => {
                self.adjust_numeric_setting(
                    dir,
                    NumericSettingSpec {
                        key: "archive_after_days",
                        label: "Auto-archive after (days, 0=off)",
                        min: 0,
                        max: 365,
                        step: 7,
                    },
                    |d| &mut d.archive_after_days,
                );
            }
            SettingsItem::ExportBackup => {}
        }
        self.sync_timer_config_to_data();
    }
}

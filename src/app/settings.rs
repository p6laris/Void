use super::*;
use crate::model::{EmptyQueueBehavior, EstimateCompleteBehavior};
use crossterm::event::{KeyCode, KeyEvent};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug)]
pub(crate) struct CachedSettingsLabel {
    pub key: &'static str,
    pub value: String,
    pub desc: &'static str,
}

fn on_off(enabled: bool) -> String {
    if enabled { "on" } else { "off" }.into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    FocusMinutes,
    ShortBreak,
    LongBreak,
    LongBreakEvery,
    CustomMinutes,
    DailyGoal,
    Sound,
    Notifications,
    AutoStartBreaks,
    AutoStartFocus,
    ActiveTaskCycle,
    AutoPickTask,
    AutoAdvanceTask,
    EmptyQueueBehavior,
    EstimateComplete,
    ThemeMode,
    DarkTheme,
    LightTheme,
    CanvasMode,
    LogBreaks,
    RestDays,
    TerminalTitle,
    WarnOneMinute,
    AutoPauseIdle,
    ArchiveAfterDays,
    ExportBackupJson,
    ExportSessionsCsv,
}

/// Items that open a new section in the Settings table, in display order.
pub(crate) const SECTION_STARTS: [SettingsItem; 6] = [
    SettingsItem::FocusMinutes,
    SettingsItem::DailyGoal,
    SettingsItem::ActiveTaskCycle,
    SettingsItem::ThemeMode,
    SettingsItem::LogBreaks,
    SettingsItem::ExportBackupJson,
];

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
            page_size: 14,
            items: vec![
                SettingsItem::FocusMinutes,
                SettingsItem::ShortBreak,
                SettingsItem::LongBreak,
                SettingsItem::LongBreakEvery,
                SettingsItem::CustomMinutes,
                SettingsItem::DailyGoal,
                SettingsItem::Sound,
                SettingsItem::Notifications,
                SettingsItem::AutoStartBreaks,
                SettingsItem::AutoStartFocus,
                SettingsItem::ActiveTaskCycle,
                SettingsItem::AutoPickTask,
                SettingsItem::AutoAdvanceTask,
                SettingsItem::EmptyQueueBehavior,
                SettingsItem::EstimateComplete,
                SettingsItem::ThemeMode,
                SettingsItem::DarkTheme,
                SettingsItem::LightTheme,
                SettingsItem::CanvasMode,
                SettingsItem::LogBreaks,
                SettingsItem::RestDays,
                SettingsItem::TerminalTitle,
                SettingsItem::WarnOneMinute,
                SettingsItem::AutoPauseIdle,
                SettingsItem::ArchiveAfterDays,
                SettingsItem::ExportBackupJson,
                SettingsItem::ExportSessionsCsv,
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

    pub(crate) fn adjust_persisted_timer_setting(
        &mut self,
        dir: i32,
        min: u32,
        max: u32,
        mut getter: impl FnMut(&Self) -> u32,
        mut apply: impl FnMut(&mut Self, u32),
        status: impl Fn(u32) -> String,
    ) {
        let cur = getter(self) as i32;
        let v = (cur + dir).clamp(min as i32, max as i32) as u32;
        apply(self, v);
        if let Err(e) = self.db.persist_timer_settings(&self.data) {
            self.set_status(format!("Save error: {e}"), true);
        }
        self.set_status(status(v), false);
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
                match item {
                    SettingsItem::ExportBackupJson => self.export_backup(),
                    SettingsItem::ExportSessionsCsv => self.export_sessions_csv(),
                    _ => self.adjust_setting(1),
                }
            }
            KeyCode::Right | KeyCode::Char('+') | KeyCode::Char('=') => {
                self.adjust_setting(1);
            }
            KeyCode::Left | KeyCode::Char('-') => {
                self.adjust_setting(-1);
            }
            KeyCode::Char('e') => {
                let item = self.settings_state.items[self.settings_state.selected];
                if item == SettingsItem::ExportSessionsCsv {
                    self.export_sessions_csv();
                } else {
                    self.export_backup();
                }
            }
            _ => {}
        }
    }

    /// Row of item `selected` in the table, counting the section headers above it.
    pub fn settings_visual_row(&self, selected: usize) -> usize {
        let headers = self
            .settings_state
            .items
            .iter()
            .take(selected + 1)
            .filter(|item| SECTION_STARTS.contains(item))
            .count();
        selected + headers
    }

    pub fn sync_settings_scroll(&mut self) {
        let visible = self.settings_state.page_size.max(1);
        let visual = self.settings_visual_row(self.settings_state.selected);
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
                self.adjust_persisted_timer_setting(
                    dir,
                    1,
                    240,
                    |app| app.timer.config.focus_minutes,
                    |app, v| {
                        app.timer.set_focus_minutes(v);
                        app.data.focus_minutes = v;
                    },
                    |v| format!("Focus: {} min", v),
                );
            }
            SettingsItem::ShortBreak => {
                self.adjust_persisted_timer_setting(
                    dir,
                    1,
                    60,
                    |app| app.timer.config.short_break_minutes,
                    |app, v| {
                        app.timer.config.short_break_minutes = v;
                        app.data.short_break_minutes = v;
                    },
                    |v| format!("Short break: {} min", v),
                );
            }
            SettingsItem::LongBreak => {
                self.adjust_persisted_timer_setting(
                    dir,
                    1,
                    120,
                    |app| app.timer.config.long_break_minutes,
                    |app, v| {
                        app.timer.config.long_break_minutes = v;
                        app.data.long_break_minutes = v;
                    },
                    |v| format!("Long break: {} min", v),
                );
            }
            SettingsItem::LongBreakEvery => {
                self.adjust_persisted_timer_setting(
                    dir,
                    1,
                    12,
                    |app| app.timer.config.long_break_every,
                    |app, v| {
                        app.timer.config.long_break_every = v;
                        app.data.long_break_every = v;
                    },
                    |v| format!("Long break every: {} sessions", v),
                );
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
                let ids: Vec<u64> = self.data.tasks.keys().copied().collect();
                let cur = self
                    .task_ui
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
                if let Some(task) = self.data.task(ids[next_idx]) {
                    self.set_status(format!("Active task: {}", task.title), false);
                }
            }
            SettingsItem::ThemeMode => {
                self.data.theme_mode = if dir >= 0 {
                    self.data.theme_mode.next()
                } else {
                    self.data.theme_mode.prev()
                };
                let key = match self.data.theme_mode {
                    crate::model::ThemeMode::Auto => "auto",
                    crate::model::ThemeMode::Dark => "dark",
                    crate::model::ThemeMode::Light => "light",
                };
                self.persist_setting("theme_mode", key);
                self.refresh_theme();
                let sys = theme::detect_system_theme();
                let sys_label = if sys.is_light() {
                    "Light detected"
                } else {
                    "Dark detected"
                };
                self.set_status(
                    format!(
                        "Theme mode: {} (OS: {sys_label})",
                        self.data.theme_mode.label()
                    ),
                    false,
                );
            }
            SettingsItem::DarkTheme => {
                let next = if dir >= 0 {
                    self.theme_catalog.next_dark_id(&self.data.dark_theme)
                } else {
                    self.theme_catalog.prev_dark_id(&self.data.dark_theme)
                };
                self.data.dark_theme = next.clone();
                self.persist_setting("dark_theme", &next);
                self.refresh_theme();
                let label = self.theme_catalog.label(&next);
                self.set_status(format!("Preferred Dark theme: {label}"), false);
            }
            SettingsItem::LightTheme => {
                let next = if dir >= 0 {
                    self.theme_catalog.next_light_id(&self.data.light_theme)
                } else {
                    self.theme_catalog.prev_light_id(&self.data.light_theme)
                };
                self.data.light_theme = next.clone();
                self.persist_setting("light_theme", &next);
                self.refresh_theme();
                let label = self.theme_catalog.label(&next);
                self.set_status(format!("Preferred Light theme: {label}"), false);
            }
            SettingsItem::CanvasMode => {
                let next = if dir >= 0 {
                    self.data.canvas_mode.next()
                } else {
                    self.data.canvas_mode.prev()
                };
                self.data.canvas_mode = next;
                let key = match next {
                    crate::model::CanvasMode::Animated => "animated",
                    crate::model::CanvasMode::Static => "static",
                    crate::model::CanvasMode::Off => "off",
                };
                self.persist_setting("canvas_mode", key);
                self.set_status(format!("Dashboard art: {}", next.label()), false);
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
            SettingsItem::RestDays => {
                // Cycle through weekdays: each press toggles the next day.
                const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                // Find the next day to toggle based on direction.
                let idx = if dir > 0 {
                    // Find first non-rest day to add, or first rest day to remove.
                    (0u8..7)
                        .find(|d| !self.data.streak_rest_days.contains(d))
                        .unwrap_or(0)
                } else {
                    // Find last rest day to remove.
                    (0u8..7)
                        .rev()
                        .find(|d| self.data.streak_rest_days.contains(d))
                        .unwrap_or(0)
                };
                if self.data.streak_rest_days.contains(&idx) {
                    self.data.streak_rest_days.retain(|d| *d != idx);
                } else {
                    self.data.streak_rest_days.push(idx);
                    self.data.streak_rest_days.sort();
                }
                let label: Vec<&str> = self
                    .data
                    .streak_rest_days
                    .iter()
                    .filter_map(|d| DAYS.get(*d as usize).copied())
                    .collect();
                let display = if label.is_empty() {
                    "none".to_string()
                } else {
                    label.join(", ")
                };
                self.persist_setting(
                    "streak_rest_days",
                    self.data
                        .streak_rest_days
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
                self.set_status(format!("Rest days: {display}"), false);
            }
            SettingsItem::ExportBackupJson => {
                self.export_backup();
            }
            SettingsItem::ExportSessionsCsv => {
                self.export_sessions_csv();
            }
        }
        self.sync_timer_config_to_data();
        self.ui.settings_labels_sig = u64::MAX;
    }

    fn settings_labels_signature(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.data.focus_minutes.hash(&mut hasher);
        self.data.short_break_minutes.hash(&mut hasher);
        self.data.long_break_minutes.hash(&mut hasher);
        self.data.long_break_every.hash(&mut hasher);
        self.timer.custom_minutes.hash(&mut hasher);
        self.data.daily_goal_minutes.hash(&mut hasher);
        self.data.sound_enabled.hash(&mut hasher);
        self.data.notify_on_finish.hash(&mut hasher);
        self.data.auto_start_breaks.hash(&mut hasher);
        self.data.auto_start_focus.hash(&mut hasher);
        self.data.auto_pick_task.hash(&mut hasher);
        self.data.auto_advance_task.hash(&mut hasher);
        self.data.log_breaks.hash(&mut hasher);
        (self.data.theme_mode as u8).hash(&mut hasher);
        (self.data.canvas_mode as u8).hash(&mut hasher);
        self.data.dark_theme.hash(&mut hasher);
        self.data.light_theme.hash(&mut hasher);
        self.data.theme.hash(&mut hasher);
        self.task_ui.active_task.hash(&mut hasher);
        if let Some(id) = self.task_ui.active_task {
            if let Some(task) = self.data.task(id) {
                task.title.hash(&mut hasher);
            }
        }
        (self.data.empty_queue_behavior as u8).hash(&mut hasher);
        (self.data.estimate_complete as u8).hash(&mut hasher);
        self.data.streak_rest_days.hash(&mut hasher);
        hasher.finish()
    }

    pub(crate) fn build_settings_labels(&self) -> Vec<CachedSettingsLabel> {
        self.settings_state
            .items
            .iter()
            .map(|&item| self.settings_label(item))
            .collect()
    }

    fn settings_label(&self, item: SettingsItem) -> CachedSettingsLabel {
        match item {
            SettingsItem::FocusMinutes => CachedSettingsLabel {
                key: "Focus minutes",
                value: format!("{} min", self.data.focus_minutes),
                desc: "per focus session",
            },
            SettingsItem::ShortBreak => CachedSettingsLabel {
                key: "Short break",
                value: format!("{} min", self.data.short_break_minutes),
                desc: "between sessions",
            },
            SettingsItem::LongBreak => CachedSettingsLabel {
                key: "Long break",
                value: format!("{} min", self.data.long_break_minutes),
                desc: "after cycle",
            },
            SettingsItem::LongBreakEvery => CachedSettingsLabel {
                key: "Long break every",
                value: format!("{} sessions", self.data.long_break_every),
                desc: "focus sessions per cycle",
            },
            SettingsItem::CustomMinutes => CachedSettingsLabel {
                key: "Custom timer",
                value: format!("{} min", self.timer.custom_minutes),
                desc: "freeform session",
            },
            SettingsItem::DailyGoal => CachedSettingsLabel {
                key: "Daily goal",
                value: format!("{} min", self.data.daily_goal_minutes),
                desc: "+/-15 per step",
            },
            SettingsItem::Sound => CachedSettingsLabel {
                key: "Sound on finish",
                value: on_off(self.data.sound_enabled),
                desc: "plays on completion",
            },
            SettingsItem::Notifications => CachedSettingsLabel {
                key: "Notifications",
                value: on_off(self.data.notify_on_finish),
                desc: "desktop alerts",
            },
            SettingsItem::AutoStartBreaks => CachedSettingsLabel {
                key: "Auto-start breaks",
                value: on_off(self.data.auto_start_breaks),
                desc: "begin break automatically",
            },
            SettingsItem::AutoStartFocus => CachedSettingsLabel {
                key: "Auto-start focus",
                value: on_off(self.data.auto_start_focus),
                desc: "begin focus after break",
            },
            SettingsItem::ActiveTaskCycle => CachedSettingsLabel {
                key: "Active task",
                value: self
                    .task_ui
                    .active_task
                    .and_then(|id| self.data.task(id))
                    .map(|t| t.title.clone())
                    .unwrap_or_else(|| "(none)".into()),
                desc: "cycle with Enter",
            },
            SettingsItem::AutoPickTask => CachedSettingsLabel {
                key: "Auto-pick task",
                value: on_off(self.data.auto_pick_task),
                desc: "pick best task on start",
            },
            SettingsItem::AutoAdvanceTask => CachedSettingsLabel {
                key: "Auto-advance task",
                value: on_off(self.data.auto_advance_task),
                desc: "next task after focus",
            },
            SettingsItem::EmptyQueueBehavior => CachedSettingsLabel {
                key: "When queue empty",
                value: self.data.empty_queue_behavior.label().to_string(),
                desc: "free focus / pause / ask",
            },
            SettingsItem::EstimateComplete => CachedSettingsLabel {
                key: "Estimate reached",
                value: self.data.estimate_complete.label().to_string(),
                desc: "nudge / off / auto-done",
            },
            SettingsItem::ThemeMode => CachedSettingsLabel {
                key: "Theme mode",
                value: self.data.theme_mode.label().to_string(),
                desc: "Auto / Dark / Light",
            },
            SettingsItem::DarkTheme => CachedSettingsLabel {
                key: "Dark theme",
                value: self.theme_catalog.label(&self.data.dark_theme),
                desc: "preferred dark palette",
            },
            SettingsItem::LightTheme => CachedSettingsLabel {
                key: "Light theme",
                value: self.theme_catalog.label(&self.data.light_theme),
                desc: "preferred light palette",
            },
            SettingsItem::CanvasMode => CachedSettingsLabel {
                key: "Dashboard art",
                value: self.data.canvas_mode.label().to_string(),
                desc: "animated / static / off",
            },
            SettingsItem::LogBreaks => CachedSettingsLabel {
                key: "Log breaks",
                value: on_off(self.data.log_breaks),
                desc: "record break sessions",
            },
            SettingsItem::TerminalTitle => CachedSettingsLabel {
                key: "Terminal title",
                value: on_off(self.data.show_terminal_title),
                desc: "show timer in window bar",
            },
            SettingsItem::WarnOneMinute => CachedSettingsLabel {
                key: "1-min warning",
                value: on_off(self.data.warn_one_minute),
                desc: "alert before timer ends",
            },
            SettingsItem::AutoPauseIdle => CachedSettingsLabel {
                key: "Auto-pause idle",
                value: if self.data.auto_pause_idle_minutes == 0 {
                    "off".into()
                } else {
                    format!("{} min", self.data.auto_pause_idle_minutes)
                },
                desc: "pause on inactivity",
            },
            SettingsItem::ArchiveAfterDays => CachedSettingsLabel {
                key: "Archive after",
                value: format!("{} days", self.data.archive_after_days),
                desc: "auto-archive completed",
            },
            SettingsItem::RestDays => {
                const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                let names: Vec<&str> = self
                    .data
                    .streak_rest_days
                    .iter()
                    .filter_map(|d| DAYS.get(*d as usize).copied())
                    .collect();
                let rest_val = if names.is_empty() {
                    "none".to_string()
                } else {
                    names.join(", ")
                };
                CachedSettingsLabel {
                    key: "Rest days",
                    value: rest_val,
                    desc: "+/- to toggle days",
                }
            }
            SettingsItem::ExportBackupJson => CachedSettingsLabel {
                key: "Export JSON",
                value: "Enter to export".into(),
                desc: "writes data.json backup",
            },
            SettingsItem::ExportSessionsCsv => CachedSettingsLabel {
                key: "Export CSV",
                value: "Enter to export".into(),
                desc: "writes sessions.csv for sheets",
            },
        }
    }

    pub(crate) fn refresh_settings_labels_cache(&mut self) {
        let sig = self.settings_labels_signature();
        if sig == self.ui.settings_labels_sig {
            return;
        }
        self.ui.settings_labels_sig = sig;
        self.ui.cached_settings_labels = self.build_settings_labels();
    }

    pub(crate) fn settings_labels(&self) -> &[CachedSettingsLabel] {
        &self.ui.cached_settings_labels
    }
}

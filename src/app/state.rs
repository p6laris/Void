use std::collections::HashSet;
use std::time::Instant;

use ratatui::widgets::ListState;

use crate::model::{Priority, StoredSession};

use super::{
    CachedSettingsLabel, FocusTab, InputField, InputMode, Popup, StatsViewMode, TaskFilter,
};

#[derive(Debug)]
pub struct UiState {
    pub tab: FocusTab,
    pub zen_mode: bool,
    pub status: Option<String>,
    pub status_error: bool,
    pub last_status_set: Instant,
    pub should_quit: bool,
    pub help_scroll: u16,
    pub about_scroll: u16,
    pub(crate) frame_today: String,
    pub(crate) frame_today_focus_mins: u32,
    pub(crate) window_title_sig: u64,
    pub(crate) cached_window_title: String,
    pub(crate) settings_labels_sig: u64,
    pub(crate) cached_settings_labels: Vec<CachedSettingsLabel>,
}

#[derive(Debug)]
pub struct InputState {
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_due_date: String,
    pub input_tags: String,
    pub input_number: u32,
    pub input_priority: Priority,
    pub input_field: InputField,
    pub popup: Option<Popup>,
}

#[derive(Debug)]
pub struct TaskUiState {
    pub task_state: ListState,
    pub dashboard_task_state: ListState,
    pub goal_switch_state: ListState,
    pub active_task: Option<u64>,
    pub task_filter: TaskFilter,
    pub active_tag_filter: Option<String>,
    pub task_search: String,
    pub task_search_lower: String,
    pub searching: bool,
    pub cached_filtered_tasks: Vec<usize>,
    pub cached_dashboard_tasks: Vec<usize>,
    pub(crate) cached_task_tags: Vec<String>,
    pub(crate) cached_task_blocked: Vec<bool>,
    pub bulk_mode: bool,
    pub bulk_selected: HashSet<u64>,
    pub reordering_task: Option<u64>,
    pub subtask_selected: usize,
    pub subtask_focus: bool,
    pub subtask_state: ListState,
}

#[derive(Debug)]
pub struct StatsState {
    pub weekly_chart: Vec<(String, u32)>,
    pub heatmap_data: Vec<(String, u32)>,
    pub session_counts: (u32, u32, u32),
    pub chart_dirty: bool,
    pub recent_sessions: Vec<StoredSession>,
    pub stats_session_selected: usize,
    pub stats_session_page: usize,
    pub stats_session_total: usize,
    pub timeline_sessions: Vec<StoredSession>,
    pub heatmap_cursor: Option<chrono::NaiveDate>,
    pub cursor_sessions: Vec<StoredSession>,
    pub stats_view_mode: StatsViewMode,
    pub tag_analytics: Vec<(String, u32)>,
    pub calendar_date: chrono::NaiveDate,
}

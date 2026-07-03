use std::collections::HashSet;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::KeyModifiers;
use ratatui::widgets::ListState;

use crate::db::Database;
use crate::model::{
    AppData, EmptyQueueBehavior, EstimateCompleteBehavior, Priority, StoredSession, TimerMode,
    TimerState,
};
use crate::sound;
use crate::storage;
use crate::timer::{Timer, TimerConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTab {
    Dashboard,
    Tasks,
    Stats,
    Settings,
    Help,
    About,
}

impl FocusTab {
    pub const fn all() -> [FocusTab; 6] {
        [
            FocusTab::Dashboard,
            FocusTab::Tasks,
            FocusTab::Stats,
            FocusTab::Settings,
            FocusTab::Help,
            FocusTab::About,
        ]
    }
    pub fn label(&self) -> &'static str {
        match self {
            FocusTab::Dashboard => "Dashboard",
            FocusTab::Tasks => "Tasks",
            FocusTab::Stats => "Stats",
            FocusTab::Settings => "Settings",
            FocusTab::Help => "Help",
            FocusTab::About => "About",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone)]
pub enum BulkAction {
    MarkDone,
    Delete,
}

#[derive(Debug, Clone)]
pub enum Popup {
    AddTask,
    EditTask(u64),
    ConfirmDelete(u64),
    EmptyQueueChoice,
    AddSubtask(u64),
    BulkConfirm(BulkAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputField {
    Title,
    Estimate,
    Priority,
    DueDate,
    Tags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFilter {
    All,
    Pending,
    Done,
    Today,
    Archived,
}

impl TaskFilter {
    pub fn label(&self) -> &'static str {
        match self {
            TaskFilter::All => "All",
            TaskFilter::Pending => "Open",
            TaskFilter::Done => "Done",
            TaskFilter::Today => "Today",
            TaskFilter::Archived => "Archive",
        }
    }

    pub fn next(self) -> Self {
        match self {
            TaskFilter::All => TaskFilter::Pending,
            TaskFilter::Pending => TaskFilter::Done,
            TaskFilter::Done => TaskFilter::Today,
            TaskFilter::Today => TaskFilter::Archived,
            TaskFilter::Archived => TaskFilter::All,
        }
    }
}

use crate::theme::{self, ThemeCatalog};
use crate::ui::IconSet;

pub use theme::Theme;

pub mod settings;
pub use settings::*;
pub mod keys;
pub mod popups;
pub mod task_ops;
pub mod timer_ops;

pub struct App {
    pub db: Database,
    pub data: AppData,
    pub timer: Timer,
    pub tab: FocusTab,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_due_date: String,
    pub input_tags: String,
    pub input_number: u32,
    pub input_priority: Priority,
    pub input_field: InputField,
    pub popup: Option<Popup>,
    pub task_state: ListState,
    pub goal_switch_state: ListState,
    pub settings_state: SettingsState,
    pub status: Option<String>,
    pub status_error: bool,
    pub last_status_set: Instant,
    pub should_quit: bool,
    pub theme: Theme,
    pub theme_catalog: ThemeCatalog,
    pub icons: IconSet,
    pub active_task: Option<u64>,
    pub zen_mode: bool,
    pub task_filter: TaskFilter,
    pub active_tag_filter: Option<String>,
    pub task_search: String,
    pub task_search_lower: String,
    pub searching: bool,
    pub cached_filtered_tasks: Vec<usize>,
    pub cached_dashboard_tasks: Vec<usize>,
    cached_task_tags: Vec<String>,
    pub weekly_chart: Vec<(String, u32)>,
    pub heatmap_data: Vec<(String, u32)>,
    pub session_counts: (u32, u32, u32),
    pub chart_dirty: bool,
    pub data_version: u64,
    pub recent_sessions: Vec<StoredSession>,
    pub stats_session_selected: usize,
    pub dashboard_task_state: ListState,
    pub calendar_date: chrono::NaiveDate,
    pub bulk_mode: bool,
    pub bulk_selected: HashSet<u64>,
    pub reordering_task: Option<u64>,
    pub subtask_selected: usize,
    pub subtask_focus: bool,
    pub subtask_state: ListState,
    pub stats_session_page: usize,
    pub stats_session_total: usize,
    pub end_warning_shown: bool,
    pub last_activity: Instant,
    pub timeline_sessions: Vec<StoredSession>,
    pub help_scroll: u16,
    pub about_scroll: u16,
    pub heatmap_cursor: Option<chrono::NaiveDate>,
    pub cursor_sessions: Vec<StoredSession>,
    pub stats_view_mode: StatsViewMode,
    pub tag_analytics: Vec<(String, u32)>,
    frame_today: String,
    frame_today_focus_mins: u32,
    window_title_sig: u64,
    cached_window_title: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatsViewMode {
    Overview,
    Analytics,
}

impl App {
    pub fn new() -> Result<Self> {
        let db = Database::open()?;
        let mut data = db.load_app_data().unwrap_or_default();
        let _ = storage::ensure_today_reset(&db, &mut data);
        let config = TimerConfig::from_app_data(&data);
        let mut timer = Timer::new(config);
        let (completed, mut mode) = db.load_timer_state();
        if mode == TimerMode::ShortBreak || mode == TimerMode::LongBreak {
            mode = TimerMode::Focus;
            let _ = db.persist_timer_state(completed, mode);
        }
        timer.completed_focus_sessions = completed;
        timer.configure(mode);
        let recent_sessions = db.recent_sessions(15).unwrap_or_default();
        let stats_session_total = db.session_count().unwrap_or(0);
        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let timeline_sessions = db.sessions_on_date(&today_str).unwrap_or_default();
        let archived = storage::auto_archive_old_tasks(&db, &mut data).unwrap_or(0);
        let mut task_state = ListState::default();
        let mut dashboard_task_state = ListState::default();
        if !data.tasks.is_empty() {
            task_state.select(Some(0));
            dashboard_task_state.select(Some(0));
        }
        let weekly_chart = storage::minutes_by_date(&db, 7).unwrap_or_default();
        let heatmap_data = storage::focus_heatmap(&db).unwrap_or_default();
        let tag_analytics = storage::tag_analytics(&db, &data, 30).unwrap_or_default();
        let session_counts = db.session_counts_by_mode().unwrap_or((0, 0, 0));
        let theme_catalog = ThemeCatalog::load();
        let theme_id = theme::normalize_theme_id(&data.theme);
        data.theme = theme_id.clone();
        let theme = theme::resolve(&theme_id, &theme_catalog).unwrap_or_else(|_| Theme::matrix());
        let icons = IconSet::detect();
        let active_task = data.active_task_id.filter(|id| {
            data.tasks
                .iter()
                .find(|t| t.id == *id)
                .is_some_and(|t| t.status != crate::model::TaskStatus::Done)
        });
        if active_task != data.active_task_id {
            data.active_task_id = active_task;
        }
        let welcome = "Welcome to Void! Press 5 or 'h' for help.";
        let mut status_msg = welcome.to_string();
        if archived > 0 {
            status_msg = format!("Auto-archived {archived} old tasks. {welcome}");
        }
        let (overdue, due_today) = storage::overdue_and_due_today(&data);
        if !overdue.is_empty() || !due_today.is_empty() {
            let mut parts = Vec::new();
            if !overdue.is_empty() {
                parts.push(format!("{} overdue", overdue.len()));
            }
            if !due_today.is_empty() {
                parts.push(format!("{} due today", due_today.len()));
            }
            if data.notify_on_finish {
                sound::notify_typed(
                    sound::NotifyKind::FocusComplete,
                    "Void · Task reminders",
                    &parts.join(", "),
                );
            }
            status_msg = format!("{} · {}", parts.join(", "), status_msg);
        }
        let mut app = Self {
            db,
            data,
            timer,
            tab: FocusTab::Dashboard,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_due_date: String::new(),
            input_tags: String::new(),
            input_number: 25,
            input_priority: Priority::Medium,
            input_field: InputField::Title,
            popup: None,
            task_state,
            goal_switch_state: ListState::default(),
            settings_state: SettingsState::new(),
            status: Some(status_msg),
            status_error: false,
            last_status_set: Instant::now(),
            should_quit: false,
            theme,
            theme_catalog,
            icons,
            active_task,
            zen_mode: false,
            task_filter: TaskFilter::All,
            active_tag_filter: None,
            task_search: String::new(),
            task_search_lower: String::new(),
            searching: false,
            cached_filtered_tasks: Vec::new(),
            cached_dashboard_tasks: Vec::new(),
            cached_task_tags: Vec::new(),
            weekly_chart,
            heatmap_data,
            session_counts,
            chart_dirty: false,
            data_version: 0,
            recent_sessions,
            stats_session_selected: 0,
            dashboard_task_state,
            calendar_date: chrono::Local::now().date_naive(),
            bulk_mode: false,
            bulk_selected: HashSet::new(),
            reordering_task: None,
            subtask_selected: 0,
            subtask_focus: false,
            subtask_state: ListState::default(),
            stats_session_page: 0,
            stats_session_total,
            end_warning_shown: false,
            last_activity: Instant::now(),
            timeline_sessions,
            help_scroll: 0,
            about_scroll: 0,
            heatmap_cursor: None,
            cursor_sessions: Vec::new(),
            stats_view_mode: StatsViewMode::Overview,
            tag_analytics,
            frame_today: String::new(),
            frame_today_focus_mins: 0,
            window_title_sig: u64::MAX,
            cached_window_title: String::new(),
        };
        app.recompute_task_caches();
        app.refresh_frame_today_cache();
        Ok(app)
    }

    pub(crate) fn refresh_frame_today_cache(&mut self) {
        self.frame_today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.frame_today_focus_mins = if self.data.today_date.as_deref() == Some(self.frame_today.as_str())
        {
            self.data.today_focus_minutes
        } else {
            0
        };
    }

    pub fn frame_today(&self) -> &str {
        &self.frame_today
    }

    pub fn today_focus_mins(&self) -> u32 {
        self.frame_today_focus_mins
    }

    pub const SESSIONS_PER_PAGE: usize = 15;

    pub fn apply_theme(&mut self, id: &str) {
        let id = theme::normalize_theme_id(id);
        match theme::resolve(&id, &self.theme_catalog) {
            Ok(resolved) => {
                self.theme = resolved;
                self.data.theme = id.clone();
                self.persist_setting("theme", &id);
            }
            Err(err) => {
                self.set_status(format!("Theme `{id}` unavailable: {err:#}"), true);
            }
        }
    }

    pub fn queue_empty(&self) -> bool {
        storage::queue_empty(&self.data)
    }

    pub fn daily_goal_met(&self) -> bool {
        self.frame_today_focus_mins >= self.data.daily_goal_minutes
    }

    fn persist_timer_state(&mut self) {
        let completed = self.timer.completed_focus_sessions;
        let mode = self.timer.mode;
        self.persist(|db| db.persist_timer_state(completed, mode));
    }

    pub(crate) fn active_stats_sessions(&self) -> &[StoredSession] {
        if self.heatmap_cursor.is_some() {
            &self.cursor_sessions
        } else {
            &self.recent_sessions
        }
    }

    pub(crate) fn clamp_stats_session_selection(&mut self) {
        let n = self.active_stats_sessions().len();
        if n == 0 {
            self.stats_session_selected = 0;
        } else if self.stats_session_selected >= n {
            self.stats_session_selected = n - 1;
        }
    }

    pub(crate) fn selected_stats_session(&self) -> Option<&StoredSession> {
        self.active_stats_sessions()
            .get(self.stats_session_selected)
    }

    pub(crate) fn refresh_recent_sessions(&mut self) {
        let offset = self.stats_session_page * Self::SESSIONS_PER_PAGE;
        match self
            .db
            .recent_sessions_paged(offset, Self::SESSIONS_PER_PAGE)
        {
            Ok(sessions) => self.recent_sessions = sessions,
            Err(e) => self.set_status(format!("Error loading sessions: {e}"), true),
        }
        self.stats_session_total = self.db.session_count().unwrap_or(0);
        self.clamp_stats_session_selection();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        self.timeline_sessions = self.db.sessions_on_date(&today).unwrap_or_default();
    }

    pub fn tick_rate(&self) -> Duration {
        match self.timer.state {
            TimerState::Running => Duration::from_millis(50),
            TimerState::Paused => Duration::from_millis(100),
            TimerState::Idle => Duration::from_millis(100),
            _ => Duration::from_millis(200),
        }
    }

    fn window_title_signature(&self) -> u64 {
        let rem_secs = self.timer.remaining_secs_f64() as u32;
        let state = self.timer.state as u8 as u64;
        let mode = self.timer.mode as u8 as u64;
        (state << 40) | (mode << 32) | rem_secs as u64
    }

    fn format_window_title(&self) -> String {
        let (main, tenths, _) = crate::canvas_timer::format_time_stack(&self.timer);
        let state = match self.timer.state {
            TimerState::Running => self.icons.play,
            TimerState::Paused => self.icons.pause,
            TimerState::Finished => self.icons.check,
            TimerState::Idle => self.icons.idle,
        };
        format!(
            "Void {} {}{} · {}",
            state,
            main,
            tenths,
            self.timer.mode.label()
        )
    }

    /// Rebuilds and returns the window title when timer state/mode/seconds change (~1/sec while running).
    pub fn poll_window_title(&mut self) -> Option<&str> {
        if !self.data.show_terminal_title {
            if self.window_title_sig != u64::MAX {
                self.window_title_sig = u64::MAX;
            }
            return None;
        }
        let sig = self.window_title_signature();
        if sig == self.window_title_sig {
            return None;
        }
        self.window_title_sig = sig;
        self.cached_window_title = self.format_window_title();
        Some(&self.cached_window_title)
    }

    pub(crate) fn bump_tasks(&mut self) {
        self.data_version = self.data_version.wrapping_add(1);
        self.recompute_task_caches();
        self.clamp_dashboard_task_selection();
    }

    pub(crate) fn bump_sessions(&mut self) {
        self.data_version = self.data_version.wrapping_add(1);
        self.chart_dirty = true;
        self.refresh_recent_sessions();
    }

    pub fn bump_data(&mut self) {
        self.bump_tasks();
        self.chart_dirty = true;
        self.refresh_recent_sessions();
    }

    fn persist<F>(&mut self, op: F)
    where
        F: FnOnce(&Database) -> anyhow::Result<()>,
    {
        if let Err(e) = op(&self.db) {
            self.set_status(format!("Save error: {e}"), true);
        }
    }

    fn persist_data<F>(&mut self, op: F)
    where
        F: FnOnce(&Database, &mut AppData) -> anyhow::Result<()>,
    {
        if let Err(e) = op(&self.db, &mut self.data) {
            self.set_status(format!("Save error: {e}"), true);
        }
    }

    fn persist_setting(&mut self, key: &str, value: impl AsRef<str>) {
        self.persist(|db| db.set_setting(key, value.as_ref()));
    }

    pub fn refresh_chart_if_needed(&mut self) {
        if self.chart_dirty {
            match storage::minutes_by_date(&self.db, 7) {
                Ok(data) => self.weekly_chart = data,
                Err(e) => self.set_status(format!("Chart error: {e}"), true),
            }
            match storage::focus_heatmap(&self.db) {
                Ok(data) => self.heatmap_data = data,
                Err(e) => self.set_status(format!("Heatmap error: {e}"), true),
            }
            match self.db.session_counts_by_mode() {
                Ok(counts) => self.session_counts = counts,
                Err(e) => self.set_status(format!("Stats error: {e}"), true),
            }
            self.chart_dirty = false;
        }
    }

    pub fn reload_heatmap(&mut self) {
        if let Ok(data) = storage::focus_heatmap(&self.db) {
            self.heatmap_data = data;
        }
    }

    fn sync_timer_config_to_data(&mut self) {
        self.data.focus_minutes = self.timer.config.focus_minutes;
        self.data.short_break_minutes = self.timer.config.short_break_minutes;
        self.data.long_break_minutes = self.timer.config.long_break_minutes;
        self.data.long_break_every = self.timer.config.long_break_every;
    }

    fn elapsed_minutes(&self, skipped: bool) -> u32 {
        let secs = self.timer.current_elapsed_seconds();
        if skipped {
            secs.div_ceil(60).max(1)
        } else {
            (secs / 60).max(1)
        }
    }

    pub fn hint(&self) -> String {
        match self.tab {
            FocusTab::Dashboard => {
                if self.zen_mode {
                    "[p] Cycle Task  [s/Space] Start/Pause  [n] Skip  [r] Reset  [z] Exit Zen"
                        .into()
                } else {
                    "[j/k] Select Task  [Enter] Status  [x] Mark Done  [z] Zen Mode".into()
                }
            }
            FocusTab::Tasks => {
                "[c] Add Task  [Enter] Edit  [j/k] Navigate  [Tab] Subtasks  [A] Archive".into()
            }
            FocusTab::Stats => {
                "[v] View  [Arrows] Heatmap  [j/k] History  [d] Delete  [Esc] Clear".into()
            }
            FocusTab::Settings => {
                "[↑↓] Navigate  [Enter] Toggle  [-/+] Adjust Value  [e] Export Data".into()
            }
            FocusTab::Help => "[j/k/Up/Down] Scroll  [Tab] Switch Tab".into(),
            FocusTab::About => "[Tab] Switch Tab".into(),
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        self.status = Some(msg.into());
        self.status_error = error;
        self.last_status_set = Instant::now();
    }

    fn check_queue_empty(&mut self) {
        if self.queue_empty() {
            self.on_queue_empty();
        }
    }

    fn on_queue_empty(&mut self) {
        match self.data.empty_queue_behavior {
            EmptyQueueBehavior::FreeFocus => {
                self.set_status(
                    "All tasks done — free focus. Sessions log as general focus. [E] end session",
                    false,
                );
            }
            EmptyQueueBehavior::PauseTimer => {
                if self.timer.state == TimerState::Running {
                    self.pause_timer();
                } else if self.timer.state != TimerState::Paused {
                    self.timer.reset();
                }
                self.set_status("All tasks done — timer paused. [E] end session", false);
            }
            EmptyQueueBehavior::AskEachTime => {
                self.popup = Some(Popup::EmptyQueueChoice);
                self.set_status(
                    "All tasks done — [Enter] free focus  [p] pause  [a] add task",
                    false,
                );
            }
        }
    }

    pub fn export_backup(&mut self) {
        match self.db.export_json() {
            Ok(path) => self.set_status(format!("Exported backup to {}", path.display()), false),
            Err(e) => self.set_status(format!("Export failed: {e}"), true),
        }
    }

    pub fn open_add_task(&mut self) {
        self.input_buffer.clear();
        self.input_due_date.clear();
        self.input_tags.clear();
        self.input_number = 25;
        self.input_priority = Priority::Medium;
        self.input_field = InputField::Title;
        self.popup = Some(Popup::AddTask);
        self.input_mode = InputMode::Editing;
    }

    pub fn open_edit_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.set_status("No task selected.", true);
            return;
        };
        if let Some(t) = self.data.tasks.iter().find(|t| t.id == id).cloned() {
            self.input_buffer = t.title;
            self.input_due_date = t.due_date.unwrap_or_default();
            self.input_tags = t.tags.join(", ");
            self.input_number = t.estimated_minutes;
            self.input_priority = t.priority;
            self.input_field = InputField::Title;
            self.popup = Some(Popup::EditTask(id));
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn open_confirm_delete(&mut self) {
        if let Some(id) = self.selected_task_id() {
            self.popup = Some(Popup::ConfirmDelete(id));
        } else {
            self.set_status("No task to delete.", true);
        }
    }

    fn popup_due_date(&self) -> Result<Option<String>, String> {
        let allow_past = matches!(self.popup, Some(crate::app::Popup::EditTask(_)));
        storage::normalize_due_date(&self.input_due_date, allow_past)
    }

    pub fn cycle_tag_filter(&mut self) {
        let tags = self.task_tags();

        if tags.is_empty() {
            self.set_status("No tags available to filter.", true);
            return;
        }

        self.active_tag_filter = match &self.active_tag_filter {
            None => Some(tags[0].clone()),
            Some(current) => {
                if let Some(idx) = tags.iter().position(|t| t == current) {
                    if idx + 1 < tags.len() {
                        Some(tags[idx + 1].clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        let msg = match &self.active_tag_filter {
            Some(t) => format!("Filtered by tag: #{}", t),
            None => "Tag filter cleared.".to_string(),
        };
        self.set_status(msg, false);

        self.clamp_dashboard_task_selection();
        let len = self.filtered_task_indices().len();
        if len == 0 {
            self.task_state.select(None);
        } else {
            let sel = self.task_state.selected().unwrap_or(0).min(len - 1);
            self.task_state.select(Some(sel));
        }
    }

    fn popup_tags(&self) -> Vec<String> {
        storage::parse_tags(&self.input_tags)
    }

    pub fn selected_task_id(&self) -> Option<u64> {
        let indices = self.filtered_task_indices();
        self.task_state
            .selected()
            .and_then(|i| indices.get(i).copied())
            .and_then(|idx| self.data.tasks.get(idx).map(|t| t.id))
    }
}

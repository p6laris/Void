use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn label(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Med",
            Priority::High => "High",
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            Priority::High => 3,
            Priority::Medium => 2,
            Priority::Low => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn label(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "Pending",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Done => "Done",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "Todo",
            TaskStatus::InProgress => "Active",
            TaskStatus::Done => "Done",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "○",
            TaskStatus::InProgress => "◉",
            TaskStatus::Done => "✓",
        }
    }

    pub fn bracket_marker(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "[ ]",
            TaskStatus::InProgress => "[~]",
            TaskStatus::Done => "[x]",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskRecurrence {
    #[default]
    None,
    Daily,
    Weekly,
    Weekdays,
}

impl TaskRecurrence {
    pub fn label(&self) -> &'static str {
        match self {
            TaskRecurrence::None => "None",
            TaskRecurrence::Daily => "Daily",
            TaskRecurrence::Weekly => "Weekly",
            TaskRecurrence::Weekdays => "Weekdays",
        }
    }

    pub fn next(self) -> Self {
        match self {
            TaskRecurrence::None => TaskRecurrence::Daily,
            TaskRecurrence::Daily => TaskRecurrence::Weekly,
            TaskRecurrence::Weekly => TaskRecurrence::Weekdays,
            TaskRecurrence::Weekdays => TaskRecurrence::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub notes: String,
    pub priority: Priority,
    pub status: TaskStatus,
    pub estimated_minutes: u32,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub actual_minutes: u32,
    pub sessions: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub today: bool,
    #[serde(default)]
    pub sort_order: u32,
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub recurrence: TaskRecurrence,
    #[serde(default)]
    pub blocked_by: Vec<u64>,
    #[serde(default)]
    pub archived: bool,
}

impl Task {
    pub fn new(id: u64, title: String) -> Self {
        Self {
            id,
            title,
            notes: String::new(),
            priority: Priority::Medium,
            status: TaskStatus::Pending,
            estimated_minutes: 25,
            created_at: Utc::now(),
            completed_at: None,
            actual_minutes: 0,
            sessions: 0,
            tags: Vec::new(),
            due_date: None,
            today: false,
            sort_order: id as u32,
            subtasks: Vec::new(),
            recurrence: TaskRecurrence::None,
            blocked_by: Vec::new(),
            archived: false,
        }
    }

    pub fn is_blocked(&self, tasks: &[Task]) -> bool {
        self.blocked_by.iter().any(|&blocker_id| {
            tasks
                .iter()
                .find(|t| t.id == blocker_id)
                .is_some_and(|t| t.status != TaskStatus::Done)
        })
    }

    pub fn subtask_progress(&self) -> Option<(usize, usize)> {
        if self.subtasks.is_empty() {
            return None;
        }
        let done = self.subtasks.iter().filter(|s| s.done).count();
        Some((done, self.subtasks.len()))
    }

    pub fn is_overdue(&self) -> bool {
        if self.status == TaskStatus::Done {
            return false;
        }
        let Some(ref due) = self.due_date else {
            return false;
        };
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        due.as_str() < today.as_str()
    }

    pub fn progress_ratio(&self) -> f64 {
        if self.estimated_minutes == 0 {
            return 0.0;
        }
        (self.actual_minutes as f64 / self.estimated_minutes as f64).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Running,
    Paused,
    Finished,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimerMode {
    Focus,
    ShortBreak,
    LongBreak,
    Custom,
}

impl TimerMode {
    pub fn label(&self) -> &'static str {
        match self {
            TimerMode::Focus => "FOCUS",
            TimerMode::ShortBreak => "SHORT BREAK",
            TimerMode::LongBreak => "LONG BREAK",
            TimerMode::Custom => "CUSTOM",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSessionRecord {
    pub date: String,
    pub minutes: u32,
    pub task_id: Option<u64>,
    pub mode: TimerMode,
    pub completed_at: DateTime<Utc>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub pause_count: u32,
    #[serde(default)]
    pub pause_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerPreset {
    pub name: String,
    pub focus_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub long_break_every: u32,
}

impl TimerPreset {
    pub fn deep_work() -> Self {
        Self {
            name: "Deep Work 50/10".into(),
            focus_minutes: 50,
            short_break_minutes: 10,
            long_break_minutes: 20,
            long_break_every: 3,
        }
    }

    pub fn quick() -> Self {
        Self {
            name: "Quick 15/3".into(),
            focus_minutes: 15,
            short_break_minutes: 3,
            long_break_minutes: 10,
            long_break_every: 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub id: i64,
    pub record: FocusSessionRecord,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EmptyQueueBehavior {
    #[default]
    FreeFocus,
    PauseTimer,
    AskEachTime,
}

impl EmptyQueueBehavior {
    pub fn label(&self) -> &'static str {
        match self {
            EmptyQueueBehavior::FreeFocus => "Free focus",
            EmptyQueueBehavior::PauseTimer => "Pause timer",
            EmptyQueueBehavior::AskEachTime => "Ask each time",
        }
    }

    pub fn next(self) -> Self {
        match self {
            EmptyQueueBehavior::FreeFocus => EmptyQueueBehavior::PauseTimer,
            EmptyQueueBehavior::PauseTimer => EmptyQueueBehavior::AskEachTime,
            EmptyQueueBehavior::AskEachTime => EmptyQueueBehavior::FreeFocus,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EstimateCompleteBehavior {
    #[default]
    Nudge,
    None,
    AutoDone,
}

impl EstimateCompleteBehavior {
    pub fn label(&self) -> &'static str {
        match self {
            EstimateCompleteBehavior::Nudge => "Nudge",
            EstimateCompleteBehavior::None => "Off",
            EstimateCompleteBehavior::AutoDone => "Auto-done",
        }
    }

    pub fn next(self) -> Self {
        match self {
            EstimateCompleteBehavior::Nudge => EstimateCompleteBehavior::None,
            EstimateCompleteBehavior::None => EstimateCompleteBehavior::AutoDone,
            EstimateCompleteBehavior::AutoDone => EstimateCompleteBehavior::Nudge,
        }
    }
}

fn default_focus_minutes() -> u32 {
    25
}

fn default_short_break() -> u32 {
    5
}

fn default_long_break() -> u32 {
    15
}

fn default_long_every() -> u32 {
    4
}

fn default_true() -> bool {
    true
}

fn default_theme_id() -> String {
    "matrix".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub tasks: Vec<Task>,
    pub total_focus_minutes: u32,
    pub total_sessions: u32,
    pub streak_days: u32,
    pub last_session_date: Option<String>,
    pub daily_goal_minutes: u32,
    pub sound_enabled: bool,
    pub auto_start_breaks: bool,
    pub auto_start_focus: bool,
    pub next_id: u64,
    #[serde(default)]
    pub today_focus_minutes: u32,
    #[serde(default)]
    pub today_date: Option<String>,
    #[serde(default = "default_focus_minutes")]
    pub focus_minutes: u32,
    #[serde(default = "default_short_break")]
    pub short_break_minutes: u32,
    #[serde(default = "default_long_break")]
    pub long_break_minutes: u32,
    #[serde(default = "default_long_every")]
    pub long_break_every: u32,
    #[serde(default)]
    pub session_history: Vec<FocusSessionRecord>,
    #[serde(default = "default_true")]
    pub auto_pick_task: bool,
    #[serde(default)]
    pub auto_advance_task: bool,
    #[serde(default = "default_theme_id")]
    pub theme: String,
    #[serde(default)]
    pub active_task_id: Option<u64>,
    #[serde(default = "default_true")]
    pub notify_on_finish: bool,
    #[serde(default)]
    pub goal_streak_days: u32,
    #[serde(default)]
    pub last_goal_date: Option<String>,
    #[serde(default)]
    pub empty_queue_behavior: EmptyQueueBehavior,
    #[serde(default)]
    pub log_breaks: bool,
    #[serde(default)]
    pub estimate_complete: EstimateCompleteBehavior,
    #[serde(default = "default_true")]
    pub show_terminal_title: bool,
    #[serde(default = "default_true")]
    pub warn_one_minute: bool,
    #[serde(default)]
    pub auto_pause_idle_minutes: u32,
    #[serde(default = "default_archive_days")]
    pub archive_after_days: u32,
    #[serde(default)]
    pub weekly_streak_weeks: u32,
    #[serde(default)]
    pub monthly_streak_months: u32,
    #[serde(default)]
    pub last_weekly_streak_key: Option<String>,
    #[serde(default)]
    pub last_monthly_streak_key: Option<String>,
    #[serde(default = "default_timer_presets")]
    pub timer_presets: Vec<TimerPreset>,
    #[serde(default)]
    pub active_preset: Option<String>,
}

fn default_archive_days() -> u32 {
    30
}

fn default_timer_presets() -> Vec<TimerPreset> {
    vec![TimerPreset::deep_work(), TimerPreset::quick()]
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            total_focus_minutes: 0,
            total_sessions: 0,
            streak_days: 0,
            last_session_date: None,
            daily_goal_minutes: 120,
            sound_enabled: true,
            auto_start_breaks: false,
            auto_start_focus: false,
            next_id: 1,
            today_focus_minutes: 0,
            today_date: None,
            focus_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
            session_history: Vec::new(),
            auto_pick_task: true,
            auto_advance_task: true,
            theme: default_theme_id(),
            active_task_id: None,
            notify_on_finish: true,
            goal_streak_days: 0,
            last_goal_date: None,
            empty_queue_behavior: EmptyQueueBehavior::FreeFocus,
            log_breaks: false,
            estimate_complete: EstimateCompleteBehavior::Nudge,
            show_terminal_title: true,
            warn_one_minute: true,
            auto_pause_idle_minutes: 0,
            archive_after_days: default_archive_days(),
            weekly_streak_weeks: 0,
            monthly_streak_months: 0,
            last_weekly_streak_key: None,
            last_monthly_streak_key: None,
            timer_presets: default_timer_presets(),
            active_preset: None,
        }
    }
}

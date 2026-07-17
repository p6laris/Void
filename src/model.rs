use chrono::{DateTime, Utc};
use indexmap::IndexMap;
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

    pub fn is_open(&self) -> bool {
        self.status != TaskStatus::Done && !self.archived
    }

    pub fn subtask(&self, id: u64) -> Option<&Subtask> {
        self.subtasks.iter().find(|s| s.id == id)
    }

    pub fn subtask_mut(&mut self, id: u64) -> Option<&mut Subtask> {
        self.subtasks.iter_mut().find(|s| s.id == id)
    }

    pub fn is_blocked(&self, tasks: &IndexMap<u64, Task>) -> bool {
        self.blocked_by.iter().any(|&blocker_id| {
            tasks
                .get(&blocker_id)
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

    pub fn is_overdue_on(&self, today: &str) -> bool {
        if self.status == TaskStatus::Done {
            return false;
        }
        let Some(ref due) = self.due_date else {
            return false;
        };
        due.as_str() < today
    }

    pub fn is_overdue(&self) -> bool {
        let today = crate::date::today_str();
        self.is_overdue_on(&today)
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

    pub fn is_break(self) -> bool {
        matches!(self, TimerMode::ShortBreak | TimerMode::LongBreak)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSessionRecord {
    pub date: String,
    pub minutes: u32,
    pub task_id: Option<u64>,
    pub mode: TimerMode,
    pub completed_at: DateTime<Utc>,
    pub note: String,
    pub tags: Vec<String>,
    pub pause_count: u32,
    pub pause_seconds: u32,
}

impl Default for FocusSessionRecord {
    fn default() -> Self {
        Self {
            date: String::new(),
            minutes: 0,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            note: String::new(),
            tags: Vec::new(),
            pause_count: 0,
            pause_seconds: 0,
        }
    }
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

mod tasks_serde {
    use super::{IndexMap, Task};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(map: &IndexMap<u64, Task>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        map.values().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<IndexMap<u64, Task>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tasks: Vec<Task> = Vec::deserialize(deserializer)?;
        Ok(tasks.into_iter().map(|t| (t.id, t)).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    #[serde(with = "tasks_serde")]
    pub tasks: IndexMap<u64, Task>,
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
    /// Days of the week excluded from streak tracking (0=Mon .. 6=Sun).
    #[serde(default = "default_rest_days")]
    pub streak_rest_days: Vec<u8>,
    /// Currently available streak freezes (max 3).
    #[serde(default)]
    pub streak_freezes: u32,
    /// Streak value when the last freeze was awarded.
    #[serde(default)]
    pub last_freeze_earned_streak: u32,
}

impl AppData {
    pub fn task(&self, id: u64) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn task_mut(&mut self, id: u64) -> Option<&mut Task> {
        self.tasks.get_mut(&id)
    }

    pub fn task_at(&self, index: usize) -> Option<&Task> {
        self.tasks.get_index(index).map(|(_, task)| task)
    }
}

fn default_archive_days() -> u32 {
    30
}

/// Saturday (5) and Sunday (6) are rest days by default.
fn default_rest_days() -> Vec<u8> {
    vec![5, 6]
}

/// Hard cap on streak freezes.
pub const STREAK_FREEZE_MAX: u32 = 3;

fn default_timer_presets() -> Vec<TimerPreset> {
    vec![TimerPreset::deep_work(), TimerPreset::quick()]
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            tasks: IndexMap::new(),
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
            streak_rest_days: default_rest_days(),
            streak_freezes: 0,
            last_freeze_earned_streak: 0,
        }
    }
}

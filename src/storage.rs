use anyhow::Result;
use chrono::{Datelike, NaiveDate, Utc, Weekday};

use crate::db::Database;
use crate::model::{
    AppData, FocusSessionRecord, Priority, Subtask, Task, TaskRecurrence, TaskStatus, TimerMode,
    TimerPreset,
};

pub fn next_id(db: &Database, data: &mut AppData) -> Result<u64> {
    let id = data.next_id;
    data.next_id = data.next_id.saturating_add(1);
    db.set_setting("next_id", data.next_id.to_string())?;
    Ok(id)
}

pub fn ensure_today_reset(db: &Database, data: &mut AppData) -> Result<bool> {
    let today = crate::date::today_str();
    if data.today_date.as_deref() != Some(today.as_str()) {
        data.today_focus_minutes = 0;
        data.today_date = Some(today.clone());
        db.set_setting("today_focus_minutes", "0")?;
        db.set_setting("today_date", &today)?;
        db.set_setting("timer_completed_focus_sessions", "0")?;
        return Ok(true);
    }
    Ok(false)
}

pub fn parse_tags(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn normalize_due_date(input: &str, allow_past: bool) -> Result<Option<String>, String> {
    let s = input.trim();
    if s.is_empty() {
        return Ok(None);
    }
    match chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(parsed) => {
            if !allow_past {
                if parsed < crate::date::today_naive() {
                    return Err("Due date cannot be in the past.".into());
                }
            }
            Ok(Some(s.to_string()))
        }
        Err(_) => match s.to_lowercase().as_str() {
            "today" => Ok(Some(crate::date::today_str())),
            "tomorrow" => Ok(Some(crate::date::tomorrow_str())),
            _ => Err("Due date must be YYYY-MM-DD, 'today', or 'tomorrow'.".into()),
        },
    }
}

#[derive(Default)]
pub struct SessionMeta {
    pub note: String,
    pub tags: Vec<String>,
    pub pause_count: u32,
    pub pause_seconds: u32,
}

pub struct TaskPayload {
    pub title: String,
    pub notes: String,
    pub estimated_minutes: u32,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub due_date: Option<String>,
}

pub fn add_task_full(db: &Database, data: &mut AppData, payload: TaskPayload) -> Result<u64> {
    let id = next_id(db, data)?;
    let mut task = Task::new(id, payload.title);
    task.notes = payload.notes;
    task.estimated_minutes = payload.estimated_minutes.clamp(1, 480);
    task.priority = payload.priority;
    task.tags = payload.tags;
    task.due_date = payload.due_date;
    db.upsert_task(&task)?;
    data.tasks.insert(task.id, task);
    Ok(id)
}

pub fn update_task(db: &Database, data: &mut AppData, id: u64, payload: TaskPayload) -> Result<()> {
    let Some(t) = data.task_mut(id) else {
        return Err(anyhow::anyhow!("task {id} not found"));
    };
    t.title = payload.title;
    t.notes = payload.notes;
    t.estimated_minutes = payload.estimated_minutes.clamp(1, 480);
    t.priority = payload.priority;
    t.tags = payload.tags;
    t.due_date = payload.due_date;
    db.upsert_task(t)?;
    Ok(())
}

pub fn delete_task(db: &Database, data: &mut AppData, id: u64) -> Result<bool> {
    if data.tasks.shift_remove(&id).is_none() {
        return Ok(false);
    }
    db.delete_task(id)?;
    if data.active_task_id == Some(id) {
        data.active_task_id = None;
        db.persist_active_task(None)?;
    }
    Ok(true)
}

pub fn promote_task_on_activate(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        if t.status == TaskStatus::Pending {
            t.status = TaskStatus::InProgress;
            db.upsert_task(t)?;
        }
    }
    Ok(())
}

pub fn mark_task_done(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    let (recurrence, title, notes, priority, tags, due_date, estimated, subtasks, blocked_by) = {
        let Some(t) = data.task(id) else {
            return Ok(());
        };
        (
            t.recurrence,
            t.title.clone(),
            t.notes.clone(),
            t.priority,
            t.tags.clone(),
            t.due_date.clone(),
            t.estimated_minutes,
            t.subtasks.clone(),
            t.blocked_by.clone(),
        )
    };
    if let Some(t) = data.task_mut(id) {
        t.status = TaskStatus::Done;
        t.completed_at = Some(Utc::now());
        db.upsert_task(t)?;
    }
    if recurrence != TaskRecurrence::None {
        spawn_recurring_task(
            db,
            data,
            RecurringSpawn {
                recurrence,
                title,
                notes,
                priority,
                tags,
                due_date,
                estimated,
                subtasks,
                blocked_by,
            },
        )?;
    }
    Ok(())
}

struct RecurringSpawn {
    recurrence: TaskRecurrence,
    title: String,
    notes: String,
    priority: Priority,
    tags: Vec<String>,
    due_date: Option<String>,
    estimated: u32,
    subtasks: Vec<Subtask>,
    blocked_by: Vec<u64>,
}

fn spawn_recurring_task(db: &Database, data: &mut AppData, spawn: RecurringSpawn) -> Result<()> {
    let RecurringSpawn {
        recurrence,
        title,
        notes,
        priority,
        tags,
        due_date,
        estimated,
        subtasks,
        blocked_by,
    } = spawn;
    let id = next_id(db, data)?;
    let mut task = Task::new(id, title);
    task.notes = notes;
    task.priority = priority;
    task.tags = tags;
    task.estimated_minutes = estimated;
    task.recurrence = recurrence;
    task.blocked_by = blocked_by;
    let mut respawned_subtasks = Vec::with_capacity(subtasks.len());
    for mut subtask in subtasks {
        subtask.id = next_id(db, data)?;
        subtask.done = false;
        respawned_subtasks.push(subtask);
    }
    task.subtasks = respawned_subtasks;
    task.due_date = next_due_date(recurrence, due_date.as_deref());
    db.upsert_task(&task)?;
    data.tasks.insert(task.id, task);
    Ok(())
}

fn next_due_date(recurrence: TaskRecurrence, current: Option<&str>) -> Option<String> {
    use chrono::{Datelike, NaiveDate, Weekday};
    let today = crate::date::today_naive();
    match recurrence {
        TaskRecurrence::None => current.map(String::from),
        TaskRecurrence::Daily => Some(crate::date::format_date(
            today + chrono::Duration::days(1),
        )),
        TaskRecurrence::Weekly => {
            let base = current
                .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .unwrap_or(today);
            Some(crate::date::format_date(
                base + chrono::Duration::days(7),
            ))
        }
        TaskRecurrence::Weekdays => {
            let mut d = today + chrono::Duration::days(1);
            while matches!(d.weekday(), Weekday::Sat | Weekday::Sun) {
                d += chrono::Duration::days(1);
            }
            Some(crate::date::format_date(d))
        }
    }
}

pub fn cycle_task_status(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        match t.status {
            TaskStatus::Pending => t.status = TaskStatus::InProgress,
            TaskStatus::InProgress => {
                t.status = TaskStatus::Done;
                t.completed_at = Some(Utc::now());
            }
            TaskStatus::Done => {
                t.status = TaskStatus::Pending;
                t.completed_at = None;
            }
        }
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn toggle_today(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.today = !t.today;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn set_priority(db: &Database, data: &mut AppData, id: u64, priority: Priority) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.priority = priority;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn move_task(db: &Database, data: &mut AppData, id: u64, delta: i32) -> Result<()> {
    let Some(idx) = data.tasks.get_index_of(&id) else {
        return Ok(());
    };
    let new_idx = (idx as i32 + delta).clamp(0, data.tasks.len() as i32 - 1) as usize;
    if idx != new_idx {
        data.tasks.move_index(idx, new_idx);
        for (i, (_, t)) in data.tasks.iter_mut().enumerate() {
            t.sort_order = i as u32;
        }
        db.sync_sort_orders(&data.tasks)?;
    }
    Ok(())
}

pub fn pick_best_task(data: &AppData) -> Option<u64> {
    data.tasks
        .values()
        .filter(|t| t.is_open())
        .max_by(|a, b| {
            a.priority
                .rank()
                .cmp(&b.priority.rank())
                .then(b.today.cmp(&a.today))
                .then(a.sort_order.cmp(&b.sort_order))
        })
        .map(|t| t.id)
}

pub fn advance_to_next_task(data: &AppData, current: Option<u64>) -> Option<u64> {
    let pending: Vec<&Task> = pending_tasks(data).collect();
    if pending.is_empty() {
        return None;
    }
    if let Some(cur) = current {
        if let Some(pos) = pending.iter().position(|t| t.id == cur) {
            let next = (pos + 1) % pending.len();
            return Some(pending[next].id);
        }
    }
    pick_best_task(data)
}

pub fn record_focus_session(
    db: &Database,
    data: &mut AppData,
    minutes: u32,
    task_id: Option<u64>,
    mode: TimerMode,
) -> Result<()> {
    record_focus_session_with_meta(db, data, minutes, task_id, mode, SessionMeta::default())
}

pub fn record_focus_session_with_meta(
    db: &Database,
    data: &mut AppData,
    minutes: u32,
    task_id: Option<u64>,
    mode: TimerMode,
    meta: SessionMeta,
) -> Result<()> {
    ensure_today_reset(db, data)?;
    let mins = minutes.max(1);
    data.total_focus_minutes = data.total_focus_minutes.saturating_add(mins);
    data.today_focus_minutes = data.today_focus_minutes.saturating_add(mins);
    data.total_sessions = data.total_sessions.saturating_add(1);

    let today = crate::date::today_str();
    match &data.last_session_date {
        Some(last) if last == &today => {}
        Some(last) => {
            let last_date = chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").ok();
            let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();
            if let (Some(l), Some(t)) = (last_date, today_date) {
                if l.succ_opt() == Some(t) {
                    data.streak_days = data.streak_days.saturating_add(1);
                } else if t != l {
                    data.streak_days = 1;
                }
            } else {
                data.streak_days = 1;
            }
        }
        None => data.streak_days = 1,
    }
    data.last_session_date = Some(today.clone());
    data.today_date = Some(today.clone());

    let record = FocusSessionRecord {
        date: today.clone(),
        minutes: mins,
        task_id,
        mode,
        completed_at: Utc::now(),
        note: meta.note,
        tags: meta.tags,
        pause_count: meta.pause_count,
        pause_seconds: meta.pause_seconds,
    };
    db.insert_focus_session(&record)?;
    update_goal_streak(data)?;
    update_period_streaks(data, &today)?;
    db.persist_session_stats(data)?;

    if let Some(id) = task_id {
        if let Some(t) = data.task_mut(id) {
            t.actual_minutes = t.actual_minutes.saturating_add(mins);
            t.sessions = t.sessions.saturating_add(1);
            if t.status == TaskStatus::Pending {
                t.status = TaskStatus::InProgress;
            }
            db.upsert_task(t)?;
        }
    }
    Ok(())
}

pub fn today_focus_minutes(data: &AppData) -> u32 {
    let today = crate::date::today_str();
    if data.today_date.as_deref() == Some(today.as_str()) {
        data.today_focus_minutes
    } else {
        0
    }
}

pub fn minutes_by_date(db: &Database, days: usize) -> Result<Vec<(String, u32)>> {
    db.minutes_by_date(days)
}

pub fn focus_heatmap(db: &Database) -> Result<Vec<(String, u32)>> {
    db.focus_minutes_grouped()
}

pub fn tag_analytics(db: &Database, data: &AppData, days: usize) -> Result<Vec<(String, u32)>> {
    db.tag_analytics(data, days)
}

pub fn pending_tasks(data: &AppData) -> impl Iterator<Item = &Task> {
    data.tasks.values().filter(|t| t.is_open())
}

pub fn sorted_pending_tasks(data: &AppData) -> Vec<&Task> {
    let mut tasks: Vec<&Task> = pending_tasks(data).collect();
    tasks.sort_by(|a, b| {
        b.priority
            .rank()
            .cmp(&a.priority.rank())
            .then(b.today.cmp(&a.today))
            .then(a.sort_order.cmp(&b.sort_order))
    });
    tasks
}

pub fn completed_tasks(data: &AppData) -> impl Iterator<Item = &Task> {
    data.tasks.values().filter(|t| t.status == TaskStatus::Done)
}

pub fn most_productive_hour_label(db: &Database) -> String {
    let hours = match db.session_minutes_by_local_hour() {
        Ok(hours) => hours,
        Err(_) => return "N/A".into(),
    };

    if let Some((hour, &mins)) = hours.iter().enumerate().max_by_key(|&(_, &c)| c) {
        if mins > 0 {
            let ampm = if hour < 12 { "AM" } else { "PM" };
            let h = if hour == 0 {
                12
            } else if hour > 12 {
                hour - 12
            } else {
                hour
            };
            return format!("{}{} ({}m)", h, ampm, mins);
        }
    }
    "N/A".into()
}

pub fn queue_empty(data: &AppData) -> bool {
    pending_tasks(data).next().is_none()
}

pub fn update_goal_streak(data: &mut AppData) -> Result<()> {
    let today = crate::date::today_str();
    if data.today_focus_minutes < data.daily_goal_minutes {
        return Ok(());
    }
    match &data.last_goal_date {
        Some(last) if last == &today => {}
        Some(last) => {
            let last_date = chrono::NaiveDate::parse_from_str(last, "%Y-%m-%d").ok();
            let today_date = chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").ok();
            if let (Some(l), Some(t)) = (last_date, today_date) {
                if l.succ_opt() == Some(t) {
                    data.goal_streak_days = data.goal_streak_days.saturating_add(1);
                } else if t != l {
                    data.goal_streak_days = 1;
                }
            } else {
                data.goal_streak_days = 1;
            }
        }
        None => data.goal_streak_days = 1,
    }
    data.last_goal_date = Some(today);
    Ok(())
}

pub fn record_break_session(
    db: &Database,
    data: &mut AppData,
    mode: TimerMode,
    minutes: u32,
) -> Result<()> {
    if !data.log_breaks {
        return Ok(());
    }
    let today = crate::date::today_str();
    let record = FocusSessionRecord {
        date: today,
        minutes: minutes.max(1),
        task_id: None,
        mode,
        completed_at: Utc::now(),
        note: String::new(),
        tags: Vec::new(),
        pause_count: 0,
        pause_seconds: 0,
    };
    db.insert_focus_session(&record)?;
    data.total_sessions = data.total_sessions.saturating_add(1);
    db.persist_session_stats(data)?;
    Ok(())
}

pub fn delete_session(db: &Database, data: &mut AppData, id: i64) -> Result<()> {
    let stored = db.get_session(id)?;
    let r = &stored.record;
    let today = crate::date::today_str();
    if matches!(r.mode, TimerMode::Focus | TimerMode::Custom) {
        data.total_focus_minutes = data.total_focus_minutes.saturating_sub(r.minutes);
        if r.date == today {
            data.today_focus_minutes = data.today_focus_minutes.saturating_sub(r.minutes);
        }
    }
    data.total_sessions = data.total_sessions.saturating_sub(1);
    if let Some(tid) = r.task_id {
        if let Some(t) = data.task_mut(tid) {
            t.actual_minutes = t.actual_minutes.saturating_sub(r.minutes);
            t.sessions = t.sessions.saturating_sub(1);
            db.upsert_task(t)?;
        }
    }
    db.delete_focus_session(id)?;
    db.persist_session_stats(data)?;
    Ok(())
}

pub fn adjust_session_minutes(
    db: &Database,
    data: &mut AppData,
    id: i64,
    new_minutes: u32,
) -> Result<()> {
    let stored = db.get_session(id)?;
    let old = stored.record.minutes;
    let new_minutes = new_minutes.clamp(1, 480);
    if old == new_minutes {
        return Ok(());
    }
    let today = crate::date::today_str();
    if matches!(stored.record.mode, TimerMode::Focus | TimerMode::Custom) {
        let delta = new_minutes as i32 - old as i32;
        if delta > 0 {
            data.total_focus_minutes = data.total_focus_minutes.saturating_add(delta as u32);
            if stored.record.date == today {
                data.today_focus_minutes = data.today_focus_minutes.saturating_add(delta as u32);
            }
        } else {
            data.total_focus_minutes = data.total_focus_minutes.saturating_sub((-delta) as u32);
            if stored.record.date == today {
                data.today_focus_minutes = data.today_focus_minutes.saturating_sub((-delta) as u32);
            }
        }
    }
    if let Some(tid) = stored.record.task_id {
        if let Some(t) = data.task_mut(tid) {
            if new_minutes > old {
                t.actual_minutes = t.actual_minutes.saturating_add(new_minutes - old);
            } else {
                t.actual_minutes = t.actual_minutes.saturating_sub(old - new_minutes);
            }
            db.upsert_task(t)?;
        }
    }
    db.update_session_minutes(id, new_minutes)?;
    update_goal_streak(data)?;
    db.persist_session_stats(data)?;
    Ok(())
}

pub fn sessions_remaining_hint(task: &Task, focus_minutes: u32) -> u32 {
    if task.estimated_minutes <= task.actual_minutes {
        return 0;
    }
    let left = task.estimated_minutes - task.actual_minutes;
    let session = focus_minutes.max(1);
    left.div_ceil(session)
}

pub fn archive_task(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.archived = true;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn unarchive_task(db: &Database, data: &mut AppData, id: u64) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.archived = false;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn auto_archive_old_tasks(db: &Database, data: &mut AppData) -> Result<u32> {
    let days = data.archive_after_days;
    if days == 0 {
        return Ok(0);
    }
    let cutoff = crate::date::format_date(
        crate::date::today_naive() - chrono::Duration::days(days as i64),
    );

    let to_archive: Vec<u64> = data
        .tasks
        .values()
        .filter(|t| t.status == TaskStatus::Done && !t.archived)
        .filter_map(|t| {
            t.completed_at.as_ref().and_then(|completed| {
                let key = crate::date::format_date(completed.date_naive());
                (key.as_str() < cutoff.as_str()).then_some(t.id)
            })
        })
        .collect();

    if to_archive.is_empty() {
        return Ok(0);
    }

    for id in &to_archive {
        if let Some(t) = data.task_mut(*id) {
            t.archived = true;
        }
    }

    let tasks_to_persist: Vec<&Task> = to_archive
        .iter()
        .filter_map(|id| data.task(*id))
        .collect();
    db.upsert_tasks(&tasks_to_persist)?;

    Ok(to_archive.len() as u32)
}

pub fn archived_tasks(data: &AppData) -> impl Iterator<Item = &Task> {
    data.tasks.values().filter(|t| t.archived)
}

pub fn toggle_subtask(
    db: &Database,
    data: &mut AppData,
    task_id: u64,
    subtask_id: u64,
) -> Result<()> {
    if let Some(t) = data.task_mut(task_id) {
        if let Some(s) = t.subtask_mut(subtask_id) {
            s.done = !s.done;
            db.upsert_task(t)?;
        }
    }
    Ok(())
}

pub fn add_subtask(db: &Database, data: &mut AppData, task_id: u64, title: String) -> Result<()> {
    let id = next_id(db, data)?;
    if let Some(t) = data.task_mut(task_id) {
        t.subtasks.push(Subtask {
            id,
            title,
            done: false,
        });
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn delete_subtask(
    db: &Database,
    data: &mut AppData,
    task_id: u64,
    subtask_id: u64,
) -> Result<()> {
    if let Some(t) = data.task_mut(task_id) {
        let before = t.subtasks.len();
        t.subtasks.retain(|s| s.id != subtask_id);
        if t.subtasks.len() != before {
            db.upsert_task(t)?;
        }
    }
    Ok(())
}

pub fn set_task_recurrence(
    db: &Database,
    data: &mut AppData,
    id: u64,
    recurrence: TaskRecurrence,
) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.recurrence = recurrence;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn set_blocked_by(
    db: &Database,
    data: &mut AppData,
    id: u64,
    blockers: Vec<u64>,
) -> Result<()> {
    if let Some(t) = data.task_mut(id) {
        t.blocked_by = blockers;
        db.upsert_task(t)?;
    }
    Ok(())
}

pub fn bulk_mark_done(db: &Database, data: &mut AppData, ids: &[u64]) -> Result<u32> {
    let mut count = 0;
    for &id in ids {
        mark_task_done(db, data, id)?;
        count += 1;
    }
    Ok(count)
}

pub fn bulk_delete(db: &Database, data: &mut AppData, ids: &[u64]) -> Result<u32> {
    let mut count = 0;
    for &id in ids {
        if delete_task(db, data, id)? {
            count += 1;
        }
    }
    Ok(count)
}

pub fn overdue_and_due_today(data: &AppData) -> (Vec<u64>, Vec<u64>) {
    let today = crate::date::today_str();
    let mut overdue = Vec::new();
    let mut due_today = Vec::new();
    for t in data.tasks.values().filter(|t| t.is_open()) {
        if let Some(ref due) = t.due_date {
            if due.as_str() < today.as_str() {
                overdue.push(t.id);
            } else if due.as_str() == today.as_str() {
                due_today.push(t.id);
            }
        }
    }
    (overdue, due_today)
}

pub fn focus_score(data: &AppData) -> u32 {
    let today_mins = today_focus_minutes(data);
    let goal = data.daily_goal_minutes.max(1);
    let goal_pct = ((today_mins as f64 / goal as f64) * 40.0).min(40.0);

    let ratio_pct = if data.total_focus_minutes > 0 {
        30.0
    } else {
        0.0
    };

    let streak_pct = (data.streak_days.min(14) as f64 / 14.0) * 30.0;

    (goal_pct + ratio_pct + streak_pct)
        .round()
        .clamp(0.0, 100.0) as u32
}

fn update_period_streaks(data: &mut AppData, today: &str) -> Result<()> {
    let today_date = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok();
    let Some(today_date) = today_date else {
        return Ok(());
    };

    let week_key = format!("{}-W{:02}", today_date.year(), today_date.iso_week().week());
    match &data.last_weekly_streak_key {
        Some(last) if last == &week_key => {}
        Some(last) => {
            if is_consecutive_week(last, &week_key) {
                data.weekly_streak_weeks = data.weekly_streak_weeks.saturating_add(1);
            } else {
                data.weekly_streak_weeks = 1;
            }
            data.last_weekly_streak_key = Some(week_key);
        }
        None => {
            data.weekly_streak_weeks = 1;
            data.last_weekly_streak_key = Some(week_key);
        }
    }

    let month_key = format!("{}-{:02}", today_date.year(), today_date.month());
    match &data.last_monthly_streak_key {
        Some(last) if last == &month_key => {}
        Some(last) => {
            if is_consecutive_month(last, &month_key) {
                data.monthly_streak_months = data.monthly_streak_months.saturating_add(1);
            } else {
                data.monthly_streak_months = 1;
            }
            data.last_monthly_streak_key = Some(month_key);
        }
        None => {
            data.monthly_streak_months = 1;
            data.last_monthly_streak_key = Some(month_key);
        }
    }
    Ok(())
}

fn is_consecutive_week(prev: &str, cur: &str) -> bool {
    parse_week_key(prev)
        .and_then(|(year, week)| week_start(year, week))
        .zip(
            parse_week_key(cur)
                .and_then(|(year, week)| week_start(year, week)),
        )
        .is_some_and(|(prev_start, cur_start)| {
            prev_start.checked_add_signed(chrono::TimeDelta::days(7)) == Some(cur_start)
        })
}

fn is_consecutive_month(prev: &str, cur: &str) -> bool {
    parse_month_key(prev)
        .zip(parse_month_key(cur))
        .is_some_and(|((prev_year, prev_month), (cur_year, cur_month))| {
            let (next_year, next_month) = if prev_month == 12 {
                (prev_year + 1, 1)
            } else {
                (prev_year, prev_month + 1)
            };
            next_year == cur_year && next_month == cur_month
        })
}

fn parse_week_key(key: &str) -> Option<(i32, u32)> {
    let (year, week) = key.split_once("-W")?;
    Some((year.parse().ok()?, week.parse().ok()?))
}

fn parse_month_key(key: &str) -> Option<(i32, u32)> {
    let (year, month) = key.split_once('-')?;
    Some((year.parse().ok()?, month.parse().ok()?))
}

fn week_start(year: i32, week: u32) -> Option<NaiveDate> {
    NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)
}

pub fn apply_timer_preset(data: &mut AppData, preset: &TimerPreset) {
    data.focus_minutes = preset.focus_minutes;
    data.short_break_minutes = preset.short_break_minutes;
    data.long_break_minutes = preset.long_break_minutes;
    data.long_break_every = preset.long_break_every;
    data.active_preset = Some(preset.name.clone());
}

pub fn cycle_timer_preset(data: &mut AppData) -> Option<TimerPreset> {
    if data.timer_presets.is_empty() {
        return None;
    }
    let next = match &data.active_preset {
        None => data.timer_presets[0].clone(),
        Some(name) => {
            let idx = data
                .timer_presets
                .iter()
                .position(|p| &p.name == name)
                .unwrap_or(0);
            data.timer_presets[(idx + 1) % data.timer_presets.len()].clone()
        }
    };
    apply_timer_preset(data, &next);
    Some(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorted_pending_tasks() {
        let mut data = AppData::default();
        let mut t1 = Task::new(1, "Task 1".into());
        t1.priority = Priority::Low;

        let mut t2 = Task::new(2, "Task 2".into());
        t2.priority = Priority::High;

        data.tasks.insert(t1.id, t1);
        data.tasks.insert(t2.id, t2);

        let sorted = sorted_pending_tasks(&data);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].id, 2); // High priority first
        assert_eq!(sorted[1].id, 1);
    }

    #[test]
    fn update_task_errors_on_unknown_id() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        let err = update_task(
            &db,
            &mut data,
            99,
            TaskPayload {
                title: "nope".into(),
                notes: String::new(),
                estimated_minutes: 25,
                priority: Priority::Medium,
                tags: vec![],
                due_date: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn auto_archive_old_tasks_persists_archived_tasks() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        data.archive_after_days = 30;

        let mut old = Task::new(1, "Old done".into());
        old.status = TaskStatus::Done;
        old.completed_at = Some(Utc::now() - chrono::Duration::days(60));
        db.upsert_task(&old).unwrap();

        let mut recent = Task::new(2, "Recent done".into());
        recent.status = TaskStatus::Done;
        recent.completed_at = Some(Utc::now());
        db.upsert_task(&recent).unwrap();

        data.tasks.insert(old.id, old);
        data.tasks.insert(recent.id, recent);

        let count = auto_archive_old_tasks(&db, &mut data).unwrap();
        assert_eq!(count, 1);
        assert!(data.task(1).unwrap().archived);
        assert!(!data.task(2).unwrap().archived);

        let loaded = db.load_app_data().unwrap();
        assert!(loaded.task(1).unwrap().archived);
        assert!(!loaded.task(2).unwrap().archived);
    }

    #[test]
    fn recurring_spawn_assigns_unique_subtask_ids() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        data.next_id = 2;

        let mut task = Task::new(1, "Daily".into());
        task.recurrence = TaskRecurrence::Daily;
        task.status = TaskStatus::Pending;
        task.subtasks = vec![
            Subtask {
                id: 1001,
                title: "Step one".into(),
                done: true,
            },
            Subtask {
                id: 1002,
                title: "Step two".into(),
                done: false,
            },
        ];
        db.upsert_task(&task).unwrap();
        data.tasks.insert(task.id, task);

        mark_task_done(&db, &mut data, 1).unwrap();

        let spawned = data.task(2).expect("spawned task");
        assert_eq!(spawned.subtasks.len(), 2);
        let ids: Vec<u64> = spawned.subtasks.iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![3, 4]);
        assert!(ids.iter().all(|id| *id != 2 * 1000 + 1 && *id != 2 * 1000 + 2));

        let loaded = db.load_app_data().unwrap();
        let all_subtask_ids: Vec<u64> = loaded
            .tasks
            .values()
            .flat_map(|t| t.subtasks.iter().map(|s| s.id))
            .collect();
        assert_eq!(
            all_subtask_ids.len(),
            all_subtask_ids.iter().collect::<std::collections::HashSet<_>>().len()
        );
    }

    #[test]
    fn test_pick_best_task() {
        let mut data = AppData::default();
        assert_eq!(pick_best_task(&data), None);

        let mut t = Task::new(1, "Task 1".into());
        t.status = TaskStatus::Pending;
        data.tasks.insert(t.id, t);

        assert_eq!(pick_best_task(&data), Some(1));

        let mut archived = Task::new(2, "Archived".into());
        archived.status = TaskStatus::Pending;
        archived.priority = Priority::High;
        archived.archived = true;
        data.tasks.insert(archived.id, archived);

        assert_eq!(pick_best_task(&data), Some(1));
    }

    fn sample_old_session() -> FocusSessionRecord {
        FocusSessionRecord {
            date: "2020-01-01".into(),
            minutes: 25,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            ..FocusSessionRecord::default()
        }
    }

    #[test]
    fn delete_session_does_not_adjust_today_focus_for_old_sessions() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        let today = crate::date::today_str();
        data.today_date = Some(today);
        data.today_focus_minutes = 50;
        data.total_focus_minutes = 100;

        let id = db.insert_focus_session(&sample_old_session()).unwrap();
        delete_session(&db, &mut data, id).unwrap();

        assert_eq!(data.today_focus_minutes, 50);
        assert_eq!(data.total_focus_minutes, 75);
    }

    #[test]
    fn adjust_session_minutes_does_not_adjust_today_focus_for_old_sessions() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        let today = crate::date::today_str();
        data.today_date = Some(today);
        data.today_focus_minutes = 50;
        data.total_focus_minutes = 100;

        let id = db.insert_focus_session(&sample_old_session()).unwrap();
        adjust_session_minutes(&db, &mut data, id, 30).unwrap();

        assert_eq!(data.today_focus_minutes, 50);
        assert_eq!(data.total_focus_minutes, 105);
    }

    #[test]
    fn consecutive_week_across_year_boundary() {
        assert!(is_consecutive_week("2025-W52", "2026-W01"));
    }

    #[test]
    fn consecutive_month_across_year_boundary() {
        assert!(is_consecutive_month("2025-12", "2026-01"));
    }

    #[test]
    fn non_consecutive_week_gap() {
        assert!(!is_consecutive_week("2025-W50", "2026-W01"));
    }

    #[test]
    fn non_consecutive_month_gap() {
        assert!(!is_consecutive_month("2025-10", "2026-01"));
    }

    #[test]
    fn consecutive_month_within_year() {
        assert!(is_consecutive_month("2025-06", "2025-07"));
    }
}

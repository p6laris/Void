mod import_export;
mod schema;

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};

use crate::model::{
    AppData, EmptyQueueBehavior, EstimateCompleteBehavior, FocusSessionRecord, Priority,
    StoredSession, Subtask, Task, TaskRecurrence, TaskStatus, TimerMode,
};
use crate::theme;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        let _existed = path.exists();
        let conn = Connection::open(&path).context("opening SQLite database")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrate(&conn)?;

        let db = Self { conn };
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        schema::migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn load_app_data(&self) -> Result<AppData> {
        let mut data = AppData::default();
        load_settings(&self.conn, &mut data)?;
        data.tasks = load_tasks(&self.conn)?;
        data.session_history = Vec::new();
        Ok(data)
    }

    pub fn save_app_data(&self, data: &AppData) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        save_settings(&tx, data)?;
        sync_tasks(&tx, &data.tasks)?;
        tx.commit()?;
        Ok(())
    }

    pub fn insert_focus_session(&self, record: &FocusSessionRecord) -> Result<i64> {
        insert_focus_session_conn(&self.conn, record)
    }

    pub fn get_session(&self, id: i64) -> Result<StoredSession> {
        let record = self.conn.query_row(
            "SELECT date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds
             FROM focus_sessions WHERE id = ?1",
            params![id],
            |row| {
                let mode_str: String = row.get(3)?;
                Ok(FocusSessionRecord {
                    date: row.get(0)?,
                    minutes: row.get(1)?,
                    task_id: read_opt_u64(row, 2)?,
                    mode: decode_timer_mode(&mode_str),
                    completed_at: parse_datetime_sql(&row.get::<_, String>(4)?)?,
                    note: row.get(5)?,
                    pause_count: row.get(6)?,
                    pause_seconds: row.get(7)?,
                    tags: Vec::new(),
                })
            },
        )?;
        Ok(StoredSession {
            id,
            record: FocusSessionRecord {
                tags: load_session_tags(&self.conn, id)?,
                ..record
            },
        })
    }

    pub fn delete_focus_session(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM focus_sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_session_minutes(&self, id: i64, minutes: u32) -> Result<()> {
        self.conn.execute(
            "UPDATE focus_sessions SET minutes = ?1 WHERE id = ?2",
            params![minutes, id],
        )?;
        Ok(())
    }

    pub fn recent_sessions(&self, limit: usize) -> Result<Vec<StoredSession>> {
        self.recent_sessions_paged(0, limit)
    }

    pub fn recent_sessions_paged(&self, offset: usize, limit: usize) -> Result<Vec<StoredSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds
             FROM focus_sessions
             ORDER BY completed_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt.query_map(params![limit as i64, offset as i64], |row| {
            let id: i64 = row.get(0)?;
            let mode_str: String = row.get(4)?;
            Ok((
                id,
                FocusSessionRecord {
                    date: row.get(1)?,
                    minutes: row.get(2)?,
                    task_id: read_opt_u64(row, 3)?,
                    mode: decode_timer_mode(&mode_str),
                    completed_at: parse_datetime_sql(&row.get::<_, String>(5)?)?,
                    note: row.get(6)?,
                    pause_count: row.get(7)?,
                    pause_seconds: row.get(8)?,
                    tags: Vec::new(),
                },
            ))
        })?;
        let tags_by_session = load_all_session_tags(&self.conn)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, mut record) = row?;
            record.tags = tags_by_session
                .get(&id)
                .cloned()
                .unwrap_or_default();
            out.push(StoredSession { id, record });
        }
        Ok(out)
    }

    pub fn session_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM focus_sessions", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn sessions_on_date(&self, date: &str) -> Result<Vec<StoredSession>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds
             FROM focus_sessions
             WHERE date = ?1
             ORDER BY completed_at ASC",
        )?;
        let rows = stmt.query_map(params![date], |row| {
            let id: i64 = row.get(0)?;
            let mode_str: String = row.get(4)?;
            Ok((
                id,
                FocusSessionRecord {
                    date: row.get(1)?,
                    minutes: row.get(2)?,
                    task_id: read_opt_u64(row, 3)?,
                    mode: decode_timer_mode(&mode_str),
                    completed_at: parse_datetime_sql(&row.get::<_, String>(5)?)?,
                    note: row.get(6)?,
                    pause_count: row.get(7)?,
                    pause_seconds: row.get(8)?,
                    tags: Vec::new(),
                },
            ))
        })?;
        let tags_by_session = load_all_session_tags(&self.conn)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, mut record) = row?;
            record.tags = tags_by_session
                .get(&id)
                .cloned()
                .unwrap_or_default();
            out.push(StoredSession { id, record });
        }
        Ok(out)
    }

    pub fn session_counts_by_mode(&self) -> Result<(u32, u32, u32)> {
        let focus: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE mode = ?1",
            params![encode_timer_mode(TimerMode::Focus)],
            |row| row.get(0),
        )?;
        let custom: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE mode = ?1",
            params![encode_timer_mode(TimerMode::Custom)],
            |row| row.get(0),
        )?;
        let breaks: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE mode IN (?1, ?2)",
            params![
                encode_timer_mode(TimerMode::ShortBreak),
                encode_timer_mode(TimerMode::LongBreak),
            ],
            |row| row.get(0),
        )?;
        Ok((focus, custom, breaks))
    }

    pub fn load_timer_state(&self) -> (u32, TimerMode) {
        let count: u32 = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'timer_completed_focus_sessions'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mode = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'timer_mode'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .map(|s| decode_timer_mode(&s))
            .unwrap_or(TimerMode::Focus);
        (count, mode)
    }

    pub fn persist_timer_state(&self, completed: u32, mode: TimerMode) -> Result<()> {
        self.set_setting("timer_completed_focus_sessions", completed.to_string())?;
        self.set_setting("timer_mode", encode_timer_mode(mode))?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: impl AsRef<str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value.as_ref()],
        )?;
        Ok(())
    }

    pub fn upsert_task(&self, task: &Task) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        upsert_task_row(&tx, task)?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_task(&self, id: u64) -> Result<()> {
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id as i64])?;
        Ok(())
    }

    pub fn sync_sort_orders(&self, tasks: &[Task]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for task in tasks {
            tx.execute(
                "UPDATE tasks SET sort_order = ?1 WHERE id = ?2",
                params![task.sort_order, task.id as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn persist_session_stats(&self, data: &AppData) -> Result<()> {
        self.set_setting("total_focus_minutes", data.total_focus_minutes.to_string())?;
        self.set_setting("total_sessions", data.total_sessions.to_string())?;
        self.set_setting("streak_days", data.streak_days.to_string())?;
        self.set_setting(
            "last_session_date",
            data.last_session_date.clone().unwrap_or_default(),
        )?;
        self.set_setting("today_focus_minutes", data.today_focus_minutes.to_string())?;
        self.set_setting("today_date", data.today_date.clone().unwrap_or_default())?;
        self.set_setting("goal_streak_days", data.goal_streak_days.to_string())?;
        self.set_setting(
            "last_goal_date",
            data.last_goal_date.clone().unwrap_or_default(),
        )?;
        Ok(())
    }

    pub fn persist_timer_settings(&self, data: &AppData) -> Result<()> {
        self.set_setting("focus_minutes", data.focus_minutes.to_string())?;
        self.set_setting("short_break_minutes", data.short_break_minutes.to_string())?;
        self.set_setting("long_break_minutes", data.long_break_minutes.to_string())?;
        self.set_setting("long_break_every", data.long_break_every.to_string())?;
        Ok(())
    }

    pub fn persist_active_task(&self, id: Option<u64>) -> Result<()> {
        let value = id.map(|i| i.to_string()).unwrap_or_default();
        self.set_setting("active_task_id", value)
    }

    pub fn export_json(&self) -> Result<PathBuf> {
        import_export::export_json(&self.conn)
    }

    pub fn import_json(&self, path: &std::path::Path) -> Result<()> {
        let conn = self.conn.unchecked_transaction()?;
        import_export::import_json(&conn, path)?;
        conn.commit()?;
        Ok(())
    }

    pub fn minutes_by_date(&self, days: usize) -> Result<Vec<(String, u32)>> {
        if days == 0 {
            return Ok(Vec::new());
        }
        let today = chrono::Local::now().date_naive();
        let start = today - chrono::Duration::days((days - 1) as i64);
        let by_date = self.focus_minutes_in_range(
            &start.format("%Y-%m-%d").to_string(),
            &today.format("%Y-%m-%d").to_string(),
        )?;
        let mut out = Vec::with_capacity(days);
        for offset in (0..days).rev() {
            let date = today - chrono::Duration::days(offset as i64);
            let key = date.format("%Y-%m-%d").to_string();
            let mins = by_date.get(&key).copied().unwrap_or(0);
            let label = date.format("%a").to_string();
            out.push((label, mins));
        }
        Ok(out)
    }

    /// Daily focus minutes keyed by `YYYY-MM-DD` (oldest first).
    pub fn focus_minutes_series(&self, days: usize) -> Result<Vec<(String, u32)>> {
        if days == 0 {
            return Ok(Vec::new());
        }
        let today = chrono::Local::now().date_naive();
        let start = today - chrono::Duration::days((days - 1) as i64);
        let by_date = self.focus_minutes_in_range(
            &start.format("%Y-%m-%d").to_string(),
            &today.format("%Y-%m-%d").to_string(),
        )?;
        let mut out = Vec::with_capacity(days);
        for offset in (0..days).rev() {
            let date = today - chrono::Duration::days(offset as i64);
            let key = date.format("%Y-%m-%d").to_string();
            let mins = by_date.get(&key).copied().unwrap_or(0);
            out.push((key, mins));
        }
        Ok(out)
    }

    /// All days with logged focus/custom minutes from the database.
    pub fn focus_minutes_grouped(&self) -> Result<Vec<(String, u32)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, COALESCE(SUM(minutes), 0) AS mins
             FROM focus_sessions
             WHERE mode IN (?1, ?2)
             GROUP BY date
             ORDER BY date ASC",
        )?;
        let rows = stmt.query_map(
            params![
                encode_timer_mode(TimerMode::Focus),
                encode_timer_mode(TimerMode::Custom),
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
        )?;
        rows.collect::<Result<Vec<_>, _>>()
            .context("loading focus minutes")
    }

    /// Tag analytics for the past N days.
    pub fn tag_analytics(&self, data: &AppData, days: usize) -> Result<Vec<(String, u32)>> {
        let cutoff = chrono::Local::now().date_naive() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let mut stmt = self.conn.prepare(
            "SELECT minutes, task_id
             FROM focus_sessions
             WHERE mode IN (?1, ?2) AND date >= ?3",
        )?;

        let rows = stmt.query_map(
            params![
                encode_timer_mode(TimerMode::Focus),
                encode_timer_mode(TimerMode::Custom),
                cutoff_str,
            ],
            |row| Ok((row.get::<_, u32>(0)?, read_opt_u64(row, 1)?)),
        )?;

        let mut tag_mins: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

        for row in rows {
            let (mins, task_id) = row?;
            if let Some(tid) = task_id {
                if let Some(task) = data.tasks.iter().find(|t| t.id == tid) {
                    if task.tags.is_empty() {
                        *tag_mins.entry("Untagged".to_string()).or_default() += mins;
                    } else {
                        for tag in &task.tags {
                            *tag_mins.entry(tag.clone()).or_default() += mins;
                        }
                    }
                } else {
                    *tag_mins.entry("Unknown".to_string()).or_default() += mins;
                }
            } else {
                *tag_mins.entry("No Task".to_string()).or_default() += mins;
            }
        }

        let mut out: Vec<_> = tag_mins.into_iter().collect();
        out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(out)
    }

    fn focus_minutes_in_range(
        &self,
        start: &str,
        end: &str,
    ) -> Result<HashMap<String, u32>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, COALESCE(SUM(minutes), 0) AS mins
             FROM focus_sessions
             WHERE date >= ?1 AND date <= ?2 AND mode IN (?3, ?4)
             GROUP BY date",
        )?;
        let rows = stmt.query_map(
            params![
                start,
                end,
                encode_timer_mode(TimerMode::Focus),
                encode_timer_mode(TimerMode::Custom),
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)),
        )?;
        let mut map = HashMap::new();
        for row in rows {
            let (date, mins) = row?;
            map.insert(date, mins);
        }
        Ok(map)
    }
}

pub fn db_path() -> Result<PathBuf> {
    let dir = data_dir()?;
    Ok(dir.join("void.db"))
}

fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .context("could not resolve local data directory")?;
    let focus_dir = dir.join("void");
    std::fs::create_dir_all(&focus_dir).context("creating data directory")?;
    Ok(focus_dir)
}

// ── settings ─────────────────────────────────────────────────────────────────

pub(crate) fn load_settings(conn: &Connection, data: &mut AppData) -> Result<()> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (key, value) = row?;
        apply_setting(data, &key, &value);
    }
    Ok(())
}

fn save_settings(conn: &Connection, data: &AppData) -> Result<()> {
    let pairs: Vec<(&str, String)> = vec![
        ("next_id", data.next_id.to_string()),
        ("total_focus_minutes", data.total_focus_minutes.to_string()),
        ("total_sessions", data.total_sessions.to_string()),
        ("streak_days", data.streak_days.to_string()),
        (
            "last_session_date",
            data.last_session_date.clone().unwrap_or_default(),
        ),
        ("daily_goal_minutes", data.daily_goal_minutes.to_string()),
        ("sound_enabled", bool_str(data.sound_enabled)),
        ("auto_start_breaks", bool_str(data.auto_start_breaks)),
        ("auto_start_focus", bool_str(data.auto_start_focus)),
        ("today_focus_minutes", data.today_focus_minutes.to_string()),
        ("today_date", data.today_date.clone().unwrap_or_default()),
        ("focus_minutes", data.focus_minutes.to_string()),
        ("short_break_minutes", data.short_break_minutes.to_string()),
        ("long_break_minutes", data.long_break_minutes.to_string()),
        ("long_break_every", data.long_break_every.to_string()),
        ("auto_pick_task", bool_str(data.auto_pick_task)),
        ("auto_advance_task", bool_str(data.auto_advance_task)),
        ("theme", data.theme.clone()),
        (
            "active_task_id",
            data.active_task_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
        ),
        ("notify_on_finish", bool_str(data.notify_on_finish)),
        ("goal_streak_days", data.goal_streak_days.to_string()),
        (
            "last_goal_date",
            data.last_goal_date.clone().unwrap_or_default(),
        ),
        (
            "empty_queue_behavior",
            encode_empty_queue(data.empty_queue_behavior).to_string(),
        ),
        ("log_breaks", bool_str(data.log_breaks)),
        (
            "estimate_complete",
            encode_estimate_complete(data.estimate_complete).to_string(),
        ),
        ("show_terminal_title", bool_str(data.show_terminal_title)),
        ("warn_one_minute", bool_str(data.warn_one_minute)),
        (
            "auto_pause_idle_minutes",
            data.auto_pause_idle_minutes.to_string(),
        ),
        ("archive_after_days", data.archive_after_days.to_string()),
        ("weekly_streak_weeks", data.weekly_streak_weeks.to_string()),
        (
            "monthly_streak_months",
            data.monthly_streak_months.to_string(),
        ),
        (
            "last_weekly_streak_key",
            data.last_weekly_streak_key.clone().unwrap_or_default(),
        ),
        (
            "last_monthly_streak_key",
            data.last_monthly_streak_key.clone().unwrap_or_default(),
        ),
        (
            "timer_presets",
            serde_json::to_string(&data.timer_presets).unwrap_or_default(),
        ),
        (
            "active_preset",
            data.active_preset.clone().unwrap_or_default(),
        ),
    ];

    for (key, value) in pairs {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
    }
    Ok(())
}

fn apply_setting(data: &mut AppData, key: &str, value: &str) {
    match key {
        "next_id" => data.next_id = parse_u64(value, data.next_id),
        "total_focus_minutes" => {
            data.total_focus_minutes = parse_u32(value, data.total_focus_minutes)
        }
        "total_sessions" => data.total_sessions = parse_u32(value, data.total_sessions),
        "streak_days" => data.streak_days = parse_u32(value, data.streak_days),
        "last_session_date" => data.last_session_date = opt_string(value),
        "daily_goal_minutes" => data.daily_goal_minutes = parse_u32(value, data.daily_goal_minutes),
        "sound_enabled" => data.sound_enabled = parse_bool(value, data.sound_enabled),
        "auto_start_breaks" => data.auto_start_breaks = parse_bool(value, data.auto_start_breaks),
        "auto_start_focus" => data.auto_start_focus = parse_bool(value, data.auto_start_focus),
        "today_focus_minutes" => {
            data.today_focus_minutes = parse_u32(value, data.today_focus_minutes)
        }
        "today_date" => data.today_date = opt_string(value),
        "focus_minutes" => data.focus_minutes = parse_u32(value, data.focus_minutes),
        "short_break_minutes" => {
            data.short_break_minutes = parse_u32(value, data.short_break_minutes)
        }
        "long_break_minutes" => data.long_break_minutes = parse_u32(value, data.long_break_minutes),
        "long_break_every" => data.long_break_every = parse_u32(value, data.long_break_every),
        "auto_pick_task" => data.auto_pick_task = parse_bool(value, data.auto_pick_task),
        "auto_advance_task" => data.auto_advance_task = parse_bool(value, data.auto_advance_task),
        "theme" if !value.is_empty() => {
            data.theme = theme::normalize_theme_id(value);
        }
        "active_task_id" => data.active_task_id = value.parse().ok(),
        "notify_on_finish" => data.notify_on_finish = parse_bool(value, data.notify_on_finish),
        "goal_streak_days" => data.goal_streak_days = parse_u32(value, data.goal_streak_days),
        "last_goal_date" => data.last_goal_date = opt_string(value),
        "empty_queue_behavior" => {
            data.empty_queue_behavior =
                decode_empty_queue(value).unwrap_or(data.empty_queue_behavior)
        }
        "log_breaks" => data.log_breaks = parse_bool(value, data.log_breaks),
        "estimate_complete" => {
            data.estimate_complete =
                decode_estimate_complete(value).unwrap_or(data.estimate_complete)
        }
        "show_terminal_title" => {
            data.show_terminal_title = parse_bool(value, data.show_terminal_title)
        }
        "warn_one_minute" => data.warn_one_minute = parse_bool(value, data.warn_one_minute),
        "auto_pause_idle_minutes" => {
            data.auto_pause_idle_minutes = parse_u32(value, data.auto_pause_idle_minutes)
        }
        "archive_after_days" => data.archive_after_days = parse_u32(value, data.archive_after_days),
        "weekly_streak_weeks" => {
            data.weekly_streak_weeks = parse_u32(value, data.weekly_streak_weeks)
        }
        "monthly_streak_months" => {
            data.monthly_streak_months = parse_u32(value, data.monthly_streak_months)
        }
        "last_weekly_streak_key" => data.last_weekly_streak_key = opt_string(value),
        "last_monthly_streak_key" => data.last_monthly_streak_key = opt_string(value),
        "timer_presets" if !value.is_empty() => {
            if let Ok(presets) = serde_json::from_str(value) {
                data.timer_presets = presets;
            }
        }
        "active_preset" => data.active_preset = opt_string(value),
        _ => {}
    }
}

// ── tasks ────────────────────────────────────────────────────────────────────

pub(crate) fn load_tasks(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, notes, priority, status, estimated_minutes, actual_minutes,
                sessions, created_at, completed_at, due_date, today, sort_order,
                archived, recurrence
         FROM tasks
         ORDER BY sort_order ASC, id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Task {
            id: read_u64(row, 0)?,
            title: row.get(1)?,
            notes: row.get(2)?,
            priority: decode_priority(&row.get::<_, String>(3)?),
            status: decode_task_status(&row.get::<_, String>(4)?),
            estimated_minutes: row.get(5)?,
            actual_minutes: row.get(6)?,
            sessions: row.get(7)?,
            created_at: parse_datetime_sql(&row.get::<_, String>(8)?)?,
            completed_at: row
                .get::<_, Option<String>>(9)?
                .map(|s| parse_datetime_sql(&s))
                .transpose()?,
            due_date: row.get::<_, Option<String>>(10)?,
            today: row.get::<_, i32>(11)? != 0,
            sort_order: row.get(12)?,
            archived: row.get::<_, i32>(13)? != 0,
            recurrence: decode_recurrence(&row.get::<_, String>(14)?),
            subtasks: Vec::new(),
            blocked_by: Vec::new(),
            tags: Vec::new(),
        })
    })?;

    let tags_by_task = load_all_task_tags(conn)?;
    let subtasks_by_task = load_all_subtasks(conn)?;
    let blocked_by_task = load_all_blocked_by(conn)?;

    let mut tasks = Vec::new();
    for row in rows {
        let mut task = row?;
        task.tags = tags_by_task
            .get(&task.id)
            .cloned()
            .unwrap_or_default();
        task.subtasks = subtasks_by_task
            .get(&task.id)
            .cloned()
            .unwrap_or_default();
        task.blocked_by = blocked_by_task
            .get(&task.id)
            .cloned()
            .unwrap_or_default();
        tasks.push(task);
    }
    Ok(tasks)
}

fn load_all_task_tags(conn: &Connection) -> Result<HashMap<u64, Vec<String>>> {
    let mut stmt =
        conn.prepare("SELECT task_id, tag FROM task_tags ORDER BY task_id ASC, tag ASC")?;
    let rows = stmt.query_map([], |row| Ok((read_u64(row, 0)?, row.get::<_, String>(1)?)))?;
    let mut map: HashMap<u64, Vec<String>> = HashMap::new();
    for row in rows {
        let (task_id, tag) = row?;
        map.entry(task_id).or_default().push(tag);
    }
    Ok(map)
}

fn load_all_subtasks(conn: &Connection) -> Result<HashMap<u64, Vec<Subtask>>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, id, title, done FROM subtasks ORDER BY task_id ASC, sort_order ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            read_u64(row, 0)?,
            Subtask {
                id: read_u64(row, 1)?,
                title: row.get(2)?,
                done: row.get::<_, i32>(3)? != 0,
            },
        ))
    })?;
    let mut map: HashMap<u64, Vec<Subtask>> = HashMap::new();
    for row in rows {
        let (task_id, subtask) = row?;
        map.entry(task_id).or_default().push(subtask);
    }
    Ok(map)
}

fn load_all_blocked_by(conn: &Connection) -> Result<HashMap<u64, Vec<u64>>> {
    let mut stmt = conn.prepare(
        "SELECT task_id, blocker_id FROM task_blocked_by ORDER BY task_id ASC, blocker_id ASC",
    )?;
    let rows = stmt.query_map([], |row| Ok((read_u64(row, 0)?, read_u64(row, 1)?)))?;
    let mut map: HashMap<u64, Vec<u64>> = HashMap::new();
    for row in rows {
        let (task_id, blocker_id) = row?;
        map.entry(task_id).or_default().push(blocker_id);
    }
    Ok(map)
}

fn sync_tasks(conn: &Connection, tasks: &[Task]) -> Result<()> {
    conn.execute("DELETE FROM task_tags", [])?;
    conn.execute("DELETE FROM task_blocked_by", [])?;
    conn.execute("DELETE FROM subtasks", [])?;
    conn.execute("DELETE FROM tasks", [])?;
    for task in tasks {
        upsert_task_row(conn, task)?;
    }
    Ok(())
}

fn upsert_task_row(conn: &Connection, task: &Task) -> Result<()> {
    conn.execute(
        "INSERT INTO tasks (
            id, title, notes, priority, status, estimated_minutes, actual_minutes,
            sessions, created_at, completed_at, due_date, today, sort_order,
            archived, recurrence
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(id) DO UPDATE SET
            title = excluded.title,
            notes = excluded.notes,
            priority = excluded.priority,
            status = excluded.status,
            estimated_minutes = excluded.estimated_minutes,
            actual_minutes = excluded.actual_minutes,
            sessions = excluded.sessions,
            created_at = excluded.created_at,
            completed_at = excluded.completed_at,
            due_date = excluded.due_date,
            today = excluded.today,
            sort_order = excluded.sort_order,
            archived = excluded.archived,
            recurrence = excluded.recurrence",
        params![
            task.id as i64,
            task.title,
            task.notes,
            encode_priority(task.priority),
            encode_task_status(task.status),
            task.estimated_minutes,
            task.actual_minutes,
            task.sessions,
            task.created_at.to_rfc3339(),
            task.completed_at.map(|dt| dt.to_rfc3339()),
            task.due_date,
            if task.today { 1 } else { 0 },
            task.sort_order,
            if task.archived { 1 } else { 0 },
            encode_recurrence(task.recurrence),
        ],
    )?;
    conn.execute(
        "DELETE FROM task_tags WHERE task_id = ?1",
        params![task.id as i64],
    )?;
    for tag in &task.tags {
        conn.execute(
            "INSERT INTO task_tags (task_id, tag) VALUES (?1, ?2)",
            params![task.id as i64, tag],
        )?;
    }
    conn.execute(
        "DELETE FROM subtasks WHERE task_id = ?1",
        params![task.id as i64],
    )?;
    for (i, sub) in task.subtasks.iter().enumerate() {
        conn.execute(
            "INSERT INTO subtasks (id, task_id, title, done, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                sub.id as i64,
                task.id as i64,
                sub.title,
                if sub.done { 1 } else { 0 },
                i as i64,
            ],
        )?;
    }
    conn.execute(
        "DELETE FROM task_blocked_by WHERE task_id = ?1",
        params![task.id as i64],
    )?;
    for blocker_id in &task.blocked_by {
        conn.execute(
            "INSERT INTO task_blocked_by (task_id, blocker_id) VALUES (?1, ?2)",
            params![task.id as i64, *blocker_id as i64],
        )?;
    }
    Ok(())
}

fn insert_focus_session_conn(conn: &Connection, record: &FocusSessionRecord) -> Result<i64> {
    conn.execute(
        "INSERT INTO focus_sessions (date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            record.date,
            record.minutes,
            record.task_id.map(|id| id as i64),
            encode_timer_mode(record.mode),
            record.completed_at.to_rfc3339(),
            record.note,
            record.pause_count,
            record.pause_seconds,
        ],
    )?;
    let id = conn.last_insert_rowid();
    for tag in &record.tags {
        conn.execute(
            "INSERT INTO session_tags (session_id, tag) VALUES (?1, ?2)",
            params![id, tag],
        )?;
    }
    Ok(id)
}

fn load_session_tags(conn: &Connection, session_id: i64) -> Result<Vec<String>> {
    let mut stmt =
        conn.prepare("SELECT tag FROM session_tags WHERE session_id = ?1 ORDER BY tag ASC")?;
    let tags = stmt
        .query_map(params![session_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(tags)
}

fn load_all_session_tags(conn: &Connection) -> Result<HashMap<i64, Vec<String>>> {
    let mut stmt = conn.prepare(
        "SELECT session_id, tag FROM session_tags ORDER BY session_id ASC, tag ASC",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))?;
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    for row in rows {
        let (session_id, tag) = row?;
        map.entry(session_id).or_default().push(tag);
    }
    Ok(map)
}

fn encode_recurrence(r: TaskRecurrence) -> &'static str {
    match r {
        TaskRecurrence::None => "none",
        TaskRecurrence::Daily => "daily",
        TaskRecurrence::Weekly => "weekly",
        TaskRecurrence::Weekdays => "weekdays",
    }
}

fn decode_recurrence(s: &str) -> TaskRecurrence {
    match s {
        "daily" => TaskRecurrence::Daily,
        "weekly" => TaskRecurrence::Weekly,
        "weekdays" => TaskRecurrence::Weekdays,
        _ => TaskRecurrence::None,
    }
}

// ── encoding ─────────────────────────────────────────────────────────────────

fn encode_priority(p: Priority) -> &'static str {
    match p {
        Priority::Low => "low",
        Priority::Medium => "medium",
        Priority::High => "high",
    }
}

fn decode_priority(s: &str) -> Priority {
    match s {
        "high" => Priority::High,
        "low" => Priority::Low,
        _ => Priority::Medium,
    }
}

fn encode_task_status(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Pending => "pending",
        TaskStatus::InProgress => "inprogress",
        TaskStatus::Done => "done",
    }
}

fn decode_task_status(s: &str) -> TaskStatus {
    match s {
        "done" => TaskStatus::Done,
        "inprogress" | "in_progress" => TaskStatus::InProgress,
        _ => TaskStatus::Pending,
    }
}

fn encode_timer_mode(m: TimerMode) -> &'static str {
    match m {
        TimerMode::Focus => "focus",
        TimerMode::ShortBreak => "shortbreak",
        TimerMode::LongBreak => "longbreak",
        TimerMode::Custom => "custom",
    }
}

fn decode_timer_mode(s: &str) -> TimerMode {
    match s {
        "shortbreak" | "short_break" => TimerMode::ShortBreak,
        "longbreak" | "long_break" => TimerMode::LongBreak,
        "custom" => TimerMode::Custom,
        _ => TimerMode::Focus,
    }
}

fn encode_empty_queue(b: EmptyQueueBehavior) -> &'static str {
    match b {
        EmptyQueueBehavior::FreeFocus => "free-focus",
        EmptyQueueBehavior::PauseTimer => "pause-timer",
        EmptyQueueBehavior::AskEachTime => "ask",
    }
}

fn decode_empty_queue(s: &str) -> Option<EmptyQueueBehavior> {
    Some(match s {
        "pause-timer" => EmptyQueueBehavior::PauseTimer,
        "ask" => EmptyQueueBehavior::AskEachTime,
        _ => EmptyQueueBehavior::FreeFocus,
    })
}

fn encode_estimate_complete(b: EstimateCompleteBehavior) -> &'static str {
    match b {
        EstimateCompleteBehavior::Nudge => "nudge",
        EstimateCompleteBehavior::None => "none",
        EstimateCompleteBehavior::AutoDone => "auto-done",
    }
}

fn decode_estimate_complete(s: &str) -> Option<EstimateCompleteBehavior> {
    Some(match s {
        "none" => EstimateCompleteBehavior::None,
        "auto-done" => EstimateCompleteBehavior::AutoDone,
        _ => EstimateCompleteBehavior::Nudge,
    })
}

pub(crate) fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .with_context(|| format!("invalid RFC3339 timestamp: {s:?}"))
}

fn parse_datetime_sql(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    parse_datetime(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
        )
    })
}

fn bool_str(v: bool) -> String {
    if v { "1" } else { "0" }.to_string()
}

fn parse_bool(s: &str, default: bool) -> bool {
    match s {
        "1" | "true" | "yes" => true,
        "0" | "false" | "no" => false,
        _ => default,
    }
}

pub(crate) fn read_u64(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<u64> {
    Ok(row.get::<_, i64>(idx)? as u64)
}

pub(crate) fn read_opt_u64(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(idx)?;
    Ok(value.map(|id| id as u64))
}

fn parse_u32(s: &str, default: u32) -> u32 {
    s.parse().unwrap_or(default)
}

fn parse_u64(s: &str, default: u64) -> u64 {
    s.parse().unwrap_or(default)
}

fn opt_string(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FocusSessionRecord, Subtask, Task, TimerMode};
    use chrono::Utc;

    #[test]
    fn test_db_in_memory_initialization() {
        let db = Database::open_in_memory().expect("failed to open in memory db");
        let data = db.load_app_data().expect("failed to load app data");
        assert_eq!(data.tasks.len(), 0);
    }

    #[test]
    fn test_save_and_load_tasks() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();
        let mut t = Task::new(1, "Test Task".into());
        t.priority = Priority::High;
        data.tasks.push(t);

        db.save_app_data(&data).unwrap();

        let loaded = db.load_app_data().unwrap();
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].title, "Test Task");
        assert_eq!(loaded.tasks[0].priority, Priority::High);
    }

    #[test]
    fn load_tasks_round_trips_tags_subtasks_and_blockers() {
        let db = Database::open_in_memory().unwrap();
        let mut data = AppData::default();

        let mut blocked = Task::new(1, "Blocker".into());
        blocked.status = TaskStatus::Pending;
        let mut task = Task::new(2, "Main".into());
        task.tags = vec!["focus".into(), "work".into()];
        task.subtasks = vec![
            Subtask {
                id: 201,
                title: "Step one".into(),
                done: false,
            },
            Subtask {
                id: 202,
                title: "Step two".into(),
                done: true,
            },
        ];
        task.blocked_by = vec![1];
        data.tasks = vec![blocked, task];

        db.save_app_data(&data).unwrap();
        let loaded = db.load_app_data().unwrap();

        assert_eq!(loaded.tasks.len(), 2);
        let main = loaded.tasks.iter().find(|t| t.id == 2).expect("main task");
        assert_eq!(main.tags, vec!["focus", "work"]);
        assert_eq!(main.subtasks.len(), 2);
        assert_eq!(main.subtasks[0].title, "Step one");
        assert!(main.subtasks[1].done);
        assert_eq!(main.blocked_by, vec![1]);
    }

    #[test]
    fn session_loaders_round_trip_tags() {
        let db = Database::open_in_memory().unwrap();
        let record = FocusSessionRecord {
            date: "2026-07-02".into(),
            minutes: 25,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            note: String::new(),
            tags: vec!["deep".into(), "work".into()],
            pause_count: 0,
            pause_seconds: 0,
        };
        db.insert_focus_session(&record).unwrap();

        let recent = db.recent_sessions_paged(0, 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].record.tags, vec!["deep", "work"]);

        let on_date = db.sessions_on_date("2026-07-02").unwrap();
        assert_eq!(on_date.len(), 1);
        assert_eq!(on_date[0].record.tags, vec!["deep", "work"]);
    }

    #[test]
    fn minutes_by_date_aggregates_focus_minutes_in_one_query() {
        let db = Database::open_in_memory().unwrap();
        let today = chrono::Local::now().date_naive();
        let yesterday = today - chrono::TimeDelta::days(1);
        let yesterday_key = yesterday.format("%Y-%m-%d").to_string();

        db.insert_focus_session(&FocusSessionRecord {
            date: yesterday_key.clone(),
            minutes: 25,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            note: String::new(),
            tags: Vec::new(),
            pause_count: 0,
            pause_seconds: 0,
        })
        .unwrap();
        db.insert_focus_session(&FocusSessionRecord {
            date: yesterday_key,
            minutes: 10,
            task_id: None,
            mode: TimerMode::ShortBreak,
            completed_at: Utc::now(),
            note: String::new(),
            tags: Vec::new(),
            pause_count: 0,
            pause_seconds: 0,
        })
        .unwrap();

        let chart = db.minutes_by_date(7).unwrap();
        assert_eq!(chart.len(), 7);
        let yesterday_label = yesterday.format("%a").to_string();
        let yesterday_mins = chart
            .iter()
            .find(|(label, _)| label == &yesterday_label)
            .map(|(_, mins)| *mins);
        assert_eq!(yesterday_mins, Some(25));

        let series = db.focus_minutes_series(7).unwrap();
        let today_key = today.format("%Y-%m-%d").to_string();
        assert_eq!(
            series.iter().find(|(key, _)| key == &today_key).map(|(_, m)| *m),
            Some(0)
        );
    }

    #[test]
    fn parse_datetime_rejects_invalid_input() {
        let err = parse_datetime("not-a-timestamp").unwrap_err();
        assert!(err.to_string().contains("invalid RFC3339 timestamp"));
    }

    #[test]
    fn load_tasks_fails_on_corrupt_created_at() {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn.execute(
            "INSERT INTO tasks (
                id, title, notes, priority, status, estimated_minutes, actual_minutes,
                sessions, created_at, completed_at, due_date, today, sort_order, archived, recurrence
             ) VALUES (1, 't', '', 'medium', 'pending', 25, 0, 0, 'bad', NULL, NULL, 0, 0, 0, 'none')",
            [],
        )
        .unwrap();

        let err = load_tasks(&conn).unwrap_err();
        assert!(err.to_string().contains("invalid RFC3339 timestamp"));
    }
}

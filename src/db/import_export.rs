use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::model::{AppData, FocusSessionRecord, TimerMode};

use super::{data_dir, load_settings, load_tasks, parse_datetime, read_opt_u64};

pub fn export_json(conn: &Connection) -> Result<PathBuf> {
    let mut data = AppData::default();
    load_settings(conn, &mut data)?;
    data.tasks = load_tasks(conn)?;
    data.session_history = load_all_sessions(conn)?;

    let path = data_dir()?.join("data.json");
    let raw = serde_json::to_string_pretty(&data).context("serializing export")?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &raw).context("writing export temp file")?;
    fs::rename(&tmp, &path).context("finalizing export")?;
    Ok(path)
}

fn load_all_sessions(conn: &Connection) -> Result<Vec<FocusSessionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT date, minutes, task_id, mode, completed_at
         FROM focus_sessions
         ORDER BY completed_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let mode_str: String = row.get(3)?;
        Ok(FocusSessionRecord {
            date: row.get(0)?,
            minutes: row.get(1)?,
            task_id: read_opt_u64(row, 2)?,
            mode: decode_timer_mode(&mode_str),
            completed_at: parse_datetime(&row.get::<_, String>(4)?),
            note: String::new(),
            tags: Vec::new(),
            pause_count: 0,
            pause_seconds: 0,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .context("loading sessions for export")
}

fn decode_timer_mode(s: &str) -> TimerMode {
    match s {
        "shortbreak" | "short_break" => TimerMode::ShortBreak,
        "longbreak" | "long_break" => TimerMode::LongBreak,
        "custom" => TimerMode::Custom,
        _ => TimerMode::Focus,
    }
}

fn encode_timer_mode(mode: TimerMode) -> &'static str {
    match mode {
        TimerMode::Focus => "focus",
        TimerMode::ShortBreak => "shortbreak",
        TimerMode::LongBreak => "longbreak",
        TimerMode::Custom => "custom",
    }
}

pub fn import_json(conn: &Connection, path: &std::path::Path) -> Result<()> {
    let raw = fs::read_to_string(path).context("reading import file")?;
    let data: AppData = serde_json::from_str(&raw).context("parsing import file")?;

    // Clear existing data and sync tasks
    super::sync_tasks(conn, &data.tasks).context("syncing tasks during import")?;

    // Save settings
    super::save_settings(conn, &data).context("saving settings during import")?;

    // Restore focus session history
    conn.execute("DELETE FROM focus_sessions", [])?;
    let mut stmt = conn.prepare(
        "INSERT INTO focus_sessions (date, minutes, task_id, mode, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for record in &data.session_history {
        stmt.execute(rusqlite::params![
            record.date,
            record.minutes,
            record.task_id.map(|id| id as i64),
            encode_timer_mode(record.mode),
            record.completed_at.to_rfc3339(),
        ])?;
    }
    Ok(())
}

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::model::{AppData, FocusSessionRecord, TimerMode};

use super::{
    data_dir, insert_focus_session_conn, load_all_session_tags, load_settings, load_tasks,
    sessions::focus_session_id_and_record,
};

#[derive(Serialize)]
struct ExportSnapshot<'a> {
    #[serde(flatten)]
    data: &'a AppData,
    session_history: Vec<FocusSessionRecord>,
}

#[derive(Serialize, Deserialize)]
struct ImportSnapshot {
    #[serde(flatten)]
    data: AppData,
    #[serde(default)]
    session_history: Vec<FocusSessionRecord>,
}

pub fn export_json(conn: &Connection) -> Result<PathBuf> {
    let mut data = AppData::default();
    load_settings(conn, &mut data)?;
    data.tasks = load_tasks(conn)?;
    let snapshot = ExportSnapshot {
        data: &data,
        session_history: load_all_sessions(conn)?,
    };

    let path = data_dir()?.join("data.json");
    let raw = serde_json::to_string_pretty(&snapshot).context("serializing export")?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &raw).context("writing export temp file")?;
    fs::rename(&tmp, &path).context("finalizing export")?;
    Ok(path)
}

/// Exports the session history as CSV, for spreadsheets and external analysis.
///
/// JSON stays the backup/restore format; this is a one-way reporting export.
pub fn export_csv(conn: &Connection) -> Result<PathBuf> {
    let sessions = load_all_sessions(conn)?;

    let mut out = String::from("date,completed_at,minutes,mode,task_id,pause_count,pause_seconds,tags,note\n");
    for s in &sessions {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            csv_field(&s.date),
            csv_field(&s.completed_at.to_rfc3339()),
            s.minutes,
            csv_field(&format!("{:?}", s.mode)),
            s.task_id.map(|id| id.to_string()).unwrap_or_default(),
            s.pause_count,
            s.pause_seconds,
            csv_field(&s.tags.join(" ")),
            csv_field(&s.note),
        ));
    }

    let path = data_dir()?.join("sessions.csv");
    let tmp = path.with_extension("csv.tmp");
    fs::write(&tmp, out.as_bytes()).context("writing csv export temp file")?;
    fs::rename(&tmp, &path).context("finalizing csv export")?;
    Ok(path)
}

/// Quotes a CSV field when it contains a delimiter, quote or newline (RFC 4180).
fn csv_field(raw: &str) -> String {
    if raw.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
}

fn load_all_sessions(conn: &Connection) -> Result<Vec<FocusSessionRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds
         FROM focus_sessions
         ORDER BY completed_at ASC",
    )?;
    let rows = stmt.query_map([], focus_session_id_and_record)?;
    let tags_by_session = load_all_session_tags(conn)?;
    let mut out = Vec::new();
    for row in rows {
        let (id, mut record) = row?;
        record.tags = tags_by_session.get(&id).cloned().unwrap_or_default();
        out.push(record);
    }
    Ok(out)
}

pub fn import_json(conn: &Connection, path: &std::path::Path) -> Result<()> {
    let raw = fs::read_to_string(path).context("reading import file")?;
    let snapshot: ImportSnapshot = serde_json::from_str(&raw).context("parsing import file")?;

    super::sync_tasks(conn, &snapshot.data.tasks).context("syncing tasks during import")?;
    super::save_settings(conn, &snapshot.data).context("saving settings during import")?;

    conn.execute("DELETE FROM focus_sessions", [])?;
    for record in &snapshot.session_history {
        insert_focus_session_conn(conn, record)?;
    }
    super::schema::optimize(conn).context("optimizing database after import")?;
    Ok(())
}

pub fn import_csv(conn: &Connection, path: &std::path::Path) -> Result<usize> {
    let raw = fs::read_to_string(path).context("reading csv import file")?;
    let records = parse_csv_records(&raw);
    if records.is_empty() {
        return Ok(0);
    }

    let first_row = &records[0];
    let is_header = first_row.iter().any(|h| {
        let s = h.trim().to_lowercase();
        s == "date" || s == "minutes" || s == "mode" || s == "completed_at"
    });

    let (header_row, data_rows) = if is_header {
        (Some(first_row), &records[1..])
    } else {
        (None, &records[..])
    };

    let mut col_date = 0;
    let mut col_completed_at = 1;
    let mut col_minutes = 2;
    let mut col_mode = 3;
    let mut col_task_id = 4;
    let mut col_pause_count = 5;
    let mut col_pause_seconds = 6;
    let mut col_tags = 7;
    let mut col_note = 8;

    if let Some(header) = header_row {
        for (i, col_name) in header.iter().enumerate() {
            let norm = col_name.trim().to_lowercase();
            match norm.as_str() {
                "date" => col_date = i,
                "completed_at" | "timestamp" | "datetime" | "time" => col_completed_at = i,
                "minutes" | "mins" | "duration" => col_minutes = i,
                "mode" | "type" => col_mode = i,
                "task_id" | "task" => col_task_id = i,
                "pause_count" | "pauses" => col_pause_count = i,
                "pause_seconds" | "paused_seconds" => col_pause_seconds = i,
                "tags" | "tag" => col_tags = i,
                "note" | "notes" | "description" => col_note = i,
                _ => {}
            }
        }
    }

    let mut imported = 0;
    for row in data_rows {
        if row.is_empty() || row.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let date = row.get(col_date).map(|s| s.trim().to_string()).unwrap_or_default();
        if date.is_empty() {
            continue;
        }

        let completed_str = row.get(col_completed_at).map(|s| s.as_str()).unwrap_or("");
        let completed_at = parse_completed_at_or_fallback(completed_str, &date);

        let minutes = row
            .get(col_minutes)
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(25)
            .clamp(1, 1440);

        let mode_str = row.get(col_mode).map(|s| s.as_str()).unwrap_or("Focus");
        let mode = parse_mode_str(mode_str);

        let task_id = row.get(col_task_id).and_then(|s| s.trim().parse::<u64>().ok());
        let valid_task_id = if let Some(tid) = task_id {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM tasks WHERE id = ?1",
                    rusqlite::params![tid as i64],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if exists {
                Some(tid)
            } else {
                None
            }
        } else {
            None
        };

        let pause_count = row
            .get(col_pause_count)
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let pause_seconds = row
            .get(col_pause_seconds)
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);

        let tags = row
            .get(col_tags)
            .map(|s| parse_csv_tags(s))
            .unwrap_or_default();

        let note = row.get(col_note).cloned().unwrap_or_default();

        let record = FocusSessionRecord {
            date,
            minutes,
            task_id: valid_task_id,
            mode,
            completed_at,
            note,
            tags,
            pause_count,
            pause_seconds,
        };

        insert_focus_session_conn(conn, &record)?;
        imported += 1;
    }

    super::schema::optimize(conn).context("optimizing database after csv import")?;
    Ok(imported)
}

fn parse_mode_str(s: &str) -> TimerMode {
    match s.trim().to_lowercase().as_str() {
        "focus" => TimerMode::Focus,
        "shortbreak" | "short_break" | "short break" | "break" => TimerMode::ShortBreak,
        "longbreak" | "long_break" | "long break" => TimerMode::LongBreak,
        "custom" => TimerMode::Custom,
        _ => TimerMode::Focus,
    }
}

fn parse_csv_tags(input: &str) -> Vec<String> {
    input
        .split(|c| c == ',' || c == ' ')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_completed_at_or_fallback(completed_str: &str, date_str: &str) -> chrono::DateTime<chrono::Utc> {
    use chrono::Utc;
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(completed_str.trim()) {
        return dt.with_timezone(&Utc);
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(completed_str.trim(), "%Y-%m-%d %H:%M:%S") {
        return chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc);
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str.trim(), "%Y-%m-%d") {
        if let Some(naive_dt) = d.and_hms_opt(12, 0, 0) {
            return chrono::DateTime::<Utc>::from_naive_utc_and_offset(naive_dt, Utc);
        }
    }
    Utc::now()
}

pub fn parse_csv_records(input: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut current_record = Vec::new();
    let mut current_field = String::new();
    let mut inside_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if inside_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current_field.push('"');
                } else {
                    inside_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else {
            match c {
                '"' => {
                    inside_quotes = true;
                }
                ',' => {
                    current_record.push(std::mem::take(&mut current_field));
                }
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    current_record.push(std::mem::take(&mut current_field));
                    records.push(std::mem::take(&mut current_record));
                }
                '\n' => {
                    current_record.push(std::mem::take(&mut current_field));
                    records.push(std::mem::take(&mut current_record));
                }
                _ => {
                    current_field.push(c);
                }
            }
        }
    }

    if !current_field.is_empty() || !current_record.is_empty() {
        current_record.push(current_field);
        records.push(current_record);
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    use crate::db::schema;
    use crate::model::{AppData, TimerMode};

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn csv_field_quotes_only_when_needed() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field(""), "");
        // A comma would otherwise split into an extra column.
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        // Embedded quotes double up, per RFC 4180.
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
    }

    #[test]
    fn load_all_sessions_includes_v2_metadata() {
        let conn = mem_conn();
        let record = FocusSessionRecord {
            date: "2026-07-02".into(),
            minutes: 25,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            note: "deep work".into(),
            tags: vec!["code".into(), "focus".into()],
            pause_count: 2,
            pause_seconds: 90,
        };
        insert_focus_session_conn(&conn, &record).unwrap();

        let loaded = load_all_sessions(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].note, "deep work");
        assert_eq!(loaded[0].tags, vec!["code", "focus"]);
        assert_eq!(loaded[0].pause_count, 2);
        assert_eq!(loaded[0].pause_seconds, 90);
    }

    #[test]
    fn import_json_restores_session_metadata() {
        let conn = mem_conn();
        let snapshot = ImportSnapshot {
            data: AppData::default(),
            session_history: vec![FocusSessionRecord {
                date: "2026-07-02".into(),
                minutes: 25,
                task_id: None,
                mode: TimerMode::Focus,
                completed_at: Utc::now(),
                note: "deep work".into(),
                tags: vec!["code".into(), "focus".into()],
                pause_count: 2,
                pause_seconds: 90,
            }],
        };
        let path = std::env::temp_dir().join("void_import_test.json");
        std::fs::write(
            &path,
            serde_json::to_string(&snapshot).expect("serializing test export"),
        )
        .unwrap();

        import_json(&conn, &path).unwrap();

        let loaded = load_all_sessions(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].note, "deep work");
        assert_eq!(loaded[0].tags, vec!["code", "focus"]);
        assert_eq!(loaded[0].pause_count, 2);
        assert_eq!(loaded[0].pause_seconds, 90);

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn import_json_rejects_legacy_session_format() {
        let conn = mem_conn();
        let json = serde_json::json!({
            "tasks": [],
            "total_focus_minutes": 0,
            "total_sessions": 0,
            "streak_days": 0,
            "last_session_date": null,
            "daily_goal_minutes": 120,
            "sound_enabled": true,
            "auto_start_breaks": false,
            "auto_start_focus": false,
            "next_id": 1,
            "session_history": [{
                "date": "2026-07-02",
                "minutes": 25,
                "task_id": null,
                "mode": "focus",
                "completed_at": "2026-07-02T12:00:00Z"
            }]
        });
        let path = std::env::temp_dir().join("void_legacy_import_test.json");
        std::fs::write(&path, serde_json::to_string(&json).unwrap()).unwrap();

        let err = import_json(&conn, &path).unwrap_err();
        assert!(err.to_string().contains("parsing import file"));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn parse_csv_records_handles_complex_quotes() {
        let csv = "date,completed_at,minutes,note\n2026-08-15,2026-08-15T10:00:00Z,25,\"hello, world\"\n2026-08-16,2026-08-16T11:00:00Z,50,\"say \"\"hi\"\"\"\n";
        let records = parse_csv_records(csv);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0], vec!["date", "completed_at", "minutes", "note"]);
        assert_eq!(records[1], vec!["2026-08-15", "2026-08-15T10:00:00Z", "25", "hello, world"]);
        assert_eq!(records[2], vec!["2026-08-16", "2026-08-16T11:00:00Z", "50", "say \"hi\""]);
    }

    #[test]
    fn import_csv_restores_sessions_and_metadata() {
        let conn = mem_conn();

        // Insert task 42 so the foreign key is present
        let task = crate::model::Task::new(42, "Task 42".into());
        super::super::sync_tasks(&conn, &indexmap::indexmap! { 42 => task }).unwrap();

        let csv_data = "date,completed_at,minutes,mode,task_id,pause_count,pause_seconds,tags,note\n\
                        2026-08-15,2026-08-15T14:30:00Z,30,Focus,42,1,15,rust cli,deep work session\n\
                        2026-08-15,2026-08-15T15:00:00Z,5,ShortBreak,,0,0,,rest\n";
        let path = std::env::temp_dir().join("void_import_csv_test.csv");
        std::fs::write(&path, csv_data).unwrap();

        let count = import_csv(&conn, &path).unwrap();
        assert_eq!(count, 2);

        let loaded = load_all_sessions(&conn).unwrap();
        assert_eq!(loaded.len(), 2);

        assert_eq!(loaded[0].date, "2026-08-15");
        assert_eq!(loaded[0].minutes, 30);
        assert_eq!(loaded[0].mode, TimerMode::Focus);
        assert_eq!(loaded[0].task_id, Some(42));
        assert_eq!(loaded[0].pause_count, 1);
        assert_eq!(loaded[0].pause_seconds, 15);
        assert_eq!(loaded[0].tags, vec!["cli", "rust"]);
        assert_eq!(loaded[0].note, "deep work session");

        assert_eq!(loaded[1].mode, TimerMode::ShortBreak);
        assert_eq!(loaded[1].minutes, 5);
        assert_eq!(loaded[1].task_id, None);

        std::fs::remove_file(path).ok();
    }
}

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::model::{AppData, FocusSessionRecord};

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
}

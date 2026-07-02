use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::model::{AppData, FocusSessionRecord};

use super::{
    data_dir, decode_timer_mode, insert_focus_session_conn, load_session_tags, load_settings,
    load_tasks, parse_datetime, read_opt_u64,
};

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
        "SELECT id, date, minutes, task_id, mode, completed_at, note, pause_count, pause_seconds
         FROM focus_sessions
         ORDER BY completed_at ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let mode_str: String = row.get(4)?;
        Ok((
            id,
            FocusSessionRecord {
                date: row.get(1)?,
                minutes: row.get(2)?,
                task_id: read_opt_u64(row, 3)?,
                mode: decode_timer_mode(&mode_str),
                completed_at: parse_datetime(&row.get::<_, String>(5)?),
                note: row.get(6)?,
                pause_count: row.get(7)?,
                pause_seconds: row.get(8)?,
                tags: Vec::new(),
            },
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, mut record) = row?;
        record.tags = load_session_tags(conn, id)?;
        out.push(record);
    }
    Ok(out)
}

pub fn import_json(conn: &Connection, path: &std::path::Path) -> Result<()> {
    let raw = fs::read_to_string(path).context("reading import file")?;
    let data: AppData = serde_json::from_str(&raw).context("parsing import file")?;

    // Clear existing data and sync tasks
    super::sync_tasks(conn, &data.tasks).context("syncing tasks during import")?;

    // Save settings
    super::save_settings(conn, &data).context("saving settings during import")?;

    // Restore focus session history (cascades to session_tags)
    conn.execute("DELETE FROM focus_sessions", [])?;
    for record in &data.session_history {
        insert_focus_session_conn(conn, record)?;
    }
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
        let mut data = AppData::default();
        data.session_history.push(FocusSessionRecord {
            date: "2026-07-02".into(),
            minutes: 25,
            task_id: None,
            mode: TimerMode::Focus,
            completed_at: Utc::now(),
            note: "deep work".into(),
            tags: vec!["code".into(), "focus".into()],
            pause_count: 2,
            pause_seconds: 90,
        });
        let path = std::env::temp_dir().join("void_import_test.json");
        std::fs::write(
            &path,
            serde_json::to_string(&data).expect("serializing test export"),
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

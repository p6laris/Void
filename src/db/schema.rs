use anyhow::Result;
use rusqlite::Connection;

pub fn migrate(conn: &Connection) -> Result<()> {
    migrate_v1(conn)?;
    migrate_v2(conn)?;
    Ok(())
}

fn migrate_v1(conn: &Connection) -> Result<()> {
    const TARGET_VERSION: i32 = 1;
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < TARGET_VERSION {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tasks (
                id                INTEGER PRIMARY KEY,
                title             TEXT NOT NULL,
                notes             TEXT NOT NULL DEFAULT '',
                priority          TEXT NOT NULL,
                status            TEXT NOT NULL,
                estimated_minutes INTEGER NOT NULL,
                actual_minutes    INTEGER NOT NULL DEFAULT 0,
                sessions          INTEGER NOT NULL DEFAULT 0,
                created_at        TEXT NOT NULL,
                completed_at      TEXT,
                due_date          TEXT,
                today             INTEGER NOT NULL DEFAULT 0,
                sort_order        INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS task_tags (
                task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                tag     TEXT NOT NULL,
                PRIMARY KEY (task_id, tag)
            );

            CREATE TABLE IF NOT EXISTS focus_sessions (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                date         TEXT NOT NULL,
                minutes      INTEGER NOT NULL,
                task_id      INTEGER REFERENCES tasks(id) ON DELETE SET NULL,
                mode         TEXT NOT NULL,
                completed_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_focus_sessions_date ON focus_sessions(date);
            CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);

            PRAGMA user_version = 1;
            ",
        )?;
    }

    Ok(())
}

fn migrate_v2(conn: &Connection) -> Result<()> {
    const TARGET_VERSION: i32 = 2;
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version < TARGET_VERSION {
        conn.execute_batch(
            "
            ALTER TABLE tasks ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE tasks ADD COLUMN recurrence TEXT NOT NULL DEFAULT 'none';

            CREATE TABLE IF NOT EXISTS subtasks (
                id      INTEGER PRIMARY KEY,
                task_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                title   TEXT NOT NULL,
                done    INTEGER NOT NULL DEFAULT 0,
                sort_order INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS task_blocked_by (
                task_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                blocker_id INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
                PRIMARY KEY (task_id, blocker_id)
            );

            ALTER TABLE focus_sessions ADD COLUMN note TEXT NOT NULL DEFAULT '';
            ALTER TABLE focus_sessions ADD COLUMN pause_count INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE focus_sessions ADD COLUMN pause_seconds INTEGER NOT NULL DEFAULT 0;

            CREATE TABLE IF NOT EXISTS session_tags (
                session_id INTEGER NOT NULL REFERENCES focus_sessions(id) ON DELETE CASCADE,
                tag        TEXT NOT NULL,
                PRIMARY KEY (session_id, tag)
            );

            CREATE INDEX IF NOT EXISTS idx_tasks_archived ON tasks(archived);
            CREATE INDEX IF NOT EXISTS idx_subtasks_task ON subtasks(task_id);

            PRAGMA user_version = 2;
            ",
        )?;
    }

    Ok(())
}

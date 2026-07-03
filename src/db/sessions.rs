use rusqlite::Row;

use crate::model::FocusSessionRecord;

use super::encoding::decode_timer_mode;
use super::{parse_datetime_sql, read_opt_u64};

/// Map a `focus_sessions` row where columns start at `base` (date, minutes, …).
pub(crate) fn focus_session_from_row(row: &Row<'_>, base: usize) -> rusqlite::Result<FocusSessionRecord> {
    let mode_str: String = row.get(base + 3)?;
    Ok(FocusSessionRecord {
        date: row.get(base)?,
        minutes: row.get(base + 1)?,
        task_id: read_opt_u64(row, base + 2)?,
        mode: decode_timer_mode(&mode_str),
        completed_at: parse_datetime_sql(&row.get::<_, String>(base + 4)?)?,
        note: row.get(base + 5)?,
        pause_count: row.get(base + 6)?,
        pause_seconds: row.get(base + 7)?,
        tags: Vec::new(),
    })
}

/// Map a `focus_sessions` row that includes `id` in column 0.
pub(crate) fn focus_session_id_and_record(row: &Row<'_>) -> rusqlite::Result<(i64, FocusSessionRecord)> {
    let id: i64 = row.get(0)?;
    let record = focus_session_from_row(row, 1)?;
    Ok((id, record))
}

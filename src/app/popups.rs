use super::*;
use crate::model::Priority;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    pub fn close_popup(&mut self) {
        self.input.popup = None;
        self.input.input_mode = InputMode::Normal;
        self.input.input_buffer.clear();
        self.input.input_due_date.clear();
        self.input.input_tags.clear();
    }

    fn preserved_task_notes(&self, id: u64) -> String {
        self.data
            .task(id)
            .map(|t| t.notes.clone())
            .unwrap_or_default()
    }

    pub fn submit_popup(&mut self) {
        match self.input.popup.clone() {
            Some(Popup::AddTask) => {
                let title = self.input.input_buffer.trim().to_string();
                if title.is_empty() {
                    self.set_status("Title cannot be empty.", true);
                    return;
                }
                let due_date = match self.popup_due_date() {
                    Ok(d) => d,
                    Err(msg) => {
                        self.set_status(msg, true);
                        return;
                    }
                };
                let tags = self.popup_tags();
                if let Err(e) = storage::add_task_full(
                    &self.db,
                    &mut self.data,
                    storage::TaskPayload {
                        title,
                        notes: String::new(),
                        estimated_minutes: self.input.input_number.clamp(1, 480),
                        priority: self.input.input_priority,
                        tags,
                        due_date,
                    },
                ) {
                    self.set_status(format!("Save error: {e}"), true);
                    return;
                }
                self.bump_tasks();
                let indices = self.filtered_task_indices();
                let sel = indices.len().saturating_sub(1);
                self.task_ui
                    .task_state
                    .select(if indices.is_empty() { None } else { Some(sel) });
                self.close_popup();
                self.set_status("Task added.", false);
            }
            Some(Popup::EditTask(id)) => {
                let title = self.input.input_buffer.trim().to_string();
                if title.is_empty() {
                    self.set_status("Title cannot be empty.", true);
                    return;
                }
                let due_date = match self.popup_due_date() {
                    Ok(d) => d,
                    Err(msg) => {
                        self.set_status(msg, true);
                        return;
                    }
                };
                let tags = self.popup_tags();
                let estimate = self.input.input_number.clamp(1, 480);
                let priority = self.input.input_priority;
                let notes = self.preserved_task_notes(id);
                if let Err(e) = storage::update_task(
                    &self.db,
                    &mut self.data,
                    id,
                    storage::TaskPayload {
                        title,
                        notes,
                        estimated_minutes: estimate,
                        priority,
                        tags,
                        due_date,
                    },
                ) {
                    self.set_status(format!("Save error: {e}"), true);
                    return;
                }
                self.bump_tasks();
                self.close_popup();
                self.set_status("Task updated.", false);
            }
            Some(Popup::ConfirmDelete(id)) => {
                self.delete_task_confirmed(id);
                self.close_popup();
            }
            Some(Popup::EmptyQueueChoice) | Some(Popup::ConfirmQuit) => {}
            Some(Popup::AddSubtask(id)) => {
                let title = self.input.input_buffer.trim().to_string();
                if title.is_empty() {
                    self.set_status("Subtask title cannot be empty.", true);
                    return;
                }
                if let Err(e) = storage::add_subtask(&self.db, &mut self.data, id, title.clone()) {
                    self.set_status(format!("Save error: {e}"), true);
                    return;
                }
                self.bump_tasks();
                if let Some(t) = self.data.task(id) {
                    self.task_ui.subtask_selected = t.subtasks.len().saturating_sub(1);
                }
                self.input.input_buffer.clear();
                self.task_ui.subtask_focus = true;
                self.sync_subtask_list();
                self.set_status(
                    format!("Added \"{title}\" — type another or q to close"),
                    false,
                );
            }
            Some(Popup::EditSubtask(task_id, sub_id)) => {
                let title = self.input.input_buffer.trim().to_string();
                if title.is_empty() {
                    self.set_status("Subtask title cannot be empty.", true);
                    return;
                }
                if let Err(e) = storage::rename_subtask(
                    &self.db,
                    &mut self.data,
                    task_id,
                    sub_id,
                    title.clone(),
                ) {
                    self.set_status(format!("Save error: {e}"), true);
                    return;
                }
                self.bump_tasks();
                self.close_popup();
                self.set_status(format!("Subtask renamed to \"{title}\""), false);
            }
            Some(Popup::BulkConfirm(action)) => {
                let ids: Vec<u64> = self.task_ui.bulk_selected.iter().copied().collect();
                let result = match action {
                    BulkAction::MarkDone => storage::bulk_mark_done(&self.db, &mut self.data, &ids),
                    BulkAction::Delete => storage::bulk_delete(&self.db, &mut self.data, &ids),
                };
                match result {
                    Ok(n) => {
                        if self.task_ui.active_task.is_some_and(|id| ids.contains(&id)) {
                            self.set_active_task(None);
                        }
                        self.task_ui.bulk_selected.clear();
                        self.task_ui.bulk_mode = false;
                        self.bump_tasks();
                        self.clamp_task_selection_after_mutation();
                        self.set_status(format!("Bulk action applied to {n} tasks."), false);
                    }
                    Err(e) => self.set_status(format!("Bulk error: {e}"), true),
                }
                self.close_popup();
            }
            None => {}
        }
    }

    fn delete_task_confirmed(&mut self, id: u64) {
        match storage::delete_task(&self.db, &mut self.data, id) {
            Ok(true) => {
                if self.task_ui.active_task == Some(id) {
                    self.set_active_task(None);
                }
                self.bump_tasks();
                self.clamp_task_selection_after_mutation();
                self.set_status("Task deleted.", false);
                self.check_queue_empty();
            }
            Ok(false) => {}
            Err(e) => self.set_status(format!("Delete error: {e}"), true),
        }
    }

    pub fn confirm_delete(&mut self) {
        if let Some(Popup::ConfirmDelete(id)) = self.input.popup.clone() {
            self.delete_task_confirmed(id);
            self.close_popup();
        }
    }

    pub(crate) fn handle_popup_key(&mut self, key: KeyEvent) {
        if matches!(self.input.popup, Some(Popup::ConfirmQuit)) {
            match key.code {
                KeyCode::Char('l') | KeyCode::Char('L') | KeyCode::Enter => {
                    self.close_popup();
                    self.force_quit();
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.close_popup();
                    self.ui.should_quit = true;
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => self.close_popup(),
                _ => {}
            }
            return;
        }
        if matches!(self.input.popup, Some(Popup::EmptyQueueChoice)) {
            match key.code {
                KeyCode::Esc => self.close_popup(),
                KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.data.empty_queue_behavior = EmptyQueueBehavior::FreeFocus;
                    self.close_popup();
                    self.set_status(
                        "All tasks done — free focus. Sessions log as general focus.",
                        false,
                    );
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.data.empty_queue_behavior = EmptyQueueBehavior::PauseTimer;
                    self.close_popup();
                    if self.timer.state == TimerState::Running {
                        self.pause_timer();
                    } else {
                        self.timer.reset();
                    }
                    self.set_status("All tasks done — timer paused.", false);
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.close_popup();
                    self.open_add_task();
                }
                _ => {}
            }
            return;
        }
        if matches!(self.input.popup, Some(Popup::ConfirmDelete(_))) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                    self.close_popup();
                }
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.confirm_delete();
                }
                _ => {}
            }
            return;
        }
        if matches!(self.input.popup, Some(Popup::BulkConfirm(_))) {
            match key.code {
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => self.close_popup(),
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => self.submit_popup(),
                _ => {}
            }
            return;
        }
        if matches!(
            self.input.popup,
            Some(Popup::AddSubtask(_)) | Some(Popup::EditSubtask(_, _))
        ) {
            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.close_popup(),
                KeyCode::Enter => self.submit_popup(),
                KeyCode::Backspace => {
                    self.input.input_buffer.pop();
                }
                KeyCode::Char(c) if !ctrl => {
                    self.input.input_buffer.push(c);
                }
                _ => {}
            }
            return;
        }

        let is_text_field = matches!(
            self.input.input_field,
            InputField::Title | InputField::DueDate | InputField::Tags
        );
        match key.code {
            KeyCode::Esc => {
                self.close_popup();
            }
            KeyCode::Tab | KeyCode::BackTab => {
                let order = [
                    InputField::Title,
                    InputField::Estimate,
                    InputField::Priority,
                    InputField::DueDate,
                    InputField::Tags,
                ];
                let idx = order
                    .iter()
                    .position(|f| *f == self.input.input_field)
                    .unwrap_or(0);
                let next = if key.code == KeyCode::Tab {
                    (idx + 1) % order.len()
                } else {
                    (idx + order.len() - 1) % order.len()
                };
                self.input.input_field = order[next];
            }
            KeyCode::Enter => {
                self.submit_popup();
            }
            _ => {
                if is_text_field {
                    self.handle_text_input(key);
                } else {
                    self.handle_field_input(key);
                }
            }
        }
    }

    pub(crate) fn handle_text_input(&mut self, key: KeyEvent) {
        if self.input.input_field == InputField::DueDate {
            match key.code {
                KeyCode::Left => {
                    self.input.calendar_date -= chrono::Duration::days(1);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Right => {
                    self.input.calendar_date += chrono::Duration::days(1);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Up => {
                    self.input.calendar_date -= chrono::Duration::days(7);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Down => {
                    self.input.calendar_date += chrono::Duration::days(7);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.input.calendar_date = crate::date::today_naive();
                    self.input.input_due_date = crate::date::today_str();
                    return;
                }
                KeyCode::Char('m') | KeyCode::Char('M') => {
                    self.input.calendar_date =
                        crate::date::today_naive() + chrono::Duration::days(1);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    self.input.calendar_date =
                        crate::date::today_naive() + chrono::Duration::days(7);
                    self.input.input_due_date = crate::date::format_date(self.input.calendar_date);
                    return;
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.input.input_due_date.clear();
                    return;
                }
                _ => {}
            }
        }
        let buf = match self.input.input_field {
            InputField::Title => &mut self.input.input_buffer,
            InputField::DueDate => &mut self.input.input_due_date,
            InputField::Tags => &mut self.input.input_tags,
            _ => return,
        };
        match key.code {
            KeyCode::Backspace => {
                buf.pop();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                buf.push(c);
            }
            _ => {}
        }
        if self.input.input_field == InputField::DueDate {
            self.sync_calendar_to_due_date();
        }
    }

    /// Points the calendar at the typed due date once it parses, else today.
    pub(crate) fn sync_calendar_to_due_date(&mut self) {
        self.input.calendar_date = storage::normalize_due_date(&self.input.input_due_date, true)
            .ok()
            .flatten()
            .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok())
            .unwrap_or_else(crate::date::today_naive);
    }

    pub(crate) fn handle_field_input(&mut self, key: KeyEvent) {
        match self.input.input_field {
            InputField::Estimate => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let d = c.to_digit(10).unwrap_or(0);
                    self.input.input_number =
                        (self.input.input_number.saturating_mul(10) + d).min(480);
                }
                // May reach 0 while typing; clamped when the form is submitted.
                KeyCode::Backspace => self.input.input_number /= 10,
                KeyCode::Up => self.input.input_number = (self.input.input_number + 5).min(480),
                KeyCode::Down => {
                    self.input.input_number = self.input.input_number.saturating_sub(5).max(1)
                }
                _ => {}
            },
            InputField::Priority => {
                let next = match key.code {
                    KeyCode::Right | KeyCode::Up | KeyCode::Char(' ') => {
                        match self.input.input_priority {
                            Priority::Low => Priority::Medium,
                            Priority::Medium => Priority::High,
                            Priority::High => Priority::Low,
                        }
                    }
                    KeyCode::Left | KeyCode::Down => match self.input.input_priority {
                        Priority::Low => Priority::High,
                        Priority::High => Priority::Medium,
                        Priority::Medium => Priority::Low,
                    },
                    KeyCode::Char('1') => Priority::Low,
                    KeyCode::Char('2') => Priority::Medium,
                    KeyCode::Char('3') => Priority::High,
                    _ => self.input.input_priority,
                };
                self.input.input_priority = next;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::storage;
    use crossterm::event::KeyModifiers;

    fn app_with_one_task() -> (App, u64) {
        let db = Database::open_in_memory().unwrap();
        let mut app = App::with_database(db).unwrap();
        let id = storage::add_task_full(
            &app.db,
            &mut app.data,
            storage::TaskPayload {
                title: "Bulk me".into(),
                notes: String::new(),
                estimated_minutes: 25,
                priority: Priority::Medium,
                tags: Vec::new(),
                due_date: None,
            },
        )
        .unwrap();
        app.recompute_task_caches();
        app.set_active_task(Some(id));
        (app, id)
    }

    fn key(app: &mut App, code: KeyCode) {
        app.handle_popup_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    #[test]
    fn the_estimate_can_be_retyped_from_scratch() {
        let (mut app, _) = app_with_one_task();
        app.open_add_task();
        app.input.input_field = InputField::Estimate;
        key(&mut app, KeyCode::Backspace);
        key(&mut app, KeyCode::Backspace);
        key(&mut app, KeyCode::Char('4'));
        key(&mut app, KeyCode::Char('7'));
        assert_eq!(app.input.input_number, 47);
    }

    #[test]
    fn an_emptied_estimate_is_saved_as_one_minute() {
        let (mut app, _) = app_with_one_task();
        app.open_add_task();
        app.input.input_buffer = "Tiny".into();
        app.input.input_field = InputField::Estimate;
        key(&mut app, KeyCode::Backspace);
        key(&mut app, KeyCode::Backspace);
        app.submit_popup();
        let t = app.data.tasks.values().find(|t| t.title == "Tiny").unwrap();
        assert_eq!(t.estimated_minutes, 1);
    }

    #[test]
    fn editing_a_task_opens_the_calendar_on_its_due_date() {
        let (mut app, id) = app_with_one_task();
        app.data.task_mut(id).unwrap().due_date = Some("2031-05-17".into());
        app.recompute_task_caches();
        app.task_ui.task_state.select(Some(0));
        app.open_edit_task();
        assert_eq!(app.input.calendar_date.to_string(), "2031-05-17");
    }

    #[test]
    fn typing_a_due_date_moves_the_calendar() {
        let (mut app, _) = app_with_one_task();
        app.open_add_task();
        app.input.input_field = InputField::DueDate;
        for c in "2031-05-17".chars() {
            key(&mut app, KeyCode::Char(c));
        }
        assert_eq!(app.input.calendar_date.to_string(), "2031-05-17");
    }

    #[test]
    fn bulk_delete_of_the_active_task_clears_it() {
        let (mut app, id) = app_with_one_task();
        assert_eq!(app.task_ui.active_task, Some(id));

        app.task_ui.bulk_mode = true;
        app.task_ui.bulk_selected.insert(id);
        app.input.popup = Some(Popup::BulkConfirm(BulkAction::Delete));
        app.submit_popup();

        assert_eq!(
            app.task_ui.active_task, None,
            "a deleted task must not stay active"
        );
    }

    #[test]
    fn bulk_mark_done_of_the_active_task_clears_it() {
        let (mut app, id) = app_with_one_task();
        app.task_ui.bulk_mode = true;
        app.task_ui.bulk_selected.insert(id);
        app.input.popup = Some(Popup::BulkConfirm(BulkAction::MarkDone));
        app.submit_popup();

        assert_eq!(
            app.task_ui.active_task, None,
            "a completed task must not stay active"
        );
    }
}

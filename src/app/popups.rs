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
                        estimated_minutes: self.input.input_number,
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
                self.task_ui.task_state
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
            Some(Popup::EmptyQueueChoice) => {}
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
            Some(Popup::BulkConfirm(action)) => {
                let ids: Vec<u64> = self.task_ui.bulk_selected.iter().copied().collect();
                let result = match action {
                    BulkAction::MarkDone => storage::bulk_mark_done(&self.db, &mut self.data, &ids),
                    BulkAction::Delete => storage::bulk_delete(&self.db, &mut self.data, &ids),
                };
                match result {
                    Ok(n) => {
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
        if matches!(self.input.popup, Some(Popup::AddSubtask(_))) {
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
                    self.stats.calendar_date -= chrono::Duration::days(1);
                    self.input.input_due_date = crate::date::format_date(self.stats.calendar_date);
                    return;
                }
                KeyCode::Right => {
                    self.stats.calendar_date += chrono::Duration::days(1);
                    self.input.input_due_date = crate::date::format_date(self.stats.calendar_date);
                    return;
                }
                KeyCode::Up => {
                    self.stats.calendar_date -= chrono::Duration::days(7);
                    self.input.input_due_date = crate::date::format_date(self.stats.calendar_date);
                    return;
                }
                KeyCode::Down => {
                    self.stats.calendar_date += chrono::Duration::days(7);
                    self.input.input_due_date = crate::date::format_date(self.stats.calendar_date);
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
    }

    pub(crate) fn handle_field_input(&mut self, key: KeyEvent) {
        match self.input.input_field {
            InputField::Estimate => match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    let d = c.to_digit(10).unwrap_or(0);
                    self.input.input_number = (self.input.input_number.saturating_mul(10) + d).min(480);
                }
                KeyCode::Backspace => {
                    self.input.input_number /= 10;
                    if self.input.input_number == 0 {
                        self.input.input_number = 1;
                    }
                }
                KeyCode::Up => self.input.input_number = (self.input.input_number + 5).min(480),
                KeyCode::Down => self.input.input_number = self.input.input_number.saturating_sub(5).max(1),
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

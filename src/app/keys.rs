use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        self.last_activity = Instant::now();
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.force_quit();
            return;
        }
        if self.task_ui.searching {
            self.handle_search_key(key);
            return;
        }
        if self.input.popup.is_some() {
            self.handle_popup_key(key);
            return;
        }
        if key.code == KeyCode::Esc && self.task_ui.bulk_mode && self.ui.tab == FocusTab::Tasks {
            self.toggle_bulk_mode();
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        // In Zen mode digits tick the active task's subtasks instead of switching tabs.
        let zen = self.ui.zen_mode && self.ui.tab == FocusTab::Dashboard;
        match key.code {
            KeyCode::Char('q') if self.task_ui.subtask_focus && self.ui.tab == FocusTab::Tasks => {
                self.task_ui.subtask_focus = false;
                self.set_status("Task list focus", false);
            }
            KeyCode::Char('q') if self.task_ui.bulk_mode && self.ui.tab == FocusTab::Tasks => {
                self.toggle_bulk_mode();
            }
            KeyCode::Char('q') => self.request_quit(),
            KeyCode::Esc if zen => self.ui.zen_mode = false,
            KeyCode::Esc if self.ui.tab == FocusTab::Dashboard => self.request_quit(),
            KeyCode::Esc
                if self.ui.tab == FocusTab::Stats && self.stats.heatmap_cursor.is_some() =>
            {
                self.handle_stats_key(key);
            }
            KeyCode::Esc => self.ui.tab = FocusTab::Dashboard,
            KeyCode::Char('s') if ctrl => self.export_backup(),
            KeyCode::Char('e') if ctrl => self.export_sessions_csv(),
            KeyCode::Char(c @ '1'..='6') if !zen => {
                self.ui.tab = FocusTab::all()[(c as u8 - b'1') as usize];
            }
            KeyCode::Char('?') => self.ui.tab = FocusTab::Help,
            KeyCode::Char('h') if self.ui.tab != FocusTab::About => {
                self.ui.tab = FocusTab::Help;
            }
            KeyCode::Tab if self.ui.tab == FocusTab::Tasks => {
                self.toggle_subtask_focus();
            }
            KeyCode::Tab | KeyCode::BackTab if self.ui.tab == FocusTab::About => {
                self.handle_about_key(key);
            }
            KeyCode::Tab => self.next_tab(),
            KeyCode::BackTab if self.ui.tab == FocusTab::Tasks && self.task_ui.subtask_focus => {
                self.task_ui.subtask_focus = false;
                self.set_status("Task list focus", false);
            }
            KeyCode::BackTab => self.prev_tab(),
            _ => match self.ui.tab {
                FocusTab::Dashboard => self.handle_dashboard_key(key),
                FocusTab::Tasks => self.handle_tasks_key(key),
                FocusTab::Stats => self.handle_stats_key(key),
                FocusTab::Settings => self.handle_settings_key(key),
                FocusTab::Help => self.handle_help_key(key),
                FocusTab::About => self.handle_about_key(key),
            },
        }
    }

    /// Inserts pasted text into the active text field or search, as one line.
    pub fn handle_paste(&mut self, text: &str) {
        self.last_activity = Instant::now();
        let line: String = text
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect();
        if self.task_ui.searching {
            self.task_ui.task_search.push_str(&line);
            self.task_ui.task_search_lower = self.task_ui.task_search.to_lowercase();
            self.recompute_task_caches();
            self.clamp_task_selection_after_mutation();
            return;
        }
        let buf = match (&self.input.popup, self.input.input_field) {
            (Some(Popup::AddSubtask(_)) | Some(Popup::EditSubtask(_, _)), _) => {
                &mut self.input.input_buffer
            }
            (Some(Popup::AddTask) | Some(Popup::EditTask(_)), InputField::Title) => {
                &mut self.input.input_buffer
            }
            (Some(Popup::AddTask) | Some(Popup::EditTask(_)), InputField::Tags) => {
                &mut self.input.input_tags
            }
            (Some(Popup::AddTask) | Some(Popup::EditTask(_)), InputField::DueDate) => {
                self.input.input_due_date.push_str(line.trim());
                self.sync_calendar_to_due_date();
                return;
            }
            _ => return,
        };
        buf.push_str(&line);
    }

    pub(crate) fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('h') | KeyCode::Left => {
                self.ui.about_active_column = 0;
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.ui.about_active_column = 1;
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.ui.about_active_column = 1 - self.ui.about_active_column;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.ui.about_active_column == 0 {
                    self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_add(1);
                } else {
                    self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_add(1);
                }
                self.ui.about_scroll = self.ui.about_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.ui.about_active_column == 0 {
                    self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_sub(1);
                } else {
                    self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_sub(1);
                }
                self.ui.about_scroll = self.ui.about_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                if self.ui.about_active_column == 0 {
                    self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_add(10);
                } else {
                    self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_add(10);
                }
                self.ui.about_scroll = self.ui.about_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                if self.ui.about_active_column == 0 {
                    self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_sub(10);
                } else {
                    self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_sub(10);
                }
                self.ui.about_scroll = self.ui.about_scroll.saturating_sub(10);
            }
            KeyCode::Home => {
                if self.ui.about_active_column == 0 {
                    self.ui.about_left_scroll = 0;
                } else {
                    self.ui.about_right_scroll = 0;
                }
                self.ui.about_scroll = 0;
            }
            _ => {}
        }
        self.clamp_scrolls();
    }

    pub(crate) fn next_tab(&mut self) {
        let cur = FocusTab::all()
            .iter()
            .position(|t| *t == self.ui.tab)
            .unwrap_or(0);
        self.ui.tab = FocusTab::all()[(cur + 1) % FocusTab::all().len()];
    }

    pub(crate) fn prev_tab(&mut self) {
        let cur = FocusTab::all()
            .iter()
            .position(|t| *t == self.ui.tab)
            .unwrap_or(0);
        let n = FocusTab::all().len();
        self.ui.tab = FocusTab::all()[(cur + n - 1) % n];
    }

    pub(crate) fn handle_dashboard_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('s') | KeyCode::Char(' ') => self.toggle_timer(),
            KeyCode::Char('p') => {
                if self.ui.zen_mode {
                    self.cycle_active_task();
                } else {
                    self.pause_timer();
                }
            }
            KeyCode::Char('r') => self.reset_timer(),
            KeyCode::Char('n') => self.skip_session(),
            KeyCode::Char('m') => self.cycle_mode(),
            KeyCode::Char('P') => self.cycle_timer_preset(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.adjust_minutes(1),
            KeyCode::Char('-') | KeyCode::Char('_') => self.adjust_minutes(-1),
            KeyCode::Char('a') => self.open_add_task(),
            KeyCode::Char('f') => {
                if let Some(id) = self.dashboard_selected_task_id() {
                    self.set_active_task(Some(id));
                    self.set_status("Task set as active.", false);
                }
            }

            KeyCode::Char('z') => {
                self.ui.zen_mode = !self.ui.zen_mode;
                self.set_status(
                    format!("Zen mode {}.", if self.ui.zen_mode { "on" } else { "off" }),
                    false,
                );
            }
            KeyCode::Down | KeyCode::Char('j') if ctrl => self.reorder_dashboard_task(1),
            KeyCode::Up | KeyCode::Char('k') if ctrl => self.reorder_dashboard_task(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_dashboard_task_selection(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_dashboard_task_selection(-1),
            KeyCode::Enter => {
                if let Some(id) = self.dashboard_selected_task_id() {
                    self.cycle_task_status_for(id, true);
                    self.clamp_dashboard_task_selection();
                } else {
                    self.cycle_active_task_status();
                }
            }
            KeyCode::Char('x') => {
                if let Some(id) = self.dashboard_selected_task_id() {
                    self.mark_task_done_by_id(id);
                    self.clamp_dashboard_task_selection();
                } else {
                    self.mark_active_task_done();
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E') => self.end_session(),
            KeyCode::Char(c) if self.ui.zen_mode && c.is_ascii_digit() && c != '0' => {
                if let Some(id) = self.task_ui.active_task {
                    let idx = (c as u8 - b'1') as usize;
                    self.persist_data(|db, data| {
                        if let Some(task) = data.task(id) {
                            if let Some(sub) = task.subtasks.get(idx) {
                                let sub_id = sub.id;
                                return storage::toggle_subtask(db, data, id, sub_id);
                            }
                        }
                        Ok(())
                    });
                    self.bump_tasks();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_stats_key(&mut self, key: KeyEvent) {
        // View-level keys work even with no history at all — otherwise a fresh install
        // cannot change the range or switch panels.
        match key.code {
            KeyCode::Char('v') => {
                self.stats.stats_view_mode = self.stats.stats_view_mode.next();
                self.set_status(
                    format!("View: {}", self.stats.stats_view_mode.label()),
                    false,
                );
                return;
            }
            KeyCode::Char('r') => {
                self.stats.stats_range = self.stats.stats_range.next();
                self.set_status(format!("Range: {}", self.stats.stats_range.label()), false);
                return;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                self.end_session();
                return;
            }
            _ => {}
        }

        if self.stats.recent_sessions.is_empty() && self.stats.heatmap_cursor.is_none() {
            return;
        }
        self.clamp_stats_session_selection();
        let n = self.active_stats_sessions().len();

        match key.code {
            KeyCode::Esc => {
                self.stats.heatmap_cursor = None;
                self.stats.stats_session_selected = 0;
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down => {
                let today = crate::date::today_naive();
                let current = self.stats.heatmap_cursor.unwrap_or(today);

                let delta = match key.code {
                    KeyCode::Left => -7,
                    KeyCode::Right => 7,
                    KeyCode::Up => -1,
                    KeyCode::Down => 1,
                    _ => 0,
                };

                let earliest = self
                    .stats
                    .heatmap_data
                    .first()
                    .and_then(|(d, _)| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                    .unwrap_or(today);

                let mut next = current + chrono::Duration::days(delta);
                if next > today {
                    next = today;
                }
                if next < earliest {
                    next = earliest;
                }

                self.focus_heatmap_date(next);
            }
            KeyCode::Char('j') => {
                if n > 0 {
                    self.stats.stats_session_selected = (self.stats.stats_session_selected + 1) % n;
                }
            }
            KeyCode::Char('k') => {
                if n > 0 {
                    self.stats.stats_session_selected = if self.stats.stats_session_selected == 0 {
                        n - 1
                    } else {
                        self.stats.stats_session_selected - 1
                    };
                }
            }
            KeyCode::Char('d') => {
                if let Some(entry) = self.selected_stats_session() {
                    let id = entry.id;
                    self.persist_data(|db, data| storage::delete_session(db, data, id));
                    self.after_stats_session_edit();
                    self.set_status("Session deleted.", false);
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                if let Some(entry) = self.selected_stats_session() {
                    let new_mins = entry.record.minutes.saturating_add(5);
                    let id = entry.id;
                    self.persist_data(|db, data| {
                        storage::adjust_session_minutes(db, data, id, new_mins)
                    });
                    self.after_stats_session_edit();
                }
            }
            KeyCode::Char('-') => {
                if let Some(entry) = self.selected_stats_session() {
                    let new_mins = entry.record.minutes.saturating_sub(5).max(1);
                    let id = entry.id;
                    self.persist_data(|db, data| {
                        storage::adjust_session_minutes(db, data, id, new_mins)
                    });
                    self.after_stats_session_edit();
                }
            }
            KeyCode::Char('[') if self.stats.stats_session_page > 0 => {
                self.stats.stats_session_page -= 1;
                self.stats.stats_session_selected = 0;
                self.refresh_recent_sessions();
            }
            KeyCode::Char(']') => {
                let max_page =
                    self.stats.stats_session_total.saturating_sub(1) / App::SESSIONS_PER_PAGE;
                if self.stats.stats_session_page < max_page {
                    self.stats.stats_session_page += 1;
                    self.stats.stats_session_selected = 0;
                    self.refresh_recent_sessions();
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_search_key(&mut self, key: KeyEvent) {
        let mut changed = false;
        match key.code {
            KeyCode::Esc => {
                self.task_ui.searching = false;
                self.task_ui.task_search.clear();
                changed = true;
            }
            KeyCode::Enter => {
                self.task_ui.searching = false;
            }
            KeyCode::Backspace => {
                if self.task_ui.task_search.pop().is_some() {
                    changed = true;
                }
            }
            KeyCode::Char(c) => {
                self.task_ui.task_search.push(c);
                changed = true;
            }
            _ => {}
        }
        if changed {
            self.task_ui.task_search_lower = self.task_ui.task_search.to_lowercase();
            self.recompute_task_caches();
            self.clamp_task_selection_after_mutation();
        }
    }

    pub(crate) fn handle_tasks_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('f') => {
                if let Some(id) = self.selected_task_id() {
                    self.start_focus_on_task(id);
                }
            }
            KeyCode::Char('g') => {
                self.cycle_task_filter();
            }
            KeyCode::Char('/') => {
                self.task_ui.searching = true;
                if !self.task_ui.task_search.is_empty() {
                    self.task_ui.task_search.clear();
                    self.task_ui.task_search_lower.clear();
                    self.recompute_task_caches();
                }
            }
            KeyCode::Char('t') => {
                if let Some(id) = self.selected_task_id() {
                    self.persist_data(|db, data| storage::toggle_today(db, data, id));
                    self.bump_tasks();
                }
            }
            KeyCode::Char('a') => self.open_add_task(),
            KeyCode::Char('e') if self.task_ui.subtask_focus => {
                self.open_edit_subtask();
            }
            KeyCode::Char('e') => self.open_edit_task(),
            KeyCode::Char('d') => self.open_confirm_delete(),
            KeyCode::Char('v') => {
                if self.task_ui.bulk_mode {
                    self.toggle_bulk_item();
                } else {
                    self.toggle_bulk_mode();
                }
            }
            KeyCode::Char('V') if self.task_ui.bulk_mode => {
                if self.task_ui.bulk_selected.is_empty() {
                    self.set_status("No tasks selected.", true);
                } else {
                    self.input.popup = Some(Popup::BulkConfirm(BulkAction::MarkDone));
                }
            }
            KeyCode::Char('D') if self.task_ui.bulk_mode => {
                if self.task_ui.bulk_selected.is_empty() {
                    self.set_status("No tasks selected.", true);
                } else {
                    self.input.popup = Some(Popup::BulkConfirm(BulkAction::Delete));
                }
            }
            KeyCode::Char('A') => self.archive_selected_task(),
            KeyCode::Char('i') => {
                if let Some(id) = self.selected_task_id() {
                    let next = self
                        .data
                        .task(id)
                        .map(|t| t.recurrence.next())
                        .unwrap_or(crate::model::TaskRecurrence::None);
                    self.persist_data(|db, data| storage::set_task_recurrence(db, data, id, next));
                    self.bump_tasks();
                    self.set_status(format!("Recurrence: {}", next.label()), false);
                }
            }
            KeyCode::Char('c') => self.open_add_subtask(),
            KeyCode::Char('x') | KeyCode::Char('X') if self.task_ui.subtask_focus => {
                self.toggle_subtask_on_selected()
            }
            KeyCode::Char('-') | KeyCode::Char('_') if self.task_ui.subtask_focus => {
                self.delete_subtask_on_selected()
            }

            KeyCode::Enter if self.task_ui.bulk_mode => self.toggle_bulk_item(),
            KeyCode::Enter if self.task_ui.subtask_focus => self.toggle_subtask_on_selected(),
            KeyCode::Enter => {
                if let Some(id) = self.selected_task_id() {
                    self.cycle_task_status_for(id, false);
                }
            }
            KeyCode::Char(' ') if !self.task_ui.subtask_focus => {
                if let Some(id) = self.selected_task_id() {
                    self.set_active_task(Some(id));
                    self.set_status("Task set as active for the timer.", false);
                }
            }
            KeyCode::Char('T') => self.cycle_tag_filter(),
            KeyCode::Char('p') => {
                if let Some(id) = self.selected_task_id() {
                    let next = match self.data.task(id).map(|t| t.priority) {
                        Some(Priority::Low) => Priority::Medium,
                        Some(Priority::Medium) => Priority::High,
                        _ => Priority::Low,
                    };
                    self.persist_data(|db, data| storage::set_priority(db, data, id, next));
                    self.bump_tasks();
                    self.set_status(format!("Priority: {}", next.label()), false);
                }
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl && self.task_ui.subtask_focus => {
                self.move_subtask_selection(1);
            }
            KeyCode::Up | KeyCode::Char('k') if !ctrl && self.task_ui.subtask_focus => {
                self.move_subtask_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') if ctrl && self.task_ui.subtask_focus => {
                self.reorder_subtask(1);
            }
            KeyCode::Up | KeyCode::Char('k') if ctrl && self.task_ui.subtask_focus => {
                self.reorder_subtask(-1);
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => self.move_task_selection(1),
            KeyCode::Up | KeyCode::Char('k') if !ctrl => self.move_task_selection(-1),
            KeyCode::Down | KeyCode::Char('j') if ctrl => self.reorder_selected_task(1),
            KeyCode::Up | KeyCode::Char('k') if ctrl => self.reorder_selected_task(-1),
            KeyCode::PageDown => self.move_task_selection(8),
            KeyCode::PageUp => self.move_task_selection(-8),
            KeyCode::Home => {
                let len = self.filtered_task_indices().len();
                if len > 0 {
                    self.task_ui.task_state.select(Some(0));
                }
            }
            KeyCode::End => {
                let len = self.filtered_task_indices().len();
                if len > 0 {
                    self.task_ui.task_state.select(Some(len - 1));
                }
            }
            _ => {}
        }
    }

    pub(crate) fn move_task_selection(&mut self, delta: i32) {
        let len = self.filtered_task_indices().len();
        if len == 0 {
            return;
        }
        let cur = self.task_ui.task_state.selected().unwrap_or(0) as i32;
        let new = (cur + delta).clamp(0, len as i32 - 1) as usize;
        self.task_ui.task_state.select(Some(new));
        self.task_ui.subtask_focus = false;
        self.reset_subtask_selection();
    }

    pub(crate) fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                self.ui.help_scroll = self.ui.help_scroll.saturating_add(1);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.ui.help_scroll = self.ui.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.ui.help_scroll = self.ui.help_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.ui.help_scroll = self.ui.help_scroll.saturating_sub(10);
            }
            KeyCode::Home => {
                self.ui.help_scroll = 0;
            }
            _ => {}
        }
        self.clamp_scrolls();
    }

    /// Keeps Help and About scroll positions within what was last drawn.
    fn clamp_scrolls(&mut self) {
        let ui = &mut self.ui;
        ui.help_scroll = ui.help_scroll.min(ui.help_scroll_max.get());
        ui.about_left_scroll = ui.about_left_scroll.min(ui.about_left_max.get());
        ui.about_right_scroll = ui.about_right_scroll.min(ui.about_right_max.get());
        ui.about_scroll = ui.about_scroll.min(ui.about_scroll_max.get());
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        self.last_activity = Instant::now();
        // Scrolling would otherwise nudge the open popup's fields through arrow keys.
        if self.input.popup.is_some() {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.ui.tab == FocusTab::Help {
                    self.ui.help_scroll = self.ui.help_scroll.saturating_sub(3);
                } else if self.ui.tab == FocusTab::About {
                    if self.ui.about_active_column == 0 {
                        self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_sub(3);
                    } else {
                        self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_sub(3);
                    }
                    self.ui.about_scroll = self.ui.about_scroll.saturating_sub(3);
                } else if self.ui.tab == FocusTab::Tasks || self.ui.tab == FocusTab::Dashboard {
                    self.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
                }
            }
            MouseEventKind::ScrollDown => {
                if self.ui.tab == FocusTab::Help {
                    self.ui.help_scroll = self.ui.help_scroll.saturating_add(3);
                } else if self.ui.tab == FocusTab::About {
                    if self.ui.about_active_column == 0 {
                        self.ui.about_left_scroll = self.ui.about_left_scroll.saturating_add(3);
                    } else {
                        self.ui.about_right_scroll = self.ui.about_right_scroll.saturating_add(3);
                    }
                    self.ui.about_scroll = self.ui.about_scroll.saturating_add(3);
                } else if self.ui.tab == FocusTab::Tasks || self.ui.tab == FocusTab::Dashboard {
                    self.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
                }
            }
            _ => {}
        }
        self.clamp_scrolls();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::{SettingsItem, SECTION_STARTS};
    use crate::db::Database;
    use crate::model::Priority;

    fn press(app: &mut App, code: KeyCode) {
        app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn app_with(titles: &[&str]) -> App {
        let mut app = App::with_database(Database::open_in_memory().unwrap()).unwrap();
        for title in titles {
            storage::add_task_full(
                &app.db,
                &mut app.data,
                storage::TaskPayload {
                    title: title.to_string(),
                    notes: String::new(),
                    estimated_minutes: 25,
                    priority: Priority::Medium,
                    tags: Vec::new(),
                    due_date: None,
                },
            )
            .unwrap();
        }
        app.recompute_task_caches();
        app.task_ui.task_state.select(Some(0));
        app
    }

    fn select_setting(app: &mut App, item: SettingsItem) {
        app.ui.tab = FocusTab::Settings;
        app.settings_state.selected = app
            .settings_state
            .items
            .iter()
            .position(|&i| i == item)
            .unwrap();
    }

    #[test]
    fn every_settings_row_is_labelled_for_its_own_item() {
        let app = app_with(&[]);
        let labels = app.build_settings_labels();
        assert_eq!(labels.len(), app.settings_state.items.len());
        let key_of = |item| {
            let i = app
                .settings_state
                .items
                .iter()
                .position(|&x| x == item)
                .unwrap();
            labels[i].key
        };
        assert_eq!(key_of(SettingsItem::RestDays), "Rest days");
        assert_eq!(key_of(SettingsItem::TerminalTitle), "Terminal title");
        assert_eq!(key_of(SettingsItem::ArchiveAfterDays), "Archive after");
    }

    #[test]
    fn enter_on_terminal_title_toggles_terminal_title() {
        let mut app = app_with(&[]);
        let before = app.data.show_terminal_title;
        let rest_before = app.data.streak_rest_days.clone();
        select_setting(&mut app, SettingsItem::TerminalTitle);
        press(&mut app, KeyCode::Enter);
        assert_ne!(app.data.show_terminal_title, before);
        assert_eq!(app.data.streak_rest_days, rest_before);
    }

    #[test]
    fn settings_visual_row_counts_every_section_header() {
        let app = app_with(&[]);
        let last = app.settings_state.items.len() - 1;
        assert_eq!(app.settings_visual_row(0), 1);
        assert_eq!(app.settings_visual_row(last), last + SECTION_STARTS.len());
    }

    #[test]
    fn digits_switch_tabs_from_tasks_and_dashboard() {
        let mut app = app_with(&["One"]);
        app.ui.tab = FocusTab::Tasks;
        press(&mut app, KeyCode::Char('3'));
        assert_eq!(app.ui.tab, FocusTab::Stats);
        app.ui.tab = FocusTab::Dashboard;
        press(&mut app, KeyCode::Char('2'));
        assert_eq!(app.ui.tab, FocusTab::Tasks);
    }

    #[test]
    fn p_cycles_the_selected_task_priority() {
        let mut app = app_with(&["One"]);
        app.ui.tab = FocusTab::Tasks;
        let id = app.selected_task_id().unwrap();
        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.data.task(id).unwrap().priority, Priority::High);
        press(&mut app, KeyCode::Char('p'));
        assert_eq!(app.data.task(id).unwrap().priority, Priority::Low);
        assert_eq!(app.ui.tab, FocusTab::Tasks);
    }

    #[test]
    fn digits_in_zen_tick_the_active_task_subtasks() {
        let mut app = app_with(&["One"]);
        let id = app.selected_task_id().unwrap();
        storage::add_subtask(&app.db, &mut app.data, id, "Sub".into()).unwrap();
        app.set_active_task(Some(id));
        app.ui.tab = FocusTab::Dashboard;
        app.ui.zen_mode = true;
        press(&mut app, KeyCode::Char('1'));
        assert_eq!(app.ui.tab, FocusTab::Dashboard);
        assert!(app.data.task(id).unwrap().subtasks[0].done);
    }

    #[test]
    fn esc_clears_the_heatmap_cursor_before_leaving_stats() {
        let mut app = app_with(&[]);
        app.ui.tab = FocusTab::Stats;
        app.stats.heatmap_cursor = Some(crate::date::today_naive());
        press(&mut app, KeyCode::Esc);
        assert!(app.stats.heatmap_cursor.is_none());
        assert_eq!(app.ui.tab, FocusTab::Stats);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.ui.tab, FocusTab::Dashboard);
        assert!(!app.ui.should_quit);
    }

    #[test]
    fn esc_returns_to_dashboard_and_only_quits_from_there() {
        let mut app = app_with(&[]);
        for tab in [
            FocusTab::Tasks,
            FocusTab::Settings,
            FocusTab::Help,
            FocusTab::About,
        ] {
            app.ui.tab = tab;
            press(&mut app, KeyCode::Esc);
            assert_eq!(app.ui.tab, FocusTab::Dashboard, "from {tab:?}");
            assert!(!app.ui.should_quit);
        }
        press(&mut app, KeyCode::Esc);
        assert!(app.ui.should_quit);
    }

    #[test]
    fn esc_in_zen_exits_zen_instead_of_quitting() {
        let mut app = app_with(&[]);
        app.ui.tab = FocusTab::Dashboard;
        app.ui.zen_mode = true;
        press(&mut app, KeyCode::Esc);
        assert!(!app.ui.zen_mode);
        assert!(!app.ui.should_quit);
    }

    #[test]
    fn search_keeps_a_valid_selection() {
        let titles: Vec<String> = (0..10).map(|i| format!("Task {i}")).collect();
        let mut refs: Vec<&str> = titles.iter().map(String::as_str).collect();
        refs.extend(["Alpha one", "Alpha two"]);
        let mut app = app_with(&refs);
        app.ui.tab = FocusTab::Tasks;
        app.task_ui.task_state.select(Some(6));
        app.task_ui.searching = true;
        for c in "alpha".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert!(app.selected_task_id().is_some());
    }

    #[test]
    fn a_multi_line_paste_goes_into_the_field_as_one_line() {
        let mut app = app_with(&[]);
        app.ui.tab = FocusTab::Tasks;
        app.open_add_task();
        app.handle_paste(
            "first line
d
y",
        );
        assert_eq!(app.input.input_buffer, "first line d y");
        assert!(matches!(app.input.popup, Some(Popup::AddTask)));
    }

    #[test]
    fn a_paste_outside_a_text_field_is_ignored() {
        let mut app = app_with(&["Keep me"]);
        app.ui.tab = FocusTab::Tasks;
        app.handle_paste(
            "d
y",
        );
        assert!(app.input.popup.is_none());
        assert_eq!(app.data.tasks.len(), 1);
    }

    #[test]
    fn the_tag_filter_applies_as_soon_as_it_is_chosen() {
        let mut app = app_with(&["Tagged", "Plain"]);
        let tagged = app
            .data
            .tasks
            .values()
            .find(|t| t.title == "Tagged")
            .unwrap()
            .id;
        app.data.task_mut(tagged).unwrap().tags = vec!["work".into()];
        app.recompute_task_caches();
        app.ui.tab = FocusTab::Tasks;
        press(&mut app, KeyCode::Char('T'));
        assert_eq!(app.filtered_task_indices().len(), 1);
        assert_eq!(app.selected_task_id(), Some(tagged));
    }

    #[test]
    fn bulk_actions_skip_selected_tasks_hidden_by_the_filter() {
        let mut app = app_with(&["Visible", "Hidden"]);
        let ids: Vec<u64> = app.data.tasks.keys().copied().collect();
        storage::mark_task_done(&app.db, &mut app.data, ids[1]).unwrap();
        app.task_ui.task_filter = crate::app::TaskFilter::Pending;
        app.recompute_task_caches();
        app.task_ui.bulk_selected.extend(ids.iter().copied());
        app.input.popup = Some(Popup::BulkConfirm(crate::app::BulkAction::Delete));
        app.submit_popup();
        assert!(app.data.task(ids[0]).is_none());
        assert!(app.data.task(ids[1]).is_some(), "hidden task was deleted");
    }

    #[test]
    fn error_messages_clear_after_ten_seconds() {
        let mut app = app_with(&[]);
        app.set_status("Save error: boom", true);
        app.ui.last_status_set = Instant::now() - std::time::Duration::from_secs(5);
        app.on_tick();
        assert!(app.ui.status.is_some());
        app.ui.last_status_set = Instant::now() - std::time::Duration::from_secs(11);
        app.on_tick();
        assert!(app.ui.status.is_none());
    }

    #[test]
    fn ctrl_c_quits_even_from_a_popup_or_search() {
        for searching in [false, true] {
            let mut app = app_with(&[]);
            app.task_ui.searching = searching;
            if !searching {
                app.open_add_task();
            }
            app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
            assert!(app.ui.should_quit);
        }
    }

    #[test]
    fn mouse_scroll_is_ignored_while_a_popup_is_open() {
        let mut app = app_with(&[]);
        app.open_add_task();
        app.input.input_field = crate::app::InputField::Estimate;
        let before = app.input.input_number;
        app.handle_mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(app.input.input_number, before);
    }

    #[test]
    fn help_scrolling_stops_at_the_end() {
        let mut app = app_with(&[]);
        app.ui.tab = FocusTab::Help;
        app.ui.help_scroll_max.set(5);
        for _ in 0..20 {
            press(&mut app, KeyCode::Char('j'));
        }
        assert_eq!(app.ui.help_scroll, 5);
        press(&mut app, KeyCode::Char('k'));
        assert_eq!(app.ui.help_scroll, 4);
    }

    #[test]
    fn question_mark_opens_help() {
        let mut app = app_with(&[]);
        press(&mut app, KeyCode::Char('?'));
        assert_eq!(app.ui.tab, FocusTab::Help);
    }

    #[test]
    fn about_keeps_h_and_tab_for_its_columns() {
        let mut app = app_with(&[]);
        app.ui.tab = FocusTab::About;
        app.ui.about_active_column = 1;
        press(&mut app, KeyCode::Char('h'));
        assert_eq!(app.ui.tab, FocusTab::About);
        assert_eq!(app.ui.about_active_column, 0);
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.ui.tab, FocusTab::About);
        assert_eq!(app.ui.about_active_column, 1);
    }
}

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        self.last_activity = Instant::now();
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
        match key.code {
            KeyCode::Char('q') if self.task_ui.subtask_focus && self.ui.tab == FocusTab::Tasks => {
                self.task_ui.subtask_focus = false;
                self.set_status("Task list focus", false);
            }
            KeyCode::Char('q') if self.task_ui.bulk_mode && self.ui.tab == FocusTab::Tasks => {
                self.toggle_bulk_mode();
            }
            KeyCode::Char('q') => self.ui.should_quit = true,
            KeyCode::Esc => self.ui.should_quit = true,
            KeyCode::Char('c') if ctrl => self.ui.should_quit = true,
            KeyCode::Char('s') if ctrl => self.export_backup(),
            KeyCode::Char('1') => self.ui.tab = FocusTab::Dashboard,
            KeyCode::Char('2') => self.ui.tab = FocusTab::Tasks,
            KeyCode::Char('3') => self.ui.tab = FocusTab::Stats,
            KeyCode::Char('4') => self.ui.tab = FocusTab::Settings,
            KeyCode::Char('5') | KeyCode::Char('h') => self.ui.tab = FocusTab::Help,
            KeyCode::Char('6') => self.ui.tab = FocusTab::About,
            KeyCode::Tab if self.ui.tab == FocusTab::Tasks && self.selected_subtask_count() > 0 => {
                self.toggle_subtask_focus();
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

    pub(crate) fn handle_about_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.ui.about_scroll = self.ui.about_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.ui.about_scroll = self.ui.about_scroll.saturating_sub(1);
            }
            _ => {}
        }
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
            KeyCode::Char('n') => {
                self.timer.skip();
                self.on_timer_finished(true);
            }
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
            KeyCode::Down | KeyCode::Char('j') if ctrl => {
                if let Some(id) = self.dashboard_selected_task_id() {
                    self.task_ui.reordering_task = Some(id);
                    self.persist_data(|db, data| storage::move_task(db, data, id, 1));
                    self.bump_tasks();
                    self.task_ui.reordering_task = None;
                    self.move_dashboard_task_selection(1);
                }
            }
            KeyCode::Up | KeyCode::Char('k') if ctrl => {
                if let Some(id) = self.dashboard_selected_task_id() {
                    self.task_ui.reordering_task = Some(id);
                    self.persist_data(|db, data| storage::move_task(db, data, id, -1));
                    self.bump_tasks();
                    self.task_ui.reordering_task = None;
                    self.move_dashboard_task_selection(-1);
                }
            }
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
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let target = if self.ui.zen_mode {
                    self.task_ui.active_task
                } else {
                    self.dashboard_selected_task_id()
                };
                if let Some(id) = target {
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
        if self.stats.recent_sessions.is_empty() && self.stats.heatmap_cursor.is_none() {
            if matches!(key.code, KeyCode::Char('e') | KeyCode::Char('E')) {
                self.end_session();
            }
            return;
        }
        self.clamp_stats_session_selection();
        let n = self.active_stats_sessions().len();

        match key.code {
            KeyCode::Char('v') => {
                self.stats.stats_view_mode = match self.stats.stats_view_mode {
                    StatsViewMode::Overview => StatsViewMode::Analytics,
                    StatsViewMode::Analytics => StatsViewMode::Overview,
                };
            }
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
            KeyCode::Char('e') | KeyCode::Char('E') => self.end_session(),
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
            KeyCode::Char('x') | KeyCode::Char('X') => self.toggle_subtask_on_selected(),
            KeyCode::Char('-') | KeyCode::Char('_') => self.delete_subtask_on_selected(),
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
            KeyCode::Char('1') => {
                if let Some(id) = self.selected_task_id() {
                    self.persist_data(|db, data| {
                        storage::set_priority(db, data, id, Priority::Low)
                    });
                    self.bump_tasks();
                }
            }
            KeyCode::Char('2') => {
                if let Some(id) = self.selected_task_id() {
                    self.persist_data(|db, data| {
                        storage::set_priority(db, data, id, Priority::Medium)
                    });
                    self.bump_tasks();
                }
            }
            KeyCode::Char('3') => {
                if let Some(id) = self.selected_task_id() {
                    self.persist_data(|db, data| {
                        storage::set_priority(db, data, id, Priority::High)
                    });
                    self.bump_tasks();
                }
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl && self.task_ui.subtask_focus => {
                self.move_subtask_selection(1);
            }
            KeyCode::Up | KeyCode::Char('k') if !ctrl && self.task_ui.subtask_focus => {
                self.move_subtask_selection(-1);
            }
            KeyCode::Down | KeyCode::Char('j') if !ctrl => self.move_task_selection(1),
            KeyCode::Up | KeyCode::Char('k') if !ctrl => self.move_task_selection(-1),
            KeyCode::Down | KeyCode::Char('j') if ctrl => {
                if let Some(id) = self.selected_task_id() {
                    self.task_ui.reordering_task = Some(id);
                    self.persist_data(|db, data| storage::move_task(db, data, id, 1));
                    self.bump_tasks();
                    self.task_ui.reordering_task = None;
                }
            }
            KeyCode::Up | KeyCode::Char('k') if ctrl => {
                if let Some(id) = self.selected_task_id() {
                    self.task_ui.reordering_task = Some(id);
                    self.persist_data(|db, data| storage::move_task(db, data, id, -1));
                    self.bump_tasks();
                    self.task_ui.reordering_task = None;
                }
            }
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
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        self.last_activity = Instant::now();
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if self.ui.tab == FocusTab::Help {
                    self.ui.help_scroll = self.ui.help_scroll.saturating_sub(3);
                } else if self.ui.tab == FocusTab::About {
                    self.ui.about_scroll = self.ui.about_scroll.saturating_sub(3);
                } else if self.ui.tab == FocusTab::Tasks || self.ui.tab == FocusTab::Dashboard {
                    self.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
                }
            }
            MouseEventKind::ScrollDown => {
                if self.ui.tab == FocusTab::Help {
                    self.ui.help_scroll = self.ui.help_scroll.saturating_add(3);
                } else if self.ui.tab == FocusTab::About {
                    self.ui.about_scroll = self.ui.about_scroll.saturating_add(3);
                } else if self.ui.tab == FocusTab::Tasks || self.ui.tab == FocusTab::Dashboard {
                    self.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
                }
            }
            _ => {}
        }
    }
}

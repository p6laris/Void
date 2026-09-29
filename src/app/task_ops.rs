use super::*;
use crate::storage;

impl App {
    pub fn pending_task_ids(&self) -> Vec<u64> {
        storage::sorted_pending_tasks(&self.data)
            .into_iter()
            .map(|t| t.id)
            .collect()
    }

    pub fn dashboard_selected_task_id(&self) -> Option<u64> {
        let indices = self.dashboard_task_indices();
        if indices.is_empty() {
            None
        } else {
            let idx = self
                .task_ui
                .dashboard_task_state
                .selected()
                .unwrap_or(0)
                .min(indices.len() - 1);
            Some(self.data.tasks[indices[idx]].id)
        }
    }

    pub(crate) fn clamp_dashboard_task_selection(&mut self) {
        let n = self.dashboard_task_indices().len();
        if n == 0 {
            self.task_ui.dashboard_task_state.select(None);
        } else {
            let sel = self
                .task_ui
                .dashboard_task_state
                .selected()
                .unwrap_or(0)
                .min(n - 1);
            self.task_ui.dashboard_task_state.select(Some(sel));
        }
    }

    pub(crate) fn clamp_task_selection_after_mutation(&mut self) {
        let len = self.filtered_task_indices().len();
        if len == 0 {
            self.task_ui.task_state.select(None);
        } else {
            let sel = self.task_ui.task_state.selected().unwrap_or(0).min(len - 1);
            self.task_ui.task_state.select(Some(sel));
        }
    }

    pub(crate) fn move_dashboard_task_selection(&mut self, delta: i32) {
        let count = self.dashboard_task_indices().len();
        if count == 0 {
            return;
        }
        let cur = self.task_ui.dashboard_task_state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(count as i32) as usize;
        self.task_ui.dashboard_task_state.select(Some(next));
    }

    fn ids_at(&self, indices: &[usize]) -> Vec<u64> {
        indices
            .iter()
            .filter_map(|&i| self.data.tasks.get_index(i).map(|(id, _)| *id))
            .collect()
    }

    /// Swaps the selected task with its visible neighbour on the Tasks tab.
    pub(crate) fn reorder_selected_task(&mut self, delta: i32) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let visible = self.ids_at(&self.task_ui.cached_filtered_tasks);
        let Some(neighbor) = neighbor_in(&visible, id, delta) else {
            return;
        };
        self.persist_data(|db, data| storage::move_task_to(db, data, id, neighbor));
        self.bump_tasks();
        let visible = self.ids_at(&self.task_ui.cached_filtered_tasks);
        if let Some(pos) = visible.iter().position(|&x| x == id) {
            self.task_ui.task_state.select(Some(pos));
        }
    }

    /// Swaps the selected dashboard task with its neighbour, within the same priority group.
    pub(crate) fn reorder_dashboard_task(&mut self, delta: i32) {
        let Some(id) = self.dashboard_selected_task_id() else {
            return;
        };
        let visible = self.ids_at(&self.task_ui.cached_dashboard_tasks);
        let Some(neighbor) = neighbor_in(&visible, id, delta) else {
            return;
        };
        let group = |id: u64| self.data.task(id).map(|t| (t.priority, t.today));
        if group(id) != group(neighbor) {
            self.set_status(
                "This list is ordered by priority first; change it with p on Tasks.",
                false,
            );
            return;
        }
        self.persist_data(|db, data| storage::move_task_to(db, data, id, neighbor));
        self.bump_tasks();
        let visible = self.ids_at(&self.task_ui.cached_dashboard_tasks);
        if let Some(pos) = visible.iter().position(|&x| x == id) {
            self.task_ui.dashboard_task_state.select(Some(pos));
        }
    }

    pub fn pending_task_count(&self) -> u32 {
        storage::pending_tasks(&self.data).count() as u32
    }

    /// Position of the active task in the "up next" list, from the cached dashboard order.
    pub fn active_task_pending_index(&self) -> Option<u32> {
        let id = self.task_ui.active_task?;
        self.task_ui
            .cached_dashboard_tasks
            .iter()
            .position(|&i| {
                self.data
                    .tasks
                    .get_index(i)
                    .is_some_and(|(&tid, _)| tid == id)
            })
            .map(|i| i as u32)
    }

    pub fn active_task_progress(&self) -> Option<f64> {
        let id = self.task_ui.active_task?;
        let task = self.data.task(id)?;
        Some(task.progress_ratio())
    }

    pub(crate) fn matches_filter(&self, t: &crate::model::Task) -> bool {
        if !self.task_ui.task_search_lower.is_empty() {
            let q = &self.task_ui.task_search_lower;
            let title_match = t.title.to_lowercase().contains(q);
            let tags_match = t.tags.iter().any(|tag| tag.to_lowercase().contains(q));
            if !title_match && !tags_match {
                return false;
            }
        }
        if let Some(ref tag) = self.task_ui.active_tag_filter {
            if !t.tags.iter().any(|t| t == tag) {
                return false;
            }
        }
        match self.task_ui.task_filter {
            TaskFilter::All => true,
            TaskFilter::Pending => t.is_open(),
            TaskFilter::Done => t.status == crate::model::TaskStatus::Done && !t.archived,
            TaskFilter::Today => t.today && t.is_open(),
            TaskFilter::Archived => t.archived,
        }
    }

    pub fn recompute_task_caches(&mut self) {
        self.task_ui.cached_filtered_tasks = self
            .data
            .tasks
            .values()
            .enumerate()
            .filter(|(_, t)| self.matches_filter(t))
            .map(|(i, _)| i)
            .collect();

        let mut dash: Vec<usize> = self
            .data
            .tasks
            .values()
            .enumerate()
            .filter(|(_, t)| t.is_open())
            .map(|(i, _)| i)
            .collect();
        dash.sort_by(|&a, &b| {
            let ta = &self.data.tasks[a];
            let tb = &self.data.tasks[b];
            tb.priority
                .rank()
                .cmp(&ta.priority.rank())
                .then(tb.today.cmp(&ta.today))
                .then(ta.sort_order.cmp(&tb.sort_order))
        });
        self.task_ui.cached_dashboard_tasks = dash;

        let mut tags: Vec<String> = self
            .data
            .tasks
            .values()
            .flat_map(|t| t.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        self.task_ui.cached_task_tags = tags;

        let open_blockers: std::collections::HashSet<u64> = self
            .data
            .tasks
            .values()
            .filter(|t| t.is_open())
            .map(|t| t.id)
            .collect();
        self.task_ui.cached_task_blocked = self
            .data
            .tasks
            .values()
            .map(|t| t.blocked_by.iter().any(|id| open_blockers.contains(id)))
            .collect();
    }

    pub fn is_task_blocked_at(&self, task_idx: usize) -> bool {
        self.task_ui
            .cached_task_blocked
            .get(task_idx)
            .copied()
            .unwrap_or(false)
    }

    pub fn is_task_blocked(&self, task_id: u64) -> bool {
        self.data
            .tasks
            .get_index_of(&task_id)
            .map(|idx| self.is_task_blocked_at(idx))
            .unwrap_or(false)
    }

    pub fn task_tags(&self) -> &[String] {
        &self.task_ui.cached_task_tags
    }

    pub fn filtered_task_indices(&self) -> &[usize] {
        &self.task_ui.cached_filtered_tasks
    }

    pub fn dashboard_task_indices(&self) -> &[usize] {
        &self.task_ui.cached_dashboard_tasks
    }

    pub fn dashboard_task(&self, index: usize) -> Option<&crate::model::Task> {
        self.task_ui
            .cached_dashboard_tasks
            .get(index)
            .map(|&i| &self.data.tasks[i])
    }

    pub fn set_active_task(&mut self, id: Option<u64>) {
        if let Some(id) = id {
            if self
                .data
                .tasks
                .get(&id)
                .is_some_and(|t| t.status == crate::model::TaskStatus::Done)
            {
                self.set_status("That task is done — pick another.", true);
                return;
            }
            if self.is_task_blocked(id) {
                self.set_status("Task is blocked — complete dependencies first.", true);
                return;
            }
            self.persist_data(|db, data| storage::promote_task_on_activate(db, data, id));
        }
        self.task_ui.active_task = id;
        self.data.active_task_id = id;
        self.persist(|db| db.persist_active_task(id));
    }

    pub fn cycle_active_task_status(&mut self) {
        let Some(id) = self.task_ui.active_task else {
            self.set_status("No active task — set one on Tasks (Space).", true);
            return;
        };
        self.cycle_task_status_for(id, false);
    }

    pub fn cycle_task_status_for(&mut self, id: u64, set_active: bool) {
        let done_recurring = self.data.task(id).is_some_and(|t| {
            t.status == crate::model::TaskStatus::Done
                && t.recurrence != crate::model::TaskRecurrence::None
        });
        if done_recurring {
            self.set_status("Recurring task — reopen its next occurrence instead.", true);
            return;
        }
        if set_active {
            self.set_active_task(Some(id));
        }
        self.persist_data(|db, data| storage::cycle_task_status(db, data, id));
        let status = self.data.task(id).map(|t| t.status);
        let Some(status) = status else {
            return;
        };
        if status == crate::model::TaskStatus::Done {
            if self.task_ui.active_task == Some(id) {
                self.task_ui.active_task = None;
                self.data.active_task_id = None;
                self.persist(|db| db.persist_active_task(None));
            }
            self.maybe_advance_task();
        }
        self.bump_tasks();
        self.set_status(format!("Task status: {}", status.label()), false);
        if status == crate::model::TaskStatus::Done {
            self.check_queue_empty();
        }
    }

    pub(crate) fn mark_task_done_by_id(&mut self, id: u64) {
        self.persist_data(|db, data| storage::mark_task_done(db, data, id));
        if self.task_ui.active_task == Some(id) {
            self.task_ui.active_task = None;
            self.data.active_task_id = None;
            self.persist(|db| db.persist_active_task(None));
            self.maybe_advance_task();
        }
        self.bump_tasks();
        if self.data.sound_enabled {
            sound::play_task_complete();
        }
        self.set_status("Task marked done.", false);
        self.check_queue_empty();
    }

    pub fn mark_active_task_done(&mut self) {
        let Some(id) = self.task_ui.active_task else {
            self.set_status("No active task — set one on Tasks (Space).", true);
            return;
        };
        self.mark_task_done_by_id(id);
    }

    pub(crate) fn auto_pick_task_if_needed(&mut self) {
        if self.task_ui.active_task.is_some() || !self.data.auto_pick_task {
            return;
        }
        if let Some(id) = storage::pick_best_task(&self.data) {
            self.set_active_task(Some(id));
        }
    }

    pub(crate) fn maybe_advance_task(&mut self) {
        if !self.data.auto_advance_task {
            return;
        }
        // Stay on a task until it's done or its estimate is used up.
        let keep_current = self
            .task_ui
            .active_task
            .and_then(|id| self.data.task(id))
            .is_some_and(|t| t.is_open() && t.actual_minutes < t.estimated_minutes);
        if keep_current {
            return;
        }
        let next = storage::advance_to_next_task(&self.data, self.task_ui.active_task);
        self.set_active_task(next);
        if let Some(id) = next {
            if let Some(t) = self.data.task(id) {
                self.set_status(format!("Next task: {}", t.title), false);
            }
        }
    }

    pub(crate) fn cycle_active_task(&mut self) {
        let pending: Vec<u64> = storage::pending_tasks(&self.data).map(|t| t.id).collect();
        if pending.is_empty() {
            self.set_status("No tasks available to switch to.", true);
            return;
        }
        let next_id = if let Some(current) = self.task_ui.active_task {
            if let Some(pos) = pending.iter().position(|&id| id == current) {
                pending[(pos + 1) % pending.len()]
            } else {
                pending[0]
            }
        } else {
            pending[0]
        };
        self.set_active_task(Some(next_id));
    }

    pub fn start_focus_on_task(&mut self, id: u64) {
        if self
            .data
            .task(id)
            .is_some_and(|t| t.status == crate::model::TaskStatus::Done)
        {
            self.set_status("That task is done — pick another.", true);
            return;
        }
        self.set_active_task(Some(id));
        self.ui.tab = FocusTab::Dashboard;
        if self.timer.mode != TimerMode::Focus {
            self.timer.configure(TimerMode::Focus);
        }
        self.start_timer();
    }

    pub fn cycle_task_filter(&mut self) {
        self.task_ui.task_filter = self.task_ui.task_filter.next();
        self.recompute_task_caches();
        self.task_ui.task_state.select(Some(0));
        self.set_status(
            format!("Filter: {}", self.task_ui.task_filter.label()),
            false,
        );
    }

    pub fn toggle_bulk_mode(&mut self) {
        self.task_ui.bulk_mode = !self.task_ui.bulk_mode;
        self.task_ui.bulk_selected.clear();
        if self.task_ui.bulk_mode {
            if let Some(id) = self.selected_task_id() {
                self.task_ui.bulk_selected.insert(id);
            }
        }
        self.set_status(
            format!(
                "Bulk select {} — [v]/Enter toggle row, [V] done, [D] delete, [q] exit",
                if self.task_ui.bulk_mode { "on" } else { "off" }
            ),
            false,
        );
    }

    pub fn toggle_bulk_item(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        if self.task_ui.bulk_selected.contains(&id) {
            self.task_ui.bulk_selected.remove(&id);
        } else {
            self.task_ui.bulk_selected.insert(id);
        }
        self.set_status(
            format!("{} task(s) selected", self.task_ui.bulk_selected.len()),
            false,
        );
    }

    pub fn clamp_subtask_selection(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.task_ui.subtask_selected = 0;
            return;
        };
        let n = self.data.task(id).map(|t| t.subtasks.len()).unwrap_or(0);
        if n == 0 {
            self.task_ui.subtask_selected = 0;
            self.task_ui.subtask_focus = false;
        } else if self.task_ui.subtask_selected >= n {
            self.task_ui.subtask_selected = n - 1;
        }
    }

    pub fn sync_subtask_list(&mut self) {
        self.clamp_subtask_selection();
    }

    pub fn selected_subtask_count(&self) -> usize {
        self.selected_task_id()
            .and_then(|id| self.data.task(id).map(|t| t.subtasks.len()))
            .unwrap_or(0)
    }

    pub fn toggle_subtask_focus(&mut self) {
        if self.selected_subtask_count() == 0 {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        }
        self.task_ui.subtask_focus = !self.task_ui.subtask_focus;
        if self.task_ui.subtask_focus {
            self.reset_subtask_selection();
            self.sync_subtask_list();
            self.set_status(
                "Subtask focus — j/k or ↑/↓ navigate · x/Enter toggle · - remove · q back",
                false,
            );
        } else {
            self.set_status("Task list focus", false);
        }
    }

    pub fn move_subtask_selection(&mut self, delta: i32) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let n = self.data.task(id).map(|t| t.subtasks.len()).unwrap_or(0);
        if n == 0 {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        }
        let cur = self.task_ui.subtask_selected as i32;
        self.task_ui.subtask_selected = (cur + delta).rem_euclid(n as i32) as usize;
        if let Some(s) = self
            .data
            .task(id)
            .and_then(|t| t.subtasks.get(self.task_ui.subtask_selected))
        {
            self.set_status(format!("Subtask: {}", s.title), false);
        }
    }

    pub fn reset_subtask_selection(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.task_ui.subtask_selected = 0;
            return;
        };
        let Some(t) = self.data.task(id) else {
            self.task_ui.subtask_selected = 0;
            return;
        };
        if t.subtasks.is_empty() {
            self.task_ui.subtask_selected = 0;
        } else {
            self.task_ui.subtask_selected = t.subtasks.iter().position(|s| !s.done).unwrap_or(0);
        }
    }

    pub fn delete_subtask_on_selected(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let sub_id = self
            .data
            .task(id)
            .and_then(|t| t.subtasks.get(self.task_ui.subtask_selected))
            .map(|s| s.id);
        let Some(sub_id) = sub_id else {
            self.set_status("No subtasks to remove.", true);
            return;
        };
        let title = self
            .data
            .task(id)
            .and_then(|t| t.subtask(sub_id))
            .map(|s| s.title.clone())
            .unwrap_or_default();
        self.persist_data(|db, data| storage::delete_subtask(db, data, id, sub_id));
        self.bump_tasks();
        self.reset_subtask_selection();
        self.set_status(format!("Removed subtask \"{title}\""), false);
    }

    pub fn open_add_subtask(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.set_status("No task selected.", true);
            return;
        };
        self.input.input_buffer.clear();
        self.input.popup = Some(Popup::AddSubtask(id));
        self.input.input_mode = InputMode::Editing;
    }

    pub fn toggle_subtask_on_selected(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let sub_id = self
            .data
            .task(id)
            .and_then(|t| t.subtasks.get(self.task_ui.subtask_selected))
            .map(|s| s.id);
        let Some(sub_id) = sub_id else {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        };
        self.persist_data(|db, data| storage::toggle_subtask(db, data, id, sub_id));
        self.bump_tasks();
        self.clamp_subtask_selection();
        if let Some(t) = self.data.task(id) {
            if let Some(s) = t.subtask(sub_id) {
                let state = if s.done { "done" } else { "open" };
                self.set_status(format!("Subtask \"{}\" marked {state}", s.title), false);
            }
        }
    }

    pub fn archive_selected_task(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.set_status("No task selected.", true);
            return;
        };
        let archived = self.data.task(id).is_some_and(|t| t.archived);
        if archived {
            self.persist_data(|db, data| storage::unarchive_task(db, data, id));
        } else {
            self.persist_data(|db, data| storage::archive_task(db, data, id));
            if self.task_ui.active_task == Some(id) {
                self.set_active_task(None);
            }
        }
        self.bump_tasks();
        self.clamp_task_selection_after_mutation();
        self.set_status(
            if archived {
                "Task restored."
            } else {
                "Task archived."
            },
            false,
        );
    }

    pub fn reorder_subtask(&mut self, dir: i32) {
        let Some(task_id) = self.selected_task_id() else {
            return;
        };
        let Some(t) = self.data.task(task_id) else {
            return;
        };
        let n = t.subtasks.len();
        if n < 2 {
            return;
        }
        let idx = self.task_ui.subtask_selected;
        let new_idx = (idx as i32 + dir).clamp(0, n as i32 - 1) as usize;
        if idx == new_idx {
            return;
        }
        self.persist_data(|db, data| storage::move_subtask(db, data, task_id, idx, new_idx));
        self.task_ui.subtask_selected = new_idx;
        self.bump_tasks();
    }

    pub fn open_edit_subtask(&mut self) {
        let Some(task_id) = self.selected_task_id() else {
            return;
        };
        let Some(t) = self.data.task(task_id) else {
            return;
        };
        let Some(sub) = t.subtasks.get(self.task_ui.subtask_selected) else {
            self.set_status("No subtask selected.", true);
            return;
        };
        self.input.input_buffer = sub.title.clone();
        self.input.popup = Some(Popup::EditSubtask(task_id, sub.id));
        self.input.input_mode = InputMode::Editing;
    }
}

/// The id `delta` places away from `id` in `ids`, if there is one.
fn neighbor_in(ids: &[u64], id: u64, delta: i32) -> Option<u64> {
    let pos = ids.iter().position(|&x| x == id)? as i32 + delta;
    usize::try_from(pos).ok().and_then(|p| ids.get(p)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn app() -> App {
        App::with_database(Database::open_in_memory().unwrap()).unwrap()
    }

    fn add(app: &mut App, title: &str, priority: crate::model::Priority) -> u64 {
        storage::add_task_full(
            &app.db,
            &mut app.data,
            storage::TaskPayload {
                title: title.into(),
                notes: String::new(),
                estimated_minutes: 25,
                priority,
                tags: Vec::new(),
                due_date: None,
            },
        )
        .unwrap()
    }

    fn select(app: &mut App, id: u64) {
        app.recompute_task_caches();
        let pos = app
            .ids_at(&app.task_ui.cached_filtered_tasks)
            .iter()
            .position(|&x| x == id)
            .unwrap();
        app.task_ui.task_state.select(Some(pos));
    }

    #[test]
    fn the_active_task_index_matches_the_up_next_order() {
        use crate::model::Priority;
        let mut app = app();
        add(&mut app, "Low", Priority::Low);
        let high = add(&mut app, "High", Priority::High);
        let mid = add(&mut app, "Mid", Priority::Medium);
        app.recompute_task_caches();
        for id in [high, mid] {
            app.set_active_task(Some(id));
            let expected = storage::sorted_pending_tasks(&app.data)
                .iter()
                .position(|t| t.id == id)
                .map(|i| i as u32);
            assert_eq!(app.active_task_pending_index(), expected);
        }
    }

    #[test]
    fn archiving_the_active_task_clears_it_and_a_restores_it() {
        use crate::model::Priority;
        let mut app = app();
        let id = add(&mut app, "Park me", Priority::Medium);
        app.set_active_task(Some(id));
        select(&mut app, id);

        app.archive_selected_task();
        assert!(app.data.task(id).unwrap().archived);
        assert_eq!(app.task_ui.active_task, None);

        app.task_ui.task_filter = TaskFilter::Archived;
        select(&mut app, id);
        app.archive_selected_task();
        assert!(!app.data.task(id).unwrap().archived);
    }

    #[test]
    fn reordering_a_filtered_list_moves_past_the_visible_neighbour() {
        use crate::model::Priority;
        let mut app = app();
        let a = add(&mut app, "A", Priority::Medium);
        let hidden = add(&mut app, "Done", Priority::Medium);
        let c = add(&mut app, "C", Priority::Medium);
        storage::mark_task_done(&app.db, &mut app.data, hidden).unwrap();
        app.task_ui.task_filter = TaskFilter::Pending;
        select(&mut app, a);

        app.reorder_selected_task(1);

        let visible = app.ids_at(&app.task_ui.cached_filtered_tasks);
        assert_eq!(visible, vec![c, a]);
        assert_eq!(app.selected_task_id(), Some(a));
    }

    #[test]
    fn dashboard_reorder_stops_at_a_priority_boundary() {
        use crate::model::Priority;
        let mut app = app();
        let high = add(&mut app, "High", Priority::High);
        add(&mut app, "Low", Priority::Low);
        app.recompute_task_caches();
        app.task_ui.dashboard_task_state.select(Some(0));
        assert_eq!(app.dashboard_selected_task_id(), Some(high));

        app.reorder_dashboard_task(1);

        assert_eq!(app.dashboard_selected_task_id(), Some(high));
        assert!(app.ui.status.as_deref().unwrap_or("").contains("priority"));
    }

    #[test]
    fn auto_advance_stays_on_a_task_until_its_estimate_is_used() {
        let mut app = app();
        app.data.auto_advance_task = true;
        let mut ids = Vec::new();
        for title in ["Long task", "Next task"] {
            ids.push(
                storage::add_task_full(
                    &app.db,
                    &mut app.data,
                    storage::TaskPayload {
                        title: title.into(),
                        notes: String::new(),
                        estimated_minutes: 100,
                        priority: crate::model::Priority::Medium,
                        tags: Vec::new(),
                        due_date: None,
                    },
                )
                .unwrap(),
            );
        }
        app.set_active_task(Some(ids[0]));

        app.data.task_mut(ids[0]).unwrap().actual_minutes = 25;
        app.maybe_advance_task();
        assert_eq!(app.task_ui.active_task, Some(ids[0]));

        app.data.task_mut(ids[0]).unwrap().actual_minutes = 100;
        app.maybe_advance_task();
        assert_eq!(app.task_ui.active_task, Some(ids[1]));
    }
}

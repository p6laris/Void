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
        let pending = self.dashboard_tasks();
        if pending.is_empty() {
            None
        } else {
            let idx = self
                .dashboard_task_state
                .selected()
                .unwrap_or(0)
                .min(pending.len() - 1);
            Some(pending[idx].id)
        }
    }

    pub(crate) fn clamp_dashboard_task_selection(&mut self) {
        let n = self.dashboard_tasks().len();
        if n == 0 {
            self.dashboard_task_state.select(None);
        } else {
            let sel = self.dashboard_task_state.selected().unwrap_or(0).min(n - 1);
            self.dashboard_task_state.select(Some(sel));
        }
    }

    pub(crate) fn clamp_task_selection_after_mutation(&mut self) {
        let len = self.filtered_task_indices().len();
        if len == 0 {
            self.task_state.select(None);
        } else {
            let sel = self.task_state.selected().unwrap_or(0).min(len - 1);
            self.task_state.select(Some(sel));
        }
    }

    pub(crate) fn move_dashboard_task_selection(&mut self, delta: i32) {
        let count = self.dashboard_tasks().len();
        if count == 0 {
            return;
        }
        let cur = self.dashboard_task_state.selected().unwrap_or(0) as i32;
        let next = (cur + delta).rem_euclid(count as i32) as usize;
        self.dashboard_task_state.select(Some(next));
    }

    pub fn pending_task_count(&self) -> u32 {
        storage::pending_tasks(&self.data).count() as u32
    }

    pub fn active_task_pending_index(&self) -> Option<u32> {
        let id = self.active_task?;
        storage::sorted_pending_tasks(&self.data)
            .iter()
            .position(|t| t.id == id)
            .map(|i| i as u32)
    }

    pub fn active_task_progress(&self) -> Option<f64> {
        let id = self.active_task?;
        let task = self.data.tasks.iter().find(|t| t.id == id)?;
        Some(task.progress_ratio())
    }

    pub(crate) fn matches_filter(&self, t: &crate::model::Task) -> bool {
        if !self.task_search_lower.is_empty() {
            let q = &self.task_search_lower;
            let title_match = t.title.to_lowercase().contains(q);
            let tags_match = t.tags.iter().any(|tag| tag.to_lowercase().contains(q));
            if !title_match && !tags_match {
                return false;
            }
        }
        if let Some(ref tag) = self.active_tag_filter {
            if !t.tags.iter().any(|t| t == tag) {
                return false;
            }
        }
        match self.task_filter {
            TaskFilter::All => true,
            TaskFilter::Pending => t.status != crate::model::TaskStatus::Done && !t.archived,
            TaskFilter::Done => t.status == crate::model::TaskStatus::Done && !t.archived,
            TaskFilter::Today => {
                t.today && t.status != crate::model::TaskStatus::Done && !t.archived
            }
            TaskFilter::Archived => t.archived,
        }
    }

    pub fn recompute_task_caches(&mut self) {
        self.cached_filtered_tasks = self
            .data
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| self.matches_filter(t))
            .map(|(i, _)| i)
            .collect();

        let mut dash = self.cached_filtered_tasks.clone();
        dash.sort_by(|&a, &b| {
            let ta = &self.data.tasks[a];
            let tb = &self.data.tasks[b];
            tb.priority
                .rank()
                .cmp(&ta.priority.rank())
                .then(tb.today.cmp(&ta.today))
                .then(ta.sort_order.cmp(&tb.sort_order))
        });
        self.cached_dashboard_tasks = dash;
    }

    pub fn filtered_task_indices(&self) -> &[usize] {
        &self.cached_filtered_tasks
    }

    pub fn dashboard_tasks(&self) -> Vec<&crate::model::Task> {
        self.cached_dashboard_tasks
            .iter()
            .map(|&i| &self.data.tasks[i])
            .collect()
    }

    pub fn set_active_task(&mut self, id: Option<u64>) {
        if let Some(id) = id {
            if self
                .data
                .tasks
                .iter()
                .find(|t| t.id == id)
                .is_some_and(|t| t.status == crate::model::TaskStatus::Done)
            {
                self.set_status("That task is done — pick another.", true);
                return;
            }
            if self
                .data
                .tasks
                .iter()
                .find(|t| t.id == id)
                .is_some_and(|t| t.is_blocked(&self.data.tasks))
            {
                self.set_status("Task is blocked — complete dependencies first.", true);
                return;
            }
            self.persist_data(|db, data| storage::promote_task_on_activate(db, data, id));
        }
        self.active_task = id;
        self.data.active_task_id = id;
        self.persist(|db| db.persist_active_task(id));
    }

    pub fn cycle_active_task_status(&mut self) {
        let Some(id) = self.active_task else {
            self.set_status("No active task — set one on Tasks (Space).", true);
            return;
        };
        self.cycle_task_status_for(id, false);
    }

    pub fn cycle_task_status_for(&mut self, id: u64, set_active: bool) {
        if set_active {
            self.set_active_task(Some(id));
        }
        self.persist_data(|db, data| storage::cycle_task_status(db, data, id));
        let status = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.status);
        let Some(status) = status else {
            return;
        };
        if status == crate::model::TaskStatus::Done {
            if self.active_task == Some(id) {
                self.active_task = None;
                self.data.active_task_id = None;
                self.persist(|db| db.persist_active_task(None));
            }
            self.maybe_advance_task();
        }
        self.bump_data();
        self.set_status(format!("Task status: {}", status.label()), false);
        if status == crate::model::TaskStatus::Done {
            self.check_queue_empty();
        }
    }

    pub fn mark_active_task_done(&mut self) {
        let Some(id) = self.active_task else {
            self.set_status("No active task — set one on Tasks (Space).", true);
            return;
        };
        self.persist_data(|db, data| storage::mark_task_done(db, data, id));
        self.active_task = None;
        self.data.active_task_id = None;
        self.persist(|db| db.persist_active_task(None));
        self.bump_data();
        self.maybe_advance_task();
        if self.data.sound_enabled {
            sound::play_task_complete();
        }
        self.set_status("Task marked done.", false);
        self.check_queue_empty();
    }

    pub(crate) fn auto_pick_task_if_needed(&mut self) {
        if self.active_task.is_some() || !self.data.auto_pick_task {
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
        let next = storage::advance_to_next_task(&self.data, self.active_task);
        self.set_active_task(next);
        if let Some(id) = next {
            if let Some(t) = self.data.tasks.iter().find(|t| t.id == id) {
                self.set_status(format!("Next task: {}", t.title), false);
            }
        }
    }

    pub(crate) fn cycle_active_task(&mut self) {
        let pending: Vec<_> = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                t.status == crate::model::TaskStatus::Pending
                    || t.status == crate::model::TaskStatus::InProgress
            })
            .map(|t| t.id)
            .collect();
        if pending.is_empty() {
            self.set_status("No tasks available to switch to.", true);
            return;
        }
        let next_id = if let Some(current) = self.active_task {
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
            .tasks
            .iter()
            .find(|t| t.id == id)
            .is_some_and(|t| t.status == crate::model::TaskStatus::Done)
        {
            self.set_status("That task is done — pick another.", true);
            return;
        }
        self.set_active_task(Some(id));
        self.tab = FocusTab::Dashboard;
        if self.timer.mode != TimerMode::Focus {
            self.timer.configure(TimerMode::Focus);
        }
        self.start_timer();
    }

    pub fn cycle_task_filter(&mut self) {
        self.task_filter = self.task_filter.next();
        self.recompute_task_caches();
        self.task_state.select(Some(0));
        self.set_status(format!("Filter: {}", self.task_filter.label()), false);
    }

    pub fn toggle_bulk_mode(&mut self) {
        self.bulk_mode = !self.bulk_mode;
        self.bulk_selected.clear();
        if self.bulk_mode {
            if let Some(id) = self.selected_task_id() {
                self.bulk_selected.insert(id);
            }
        }
        self.set_status(
            format!(
                "Bulk select {} — [v]/Enter toggle row, [V] done, [D] delete, [q] exit",
                if self.bulk_mode { "on" } else { "off" }
            ),
            false,
        );
    }

    pub fn toggle_bulk_item(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        if self.bulk_selected.contains(&id) {
            self.bulk_selected.remove(&id);
        } else {
            self.bulk_selected.insert(id);
        }
        self.set_status(
            format!("{} task(s) selected", self.bulk_selected.len()),
            false,
        );
    }

    pub fn clamp_subtask_selection(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.subtask_selected = 0;
            self.subtask_state.select(None);
            return;
        };
        let n = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.subtasks.len())
            .unwrap_or(0);
        if n == 0 {
            self.subtask_selected = 0;
            self.subtask_focus = false;
            self.subtask_state.select(None);
        } else if self.subtask_selected >= n {
            self.subtask_selected = n - 1;
            self.subtask_state.select(Some(self.subtask_selected));
        } else {
            self.subtask_state.select(Some(self.subtask_selected));
        }
    }

    pub fn sync_subtask_list(&mut self) {
        self.clamp_subtask_selection();
    }

    pub fn selected_subtask_count(&self) -> usize {
        self.selected_task_id()
            .and_then(|id| {
                self.data
                    .tasks
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| t.subtasks.len())
            })
            .unwrap_or(0)
    }

    pub fn toggle_subtask_focus(&mut self) {
        if self.selected_subtask_count() == 0 {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        }
        self.subtask_focus = !self.subtask_focus;
        if self.subtask_focus {
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
        let n = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.subtasks.len())
            .unwrap_or(0);
        if n == 0 {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        }
        let cur = self.subtask_selected as i32;
        self.subtask_selected = (cur + delta).rem_euclid(n as i32) as usize;
        self.subtask_state.select(Some(self.subtask_selected));
        if let Some(s) = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.subtasks.get(self.subtask_selected))
        {
            self.set_status(format!("Subtask: {}", s.title), false);
        }
    }

    pub fn reset_subtask_selection(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.subtask_selected = 0;
            return;
        };
        let Some(t) = self.data.tasks.iter().find(|t| t.id == id) else {
            self.subtask_selected = 0;
            return;
        };
        if t.subtasks.is_empty() {
            self.subtask_selected = 0;
            self.subtask_state.select(None);
        } else {
            self.subtask_selected = t.subtasks.iter().position(|s| !s.done).unwrap_or(0);
            self.subtask_state.select(Some(self.subtask_selected));
        }
    }

    pub fn delete_subtask_on_selected(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let sub_id = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.subtasks.get(self.subtask_selected))
            .map(|s| s.id);
        let Some(sub_id) = sub_id else {
            self.set_status("No subtasks to remove.", true);
            return;
        };
        let title = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.subtasks.iter().find(|s| s.id == sub_id))
            .map(|s| s.title.clone())
            .unwrap_or_default();
        self.persist_data(|db, data| storage::delete_subtask(db, data, id, sub_id));
        self.bump_data();
        self.reset_subtask_selection();
        self.set_status(format!("Removed subtask \"{title}\""), false);
    }

    pub fn open_add_subtask(&mut self) {
        let Some(id) = self.selected_task_id() else {
            self.set_status("No task selected.", true);
            return;
        };
        self.input_buffer.clear();
        self.popup = Some(Popup::AddSubtask(id));
        self.input_mode = InputMode::Editing;
    }

    pub fn toggle_subtask_on_selected(&mut self) {
        let Some(id) = self.selected_task_id() else {
            return;
        };
        let sub_id = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .and_then(|t| t.subtasks.get(self.subtask_selected))
            .map(|s| s.id);
        let Some(sub_id) = sub_id else {
            self.set_status("No subtasks — press [c] to add.", true);
            return;
        };
        self.persist_data(|db, data| storage::toggle_subtask(db, data, id, sub_id));
        self.bump_data();
        self.clamp_subtask_selection();
        if let Some(t) = self.data.tasks.iter().find(|t| t.id == id) {
            if let Some(s) = t.subtasks.iter().find(|s| s.id == sub_id) {
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
        self.persist_data(|db, data| storage::archive_task(db, data, id));
        self.bump_data();
        self.clamp_task_selection_after_mutation();
        self.set_status("Task archived.", false);
    }
}

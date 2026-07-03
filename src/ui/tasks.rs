use super::*;

pub(crate) fn draw_tasks(f: &mut Frame, app: &mut App, area: Rect) {
    let icons = app.icons;
    let frame_today = app.frame_today();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let indices = &app.cached_filtered_tasks;
    let filtered_count = indices.len();

    let selected_idx = app.task_state.selected();
    let title_max = chunks[0].width.saturating_sub(22) as usize;
    let items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(list_idx, &idx)| {
            let t = &app.data.tasks[idx];
            let marker = task_status_icon(icons, t.status);
            let prio_color = match t.priority {
                crate::model::Priority::High => app.theme.warning,
                crate::model::Priority::Medium => app.theme.info,
                crate::model::Priority::Low => app.theme.dim,
            };
            let is_active = app.active_task == Some(t.id);
            let subtask_mark = t
                .subtask_progress()
                .map(|(d, n)| format!(" ({d}/{n})"))
                .unwrap_or_default();
            let blocked_mark = if app.is_task_blocked_at(idx) {
                "!"
            } else {
                ""
            };
            let is_reordering = app.reordering_task == Some(t.id);
            let reorder_mark = if is_reordering { " ↕ " } else { "" };
            let is_cursor = selected_idx == Some(list_idx);
            let bulk_selected = app.bulk_mode && app.bulk_selected.contains(&t.id);
            let bulk_mark = if app.bulk_mode {
                if bulk_selected {
                    Span::styled(
                        icons.check,
                        Style::default()
                            .fg(app.theme.on_accent)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("○", Style::default().fg(app.theme.dim))
                }
            } else {
                Span::raw("")
            };
            let style = if bulk_selected {
                Style::default()
                    .bg(app.theme.info)
                    .fg(app.theme.on_accent)
                    .add_modifier(Modifier::BOLD)
            } else if is_active && !is_cursor {
                Style::default()
                    .bg(app.theme.active_bg)
                    .fg(app.theme.active_fg)
                    .add_modifier(Modifier::BOLD)
            } else if t.is_overdue_on(frame_today) && !is_cursor {
                Style::default().fg(app.theme.error)
            } else {
                Style::default().fg(app.theme.text)
            };
            let overdue_mark = if t.is_overdue_on(frame_today) { icons.alert } else { " " };
            let today_mark = if t.today { icons.star } else { " " };
            let active_mark = if is_active { icons.task_active } else { " " };
            let active_style = if is_active {
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                style
            };
            let tags_label = if t.tags.is_empty() {
                String::new()
            } else {
                format!(" #{}", truncate(&t.tags.join(", "), 12))
            };
            let mut spans = vec![Span::styled(
                format!("{} ", active_mark),
                if is_active && is_cursor {
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_active {
                    active_style
                } else {
                    style
                },
            )];
            if app.bulk_mode {
                spans.push(bulk_mark);
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("{}{}{}{} ", overdue_mark, today_mark, marker, reorder_mark),
                if is_active && is_cursor {
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_active {
                    active_style
                } else {
                    style
                },
            ));
            if !blocked_mark.is_empty() {
                spans.push(Span::styled(
                    blocked_mark,
                    Style::default().fg(app.theme.warning),
                ));
            }
            spans.extend([
                Span::styled(
                    format!("{:<3} ", t.priority.label()),
                    Style::default().fg(prio_color),
                ),
                Span::styled(format!("{} ", truncate(&t.title, title_max.max(8))), style),
                Span::styled(subtask_mark, Style::default().fg(app.theme.dim)),
                Span::styled(
                    format!("{:>3}/{:<3}m", t.actual_minutes, t.estimated_minutes),
                    Style::default().fg(app.theme.dim),
                ),
            ]);
            if !tags_label.is_empty() {
                spans.push(Span::styled(tags_label, Style::default().fg(app.theme.info)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let filter_label = if app.task_search.is_empty() {
        app.task_filter.label().to_string()
    } else {
        format!("'{}'", app.task_search)
    };

    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    let has_overflow = filtered_count > visible_height;
    let at_bottom = app
        .task_state
        .selected()
        .map(|sel| sel + 1 >= filtered_count)
        .unwrap_or(true);
    let more_indicator = if has_overflow && !at_bottom {
        " ↓ more "
    } else {
        ""
    };

    let bulk_hint = if app.bulk_mode { " · BULK" } else { "" };
    let block = themed_panel(
        &app.theme,
        Line::from(vec![
            Span::styled(
                format!(
                    " {} Tasks [{}] ({}){} ",
                    icons.tasks, filter_label, filtered_count, bulk_hint
                ),
                Style::default()
                    .fg(if app.bulk_mode {
                        app.theme.info
                    } else {
                        app.theme.accent
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(more_indicator, Style::default().fg(app.theme.dim)),
        ]),
    );
    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(app.theme.select_bg)
                .fg(app.theme.select_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, chunks[0], &mut app.task_state);

    let detail_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(chunks[1]);
    let progress_ratio = app
        .task_state
        .selected()
        .and_then(|sel| indices.get(sel).copied())
        .map(|idx| app.data.tasks[idx].progress_ratio())
        .unwrap_or(0.0);
    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(app.theme.accent).bg(app.theme.dim))
            .ratio(progress_ratio)
            .label(format!("Progress {}%", (progress_ratio * 100.0) as u32))
            .block(themed_panel(
                &app.theme,
                Line::from(Span::styled(
                    " Progress ",
                    Style::default().fg(app.theme.accent),
                )),
            )),
        detail_layout[0],
    );
    let has_subtasks = app
        .task_state
        .selected()
        .and_then(|s| indices.get(s).copied())
        .map(|idx| !app.data.tasks[idx].subtasks.is_empty())
        .unwrap_or(false);

    if has_subtasks {
        let sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(detail_layout[1]);
        let meta_block = themed_panel(
            &app.theme,
            Line::from(Span::styled(" Details ", Style::default().fg(app.theme.accent))),
        );
        f.render_widget(
            Paragraph::new(build_task_detail_meta(app))
                .block(meta_block)
                .wrap(Wrap { trim: false }),
            sub_chunks[0],
        );
        if let Some(sel) = app
            .task_state
            .selected()
            .and_then(|s| indices.get(s).copied())
        {
            let task = app.data.tasks[sel].clone();
            draw_subtask_panel(f, app, sub_chunks[1], &task);
        }
    } else {
        let detail_block = themed_panel(
            &app.theme,
            Line::from(Span::styled(" Details ", Style::default().fg(app.theme.accent))),
        );
        f.render_widget(
            Paragraph::new(build_task_detail(app))
                .block(detail_block)
                .wrap(Wrap { trim: false }),
            detail_layout[1],
        );
    }

    if app.searching {
        let search_area = centered_rect(50, 20, area);
        f.render_widget(Clear, search_area);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "Search tasks (title or tags)",
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(format!("{}|", app.task_search)),
                Line::from(Span::styled(
                    "Enter confirm · Esc cancel",
                    Style::default().fg(app.theme.dim),
                )),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(app.theme.accent)),
            ),
            search_area,
        );
    }
}

pub(crate) fn build_task_detail(app: &App) -> Vec<Line<'_>> {
    let mut lines = build_task_detail_meta(app);
    let indices = &app.cached_filtered_tasks;
    if indices.is_empty() {
        return lines;
    }
    let sel = app
        .task_state
        .selected()
        .unwrap_or(0)
        .min(indices.len() - 1);
    let t = &app.data.tasks[indices[sel]];
    if t.subtasks.is_empty() && t.status != crate::model::TaskStatus::Done {
        lines.push(Line::from(Span::styled(
            "No subtasks — [c] add · [Tab] focus when added",
            Style::default().fg(app.theme.dim),
        )));
    }
    lines
}

fn build_task_detail_meta(app: &App) -> Vec<Line<'_>> {
    let frame_today = app.frame_today();
    let indices = &app.cached_filtered_tasks;
    if indices.is_empty() {
        let msg = match app.task_filter {
            TaskFilter::All => "No tasks yet. Press 'a' to add one.",
            TaskFilter::Pending => "All tasks done! Great work.",
            TaskFilter::Done => "No completed tasks yet.",
            TaskFilter::Today => "Nothing queued for today. Press 't' to tag tasks.",
            TaskFilter::Archived => "No archived tasks.",
        };
        return vec![Line::from(Span::styled(
            msg,
            Style::default().fg(app.theme.dim),
        ))];
    }
    let sel = app
        .task_state
        .selected()
        .unwrap_or(0)
        .min(indices.len() - 1);
    let task_idx = indices[sel];
    let t = &app.data.tasks[task_idx];
    let mut lines = Vec::new();
    if t.is_overdue_on(frame_today) {
        lines.push(Line::from(Span::styled(
            "OVERDUE",
            Style::default()
                .fg(app.theme.error)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));
    }
    let status_color = match t.status {
        crate::model::TaskStatus::Done => app.theme.success,
        crate::model::TaskStatus::InProgress => app.theme.warning,
        crate::model::TaskStatus::Pending => app.theme.dim,
    };
    lines.push(Line::from(Span::styled(
        t.title.clone(),
        Style::default().fg(app.theme.text).add_modifier(Modifier::BOLD),
    )));
    lines.extend(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("ID:        ", Style::default().fg(app.theme.dim)),
            Span::styled(format!("{}", t.id), Style::default().fg(app.theme.text)),
        ]),
        Line::from(vec![
            Span::styled("Priority:  ", Style::default().fg(app.theme.dim)),
            Span::styled(t.priority.label(), Style::default().fg(app.theme.warning)),
        ]),
        Line::from(vec![
            Span::styled("Status:    ", Style::default().fg(app.theme.dim)),
            Span::styled(t.status.label(), Style::default().fg(status_color)),
        ]),
        Line::from(vec![
            Span::styled("Estimate:  ", Style::default().fg(app.theme.dim)),
            Span::styled(
                format_minutes(t.estimated_minutes),
                Style::default().fg(app.theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled("Logged:    ", Style::default().fg(app.theme.dim)),
            Span::styled(
                format!(
                    "{} across {} sessions",
                    format_minutes(t.actual_minutes),
                    t.sessions
                ),
                Style::default().fg(app.theme.success),
            ),
        ]),
        Line::from(vec![
            Span::styled("Remaining: ", Style::default().fg(app.theme.dim)),
            Span::styled(
                format!(
                    "~{} sessions ({}m each)",
                    crate::storage::sessions_remaining_hint(t, app.data.focus_minutes),
                    app.data.focus_minutes
                ),
                Style::default().fg(app.theme.info),
            ),
        ]),
        Line::from(vec![
            Span::styled("Today:     ", Style::default().fg(app.theme.dim)),
            Span::styled(
                if t.today { "yes" } else { "no" },
                Style::default().fg(if t.today { app.theme.success } else { app.theme.dim }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Created:   ", Style::default().fg(app.theme.dim)),
            Span::styled(
                t.created_at.format("%Y-%m-%d %H:%M").to_string(),
                Style::default().fg(app.theme.text),
            ),
        ]),
    ]);
    if let Some(c) = t.completed_at {
        lines.push(Line::from(vec![
            Span::styled("Done:      ", Style::default().fg(app.theme.dim)),
            Span::styled(
                c.format("%Y-%m-%d %H:%M").to_string(),
                Style::default().fg(app.theme.success),
            ),
        ]));
    }
    if let Some(ref due) = t.due_date {
        let overdue = t.is_overdue_on(frame_today);
        lines.push(Line::from(vec![
            Span::styled("Due:       ", Style::default().fg(app.theme.dim)),
            Span::styled(
                due.clone(),
                Style::default().fg(if overdue { app.theme.error } else { app.theme.text }),
            ),
        ]));
    }
    if !t.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags:      ", Style::default().fg(app.theme.dim)),
            Span::styled(t.tags.join(", "), Style::default().fg(app.theme.info)),
        ]));
    }
    if t.recurrence != crate::model::TaskRecurrence::None {
        lines.push(Line::from(vec![
            Span::styled("Repeats:   ", Style::default().fg(app.theme.dim)),
            Span::styled(t.recurrence.label(), Style::default().fg(app.theme.info)),
        ]));
    }
    if !t.blocked_by.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Blocked:   ", Style::default().fg(app.theme.dim)),
            Span::styled(
                t.blocked_by
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                Style::default().fg(if app.is_task_blocked_at(task_idx) {
                    app.theme.error
                } else {
                    app.theme.text
                }),
            ),
        ]));
    }
    if t.subtasks.is_empty() && t.status != crate::model::TaskStatus::Done {
        // Shown in build_task_detail for tasks without subtask panel
    }
    if app.active_task == Some(t.id) {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("{} ACTIVE — press [f] to focus", app.icons.focus),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines
}

fn draw_subtask_panel(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    task: &crate::model::Task,
) {
    let theme = &app.theme;
    let (done, total) = task.subtask_progress().unwrap_or((0, 0));
    let focus_label = if app.subtask_focus { " · FOCUS" } else { "" };
    let border_color = if app.subtask_focus {
        theme.accent
    } else {
        theme.panel_border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Line::from(vec![
            Span::styled(
                format!(" Subtasks ({done}/{total}){focus_label} ",),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "[Tab] focus · j/k nav · x toggle · q back · - remove",
                Style::default().fg(theme.dim),
            ),
        ]));
    let inner = block.inner(area);
    f.render_widget(block, area);

    app.subtask_state.select(Some(app.subtask_selected));
    let items: Vec<ListItem> = task
        .subtasks
        .iter()
        .map(|s| {
            let mark = if s.done {
                Span::styled(
                    format!("{} ", app.icons.check),
                    Style::default().fg(theme.success),
                )
            } else {
                Span::styled("○ ", Style::default().fg(theme.dim))
            };
            let title = Span::styled(
                s.title.clone(),
                if s.done {
                    Style::default().fg(theme.dim)
                } else {
                    Style::default().fg(theme.text)
                },
            );
            ListItem::new(Line::from(vec![mark, title]))
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .bg(theme.select_bg)
            .fg(theme.select_fg)
            .add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, inner, &mut app.subtask_state);
}

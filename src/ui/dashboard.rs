use super::*;

pub(crate) fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    draw_compact_timer_block(f, app, chunks[0]);

    let today = app.today_focus_mins();
    let goal = app.data.daily_goal_minutes.max(1);
    let progress_ratio = (today as f64 / goal as f64).min(1.0);

    let task_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let task_block = dense_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} Tasks ", icons.tasks),
            Style::default().fg(theme.accent),
        )),
    );
    let indices = app.dashboard_task_indices();
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(task_chunks[0]);

    if indices.is_empty() {
        let empty_msg = if app.queue_empty() && !app.data.tasks.is_empty() {
            "All tasks done — free focus or [a] add more"
        } else {
            "No tasks yet — press [a] to add one"
        };
        let empty_list = List::new(vec![ListItem::new(Span::styled(
            empty_msg,
            Style::default().fg(if app.queue_empty() && !app.data.tasks.is_empty() {
                theme.success
            } else {
                theme.dim
            }),
        ))])
        .block(task_block);
        f.render_widget(empty_list, left_chunks[0]);
    } else {
        let selected_idx = app.dashboard_task_state.selected().unwrap_or(0);
        let pending: Vec<ListItem> = indices
            .iter()
            .enumerate()
            .map(|(idx, &task_i)| {
                let t = &app.data.tasks[task_i];
                let selected = idx == selected_idx;
                let marker = match t.priority {
                    crate::model::Priority::High => icons.alert,
                    crate::model::Priority::Medium => icons.dot,
                    crate::model::Priority::Low => " ",
                };
                let active = if app.active_task == Some(t.id) {
                    format!("{} ", icons.task_active)
                } else if selected {
                    format!("{} ", icons.chevron)
                } else {
                    "  ".into()
                };
                let status_color = task_status_color(theme, t.status);
                let row_style = if selected {
                    Style::default()
                        .bg(theme.select_bg)
                        .fg(theme.select_fg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(active, Style::default().fg(theme.accent)),
                    Span::styled(
                        format!("{} ", task_status_icon(icons, t.status)),
                        Style::default().fg(status_color),
                    ),
                    Span::styled(format!("[{}] ", marker), Style::default().fg(theme.warning)),
                    Span::styled(t.title.clone(), row_style),
                    Span::styled(
                        format!("  {}m", t.estimated_minutes),
                        Style::default().fg(theme.dim),
                    ),
                ]))
                .style(row_style)
            })
            .collect();
        let list = List::new(pending).block(task_block);
        f.render_stateful_widget(list, left_chunks[0], &mut app.dashboard_task_state);

        if let Some(id) = app.dashboard_selected_task_id() {
            if let Some(task) = app.data.tasks.iter().find(|t| t.id == id) {
                draw_dashboard_task_details(f, app, task, left_chunks[1]);
            }
        }
    }

    let goal_met = app.daily_goal_met();
    let remaining = goal.saturating_sub(today);
    let goal_reached = progress_ratio >= 1.0;
    let gauge_color = if goal_reached {
        theme.accent
    } else {
        theme.success
    };

    let goal_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(task_chunks[1]);

    let actual_ratio = if goal > 0 {
        today as f64 / goal as f64
    } else {
        0.0
    };

    f.render_widget(
        Gauge::default()
            .gauge_style(Style::default().fg(gauge_color).bg(theme.task_track))
            .ratio(progress_ratio)
            .label(format!(
                "{} {}/{} ({}%)",
                icons.target,
                format_minutes(today),
                format_minutes(goal),
                (actual_ratio * 100.0) as u32
            ))
            .block(dense_panel(
                theme,
                Line::from(Span::styled(
                    format!(" {} Daily goal ", icons.target),
                    Style::default().fg(theme.accent),
                )),
            )),
        goal_inner[0],
    );

    let goal_lines = vec![
        Line::from(Span::styled(
            if goal_met && today > goal {
                format!(
                    "{} Goal complete! (+{} over)",
                    icons.check,
                    format_minutes(today - goal)
                )
            } else if goal_met {
                format!("{} Goal complete!", icons.check)
            } else {
                format!(
                    "{} {} remaining today",
                    icons.dot,
                    format_minutes(remaining)
                )
            },
            Style::default().fg(if goal_met { theme.accent } else { theme.text }),
        )),
        Line::from(vec![
            Span::styled(format!("{} ", icons.fire), Style::default().fg(theme.info)),
            Span::styled(
                format!(
                    "{}d · {}d goal · {} open",
                    app.data.streak_days,
                    app.data.goal_streak_days,
                    crate::storage::pending_tasks(&app.data).count()
                ),
                Style::default().fg(theme.dim),
            ),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", icons.chart), Style::default().fg(theme.dim)),
            Span::styled(
                format!(
                    "{} all-time focus",
                    format_minutes(app.data.total_focus_minutes)
                ),
                Style::default().fg(theme.dim),
            ),
        ]),
    ];
    f.render_widget(
        Paragraph::new(goal_lines).block(dense_panel(
            theme,
            Line::from(Span::styled(
                format!(" {} Today ", icons.stats),
                Style::default().fg(theme.accent),
            )),
        )),
        goal_inner[1],
    );
}

pub(crate) fn draw_compact_timer_block(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let t = &app.timer;
    let mc = mode_color(theme, t.mode);

    let is_finished = t.state == crate::model::TimerState::Finished;
    let border_color = if is_finished {
        theme.success
    } else {
        theme.panel_border
    };
    let title_suffix = if is_finished {
        format!(" {} DONE ", app.icons.check)
    } else {
        String::new()
    };
    let outer = timer_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} {}", t.mode.label(), title_suffix),
            Style::default()
                .fg(if is_finished { theme.success } else { mc })
                .add_modifier(Modifier::BOLD),
        )),
        border_color,
    );
    f.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let on_break = is_break_mode(t.mode);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if on_break {
            [
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(2),
            ]
        } else {
            [
                Constraint::Min(5),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        })
        .split(inner);

    let cycle = t.config.long_break_every.max(1);
    let style = theme.scene_style(mc);
    let options = DashboardSceneOptions {
        task_progress: app.active_task_progress(),
        pending_tasks: app.pending_task_count(),
        active_task_index: app.active_task_pending_index(),
        sessions_done: t.completed_focus_sessions % cycle,
        sessions_total: cycle,
        layout: crate::canvas_timer::SceneLayout::Dashboard,
    };
    draw_dashboard_canvas(f, layout[0], t, &style, &options);

    let (main_time, tenths, _) = format_time_stack(t);
    let today_logged = app.today_focus_mins();

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                main_time,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(tenths, Style::default().fg(theme.dim)),
        ]),
        Line::from(Span::styled(
            format!(
                "{} logged today",
                super::widgets::format_minutes(today_logged)
            ),
            Style::default().fg(theme.dim),
        )),
    ];

    if t.mode == TimerMode::Focus {
        let (quality_label, quality_color) = match t.session_pause_count {
            0 => ("Uninterrupted Focus", theme.success),
            1 | 2 => ("Minor Interruptions", theme.warning),
            _ => ("Heavy Interruptions", theme.error),
        };
        lines.push(Line::from(Span::styled(
            format!("{} {}", app.icons.chart, quality_label),
            Style::default().fg(quality_color),
        )));
    } else {
        lines.push(Line::from(""));
    }

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        layout[1],
    );

    draw_timer_footer(f, app, &layout[2..], mc);
}

pub(crate) fn draw_timer_footer(f: &mut Frame, app: &App, areas: &[Rect], mc: Color) {
    let theme = &app.theme;
    let t = &app.timer;

    let cycle = t.config.long_break_every.max(1);
    let done_in_cycle = t.completed_focus_sessions % cycle;
    let in_focus = t.mode == TimerMode::Focus
        && matches!(
            t.state,
            crate::model::TimerState::Running | crate::model::TimerState::Paused
        );
    let dots = session_dots(done_in_cycle, cycle, in_focus);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(dots, Style::default().fg(mc)),
            Span::styled(
                format!("  {}  ", t.cycle_label()),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("{}% left", ((1.0 - t.progress()) * 100.0) as u32),
                Style::default().fg(theme.dim),
            ),
        ]))
        .alignment(Alignment::Center),
        areas[0],
    );

    if let Some(mut spans) = active_task_spans(app, theme) {
        if let Some(id) = app.active_task {
            if let Some(task) = app.data.tasks.iter().find(|t| t.id == id) {
                let left = crate::storage::sessions_remaining_hint(task, app.data.focus_minutes);
                if left > 0 {
                    spans.push(Span::styled(
                        format!("  ~{} left", left),
                        Style::default().fg(theme.dim),
                    ));
                }
                f.render_widget(
                    Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
                    areas[1],
                );
            }
        }
    } else if areas.len() > 1 {
        let msg = if app.queue_empty() {
            "All tasks done — free focus (general sessions)"
        } else {
            "No active task — Tasks tab, Space to set one"
        };
        f.render_widget(
            Paragraph::new(Span::styled(msg, Style::default().fg(theme.dim)))
                .alignment(Alignment::Center),
            areas[1],
        );
    }

    if areas.len() > 2 {
        if is_break_mode(t.mode) {
            draw_break_tip(f, areas[2], t, mc, theme.text, theme.dim, app.icons.heart);
        } else {
            let state_label = match t.state {
                crate::model::TimerState::Idle => "ready",
                crate::model::TimerState::Running => "focusing",
                crate::model::TimerState::Paused => "paused",
                crate::model::TimerState::Finished => "complete",
            };
            f.render_widget(
                Paragraph::new(Span::styled(state_label, Style::default().fg(mc)))
                    .alignment(Alignment::Center),
                areas[2],
            );
        }
    }
}

pub(crate) fn mode_color(theme: &crate::app::Theme, mode: TimerMode) -> Color {
    match mode {
        TimerMode::Focus => theme.accent,
        TimerMode::ShortBreak => theme.success,
        TimerMode::LongBreak => theme.warning,
        TimerMode::Custom => theme.info,
    }
}

fn draw_dashboard_task_details(f: &mut Frame, app: &App, task: &crate::model::Task, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let mut lines = Vec::new();

    if !task.notes.is_empty() {
        lines.push(Line::from(Span::styled(
            truncate(&task.notes, area.width as usize - 4),
            Style::default().fg(theme.dim),
        )));
        lines.push(Line::from(""));
    }

    if app.is_task_blocked(task.id) {
        let mut blocker_names = Vec::new();
        for &b_id in &task.blocked_by {
            if let Some(b) = app
                .data
                .tasks
                .iter()
                .find(|t| t.id == b_id && t.status != crate::model::TaskStatus::Done)
            {
                blocker_names.push(b.title.clone());
            }
        }
        if !blocker_names.is_empty() {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", icons.alert),
                    Style::default().fg(theme.error),
                ),
                Span::styled(
                    format!("Blocked by: {}", blocker_names.join(", ")),
                    Style::default().fg(theme.error),
                ),
            ]));
            lines.push(Line::from(""));
        }
    }

    if !task.tags.is_empty() {
        let mut tag_spans = vec![Span::raw("  ")];
        for tag in &task.tags {
            tag_spans.push(Span::styled(
                format!(" {} ", tag),
                Style::default()
                    .bg(theme.panel_border)
                    .fg(theme.text)
                    .add_modifier(Modifier::BOLD),
            ));
            tag_spans.push(Span::raw(" "));
        }
        lines.push(Line::from(tag_spans));
        lines.push(Line::from(""));
    }

    if task.estimated_minutes > 0 {
        let actual = task.actual_minutes;
        let est = task.estimated_minutes;
        let time_str = format!(
            "{} / {}",
            super::widgets::format_minutes(actual),
            super::widgets::format_minutes(est)
        );

        let (indicator, color) = if actual < est {
            (
                format!(
                    "(Running {} ahead)",
                    super::widgets::format_minutes(est - actual)
                ),
                theme.success,
            )
        } else if actual > est {
            (
                format!(
                    "(Running {} over)",
                    super::widgets::format_minutes(actual - est)
                ),
                theme.warning,
            )
        } else {
            ("(On track)".to_string(), theme.dim)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", icons.timer),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("Time: {}  ", time_str),
                Style::default().fg(theme.text),
            ),
            Span::styled(indicator, Style::default().fg(color)),
        ]));
        lines.push(Line::from(""));
    }

    for (i, subtask) in task.subtasks.iter().enumerate().take(9) {
        let icon = if subtask.done { icons.check } else { icons.dot };
        let style = if subtask.done {
            Style::default().fg(theme.dim)
        } else {
            Style::default().fg(theme.text)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" [{}] {} ", i + 1, icon), style),
            Span::styled(truncate(&subtask.title, area.width as usize - 10), style),
        ]));
    }

    let recent_for_task: Vec<_> = app
        .recent_sessions
        .iter()
        .filter(|s| s.record.task_id == Some(task.id))
        .take(3)
        .collect();
    if !recent_for_task.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Recent Activity",
            Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
        )));
        for s in recent_for_task {
            let local_time = s.record.completed_at.with_timezone(&chrono::Local);
            let time_str = local_time.format("%b %d, %H:%M").to_string();
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", icons.dot), Style::default().fg(theme.dim)),
                Span::styled(format!("{}: ", time_str), Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{}m {}", s.record.minutes, s.record.mode.label()),
                    Style::default().fg(theme.text),
                ),
            ]));
        }
    }

    let block = dense_panel(
        theme,
        Line::from(Span::styled(" Details ", Style::default().fg(theme.dim))),
    );
    f.render_widget(Paragraph::new(lines).block(block), area);
}

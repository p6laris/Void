use super::*;

/// Three stacked bands rather than a 2x2 grid of panels:
///
///   timer         — the thing you look at while focusing
///   today         — full-width goal bar, the day's headline number
///   tasks│details — everything left, split into two real columns
///
/// The old layout gave the task list a quarter of the screen and the goal bar a narrow
/// side column; this gives each band a job and lets the list breathe.
pub(crate) fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(46),
            Constraint::Length(4),
            Constraint::Min(6),
        ])
        .split(area);

    draw_compact_timer_block(f, app, rows[0]);
    draw_today_band(f, app, rows[1]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55),
            Constraint::Length(3),
            Constraint::Percentage(45),
        ])
        .split(rows[2]);
    vertical_rule(f, &app.theme, cols[1]);
    let cols = [cols[0], cols[2]];

    draw_dashboard_tasks(f, app, cols[0]);

    match app
        .dashboard_selected_task_id()
        .and_then(|id| app.data.task(id))
    {
        Some(task) => draw_dashboard_task_details(f, app, task, cols[1]),
        None => {
            let theme = &app.theme;
            f.render_widget(
                Paragraph::new(comment_line(theme, "select a task to see its details"))
                    .block(dense_panel(
                        theme,
                        section_title(theme, app.icons.about, "Details"),
                    )),
                cols[1],
            );
        }
    }
}

/// Full-width daily goal band: one bar, one annotation line.
fn draw_today_band(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let today = app.today_focus_mins();
    let goal = app.data.daily_goal_minutes.max(1);
    let ratio = today as f64 / goal as f64;
    let goal_met = app.daily_goal_met();
    let fill = if goal_met { theme.success } else { theme.accent };

    let block = dense_panel(theme, section_title(theme, icons.target, "Today"));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    let label = format!(
        "  {} / {}  ({}%)",
        format_minutes(today),
        format_minutes(goal),
        (ratio * 100.0) as u32
    );
    let bar_w = (inner.width as usize).saturating_sub(label.chars().count() + 2);
    let mut spans = vec![Span::raw(" ")];
    spans.extend(text_gauge(theme, ratio.min(1.0), bar_w, fill));
    spans.push(Span::styled(
        label,
        Style::default()
            .fg(if goal_met { theme.success } else { theme.text })
            .add_modifier(Modifier::BOLD),
    ));

    let status = if goal_met && today > goal {
        format!("goal met — {} over", format_minutes(today - goal))
    } else if goal_met {
        "goal met".to_string()
    } else {
        format!("{} remaining", format_minutes(goal.saturating_sub(today)))
    };

    let lines = vec![
        Line::from(spans),
        // Leading space keeps the annotation flush with the bar above it.
        Line::from(vec![
            Span::raw(" "),
            comment_span(
                theme,
                format!(
                    "{} · {}d streak · {}d goal streak · {} open · {} all-time",
                    status,
                    app.data.streak_days,
                    app.data.goal_streak_days,
                    app.pending_task_count(),
                    format_minutes(app.data.total_focus_minutes),
                ),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_dashboard_tasks(f: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let indices = app.dashboard_task_indices();

    let count = indices.len();
    let block = dense_panel(theme, section_title(theme, icons.tasks, "Up next")).title(
        Line::from(Span::styled(
            format!(" {count} open "),
            Style::default().fg(theme.comment),
        ))
        .alignment(Alignment::Right),
    );

    if indices.is_empty() {
        let empty_msg = if app.queue_empty() && !app.data.tasks.is_empty() {
            "all tasks done — free focus, or [a] to add more"
        } else {
            "no tasks yet — press [a] to add one"
        };
        f.render_widget(
            Paragraph::new(comment_line(theme, empty_msg)).block(block),
            area,
        );
        return;
    }

    let selected_idx = app.task_ui.dashboard_task_state.selected().unwrap_or(0);
    let est_w = 6;
    let title_w = (area.width as usize).saturating_sub(est_w + 10).max(8);

    let rows: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(idx, &task_i)| {
            let t = &app.data.tasks[task_i];
            let selected = idx == selected_idx;
            // Priority shows only when it is not the default — a marker on every row is
            // noise, and Low needs no ink at all.
            let (marker, marker_color) = match t.priority {
                crate::model::Priority::High => (icons.alert, theme.error),
                crate::model::Priority::Medium => (icons.dot, theme.dim),
                crate::model::Priority::Low => (" ", theme.dim),
            };
            let lead = if app.task_ui.active_task == Some(t.id) {
                format!("{} ", icons.task_active)
            } else if selected {
                format!("{} ", icons.chevron)
            } else {
                "  ".into()
            };
            let row_style = if selected {
                Style::default()
                    .bg(theme.select_bg)
                    .fg(theme.select_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            // A selected row paints its own background, so the checkbox has to inherit
            // that style rather than keep its own colours.
            let inherit = selected.then_some(row_style);

            let title = truncate(&t.title, title_w);
            let pad = title_w.saturating_sub(unicode_width::UnicodeWidthStr::width(title.as_str()));

            let mut spans = vec![Span::styled(lead, Style::default().fg(theme.accent))];
            spans.extend(status_checkbox(theme, icons, t.status, inherit));
            spans.push(Span::styled(
                format!(" {marker} "),
                inherit.unwrap_or(Style::default().fg(marker_color)),
            ));
            spans.push(Span::styled(format!("{title}{}", " ".repeat(pad)), row_style));
            spans.push(Span::styled(
                format!("{:>5}m", t.estimated_minutes),
                inherit.unwrap_or(Style::default().fg(theme.comment)),
            ));
            ListItem::new(Line::from(spans)).style(row_style)
        })
        .collect();

    let list = List::new(rows).block(block);
    f.render_stateful_widget(list, area, &mut app.task_ui.dashboard_task_state);
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
        format!(" {} done ", app.icons.check)
    } else {
        String::new()
    };
    // Run state lives in the header instead of a lone word at the bottom of the panel —
    // it belongs with the mode, and it buys the canvas a line back.
    let (state_label, state_color) = match t.state {
        crate::model::TimerState::Idle => ("ready", theme.dim),
        crate::model::TimerState::Running => ("focusing", mc),
        crate::model::TimerState::Paused => ("paused", theme.warning),
        crate::model::TimerState::Finished => ("complete", theme.success),
    };
    let outer = timer_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} {}", t.mode.label().to_lowercase(), title_suffix),
            Style::default()
                .fg(if is_finished { theme.success } else { mc })
                .add_modifier(Modifier::BOLD),
        )),
        border_color,
    )
    .title(
        Line::from(Span::styled(
            format!("{state_label} "),
            Style::default().fg(state_color),
        ))
        .alignment(Alignment::Right),
    );
    f.render_widget(outer, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let on_break = t.mode.is_break();
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
        comment_line(
            theme,
            format!(
                "{} logged today",
                super::widgets::format_minutes(today_logged)
            ),
        ),
    ];

    if t.mode == TimerMode::Focus {
        let (quality_label, quality_color) = match t.session_pause_count {
            0 => ("uninterrupted focus", theme.success),
            1 | 2 => ("minor interruptions", theme.warning),
            _ => ("heavy interruptions", theme.error),
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
    // Marks the current slot whenever we are on a focus session, running or not — an idle
    // timer is still sitting on session N of the cycle.
    let dots = session_dots(done_in_cycle, cycle, t.mode == TimerMode::Focus);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(dots, Style::default().fg(mc)),
            Span::styled(
                format!("   {}   ", t.cycle_label().to_lowercase()),
                Style::default().fg(theme.dim),
            ),
            Span::styled(
                format!("{}% left", ((1.0 - t.progress()) * 100.0) as u32),
                Style::default().fg(theme.comment),
            ),
        ]))
        .alignment(Alignment::Center),
        areas[0],
    );

    if let Some(mut spans) = active_task_spans(app, theme) {
        if let Some(id) = app.task_ui.active_task {
            if let Some(task) = app.data.task(id) {
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
            "all tasks done — free focus, logging general sessions"
        } else {
            "no active task — pick one on the tasks tab with [space]"
        };
        f.render_widget(
            Paragraph::new(comment_line(theme, msg)).alignment(Alignment::Center),
            areas[1],
        );
    }

    // Run state moved to the panel header, so this row is now only the break tip. On a
    // focus session it stays blank on purpose — the whitespace is what makes the band calm.
    if areas.len() > 2 && t.mode.is_break() {
        draw_break_tip(f, areas[2], t, mc, theme.text, theme.dim, app.icons.heart);
    }
}

pub(crate) fn mode_color(theme: &crate::app::Theme, mode: TimerMode) -> Color {
    theme.mode_color(mode)
}

fn draw_dashboard_task_details(f: &mut Frame, app: &App, task: &crate::model::Task, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let mut lines = Vec::new();

    // `area` can be narrower than the padding on a small terminal; saturating_sub keeps
    // these from underflowing into a panic.
    let text_w = (area.width as usize).saturating_sub(4).max(8);

    if !task.notes.is_empty() {
        lines.push(comment_line(theme, truncate(&task.notes, text_w)));
        lines.push(Line::from(""));
    }

    if app.is_task_blocked(task.id) {
        let mut blocker_names = Vec::new();
        for &b_id in &task.blocked_by {
            if let Some(b) = app
                .data
                .tasks
                .get(&b_id)
                .filter(|t| t.status != crate::model::TaskStatus::Done)
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
                    format!("blocked by: {}", blocker_names.join(", ")),
                    Style::default().fg(theme.error),
                ),
            ]));
            lines.push(Line::from(""));
        }
    }

    if !task.tags.is_empty() {
        let mut tag_spans = vec![Span::raw("  ")];
        for tag in &task.tags {
            tag_spans.extend(tag_span(theme, tag));
            tag_spans.push(Span::raw("  "));
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
                format!("{} ahead", super::widgets::format_minutes(est - actual)),
                theme.success,
            )
        } else if actual > est {
            (
                format!("{} over", super::widgets::format_minutes(actual - est)),
                theme.warning,
            )
        } else {
            ("on track".to_string(), theme.dim)
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", icons.timer),
                Style::default().fg(theme.dim),
            ),
            Span::styled(format!("{time_str}  "), Style::default().fg(theme.text)),
            Span::styled(indicator, Style::default().fg(color)),
        ]));
        lines.push(Line::from(""));
    }

    lines.extend(super::widgets::subtask_inline_lines(
        &task.subtasks,
        theme,
        icons,
        " ",
        Some(text_w.saturating_sub(6).max(8)),
    ));

    let recent_for_task: Vec<_> = app
        .stats
        .recent_sessions
        .iter()
        .filter(|s| s.record.task_id == Some(task.id))
        .take(3)
        .collect();
    if !recent_for_task.is_empty() {
        lines.push(Line::from(""));
        lines.push(comment_line(theme, "recent activity"));
        for s in recent_for_task {
            let local_time = s.record.completed_at.with_timezone(&chrono::Local);
            let time_str = local_time.format("%b %d, %H:%M").to_string();
            lines.push(Line::from(vec![
                Span::styled(format!("   {} ", icons.dot), Style::default().fg(theme.dim)),
                Span::styled(format!("{time_str}  "), Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{}m {}", s.record.minutes, s.record.mode.label()),
                    Style::default().fg(theme.text),
                ),
            ]));
        }
    }

    let block = dense_panel(theme, section_title(theme, icons.about, "Details"));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

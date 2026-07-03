use super::*;

pub(crate) fn draw_zen_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let t = &app.timer;
    let on_break = t.mode.is_break();
    let mc = mode_color(theme, t.mode);

    let mut active_task_ref = None;
    if let Some(id) = app.active_task {
        if let Some(task) = app.data.task(id) {
            active_task_ref = Some(task);
        }
    }

    let chunks = Layout::default()
        .constraints([
            Constraint::Min(1),
            Constraint::Length(if on_break { 3 } else { 0 }),
            Constraint::Length(2),
        ])
        .split(area);

    let cycle = t.config.long_break_every.max(1);
    let style = theme.scene_style(mc);
    let options = ZenSceneOptions {
        task_progress: app.active_task_progress(),
        sessions_done: t.completed_focus_sessions % cycle,
        sessions_total: cycle,
        pending_tasks: app.pending_task_count(),
        active_task_index: app.active_task_pending_index(),
        layout: crate::canvas_timer::SceneLayout::Zen,
    };
    draw_zen_canvas(f, chunks[0], t, &style, &options);

    let (main_time, tenths, _) = format_time_stack(t);
    let session_label = t.cycle_label();

    let mut overlay_lines = vec![
        Line::from(vec![
            Span::styled(
                main_time,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(tenths, Style::default().fg(theme.dim)),
        ]),
        Line::from(Span::styled(session_label, Style::default().fg(theme.dim))),
        Line::from(Span::raw(" ")),
    ];

    if let Some(task) = active_task_ref {
        overlay_lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", icons.task_active),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                truncate(&task.title, 64),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  {} {}",
                    task_status_icon(icons, task.status),
                    task.status.short_label()
                ),
                Style::default().fg(task_status_color(theme, task.status)),
            ),
        ]));

        if task.estimated_minutes > 0 {
            let sessions_done = task.sessions;
            let sessions_total = (task.estimated_minutes as f32
                / app.timer.config.focus_minutes.max(1) as f32)
                .ceil() as u32;
            let sessions_total = sessions_total.max(1);
            let percent = (sessions_done as f32 / sessions_total as f32).min(1.0);
            let width = 16;
            let filled = (percent * width as f32).round() as usize;
            let bar = format!("{}{}", "█".repeat(filled), "░".repeat(width - filled));
            overlay_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", icons.timer),
                    Style::default().fg(theme.dim),
                ),
                Span::styled(
                    format!("{}/{} sessions  ", sessions_done, sessions_total),
                    Style::default().fg(theme.text),
                ),
                Span::styled(bar, Style::default().fg(theme.accent)),
            ]));
        }

        for (i, subtask) in task.subtasks.iter().enumerate().take(9) {
            let icon = if subtask.done { icons.check } else { icons.dot };
            let style = if subtask.done {
                Style::default().fg(theme.dim)
            } else {
                Style::default().fg(theme.text)
            };
            overlay_lines.push(Line::from(vec![
                Span::styled(format!("   [{}] {} ", i + 1, icon), style),
                Span::styled(&subtask.title, style),
            ]));
        }
    } else {
        overlay_lines.push(Line::from(Span::styled(
            if app.queue_empty() {
                format!("{} All tasks done — free focus", icons.check)
            } else {
                format!("{} No active task — [f] on dashboard", icons.dot)
            },
            Style::default().fg(theme.dim),
        )));
    }

    let time_area = Rect {
        x: chunks[0].x,
        y: chunks[0].y + chunks[0].height.saturating_sub(overlay_lines.len() as u16) / 2,
        width: chunks[0].width,
        height: overlay_lines.len() as u16,
    };
    f.render_widget(
        Paragraph::new(overlay_lines).alignment(Alignment::Center),
        time_area,
    );

    if on_break {
        draw_break_tip(f, chunks[1], t, mc, theme.text, theme.dim, icons.heart);
    }

    chrome::draw_zen_footer(f, app, chunks[2]);
}

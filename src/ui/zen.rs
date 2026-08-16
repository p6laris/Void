use super::*;

/// Blank columns kept between the overlay text and the edge of the cleared plate.
///
/// A box cut tight to the glyphs reads as a hard rectangle stamped on the artwork; the
/// margin reads as space the canvas simply does not enter.
const PLATE_PAD_X: u16 = 6;

pub(crate) fn draw_zen_dashboard(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let t = &app.timer;
    let on_break = t.mode.is_break();
    let mc = mode_color(theme, t.mode);

    let mut active_task_ref = None;
    if let Some(id) = app.task_ui.active_task {
        if let Some(task) = app.data.task(id) {
            active_task_ref = Some(task);
        }
    }

    let chunks = Layout::default()
        .constraints([Constraint::Min(1), Constraint::Length(2)])
        .split(area);

    let cycle = t.config.long_break_every.max(1);
    let style = theme.scene_style(mc);
    let is_canvas_off = app.data.canvas_mode == crate::model::CanvasMode::Off;

    if !is_canvas_off {
        let options = ZenSceneOptions {
            task_progress: app.active_task_progress(),
            sessions_done: t.completed_focus_sessions % cycle,
            sessions_total: cycle,
            pending_tasks: app.pending_task_count(),
            active_task_index: app.active_task_pending_index(),
            layout: crate::canvas_timer::SceneLayout::Zen,
            animated: app.data.canvas_mode == crate::model::CanvasMode::Animated,
        };
        draw_zen_canvas(f, chunks[0], t, &style, &options);
    }

    // Everything below is laid out to fit inside the wreath, so the ring stays a whole
    // circle instead of being cut in half by the plate that clears the canvas behind it.
    let plate_w = if is_canvas_off {
        chunks[0].width
    } else {
        crate::canvas_timer::scene_plate_width(chunks[0])
    };
    let text_cap = plate_w.saturating_sub(PLATE_PAD_X * 2).max(12) as usize;

    let (main_time, tenths, _) = format_time_stack(t);
    let cycle_done = t.completed_focus_sessions % cycle;
    let on_focus_cycle = t.mode == TimerMode::Focus;

    // Zen shows one thing at a time. Blank lines are load-bearing here — they are what
    // separates this from the dashboard, which shows everything at once.
    let mut overlay_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                main_time,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(tenths, Style::default().fg(theme.dim)),
        ]),
        comment_line(theme, t.cycle_label().to_lowercase()),
        Line::from(""),
        Line::from(Span::styled(
            session_dots(cycle_done, cycle, on_focus_cycle),
            Style::default().fg(mc),
        )),
    ];

    if is_canvas_off {
        let progress_ratio = t.progress().clamp(0.0, 1.0);
        let bar_width = 20.min((chunks[0].width as usize).saturating_sub(10)).max(6);
        let filled = (progress_ratio * bar_width as f64).round() as usize;
        let bar_str: String = (0..bar_width)
            .map(|i| if i < filled { '━' } else { '─' })
            .collect();
        let pct = (progress_ratio * 100.0) as u32;
        overlay_lines.push(Line::from(vec![
            Span::styled(bar_str, Style::default().fg(mc)),
            Span::styled(format!(" {:>3}%", pct), Style::default().fg(theme.dim)),
        ]));
    }
    overlay_lines.push(Line::from(""));

    if let Some(task) = active_task_ref {
        overlay_lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", icons.task_active),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                truncate(&task.title, text_cap.saturating_sub(2)),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));

        if task.estimated_minutes > 0 {
            let sessions_done = task.sessions;
            let sessions_total = (task.estimated_minutes as f32
                / app.timer.config.focus_minutes.max(1) as f32)
                .ceil()
                .max(1.0) as u32;
            let ratio = (sessions_done as f64 / sessions_total as f64).min(1.0);
            let gauge_w = text_cap.saturating_sub(16).clamp(6, 18);
            let mut spans = super::widgets::text_gauge(theme, ratio, gauge_w, theme.accent);
            spans.push(Span::styled(
                format!("  {sessions_done}/{sessions_total} sessions"),
                Style::default().fg(theme.comment),
            ));
            overlay_lines.push(Line::from(spans));
        }

        // At most three subtasks — a full checklist belongs on the Tasks tab, not here.
        let shown: Vec<_> = task.subtasks.iter().take(3).cloned().collect();
        if !shown.is_empty() {
            overlay_lines.push(Line::from(""));
            overlay_lines.extend(super::widgets::subtask_inline_lines(
                &shown,
                theme,
                icons,
                "",
                Some(text_cap),
            ));
            if task.subtasks.len() > shown.len() {
                overlay_lines.push(comment_line(
                    theme,
                    format!("+{} more", task.subtasks.len() - shown.len()),
                ));
            }
        }
    } else {
        overlay_lines.push(comment_line(
            theme,
            if app.queue_empty() {
                "all tasks done — free focus".to_string()
            } else {
                "no active task — [f] on the dashboard".to_string()
            },
        ));
    }

    overlay_lines.push(Line::from(""));

    // The overlay sits on top of the animated canvas. Without clearing a plate behind it
    // the canvas glyphs collide with the digits and the timer becomes unreadable.
    //
    // The plate is deliberately wider than the text: a box cut tight to the glyphs reads
    // as a hard rectangle stamped on the artwork, whereas the margin reads as space the
    // canvas simply does not enter. The leading and trailing blank lines above do the
    // same job vertically.
    let text_w = overlay_lines
        .iter()
        .map(|l| l.width() as u16)
        .max()
        .unwrap_or(0);
    let overlay_w = text_w
        .saturating_add(PLATE_PAD_X * 2)
        .min(plate_w)
        .min(chunks[0].width);
    let overlay_h = (overlay_lines.len() as u16).min(chunks[0].height);
    let time_area = Rect {
        x: chunks[0].x + chunks[0].width.saturating_sub(overlay_w) / 2,
        y: chunks[0].y + chunks[0].height.saturating_sub(overlay_h) / 2,
        width: overlay_w,
        height: overlay_h,
    };
    f.render_widget(Clear, time_area);
    f.render_widget(
        Block::default().style(Style::default().bg(theme.bg)),
        time_area,
    );
    f.render_widget(
        Paragraph::new(overlay_lines).alignment(Alignment::Center),
        time_area,
    );

    if on_break {
        let tip_area = Rect {
            x: chunks[0].x,
            y: chunks[0].y + chunks[0].height.saturating_sub(1),
            width: chunks[0].width,
            height: 1,
        };
        draw_break_tip(f, tip_area, t, mc, theme.text, theme.dim, icons.heart);
    }

    chrome::draw_zen_footer(f, app, chunks[1]);
}

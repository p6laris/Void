use super::*;
use crate::model::{Priority, TaskStatus, TimerMode};
use crate::ui::widgets::{
    chip, comment_line, dense_panel, format_minutes, section_title, status_checkbox, tag_span,
    text_gauge, timer_panel, truncate,
};

/// Three stacked bands rather than a 2x2 grid of panels:
///
///   timer         — the thing you look at while focusing
///   today         — full-width goal bar, the day's headline number
///   tasks│details — everything left, split into two real columns
pub(crate) fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
    // Rows the timer band needs to show the clock; on short terminals the task list gives way first.
    const TIMER_MIN_ROWS: u16 = 5;
    let timer_rows = (area.height * 46 / 100)
        .max(TIMER_MIN_ROWS)
        .min(area.height);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(timer_rows),
            Constraint::Length(4),
            Constraint::Min(0),
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

    draw_dashboard_tasks(f, app, cols[0]);

    match app
        .dashboard_selected_task_id()
        .and_then(|id| app.data.task(id))
    {
        Some(task) => draw_dashboard_task_details(f, app, task, cols[2]),
        None => {
            let theme = &app.theme;
            f.render_widget(
                Paragraph::new(comment_line(theme, "select a task to see its details")).block(
                    dense_panel(theme, section_title(theme, app.icons.about, "Details")),
                ),
                cols[2],
            );
        }
    }
}

/// Full-width daily goal band: one bar, styled stat chips.
fn draw_today_band(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let today = app.today_focus_mins();
    let goal = app.data.daily_goal_minutes.max(1);
    let ratio = today as f64 / goal as f64;
    let goal_met = app.daily_goal_met();
    let fill = if goal_met {
        theme.success
    } else {
        theme.accent
    };

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

    let status_str = if goal_met && today > goal {
        format!("+{} over goal", format_minutes(today - goal))
    } else if goal_met {
        "goal met".to_string()
    } else {
        format!("{} remaining", format_minutes(goal.saturating_sub(today)))
    };

    let mut chip_spans = vec![Span::raw(" ")];
    chip_spans.extend(chip(
        icons.fire,
        format!("{}d streak", app.data.streak_days),
        theme.warning,
        theme.comment,
    ));
    chip_spans.push(Span::raw("  "));
    chip_spans.extend(chip(
        icons.target,
        status_str,
        if goal_met {
            theme.success
        } else {
            theme.accent
        },
        theme.comment,
    ));
    chip_spans.push(Span::raw("  "));
    chip_spans.extend(chip(
        icons.tasks,
        format!("{} open", app.pending_task_count()),
        theme.info,
        theme.comment,
    ));
    chip_spans.push(Span::raw("  "));
    chip_spans.extend(chip(
        icons.chart,
        format!("{} all-time", format_minutes(app.data.total_focus_minutes)),
        theme.dim,
        theme.comment,
    ));

    let lines = vec![Line::from(spans), Line::from(chip_spans)];
    f.render_widget(Paragraph::new(lines), inner);
}

fn priority_glyph(p: Priority) -> &'static str {
    match p {
        Priority::High => "★",
        Priority::Medium => "◆",
        Priority::Low => "·",
    }
}

fn priority_color(theme: &Theme, p: Priority) -> Color {
    match p {
        Priority::High => theme.error,
        Priority::Medium => theme.warning,
        Priority::Low => theme.comment,
    }
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
    let usable_w = (area.width as usize).saturating_sub(2);
    let lead_w = 9;
    let sub_w = 7;
    let time_w = 10;
    let title_w = usable_w.saturating_sub(lead_w + sub_w + time_w).max(8);

    let rows: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(idx, &task_i)| {
            let t = &app.data.tasks[task_i];
            let selected = idx == selected_idx;
            let is_active = app.task_ui.active_task == Some(t.id);

            let row_style = if selected {
                Style::default()
                    .bg(theme.select_bg)
                    .fg(theme.select_fg)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            let inherit = selected.then_some(row_style);

            let lead = if is_active {
                format!("{} ", icons.task_active)
            } else if selected {
                format!("{} ", icons.chevron)
            } else {
                "  ".into()
            };

            let mut spans = vec![Span::styled(
                lead,
                inherit.unwrap_or(Style::default().fg(theme.accent)),
            )];
            spans.extend(status_checkbox(theme, icons, t.status, inherit));
            spans.push(Span::raw(" "));

            // Compact priority glyph
            spans.push(Span::styled(
                format!("{} ", priority_glyph(t.priority)),
                inherit.unwrap_or(
                    Style::default()
                        .fg(priority_color(theme, t.priority))
                        .add_modifier(Modifier::BOLD),
                ),
            ));

            let title = truncate(&t.title, title_w);
            let pad = title_w.saturating_sub(unicode_width::UnicodeWidthStr::width(title.as_str()));
            spans.push(Span::styled(
                format!("{title}{}", " ".repeat(pad)),
                row_style,
            ));

            // Subtask badge
            if let Some((d, n)) = t.subtask_progress() {
                let sub_style = if d == n && n > 0 {
                    inherit.unwrap_or(Style::default().fg(theme.success))
                } else {
                    inherit.unwrap_or(Style::default().fg(theme.comment))
                };
                let s = format!("({d}/{n})");
                spans.push(Span::styled(format!("{:>7}", s), sub_style));
            } else {
                spans.push(Span::raw(" ".repeat(sub_w)));
            }

            // Minutes
            let mins_str = format!("{}/{}m", t.actual_minutes, t.estimated_minutes);
            spans.push(Span::styled(
                format!(" {:>9}", mins_str),
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

    let (state_label, state_color) = match t.state {
        crate::model::TimerState::Idle => ("ready", theme.dim),
        crate::model::TimerState::Running => ("focusing", mc),
        crate::model::TimerState::Paused => ("paused", theme.warning),
        crate::model::TimerState::Finished => ("complete", theme.success),
    };

    let mode_icon = match t.mode {
        TimerMode::Focus => app.icons.timer,
        TimerMode::ShortBreak => app.icons.heart,
        TimerMode::LongBreak => app.icons.zen,
        TimerMode::Custom => app.icons.focus,
    };

    let outer = timer_panel(
        theme,
        Line::from(vec![
            Span::styled(
                format!(" {mode_icon} "),
                Style::default().fg(if is_finished { theme.success } else { mc }),
            ),
            Span::styled(
                format!("{}{}", t.mode.label().to_lowercase(), title_suffix),
                Style::default()
                    .fg(if is_finished { theme.success } else { mc })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
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
    let (main_time, tenths, _) = format_time_stack(t);
    let today_logged = app.today_focus_mins();

    if app.data.canvas_mode == crate::model::CanvasMode::Off {
        let minimal_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(inner);

        let time_line = Line::from(vec![
            Span::styled(
                format!("{main_time} "),
                Style::default()
                    .fg(if is_finished {
                        theme.success
                    } else {
                        theme.text
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(tenths, Style::default().fg(theme.dim)),
        ]);
        f.render_widget(
            Paragraph::new(time_line).alignment(Alignment::Center),
            minimal_layout[1],
        );

        let mut sub_spans = vec![Span::styled(
            format!(
                "{} logged today",
                super::widgets::format_minutes(today_logged)
            ),
            Style::default().fg(theme.comment),
        )];
        if t.mode == TimerMode::Focus {
            let (quality_label, quality_color) = match t.session_pause_count {
                0 => ("uninterrupted", theme.success),
                1 | 2 => ("minor pauses", theme.warning),
                _ => ("interrupted", theme.error),
            };
            sub_spans.push(Span::styled(" · ", Style::default().fg(theme.comment)));
            sub_spans.push(Span::styled(
                format!("{} {}", app.icons.chart, quality_label),
                Style::default().fg(quality_color),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(sub_spans)).alignment(Alignment::Center),
            minimal_layout[2],
        );

        let progress_ratio = t.progress().clamp(0.0, 1.0);
        let bar_width = (inner.width as usize).saturating_sub(14).max(8);
        let filled = (progress_ratio * bar_width as f64).round() as usize;
        let bar_str: String = (0..bar_width)
            .map(|i| if i < filled { '━' } else { '─' })
            .collect();
        let pct_str = if is_finished {
            "done".to_string()
        } else {
            format!("{:>3}%", (progress_ratio * 100.0) as u32)
        };
        let bar_line = Line::from(vec![
            Span::styled(format!("{pct_str} "), Style::default().fg(theme.dim)),
            Span::styled(
                bar_str,
                Style::default().fg(if is_finished { theme.success } else { mc }),
            ),
            Span::styled(
                format!(" {}m", (t.remaining_seconds() / 60)),
                Style::default().fg(theme.dim),
            ),
        ]);
        f.render_widget(
            Paragraph::new(bar_line).alignment(Alignment::Center),
            minimal_layout[3],
        );

        if on_break {
            crate::canvas_timer::draw_break_tip(
                f,
                minimal_layout[4],
                t,
                mc,
                theme.text,
                theme.dim,
                app.icons.heart,
            );
        } else if let Some(id) = app.task_ui.active_task {
            if let Some(task) = app.data.task(id) {
                let task_line = Line::from(vec![
                    Span::styled(
                        format!("{} ", app.icons.task_active),
                        Style::default().fg(theme.accent),
                    ),
                    Span::styled(&task.title, Style::default().fg(theme.text)),
                ]);
                f.render_widget(
                    Paragraph::new(task_line).alignment(Alignment::Center),
                    minimal_layout[4],
                );
            }
        }

        draw_timer_footer(f, app, &minimal_layout[5..], mc);
        return;
    }

    // The canvas fills what's left, so on short terminals it shrinks before the clock does.
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if on_break {
            [
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(2),
            ]
        } else {
            [
                Constraint::Fill(1),
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
        animated: app.data.canvas_mode == crate::model::CanvasMode::Animated,
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
    if areas.is_empty() {
        return;
    }
    let theme = &app.theme;
    let t = &app.timer;

    let cycle = t.config.long_break_every.max(1);
    let done_in_cycle = t.completed_focus_sessions % cycle;
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

    if areas.len() > 1 {
        if let Some(mut spans) = active_task_spans(app, theme) {
            if let Some(id) = app.task_ui.active_task {
                if let Some(task) = app.data.task(id) {
                    let left =
                        crate::storage::sessions_remaining_hint(task, app.data.focus_minutes);
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
        } else {
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
    }

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
    let text_w = (area.width as usize).saturating_sub(4).max(8);

    // Title with priority glyph
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", priority_glyph(task.priority)),
            Style::default()
                .fg(priority_color(theme, task.priority))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            task.title.clone(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    // Tags
    if !task.tags.is_empty() {
        let mut tag_spans = vec![Span::raw(" ")];
        for tag in &task.tags {
            tag_spans.extend(tag_span(theme, tag));
            tag_spans.push(Span::raw("  "));
        }
        lines.push(Line::from(tag_spans));
    }

    // State Chips
    let mut chips: Vec<Span> = Vec::new();
    let push_chip = |chips: &mut Vec<Span>, spans: Vec<Span<'static>>| {
        if !chips.is_empty() {
            chips.push(Span::raw(" "));
        }
        chips.extend(spans);
    };
    push_chip(
        &mut chips,
        chip(
            "",
            task.status.label().to_lowercase(),
            task_status_color(theme, task.status),
            theme.comment,
        ),
    );
    push_chip(
        &mut chips,
        chip(
            "",
            task.priority.label().to_lowercase(),
            priority_color(theme, task.priority),
            theme.comment,
        ),
    );
    if task.today {
        push_chip(
            &mut chips,
            chip("", "today".into(), theme.accent, theme.comment),
        );
    }
    if app.task_ui.active_task == Some(task.id) {
        push_chip(
            &mut chips,
            chip("", "focusing".into(), theme.accent, theme.comment),
        );
    }
    lines.push(Line::from(""));
    lines.push(Line::from(chips));

    // Focus Time Progress Bar
    let ratio = task.progress_ratio();
    let pct = format!(" {}%", (ratio * 100.0) as u32);
    let bar_w = text_w.saturating_sub(pct.chars().count() + 2).clamp(4, 28);
    let fill = if ratio >= 1.0 {
        theme.success
    } else {
        theme.accent
    };
    let mut bar = vec![Span::raw(" ")];
    bar.extend(text_gauge(theme, ratio, bar_w, fill));
    bar.push(Span::styled(
        pct,
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
    ));
    bar.push(Span::styled(
        format!(
            " ({}/{})",
            format_minutes(task.actual_minutes),
            format_minutes(task.estimated_minutes)
        ),
        Style::default().fg(theme.comment),
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(bar));

    // Subtasks Checklist (clean format)
    if !task.subtasks.is_empty() {
        lines.push(Line::from(""));
        let (done, total) = task.subtask_progress().unwrap_or((0, 0));
        let sub_pct = (done * 100).checked_div(total).unwrap_or(0);
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} subtasks ", icons.tasks),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({done}/{total} · {sub_pct}%)"),
                Style::default().fg(theme.comment),
            ),
        ]));

        for sub in task.subtasks.iter().take(5) {
            let status = if sub.done {
                TaskStatus::Done
            } else {
                TaskStatus::Pending
            };
            let style = if sub.done {
                Style::default().fg(theme.dim)
            } else {
                Style::default().fg(theme.text)
            };
            let mut spans = vec![Span::raw("  ")];
            spans.extend(status_checkbox(theme, icons, status, None));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                truncate(&sub.title, text_w.saturating_sub(8)),
                style,
            ));
            lines.push(Line::from(spans));
        }
        if task.subtasks.len() > 5 {
            lines.push(comment_line(
                theme,
                format!("  +{} more in Tasks tab [2]", task.subtasks.len() - 5),
            ));
        }
    }

    // Notes (if any)
    if !task.notes.is_empty() {
        lines.push(Line::from(""));
        for para in task.notes.lines().take(3) {
            lines.push(comment_line(theme, para.to_string()));
        }
    }

    let block = dense_panel(theme, section_title(theme, icons.about, "Details"));
    f.render_widget(Paragraph::new(lines).block(block), area);
}

use super::*;
use crate::model::Priority;
use crate::ui::widgets::bracket_toggle;

// Row column widths. Fixed so titles, counts, minutes and tags line up down the list
// instead of drifting with the length of whatever is to their left.
const ACTIVE_W: usize = 2; // active marker + gap
const FLAG_W: usize = 2; // overdue + today
const CHECK_W: usize = 3; // [x]
const PRIO_W: usize = 2; // "★ " — compact 1-char symbol + space
const BULK_W: usize = 2; // selection circle + gap, only in bulk mode
const SUB_W: usize = 7; // "(9/9) "
const TIME_W: usize = 10; // " 999/999m"
const HIGHLIGHT_W: usize = 2; // "▸ ", which the List takes out of the item area
const MIN_TITLE_W: usize = 12;

/// Widest tag column we will ever give up.
const TAG_W_MAX: usize = 26;
/// Below this a tag column shows nothing useful — just a bare `+2` — so it is dropped.
const TAG_W_MIN: usize = 7;
/// Title width to protect before handing space to any other column.
const COMFY_TITLE_W: usize = 22;

pub(crate) fn draw_tasks(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(52),
            Constraint::Length(1),
            Constraint::Percentage(48),
        ])
        .split(area);

    draw_task_list(f, app, rows[0]);
    draw_divider(f, rows[1], &app.theme);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(3),
            Constraint::Percentage(50),
        ])
        .split(rows[2]);

    draw_subtasks_panel(f, app, bottom_cols[0]);
    vertical_rule(f, &app.theme, bottom_cols[1]);
    draw_task_details(f, app, bottom_cols[2]);

    if app.task_ui.searching {
        draw_search_popup(f, app, area);
    }
}

fn draw_divider(f: &mut Frame, area: Rect, theme: &Theme) {
    let border = Style::default().fg(theme.panel_border);
    f.render_widget(
        Paragraph::new(Span::styled("─".repeat(area.width as usize), border)),
        area,
    );
}

// ── task list ────────────────────────────────────────────────────────────────

/// Column widths for one task row, given the width the list has to render into.
struct RowLayout {
    title: usize,
    tags: usize,
    show_subtask_count: bool,
    show_minutes: bool,
}

impl RowLayout {
    fn build(width: usize, bulk: bool, tags_want: usize) -> Self {
        let usable = width.saturating_sub(HIGHLIGHT_W);
        let lead = ACTIVE_W + FLAG_W + CHECK_W + 1 + PRIO_W + if bulk { BULK_W } else { 0 };
        let mut avail = usable.saturating_sub(lead);

        let show_minutes = avail >= COMFY_TITLE_W + TIME_W;
        if show_minutes {
            avail -= TIME_W;
        }
        let show_subtask_count = avail >= COMFY_TITLE_W + SUB_W;
        if show_subtask_count {
            avail -= SUB_W;
        }
        let mut tags = tags_want
            .min(TAG_W_MAX)
            .min(avail.saturating_sub(COMFY_TITLE_W));
        if tags < TAG_W_MIN {
            tags = 0;
        }
        avail -= tags;

        Self {
            title: avail.max(MIN_TITLE_W),
            tags,
            show_subtask_count,
            show_minutes,
        }
    }
}

/// Display width of a task's full tag run, `#a #b #c`.
fn tag_run_width(tags: &[String]) -> usize {
    if tags.is_empty() {
        return 0;
    }
    tags.iter()
        .map(|t| unicode_width::UnicodeWidthStr::width(t.as_str()) + 1)
        .sum::<usize>()
        + tags.len()
}

/// Interactive filter tabs in the task list header: `[g] [open] [today] [done] [all] [archive]`
fn tasks_filter_tabs(theme: &Theme, current: TaskFilter, width: u16) -> Line<'static> {
    if width < 54 {
        return Line::from(vec![
            Span::styled("[g] ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("[{}]", current.label().to_lowercase()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    let mut spans = Vec::with_capacity(16);
    spans.push(Span::styled("[g] ", Style::default().fg(theme.dim)));
    for filter in [
        TaskFilter::Pending,
        TaskFilter::Today,
        TaskFilter::Done,
        TaskFilter::All,
        TaskFilter::Archived,
    ] {
        spans.extend(bracket_toggle(
            theme,
            filter.label().to_lowercase().as_str(),
            filter == current,
        ));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

fn draw_task_list(f: &mut Frame, app: &mut App, area: Rect) {
    let icons = app.icons;
    let theme = &app.theme;
    let frame_today = app.frame_today();
    let indices = &app.task_ui.cached_filtered_tasks;
    let filtered_count = indices.len();
    let selected_idx = app.task_ui.task_state.selected();
    let tags_want = indices
        .iter()
        .map(|&i| tag_run_width(&app.data.tasks[i].tags))
        .max()
        .unwrap_or(0);
    let cols = RowLayout::build(area.width as usize, app.task_ui.bulk_mode, tags_want);

    let items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(list_idx, &idx)| {
            let task = &app.data.tasks[idx];
            let is_cursor = selected_idx == Some(list_idx);
            let line = task_row(app, task, idx, is_cursor, frame_today, &cols);
            ListItem::new(vec![line])
        })
        .collect();

    let title_color = if app.task_ui.bulk_mode {
        theme.info
    } else {
        theme.accent
    };

    let title_spans = vec![Span::styled(
        format!(
            " {} tasks ({}){} ",
            icons.tasks,
            filtered_count,
            if app.task_ui.bulk_mode {
                " · bulk"
            } else {
                ""
            }
        ),
        Style::default()
            .fg(title_color)
            .add_modifier(Modifier::BOLD),
    )];

    let block = dense_panel(theme, Line::from(title_spans)).title(
        tasks_filter_tabs(theme, app.task_ui.task_filter, area.width).alignment(Alignment::Right),
    );

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.select_bg)
                .fg(theme.select_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");
    f.render_stateful_widget(list, area, &mut app.task_ui.task_state);
}

fn priority_glyph(p: Priority) -> &'static str {
    match p {
        Priority::High => "★",
        Priority::Medium => "◆",
        Priority::Low => "·",
    }
}

fn task_row<'a>(
    app: &App,
    task: &crate::model::Task,
    task_idx: usize,
    is_cursor: bool,
    frame_today: &str,
    cols: &RowLayout,
) -> Line<'a> {
    let theme = &app.theme;
    let icons = app.icons;
    let is_active = app.task_ui.active_task == Some(task.id);
    let bulk_selected = app.task_ui.bulk_mode && app.task_ui.bulk_selected.contains(&task.id);
    let overdue = task.is_overdue_on(frame_today);

    let style = if bulk_selected {
        Style::default().fg(theme.info).add_modifier(Modifier::BOLD)
    } else if is_active && !is_cursor {
        Style::default()
            .bg(theme.active_bg)
            .fg(theme.active_fg)
            .add_modifier(Modifier::BOLD)
    } else if overdue && !is_cursor {
        Style::default().fg(theme.error)
    } else {
        Style::default().fg(theme.text)
    };

    let mark_style = if is_active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        style
    };

    let mut spans = vec![Span::styled(
        format!("{} ", if is_active { icons.task_active } else { " " }),
        mark_style,
    )];
    if app.task_ui.bulk_mode {
        spans.push(if bulk_selected {
            Span::styled(
                icons.check,
                Style::default().fg(theme.info).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("○", Style::default().fg(theme.dim))
        });
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(
        format!(
            "{}{}",
            if overdue { icons.alert } else { " " },
            if task.today { icons.star } else { " " }
        ),
        mark_style,
    ));

    // A `[x]` checkbox instead of a bare status glyph — reads as a clean checklist.
    spans.extend(status_checkbox(
        theme,
        icons,
        task.status,
        (is_active || bulk_selected).then_some(mark_style),
    ));
    spans.push(Span::raw(" "));

    // Compact priority glyph
    spans.push(Span::styled(
        format!("{} ", priority_glyph(task.priority)),
        Style::default()
            .fg(priority_color(theme, task.priority))
            .add_modifier(Modifier::BOLD),
    ));

    let reorder = if app.task_ui.reordering_task == Some(task.id) {
        "↕ "
    } else if app.is_task_blocked_at(task_idx) {
        "! "
    } else {
        ""
    };
    let title = truncate(&format!("{reorder}{}", task.title), cols.title);
    spans.push(Span::styled(
        pad_to(&title, cols.title),
        if reorder.is_empty() {
            style
        } else {
            Style::default().fg(theme.warning)
        },
    ));

    if cols.show_subtask_count {
        if let Some((d, n)) = task.subtask_progress() {
            let sub_style = if d == n && n > 0 {
                Style::default().fg(theme.success)
            } else {
                Style::default().fg(theme.comment)
            };
            let s = format!("({d}/{n})");
            spans.push(Span::styled(format!("{:>SUB_W$}", s), sub_style));
        } else {
            spans.push(Span::raw(" ".repeat(SUB_W)));
        }
    }

    if cols.show_minutes {
        let mins_str = format!("{}/{}m", task.actual_minutes, task.estimated_minutes);
        spans.push(Span::styled(
            format!(" {:>TIME_W$}", mins_str),
            Style::default().fg(theme.comment),
        ));
    }
    if cols.tags > 0 {
        spans.push(Span::raw(" "));
        let (tag_spans, used_w) = fit_tags(theme, &task.tags, cols.tags.saturating_sub(1));
        spans.extend(tag_spans);
        if used_w < cols.tags.saturating_sub(1) {
            spans.push(Span::raw(" ".repeat(cols.tags.saturating_sub(1) - used_w)));
        }
    }
    Line::from(spans)
}

/// Tags that fit the budget whole, plus a `+N` for the rest.
fn fit_tags<'a>(theme: &Theme, tags: &[String], budget: usize) -> (Vec<Span<'a>>, usize) {
    if tags.is_empty() {
        return (Vec::new(), 0);
    }
    let mut spans = Vec::new();
    let mut used = 0usize;
    let mut shown = 0usize;

    for tag in tags {
        let w = unicode_width::UnicodeWidthStr::width(tag.as_str()) + 1; // the '#'
        let sep = usize::from(shown > 0);
        // Keep room for a "+N" if this is not the last tag.
        let reserve = if shown + 1 < tags.len() { 3 } else { 0 };
        if used + sep + w + reserve > budget {
            break;
        }
        if sep > 0 {
            spans.push(Span::raw(" "));
            used += 1;
        }
        spans.extend(tag_span(theme, tag));
        used += w;
        shown += 1;
    }

    if shown < tags.len() {
        let rest = tags.len() - shown;
        if shown > 0 {
            spans.push(Span::raw(" "));
            used += 1;
        }
        let rest_str = format!("+{rest}");
        used += rest_str.chars().count();
        spans.push(Span::styled(rest_str, Style::default().fg(theme.comment)));
    }
    (spans, used)
}

// ── dedicated subtasks panel ─────────────────────────────────────────────────

fn draw_subtasks_panel(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let focused = app.task_ui.subtask_focus;

    let indices = &app.task_ui.cached_filtered_tasks;
    if indices.is_empty() {
        let block = dense_panel(theme, section_title(theme, icons.tasks, "Subtasks"));
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                comment_line(theme, "no task selected"),
            ]),
            inner,
        );
        return;
    }

    let sel = app
        .task_ui
        .task_state
        .selected()
        .unwrap_or(0)
        .min(indices.len() - 1);
    let task_idx = indices[sel];
    let t = &app.data.tasks[task_idx];

    let (done, total) = t.subtask_progress().unwrap_or((0, 0));
    let sub_pct = (done * 100).checked_div(total).unwrap_or(0);

    let count_label = if total > 0 {
        format!("({done}/{total} · {sub_pct}%) ")
    } else {
        "(0) ".into()
    };

    let title_style = if focused {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.panel_border)
    };

    let mut right_spans = vec![Span::styled(
        count_label,
        Style::default().fg(if focused { theme.accent } else { theme.comment }),
    )];
    if focused {
        right_spans.push(Span::styled(
            "[FOCUS]",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ));
        right_spans.push(Span::raw(" "));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(title_style)
        .title(section_title(theme, icons.tasks, "Subtasks"))
        .title(Line::from(right_spans).alignment(Alignment::Right));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_w = inner.width as usize;
    let mut lines = Vec::new();

    if total > 0 {
        lines.push(Line::from(""));
        const SUB_BAR_MAX: usize = 32;
        let bar_w = inner_w.saturating_sub(8).clamp(4, SUB_BAR_MAX);
        let ratio = done as f64 / total as f64;
        let fill = if done == total {
            theme.success
        } else {
            theme.accent
        };
        let mut sub_gauge = text_gauge(theme, ratio, bar_w, fill);
        sub_gauge.push(Span::styled(
            format!(" {:>3}%", sub_pct),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(sub_gauge));
    }

    if t.subtasks.is_empty() {
        lines.push(Line::from(""));
        lines.push(comment_line(theme, "no subtasks yet"));
        lines.push(comment_line(theme, "press [c] to add"));
    } else {
        lines.push(Line::from(""));
        let visible_limit = (inner.height as usize).saturating_sub(6).max(3);
        let start = if !focused || total <= visible_limit {
            0
        } else {
            app.task_ui
                .subtask_selected
                .saturating_sub(visible_limit / 2)
                .min(total.saturating_sub(visible_limit))
        };
        let end = (start + visible_limit).min(total);

        if start > 0 {
            lines.push(comment_line(theme, format!("  +{start} above...")));
        }

        for (offset, sub) in t.subtasks[start..end].iter().enumerate() {
            let i = start + offset;
            let on_cursor = focused && app.task_ui.subtask_selected == i;
            let status = if sub.done {
                crate::model::TaskStatus::Done
            } else {
                crate::model::TaskStatus::Pending
            };

            let marker = if on_cursor { "▸ " } else { "  " };
            let marker_style = if on_cursor {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.dim)
            };

            let mut spans = vec![Span::styled(marker, marker_style)];
            spans.extend(status_checkbox(theme, icons, status, None));
            spans.push(Span::raw(" "));

            let title_style = if on_cursor {
                Style::default()
                    .fg(theme.select_fg)
                    .bg(theme.select_bg)
                    .add_modifier(Modifier::BOLD)
            } else if sub.done {
                Style::default().fg(theme.dim)
            } else {
                Style::default().fg(theme.text)
            };

            let body_w = inner_w.saturating_sub(8);
            spans.push(Span::styled(truncate(&sub.title, body_w), title_style));
            lines.push(Line::from(spans));
        }

        if end < total {
            let rest = total - end;
            lines.push(comment_line(theme, format!("  +{rest} below...")));
        }

        if inner.height >= 8 {
            lines.push(Line::from(""));
            let hint = if focused {
                "[j/k] nav · [x] check · [c] add · [-] del · [Tab] done"
            } else {
                "[Tab] focus subtasks · [c] add"
            };
            lines.push(comment_line(theme, hint));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── details column ───────────────────────────────────────────────────────────

fn draw_task_details(f: &mut Frame, app: &App, area: Rect) {
    let block = dense_panel(
        &app.theme,
        section_title(&app.theme, app.icons.about, "Details"),
    );
    let inner_w = block.inner(area).width;
    f.render_widget(
        Paragraph::new(build_task_detail(app, inner_w))
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn build_task_detail(app: &App, width: u16) -> Vec<Line<'_>> {
    let theme = &app.theme;
    let indices = &app.task_ui.cached_filtered_tasks;
    if indices.is_empty() {
        let msg = match app.task_ui.task_filter {
            TaskFilter::All => "no tasks yet — press [a] to add one",
            TaskFilter::Pending => "all tasks done — nice work",
            TaskFilter::Done => "no completed tasks yet",
            TaskFilter::Today => "nothing queued for today — press [t] to tag tasks",
            TaskFilter::Archived => "no archived tasks",
        };
        return vec![Line::from(""), comment_line(theme, msg)];
    }

    let frame_today = app.frame_today();
    let sel = app
        .task_ui
        .task_state
        .selected()
        .unwrap_or(0)
        .min(indices.len() - 1);
    let task_idx = indices[sel];
    let t = &app.data.tasks[task_idx];
    let mut lines = Vec::new();

    // ── Task Header ──────────────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            format!("{} ", priority_glyph(t.priority)),
            Style::default()
                .fg(priority_color(theme, t.priority))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            t.title.clone(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ]));

    if !t.tags.is_empty() {
        let mut spans = Vec::new();
        for (i, tag) in t.tags.iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.extend(tag_span(theme, tag));
        }
        lines.push(Line::from(spans));
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
            t.status.label().to_lowercase(),
            task_status_color(theme, t.status),
            theme.comment,
        ),
    );
    push_chip(
        &mut chips,
        chip(
            "",
            t.priority.label().to_lowercase(),
            priority_color(theme, t.priority),
            theme.comment,
        ),
    );
    if t.today {
        push_chip(
            &mut chips,
            chip("", "today".into(), theme.accent, theme.comment),
        );
    }
    if t.is_overdue_on(frame_today) {
        push_chip(
            &mut chips,
            chip("", "overdue".into(), theme.error, theme.comment),
        );
    }
    if app.task_ui.active_task == Some(t.id) {
        push_chip(
            &mut chips,
            chip("", "focusing".into(), theme.accent, theme.comment),
        );
    }
    lines.push(Line::from(""));
    lines.push(Line::from(chips));

    // ── Effort & Focus Progress ──────────────────────────────────────────────
    let ratio = t.progress_ratio();
    let pct = format!(" {}%", (ratio * 100.0) as u32);
    const BAR_W_MAX: usize = 36;
    let bar_w = (width as usize)
        .saturating_sub(pct.chars().count())
        .clamp(4, BAR_W_MAX);
    let fill = if ratio >= 1.0 {
        theme.success
    } else {
        theme.accent
    };
    lines.push(Line::from(""));
    let mut bar = text_gauge(theme, ratio, bar_w, fill);
    bar.push(Span::styled(
        pct,
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::from(bar));

    lines.push(Line::from(""));
    lines.push(meta_row(
        theme,
        "estimate",
        format_minutes(t.estimated_minutes),
        theme.text,
    ));
    lines.push(meta_row(
        theme,
        "logged",
        format!(
            "{} in {} session{}",
            format_minutes(t.actual_minutes),
            t.sessions,
            if t.sessions == 1 { "" } else { "s" }
        ),
        theme.success,
    ));
    lines.push(meta_row(
        theme,
        "remaining",
        format!(
            "~{} sessions of {}m",
            crate::storage::sessions_remaining_hint(t, app.data.focus_minutes),
            app.data.focus_minutes
        ),
        theme.info,
    ));

    // ── Dates & Recurrence ───────────────────────────────────────────────────
    lines.push(Line::from(""));
    lines.push(meta_row(
        theme,
        "created",
        t.created_at.format("%Y-%m-%d %H:%M").to_string(),
        theme.text,
    ));
    if let Some(ref due) = t.due_date {
        lines.push(meta_row(
            theme,
            "due",
            due.clone(),
            if t.is_overdue_on(frame_today) {
                theme.error
            } else {
                theme.text
            },
        ));
    }
    if let Some(c) = t.completed_at {
        lines.push(meta_row(
            theme,
            "done",
            c.format("%Y-%m-%d %H:%M").to_string(),
            theme.success,
        ));
    }

    let repeats = t.recurrence != crate::model::TaskRecurrence::None;
    let blocked = !t.blocked_by.is_empty();
    if repeats || blocked {
        lines.push(Line::from(""));
        if repeats {
            lines.push(meta_row(
                theme,
                "repeats",
                t.recurrence.label().to_lowercase(),
                theme.info,
            ));
        }
        if blocked {
            lines.push(meta_row(
                theme,
                "blocked by",
                t.blocked_by
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                if app.is_task_blocked_at(task_idx) {
                    theme.error
                } else {
                    theme.text
                },
            ));
        }
    }

    // ── Notes ────────────────────────────────────────────────────────────────
    if !t.notes.trim().is_empty() {
        lines.push(Line::from(""));
        for para in t.notes.lines() {
            lines.push(comment_line(theme, para.to_string()));
        }
    }

    lines.push(Line::from(""));
    lines.push(comment_line(theme, format!("id {}", t.id)));
    lines
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn priority_color(theme: &Theme, priority: crate::model::Priority) -> Color {
    match priority {
        crate::model::Priority::High => theme.error,
        crate::model::Priority::Medium => theme.warning,
        crate::model::Priority::Low => theme.comment,
    }
}

/// Right-pads to an exact display width, so the columns after it start where they should.
fn pad_to(s: &str, width: usize) -> String {
    let w = unicode_width::UnicodeWidthStr::width(s);
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

fn draw_search_popup(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let search_area = centered_rect(50, 20, area);
    f.render_widget(Clear, search_area);
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "search tasks",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            comment_line(theme, "matches title or tags"),
            Line::from(""),
            Line::from(vec![
                Span::styled("$ ", Style::default().fg(theme.comment)),
                Span::styled(
                    app.task_ui.task_search.clone(),
                    Style::default().fg(theme.text),
                ),
                Span::styled("█", Style::default().fg(theme.accent)),
            ]),
            Line::from(""),
            comment_line(theme, "[enter] confirm · [esc] cancel"),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.accent)),
        ),
        search_area,
    );
}

use super::*;

// Row column widths. Fixed so titles, counts, minutes and tags line up down the list
// instead of drifting with the length of whatever is to their left.
const ACTIVE_W: usize = 2; // active marker + gap
const FLAG_W: usize = 2; // overdue + today
const CHECK_W: usize = 3; // [x]
const PRIO_W: usize = 5; // "high " — "high" is already four characters
const BULK_W: usize = 2; // selection circle + gap, only in bulk mode
const SUB_W: usize = 7; // " (9/9)"
const TIME_W: usize = 10; // " 999/999m"
const HIGHLIGHT_W: usize = 2; // "▸ ", which the List takes out of the item area
const MIN_TITLE_W: usize = 12;

/// Widest tag column we will ever give up.
const TAG_W_MAX: usize = 26;
/// Below this a tag column shows nothing useful — just a bare `+2` — so it is dropped.
const TAG_W_MIN: usize = 7;
/// Title width to protect before handing space to any other column.
const COMFY_TITLE_W: usize = 26;

/// Subtasks shown under the selected task before the rest are summarised.
const INLINE_SUBTASK_MAX: usize = 12;

pub(crate) fn draw_tasks(f: &mut Frame, app: &mut App, area: Rect) {
    // A gutter column between list and details; without it the task rows run straight
    // into the detail panel with no visual separation.
    let outer = Layout::default()
        .direction(Direction::Horizontal)
        .margin(1)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Length(3),
            Constraint::Percentage(40),
        ])
        .split(area);
    let (list_area, detail_area) = (outer[0], outer[2]);
    vertical_rule(f, &app.theme, outer[1]);

    draw_task_list(f, app, list_area);
    draw_task_details(f, app, detail_area);

    if app.task_ui.searching {
        draw_search_popup(f, app, area);
    }
}

// ── task list ────────────────────────────────────────────────────────────────

/// Column widths for one task row, given the width the list has to render into.
struct RowLayout {
    /// Where a subtask line starts, and how much room it then has.
    subtask_indent: usize,
    subtask_body: usize,
    title: usize,
    tags: usize,
    show_subtask_count: bool,
    show_minutes: bool,
}

impl RowLayout {
    /// `tags_want` is the widest tag run in the list — sizing the column to the data keeps
    /// every row aligned without reserving a fixed quarter of the pane for two short tags.
    fn build(width: usize, bulk: bool, tags_want: usize) -> Self {
        let usable = width.saturating_sub(HIGHLIGHT_W);
        let lead = ACTIVE_W + FLAG_W + CHECK_W + 1 + PRIO_W + if bulk { BULK_W } else { 0 };
        let mut avail = usable.saturating_sub(lead);

        // The title is the only column you cannot do without, so every other column has to
        // buy its space and only while a comfortable title still fits. Reserving fixed
        // widths first was leaving twelve columns for the title on a narrow pane.
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

        let subtask_indent = ACTIVE_W + FLAG_W;
        Self {
            subtask_indent,
            // spine, caret, checkbox and the gap after it.
            subtask_body: usable.saturating_sub(subtask_indent + 2 + 2 + CHECK_W + 1),
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

    let mut extra_rows = 0usize;
    let items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(list_idx, &idx)| {
            let task = &app.data.tasks[idx];
            let is_cursor = selected_idx == Some(list_idx);
            let mut lines = vec![task_row(app, task, idx, is_cursor, frame_today, &cols)];
            if is_cursor {
                let sub = subtask_lines(app, task, &cols);
                extra_rows += sub.len();
                lines.extend(sub);
            }
            ListItem::new(lines)
        })
        .collect();

    let filter_label = if app.task_ui.task_search.is_empty() {
        app.task_ui.task_filter.label().to_string()
    } else {
        format!("'{}'", app.task_ui.task_search)
    };

    // The expanded subtasks are part of the selected item, so the list is taller than the
    // task count — counting rows rather than tasks is what keeps "more" honest.
    let rendered_rows = filtered_count + extra_rows;
    let visible = area.height.saturating_sub(1) as usize;
    let at_bottom = selected_idx
        .map(|sel| sel + 1 >= filtered_count)
        .unwrap_or(true);
    let more = if rendered_rows > visible && !at_bottom {
        " ↓ more "
    } else {
        ""
    };

    let title_color = if app.task_ui.bulk_mode {
        theme.info
    } else {
        theme.accent
    };
    let block = dense_panel(
        theme,
        Line::from(vec![
            Span::styled(
                format!(
                    " {} tasks [{}] ({}){} ",
                    icons.tasks,
                    filter_label.to_lowercase(),
                    filtered_count,
                    if app.task_ui.bulk_mode { " · bulk" } else { "" }
                ),
                Style::default().fg(title_color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(more, Style::default().fg(theme.comment)),
        ]),
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
    // Marks keep the row's own styling when it is active or bulk-selected, so the row
    // reads as one band rather than a run of differently coloured glyphs.
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
                Style::default()
                    .fg(theme.info)
                    .add_modifier(Modifier::BOLD),
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
    // A `[x]` checkbox instead of a bare status glyph — reads as a checklist, and the
    // state stays legible when the row is selected or colour is unavailable.
    spans.extend(status_checkbox(
        theme,
        icons,
        task.status,
        (is_active || bulk_selected).then_some(mark_style),
    ));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(
        format!("{:<PRIO_W$}", task.priority.label().to_lowercase()),
        Style::default().fg(priority_color(theme, task.priority)),
    ));

    let reorder = if app.task_ui.reordering_task == Some(task.id) {
        "↕ "
    } else if app.is_task_blocked_at(task_idx) {
        "! "
    } else {
        ""
    };
    let title = truncate(
        &format!("{reorder}{}", task.title),
        cols.title,
    );
    spans.push(Span::styled(
        pad_to(&title, cols.title),
        if reorder.is_empty() {
            style
        } else {
            Style::default().fg(theme.warning)
        },
    ));

    if cols.show_subtask_count {
        let text = task
            .subtask_progress()
            .map(|(d, n)| format!("({d}/{n})"))
            .unwrap_or_default();
        spans.push(Span::styled(
            format!("{:>SUB_W$}", text),
            Style::default().fg(theme.comment),
        ));
    }
    if cols.show_minutes {
        spans.push(Span::styled(
            format!(
                "{:>TIME_W$}",
                format!("{}/{}m", task.actual_minutes, task.estimated_minutes)
            ),
            Style::default().fg(theme.comment),
        ));
    }
    if cols.tags > 0 && !task.tags.is_empty() {
        spans.push(Span::raw(" "));
        spans.extend(fit_tags(theme, &task.tags, cols.tags.saturating_sub(1)));
    }
    Line::from(spans)
}

/// Tags that fit the budget whole, plus a `+N` for the rest.
///
/// The previous version truncated the joined string, which cut through a tag name and left
/// rows reading `#acc #fres` — worse than not showing the tag at all, because a clipped tag
/// looks like a different tag.
fn fit_tags<'a>(theme: &Theme, tags: &[String], budget: usize) -> Vec<Span<'a>> {
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
        }
        spans.extend(tag_span(theme, tag));
        used += sep + w;
        shown += 1;
    }

    if shown < tags.len() {
        let rest = tags.len() - shown;
        if shown > 0 {
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(
            format!("+{rest}"),
            Style::default().fg(theme.comment),
        ));
    }
    spans
}

/// Subtasks of the selected task, indented under it on a spine.
///
/// They used to live in a panel under the details column, which meant they were invisible
/// until you selected a task that had some, and shared 45% of the narrower column with
/// nothing to anchor them to. Inline they sit against the task they belong to, and the list
/// pane has the vertical room to spare.
fn subtask_lines<'a>(app: &App, task: &crate::model::Task, cols: &RowLayout) -> Vec<Line<'a>> {
    let theme = &app.theme;
    let focused = app.task_ui.subtask_focus;
    let spine_color = if focused { theme.accent } else { theme.panel_border };
    let indent = " ".repeat(cols.subtask_indent);
    let spine = |extra: &str| {
        vec![
            Span::raw(indent.clone()),
            Span::styled("│ ", Style::default().fg(spine_color)),
            Span::raw(extra.to_string()),
        ]
    };
    let body_w = cols.subtask_body;

    if task.subtasks.is_empty() {
        return vec![Line::from(
            [
                spine(""),
                vec![comment_span(theme, "no subtasks · [c] to add")],
            ]
            .concat(),
        )];
    }

    // Window around the cursor rather than always the first N. `subtask_selected` can be
    // any index, so a flat `take(N)` let the cursor sit on a row that was never drawn —
    // nothing looked selected and [x] toggled something invisible.
    let total = task.subtasks.len();
    let start = if !focused || total <= INLINE_SUBTASK_MAX {
        0
    } else {
        app.task_ui
            .subtask_selected
            .saturating_sub(INLINE_SUBTASK_MAX / 2)
            .min(total - INLINE_SUBTASK_MAX)
    };
    let end = (start + INLINE_SUBTASK_MAX).min(total);

    let mut lines: Vec<Line> = Vec::new();
    if start > 0 {
        lines.push(Line::from(
            [
                spine("  "),
                vec![comment_span(theme, format!("+{start} above"))],
            ]
            .concat(),
        ));
    }
    lines.extend(task.subtasks[start..end]
        .iter()
        .enumerate()
        .map(|(offset, sub)| {
            let i = start + offset;
            let on_cursor = focused && app.task_ui.subtask_selected == i;
            let status = if sub.done {
                crate::model::TaskStatus::Done
            } else {
                crate::model::TaskStatus::Pending
            };
            let mut spans = spine(if on_cursor { "▸ " } else { "  " });
            spans.extend(status_checkbox(theme, app.icons, status, None));
            spans.push(Span::raw(" "));
            let mut title_style = super::widgets::subtask_line_style(theme, sub.done);
            if on_cursor {
                title_style = title_style
                    .fg(theme.select_fg)
                    .add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(truncate(&sub.title, body_w), title_style));
            Line::from(spans)
        }));

    if end < total {
        let rest = total - end;
        lines.push(Line::from(
            [
                spine("  "),
                vec![comment_span(theme, format!("+{rest} below"))],
            ]
            .concat(),
        ));
    }

    let (done, total) = task.subtask_progress().unwrap_or((0, 0));
    let hint = if focused {
        format!("{done}/{total} done · [x] toggle · [e] edit · [c] add · [Tab] back")
    } else {
        format!("{done}/{total} done · [Tab] to edit them")
    };
    lines.push(Line::from(
        [spine("  "), vec![comment_span(theme, hint)]].concat(),
    ));
    lines
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

    // Identity first: title, then the tags that qualify it, then the state chips. These
    // are what the task *is*; the numbers below are what has happened to it.
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        t.title.clone(),
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
    )));
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

    // Notes were not shown anywhere on this page. Somewhere to write what a task actually
    // involves is most of what the notes field is for.
    if !t.notes.trim().is_empty() {
        lines.push(Line::from(""));
        for para in t.notes.lines() {
            lines.push(comment_line(theme, para.to_string()));
        }
    }

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

    // Progress. Labelled, because an unlabelled bar hanging at the top of the panel reads
    // as decoration rather than as this task's completion.
    let ratio = t.progress_ratio();
    let pct = format!(" {}%", (ratio * 100.0) as u32);
    // Capped: stretched across a wide column the bar stops reading as a measurement and
    // starts reading as a coloured band across the panel.
    const BAR_W_MAX: usize = 40;
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

    // Effort, then dates, then relations — blank lines between so the panel is three short
    // blocks instead of one wall of colons.
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
    if let Some((done, total)) = t.subtask_progress() {
        lines.push(meta_row(
            theme,
            "subtasks",
            format!("{done} of {total} done"),
            if done == total {
                theme.success
            } else {
                theme.text
            },
        ));
    }

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
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
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

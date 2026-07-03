use super::*;

fn popup_size(popup: &crate::app::Popup) -> (u16, u16) {
    match popup {
        crate::app::Popup::AddSubtask(_) => (48, 24),
        crate::app::Popup::ConfirmDelete(_)
        | crate::app::Popup::BulkConfirm(_)
        | crate::app::Popup::EmptyQueueChoice => (50, 28),
        _ => (68, 78),
    }
}

fn popup_min_size(popup: &crate::app::Popup) -> (u16, u16) {
    match popup {
        crate::app::Popup::AddSubtask(_) => (36, 10),
        _ => (32, 10),
    }
}

fn popup_rect(popup: &crate::app::Popup, area: Rect) -> Rect {
    let (pw, ph) = popup_size(popup);
    let (min_w, min_h) = popup_min_size(popup);
    let mut r = centered_rect(pw, ph, area);
    r.width = r.width.max(min_w).min(area.width);
    r.height = r.height.max(min_h).min(area.height);
    r
}

fn rect_ok(area: Rect) -> bool {
    area.width > 0 && area.height > 0
}

pub(crate) fn draw_popup(f: &mut Frame, app: &mut App) {
    let Some(popup) = app.popup.clone() else {
        return;
    };
    let icons = app.icons;
    let area = f.area();
    let popup_area = popup_rect(&popup, area);
    f.render_widget(Clear, popup_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            match &popup {
                crate::app::Popup::AddTask => format!(" {} Add Task ", icons.plus),
                crate::app::Popup::EditTask(_) => format!(" {} Edit Task ", icons.edit),
                crate::app::Popup::ConfirmDelete(_) => format!(" {} Confirm Delete ", icons.delete),
                crate::app::Popup::EmptyQueueChoice => format!(" {} All Tasks Done ", icons.check),
                crate::app::Popup::AddSubtask(_) => format!(" {} Add Subtask ", icons.plus),
                crate::app::Popup::BulkConfirm(_) => format!(" {} Bulk Action ", icons.tasks),
            },
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    let body = block.inner(popup_area);
    f.render_widget(block, popup_area);

    match &popup {
        crate::app::Popup::AddTask | crate::app::Popup::EditTask(_) => {
            let theme = &app.theme;
            let chunks = popup_body_layout(body, PopupLayout::Form);
            if chunks.is_empty() {
                return;
            }
            let form_area = chunks[0];
            let (left_area, right_area) = if matches!(app.input_field, InputField::DueDate) {
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(form_area);
                (cols[0], Some(cols[1]))
            } else {
                (form_area, None)
            };

            let cursor = |active: bool, text: &str| -> String {
                if active {
                    if text.is_empty() {
                        "|".to_string()
                    } else {
                        format!("{}|", text)
                    }
                } else if text.is_empty() {
                    "—".to_string()
                } else {
                    text.to_string()
                }
            };
            let due_display = if matches!(app.input_field, InputField::DueDate) {
                cursor(true, &app.input_due_date)
            } else if app.input_due_date.is_empty() {
                "—".to_string()
            } else {
                app.input_due_date.clone()
            };
            let tags_display = cursor(matches!(app.input_field, InputField::Tags), &app.input_tags);
            let value_max = left_area.width.saturating_sub(22) as usize;
            let p = Paragraph::new(vec![
                popup_field_line(
                    theme,
                    "Title",
                    cursor(
                        matches!(app.input_field, InputField::Title),
                        &truncate_field(&app.input_buffer, value_max),
                    ),
                    matches!(app.input_field, InputField::Title),
                    value_max,
                ),
                popup_field_line(
                    theme,
                    "Estimate (min)",
                    if matches!(app.input_field, InputField::Estimate) {
                        format!("{}|", app.input_number)
                    } else {
                        app.input_number.to_string()
                    },
                    matches!(app.input_field, InputField::Estimate),
                    value_max,
                ),
                popup_field_line(
                    theme,
                    "Priority",
                    app.input_priority.label().to_string(),
                    matches!(app.input_field, InputField::Priority),
                    value_max,
                ),
                popup_field_line(
                    theme,
                    "Due (YYYY-MM-DD)",
                    truncate_field(&due_display, value_max),
                    matches!(app.input_field, InputField::DueDate),
                    value_max,
                ),
                popup_field_line(
                    theme,
                    "Tags (comma-sep)",
                    truncate_field(&tags_display, value_max),
                    matches!(app.input_field, InputField::Tags),
                    value_max,
                ),
            ]);
            f.render_widget(p, left_area);

            if let Some(r) = right_area {
                let d = app.calendar_date;
                if let Ok(time_date) = time::Date::from_calendar_date(
                    d.year(),
                    time::Month::try_from(d.month() as u8).unwrap_or(time::Month::January),
                    d.day() as u8,
                ) {
                    let mut store = ratatui::widgets::calendar::CalendarEventStore::default();
                    store.add(
                        time_date,
                        Style::default()
                            .bg(theme.accent)
                            .fg(theme.on_accent)
                            .add_modifier(Modifier::BOLD),
                    );

                    let monthly = ratatui::widgets::calendar::Monthly::new(time_date, store)
                        .show_month_header(
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::BOLD),
                        )
                        .show_weekdays_header(Style::default().fg(theme.dim));
                    f.render_widget(monthly, r);
                }
            }
            let hint = if matches!(app.input_field, InputField::DueDate) {
                "←→ day · ↑↓ week · Tab field · Enter save · Esc cancel"
            } else {
                "Tab field · Enter save · Esc cancel"
            };
            if chunks.len() > 1 {
                draw_popup_hint(f, chunks[1], theme, hint);
            }
        }
        crate::app::Popup::ConfirmDelete(id) => {
            let theme = &app.theme;
            let chunks = popup_body_layout(body, PopupLayout::Message);
            if chunks.is_empty() {
                return;
            }
            let title = app
                .data
                .task(*id)
                .map(|t| t.title.as_str())
                .unwrap_or("Unknown task");
            let p = Paragraph::new(vec![
                Line::from(Span::styled(
                    format!("Delete \"{}\"?", title),
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Press y or Enter to confirm, n or Esc to cancel.",
                    Style::default().fg(theme.dim),
                )),
                Line::from(Span::styled(
                    "This cannot be undone.",
                    Style::default().fg(theme.error),
                )),
            ]);
            f.render_widget(p, chunks[0]);
        }
        crate::app::Popup::EmptyQueueChoice => {
            let theme = &app.theme;
            let chunks = popup_body_layout(body, PopupLayout::Message);
            if chunks.is_empty() {
                return;
            }
            let p = Paragraph::new(vec![
                Line::from(Span::styled(
                    "You've completed every task in your queue.",
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "[Enter]  Continue free focus (log general sessions)",
                    Style::default().fg(theme.success),
                )),
                Line::from(Span::styled(
                    "[p]      Pause the timer",
                    Style::default().fg(theme.warning),
                )),
                Line::from(Span::styled(
                    "[a]      Add another task",
                    Style::default().fg(theme.accent),
                )),
                Line::from(Span::styled(
                    "[Esc]    Dismiss",
                    Style::default().fg(theme.dim),
                )),
            ]);
            f.render_widget(p, chunks[0]);
        }
        crate::app::Popup::AddSubtask(id) => {
            draw_add_subtask_popup(f, app, body, *id);
        }
        crate::app::Popup::BulkConfirm(action) => {
            let theme = &app.theme;
            let chunks = popup_body_layout(body, PopupLayout::Message);
            if chunks.is_empty() {
                return;
            }
            let (title, detail, accent) = match action {
                crate::app::BulkAction::MarkDone => (
                    "Mark selected tasks as done?",
                    format!("{} task(s) selected.", app.bulk_selected.len()),
                    theme.success,
                ),
                crate::app::BulkAction::Delete => (
                    "Delete selected tasks?",
                    format!(
                        "{} task(s) will be removed permanently.",
                        app.bulk_selected.len()
                    ),
                    theme.error,
                ),
            };
            let p = Paragraph::new(vec![
                Line::from(Span::styled(
                    title,
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(detail, Style::default().fg(theme.text))),
                Line::from(""),
                Line::from(Span::styled(
                    "[y] confirm  [n/Esc] cancel",
                    Style::default().fg(theme.dim),
                )),
            ]);
            f.render_widget(p, chunks[0]);
        }
    }
}

enum PopupLayout {
    Form,
    Subtask,
    Message,
}

fn popup_body_layout(body: Rect, kind: PopupLayout) -> Vec<Rect> {
    if !rect_ok(body) {
        return vec![];
    }
    let margin = u16::from(body.height >= 8 && body.width >= 8);
    let constraints = match kind {
        PopupLayout::Form => vec![Constraint::Min(4), Constraint::Length(1)],
        PopupLayout::Subtask => vec![
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ],
        PopupLayout::Message => vec![Constraint::Min(1)],
    };
    Layout::default()
        .direction(Direction::Vertical)
        .margin(margin)
        .constraints(constraints)
        .split(body)
        .to_vec()
}

fn task_title(app: &App, id: u64) -> String {
    app.data
        .task(id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Unknown task".into())
}

fn draw_add_subtask_popup(f: &mut Frame, app: &App, body: Rect, task_id: u64) {
    let theme = &app.theme;
    let chunks = popup_body_layout(body, PopupLayout::Subtask);
    if chunks.len() < 3 {
        return;
    }
    let parent = task_title(app, task_id);
    let existing = app
        .data
        .task(task_id)
        .map(|t| t.subtasks.len())
        .unwrap_or(0);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Task  ", Style::default().fg(theme.dim)),
            Span::styled(
                super::widgets::truncate(&parent, chunks[0].width.saturating_sub(8) as usize),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "  ({existing} subtask{})",
                    if existing == 1 { "" } else { "s" }
                ),
                Style::default().fg(theme.dim),
            ),
        ])),
        chunks[0],
    );
    draw_singleline_editor(f, chunks[1], theme, &app.input_buffer);
    draw_action_footer(
        f,
        chunks[2],
        theme,
        &[
            ("Enter", "add", theme.success),
            ("q", "done", theme.warning),
        ],
    );
}

fn draw_action_footer(
    f: &mut Frame,
    area: Rect,
    theme: &crate::app::Theme,
    actions: &[(&str, &str, ratatui::style::Color)],
) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let sep = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));
    let inner = sep.inner(area);
    f.render_widget(sep, area);

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label, color)) in actions.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}"),
            Style::default().fg(theme.dim),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn draw_singleline_editor(f: &mut Frame, area: Rect, theme: &crate::app::Theme, text: &str) {
    if !rect_ok(area) {
        return;
    }
    let input_block = Block::default()
        .title(Span::styled(" Title ", Style::default().fg(theme.dim)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.panel).fg(theme.text));
    let inner = input_block.inner(area);
    f.render_widget(input_block, area);
    if !rect_ok(inner) {
        return;
    }
    let max_w = inner.width.saturating_sub(2) as usize;
    let content = if text.is_empty() {
        Line::from(vec![
            Span::styled("Subtask title…", Style::default().fg(theme.dim)),
            Span::styled("|", Style::default().fg(theme.accent)),
        ])
    } else {
        Line::from(Span::styled(
            format_input_line(text, max_w),
            Style::default().fg(theme.text),
        ))
    };
    f.render_widget(Paragraph::new(content).alignment(Alignment::Left), inner);
}

fn draw_popup_hint(f: &mut Frame, area: Rect, theme: &crate::app::Theme, hint: &str) {
    if area.height == 0 {
        return;
    }
    let max = area.width.saturating_sub(2) as usize;
    f.render_widget(
        Paragraph::new(Span::styled(
            super::widgets::truncate(hint, max),
            Style::default().fg(theme.dim),
        )),
        area,
    );
}

fn format_input_line(text: &str, max_w: usize) -> String {
    if text.is_empty() {
        return "|".to_string();
    }
    let max_text = max_w.saturating_sub(1);
    format!("{}|", super::widgets::truncate(text, max_text))
}

fn truncate_field(s: &str, max: usize) -> String {
    super::widgets::truncate(s, max)
}

pub(crate) fn popup_field_line(
    theme: &crate::app::Theme,
    label: &str,
    value: String,
    active: bool,
    value_max: usize,
) -> Line<'static> {
    let label_style = if active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };
    let value_style = if active {
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text)
    };
    Line::from(vec![
        Span::styled(format!("{:<20} ", label), label_style),
        Span::styled(truncate_field(&value, value_max), value_style),
    ])
}

pub(crate) fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .title(Span::styled(" Input ", Style::default().fg(theme.accent)));
    let p = Paragraph::new(format!("{}|", app.input_buffer))
        .style(Style::default().fg(theme.text))
        .block(block);
    f.render_widget(p, chunks[0]);
}

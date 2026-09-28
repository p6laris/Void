use super::*;
use crate::app::{BulkAction, InputField, Popup};
use crate::model::Priority;

fn popup_size(popup: &Popup, area: Rect) -> (u16, u16) {
    match popup {
        Popup::AddTask | Popup::EditTask(_) => {
            let w = (area.width.saturating_mul(68) / 100).clamp(58, 86);
            let h = (area.height.saturating_mul(75) / 100).clamp(16, 26);
            (w, h)
        }
        Popup::AddSubtask(_) | Popup::EditSubtask(_, _) => {
            let w = (area.width.saturating_mul(55) / 100).clamp(48, 70);
            let h = (area.height.saturating_mul(55) / 100).clamp(14, 20);
            (w, h)
        }
        Popup::ConfirmDelete(_) => {
            let w = (area.width.saturating_mul(50) / 100).clamp(46, 64);
            let h = 12u16.min(area.height);
            (w, h)
        }
        Popup::BulkConfirm(_) => {
            let w = (area.width.saturating_mul(55) / 100).clamp(50, 70);
            let h = 14u16.min(area.height);
            (w, h)
        }
        Popup::ConfirmQuit => {
            let w = (area.width.saturating_mul(55) / 100).clamp(52, 68);
            let h = 11u16.min(area.height);
            (w, h)
        }
        Popup::EmptyQueueChoice => {
            let w = (area.width.saturating_mul(55) / 100).clamp(52, 68);
            let h = 15u16.min(area.height);
            (w, h)
        }
    }
}

fn popup_rect(popup: &Popup, area: Rect) -> Rect {
    let (pw, ph) = popup_size(popup, area);
    let mut r = Rect::default();
    r.width = pw.min(area.width);
    r.height = ph.min(area.height);
    r.x = (area.width.saturating_sub(r.width)) / 2;
    r.y = (area.height.saturating_sub(r.height)) / 2;
    r
}

fn rect_ok(area: Rect) -> bool {
    area.width > 0 && area.height > 0
}

pub(crate) fn draw_popup(f: &mut Frame, app: &mut App) {
    let Some(popup) = app.input.popup.clone() else {
        return;
    };
    let icons = app.icons;
    let area = f.area();
    let popup_area = popup_rect(&popup, area);
    if !rect_ok(popup_area) {
        return;
    }

    f.render_widget(Clear, popup_area);

    let (border_color, title_text) = match &popup {
        Popup::AddTask => (app.theme.accent, format!(" {} Add Task ", icons.plus)),
        Popup::EditTask(_) => (app.theme.accent, format!(" {} Edit Task ", icons.edit)),
        Popup::ConfirmDelete(_) => (
            app.theme.error,
            format!(" {} Confirm Delete ", icons.delete),
        ),
        Popup::ConfirmQuit => (app.theme.warning, format!(" {} Quit Void ", icons.timer)),
        Popup::EmptyQueueChoice => (
            app.theme.success,
            format!(" {} Queue Cleared ", icons.check),
        ),
        Popup::AddSubtask(_) => (app.theme.accent, format!(" {} Add Subtask ", icons.plus)),
        Popup::EditSubtask(_, _) => (app.theme.accent, format!(" {} Edit Subtask ", icons.edit)),
        Popup::BulkConfirm(action) => match action {
            BulkAction::Delete => (app.theme.error, format!(" {} Bulk Delete ", icons.delete)),
            BulkAction::MarkDone => (
                app.theme.success,
                format!(" {} Bulk Complete ", icons.check),
            ),
        },
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(app.theme.bg))
        .title(Span::styled(
            title_text,
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ));
    let body = block.inner(popup_area);
    f.render_widget(block, popup_area);

    if !rect_ok(body) {
        return;
    }

    match &popup {
        Popup::AddTask => draw_task_form_popup(f, app, body, false),
        Popup::EditTask(_) => draw_task_form_popup(f, app, body, true),
        Popup::ConfirmDelete(id) => draw_confirm_delete_popup(f, app, body, *id),
        Popup::EmptyQueueChoice => draw_empty_queue_popup(f, app, body),
        Popup::ConfirmQuit => draw_confirm_quit_popup(f, app, body),
        Popup::AddSubtask(id) => draw_subtask_popup(f, app, body, *id, false),
        Popup::EditSubtask(task_id, _) => draw_subtask_popup(f, app, body, *task_id, true),
        Popup::BulkConfirm(action) => draw_bulk_confirm_popup(f, app, body, action),
    }
}

fn draw_task_form_popup(f: &mut Frame, app: &App, body: Rect, _is_edit: bool) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(body);

    let form_area = chunks[0];
    let (left_area, right_area) =
        if matches!(app.input.input_field, InputField::DueDate) && form_area.width >= 54 {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
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

    let title_display = cursor(
        matches!(app.input.input_field, InputField::Title),
        &app.input.input_buffer,
    );

    let estimate_display = if matches!(app.input.input_field, InputField::Estimate) {
        format!("{} min  [↑/↓ ±5m]", app.input.input_number)
    } else {
        format!("{} min", app.input.input_number)
    };

    let due_display = if matches!(app.input.input_field, InputField::DueDate) {
        cursor(true, &app.input.input_due_date)
    } else if app.input.input_due_date.is_empty() {
        "— None".to_string()
    } else {
        app.input.input_due_date.clone()
    };

    let tags_display = cursor(
        matches!(app.input.input_field, InputField::Tags),
        &app.input.input_tags,
    );

    let val_max = left_area.width.saturating_sub(20) as usize;

    let form_lines = vec![
        // Title Row
        popup_field_row(
            theme,
            icons.edit,
            "Title",
            &title_display,
            matches!(app.input.input_field, InputField::Title),
            val_max,
        ),
        Line::from(""),
        // Estimate Row
        popup_field_row(
            theme,
            icons.timer,
            "Estimate",
            &estimate_display,
            matches!(app.input.input_field, InputField::Estimate),
            val_max,
        ),
        Line::from(""),
        // Priority Row (Chips)
        popup_priority_row(
            theme,
            icons.tasks,
            app.input.input_priority,
            matches!(app.input.input_field, InputField::Priority),
        ),
        Line::from(""),
        // Due Date Row
        popup_field_row(
            theme,
            icons.calendar,
            "Due Date",
            &due_display,
            matches!(app.input.input_field, InputField::DueDate),
            val_max,
        ),
        Line::from(""),
        // Tags Row
        popup_field_row(
            theme,
            icons.dot,
            "Tags",
            &tags_display,
            matches!(app.input.input_field, InputField::Tags),
            val_max,
        ),
    ];

    f.render_widget(Paragraph::new(form_lines), left_area);

    if let Some(r) = right_area {
        super::calendar::render_due_date_calendar(f, r, app.stats.calendar_date, theme);
    }

    if chunks.len() > 1 {
        let hint_line = if matches!(app.input.input_field, InputField::DueDate) {
            Line::from(vec![
                Span::styled(
                    "t",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" today  ", Style::default().fg(theme.dim)),
                Span::styled(
                    "m",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" tmrw  ", Style::default().fg(theme.dim)),
                Span::styled(
                    "w",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" +1wk  ", Style::default().fg(theme.dim)),
                Span::styled(
                    "c",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" clear  ", Style::default().fg(theme.dim)),
                Span::styled("Tab", Style::default().fg(theme.accent)),
                Span::styled(" next  ", Style::default().fg(theme.dim)),
                Span::styled("Enter", Style::default().fg(theme.success)),
                Span::styled(" save  ", Style::default().fg(theme.dim)),
                Span::styled("Esc", Style::default().fg(theme.dim)),
                Span::styled(" cancel", Style::default().fg(theme.dim)),
            ])
        } else if matches!(app.input.input_field, InputField::Priority) {
            Line::from(vec![
                Span::styled(
                    "1/2/3",
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" set priority  ", Style::default().fg(theme.dim)),
                Span::styled("Space / ←→", Style::default().fg(theme.accent)),
                Span::styled(" cycle  ", Style::default().fg(theme.dim)),
                Span::styled("Tab", Style::default().fg(theme.accent)),
                Span::styled(" next  ", Style::default().fg(theme.dim)),
                Span::styled("Enter", Style::default().fg(theme.success)),
                Span::styled(" save", Style::default().fg(theme.dim)),
            ])
        } else {
            Line::from(vec![
                Span::styled("Tab / Shift+Tab", Style::default().fg(theme.accent)),
                Span::styled(" switch field  ", Style::default().fg(theme.dim)),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" save task  ", Style::default().fg(theme.dim)),
                Span::styled("Esc", Style::default().fg(theme.dim)),
                Span::styled(" cancel", Style::default().fg(theme.dim)),
            ])
        };
        f.render_widget(
            Paragraph::new(hint_line).alignment(Alignment::Center),
            chunks[1],
        );
    }
}

fn popup_field_row<'a>(
    theme: &Theme,
    icon: &'a str,
    label: &'a str,
    value: &'a str,
    active: bool,
    max_w: usize,
) -> Line<'a> {
    let (label_style, val_style) = if active {
        (
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        )
    } else {
        (
            Style::default().fg(theme.dim),
            Style::default().fg(theme.text),
        )
    };

    let bullet = if active { "●" } else { " " };
    let active_indicator = if active {
        Span::styled(format!("{bullet} "), Style::default().fg(theme.accent))
    } else {
        Span::styled("  ", Style::default().fg(theme.dim))
    };

    Line::from(vec![
        active_indicator,
        Span::styled(format!("{} {:<12} ", icon, label), label_style),
        Span::styled(super::widgets::truncate(value, max_w), val_style),
    ])
}

fn popup_priority_row<'a>(
    theme: &Theme,
    icon: &'a str,
    priority: Priority,
    active: bool,
) -> Line<'a> {
    let label_style = if active {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.dim)
    };

    let bullet = if active { "●" } else { " " };
    let active_indicator = if active {
        Span::styled(format!("{bullet} "), Style::default().fg(theme.accent))
    } else {
        Span::styled("  ", Style::default().fg(theme.dim))
    };

    let make_chip =
        |label: &'static str, color: ratatui::style::Color, is_sel: bool| -> Span<'static> {
            if is_sel {
                Span::styled(
                    format!(" [● {}] ", label),
                    Style::default()
                        .fg(color)
                        .bg(theme.panel)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(format!("  ○ {}  ", label), Style::default().fg(theme.dim))
            }
        };

    Line::from(vec![
        active_indicator,
        Span::styled(format!("{} Priority     ", icon), label_style),
        make_chip("Low", theme.info, priority == Priority::Low),
        make_chip("Med", theme.warning, priority == Priority::Medium),
        make_chip("High", theme.error, priority == Priority::High),
    ])
}

fn draw_subtask_popup(f: &mut Frame, app: &App, body: Rect, task_id: u64, is_edit: bool) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(body);

    let parent_title = app
        .data
        .task(task_id)
        .map(|t| t.title.clone())
        .unwrap_or_else(|| "Unknown task".into());

    let existing_subtasks = app
        .data
        .task(task_id)
        .map(|t| t.subtasks.clone())
        .unwrap_or_default();

    // 1. Parent Task Header
    let header_line = Line::from(vec![
        Span::styled(
            format!("{} Parent Task: ", icons.tasks),
            Style::default().fg(theme.dim),
        ),
        Span::styled(
            super::widgets::truncate(&parent_title, chunks[0].width.saturating_sub(20) as usize),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({} subtasks)", existing_subtasks.len()),
            Style::default().fg(theme.accent),
        ),
    ]);
    f.render_widget(Paragraph::new(header_line), chunks[0]);

    // 2. Existing Subtasks Preview Card
    let mut preview_lines = Vec::new();
    if existing_subtasks.is_empty() {
        preview_lines.push(Line::from(Span::styled(
            "  No subtasks added yet.",
            Style::default().fg(theme.dim),
        )));
    } else {
        let start = existing_subtasks.len().saturating_sub(4);
        for sub in &existing_subtasks[start..] {
            let (icon, color) = if sub.done {
                (icons.check, theme.success)
            } else {
                ("○", theme.dim)
            };
            preview_lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::styled(
                    super::widgets::truncate(
                        &sub.title,
                        chunks[1].width.saturating_sub(8) as usize,
                    ),
                    if sub.done {
                        Style::default().fg(theme.dim)
                    } else {
                        Style::default().fg(theme.text)
                    },
                ),
            ]));
        }
    }
    f.render_widget(
        Paragraph::new(preview_lines).block(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::default().bg(theme.bg)),
        ),
        chunks[1],
    );

    // 3. Subtask Input Box
    let label = if is_edit {
        " Edit Subtask Title "
    } else {
        " New Subtask Title "
    };
    let input_block = Block::default()
        .title(Span::styled(label, Style::default().fg(theme.accent)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.panel));
    let inner = input_block.inner(chunks[2]);
    f.render_widget(input_block, chunks[2]);

    if rect_ok(inner) {
        let val = if app.input.input_buffer.is_empty() {
            Line::from(vec![
                Span::styled("Type subtask title…", Style::default().fg(theme.dim)),
                Span::styled("|", Style::default().fg(theme.accent)),
            ])
        } else {
            Line::from(vec![
                Span::styled(
                    &app.input.input_buffer,
                    Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                ),
                Span::styled("|", Style::default().fg(theme.accent)),
            ])
        };
        f.render_widget(Paragraph::new(val), inner);
    }

    // 4. Action Footer
    let footer_line = if is_edit {
        Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" save  ", Style::default().fg(theme.dim)),
            Span::styled("Esc", Style::default().fg(theme.dim)),
            Span::styled(" cancel", Style::default().fg(theme.dim)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                "Enter",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" add & type next  ", Style::default().fg(theme.dim)),
            Span::styled("Esc / q", Style::default().fg(theme.dim)),
            Span::styled(" done", Style::default().fg(theme.dim)),
        ])
    };
    f.render_widget(
        Paragraph::new(footer_line).alignment(Alignment::Center),
        chunks[3],
    );
}

fn draw_confirm_delete_popup(f: &mut Frame, app: &App, body: Rect, task_id: u64) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(body);

    let task = app.data.task(task_id);
    let title = task.map(|t| t.title.as_str()).unwrap_or("Unknown task");
    let sub_count = task.map(|t| t.subtasks.len()).unwrap_or(0);
    let mins = task.map(|t| t.actual_minutes).unwrap_or(0);

    // 1. Header
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("{} Permanent Task Deletion", icons.delete),
            Style::default()
                .fg(theme.error)
                .add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    // 2. Target Task Name
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Delete \"", Style::default().fg(theme.text)),
            Span::styled(
                super::widgets::truncate(title, chunks[1].width.saturating_sub(12) as usize),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("\"?", Style::default().fg(theme.text)),
        ])),
        chunks[1],
    );

    // 3. Metadata Context
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("Contains {} subtask(s) · {}m focused", sub_count, mins),
            Style::default().fg(theme.dim),
        )])),
        chunks[2],
    );

    // 4. Warning Alert
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "This action cannot be undone.",
            Style::default().fg(theme.warning),
        )])),
        chunks[3],
    );

    // 5. Buttons
    let buttons = Line::from(vec![
        Span::styled(
            " [y] Delete ",
            Style::default()
                .fg(theme.error)
                .bg(theme.panel)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("    ", Style::default()),
        Span::styled(
            " [n / Esc] Cancel ",
            Style::default().fg(theme.dim).bg(theme.panel),
        ),
    ]);
    f.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        chunks[4],
    );
}

fn draw_bulk_confirm_popup(f: &mut Frame, app: &App, body: Rect, action: &BulkAction) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(body);

    let count = app.task_ui.bulk_selected.len();
    let (action_title, action_color) = match action {
        BulkAction::MarkDone => (
            format!("{} Complete {} Selected Tasks", icons.check, count),
            theme.success,
        ),
        BulkAction::Delete => (
            format!(
                "{} Permanently Delete {} Selected Tasks",
                icons.delete, count
            ),
            theme.error,
        ),
    };

    // 1. Header
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            action_title,
            Style::default()
                .fg(action_color)
                .add_modifier(Modifier::BOLD),
        ))),
        chunks[0],
    );

    // 2. Preview List of Selected Tasks
    let mut preview_lines = Vec::new();
    let mut shown = 0;
    for &id in &app.task_ui.bulk_selected {
        if shown >= 3 {
            let rem = count - shown;
            preview_lines.push(Line::from(Span::styled(
                format!("  (+ {} more tasks...)", rem),
                Style::default().fg(theme.dim),
            )));
            break;
        }
        if let Some(t) = app.data.task(id) {
            preview_lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(action_color)),
                Span::styled(
                    super::widgets::truncate(&t.title, chunks[1].width.saturating_sub(6) as usize),
                    Style::default().fg(theme.text),
                ),
            ]));
            shown += 1;
        }
    }
    f.render_widget(Paragraph::new(preview_lines), chunks[1]);

    // 3. Warning (if delete)
    if matches!(action, BulkAction::Delete) {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Warning: All selected tasks will be removed permanently.",
                Style::default().fg(theme.warning),
            ))),
            chunks[2],
        );
    }

    // 4. Action Buttons
    let buttons = Line::from(vec![
        Span::styled(
            " [y] Confirm ",
            Style::default()
                .fg(action_color)
                .bg(theme.panel)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("    ", Style::default()),
        Span::styled(
            " [n / Esc] Cancel ",
            Style::default().fg(theme.dim).bg(theme.panel),
        ),
    ]);
    f.render_widget(
        Paragraph::new(buttons).alignment(Alignment::Center),
        chunks[3],
    );
}

fn draw_confirm_quit_popup(f: &mut Frame, app: &App, body: Rect) {
    let theme = &app.theme;
    let mins = app.timer.current_elapsed_seconds() / 60;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(2), Constraint::Min(5)])
        .split(body);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} min ", mins),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "of focus in progress. Save it before quitting?",
                Style::default().fg(theme.text),
            ),
        ])),
        chunks[0],
    );

    let option = |key: &str, color, label: &str, detail: &str| {
        Line::from(vec![
            Span::styled(
                format!(" [{key}] "),
                Style::default()
                    .fg(color)
                    .bg(theme.panel)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {label:<9}"),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("— {detail}"), Style::default().fg(theme.dim)),
        ])
    };
    let options = vec![
        option("  l  ", theme.success, "Log", "save the session and quit"),
        Line::from(""),
        option("  d  ", theme.error, "Discard", "quit without saving"),
        Line::from(""),
        option(" Esc ", theme.dim, "Cancel", "keep focusing"),
    ];
    f.render_widget(Paragraph::new(options), chunks[1]);
}

fn draw_empty_queue_popup(f: &mut Frame, app: &App, body: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(6),
        ])
        .split(body);

    // 1. Celebratory Banner
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} Queue Cleared! ", icons.check),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "All tasks in your queue are completed.",
                Style::default().fg(theme.text),
            ),
        ])),
        chunks[0],
    );

    // 2. Subtitle
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Choose how you want your focus session to proceed:",
            Style::default().fg(theme.dim),
        ))),
        chunks[1],
    );

    // 3. Option Cards
    let options = vec![
        Line::from(vec![
            Span::styled(
                " [Enter] ",
                Style::default()
                    .fg(theme.success)
                    .bg(theme.panel)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Free Focus   ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "— Continue timer, log as general focus",
                Style::default().fg(theme.dim),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [  p  ] ",
                Style::default()
                    .fg(theme.warning)
                    .bg(theme.panel)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Pause Timer  ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "— Pause timer and take a restorative break",
                Style::default().fg(theme.dim),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [  a  ] ",
                Style::default()
                    .fg(theme.accent)
                    .bg(theme.panel)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " Add Task     ",
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "— Open task creator to add more work",
                Style::default().fg(theme.dim),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [ Esc ] ", Style::default().fg(theme.dim).bg(theme.panel)),
            Span::styled(" Dismiss      ", Style::default().fg(theme.dim)),
            Span::styled("— Close this dialog", Style::default().fg(theme.dim)),
        ]),
    ];
    f.render_widget(Paragraph::new(options), chunks[2]);
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
    let p = Paragraph::new(format!("{}|", app.input.input_buffer))
        .style(Style::default().fg(theme.text))
        .block(block);
    f.render_widget(p, chunks[0]);
}

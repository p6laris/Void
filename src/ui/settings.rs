use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsRow {
    Header(&'static str, &'static str),
    Item(usize),
}

pub(crate) fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(area);

    let visible_height = chunks[0].height.saturating_sub(3) as usize;
    let selected = app.settings_state.selected;
    app.settings_state.page_size = visible_height.max(6);
    app.sync_settings_scroll();
    let scroll_offset = app.settings_state.scroll_offset;

    let theme = &app.theme;
    let icons = app.icons;
    let settings_labels: Vec<(&str, String, &str)> = vec![
        (
            "Focus minutes",
            format!("{} min", app.data.focus_minutes),
            "per focus session",
        ),
        (
            "Short break",
            format!("{} min", app.data.short_break_minutes),
            "between sessions",
        ),
        (
            "Long break",
            format!("{} min", app.data.long_break_minutes),
            "after cycle",
        ),
        (
            "Long break every",
            format!("{} sessions", app.data.long_break_every),
            "focus sessions per cycle",
        ),
        (
            "Daily goal",
            format!("{} min", app.data.daily_goal_minutes),
            "+/-15 per step",
        ),
        (
            "Sound on finish",
            if app.data.sound_enabled {
                "on".into()
            } else {
                "off".into()
            },
            "plays on completion",
        ),
        (
            "Notifications",
            if app.data.notify_on_finish {
                "on".into()
            } else {
                "off".into()
            },
            "desktop alerts",
        ),
        (
            "Auto-start breaks",
            if app.data.auto_start_breaks {
                "on".into()
            } else {
                "off".into()
            },
            "begin break automatically",
        ),
        (
            "Auto-start focus",
            if app.data.auto_start_focus {
                "on".into()
            } else {
                "off".into()
            },
            "begin focus after break",
        ),
        (
            "Active task",
            app.active_task
                .and_then(|id| app.data.tasks.iter().find(|t| t.id == id))
                .map(|t| t.title.clone())
                .unwrap_or_else(|| "(none)".into()),
            "cycle with Enter",
        ),
        (
            "Theme",
            app.theme_catalog.label(&app.data.theme),
            "cycle themes",
        ),
        (
            "Custom timer",
            format!("{} min", app.timer.custom_minutes),
            "freeform session",
        ),
        (
            "Auto-pick task",
            if app.data.auto_pick_task {
                "on".into()
            } else {
                "off".into()
            },
            "pick best task on start",
        ),
        (
            "Auto-advance task",
            if app.data.auto_advance_task {
                "on".into()
            } else {
                "off".into()
            },
            "next task after focus",
        ),
        (
            "When queue empty",
            app.data.empty_queue_behavior.label().to_string(),
            "free focus / pause / ask",
        ),
        (
            "Log breaks",
            if app.data.log_breaks {
                "on".into()
            } else {
                "off".into()
            },
            "record break sessions",
        ),
        (
            "Estimate reached",
            app.data.estimate_complete.label().to_string(),
            "nudge / off / auto-done",
        ),
        (
            "Export backup",
            "Enter to export".into(),
            "writes data.json for backup",
        ),
    ];

    let section_headers: &[(usize, &str, &str)] = &[
        (0, icons.timer, "Timer"),
        (5, icons.cycle, "Behavior"),
        (9, icons.tasks, "Tasks"),
        (10, icons.star, "Appearance"),
        (14, icons.play, "Sessions"),
        (17, icons.export, "Data"),
    ];

    let mut layout: Vec<SettingsRow> = Vec::new();
    for (i, _) in settings_labels.iter().enumerate() {
        for &(at, icon, name) in section_headers {
            if at == i {
                layout.push(SettingsRow::Header(icon, name));
            }
        }
        layout.push(SettingsRow::Item(i));
    }

    let visible_rows: Vec<&SettingsRow> = layout
        .iter()
        .skip(scroll_offset)
        .take(visible_height)
        .collect();

    let total_rows = layout.len();
    let can_scroll = total_rows > visible_height;

    let mut rows: Vec<Row> = Vec::new();
    for entry in &visible_rows {
        match entry {
            SettingsRow::Header(icon, name) => {
                rows.push(Row::new(vec![
                    Cell::from(""),
                    Cell::from(Span::styled(
                        format!("{icon} {name}"),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Cell::from(""),
                ]));
            }
            SettingsRow::Item(i) => {
                let (k, v, desc) = &settings_labels[*i];
                let is_selected = *i == selected;
                let marker = if is_selected { icons.chevron } else { " " };
                let row_style = if is_selected {
                    Style::default().bg(theme.select_bg).fg(theme.select_fg)
                } else {
                    Style::default().fg(theme.text)
                };
                let key_style = if is_selected {
                    Style::default()
                        .fg(theme.accent)
                        .bg(theme.select_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                let val_style = if is_selected {
                    Style::default().fg(theme.success).bg(theme.select_bg)
                } else {
                    Style::default().fg(theme.dim)
                };
                let value_with_desc = if desc.is_empty() {
                    v.clone()
                } else {
                    format!("{} ({})", v, desc)
                };
                rows.push(
                    Row::new(vec![
                        Cell::from(marker.to_string()).style(key_style),
                        Cell::from(k.to_string()).style(key_style),
                        Cell::from(value_with_desc).style(val_style),
                    ])
                    .style(row_style),
                );
            }
        }
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(22),
            Constraint::Min(10),
        ],
    )
    .block(themed_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} Settings ", icons.settings),
            Style::default().fg(theme.accent),
        )),
    ));
    f.render_widget(table, chunks[0]);

    let scroll_hint = if can_scroll {
        format!(
            "  {} {}/{}",
            icons.dot,
            scroll_offset + visible_rows.len(),
            total_rows
        )
    } else {
        String::new()
    };
    let hint = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} Up/Down", icons.chevron),
            Style::default().fg(theme.accent),
        ),
        Span::styled(" scroll  ", Style::default().fg(theme.dim)),
        Span::styled("j/k", Style::default().fg(theme.accent)),
        Span::styled(" nav  ", Style::default().fg(theme.dim)),
        Span::styled("Enter", Style::default().fg(theme.accent)),
        Span::styled(" toggle  ", Style::default().fg(theme.dim)),
        Span::styled("+/-", Style::default().fg(theme.accent)),
        Span::styled(" adjust", Style::default().fg(theme.dim)),
        Span::styled(scroll_hint, Style::default().fg(theme.dim)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[1]);
}

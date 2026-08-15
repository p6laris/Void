use super::*;

pub(crate) fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(2)])
        .split(area);

    let two_column = chunks[0].width >= 96;

    if two_column {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[0]);

        draw_help_column(f, app, cols[0], true);
        draw_help_column(f, app, cols[1], false);
    } else {
        draw_help_single(f, app, chunks[0]);
    }

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(format!("{} j/k / Up/Down", icons.chevron), Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" scroll  ", Style::default().bg(theme.bg).fg(theme.dim)),
        Span::styled("Tab / 1-6", Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" switch tabs  ", Style::default().bg(theme.bg).fg(theme.dim)),
        Span::styled("q / Esc", Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" close / return to dashboard", Style::default().bg(theme.bg).fg(theme.dim)),
    ]))
    .style(Style::default().bg(theme.bg))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[1]);
}

fn shortcut_row<'a>(theme: &'a Theme, key: &'a str, desc: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("  {:14} ", key), Style::default().bg(theme.bg).fg(theme.accent).add_modifier(Modifier::BOLD)),
        Span::styled(desc, Style::default().bg(theme.bg).fg(theme.text)),
    ])
}

fn section_header<'a>(theme: &'a Theme, icon: &'a str, title: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{icon} {title}"), Style::default().bg(theme.bg).fg(theme.mode_focus).add_modifier(Modifier::BOLD)),
    ])
}

fn draw_help_column(f: &mut Frame, app: &App, area: Rect, left: bool) {
    let theme = &app.theme;
    let icons = app.icons;
    let st_dim = Style::default().bg(theme.bg).fg(theme.dim);

    let mut lines = Vec::new();

    if left {
        // Left Column: Global, Timer & Zen, Settings
        lines.push(section_header(theme, icons.dashboard, "Global & Navigation"));
        lines.push(shortcut_row(theme, "Tab / 1-6", "Switch primary tabs"));
        lines.push(shortcut_row(theme, "q / Esc", "Quit Void (auto-saves all state)"));
        lines.push(shortcut_row(theme, "Ctrl-S", "Export instant JSON backup"));
        lines.push(shortcut_row(theme, "h / ?", "Open this help cheat sheet"));
        lines.push(Line::from(Span::styled(" ", st_dim)));

        lines.push(section_header(theme, icons.timer, "Timer & Focus"));
        lines.push(shortcut_row(theme, "Space / s", "Start / pause timer"));
        lines.push(shortcut_row(theme, "z", "Toggle distraction-free Zen mode"));
        lines.push(shortcut_row(theme, "m", "Toggle mode (Focus / Custom timer)"));
        lines.push(shortcut_row(theme, "+ / -", "Adjust timer duration (+/- 1 min)"));
        lines.push(shortcut_row(theme, "n", "Skip session (logs elapsed time)"));
        lines.push(shortcut_row(theme, "r", "Reset timer to full duration"));
        lines.push(shortcut_row(theme, "E", "End session early with summary"));
        lines.push(Line::from(Span::styled(" ", st_dim)));

        lines.push(section_header(theme, icons.settings, "Settings & Appearance"));
        lines.push(shortcut_row(theme, "j / k", "Navigate settings rows"));
        lines.push(shortcut_row(theme, "Enter / +-", "Toggle / cycle options"));
        lines.push(shortcut_row(theme, "Theme mode", "Auto (System OS) / Dark / Light"));
        lines.push(shortcut_row(theme, "e", "Export database to JSON backup"));
    } else {
        // Right Column: Tasks, Subtasks, Stats
        lines.push(section_header(theme, icons.tasks, "Task Management"));
        lines.push(shortcut_row(theme, "a", "Add new task (title, estimate, due, tags)"));
        lines.push(shortcut_row(theme, "e", "Edit selected task properties"));
        lines.push(shortcut_row(theme, "d", "Delete selected task (with confirmation)"));
        lines.push(shortcut_row(theme, "Enter", "Cycle status (Pending → Active → Done)"));
        lines.push(shortcut_row(theme, "Space / f", "Set task as active for focus timer"));
        lines.push(shortcut_row(theme, "t", "Toggle today focus queue"));
        lines.push(shortcut_row(theme, "g", "Cycle filters (Open / Today / Done / All)"));
        lines.push(shortcut_row(theme, "/", "Instant fuzzy search by title & tags"));
        lines.push(shortcut_row(theme, "1 / 2 / 3", "Set priority: Low (·) / Med (◆) / High (★)"));
        lines.push(Line::from(Span::styled(" ", st_dim)));

        lines.push(section_header(theme, icons.tasks, "Subtasks Panel"));
        lines.push(shortcut_row(theme, "c", "Quick-add subtask to current task"));
        lines.push(shortcut_row(theme, "Tab", "Switch focus to Subtasks panel (q to exit)"));
        lines.push(shortcut_row(theme, "x", "Toggle subtask done [✓] / open [ ]"));
        lines.push(shortcut_row(theme, "Ctrl+j/k", "Reorder subtasks up/down"));
        lines.push(Line::from(Span::styled(" ", st_dim)));

        lines.push(section_header(theme, icons.chart, "Stats & Analytics"));
        lines.push(shortcut_row(theme, "v", "Cycle sub-views: week / tags / weekday / hourly"));
        lines.push(shortcut_row(theme, "Arrows", "Navigate heatmap grid and filter sessions"));
        lines.push(shortcut_row(theme, "Esc", "Clear heatmap date filter"));
        lines.push(shortcut_row(theme, "j / k", "Select session log"));
        lines.push(shortcut_row(theme, "+ / -", "Adjust session logged minutes"));
    }

    let title = if left { " Shortcuts (General & Timer) " } else { " Shortcuts (Tasks & Stats) " };
    let block = Block::default()
        .title(Span::styled(
            format!(" {} {title}", icons.help),
            Style::default().bg(theme.bg).fg(theme.accent),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().bg(theme.bg).fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));

    let scroll = app.ui.help_scroll;
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .block(block)
            .scroll((scroll, 0)),
        area,
    );
}

fn draw_help_single(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let st_dim = Style::default().bg(theme.bg).fg(theme.dim);

    let mut lines = Vec::new();
    lines.push(section_header(theme, icons.dashboard, "Global & Navigation"));
    lines.push(shortcut_row(theme, "Tab / 1-6", "Switch primary tabs"));
    lines.push(shortcut_row(theme, "q / Esc", "Quit Void (auto-saves all state)"));
    lines.push(shortcut_row(theme, "Ctrl-S", "Export instant JSON backup"));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    lines.push(section_header(theme, icons.timer, "Timer & Focus"));
    lines.push(shortcut_row(theme, "Space / s", "Start / pause timer"));
    lines.push(shortcut_row(theme, "z", "Toggle distraction-free Zen mode"));
    lines.push(shortcut_row(theme, "m", "Toggle mode (Focus / Custom timer)"));
    lines.push(shortcut_row(theme, "+ / -", "Adjust timer duration (+/- 1 min)"));
    lines.push(shortcut_row(theme, "n", "Skip session (logs elapsed time)"));
    lines.push(shortcut_row(theme, "r", "Reset timer to full duration"));
    lines.push(shortcut_row(theme, "E", "End session early with summary"));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    lines.push(section_header(theme, icons.tasks, "Task Management"));
    lines.push(shortcut_row(theme, "a", "Add new task (title, estimate, due, tags)"));
    lines.push(shortcut_row(theme, "e", "Edit selected task properties"));
    lines.push(shortcut_row(theme, "d", "Delete selected task"));
    lines.push(shortcut_row(theme, "Enter", "Cycle status (Pending → Active → Done)"));
    lines.push(shortcut_row(theme, "Space / f", "Set task as active for focus timer"));
    lines.push(shortcut_row(theme, "t", "Toggle today focus queue"));
    lines.push(shortcut_row(theme, "g", "Cycle filters (Open / Today / Done / All)"));
    lines.push(shortcut_row(theme, "/", "Instant fuzzy search by title & tags"));
    lines.push(shortcut_row(theme, "1 / 2 / 3", "Set priority: Low / Med / High"));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    lines.push(section_header(theme, icons.tasks, "Subtasks Panel"));
    lines.push(shortcut_row(theme, "c", "Quick-add subtask to current task"));
    lines.push(shortcut_row(theme, "Tab", "Switch focus to Subtasks panel"));
    lines.push(shortcut_row(theme, "x", "Toggle subtask done [✓] / open [ ]"));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    lines.push(section_header(theme, icons.chart, "Stats & Analytics"));
    lines.push(shortcut_row(theme, "v", "Cycle sub-views: week / tags / weekday / hourly"));
    lines.push(shortcut_row(theme, "Arrows", "Navigate heatmap grid and filter sessions"));
    lines.push(shortcut_row(theme, "Esc", "Clear heatmap date filter"));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    lines.push(section_header(theme, icons.settings, "Settings & Appearance"));
    lines.push(shortcut_row(theme, "j / k", "Navigate settings rows"));
    lines.push(shortcut_row(theme, "Enter / +-", "Toggle / cycle options"));
    lines.push(shortcut_row(theme, "Theme mode", "Auto (System OS) / Dark / Light"));

    let block = Block::default()
        .title(Span::styled(
            format!(" {} Help & Shortcuts ", icons.help),
            Style::default().bg(theme.bg).fg(theme.accent),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().bg(theme.bg).fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));

    let scroll = app.ui.help_scroll;
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .block(block)
            .scroll((scroll, 0)),
        area,
    );
}

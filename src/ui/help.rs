use super::*;

pub(crate) fn draw_help(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let text = vec![
        Line::from(Span::styled(
            "Void - keyboard shortcuts",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("Global", Style::default().fg(theme.accent))),
        Line::from("  Tab / 1-5   Switch tab (Dashboard, Tasks, Stats, Settings, Help)"),
        Line::from("  q / Esc     Quit (auto-saves)"),
        Line::from("  Ctrl-S      Export JSON backup"),
        Line::from(""),
        Line::from(Span::styled(
            "Timer / Zen",
            Style::default().fg(theme.accent),
        )),
        Line::from("  s / Space   Start / resume timer"),
        Line::from("  p           Pause timer (in Dashboard) / Cycle tasks (in Zen mode)"),
        Line::from("  r           Reset timer"),
        Line::from("  n           Skip (logs elapsed; does not advance pomodoro cycle)"),
        Line::from("  E           End session (pause + summary)"),
        Line::from("  m           Cycle mode (Focus / Short / Long / Custom)"),
        Line::from("  + / =       Increase duration by 1 min"),
        Line::from("  -           Decrease duration by 1 min"),
        Line::from("  z           Toggle Zen mode (distraction-free timer)"),
        Line::from("  Enter       Cycle active task status (Todo → Active → Done)"),
        Line::from("  x           Mark active task done"),
        Line::from(""),
        Line::from(Span::styled("Pomodoro", Style::default().fg(theme.accent))),
        Line::from("  Cycle position persists across restarts"),
        Line::from("  After focus, auto-switches to break (enable auto-start in Settings)"),
        Line::from("  Long break every N focus sessions (configurable)"),
        Line::from("  All tasks done → free focus, pause, or prompt (Settings)"),
        Line::from(""),
        Line::from(Span::styled("Stats", Style::default().fg(theme.accent))),
        Line::from("  v           Toggle Analytics / Overview view"),
        Line::from("  Arrows      Navigate heatmap & filter sessions"),
        Line::from("  Esc         Clear heatmap filter"),
        Line::from("  j / k       Select session in list"),
        Line::from("  d           Delete selected session"),
        Line::from("  + / -       Adjust session duration"),
        Line::from(""),
        Line::from(Span::styled("Dashboard", Style::default().fg(theme.accent))),
        Line::from("  j / k       Navigate pending tasks"),
        Line::from("  Ctrl+j/k    Reorder selected task in queue"),
        Line::from("  1-9         Toggle subtasks for selected task"),
        Line::from("  f           Set selected task as active for timer"),
        Line::from("  Enter       Cycle status of selected task"),
        Line::from("  x           Mark selected task done"),
        Line::from(""),
        Line::from(Span::styled("Tasks", Style::default().fg(theme.accent))),
        Line::from("  a           Add task (title, estimate, due, tags)"),
        Line::from("  e           Edit selected task"),
        Line::from("  d           Delete selected task (with confirmation)"),
        Line::from("  c           Add subtask to selected task"),
        Line::from("  Tab         Focus subtask list (j/k navigate, x toggle, q back)"),
        Line::from("  x           Toggle selected subtask done/open"),
        Line::from("  -           Remove selected subtask"),
        Line::from("  Enter       Cycle status: Pending → In Progress → Done"),
        Line::from("  Space       Set as active task for timer"),
        Line::from("  f           Start focus on selected task"),
        Line::from("  t           Toggle today-queue flag"),
        Line::from("  g           Cycle filter (All / Open / Done / Today)"),
        Line::from("  /           Search tasks by title or tags"),
        Line::from("  1 / 2 / 3   Set priority Low / Med / High"),
        Line::from("  j / k       Navigate list"),
        Line::from("  Ctrl+j/k    Reorder task in queue"),
        Line::from(""),
        Line::from(Span::styled("Settings", Style::default().fg(theme.accent))),
        Line::from("  Up / Down   Navigate"),
        Line::from("  Enter / +-  Increment value"),
        Line::from("  Left / -    Decrement value"),
        Line::from("  Theme       Cycle Dark / Light / Polaris / Matrix in Settings"),
        Line::from("  e           Export data (JSON backup)"),
        Line::from(""),
        Line::from(Span::styled("Data", Style::default().fg(theme.accent))),
        Line::from("  Data persists locally in SQLite (~/.local/share/void/)."),
        Line::from("  Press 'e' in Settings to export a JSON backup."),
        Line::from("  Void never sends your tasks anywhere — fully offline."),
    ];
    let block = Block::default()
        .title(Span::styled(
            format!(" {} Help ", app.icons.help),
            Style::default().fg(theme.accent),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.dim));
    let max_scroll = (text.len() as u16).saturating_sub(area.height.saturating_sub(2));
    let scroll = app.ui.help_scroll.min(max_scroll);

    f.render_widget(
        Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
}

use super::*;

pub(crate) fn draw_about(f: &mut Frame, app: &App, area: Rect) {
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

        draw_about_left(f, app, cols[0]);
        draw_about_right(f, app, cols[1]);
    } else {
        draw_about_single(f, app, chunks[0]);
    }

    let footer = Paragraph::new(Line::from(vec![
        Span::styled(format!("{} j/k / Up/Down", icons.chevron), Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" scroll  ", Style::default().bg(theme.bg).fg(theme.dim)),
        Span::styled("Tab / 1-6", Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" switch tabs  ", Style::default().bg(theme.bg).fg(theme.dim)),
        Span::styled("q / Esc", Style::default().bg(theme.bg).fg(theme.accent)),
        Span::styled(" return to dashboard", Style::default().bg(theme.bg).fg(theme.dim)),
    ]))
    .style(Style::default().bg(theme.bg))
    .alignment(Alignment::Center);
    f.render_widget(footer, chunks[1]);
}

fn draw_about_left(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let st_dim = Style::default().bg(theme.bg).fg(theme.dim);
    let st_text = Style::default().bg(theme.bg).fg(theme.text);
    let st_bold = Style::default().bg(theme.bg).fg(theme.text).add_modifier(Modifier::BOLD);
    let st_accent = Style::default().bg(theme.bg).fg(theme.accent).add_modifier(Modifier::BOLD);
    let st_focus = Style::default().bg(theme.bg).fg(theme.mode_focus);
    let st_success = Style::default().bg(theme.bg).fg(theme.success);
    let st_warn = Style::default().bg(theme.bg).fg(theme.warning);

    let lines = vec![
        Line::from(vec![
            Span::styled(" 󰖔 Void ", st_accent),
            Span::styled("v0.5.0-beta.2", st_focus),
        ]),
        Line::from(Span::styled("  A distraction-free terminal sanctuary for deep focus.", st_dim)),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  [", st_dim),
            Span::styled("Rust 2021", st_focus),
            Span::styled("]  [", st_dim),
            Span::styled("Local SQLite", st_success),
            Span::styled("]  [", st_dim),
            Span::styled("100% Offline", st_accent),
            Span::styled("]  [", st_dim),
            Span::styled("MIT License", st_warn),
            Span::styled("]", st_dim),
        ]),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(Span::styled("  Design Principles", st_accent)),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  • Keyboard-Centric: ", st_bold),
            Span::styled("Every action is controllable instantly without touching a mouse.", st_text),
        ]),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  • Fluid Pomodoro: ", st_bold),
            Span::styled("Effortlessly transition between deep focus and restorative breaks.", st_text),
        ]),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  • Absolute Privacy: ", st_bold),
            Span::styled("Zero telemetry and zero cloud dependencies. Your data never leaves your machine.", st_text),
        ]),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  • Adaptive Themes: ", st_bold),
            Span::styled("Auto-detects OS Light/Dark appearance and applies your preferred palette.", st_text),
        ]),
    ];

    let block = Block::default()
        .title(Span::styled(
            format!(" {} Overview & Philosophy ", icons.about),
            st_accent,
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().bg(theme.bg).fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));

    let scroll = app.ui.about_scroll;
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .block(block)
            .scroll((scroll, 0)),
        area,
    );
}

fn draw_about_right(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let st_dim = Style::default().bg(theme.bg).fg(theme.dim);
    let st_text = Style::default().bg(theme.bg).fg(theme.text);
    let st_bold = Style::default().bg(theme.bg).fg(theme.text).add_modifier(Modifier::BOLD);
    let st_accent = Style::default().bg(theme.bg).fg(theme.accent).add_modifier(Modifier::BOLD);
    let st_link = Style::default().bg(theme.bg).fg(theme.accent);
    let st_focus = Style::default().bg(theme.bg).fg(theme.mode_focus).add_modifier(Modifier::BOLD);

    let acks = [
        ("Ratatui", "Terminal UI & rendering engine"),
        ("Crossterm", "Terminal input & event loop"),
        ("Rodio", "Hardware focus audio playback"),
        ("Rusqlite", "Embedded transactional SQLite database"),
        ("Chrono / Time", "Precision date-time calculation"),
        ("Nerd Fonts", "Developer iconography & symbols"),
    ];

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(" Open-Source Ecosystem", st_accent)));
    lines.push(Line::from(Span::styled(" ", st_dim)));

    for (name, desc) in acks {
        lines.push(Line::from(vec![
            Span::styled(format!(" • {:15} ", name), st_focus),
            Span::styled(desc, st_text),
        ]));
    }

    lines.push(Line::from(Span::styled(" ", st_dim)));
    lines.push(Line::from(Span::styled(" Soundtrack & Audio", st_accent)));
    lines.push(Line::from(vec![
        Span::styled(" • Ambient audio by ", st_text),
        Span::styled("Universfield", st_bold),
        Span::styled(" (Pixabay)", st_dim),
    ]));

    lines.push(Line::from(Span::styled(" ", st_dim)));
    lines.push(Line::from(Span::styled(" Project & Community", st_accent)));
    lines.push(Line::from(vec![
        Span::styled(" • GitHub: ", st_bold),
        Span::styled("https://github.com/p6laris/Void", st_link),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" • Copyright: ", st_text),
        Span::styled("© 2024–2026 p6laris and Void Contributors", st_dim),
    ]));

    let block = Block::default()
        .title(Span::styled(
            format!(" {} Tech Stack & Credits ", icons.star),
            st_accent,
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().bg(theme.bg).fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));

    let scroll = app.ui.about_scroll;
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .block(block)
            .scroll((scroll, 0)),
        area,
    );
}

fn draw_about_single(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let st_dim = Style::default().bg(theme.bg).fg(theme.dim);
    let st_text = Style::default().bg(theme.bg).fg(theme.text);
    let st_bold = Style::default().bg(theme.bg).fg(theme.text).add_modifier(Modifier::BOLD);
    let st_accent = Style::default().bg(theme.bg).fg(theme.accent).add_modifier(Modifier::BOLD);
    let st_link = Style::default().bg(theme.bg).fg(theme.accent);
    let st_focus = Style::default().bg(theme.bg).fg(theme.mode_focus);

    let lines = vec![
        Line::from(vec![
            Span::styled(" 󰖔 Void ", st_accent),
            Span::styled("v0.5.0-beta.2", st_focus),
        ]),
        Line::from(Span::styled("  A minimalist, keyboard-driven productivity sanctuary.", st_dim)),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(Span::styled("  • 100% Offline with local SQLite storage", st_text)),
        Line::from(Span::styled("  • Fluid Pomodoro & task workflow", st_text)),
        Line::from(Span::styled("  • Adaptive OS Light/Dark theme detection", st_text)),
        Line::from(Span::styled(" ", st_dim)),
        Line::from(vec![
            Span::styled("  GitHub: ", st_bold),
            Span::styled("https://github.com/p6laris/Void", st_link),
        ]),
    ];

    let block = Block::default()
        .title(Span::styled(
            format!(" {} About Void ", icons.about),
            st_accent,
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().bg(theme.bg).fg(theme.panel_border))
        .style(Style::default().bg(theme.bg));

    let scroll = app.ui.about_scroll;
    f.render_widget(
        Paragraph::new(lines)
            .style(Style::default().bg(theme.bg))
            .block(block)
            .scroll((scroll, 0)),
        area,
    );
}

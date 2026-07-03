use super::*;

pub(crate) fn draw_about(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    // The user's exact ASCII art representation
    let braille_art = [
        "                            ...                 ",
        "                            -##                 ",
        "                            -.                  ",
        "                          ..                    ",
        "                         ..                     ",
        "                       ##+.                     ",
        "                       +#-.                     ",
        "                      ..                        ",
        "                      -.                        ",
        "                     ..                         ",
        "                   +##-                         ",
        "                   .##.                         ",
        "                     ..                         ",
        "                     ..                         ",
        "                      ..                        ",
        "                     .+#-                       ",
        "                 .....##-                       ",
        "                 ###.   ...                     ",
        "                 .-+.     -                     ",
        "                    -.     --..                 ",
        "                     ..    .##.                 ",
        "                      -. .-.                    ",
        "                      .##+                      ",
        "                       ...                      ",
    ];

    let summary_lines = vec![
        "Void is a minimalist, keyboard-driven productivity environment built for deep, continuous focus.",
        "At its core is a fluid Pomodoro system designed to naturally alternate between intensive work and restful breaks.",
        "All tasks, session logs, and statistics are stored locally on-device, guaranteeing absolute privacy and data ownership.",
        "The terminal interface is designed to be both visually striking and unobtrusive, stripping away unnecessary distractions.",
        "Everything from task management to performance analytics can be controlled efficiently without ever reaching for a mouse.",
        "Ultimately, Void serves as a sanctuary within the terminal, crafted to help tune out the noise and center attention on what truly matters."
    ];

    let mut text = vec![];

    // Add empty space at the top
    text.push(Line::from(""));
    text.push(Line::from(""));

    // Add the ASCII art
    for line in braille_art.iter() {
        text.push(
            Line::from(Span::styled(
                format!("  {}", *line),
                Style::default().fg(theme.accent),
            ))
            .alignment(Alignment::Left),
        );
    }

    text.push(Line::from(""));
    text.push(Line::from(""));

    // Add the summary
    for summary_line in summary_lines {
        text.push(
            Line::from(Span::styled(
                format!("  {}", summary_line),
                Style::default().fg(theme.text),
            ))
            .alignment(Alignment::Left),
        );
        text.push(Line::from(""));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  Acknowledgments & Open Source",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(""));

    let acks = vec![
        (
            "Ratatui",
            "https://github.com/ratatui/ratatui",
            "Terminal UI rendering",
        ),
        (
            "Crossterm",
            "https://github.com/crossterm-rs/crossterm",
            "Terminal input & event handling",
        ),
        (
            "Rodio",
            "https://github.com/RustAudio/rodio",
            "Audio playback",
        ),
        (
            "Rusqlite",
            "https://github.com/rusqlite/rusqlite",
            "Local SQLite database storage",
        ),
        (
            "Chrono",
            "https://github.com/chronotope/chrono",
            "Date & time formatting",
        ),
    ];

    for (name, link, desc) in acks {
        text.push(Line::from(vec![
            Span::styled(
                format!("  • {} ", name),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("- {} ", desc), Style::default().fg(theme.text)),
            Span::styled(format!("({})", link), Style::default().fg(theme.dim)),
        ]));
    }

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  Special Thanks:",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(vec![
        Span::styled("  • Sound Effects by ", Style::default().fg(theme.text)),
        Span::styled(
            "Universfield",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" from Pixabay ", Style::default().fg(theme.text)),
        Span::styled(
            "(https://pixabay.com/users/universfield-28281460/)",
            Style::default().fg(theme.dim),
        ),
    ]));

    text.push(Line::from(""));
    text.push(Line::from(Span::styled(
        "  License",
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    text.push(Line::from(Span::styled(
        "  MIT License",
        Style::default().fg(theme.text),
    )));
    text.push(Line::from(Span::styled(
        "  Copyright (c) 2024 p6laris and the Void contributors. All rights reserved.",
        Style::default().fg(theme.dim),
    )));

    text.push(Line::from(""));
    text.push(Line::from(vec![
        Span::styled(
            "  \u{f09b} GitHub: ",
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "https://github.com/p6laris/void",
            Style::default().fg(theme.dim),
        ),
    ]));

    text.push(Line::from(""));
    text.push(Line::from(""));

    let block = Block::default()
        .title(Span::styled(
            format!(" {} About Void ", app.icons.about),
            Style::default().fg(theme.accent),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.dim));

    let max_scroll = (text.len() as u16).saturating_sub(area.height.saturating_sub(2));
    let scroll = app.ui.about_scroll.min(max_scroll);

    f.render_widget(
        Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        area,
    );
}

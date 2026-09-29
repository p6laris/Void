use super::*;
use crate::app::settings::SECTION_STARTS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsRow {
    Header(&'static str, &'static str),
    Item(usize),
}

pub(crate) fn draw_settings(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(3)])
        .split(area);

    let visible_height = chunks[0].height.saturating_sub(3) as usize;
    let selected = app.settings_state.selected;
    app.settings_state.page_size = visible_height;
    app.sync_settings_scroll();
    let scroll_offset = app.settings_state.scroll_offset;

    app.refresh_settings_labels_cache();
    let settings_labels = app.settings_labels();

    let theme = &app.theme;
    let icons = app.icons;

    // One entry per `SECTION_STARTS` item, in the same order.
    let section_names: [(&str, &str); 6] = [
        (icons.timer, "Timer"),
        (icons.cycle, "Behavior"),
        (icons.tasks, "Tasks"),
        (icons.star, "Appearance"),
        (icons.play, "Sessions"),
        (icons.export, "Data"),
    ];

    let mut layout: Vec<SettingsRow> = Vec::new();
    for (i, item) in app.settings_state.items.iter().enumerate() {
        if let Some(pos) = SECTION_STARTS.iter().position(|s| s == item) {
            let (icon, name) = section_names[pos];
            layout.push(SettingsRow::Header(icon, name));
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
                let row = &settings_labels[*i];
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
                let value_with_desc = if row.desc.is_empty() {
                    row.value.clone()
                } else {
                    format!("{} ({})", row.value, row.desc)
                };
                rows.push(
                    Row::new(vec![
                        Cell::from(marker.to_string()).style(key_style),
                        Cell::from(row.key.to_string()).style(key_style),
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
    .block(dense_panel(
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
    .style(Style::default().bg(theme.bg))
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[1]);
}

#[cfg(test)]
mod tests {
    use crate::app::{App, FocusTab};
    use crate::db::Database;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{backend::TestBackend, Terminal};

    fn screen(term: &mut Terminal<TestBackend>, app: &mut App) -> String {
        term.draw(|f| crate::ui::render(f, app)).unwrap();
        let buf = term.backend().buffer();
        (0..buf.area.height)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .map(|(x, y)| buf[(x, y)].symbol().to_string())
            .collect()
    }

    #[test]
    fn the_selected_setting_stays_visible_on_a_short_terminal() {
        let mut app = App::with_database(Database::open_in_memory().unwrap()).unwrap();
        app.ui.tab = FocusTab::Settings;
        let labels = app.build_settings_labels();
        let n = labels.len();
        let mut term = Terminal::new(TestBackend::new(100, 18)).unwrap();
        screen(&mut term, &mut app);

        let keys = std::iter::repeat_n(KeyCode::Char('j'), n - 1)
            .chain(std::iter::repeat_n(KeyCode::Char('k'), n - 1));
        for code in keys {
            app.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
            let selected = app.settings_state.selected;
            assert!(
                screen(&mut term, &mut app).contains(labels[selected].key),
                "row {selected} ({}) scrolled out of view",
                labels[selected].key
            );
        }
    }
}

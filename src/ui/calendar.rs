use chrono::{Datelike, NaiveDate};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::calendar::{CalendarEventStore, Monthly};
use ratatui::Frame;

use crate::app::Theme;

/// Ratatui's calendar widget uses `time::Date`; this module isolates that from chrono elsewhere.
pub fn render_due_date_calendar(
    frame: &mut Frame,
    area: Rect,
    date: NaiveDate,
    theme: &Theme,
) {
    let Ok(time_date) = time::Date::from_calendar_date(
        date.year(),
        time::Month::try_from(date.month() as u8).unwrap_or(time::Month::January),
        date.day() as u8,
    ) else {
        return;
    };

    let mut store = CalendarEventStore::default();
    store.add(
        time_date,
        Style::default()
            .bg(theme.accent)
            .fg(theme.on_accent)
            .add_modifier(Modifier::BOLD),
    );

    let monthly = Monthly::new(time_date, store)
        .show_month_header(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .show_weekdays_header(Style::default().fg(theme.dim));
    frame.render_widget(monthly, area);
}

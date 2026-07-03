use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use super::icons::IconSet;
use crate::app::{App, Theme};
use crate::model::TaskStatus;

pub fn format_minutes(mins: u32) -> String {
    if mins >= 60 {
        format!("{}h {}m", mins / 60, mins % 60)
    } else {
        format!("{}m", mins)
    }
}

pub fn themed_panel<'a>(theme: &Theme, title: Line<'a>) -> Block<'a> {
    dense_panel(theme, title)
}

pub fn dense_panel<'a>(theme: &Theme, title: Line<'a>) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.panel_border))
        .style(Style::default().bg(theme.bg).fg(theme.text))
}

/// Bordered stats section with title (top + sides).
pub fn section_panel<'a>(theme: &Theme, title: Line<'a>) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(theme.panel_border))
        .style(Style::default().bg(theme.bg).fg(theme.text))
}

/// Bottom cap for the last section in a column.
pub fn section_panel_bottom<'a>(theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(theme.panel_border))
        .style(Style::default().bg(theme.bg).fg(theme.text))
}

pub fn timer_panel<'a>(theme: &Theme, title: Line<'a>, border: Color) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::TOP)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(theme.bg).fg(theme.text))
}

pub fn task_status_color(theme: &Theme, status: TaskStatus) -> Color {
    match status {
        TaskStatus::Done => theme.success,
        TaskStatus::InProgress => theme.warning,
        TaskStatus::Pending => theme.dim,
    }
}

pub fn task_status_icon(icons: IconSet, status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Done => icons.task_done,
        TaskStatus::InProgress => icons.task_progress,
        TaskStatus::Pending => icons.task_todo,
    }
}

pub fn active_task_spans(app: &App, theme: &Theme) -> Option<Vec<Span<'static>>> {
    let id = app.active_task?;
    let task = app.data.task(id)?;
    let icons = app.icons;
    let status_color = task_status_color(theme, task.status);
    Some(vec![
        Span::styled(
            format!("{} ", icons.task_active),
            Style::default().fg(theme.accent),
        ),
        Span::styled(truncate(&task.title, 22), Style::default().fg(theme.text)),
        Span::styled(
            format!(
                "  {} {}",
                task_status_icon(icons, task.status),
                task.status.short_label()
            ),
            Style::default().fg(status_color),
        ),
        Span::styled(
            format!("  {}/{}m", task.actual_minutes, task.estimated_minutes),
            Style::default().fg(theme.success),
        ),
    ])
}

pub fn chip<'a>(icon: &str, text: String, fg: Color, bg: Color) -> Span<'a> {
    Span::styled(
        format!(" {icon} {text} "),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

pub fn truncate(s: &str, max: usize) -> String {
    let width = unicode_width::UnicodeWidthStr::width(s);
    if width <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut w = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if w + cw + 1 > max {
            out.push('…');
            break;
        }
        out.push(ch);
        w += cw;
    }
    out
}

pub fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
            ratatui::layout::Constraint::Percentage(percent_y),
            ratatui::layout::Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
            ratatui::layout::Constraint::Percentage(percent_x),
            ratatui::layout::Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

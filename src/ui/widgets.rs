use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use super::icons::IconSet;
use crate::app::{App, Theme};
use crate::model::{Subtask, TaskStatus};

pub fn format_minutes(mins: u32) -> String {
    if mins >= 60 {
        format!("{}h {}m", mins / 60, mins % 60)
    } else {
        format!("{}m", mins)
    }
}

pub fn dense_panel<'a>(theme: &Theme, title: Line<'a>) -> Block<'a> {
    Block::default()
        .title(title)
        .borders(Borders::TOP)
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

pub fn subtask_line_style(theme: &Theme, done: bool) -> Style {
    if done {
        Style::default().fg(theme.dim)
    } else {
        Style::default().fg(theme.text)
    }
}

/// Numbered inline subtask lines for dashboard details and zen overlay (max 9).
pub fn subtask_inline_lines(
    subtasks: &[Subtask],
    theme: &Theme,
    icons: IconSet,
    indent: &str,
    title_max_chars: Option<usize>,
) -> Vec<Line<'static>> {
    subtasks
        .iter()
        .enumerate()
        .take(9)
        .map(|(i, subtask)| {
            let icon = if subtask.done { icons.check } else { icons.dot };
            let style = subtask_line_style(theme, subtask.done);
            let title = match title_max_chars {
                Some(max) => truncate(&subtask.title, max),
                None => subtask.title.clone(),
            };
            Line::from(vec![
                Span::styled(format!("{indent}[{}] {} ", i + 1, icon), style),
                Span::styled(title, style),
            ])
        })
        .collect()
}

pub fn active_task_spans(app: &App, theme: &Theme) -> Option<Vec<Span<'static>>> {
    let id = app.task_ui.active_task?;
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

/// A bracketed status chip — `[ 12d ]` with dim brackets and a coloured value.
///
/// Flat rather than filled: the terminal aesthetic reads as text, and solid background
/// chips fight with the panel backgrounds once several sit side by side.
pub fn chip<'a>(icon: &str, text: String, fg: Color, bracket: Color) -> Vec<Span<'a>> {
    let bracket_style = Style::default().fg(bracket);
    let body = if icon.is_empty() {
        text
    } else {
        format!("{icon} {text}")
    };
    vec![
        Span::styled("[", bracket_style),
        Span::styled(body, Style::default().fg(fg).add_modifier(Modifier::BOLD)),
        Span::styled("]", bracket_style),
    ]
}

/// A `// annotation` line in the quietest foreground token.
pub fn comment_line<'a>(theme: &Theme, text: impl Into<String>) -> Line<'a> {
    Line::from(Span::styled(
        format!("// {}", text.into()),
        Style::default().fg(theme.comment),
    ))
}

/// A `// annotation` span, for appending after other content on the same line.
pub fn comment_span<'a>(theme: &Theme, text: impl Into<String>) -> Span<'a> {
    Span::styled(
        format!("// {}", text.into()),
        Style::default().fg(theme.comment),
    )
}

/// Lowercase section heading: `▸ focus activity`. Lowercase and unadorned keeps the
/// headings from competing with the data underneath them.
pub fn section_title<'a>(theme: &Theme, icon: &str, label: &str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" {icon} "), Style::default().fg(theme.accent)),
        Span::styled(
            format!("{} ", label.to_lowercase()),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// A `[ ]` / `[~] `/ `[x]` checkbox for a task status. Brackets stay dim so the mark
/// carries the colour, the way a terminal checklist reads.
pub fn status_checkbox<'a>(
    theme: &Theme,
    _icons: IconSet,
    status: TaskStatus,
    override_style: Option<Style>,
) -> Vec<Span<'a>> {
    let (mark, color) = match status {
        TaskStatus::Done => ("✓", theme.success),
        TaskStatus::InProgress => ("~", theme.warning),
        TaskStatus::Pending => (" ", theme.dim),
    };
    let bracket = override_style.unwrap_or(Style::default().fg(theme.comment));
    let mark_style = override_style.unwrap_or(Style::default().fg(color));
    vec![
        Span::styled("[", bracket),
        Span::styled(mark, mark_style),
        Span::styled("]", bracket),
    ]
}

/// A block-drawing progress bar: `████████░░░░`.
///
/// Preferred over ratatui's `Gauge` for inline use — `Gauge` fills its whole rect with a
/// background colour, which reads as a solid slab next to flat text.
pub fn text_gauge<'a>(theme: &Theme, ratio: f64, width: usize, fill: Color) -> Vec<Span<'a>> {
    let width = width.max(1);
    let filled = ((ratio.clamp(0.0, 1.0) * width as f64).round() as usize).min(width);
    vec![
        Span::styled("█".repeat(filled), Style::default().fg(fill)),
        Span::styled(
            "░".repeat(width - filled),
            Style::default().fg(theme.progress_dim),
        ),
    ]
}

/// A full-height vertical rule, used as the gutter between side-by-side columns.
pub fn vertical_rule(f: &mut Frame, theme: &Theme, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let pipe = Span::styled("│", Style::default().fg(theme.panel_border));
    let lines: Vec<Line> = (0..area.height).map(|_| Line::from(pipe.clone())).collect();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

/// A `key:  value` metadata row with a lowercase, fixed-width key column.
pub fn meta_row<'a>(theme: &Theme, key: &str, value: String, value_color: Color) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{:<11}", format!("{}:", key.to_lowercase())),
            Style::default().fg(theme.dim),
        ),
        Span::styled(value, Style::default().fg(value_color)),
    ])
}

/// A `#tag` pill. Flat with a coloured hash rather than a filled background.
pub fn tag_span<'a>(theme: &Theme, tag: &str) -> Vec<Span<'a>> {
    vec![
        Span::styled("#", Style::default().fg(theme.comment)),
        Span::styled(tag.to_string(), Style::default().fg(theme.info)),
    ]
}

/// A selectable `[30d]` toggle from a row of options.
pub fn bracket_toggle<'a>(theme: &Theme, label: &str, selected: bool) -> Vec<Span<'a>> {
    let (bracket, text) = if selected {
        (
            Style::default().fg(theme.accent),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            Style::default().fg(theme.comment),
            Style::default().fg(theme.dim),
        )
    };
    vec![
        Span::styled("[", bracket),
        Span::styled(label.to_string(), text),
        Span::styled("]", bracket),
    ]
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

//! Statistics dashboard — heatmap, summary panel, weekly bar chart, recent sessions.

use chrono::Timelike;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, List, ListItem, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, Theme};
use crate::ui::IconSet;

use super::heatmap;
use super::widgets::{format_minutes, section_panel, section_panel_bottom};

// ── Main entry point ─────────────────────────────────────────────────────────

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(1),
            Constraint::Min(6),
        ])
        .split(area);

    draw_heatmap_section(f, app, rows[0]);
    draw_divider(f, rows[1], theme);

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Length(1),
            Constraint::Percentage(33),
            Constraint::Length(1),
            Constraint::Percentage(34),
        ])
        .split(rows[2]);

    draw_summary(f, app, bottom_cols[0]);
    draw_vdivider(f, bottom_cols[1], theme);
    if app.stats.stats_view_mode == crate::app::StatsViewMode::Overview {
        draw_week_bars(f, app, bottom_cols[2]);
    } else {
        draw_tag_analytics(f, app, bottom_cols[2]);
    }
    draw_vdivider(f, bottom_cols[3], theme);
    draw_recent_sessions(f, app, bottom_cols[4]);
}

// ── Dividers ─────────────────────────────────────────────────────────────────

fn draw_divider(f: &mut Frame, area: Rect, theme: &Theme) {
    let border = Style::default().fg(theme.panel_border);
    f.render_widget(
        Paragraph::new(Span::styled("─".repeat(area.width as usize), border)),
        area,
    );
}

fn draw_vdivider(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height == 0 {
        return;
    }
    let border = Style::default().fg(theme.panel_border);
    let pipe = Span::styled("│", border);
    let lines: Vec<Line> = (0..area.height).map(|_| Line::from(pipe.clone())).collect();
    f.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

// ── Heatmap section ──────────────────────────────────────────────────────────

fn draw_heatmap_section(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let block = section_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} Focus activity ", icons.calendar),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let today_mins = app.today_focus_mins();
    heatmap::draw_focus_heatmap(
        f,
        inner,
        theme,
        icons,
        &app.stats.heatmap_data,
        app.data.daily_goal_minutes,
        today_mins,
        app.stats.heatmap_cursor,
    );

    render_bottom_cap(f, theme, area);
}

// ── Summary panel ────────────────────────────────────────────────────────────

fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let (focus_n, custom_n, break_n) = app.stats.session_counts;
    let today = app.today_focus_mins();

    let dim_style = Style::default().fg(theme.dim);
    let val_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);

    let summary_rows: [(&str, &str, String); 6] = [
        (
            icons.fire,
            "Streak",
            format!(
                "{}d / {}d goal",
                app.data.streak_days, app.data.goal_streak_days
            ),
        ),
        (
            icons.timer,
            "Sessions",
            format!("{focus_n}p · {custom_n}c · {break_n}b"),
        ),
        (
            icons.chart,
            "Total",
            format_minutes(app.data.total_focus_minutes),
        ),
        (
            icons.star,
            "Peak Time",
            crate::storage::most_productive_hour_label(&app.db),
        ),
        (
            icons.heart,
            "Focus score",
            format!("{}%", crate::storage::focus_score(&app.data)),
        ),
        (
            icons.fire,
            "Streaks",
            format!(
                "{}d · {}w · {}mo",
                app.data.streak_days, app.data.weekly_streak_weeks, app.data.monthly_streak_months
            ),
        ),
    ];

    let block = section_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} Summary ", icons.stats),
            Style::default().fg(theme.accent),
        )),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let table_rows: Vec<Row> = summary_rows
        .iter()
        .map(|(icon, label, value)| {
            Row::new([
                Cell::from(Span::styled(format!("{icon} {label}"), dim_style)),
                Cell::from(Span::styled(value.as_str(), val_style)),
            ])
        })
        .collect();

    let table = Table::new(table_rows, [Constraint::Length(14), Constraint::Min(6)]);
    f.render_widget(table, layout[0]);

    let goal_mins = app.data.daily_goal_minutes.max(1) as f64;
    let percent = (today as f64 / goal_mins).clamp(0.0, 1.0);

    let gauge = ratatui::widgets::Gauge::default()
        .gauge_style(Style::default().fg(theme.accent).bg(theme.progress_dim))
        .percent((percent * 100.0) as u16)
        .label(format!(
            "{} / {}",
            format_minutes(today),
            format_minutes(app.data.daily_goal_minutes)
        ));

    f.render_widget(gauge, layout[2]);

    if inner.height > summary_rows.len() as u16 + 2 {
        let timeline_area = Rect {
            x: inner.x,
            y: inner.y + summary_rows.len() as u16 + 2,
            width: inner.width,
            height: 1,
        };
        draw_daily_timeline(f, app, timeline_area);
    }

    render_bottom_cap(f, theme, area);
}

// ── Weekly bar chart ─────────────────────────────────────────────────────────

fn draw_week_bars(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let data = &app.stats.weekly_chart;

    let block = section_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} This week ", icons.chart),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    if data.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No sessions this week",
                Style::default().fg(theme.dim),
            ))
            .alignment(Alignment::Center),
            inner,
        );
    } else {
        render_week_chart(f, theme, icons, data, inner);
    }

    render_bottom_cap(f, theme, area);
}

/// Core bar-chart rendering extracted for clarity.
fn render_week_chart(
    f: &mut Frame,
    theme: &Theme,
    icons: IconSet,
    data: &[(String, u32)],
    inner: Rect,
) {
    if data.is_empty() {
        f.render_widget(
            Paragraph::new("No focus data yet")
                .alignment(Alignment::Center)
                .style(Style::default().fg(theme.dim)),
            inner,
        );
        return;
    }

    let max_mins = data.iter().map(|(_, m)| *m).max().unwrap_or(1).max(1);
    let last_idx = data.len() - 1;
    let total_mins: u32 = data.iter().map(|(_, m)| *m).sum();
    let avg_mins = total_mins / data.len() as u32;

    // Layout: "▸DAY ████████░░░░ XXm"
    const LABEL_W: usize = 4;
    const MINS_W: usize = 6;
    let bar_max = (inner.width as usize)
        .saturating_sub(LABEL_W + MINS_W + 2)
        .max(4);

    // Show the most recent days so today is always visible.
    let visible_days = (inner.height as usize)
        .saturating_sub(3)
        .min(7)
        .min(data.len());
    let start_idx = data.len() - visible_days;

    // Pre-compute shared styles.
    let today_style = Style::default()
        .fg(theme.success)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(theme.dim);
    let text_style = Style::default().fg(theme.text);
    let track_style = Style::default().fg(theme.progress_dim);
    let hidden_marker = Style::default().fg(theme.bg);

    let mut lines = Vec::with_capacity(visible_days + 2);

    for (idx, (day_label, mins)) in data.iter().enumerate().skip(start_idx) {
        let mins = *mins;
        let is_today = idx == last_idx;

        let fill = ((mins as u64 * bar_max as u64) / max_mins as u64) as usize;
        let empty = bar_max - fill;

        let (day_style, mins_style, bar_fg, marker, marker_style) = if is_today {
            (today_style, today_style, theme.success, "▸", today_style)
        } else {
            (dim_style, text_style, theme.accent, " ", hidden_marker)
        };

        lines.push(Line::from(vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{:<3} ", day_label), day_style),
            Span::styled("█".repeat(fill), Style::default().fg(bar_fg)),
            Span::styled("░".repeat(empty), track_style),
            Span::styled(format!(" {:>4}", format_minutes(mins)), mins_style),
        ]));
    }

    // Summary footer.
    if inner.height as usize > visible_days + 1 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", icons.chart),
                Style::default().fg(theme.accent),
            ),
            Span::styled(
                format!("{} total", format_minutes(total_mins)),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}  ", icons.dot),
                Style::default().fg(theme.panel_border),
            ),
            Span::styled(format!("~{} avg/day", format_minutes(avg_mins)), dim_style),
        ]));
    }

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}

// ── Recent sessions ──────────────────────────────────────────────────────────

fn draw_recent_sessions(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let (title, data) = if let Some(cursor) = app.stats.heatmap_cursor {
        (
            format!(
                " {} Sessions on {} ",
                icons.calendar,
                cursor.format("%b %d")
            ),
            &app.stats.cursor_sessions,
        )
    } else {
        (
            format!(" {} Today's Timeline ", icons.calendar),
            &app.stats.timeline_sessions,
        )
    };

    let block = section_panel(
        theme,
        Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = if data.is_empty() {
        vec![ListItem::new(Span::styled(
            "No sessions yet",
            Style::default().fg(theme.dim),
        ))]
    } else {
        let dim_style = Style::default().fg(theme.dim);
        let normal_style = Style::default().fg(theme.text);
        let selected_style = Style::default()
            .fg(theme.select_fg)
            .bg(theme.select_bg)
            .add_modifier(Modifier::BOLD);
        let mins_style = Style::default().fg(theme.success);

        data.iter()
            .take(inner.height as usize)
            .enumerate()
            .map(|(idx, s)| {
                let style = if idx == app.stats.stats_session_selected {
                    selected_style
                } else {
                    normal_style
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        s.record.completed_at.format("%H:%M ").to_string(),
                        dim_style,
                    ),
                    Span::styled(format!("{}m ", s.record.minutes), mins_style),
                    Span::styled(session_task_label(app, s.record.task_id), style),
                ]))
                .style(style)
            })
            .collect()
    };

    f.render_widget(List::new(items), inner);

    render_bottom_cap(f, theme, area);
}

// ── Tag Analytics ────────────────────────────────────────────────────────────

fn draw_tag_analytics(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let block = section_panel(
        theme,
        Line::from(Span::styled(
            format!(" {} Tag Analytics ", icons.chart),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.stats.tag_analytics.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No tagged sessions (30d)",
                Style::default().fg(theme.dim),
            ))
            .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let bars: Vec<(&str, u64)> = app
        .stats.tag_analytics
        .iter()
        .take(10)
        .map(|(k, v)| (k.as_str(), *v as u64))
        .collect();

    let chart = ratatui::widgets::BarChart::default()
        .data(&bars)
        .bar_width(6)
        .bar_gap(2)
        .bar_style(Style::default().fg(theme.accent))
        .value_style(
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .label_style(Style::default().fg(theme.text));

    f.render_widget(chart, inner);
    render_bottom_cap(f, theme, area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

#[allow(clippy::needless_range_loop)]
fn draw_daily_timeline(f: &mut Frame, app: &App, area: Rect) {
    if area.height < 1 {
        return;
    }
    let theme = &app.theme;
    let width = area.width as usize;
    if width < 8 {
        return;
    }

    let prefix_len = 6; // length of "Today "
    let timeline_w = width.saturating_sub(prefix_len);
    if timeline_w == 0 {
        return;
    }

    let mut blocks = vec!['·'; timeline_w];
    let mins_per_day = 24.0 * 60.0;

    for s in &app.stats.timeline_sessions {
        let end_time = s.record.completed_at.with_timezone(&chrono::Local);
        let end_mins = (end_time.hour() * 60 + end_time.minute()) as f64;
        let start_mins = (end_mins - s.record.minutes as f64).max(0.0);

        let start_idx = ((start_mins / mins_per_day) * timeline_w as f64).floor() as usize;
        let end_idx = ((end_mins / mins_per_day) * timeline_w as f64).ceil() as usize;

        let start_idx = start_idx.clamp(0, timeline_w.saturating_sub(1));
        let end_idx = end_idx.clamp(start_idx, timeline_w.saturating_sub(1));

        let char_to_draw = match s.record.mode {
            crate::model::TimerMode::Focus | crate::model::TimerMode::Custom => '█',
            _ => '░',
        };

        for i in start_idx..=end_idx {
            // Only overwrite empty space or breaks, so focus blocks take priority visually
            if blocks[i] == '·' || (blocks[i] == '░' && char_to_draw == '█') {
                blocks[i] = char_to_draw;
            }
        }
    }
    let line = Line::from(vec![
        Span::styled("Today ", Style::default().fg(theme.dim)),
        Span::styled(
            blocks.into_iter().collect::<String>(),
            Style::default().fg(theme.accent),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

/// Renders the bottom border cap shared by all panel sections.
#[inline]
fn render_bottom_cap(f: &mut Frame, theme: &Theme, area: Rect) {
    let bottom = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    f.render_widget(section_panel_bottom(theme), bottom);
}

/// Resolves a task ID to a truncated display label.
fn session_task_label(app: &App, task_id: Option<u64>) -> String {
    match task_id {
        None => "general".into(),
        Some(id) => app
            .data
            .task(id)
            .map(|t| super::widgets::truncate(&t.title, 16))
            .unwrap_or_else(|| "?".into()),
    }
}

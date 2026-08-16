//! Statistics dashboard — heatmap, summary panel, weekly bar chart, recent sessions.

use chrono::{Datelike, Timelike};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, List, ListItem, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, Theme};
use crate::ui::IconSet;

use super::heatmap;
use super::widgets::{
    bracket_toggle, comment_line, comment_span, dense_panel, format_minutes, section_title,
};

// ── Main entry point ─────────────────────────────────────────────────────────

pub fn draw_stats(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Length(1),
            Constraint::Percentage(50),
        ])
        .split(area);

    draw_heatmap_section(f, app, rows[0]);
    draw_divider(f, rows[1], theme);

    // Three-wide gutters, matching the tasks tab: a bare one-column rule left the gauge and
    // the timeline butted straight up against the divider.
    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Length(3),
            Constraint::Percentage(33),
            Constraint::Length(3),
            Constraint::Percentage(34),
        ])
        .split(rows[2]);

    draw_summary(f, app, bottom_cols[0]);
    draw_vdivider(f, bottom_cols[1], theme);
    match app.stats.stats_view_mode {
        crate::app::StatsViewMode::Overview => draw_week_bars(f, app, bottom_cols[2]),
        crate::app::StatsViewMode::Analytics => draw_tag_analytics(f, app, bottom_cols[2]),
        crate::app::StatsViewMode::Weekday => draw_weekday_breakdown(f, app, bottom_cols[2]),
        crate::app::StatsViewMode::Hourly => draw_hourly_breakdown(f, app, bottom_cols[2]),
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
    super::widgets::vertical_rule(f, theme, area);
}

/// Interactive tab toggles for the middle statistics panel: `[v] [week] [tags] [weekday] [hourly]`
fn stats_submode_tabs(
    theme: &Theme,
    current: crate::app::StatsViewMode,
    width: u16,
) -> Line<'static> {
    if width < 56 {
        return Line::from(vec![
            Span::styled("[v] ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("[{}]", current.label()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]);
    }

    let mut spans = Vec::with_capacity(16);
    spans.push(Span::styled("[v] ", Style::default().fg(theme.dim)));
    for mode in crate::app::StatsViewMode::all() {
        spans.extend(bracket_toggle(theme, mode.label(), mode == current));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

// ── Heatmap section ──────────────────────────────────────────────────────────

fn draw_heatmap_section(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    // Range presets live in the panel title: `[7d] [30d] [90d] …`.
    let range_spans = if area.width < 56 {
        vec![
            Span::styled("[r] ", Style::default().fg(theme.dim)),
            Span::styled(
                format!("[{}]", app.stats.stats_range.label()),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]
    } else {
        let mut spans = Vec::with_capacity(16);
        for range in crate::app::StatsRange::all() {
            spans.extend(bracket_toggle(
                theme,
                range.label(),
                range == app.stats.stats_range,
            ));
            spans.push(Span::raw(" "));
        }
        spans
    };

    let block = dense_panel(
        theme,
        section_title(theme, icons.calendar, "Focus activity"),
    )
    .title(Line::from(range_spans).alignment(Alignment::Right));
    let inner = block.inner(area);
    f.render_widget(block, area);

    heatmap::draw_focus_heatmap(
        f,
        inner,
        theme,
        icons,
        &app.stats.heatmap_data,
        heatmap::HeatmapOptions {
            goal: app.data.daily_goal_minutes,
            today_live_mins: app.today_focus_mins(),
            cursor: app.stats.heatmap_cursor,
            max_weeks: app.stats.stats_range.weeks(),
        },
    );
}

// ── Summary panel ────────────────────────────────────────────────────────────

fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let (focus_n, custom_n, break_n) = app.stats.session_counts;
    let today = app.today_focus_mins();

    let dim_style = Style::default().fg(theme.dim);
    let val_style = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);

    // "Streak" and "Streaks" both reported `streak_days`, so the same number appeared
    // twice under near-identical labels. Daily/weekly/monthly now share one row and the
    // goal streak gets its own.
    let summary_rows: [(&str, &str, String); 7] = [
        (
            icons.fire,
            "streak",
            format!(
                "{}d · {}w · {}mo",
                app.data.streak_days, app.data.weekly_streak_weeks, app.data.monthly_streak_months
            ),
        ),
        (
            icons.target,
            "goal streak",
            format!("{}d", app.data.goal_streak_days),
        ),
        (
            icons.shield,
            "freezes",
            format!(
                "{}/{}",
                app.data.streak_freezes,
                crate::model::STREAK_FREEZE_MAX
            ),
        ),
        (
            icons.timer,
            "sessions",
            format!("{focus_n}p · {custom_n}c · {break_n}b"),
        ),
        (
            icons.chart,
            "total",
            format_minutes(app.data.total_focus_minutes),
        ),
        (icons.star, "peak", app.stats.peak_hour_label.clone()),
        (
            icons.heart,
            "score",
            format!("{}%", crate::storage::focus_score(&app.data)),
        ),
    ];

    let block = dense_panel(theme, section_title(theme, icons.stats, "Summary"));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Rows, gauge, timeline. The timeline used to be positioned by a hand-built `Rect` at a
    // fixed offset from the top, which left it stranded in the middle of the panel with a
    // gap above it whenever the panel was taller than the rows needed.
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_rows.len() as u16),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
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

    // The label column gives way on a narrow panel; fixed at 14 it left twelve columns for
    // values like "0d · 0w · 0mo", which clipped.
    let label_w = 14u16.min(inner.width.saturating_sub(13));
    let table = Table::new(
        table_rows,
        [Constraint::Length(label_w), Constraint::Min(6)],
    );
    f.render_widget(table, layout[0]);

    let goal_mins = app.data.daily_goal_minutes.max(1) as f64;
    let percent = (today as f64 / goal_mins).clamp(0.0, 1.0);
    let goal_met = today as f64 >= goal_mins;

    let label = format!(
        " {} / {}",
        format_minutes(today),
        format_minutes(app.data.daily_goal_minutes)
    );
    let bar_w = (layout[2].width as usize).saturating_sub(label.chars().count());
    let fill = if goal_met {
        theme.success
    } else {
        theme.accent
    };
    let mut spans = super::widgets::text_gauge(theme, percent, bar_w, fill);
    spans.push(Span::styled(
        label,
        Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
    ));
    f.render_widget(Paragraph::new(Line::from(spans)), layout[2]);
    draw_daily_timeline(f, app, layout[3]);
}

// ── Weekly bar chart ─────────────────────────────────────────────────────────

fn draw_week_bars(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;
    let data = &app.stats.weekly_chart;

    let block = dense_panel(theme, section_title(theme, icons.chart, "Last 7 days")).title(
        stats_submode_tabs(theme, app.stats.stats_view_mode, area.width)
            .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Seven full-width empty tracks read as a grey slab punched into the panel, not as a
    // chart of zeroes — so when there is nothing to plot, say so instead of drawing them.
    if data.is_empty() || data.iter().all(|(_, m)| *m == 0) {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                comment_line(theme, "no focus logged in the last 7 days"),
            ]),
            inner,
        );
    } else {
        render_week_chart(f, theme, icons, data, inner);
    }
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
            Paragraph::new(comment_line(theme, "no focus data yet")).alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let max_mins = data.iter().map(|(_, m)| *m).max().unwrap_or(1).max(1);
    let last_idx = data.len() - 1;
    let total_mins: u32 = data.iter().map(|(_, m)| *m).sum();
    let avg_mins = total_mins / data.len() as u32;

    // Layout: "▸day ████████░░░░  XXm". The bar is capped so the value column stays beside
    // it — bound only by the panel, a wide column stretched the track into a grey band with
    // the minutes stranded an inch away from it.
    const LABEL_W: usize = 4;
    const MINS_W: usize = 6;
    const BAR_W_MAX: usize = 32;
    let bar_max = (inner.width as usize)
        .saturating_sub(LABEL_W + MINS_W + 2)
        .clamp(4, BAR_W_MAX);

    // Show the most recent days so today is always visible.
    let visible_days = (inner.height as usize)
        .saturating_sub(3)
        .min(7)
        .min(data.len());
    if visible_days == 0 {
        return;
    }
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
            Span::styled(format!("{:<3} ", day_label.to_lowercase()), day_style),
            Span::styled("█".repeat(fill), Style::default().fg(bar_fg)),
            Span::styled("░".repeat(empty), track_style),
            Span::styled(format!(" {:>6}", format_minutes(mins)), mins_style),
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
                format!("{} total  ", format_minutes(total_mins)),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            comment_span(theme, format!("~{} avg/day", format_minutes(avg_mins))),
        ]));
    }

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}

// ── Recent sessions ──────────────────────────────────────────────────────────

fn draw_recent_sessions(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    // Must stay in lockstep with `App::active_stats_sessions` — the j/k/d/+/- keys act on
    // that list, so rendering anything else edits a session the user cannot see.
    let data = app.active_stats_sessions();
    let title = if let Some(cursor) = app.stats.heatmap_cursor {
        format!(
            " {} sessions on {} ",
            icons.calendar,
            cursor.format("%b %-d")
        )
    } else {
        let pages = app
            .stats
            .stats_session_total
            .div_ceil(crate::app::App::SESSIONS_PER_PAGE)
            .max(1);
        if pages > 1 {
            format!(
                " {} recent sessions [{}/{}] ",
                icons.calendar,
                app.stats.stats_session_page + 1,
                pages
            )
        } else {
            format!(" {} recent sessions ", icons.calendar)
        }
    };

    let block = dense_panel(
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
        let hint = if app.stats.heatmap_cursor.is_some() {
            "nothing logged on this day · [esc] to clear"
        } else {
            "no sessions yet · [←↑↓→] to pick a day"
        };
        vec![ListItem::new(comment_line(theme, hint))]
    } else {
        let dim_style = Style::default().fg(theme.dim);
        let normal_style = Style::default().fg(theme.text);
        let selected_style = Style::default()
            .fg(theme.select_fg)
            .bg(theme.select_bg)
            .add_modifier(Modifier::BOLD);
        let mins_style = Style::default().fg(theme.success);

        // Scroll the window so the selection stays on screen; the selection index runs over
        // the whole list, not just the rows that happen to fit.
        let visible = (inner.height as usize).max(1);
        let selected = app.stats.stats_session_selected;
        let start = selected
            .saturating_sub(visible - 1)
            .min(data.len().saturating_sub(visible));

        data.iter()
            .enumerate()
            .skip(start)
            .take(visible)
            .map(|(idx, s)| {
                let style = if idx == selected {
                    selected_style
                } else {
                    normal_style
                };

                ListItem::new(Line::from(vec![
                    Span::styled(s.record.completed_at.format("%H:%M").to_string(), dim_style),
                    Span::styled(format!("{:>5}m ", s.record.minutes), mins_style),
                    Span::styled(session_task_label(app, s.record.task_id), style),
                ]))
                .style(style)
            })
            .collect()
    };

    f.render_widget(List::new(items), inner);
}

// ── Day-of-week breakdown ────────────────────────────────────────────────────

const WEEKDAY_LABELS: [&str; 7] = ["mo", "tu", "we", "th", "fr", "sa", "su"];

/// Average focus minutes per weekday over the selected range, plus the best and worst day.
/// Derived from the already-loaded heatmap data, so this costs no extra query.
fn weekday_averages(app: &App) -> [u32; 7] {
    let today = crate::date::today_naive();
    let cutoff = app
        .stats
        .stats_range
        .days()
        .map(|d| today - chrono::Duration::days(d as i64 - 1));

    let mut totals = [0u64; 7];
    let mut counts = [0u64; 7];

    for (date_str, mins) in &app.stats.heatmap_data {
        let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
            continue;
        };
        if date > today || cutoff.is_some_and(|c| date < c) {
            continue;
        }
        let idx = date.weekday().num_days_from_monday() as usize;
        totals[idx] += *mins as u64;
        counts[idx] += 1;
    }

    let mut out = [0u32; 7];
    for i in 0..7 {
        out[i] = totals[i].checked_div(counts[i]).unwrap_or(0) as u32;
    }
    out
}

fn draw_weekday_breakdown(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let block = dense_panel(theme, section_title(theme, icons.calendar, "Day of week")).title(
        stats_submode_tabs(theme, app.stats.stats_view_mode, area.width)
            .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let averages = weekday_averages(app);
    let peak = averages.iter().copied().max().unwrap_or(0);

    if peak == 0 {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                comment_line(theme, "no focus data in range"),
            ]),
            inner,
        );
        return;
    }

    const LABEL_W: usize = 3;
    const MINS_W: usize = 7;
    let bar_max = (inner.width as usize)
        .saturating_sub(LABEL_W + MINS_W + 1)
        .max(4);

    let today_idx = crate::date::today_naive().weekday().num_days_from_monday() as usize;
    let dim_style = Style::default().fg(theme.dim);
    let track_style = Style::default().fg(theme.progress_dim);

    let mut lines: Vec<Line> = Vec::with_capacity(10);
    for (idx, &mins) in averages.iter().enumerate() {
        let fill = ((mins as u64 * bar_max as u64) / peak as u64) as usize;
        let is_today = idx == today_idx;
        let label_style = if is_today {
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD)
        } else {
            dim_style
        };
        let bar_fg = if is_today {
            theme.success
        } else {
            theme.accent
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{:<3}", WEEKDAY_LABELS[idx]), label_style),
            Span::styled("█".repeat(fill), Style::default().fg(bar_fg)),
            Span::styled("░".repeat(bar_max - fill), track_style),
            Span::styled(
                format!(" {:>6}", format_minutes(mins)),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    // Weekday vs Weekend averages
    let weekday_avg: u32 = averages[0..5].iter().sum::<u32>() / 5;
    let weekend_avg: u32 = averages[5..7].iter().sum::<u32>() / 2;

    let tracked: Vec<(usize, u32)> = averages
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, m)| *m > 0)
        .collect();

    if inner.height >= 9 {
        lines.push(Line::from(""));
        if tracked.len() > 1 {
            let best = tracked.iter().max_by_key(|(_, m)| *m).unwrap();
            lines.push(Line::from(comment_span(
                theme,
                format!(
                    "weekday avg {} · weekend {} · best {}",
                    format_minutes(weekday_avg),
                    format_minutes(weekend_avg),
                    WEEKDAY_LABELS[best.0],
                ),
            )));
        } else {
            lines.push(Line::from(comment_span(
                theme,
                format!(
                    "weekday avg {} · weekend avg {}",
                    format_minutes(weekday_avg),
                    format_minutes(weekend_avg),
                ),
            )));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ── Tag Analytics (Ranked Horizontal Bar List) ───────────────────────────────

fn draw_tag_analytics(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let block = dense_panel(theme, section_title(theme, icons.chart, "Tag analytics")).title(
        stats_submode_tabs(theme, app.stats.stats_view_mode, area.width)
            .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.stats.tag_analytics.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                comment_line(theme, "no tagged sessions (30d)"),
            ]),
            inner,
        );
        return;
    }

    let total_mins: u32 = app.stats.tag_analytics.iter().map(|(_, m)| *m).sum();
    let max_mins = app
        .stats
        .tag_analytics
        .iter()
        .map(|(_, m)| *m)
        .max()
        .unwrap_or(1)
        .max(1);

    const LABEL_W: usize = 10;
    const MINS_W: usize = 12; // " 100%  99h 59m"
    let bar_max = (inner.width as usize)
        .saturating_sub(LABEL_W + MINS_W + 2)
        .clamp(4, 28);

    let visible_tags = (inner.height as usize)
        .saturating_sub(3)
        .min(7)
        .min(app.stats.tag_analytics.len());

    let dim_style = Style::default().fg(theme.dim);
    let text_style = Style::default().fg(theme.text);
    let track_style = Style::default().fg(theme.progress_dim);

    let mut lines = Vec::with_capacity(visible_tags + 2);

    for (tag, mins) in app.stats.tag_analytics.iter().take(visible_tags) {
        let mins = *mins;
        let fill = ((mins as u64 * bar_max as u64) / max_mins as u64) as usize;
        let empty = bar_max - fill;
        let pct = if total_mins > 0 {
            (mins as u64 * 100 / total_mins as u64).min(100)
        } else {
            0
        };

        let tag_display = if tag.starts_with('#') {
            tag.clone()
        } else {
            format!("#{tag}")
        };
        let tag_truncated = super::widgets::truncate(&tag_display, LABEL_W);

        lines.push(Line::from(vec![
            Span::styled(
                format!("{:<width$} ", tag_truncated, width = LABEL_W),
                dim_style,
            ),
            Span::styled("█".repeat(fill), Style::default().fg(theme.accent)),
            Span::styled("░".repeat(empty), track_style),
            Span::styled(
                format!(" {:>3}% {:>6}", pct, format_minutes(mins)),
                text_style,
            ),
        ]));
    }

    if inner.height as usize > visible_tags + 1 {
        lines.push(Line::from(""));
        let top_tag = app
            .stats
            .tag_analytics
            .first()
            .map(|(t, _)| t.as_str())
            .unwrap_or("none");
        lines.push(Line::from(comment_span(
            theme,
            format!(
                "{} tags active · top: #{} ({})",
                app.stats.tag_analytics.len(),
                top_tag.trim_start_matches('#'),
                format_minutes(total_mins),
            ),
        )));
    }

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
}

// ── Time-of-Day / Hourly Breakdown ──────────────────────────────────────────

fn draw_hourly_breakdown(f: &mut Frame, app: &App, area: Rect) {
    let theme = &app.theme;
    let icons = app.icons;

    let block = dense_panel(theme, section_title(theme, icons.timer, "Time of day")).title(
        stats_submode_tabs(theme, app.stats.stats_view_mode, area.width)
            .alignment(Alignment::Right),
    );
    let inner = block.inner(area);
    f.render_widget(block, area);

    let hours = &app.stats.hourly_distribution;
    let total_mins: u32 = hours.iter().sum();

    if total_mins == 0 {
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                comment_line(theme, "no focus history recorded"),
            ]),
            inner,
        );
        return;
    }

    // 4 productivity quadrants:
    // Morning (06:00..12:00)
    // Afternoon (12:00..18:00)
    // Evening (18:00..22:00)
    // Night (22:00..06:00)
    let morning: u32 = hours[6..12].iter().sum();
    let afternoon: u32 = hours[12..18].iter().sum();
    let evening: u32 = hours[18..22].iter().sum();
    let night: u32 = hours[22..24].iter().sum::<u32>() + hours[0..6].iter().sum::<u32>();

    let quadrants = [
        ("morning", morning),
        ("afternoon", afternoon),
        ("evening", evening),
        ("night", night),
    ];

    let max_quadrant = quadrants.iter().map(|(_, m)| *m).max().unwrap_or(1).max(1);

    const LABEL_W: usize = 10; // "morning   "
    const MINS_W: usize = 12; // " 100%  99h 59m"
    let bar_max = (inner.width as usize)
        .saturating_sub(LABEL_W + MINS_W + 2)
        .clamp(4, 28);

    let dim_style = Style::default().fg(theme.dim);
    let text_style = Style::default().fg(theme.text);
    let track_style = Style::default().fg(theme.progress_dim);

    let mut lines = Vec::with_capacity(7);

    for (name, mins) in quadrants {
        let fill = ((mins as u64 * bar_max as u64) / max_quadrant as u64) as usize;
        let empty = bar_max - fill;
        let pct = if total_mins > 0 {
            (mins as u64 * 100 / total_mins as u64).min(100)
        } else {
            0
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{:<width$} ", name, width = LABEL_W), dim_style),
            Span::styled("█".repeat(fill), Style::default().fg(theme.accent)),
            Span::styled("░".repeat(empty), track_style),
            Span::styled(
                format!(" {:>3}% {:>6}", pct, format_minutes(mins)),
                text_style,
            ),
        ]));
    }

    if inner.height >= 7 {
        lines.push(Line::from(""));
        let peak_quad = quadrants
            .iter()
            .max_by_key(|(_, m)| *m)
            .map(|(n, _)| *n)
            .unwrap_or("morning");
        lines.push(Line::from(comment_span(
            theme,
            format!(
                "peak window: {} · peak hour: {}",
                peak_quad, app.stats.peak_hour_label,
            ),
        )));
    }

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), inner);
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
        Span::styled("today ", Style::default().fg(theme.dim)),
        Span::styled(
            blocks.into_iter().collect::<String>(),
            Style::default().fg(theme.accent),
        ),
    ]);
    f.render_widget(Paragraph::new(line), area);
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

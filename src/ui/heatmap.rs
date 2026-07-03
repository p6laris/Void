//! GitHub-style focus heatmap rendered in the terminal.
//!
//! Seven rows (Mon–Sun), one column per week, one cell per day.
//! Left → right = older → newer; the rightmost column is the current week.

use chrono::{Datelike, Duration, NaiveDate};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::Theme;
use crate::ui::IconSet;

use super::widgets::format_minutes;

const CELL: &str = "■";
const LABEL_COL: usize = 4;
const GAP: usize = 1;
const MIN_MONTH_LABEL_GAP: usize = 4;
const DAYS_PER_WEEK: usize = 7;
const DAY_LABELS: [&str; DAYS_PER_WEEK] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

struct HeatmapLayout {
    weeks: usize,
    cell_w: usize,
    stride: usize,
}

impl HeatmapLayout {
    fn build(width: usize) -> Self {
        let cell_w = UnicodeWidthStr::width(CELL).max(1);
        let stride = cell_w + GAP;
        let weeks = calc_weeks(width, stride);
        Self {
            weeks,
            cell_w,
            stride,
        }
    }

    fn week_x(&self, col: usize) -> usize {
        LABEL_COL + col * self.stride
    }
}

#[derive(Clone, Copy)]
enum GridCell {
    Future,
    Day(u32),
}

impl GridCell {
    fn color(self, max_mins: u32, goal: u32, theme: &Theme) -> Color {
        match self {
            Self::Future => theme.bg,
            Self::Day(0) => theme.task_track,
            Self::Day(mins) => heat_color(mins, max_mins, goal, theme),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_focus_heatmap(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    icons: IconSet,
    data: &[(String, u32)],
    goal: u32,
    today_live_mins: u32,
    cursor: Option<NaiveDate>,
) {
    let width = area.width as usize;
    if area.height < 4 || width < 10 {
        f.render_widget(
            Paragraph::new(Span::styled("—", Style::default().fg(theme.dim)))
                .alignment(Alignment::Center),
            area,
        );
        return;
    }

    let layout = HeatmapLayout::build(width);
    let today = crate::date::today_naive();
    let grid_start = monday_of(today) - Duration::days((layout.weeks as i64 - 1) * 7);

    let grid_data = build_grid(data, today, grid_start, layout.weeks, goal, today_live_mins);

    let lines = render_lines(HeatmapRenderContext {
        theme,
        icons,
        layout: &layout,
        grid_start,
        today,
        grid_data: &grid_data,
        goal,
        available_height: area.height as usize,
        cursor,
    });

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
}

struct GridData {
    grid: Vec<[GridCell; 7]>,
    month_marks: Vec<(usize, &'static str)>,
    max_mins: u32,
    total_logged: u32,
}

fn build_grid(
    data: &[(String, u32)],
    today: NaiveDate,
    grid_start: NaiveDate,
    weeks: usize,
    goal: u32,
    today_live_mins: u32,
) -> GridData {
    let mut grid = vec![[GridCell::Future; DAYS_PER_WEEK]; weeks];
    let month_marks = collect_month_marks(grid_start, weeks);
    let mut max_mins = goal.max(1);
    let mut total_logged: u32 = 0;
    let mut date_key_buf = String::with_capacity(10);

    for (col, grid_col) in grid.iter_mut().enumerate().take(weeks) {
        let week_monday = grid_start + Duration::days(col as i64 * 7);

        for (row, cell) in grid_col.iter_mut().enumerate().take(DAYS_PER_WEEK) {
            let date = week_monday + Duration::days(row as i64);

            if date > today {
                *cell = GridCell::Future;
            } else {
                write_date_key(&mut date_key_buf, date);
                let mut mins = lookup_minutes(data, &date_key_buf);
                if date == today {
                    mins = mins.max(today_live_mins);
                }
                max_mins = max_mins.max(mins);
                total_logged += mins;
                *cell = GridCell::Day(mins);
            }
        }
    }

    GridData {
        grid,
        month_marks,
        max_mins,
        total_logged,
    }
}

fn collect_month_marks(grid_start: NaiveDate, weeks: usize) -> Vec<(usize, &'static str)> {
    let mut marks = Vec::new();
    let mut prev_month = Some((grid_start - Duration::days(4)).month());

    for col in 0..weeks {
        let thursday = grid_start + Duration::days(col as i64 * 7 + 3);
        let month = thursday.month();
        if prev_month != Some(month) {
            marks.push((col, month_abbr(month)));
            prev_month = Some(month);
        }
    }

    marks
}

#[inline]
fn month_abbr(month: u32) -> &'static str {
    match month {
        1..=12 => MONTH_ABBR[(month - 1) as usize],
        _ => "?",
    }
}

#[inline]
fn lookup_minutes(data: &[(String, u32)], key: &str) -> u32 {
    data.binary_search_by_key(&key, |(d, _)| d.as_str())
        .map(|idx| data[idx].1)
        .unwrap_or(0)
}

struct HeatmapRenderContext<'a> {
    theme: &'a Theme,
    icons: IconSet,
    layout: &'a HeatmapLayout,
    grid_start: NaiveDate,
    today: NaiveDate,
    grid_data: &'a GridData,
    goal: u32,
    available_height: usize,
    cursor: Option<NaiveDate>,
}

fn render_lines<'a>(ctx: HeatmapRenderContext<'a>) -> Vec<Line<'a>> {
    let HeatmapRenderContext {
        theme,
        icons,
        layout,
        grid_start,
        today,
        grid_data,
        goal,
        available_height,
        cursor,
    } = ctx;
    let dim = Style::default().fg(theme.dim);
    let mut lines = Vec::with_capacity(10);

    lines.push(build_month_row(layout, &grid_data.month_marks, dim));

    for (row_idx, label) in DAY_LABELS.iter().enumerate() {
        let mut spans = Vec::with_capacity(1 + layout.weeks * 2);
        spans.push(Span::styled(pad_label(label), dim));

        for (col, week) in grid_data.grid.iter().enumerate().take(layout.weeks) {
            if col > 0 {
                spans.push(Span::raw(" ".repeat(GAP)));
            }
            let cell = week[row_idx];
            let week_monday = grid_start + Duration::days(col as i64 * 7);
            let date = week_monday + Duration::days(row_idx as i64);
            spans.push(cell_span(
                cell,
                cell.color(grid_data.max_mins, goal, theme),
                layout.cell_w,
                date == today,
                Some(date) == cursor,
                theme,
            ));
        }

        lines.push(Line::from(spans));
    }

    if available_height > lines.len() + 1 {
        lines.push(Line::from(""));
        lines.push(build_legend_row(
            theme,
            icons,
            dim,
            grid_data.max_mins,
            goal,
            grid_data.total_logged,
        ));
    }

    lines
}

fn pad_label(label: &str) -> String {
    let w = UnicodeWidthStr::width(label);
    if w >= LABEL_COL {
        label.to_string()
    } else {
        format!("{label}{}", " ".repeat(LABEL_COL - w))
    }
}

fn cell_span(
    _cell: GridCell,
    mut color: Color,
    cell_w: usize,
    is_today: bool,
    is_cursor: bool,
    theme: &Theme,
) -> Span<'static> {
    let glyph = CELL;
    let w = UnicodeWidthStr::width(glyph);
    let pad = cell_w.saturating_sub(w);

    if is_cursor {
        color = theme.accent;
    }

    let mut style = Style::default().fg(color);
    if is_today {
        style = style.add_modifier(Modifier::BOLD);
    }
    Span::styled(format!("{glyph}{}", " ".repeat(pad)), style)
}

fn build_month_row<'a>(layout: &HeatmapLayout, marks: &[(usize, &str)], dim: Style) -> Line<'a> {
    let mut spans = Vec::with_capacity(marks.len() * 2 + 1);
    spans.push(Span::raw(" ".repeat(LABEL_COL)));

    let mut cursor = LABEL_COL;
    for &(col, label) in marks {
        let target = layout.week_x(col);
        if col > 0 && target < cursor + MIN_MONTH_LABEL_GAP {
            continue;
        }
        if target > cursor {
            spans.push(Span::raw(" ".repeat(target - cursor)));
        }
        spans.push(Span::styled(label.to_owned(), dim));
        cursor = target + UnicodeWidthStr::width(label);
    }

    Line::from(spans)
}

fn build_legend_row<'a>(
    theme: &Theme,
    icons: IconSet,
    dim: Style,
    max_mins: u32,
    goal: u32,
    total_in_range: u32,
) -> Line<'a> {
    let mut spans = Vec::with_capacity(14);
    spans.push(Span::raw(" ".repeat(LABEL_COL)));
    spans.push(Span::styled("Less ", dim));

    let levels = legend_levels(max_mins);
    for (idx, &lvl) in levels.iter().enumerate() {
        let color = heat_color(lvl, max_mins, goal, theme);
        spans.push(Span::styled(CELL, Style::default().fg(color)));
        if idx + 1 < levels.len() {
            spans.push(Span::raw(" "));
        }
    }

    spans.push(Span::styled(" More", dim));
    spans.push(Span::styled(
        format!(
            "   {} peak {} {} {} in range",
            icons.dot,
            format_minutes(max_mins),
            icons.dot,
            format_minutes(total_in_range),
        ),
        dim,
    ));

    Line::from(spans)
}

fn legend_levels(max_mins: u32) -> Vec<u32> {
    let m = max_mins.max(1);
    let mut levels = Vec::with_capacity(5);
    for v in [0, m / 4, m / 2, (m * 3) / 4, m] {
        if levels.last().copied() != Some(v) {
            levels.push(v);
        }
    }
    if levels.len() < 2 {
        levels.push(m);
    }
    levels
}

#[inline]
fn heat_color(mins: u32, max_mins: u32, goal: u32, theme: &Theme) -> Color {
    if mins == 0 {
        return theme.task_track;
    }
    let max = max_mins.max(goal).max(1);
    let ratio = ((mins as u64 * 256) / max as u64).min(256) as u32;

    match ratio {
        0..=63 => blend(theme.task_track, theme.accent, 102),
        64..=127 => blend(theme.task_track, theme.accent, 179),
        128..=191 => theme.accent,
        _ => theme.success,
    }
}

#[inline]
fn blend(a: Color, b: Color, t: u8) -> Color {
    let (ar, ag, ab) = color_rgb(a);
    let (br, bg, bb) = color_rgb(b);
    let t16 = t as u16;
    let inv = 255 - t16;
    Color::Rgb(
        ((ar as u16 * inv + br as u16 * t16) / 255) as u8,
        ((ag as u16 * inv + bg as u16 * t16) / 255) as u8,
        ((ab as u16 * inv + bb as u16 * t16) / 255) as u8,
    )
}

#[inline]
fn color_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::White => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

fn calc_weeks(width: usize, stride: usize) -> usize {
    let usable = width.saturating_sub(LABEL_COL);
    if usable < stride {
        return 1;
    }
    usable / stride
}

#[inline]
fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

#[inline]
fn write_date_key(buf: &mut String, d: NaiveDate) {
    use std::fmt::Write;
    buf.clear();
    let _ = write!(buf, "{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calc_weeks_respects_narrow_width() {
        let stride = UnicodeWidthStr::width(CELL).max(1) + GAP;
        assert_eq!(calc_weeks(10, stride), 3);
        assert_eq!(calc_weeks(8, stride), 2);
    }

    #[test]
    fn month_marks_change_on_month_boundary() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(); // Monday
        let marks = collect_month_marks(start, 8);
        assert_eq!(marks[0], (4, "Feb"));
    }

    #[test]
    fn calc_weeks_edge_cases() {
        let stride = UnicodeWidthStr::width(CELL).max(1) + GAP;
        assert_eq!(calc_weeks(0, stride), 1); // Minimum width 1
        assert_eq!(calc_weeks(5000, stride), 2498);
    }

    #[test]
    fn month_abbr_handles_invalid_month() {
        assert_eq!(month_abbr(1), "Jan");
        assert_eq!(month_abbr(12), "Dec");
        assert_eq!(month_abbr(0), "?");
        assert_eq!(month_abbr(13), "?");
    }
}

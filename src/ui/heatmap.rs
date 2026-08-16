//! GitHub-style focus heatmap rendered in the terminal.
//!
//! Seven rows (Mon–Sun), one column per week, one cell per day.
//! Left → right = older → newer; the rightmost column is the current week.
//!
//! Intensity comes from `Theme::heat`, a five-step single-hue ramp. Both the grid and the
//! legend read that array through [`heat_index`], so the legend can never drift out of
//! sync with the colours actually drawn.

use chrono::{Datelike, Duration, NaiveDate};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

use crate::app::Theme;
use crate::ui::IconSet;

use super::widgets::format_minutes;

const LABEL_COL: usize = 4;
const MIN_MONTH_LABEL_GAP: usize = 4;
const DAYS_PER_WEEK: usize = 7;
/// One year of columns, matching GitHub's contribution graph. Without a cap a wide
/// terminal would render decades of empty grid.
const MAX_WEEKS: usize = 53;
/// GitHub labels only these rows; seven labels at full strength compete with the grid.
const DAY_LABELS: [&str; DAYS_PER_WEEK] = ["Mon", "", "Wed", "", "Fri", "", ""];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Columns one week occupies: the tile glyph plus the gap after it.
const MIN_STRIDE: usize = 2;
/// Widest a week column gets. The tile is one glyph however wide the column is, so past
/// this only the space around it grows — a sparser grid rather than a bigger one. Slack
/// beyond this goes to the month separators instead.
const MAX_STRIDE: usize = 3;
/// Widest separator inserted between months.
///
/// This is where slack goes before the week columns are allowed to spread, but only so far:
/// past about four columns the grid stops reading as one year and starts reading as twelve
/// separate blocks.
const MAX_MONTH_GAP: usize = 4;

/// Rows the heatmap wants: month labels, seven days, a blank, the legend and the caption.
///
/// It is a fixed amount of information, so a caller that gives it every spare row just
/// leaves blank ones under the caption. Let the panels below take the slack instead.
pub const PREFERRED_HEIGHT: u16 = 11;

struct HeatmapLayout {
    weeks: usize,
    stride: usize,
    /// Left edge of each week column, already including the month separators.
    col_x: Vec<usize>,
}

impl HeatmapLayout {
    /// Sizes the grid to the panel and spends whatever is left over on month separators.
    ///
    /// `stride` has to be a whole number of columns, and 53 of them almost never divide a
    /// panel evenly — the old layout floored the division and left the remainder as dead
    /// space at the right edge (19% of a 135-column panel). Handing the remainder to the
    /// month boundaries fills the width, keeps every tile the same size, and groups the
    /// grid the way the month labels above it already imply.
    fn build(width: usize, max_weeks: usize, today: NaiveDate) -> Self {
        let usable = width.saturating_sub(LABEL_COL);
        let want = MAX_WEEKS.min(max_weeks).max(1);
        // The tightest stride that can still reach the right-hand edge once the month
        // separators have taken up the slack. Sizing off `usable / want` instead spread the
        // weeks apart on a wide panel — the tiles stayed one column and only the space
        // around them grew, which is the opposite of what a wide panel should buy you.
        let stride = (MIN_STRIDE..MAX_STRIDE)
            .find(|&stride| {
                let weeks = (usable / stride).min(want).max(1);
                let seams = month_boundaries(grid_start_for(today, weeks), weeks).len();
                weeks * stride + seams * MAX_MONTH_GAP >= usable
            })
            .unwrap_or(MAX_STRIDE);
        let weeks = (usable / stride).min(want).max(1);
        let grid_start = grid_start_for(today, weeks);

        let boundaries: Vec<usize> = month_boundaries(grid_start, weeks);
        let mut spare = usable.saturating_sub(weeks * stride);
        let mut pre_gap = vec![0usize; weeks];

        if !boundaries.is_empty() && spare > 0 {
            let n = boundaries.len();
            let per = (spare / n).min(MAX_MONTH_GAP);
            // Any columns left after an even share go one each to the earliest boundaries,
            // so the separators differ by at most one and the grid still ends flush.
            let extra = if per < MAX_MONTH_GAP {
                (spare - per * n).min(n)
            } else {
                0
            };
            for (i, &col) in boundaries.iter().enumerate() {
                pre_gap[col] = per + usize::from(i < extra);
            }
            spare -= per * n + extra;
        }

        // Whatever could not be spent — a short range, where the grid is far narrower than
        // the panel — is split either side so the grid sits centred instead of stranded
        // against the left edge.
        let mut x = LABEL_COL + spare / 2;
        let mut col_x = Vec::with_capacity(weeks);
        for gap in pre_gap.iter().take(weeks) {
            x += gap;
            col_x.push(x);
            x += stride;
        }

        Self {
            weeks,
            stride,
            col_x,
        }
    }

    fn week_x(&self, col: usize) -> usize {
        self.col_x.get(col).copied().unwrap_or(LABEL_COL)
    }

    /// One past the last column the grid occupies — used to keep month labels inside it.
    fn grid_end(&self) -> usize {
        self.col_x.last().map_or(LABEL_COL, |x| x + self.stride)
    }
}

#[inline]
fn grid_start_for(today: NaiveDate, weeks: usize) -> NaiveDate {
    monday_of(today) - Duration::days((weeks as i64 - 1) * 7)
}

/// Columns (after the first) whose week begins a different month than the one before it.
fn month_boundaries(grid_start: NaiveDate, weeks: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut prev = None;
    for col in 0..weeks {
        let month = (grid_start + Duration::days(col as i64 * 7)).month();
        if prev.is_some() && prev != Some(month) {
            out.push(col);
        }
        prev = Some(month);
    }
    out
}

#[derive(Clone, Copy)]
enum GridCell {
    Future,
    Day(u32),
}

/// Maps minutes onto a step of the `Theme::heat` ramp. Step 0 means "no activity".
#[inline]
fn heat_index(mins: u32, scale: u32) -> usize {
    if mins == 0 {
        return 0;
    }
    let scale = scale.max(1) as u64;
    ((mins as u64 * 4).div_ceil(scale) as usize).clamp(1, crate::theme::HEAT_STEPS - 1)
}

/// Everything the heatmap needs beyond the raw day totals.
#[derive(Clone, Copy)]
pub struct HeatmapOptions {
    /// Daily focus goal, used for the intensity floor and the perfect-day count.
    pub goal: u32,
    /// Minutes accrued today by a running timer, not yet written to the database.
    pub today_live_mins: u32,
    pub cursor: Option<NaiveDate>,
    /// Upper bound on columns, from the selected stats range.
    pub max_weeks: usize,
}

impl Default for HeatmapOptions {
    fn default() -> Self {
        Self {
            goal: 0,
            today_live_mins: 0,
            cursor: None,
            max_weeks: MAX_WEEKS,
        }
    }
}

pub fn draw_focus_heatmap(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    icons: IconSet,
    data: &[(String, u32)],
    opts: HeatmapOptions,
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

    let today = crate::date::today_naive();
    let layout = HeatmapLayout::build(width, opts.max_weeks, today);
    let grid_start = grid_start_for(today, layout.weeks);

    let grid_data = build_grid(
        data,
        today,
        grid_start,
        &layout,
        opts.goal,
        opts.today_live_mins,
        opts.cursor,
    );

    let lines = render_lines(HeatmapRenderContext {
        theme,
        icons,
        layout: &layout,
        grid_start,
        today,
        grid_data: &grid_data,
        available_height: area.height as usize,
        cursor: opts.cursor,
    });

    f.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
}

struct GridData {
    grid: Vec<[GridCell; DAYS_PER_WEEK]>,
    month_marks: Vec<(usize, &'static str)>,
    /// Denominator for the intensity ramp: the busiest day, never below the daily goal so
    /// a single outlier day cannot wash the rest of the grid out.
    scale: u32,
    max_mins: u32,
    total_logged: u32,
    days_tracked: u32,
    perfect_days: u32,
    goal: u32,
    /// Minutes on the cursor's day, so the caption can read it out.
    cursor_mins: Option<u32>,
}

fn build_grid(
    data: &[(String, u32)],
    today: NaiveDate,
    grid_start: NaiveDate,
    layout: &HeatmapLayout,
    goal: u32,
    today_live_mins: u32,
    cursor: Option<NaiveDate>,
) -> GridData {
    let weeks = layout.weeks;
    let mut grid = vec![[GridCell::Future; DAYS_PER_WEEK]; weeks];
    let month_marks = collect_month_marks(grid_start, weeks);
    let mut max_mins = 0;
    let mut total_logged: u32 = 0;
    let mut days_tracked: u32 = 0;
    let mut perfect_days: u32 = 0;
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
                if mins > 0 {
                    days_tracked += 1;
                }
                if goal > 0 && mins >= goal {
                    perfect_days += 1;
                }
                *cell = GridCell::Day(mins);
            }
        }
    }

    GridData {
        grid,
        month_marks,
        scale: max_mins.max(goal).max(1),
        max_mins,
        total_logged,
        days_tracked,
        perfect_days,
        goal,
        cursor_mins: cursor.map(|d| {
            let mut key = String::with_capacity(10);
            write_date_key(&mut key, d);
            let mins = lookup_minutes(data, &key);
            if d == today {
                mins.max(today_live_mins)
            } else {
                mins
            }
        }),
    }
}

/// Labels the first column of each month, so a label sits above the week the month starts in.
fn collect_month_marks(grid_start: NaiveDate, weeks: usize) -> Vec<(usize, &'static str)> {
    let mut marks = Vec::new();
    let mut prev_month = None;

    for col in 0..weeks {
        let month = (grid_start + Duration::days(col as i64 * 7)).month();
        if prev_month != Some(month) {
            if prev_month.is_some() {
                marks.push((col, month_abbr(month)));
            }
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
        available_height,
        cursor,
    } = ctx;
    let dim = Style::default().fg(theme.dim);
    let mut lines = Vec::with_capacity(DAYS_PER_WEEK + 4);

    lines.push(build_month_row(layout, &grid_data.month_marks, dim));

    for (row_idx, label) in DAY_LABELS.iter().enumerate() {
        let mut spans = Vec::with_capacity(1 + layout.weeks * 2);
        spans.push(Span::styled(pad_label(label), dim));
        let mut x = LABEL_COL;

        for (col, week) in grid_data.grid.iter().enumerate().take(layout.weeks) {
            // Column positions carry the month separators and the centring offset, so the
            // row is laid out by walking to each one rather than by a uniform gap.
            let target = layout.week_x(col);
            if target > x {
                spans.push(Span::raw(" ".repeat(target - x)));
                x = target;
            }
            let date = grid_start + Duration::days(col as i64 * 7 + row_idx as i64);
            spans.push(cell_span(
                week[row_idx],
                grid_data.scale,
                layout.stride,
                CellFlags {
                    is_today: date == today,
                    is_cursor: Some(date) == cursor,
                },
                theme,
                icons,
            ));
            x += layout.stride;
        }

        lines.push(Line::from(spans));
    }

    // Legend and caption are the first things to go when the panel is short.
    if available_height > lines.len() + 1 {
        lines.push(Line::from(""));
        lines.push(build_legend_row(theme, icons, dim, layout.stride));
    }
    if available_height > lines.len() {
        lines.push(build_caption_row(theme, grid_data, cursor));
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

#[derive(Clone, Copy)]
struct CellFlags {
    is_today: bool,
    is_cursor: bool,
}

/// A day tile: one centred square glyph, padded out to the stride.
///
/// Block elements were tried and are wrong for this. A full block (`█`) fills its cell top
/// to bottom, so a column of days merges into one solid bar and the grid reads as vertical
/// stripes. Half-height blocks (`▀`) fix that but anchor the ink to the top of the cell, so
/// the grid reads as rows of floating dashes. `■` is centred in its cell by the font, with
/// side and vertical bearings — the gap comes free, in both directions, and the tile sits
/// where the eye expects it.
fn tile(glyph: &str, stride: usize) -> String {
    let pad = stride.saturating_sub(UnicodeWidthStr::width(glyph));
    format!("{glyph}{}", " ".repeat(pad))
}

fn cell_span(
    cell: GridCell,
    scale: u32,
    stride: usize,
    flags: CellFlags,
    theme: &Theme,
    icons: IconSet,
) -> Span<'static> {
    // Days that have not happened yet are blank rather than background-coloured, so the
    // grid visibly stops at today instead of trailing off into invisible cells.
    let GridCell::Day(mins) = cell else {
        return Span::raw(" ".repeat(stride));
    };
    let color = theme.heat[heat_index(mins, scale)];

    let glyph = if flags.is_today {
        icons.heat_today
    } else if mins == 0 {
        icons.heat_empty
    } else {
        icons.heat_cell
    };

    if flags.is_cursor {
        // Selected cell: exact same cell shape and size, highlighted with crisp bold text color
        return Span::styled(
            tile(glyph, stride),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        );
    }

    if flags.is_today {
        let mut style = Style::default().fg(if mins == 0 { theme.dim } else { color });
        style = style.add_modifier(Modifier::BOLD);
        return Span::styled(tile(glyph, stride), style);
    }

    if mins == 0 {
        return Span::styled(tile(glyph, stride), Style::default().fg(theme.task_track));
    }

    Span::styled(tile(glyph, stride), Style::default().fg(color))
}

fn build_month_row<'a>(layout: &HeatmapLayout, marks: &[(usize, &str)], dim: Style) -> Line<'a> {
    let mut spans = Vec::with_capacity(marks.len() * 2 + 1);
    spans.push(Span::raw(" ".repeat(LABEL_COL)));

    let mut cursor = LABEL_COL;
    let grid_end = layout.grid_end();
    for &(col, label) in marks {
        let target = layout.week_x(col);
        if target < cursor + MIN_MONTH_LABEL_GAP {
            continue;
        }
        // Drop labels that would spill past the grid rather than let them clip.
        if target + UnicodeWidthStr::width(label) > grid_end {
            continue;
        }
        spans.push(Span::raw(" ".repeat(target - cursor)));
        spans.push(Span::styled(label.to_owned(), dim));
        cursor = target + UnicodeWidthStr::width(label);
    }

    Line::from(spans)
}

/// Renders the ramp straight out of `Theme::heat`, so the swatches are by construction the
/// exact colours the grid uses.
fn build_legend_row<'a>(theme: &Theme, icons: IconSet, dim: Style, stride: usize) -> Line<'a> {
    let mut spans = Vec::with_capacity(crate::theme::HEAT_STEPS * 2 + 4);
    spans.push(Span::raw(" ".repeat(LABEL_COL)));
    spans.push(Span::styled("less ", dim));

    // Empty day swatch matching inactive grid cells
    spans.push(Span::styled(
        tile(icons.heat_empty, stride),
        Style::default().fg(theme.task_track),
    ));

    // Active day swatches matching active grid cells
    for color in theme.heat.iter().skip(1) {
        spans.push(Span::styled(
            tile(icons.heat_cell, stride),
            Style::default().fg(*color),
        ));
    }

    spans.push(Span::styled(" more", dim));
    Line::from(spans)
}

/// The line under the legend: what the cursor is sitting on, or the totals when it is not
/// on anything.
///
/// Moving the cursor around a grid of coloured squares tells you nothing on its own — the
/// day and the minutes behind the square are the whole reason to move it.
fn build_caption_row<'a>(theme: &Theme, grid: &GridData, cursor: Option<NaiveDate>) -> Line<'a> {
    let comment = Style::default().fg(theme.comment);
    if let Some(date) = cursor {
        let mins = grid.cursor_mins.unwrap_or(0);
        let detail = if mins == 0 {
            "nothing logged".to_string()
        } else if grid.goal > 0 {
            format!(
                "{} · {}% of goal",
                format_minutes(mins),
                (mins as u64 * 100 / grid.goal as u64).min(999)
            )
        } else {
            format_minutes(mins)
        };
        return Line::from(vec![
            Span::raw(" ".repeat(LABEL_COL)),
            Span::styled(
                format!("{} {} ", month_abbr(date.month()), date.day()),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("· {detail}"), comment),
        ]);
    }

    Line::from(vec![
        Span::raw(" ".repeat(LABEL_COL)),
        Span::styled(
            format!(
                "// {} days tracked · {} total · {} perfect · peak {}",
                grid.days_tracked,
                format_minutes(grid.total_logged),
                grid.perfect_days,
                format_minutes(grid.max_mins),
            ),
            comment,
        ),
    ])
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

    fn layout(width: usize) -> HeatmapLayout {
        // A fixed Monday, so month boundaries — and therefore the separators — are stable.
        let today = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        HeatmapLayout::build(width, usize::MAX, today)
    }

    /// A wide terminal must not render decades of grid.
    #[test]
    fn layout_caps_weeks_at_one_year() {
        assert_eq!(layout(5000).weeks, MAX_WEEKS);
    }

    #[test]
    fn narrow_widths_shrink_the_grid_not_the_tile() {
        let l = layout(10);
        assert_eq!(l.stride, MIN_STRIDE);
        assert_eq!(l.weeks, 3);
    }

    /// The whole point of spending the remainder on month separators: the grid has to reach
    /// the right edge at the widths people actually run at. Flooring `usable / weeks` and
    /// leaving the rest as dead space used 81% of a 135-column panel and 85% of a
    /// 190-column one.
    ///
    /// Past roughly 200 columns a year of tight tiles cannot span the panel however the
    /// slack is spent — see `a_very_wide_panel_centres_the_grid` for what happens there.
    #[test]
    fn a_year_of_grid_fills_the_panel() {
        for width in [135usize, 150, 170, 190] {
            let l = layout(width);
            assert_eq!(l.weeks, MAX_WEEKS, "width {width} lost columns");
            let usable = width - LABEL_COL;
            let used = l.grid_end() - LABEL_COL;
            let pct = used * 100 / usable;
            assert!(
                pct >= 98,
                "width {width}: grid uses {pct}% of the panel (stride {})",
                l.stride
            );
        }
    }

    /// Neither a very wide panel nor a short range can be filled by a grid of tight tiles,
    /// so both sit centred rather than stranded against the left edge.
    #[test]
    fn a_very_wide_panel_centres_the_grid() {
        let l = layout(300);
        let left = l.week_x(0) - LABEL_COL;
        let right = (300 - LABEL_COL) - (l.grid_end() - LABEL_COL);
        assert!(
            left.abs_diff(right) <= 1,
            "grid is not centred: {left} left, {right} right"
        );
    }

    /// A short range cannot fill the width at any sane tile size, so it must sit centred
    /// rather than stranded against the left edge with a panel of blank to its right.
    #[test]
    fn a_short_range_is_centred() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 10).unwrap();
        let l = HeatmapLayout::build(135, 2, today);
        assert_eq!(l.weeks, 2);
        let left = l.week_x(0) - LABEL_COL;
        let right = (135 - LABEL_COL) - (l.grid_end() - LABEL_COL);
        assert!(
            left.abs_diff(right) <= 1,
            "grid is not centred: {left} left, {right} right"
        );
    }

    /// Tiles must actually get bigger as the panel does — that is the user-visible change.
    #[test]
    fn tiles_grow_with_panel_width() {
        assert!(
            layout(220).stride > layout(120).stride,
            "tile stayed at {} columns on a much wider panel",
            layout(120).stride
        );
    }

    #[test]
    fn stride_stays_within_bounds() {
        for width in [10usize, 40, 80, 120, 200, 400, 5000] {
            let l = layout(width);
            assert!(
                (MIN_STRIDE..=MAX_STRIDE).contains(&l.stride),
                "width {width}"
            );
        }
    }

    /// Columns must never overlap, whatever the separators work out to.
    #[test]
    fn columns_never_collide() {
        for width in [12usize, 40, 135, 190, 400] {
            let l = layout(width);
            for pair in l.col_x.windows(2) {
                assert!(
                    pair[1] >= pair[0] + l.stride,
                    "width {width}: columns at {} and {} overlap (stride {})",
                    pair[0],
                    pair[1],
                    l.stride
                );
            }
            assert!(
                l.grid_end() <= width,
                "width {width}: grid runs past the panel"
            );
        }
    }

    /// A tile must occupy exactly the stride — anything else and every column after it
    /// shifts, and the month labels stop sitting above their own group.
    #[test]
    fn a_tile_is_exactly_one_stride_wide() {
        for glyph in ["■", "▣", "#"] {
            for stride in MIN_STRIDE..=MAX_STRIDE {
                assert_eq!(tile(glyph, stride).width(), stride, "{glyph:?} at {stride}");
            }
        }
    }

    #[test]
    fn month_marks_change_on_month_boundary() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap(); // Monday
        let marks = collect_month_marks(start, 8);
        assert_eq!(marks[0], (4, "Feb"));
    }

    #[test]
    fn month_abbr_handles_invalid_month() {
        assert_eq!(month_abbr(1), "Jan");
        assert_eq!(month_abbr(12), "Dec");
        assert_eq!(month_abbr(0), "?");
        assert_eq!(month_abbr(13), "?");
    }

    #[test]
    fn heat_index_spans_the_whole_ramp() {
        // Zero is always the "no activity" step.
        assert_eq!(heat_index(0, 100), 0);
        // Every non-zero value lands on a visible step, and the scale maximum tops out.
        assert_eq!(heat_index(1, 100), 1);
        assert_eq!(heat_index(25, 100), 1);
        assert_eq!(heat_index(50, 100), 2);
        assert_eq!(heat_index(75, 100), 3);
        assert_eq!(heat_index(100, 100), 4);
        // Anything above the scale stays clamped to the top step.
        assert_eq!(heat_index(1_000, 100), 4);
    }

    #[test]
    fn heat_index_is_monotonic() {
        let mut prev = 0;
        for mins in 0..=200 {
            let idx = heat_index(mins, 200);
            assert!(idx >= prev, "step went backwards at {mins}m");
            prev = idx;
        }
        assert_eq!(prev, crate::theme::HEAT_STEPS - 1);
    }

    /// Guards the defect where the legend showed a colour the grid never drew.
    #[test]
    fn every_ramp_step_is_reachable() {
        let scale = 400;
        let reached: std::collections::HashSet<usize> =
            (0..=scale).map(|m| heat_index(m, scale)).collect();
        for step in 0..crate::theme::HEAT_STEPS {
            assert!(reached.contains(&step), "step {step} is never drawn");
        }
    }

    #[test]
    fn month_labels_stay_inside_the_grid() {
        let layout = layout(20);
        // A label parked on the final column would overflow; it must be dropped.
        let marks = vec![(layout.weeks - 1, "Dec")];
        let line = build_month_row(&layout, &marks, Style::default());
        let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(rendered.width() <= layout.grid_end());
    }
}

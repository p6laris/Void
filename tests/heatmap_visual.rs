//! Rendering-level checks for the focus heatmap.
//!
//! Run with `cargo test --test heatmap_visual -- --nocapture` to also eyeball the grid.

use chrono::{Datelike, Duration, Local};
use ratatui::style::Color;
use ratatui::{backend::TestBackend, Terminal};

fn sample_data(today: chrono::NaiveDate) -> Vec<(String, u32)> {
    // Seeded pseudo-random minutes so the ramp gets exercised across a year.
    let mut data: Vec<(String, u32)> = Vec::new();
    let mut seed: u64 = 42;
    for i in 0..300i64 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let r = (seed >> 33) % 100;
        if r < 35 {
            continue;
        }
        let mins = (r as u32 * 4) % 190;
        let d = today - Duration::days(299 - i);
        data.push((d.format("%Y-%m-%d").to_string(), mins));
    }
    data.sort();
    data
}

fn render(width: u16, height: u16) -> Vec<String> {
    render_cursor(width, height, None)
}

fn render_cursor(width: u16, height: u16, cursor: Option<chrono::NaiveDate>) -> Vec<String> {
    let today = Local::now().date_naive();
    let data = sample_data(today);

    let mut term = Terminal::new(TestBackend::new(width, height)).unwrap();
    let theme = void::theme::Theme::dark();
    let icons = void::ui::IconSet::detect();
    term.draw(|f| {
        void::ui::heatmap::draw_focus_heatmap(
            f,
            f.area(),
            &theme,
            icons,
            &data,
            void::ui::heatmap::HeatmapOptions {
                goal: 120,
                cursor,
                ..Default::default()
            },
        );
    })
    .unwrap();

    let buf = term.backend().buffer();
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

#[test]
#[ignore = "visual only"]
fn preview_wide() {
    let cursor = Local::now().date_naive() - Duration::days(9);
    for w in [135u16, 190] {
        println!("--- width {w} ---");
        for l in render(w, 12) {
            println!("{l}");
        }
        println!("--- width {w}, cursor ---");
        for l in render_cursor(w, 12, Some(cursor)) {
            println!("{l}");
        }
    }
}

#[test]
fn renders_grid_legend_and_caption() {
    let lines = render(120, 12);
    for line in &lines {
        println!("{line}");
    }

    // Month header, seven weekday rows, blank, legend, caption. Only Mon/Wed/Fri are
    // labelled — seven labels at full strength competed with the grid.
    assert!(
        lines[1].starts_with("Mon"),
        "expected Mon row, got {:?}",
        lines[1]
    );
    assert!(
        lines[3].starts_with("Wed"),
        "expected Wed row, got {:?}",
        lines[3]
    );
    assert!(
        lines[5].starts_with("Fri"),
        "expected Fri row, got {:?}",
        lines[5]
    );
    for row in [2usize, 4, 6, 7] {
        assert!(
            lines[row].starts_with("    "),
            "row {row} should be unlabelled, got {:?}",
            lines[row]
        );
    }
    assert!(
        lines
            .iter()
            .any(|l| l.contains("less") && l.contains("more")),
        "legend missing"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("days tracked") && l.contains("perfect")),
        "caption missing"
    );
}

/// Today gets its own glyph, drawn exactly once.
///
/// It also falls back to a visible foreground when nothing has been logged yet: today's own
/// heat step is by definition the empty colour on a blank day, which is exactly when you
/// want to be able to find it.
#[test]
fn today_is_marked_exactly_once() {
    let lines = render(150, 12);
    let icons = void::ui::IconSet::detect();
    let grid: String = lines[1..8].join(
        "
",
    );
    assert!(
        grid.contains(icons.heat_today),
        "today marker {:?} not drawn",
        icons.heat_today
    );
    assert_eq!(
        grid.matches(icons.heat_today).count(),
        1,
        "today must be marked exactly once"
    );
}

/// Future days are blank, so the grid visibly stops at today.
#[test]
fn future_days_are_blank() {
    let lines = render(120, 12);
    let today_weekday = Local::now().date_naive().weekday().num_days_from_monday() as usize;
    // Rows below today's weekday have one fewer cell in the final column.
    let full = lines[1 + today_weekday].chars().count();
    for row in (today_weekday + 1)..7 {
        assert!(
            lines[1 + row].chars().count() < full,
            "row {row} should stop before today's column"
        );
    }
}

fn rgb(c: Color) -> (i32, i32, i32) {
    match c {
        Color::Rgb(r, g, b) => (r as i32, g as i32, b as i32),
        other => panic!("expected an rgb token, got {other:?}"),
    }
}

fn distance(a: Color, b: Color) -> i32 {
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    (ar - br).pow(2) + (ag - bg).pow(2) + (ab - bb).pow(2)
}

/// Each step must sit further from the background than the one before it. This is what
/// makes the ramp read as "more activity" rather than as a set of unrelated colours.
#[test]
fn heat_ramp_is_monotone_in_every_builtin_theme() {
    for (name, theme) in [
        ("dark", void::theme::Theme::dark()),
        ("light", void::theme::Theme::light()),
        ("polaris", void::theme::Theme::polaris()),
        ("matrix", void::theme::Theme::matrix()),
    ] {
        let steps: Vec<String> = theme.heat.iter().map(|c| format!("{c:?}")).collect();
        println!("{name:8} bg={:?}\n         {}", theme.bg, steps.join("  "));

        let mut prev = -1;
        for (idx, step) in theme.heat.iter().enumerate() {
            let d = distance(*step, theme.bg);
            assert!(
                d > prev,
                "{name}: heat[{idx}] is not further from bg than heat[{}]",
                idx.saturating_sub(1)
            );
            prev = d;
        }
    }
}

#[test]
fn heat_ramp_steps_are_distinct() {
    for (name, theme) in [
        ("dark", void::theme::Theme::dark()),
        ("light", void::theme::Theme::light()),
        ("polaris", void::theme::Theme::polaris()),
        ("matrix", void::theme::Theme::matrix()),
    ] {
        for i in 0..theme.heat.len() {
            for j in (i + 1)..theme.heat.len() {
                assert_ne!(
                    theme.heat[i], theme.heat[j],
                    "{name}: heat[{i}] and heat[{j}] are the same colour"
                );
            }
        }
    }
}

/// The cursor has to be visible on a filled day and on an empty one.
///
/// It used to be a small mark on a selection background behind a solid block glyph — and a
/// block covers its own cell completely, so the background never showed. A shade glyph
/// mixes foreground and background in the same cell, which is what makes the tint land.
#[test]
fn cursor_cell_is_visibly_highlighted() {
    let today = Local::now().date_naive();
    let data = sample_data(today);
    let theme = void::theme::Theme::dark();
    let icons = void::ui::IconSet::detect();

    for back in [9i64, 10, 11, 12] {
        let cursor = today - Duration::days(back);
        let mut term = Terminal::new(TestBackend::new(190, 12)).unwrap();
        term.draw(|f| {
            void::ui::heatmap::draw_focus_heatmap(
                f,
                f.area(),
                &theme,
                icons,
                &data,
                void::ui::heatmap::HeatmapOptions {
                    goal: 120,
                    cursor: Some(cursor),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        let buf = term.backend().buffer();
        // The cursor cell is highlighted in bold theme.text
        let highlighted: Vec<_> = (1..=7u16)
            .flat_map(|y| (0..buf.area.width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let cell = &buf[(x, y)];
                let sym = cell.symbol();
                (sym == icons.heat_cell || sym == icons.heat_empty || sym == icons.heat_today)
                    && cell.fg == theme.text
                    && cell.modifier.contains(ratatui::style::Modifier::BOLD)
            })
            .collect();

        assert!(
            !highlighted.is_empty(),
            "{back} days back: cursor is not highlighted at all"
        );
    }
}

/// The grid must reach the right-hand edge at the widths people actually run at.
///
/// `stride` is a whole number of columns and 53 of them almost never divide a panel evenly.
/// Flooring the division and leaving the remainder as dead space used 81% of a 135-column
/// panel — a visible empty band down the right of the heatmap. Past ~200 columns a year of
/// tight tiles cannot span the panel at all, and the layout centres it instead.
#[test]
fn the_grid_reaches_the_right_edge() {
    for width in [135u16, 150, 170, 190] {
        let lines = render(width, 12);
        let widest = lines[1..8]
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let pct = widest * 100 / width as usize;
        assert!(
            pct >= 97,
            "width {width}: grid reaches only {pct}% across ({widest} columns)"
        );
    }
}

/// The caption reads out the cursor, because a coloured square on its own tells you nothing
/// about which day it is or how long you focused.
#[test]
fn the_caption_reads_out_the_cursor() {
    let today = Local::now().date_naive();
    let cursor = today - Duration::days(9);
    let lines = render_cursor(190, 12, Some(cursor));
    for l in &lines {
        println!("{l}");
    }

    // The panel is taller than the heatmap, so the last row is blank padding.
    let caption = lines
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("no caption");
    let expected_day = format!("{}", cursor.day());
    assert!(
        caption.contains(&expected_day),
        "caption {caption:?} does not name the cursor's day"
    );
    assert!(
        !caption.contains("days tracked"),
        "caption still shows the totals while a day is selected: {caption:?}"
    );
}

/// Tiles must not be drawn with a glyph that fills its whole cell, or the grid stops being
/// a grid.
///
/// Block elements were tried here and both failed. A full block (`█`) fills its cell top to
/// bottom, so a run of days down a column merged into one solid bar and the heatmap read as
/// vertical stripes with no day boundaries in it at all. Half-height blocks (`▀`) separate
/// the rows but anchor the ink to the top of the cell, so it read as floating dashes. The
/// gap in both directions has to come from the glyph's own bearings, which is what `■`
/// gives and no block element does.
#[test]
fn tiles_never_merge_into_vertical_bars() {
    // Glyphs that paint edge to edge in at least one direction.
    const FILLS_ITS_CELL: [&str; 10] = ["█", "▇", "▆", "▅", "▄", "▀", "▌", "▐", "▓", "▒"];

    for width in [80u16, 135, 190] {
        let today = Local::now().date_naive();
        let data = sample_data(today);
        let theme = void::theme::Theme::dark();
        let icons = void::ui::IconSet::detect();

        let mut term = Terminal::new(TestBackend::new(width, 12)).unwrap();
        term.draw(|f| {
            void::ui::heatmap::draw_focus_heatmap(
                f,
                f.area(),
                &theme,
                icons,
                &data,
                void::ui::heatmap::HeatmapOptions {
                    goal: 120,
                    cursor: Some(today - Duration::days(9)),
                    ..Default::default()
                },
            );
        })
        .unwrap();

        let buf = term.backend().buffer();
        // Rows 1..=7 are the weekday rows; row 0 is the month header.
        for y in 1..=7u16 {
            for x in 0..width {
                let sym = buf[(x, y)].symbol();
                assert!(
                    !FILLS_ITS_CELL.contains(&sym),
                    "width {width}: cell ({x},{y}) draws {sym:?}, which paints edge to edge                      and merges with its neighbour"
                );
            }
        }
    }
}

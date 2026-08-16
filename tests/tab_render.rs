//! Full-page render checks for the Dashboard and Tasks tabs.
//!
//! Run with `cargo test --test tab_render -- --nocapture` to eyeball the layout.

use ratatui::{backend::TestBackend, Terminal};
use void::app::{App, FocusTab};
use void::db::Database;
use void::model::{Priority, TaskStatus};

fn app_with_tasks() -> App {
    let db = Database::open_in_memory().unwrap();
    let mut app = App::with_database(db).unwrap();

    for (title, prio, est) in [
        ("Work on group presentation slides", Priority::High, 50u32),
        ("Read Chapter 4 for CS 101", Priority::Medium, 25),
        ("Prepare the Q3 budget spreadsheet", Priority::Low, 75),
    ] {
        void::storage::add_task_full(
            &app.db,
            &mut app.data,
            void::storage::TaskPayload {
                title: title.to_string(),
                notes: "a short note about the task".to_string(),
                estimated_minutes: est,
                priority: prio,
                tags: vec!["uni".into(), "deep-work".into()],
                due_date: None,
            },
        )
        .unwrap();
    }

    // Vary the statuses so the checkbox states differ across rows.
    let ids: Vec<u64> = app.data.tasks.keys().copied().collect();
    if let Some(t) = ids.get(1).and_then(|id| app.data.tasks.get_mut(id)) {
        t.status = TaskStatus::InProgress;
    }
    if let Some(t) = ids.get(2).and_then(|id| app.data.tasks.get_mut(id)) {
        t.status = TaskStatus::Done;
    }
    // The first task carries subtasks so the inline expansion under the cursor renders.
    if let Some(&id) = ids.first() {
        for title in [
            "Draft the outline",
            "Collect the numbers",
            "Write the summary slide",
        ] {
            void::storage::add_subtask(&app.db, &mut app.data, id, title.to_string()).unwrap();
        }
        if let Some(t) = app.data.tasks.get_mut(&id) {
            t.subtasks[0].done = true;
        }
    }

    app.task_ui.active_task = ids.first().copied();
    app.recompute_task_caches();
    app.task_ui.task_state.select(Some(0));
    app.task_ui.dashboard_task_state.select(Some(0));
    app
}

fn render_tab(app: &mut App, tab: FocusTab, w: u16, h: u16) -> Vec<String> {
    app.ui.tab = tab;
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| void::ui::render(f, app)).unwrap();
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
fn dashboard_renders() {
    let mut app = app_with_tasks();
    let lines = render_tab(&mut app, FocusTab::Dashboard, 100, 30);
    for l in &lines {
        println!("{l}");
    }
    let all = lines.join("\n");
    // The three bands: timer, full-width goal, then tasks│details.
    assert!(all.contains("focus "), "timer band missing");
    assert!(all.contains("today "), "goal band missing");
    assert!(all.contains("up next"), "task band missing");
    assert!(all.contains("details"), "details column missing");
    assert!(all.contains("[ ]"), "checkbox missing");
}

#[test]
fn tasks_renders() {
    let mut app = app_with_tasks();
    let lines = render_tab(&mut app, FocusTab::Tasks, 100, 30);
    for l in &lines {
        println!("{l}");
    }
    let all = lines.join("\n");
    assert!(all.contains("details"), "details panel missing");
    // The progress bar folded into the details panel; it no longer has its own header.
    assert!(
        all.contains('░') || all.contains('█'),
        "progress bar missing"
    );
    assert!(
        !all.contains("progress ─"),
        "progress kept a panel of its own"
    );
    // meta_row keys are lowercase now. Priority and status moved to chips beside the
    // title, so the rows that remain are the numbers.
    assert!(all.contains("estimate:"), "meta rows missing");
    assert!(all.contains("subtasks"), "subtask panel missing");
    assert!(all.contains("Draft the outline"), "subtasks missing");
    assert!(all.contains('│'), "column gutter missing");
}

/// The details panel used to slice `area.width - 4` and `- 10`, which panics once the
/// panel is narrower than the padding.
#[test]
fn narrow_terminals_do_not_panic() {
    let mut app = app_with_tasks();
    for w in [12u16, 16, 20, 28, 40] {
        for h in [8u16, 12, 20] {
            for tab in [FocusTab::Dashboard, FocusTab::Tasks, FocusTab::Stats] {
                let _ = render_tab(&mut app, tab, w, h);
            }
        }
    }
}

#[test]
fn zen_overlay_stays_readable_over_the_canvas() {
    let mut app = app_with_tasks();
    app.ui.tab = FocusTab::Dashboard;
    app.ui.zen_mode = true;

    let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
    term.draw(|f| void::ui::render(f, &mut app)).unwrap();
    let buf = term.backend().buffer();

    let lines: Vec<String> = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect();
    for l in &lines {
        println!("{}", l.trim_end());
    }

    // The plate is a centred box, so canvas art legitimately remains at the far edges of
    // the clock's row. What must hold is that the plate itself is clean: a run of blank
    // columns on both sides of the digits, with no canvas glyph inside that margin.
    let clock = lines
        .iter()
        .find(|l| l.contains("25:00") || l.contains(':') && l.contains('0'))
        .expect("no clock line rendered");

    let chars: Vec<char> = clock.chars().collect();
    let start = clock
        .find(|c: char| c.is_ascii_digit())
        .map(|byte_idx| clock[..byte_idx].chars().count())
        .expect("no digits on the clock line");
    let end = chars
        .iter()
        .rposition(|c| c.is_ascii_digit())
        .expect("no digits on the clock line");

    // Digits and separators only, between first and last digit.
    let core: String = chars[start..=end].iter().collect();
    assert!(
        core.chars()
            .all(|c| c.is_ascii_digit() || c == ':' || c == '.'),
        "canvas bled between the digits: {core:?}"
    );

    // At least a few blank columns of plate margin on each side.
    const MARGIN: usize = 4;
    for offset in 1..=MARGIN {
        if let Some(i) = start.checked_sub(offset) {
            assert_eq!(chars[i], ' ', "canvas inside the left plate margin");
        }
        if let Some(&c) = chars.get(end + offset) {
            assert_eq!(c, ' ', "canvas inside the right plate margin");
        }
    }
}

/// No cell may keep `Color::Reset` as its background.
///
/// `Reset` means "whatever the terminal's own background is", so any widget that leaves it
/// makes the app take on the terminal theme in patches — a dark theme running in a light
/// terminal showed pale rectangles wherever this leaked. Canvas was doing exactly that:
/// it repaints its whole area with `background_color`, which defaults to `Reset`.
#[test]
fn no_widget_leaks_the_terminal_background() {
    use ratatui::style::Color;

    let mut app = app_with_tasks();
    let theme_bg = app.theme.bg;

    let mut checked = 0;
    for (label, zen, tab) in [
        ("dashboard", false, FocusTab::Dashboard),
        ("zen", true, FocusTab::Dashboard),
        ("tasks", false, FocusTab::Tasks),
        ("stats", false, FocusTab::Stats),
        ("settings", false, FocusTab::Settings),
        ("help", false, FocusTab::Help),
        ("about", false, FocusTab::About),
    ] {
        app.ui.zen_mode = zen;
        app.ui.tab = tab;

        let mut term = Terminal::new(TestBackend::new(100, 30)).unwrap();
        term.draw(|f| void::ui::render(f, &mut app)).unwrap();
        let buf = term.backend().buffer();

        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let bg = buf[(x, y)].bg;
                assert_ne!(
                    bg,
                    Color::Reset,
                    "{label}: cell ({x},{y}) leaks the terminal background \
                     (theme bg is {theme_bg:?})"
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 0);
}

/// Eyeball the canvas at the sizes people actually run the app at.
///
/// `cargo test --test tab_render preview_scene -- --ignored --nocapture`
#[test]
#[ignore = "visual only"]
fn preview_scene() {
    use void::model::TimerState;

    let mut app = app_with_tasks();
    for (label, zen, running, w, h) in [
        ("dashboard idle 190x44", false, false, 190u16, 44u16),
        ("dashboard running 190x44", false, true, 190, 44),
        ("zen idle 190x44", true, false, 190, 44),
        ("zen running 120x36", true, true, 120, 36),
    ] {
        println!("--- {label} ---");
        app.ui.zen_mode = zen;
        if running {
            app.timer.start();
            // A running timer reads its elapsed time off `started_at`, so wind the clock
            // back instead of setting the field. Two fifths in shows the wreath boundary.
            let in_secs = u64::from(app.timer.total_seconds) * 2 / 5;
            app.timer.started_at =
                Some(std::time::Instant::now() - std::time::Duration::from_secs(in_secs));
        } else {
            app.timer.state = TimerState::Idle;
            app.timer.started_at = None;
            app.timer.elapsed_seconds = 0;
        }
        for l in render_tab(&mut app, FocusTab::Dashboard, w, h) {
            println!("{l}");
        }
    }
}

/// Eyeball the Tasks page, including the subtask expansion in both focus states.
///
/// `cargo test --test tab_render preview_tasks -- --ignored --nocapture`
#[test]
#[ignore = "visual only"]
fn preview_tasks() {
    let mut app = app_with_tasks();
    for (label, focus, w, h) in [
        ("browsing 190x40", false, 190u16, 40u16),
        ("subtask focus 190x40", true, 190, 40),
        ("narrow 80x24", false, 80, 24),
    ] {
        println!("--- {label} ---");
        app.task_ui.subtask_focus = focus;
        app.task_ui.subtask_selected = 1;
        for l in render_tab(&mut app, FocusTab::Tasks, w, h) {
            println!("{l}");
        }
    }
}

/// The inline expansion caps how many subtasks it draws, but `subtask_selected` is not
/// capped — so a flat "first N" let the cursor sit on a row that was never rendered:
/// nothing looked selected and `[x]` toggled something invisible.
#[test]
fn a_far_subtask_cursor_stays_on_screen() {
    let mut app = app_with_tasks();
    let id = *app.data.tasks.keys().next().unwrap();
    for i in 4..=20 {
        void::storage::add_subtask(&app.db, &mut app.data, id, format!("Subtask number {i}"))
            .unwrap();
    }
    app.task_ui.subtask_focus = true;
    app.task_ui.subtask_selected = 15;

    let lines = render_tab(&mut app, FocusTab::Tasks, 120, 30);
    for l in &lines {
        println!("{l}");
    }
    let cursor = lines
        .iter()
        .find(|l| l.contains("▸ ["))
        .expect("no subtask cursor drawn");
    assert!(
        cursor.contains("Subtask number 16"),
        "cursor is on the wrong subtask: {cursor:?}"
    );
}

/// Test that all 4 Stats sub-view modes (Week, Tags, Weekday, Hourly) render without panic.
#[test]
fn all_stats_view_modes_render_cleanly() {
    let mut app = app_with_tasks();
    app.stats.tag_analytics = vec![
        ("rust".into(), 120),
        ("tui".into(), 60),
        ("study".into(), 30),
    ];
    app.stats.hourly_distribution = [
        0, 0, 0, 0, 0, 0, 30, 45, 60, 90, 120, 80, 50, 40, 60, 75, 45, 30, 20, 15, 10, 5, 0, 0,
    ];

    for mode in void::app::StatsViewMode::all() {
        app.stats.stats_view_mode = mode;
        let lines = render_tab(&mut app, FocusTab::Stats, 135, 36);
        assert!(
            !lines.is_empty(),
            "Stats mode {mode:?} rendered empty lines"
        );
        let rendered = lines.join("\n");
        assert!(
            rendered.contains("[v]"),
            "Mode {mode:?} missing [v] tab header in {rendered}"
        );
    }
}

/// Eyeball the Stats page.
///
/// `cargo test --test tab_render preview_stats -- --ignored --nocapture`
#[test]
#[ignore = "visual only"]
fn preview_stats() {
    let mut app = app_with_tasks();
    app.stats.tag_analytics = vec![
        ("rust".into(), 120),
        ("tui".into(), 60),
        ("study".into(), 30),
    ];
    app.stats.hourly_distribution = [
        0, 0, 0, 0, 0, 0, 30, 45, 60, 90, 120, 80, 50, 40, 60, 75, 45, 30, 20, 15, 10, 5, 0, 0,
    ];
    for mode in void::app::StatsViewMode::all() {
        app.stats.stats_view_mode = mode;
        println!("=== STATS MODE: {:?} ===", mode);
        for (label, w, h) in [
            ("135x36", 135u16, 36u16),
            ("190x40", 190, 40),
            ("80x26", 80, 26),
        ] {
            println!("--- stats {label} ---");
            for l in render_tab(&mut app, FocusTab::Stats, w, h) {
                println!("{l}");
            }
        }
    }
}

#[test]
fn dashboard_renders_with_all_canvas_modes() {
    let mut app = app_with_tasks();
    for mode in [
        void::model::CanvasMode::Animated,
        void::model::CanvasMode::Static,
        void::model::CanvasMode::Off,
    ] {
        app.data.canvas_mode = mode;
        let lines = render_tab(&mut app, FocusTab::Dashboard, 120, 30);
        assert!(!lines.is_empty());
        let full = lines.join("\n");
        assert!(
            full.contains("focus"),
            "Dashboard missing focus in mode {mode:?}"
        );
    }
}

#[test]
fn zen_renders_with_all_canvas_modes() {
    let mut app = app_with_tasks();
    app.ui.zen_mode = true;
    for mode in [
        void::model::CanvasMode::Animated,
        void::model::CanvasMode::Static,
        void::model::CanvasMode::Off,
    ] {
        app.data.canvas_mode = mode;
        let lines = render_tab(&mut app, FocusTab::Dashboard, 120, 30);
        assert!(!lines.is_empty());
        let full = lines.join("\n");
        assert!(
            full.contains("25:00") || full.contains("focus"),
            "Zen missing content in mode {mode:?}"
        );
    }
}

#[test]
fn about_tab_renders_with_ursa_minor_constellation() {
    let mut app = app_with_tasks();
    for (w, h) in [(120, 36), (80, 24)] {
        let lines = render_tab(&mut app, FocusTab::About, w, h);
        assert!(!lines.is_empty());
        let full = lines.join("\n");
        assert!(
            full.contains("Polaris") || full.contains("Ursa Minor"),
            "About missing Ursa Minor in {w}x{h}"
        );
    }
}

#[test]
fn all_popups_render_cleanly_on_various_resolutions() {
    let mut app = app_with_tasks();
    let task_id = app.data.tasks[0].id;
    app.task_ui.bulk_selected.insert(task_id);

    let popups = vec![
        void::app::Popup::AddTask,
        void::app::Popup::EditTask(task_id),
        void::app::Popup::AddSubtask(task_id),
        void::app::Popup::EditSubtask(task_id, 1),
        void::app::Popup::ConfirmDelete(task_id),
        void::app::Popup::BulkConfirm(void::app::BulkAction::Delete),
        void::app::Popup::BulkConfirm(void::app::BulkAction::MarkDone),
        void::app::Popup::EmptyQueueChoice,
    ];

    for popup in popups {
        app.input.popup = Some(popup.clone());
        for (w, h) in [(120, 36), (80, 24)] {
            let lines = render_tab(&mut app, FocusTab::Tasks, w, h);
            assert!(!lines.is_empty());
            let full = lines.join("\n");
            match &popup {
                void::app::Popup::AddTask => assert!(full.contains("Add Task")),
                void::app::Popup::EditTask(_) => assert!(full.contains("Edit Task")),
                void::app::Popup::AddSubtask(_) => assert!(full.contains("Add Subtask")),
                void::app::Popup::EditSubtask(_, _) => assert!(full.contains("Edit Subtask")),
                void::app::Popup::ConfirmDelete(_) => assert!(full.contains("Delete")),
                void::app::Popup::BulkConfirm(void::app::BulkAction::Delete) => {
                    assert!(full.contains("Bulk Delete"))
                }
                void::app::Popup::BulkConfirm(void::app::BulkAction::MarkDone) => {
                    assert!(full.contains("Bulk Complete"))
                }
                void::app::Popup::EmptyQueueChoice => assert!(full.contains("Queue Cleared")),
            }
        }
    }
}

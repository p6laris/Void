//! Diagnostic timings for the hottest render paths. These make no assertions and are
//! ignored by default; run them deliberately with:
//!
//!   cargo test --release --test perf_probe -- --ignored --nocapture
use chrono::{Duration, Local};
use ratatui::{backend::TestBackend, Terminal};
use std::time::Instant;

#[test]
#[ignore = "diagnostic timing, not a pass/fail test"]
fn time_heatmap_render() {
    let today = Local::now().date_naive();
    // Three years of daily entries — a heavy but realistic long-term user.
    let data: Vec<(String, u32)> = (0..1095i64)
        .map(|i| {
            let d = today - Duration::days(1094 - i);
            (d.format("%Y-%m-%d").to_string(), (i as u32 * 7) % 200)
        })
        .collect();

    let theme = void::theme::Theme::dark();
    let icons = void::ui::IconSet::detect();
    let mut term = Terminal::new(TestBackend::new(160, 14)).unwrap();

    // Warm up, then time a batch.
    for _ in 0..10 {
        term.draw(|f| {
            void::ui::heatmap::draw_focus_heatmap(
                f, f.area(), &theme, icons, &data,
                void::ui::heatmap::HeatmapOptions { goal: 120, ..Default::default() },
            );
        })
        .unwrap();
    }

    let n = 1000;
    let start = Instant::now();
    for _ in 0..n {
        term.draw(|f| {
            void::ui::heatmap::draw_focus_heatmap(
                f, f.area(), &theme, icons, &data,
                void::ui::heatmap::HeatmapOptions { goal: 120, ..Default::default() },
            );
        })
        .unwrap();
    }
    let per = start.elapsed() / n;
    println!("heatmap render: {per:?} per frame ({} day entries)", data.len());
}

#[test]
#[ignore = "diagnostic timing, not a pass/fail test"]
fn time_peak_hour_query() {
    use void::model::{FocusSessionRecord, TimerMode};

    for n in [500usize, 5_000, 20_000] {
        let db = void::db::Database::open_in_memory().unwrap();
        let base = chrono::Utc::now();
        for i in 0..n {
            let rec = FocusSessionRecord {
                date: (base - chrono::Duration::minutes(i as i64 * 30))
                    .format("%Y-%m-%d")
                    .to_string(),
                minutes: 25,
                task_id: None,
                mode: TimerMode::Focus,
                completed_at: base - chrono::Duration::minutes(i as i64 * 30),
                note: String::new(),
                tags: vec![],
                pause_count: 0,
                pause_seconds: 0,
            };
            db.insert_focus_session(&rec).unwrap();
        }

        let start = Instant::now();
        let reps = 20;
        for _ in 0..reps {
            let _ = void::storage::most_productive_hour_label(&db);
        }
        println!("peak-hour query with {n:>6} sessions: {:?} per call", start.elapsed() / reps);
    }
}

#[test]
#[ignore = "diagnostic timing, not a pass/fail test"]
fn time_canvas_scene() {
    use void::canvas_timer::{draw_dashboard_canvas, draw_zen_canvas, SceneLayout, SceneOptions};
    use void::timer::{Timer, TimerConfig};

    let theme = void::theme::Theme::dark();
    let style = theme.scene_style(theme.accent);
    let timer = Timer::new(TimerConfig::default());

    for (name, w, h) in [("dashboard", 100u16, 16u16), ("zen", 190, 44)] {
        let zen = name == "zen";
        let opts = SceneOptions {
            task_progress: Some(0.4),
            pending_tasks: 5,
            active_task_index: Some(1),
            sessions_done: 2,
            sessions_total: 4,
            layout: if zen { SceneLayout::Zen } else { SceneLayout::Dashboard },
            animated: true,
        };
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();

        for _ in 0..5 {
            term.draw(|f| {
                if zen { draw_zen_canvas(f, f.area(), &timer, &style, &opts) }
                else { draw_dashboard_canvas(f, f.area(), &timer, &style, &opts) }
            })
            .unwrap();
        }

        let n = 300;
        let start = Instant::now();
        for _ in 0..n {
            term.draw(|f| {
                if zen { draw_zen_canvas(f, f.area(), &timer, &style, &opts) }
                else { draw_dashboard_canvas(f, f.area(), &timer, &style, &opts) }
            })
            .unwrap();
        }
        println!("{name:10} {w}x{h}: {:?} per frame", start.elapsed() / n);
    }
}

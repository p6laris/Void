use std::io::{self, Stdout, Write};
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use void::app::App;
use void::ui;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if handle_cli(args)? {
        return Ok(());
    }

    void::sound::init_audio();

    let mut terminal = setup_terminal()?;
    let mut app = App::new()?;
    let res = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    if let Err(e) = res {
        eprintln!("Void error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn parse_cli_task_id(raw: &str, command: &str) -> Option<u64> {
    match raw.parse() {
        Ok(id) => Some(id),
        Err(_) => {
            eprintln!("Invalid task_id: {raw}");
            eprintln!("Usage: void {command} <task_id>");
            None
        }
    }
}

fn handle_cli(args: Vec<String>) -> Result<bool> {
    if args.len() < 2 {
        return Ok(false);
    }
    match args[1].as_str() {
        "add" => {
            if args.len() < 3 {
                eprintln!("Usage: void add \"Task title\" [--due YYYY-MM-DD|today|tomorrow] [--tags tag1,tag2]");
                return Ok(true);
            }
            let title = args[2].clone();
            let mut due = None;
            let mut tags = Vec::new();

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--due" => {
                        i += 1;
                        if i < args.len() {
                            let val = args[i].as_str();
                            match void::storage::normalize_due_date(val, false) {
                                Ok(d) => due = d,
                                Err(e) => {
                                    eprintln!("Invalid due date: {}", e);
                                    return Ok(true);
                                }
                            }
                        }
                    }
                    "--tags" => {
                        i += 1;
                        if i < args.len() {
                            tags = void::storage::parse_tags(&args[i]);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            let db = void::db::Database::open()?;
            let mut data = db.load_app_data().unwrap_or_default();
            let id = void::storage::add_task_full(
                &db,
                &mut data,
                void::storage::TaskPayload {
                    title: title.clone(),
                    notes: String::new(),
                    estimated_minutes: 25,
                    priority: void::model::Priority::Medium,
                    tags,
                    due_date: due,
                },
            )?;
            println!("Added task: \"{}\" (ID: {})", title, id);
            Ok(true)
        }
        "list" => {
            let db = void::db::Database::open()?;
            let data = db.load_app_data().unwrap_or_default();
            let pending = void::storage::sorted_pending_tasks(&data);
            if pending.is_empty() {
                println!("No pending tasks. You're all caught up!");
            } else {
                println!(
                    "{:<5} | {:<40} | {:<10} | {:<10}",
                    "ID", "TITLE", "PRIORITY", "DUE DATE"
                );
                println!("{:-<5}-+-{:-<40}-+-{:-<10}-+-{:-<10}", "", "", "", "");
                for t in pending {
                    let due = t.due_date.as_deref().unwrap_or("-");
                    println!(
                        "{:<5} | {:<40} | {:<10} | {:<10}",
                        t.id,
                        t.title.chars().take(40).collect::<String>(),
                        t.priority.label(),
                        due
                    );
                }
            }
            Ok(true)
        }
        "done" => {
            if args.len() < 3 {
                eprintln!("Usage: void done <task_id>");
                return Ok(true);
            }
            let Some(id) = parse_cli_task_id(&args[2], "done") else {
                return Ok(true);
            };
            let db = void::db::Database::open()?;
            let mut data = db.load_app_data().unwrap_or_default();

            if data.tasks.iter().any(|t| t.id == id) {
                void::storage::mark_task_done(&db, &mut data, id)?;
                println!("Task {} marked as done.", id);
            } else {
                eprintln!("Task {} not found.", id);
            }
            Ok(true)
        }
        "start" => {
            if args.len() < 3 {
                eprintln!("Usage: void start <task_id>");
                return Ok(true);
            }
            let Some(id) = parse_cli_task_id(&args[2], "start") else {
                return Ok(true);
            };
            let db = void::db::Database::open()?;
            let mut data = db.load_app_data().unwrap_or_default();

            if data
                .tasks
                .iter()
                .any(|t| t.id == id && t.status != void::model::TaskStatus::Done)
            {
                void::storage::promote_task_on_activate(&db, &mut data, id)?;
                db.persist_active_task(Some(id))?;
                // Return false to let the GUI boot up
                Ok(false)
            } else {
                eprintln!("Task {} not found or already done.", id);
                Ok(true)
            }
        }
        "help" | "--help" | "-h" => {
            println!("Void CLI - Terminal Focus Application\n");
            println!("Commands:");
            println!("  add \"Title\" [--due YYYY-MM-DD|today|tomorrow] [--tags tag1,tag2]");
            println!("  list              (Lists pending tasks)");
            println!("  done <task_id>    (Marks task as complete)");
            println!("  start <task_id>   (Sets task active and launches the GUI)");
            println!("  archive list      (Lists archived tasks)");
            println!("  --export [path]   (Exports database to JSON)");
            println!("  --import <path>   (Imports database from JSON, OVERWRITING current data)");
            println!("  help              (Shows this message)");
            println!("\nRun without arguments to launch the GUI interface.");
            Ok(true)
        }
        "archive" => {
            if args.len() < 3 || args[2] != "list" {
                eprintln!("Usage: void archive list");
                return Ok(true);
            }
            let db = void::db::Database::open()?;
            let data = db.load_app_data().unwrap_or_default();
            let archived: Vec<_> = void::storage::archived_tasks(&data).collect();
            if archived.is_empty() {
                println!("No archived tasks.");
            } else {
                for t in archived {
                    println!("{} | {}", t.id, t.title);
                }
            }
            Ok(true)
        }
        "export" | "--export" => {
            let db = void::db::Database::open()?;
            let path = if args.len() >= 3 {
                let dest = std::path::PathBuf::from(&args[2]);
                let data = db.load_app_data().unwrap_or_default();
                let raw = serde_json::to_string_pretty(&data)?;
                std::fs::write(&dest, raw)?;
                dest
            } else {
                db.export_json()?
            };
            println!("Exported backup to {}", path.display());
            Ok(true)
        }
        "import" | "--import" => {
            if args.len() < 3 {
                eprintln!("Usage: void --import <path_to_json>");
                return Ok(true);
            }
            let path = std::path::PathBuf::from(&args[2]);
            if !path.exists() {
                eprintln!("Error: File not found at {}", path.display());
                return Ok(true);
            }

            print!("WARNING: This will completely overwrite your current tasks and focus history.\nAre you sure you want to proceed? (y/N): ");
            if let Err(e) = std::io::stdout().flush() {
                eprintln!("Could not show import prompt: {e}");
                return Ok(true);
            }
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() != "y" {
                println!("Import cancelled.");
                return Ok(true);
            }

            let db = void::db::Database::open()?;
            if let Err(e) = db.import_json(&path) {
                eprintln!("Import failed: {e:#}");
            } else {
                println!("Successfully imported database from {}", path.display());
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn set_window_title(title: &str) {
    let _ = execute!(
        io::stdout(),
        crossterm::style::Print(format!("\x1b]0;{}\x07", title.replace('\x1b', "")))
    );
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut last_tick = std::time::Instant::now();
    loop {
        app.refresh_chart_if_needed();
        if app.data.show_terminal_title {
            set_window_title(&app.window_title());
        }
        terminal.draw(|f| ui::render(f, app))?;

        let tick_rate = app.tick_rate();
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        app.handle_key(key);
                    }
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse);
                }
                _ => {}
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

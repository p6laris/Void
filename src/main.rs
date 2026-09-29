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
    match handle_cli(args)? {
        None => {}
        Some(0) => return Ok(()),
        Some(code) => {
            let _ = io::stdout().flush();
            std::process::exit(code);
        }
    }

    void::sound::init_audio();

    // Built first so a startup error can't leave the terminal in raw mode.
    let mut app = App::new()?;
    let mut terminal = setup_terminal()?;
    install_panic_hook();
    let res = run_app(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    if let Err(e) = res {
        eprintln!("Void error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

fn print_csv_import(summary: &void::db::CsvImportSummary, path: &std::path::Path) {
    println!(
        "Imported {} session(s) from {}",
        summary.imported,
        path.display()
    );
    if summary.skipped > 0 {
        println!(
            "Skipped {} row(s) with an invalid date or already imported.",
            summary.skipped
        );
    }
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

/// Exit code for a command that ran but failed, or was cancelled.
const EXIT_FAILED: i32 = 1;
/// Exit code for a command given invalid arguments.
const EXIT_USAGE: i32 = 2;

const USAGE: &str = "Usage: void [command]

Commands:
  add \"Title\" [--due YYYY-MM-DD|today|tomorrow] [--tags tag1,tag2]
  list                 List pending tasks
  done <task_id>       Mark a task as done
  start <task_id>      Make a task active and open Void
  archive list         List archived tasks
  export [path]        Export a full JSON backup (CSV if the path ends in .csv)
  export-csv [path]    Export focus and break sessions to CSV
  import <path> [--yes]      Restore a JSON backup, or add sessions from a CSV
  import-csv <path> [--yes]  Add sessions from a CSV file
  help                 Show this message
  version              Show the version

Commands also accept a leading --, as in --export.
Run without a command to open Void.";

struct AddArgs {
    title: String,
    due: Option<String>,
    tags: Vec<String>,
}

/// Parses the arguments after `void add`.
fn parse_add_args(args: &[String]) -> Result<AddArgs, String> {
    let mut title = None;
    let mut due = None;
    let mut tags = Vec::new();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--due" => {
                let value = rest.next().ok_or("--due needs a date")?;
                due = void::storage::normalize_due_date(value, false)
                    .map_err(|e| format!("Invalid due date: {e}"))?;
            }
            "--tags" => {
                let value = rest.next().ok_or("--tags needs a value")?;
                tags = void::storage::parse_tags(value);
            }
            flag if flag.starts_with("--") => return Err(format!("Unknown option: {flag}")),
            text if title.is_none() => title = Some(text.to_string()),
            extra => {
                return Err(format!(
                    "Unexpected argument: {extra} (quote a title that has spaces)"
                ))
            }
        }
    }
    let title = title.ok_or("Missing task title")?;
    Ok(AddArgs { title, due, tags })
}

/// Asks a yes/no question on the terminal; anything but `y` is no.
fn confirm(prompt: &str) -> Result<bool> {
    print!("{prompt} (y/N): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// The path argument of an import command, or the exit code if it's missing.
fn existing_file(raw: Option<&String>, usage: &str) -> Result<std::path::PathBuf, i32> {
    let Some(raw) = raw else {
        eprintln!("{usage}");
        return Err(EXIT_USAGE);
    };
    let path = std::path::PathBuf::from(raw);
    if !path.exists() {
        eprintln!("File not found: {}", path.display());
        return Err(EXIT_FAILED);
    }
    Ok(path)
}

fn import_csv_file(path: &std::path::Path, yes: bool) -> Result<i32> {
    if !yes && !confirm("Import sessions from CSV into your current database?")? {
        println!("Import cancelled.");
        return Ok(EXIT_FAILED);
    }
    let db = void::db::Database::open()?;
    match db.import_csv(path) {
        Ok(summary) => {
            print_csv_import(&summary, path);
            Ok(0)
        }
        Err(e) => {
            eprintln!("CSV import failed: {e:#}");
            Ok(EXIT_FAILED)
        }
    }
}

fn export_csv_to(db: &void::db::Database, dest: Option<&String>) -> Result<std::path::PathBuf> {
    let Some(dest) = dest else {
        return db.export_csv();
    };
    let dest = std::path::PathBuf::from(dest);
    db.export_csv_to(&dest)?;
    Ok(dest)
}

fn is_csv_path(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("csv"))
}

fn is_yes_flag(arg: &str) -> bool {
    arg == "--yes" || arg == "-y"
}

/// Fits `text` to exactly `width` terminal columns, padding or cutting with `…`.
fn pad_to_width(text: &str, width: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
    if text.width() <= width {
        return format!("{text}{}", " ".repeat(width - text.width()));
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(1);
        if used + w + 1 > width {
            break;
        }
        out.push(ch);
        used += w;
    }
    format!("{out}…{}", " ".repeat(width - used - 1))
}

/// Runs a CLI command. `None` means the TUI should open.
fn handle_cli(args: Vec<String>) -> Result<Option<i32>> {
    let Some(command) = args.get(1) else {
        return Ok(None);
    };
    let command = command.strip_prefix("--").unwrap_or(command);
    let code = match command {
        "add" => {
            let parsed = match parse_add_args(&args[2..]) {
                Ok(parsed) => parsed,
                Err(e) => {
                    eprintln!("{e}");
                    eprintln!("Usage: void add \"Task title\" [--due YYYY-MM-DD|today|tomorrow] [--tags tag1,tag2]");
                    return Ok(Some(EXIT_USAGE));
                }
            };
            let db = void::db::Database::open()?;
            let mut data = db.load_app_data()?;
            let id = void::storage::add_task_full(
                &db,
                &mut data,
                void::storage::TaskPayload {
                    title: parsed.title.clone(),
                    notes: String::new(),
                    estimated_minutes: 25,
                    priority: void::model::Priority::Medium,
                    tags: parsed.tags,
                    due_date: parsed.due,
                },
            )?;
            println!("Added task: \"{}\" (ID: {})", parsed.title, id);
            0
        }
        "list" => {
            let db = void::db::Database::open()?;
            let data = db.load_app_data()?;
            let pending = void::storage::sorted_pending_tasks(&data);
            if pending.is_empty() {
                println!("No pending tasks. You're all caught up!");
            } else {
                println!(
                    "{:<5} | {} | {:<10} | {:<10}",
                    "ID",
                    pad_to_width("TITLE", 40),
                    "PRIORITY",
                    "DUE DATE"
                );
                println!("{:-<5}-+-{:-<40}-+-{:-<10}-+-{:-<10}", "", "", "", "");
                for t in pending {
                    let due = t.due_date.as_deref().unwrap_or("-");
                    println!(
                        "{:<5} | {} | {:<10} | {:<10}",
                        t.id,
                        pad_to_width(&t.title, 40),
                        t.priority.label(),
                        due
                    );
                }
            }
            0
        }
        "done" | "start" => {
            let Some(raw) = args.get(2) else {
                eprintln!("Usage: void {command} <task_id>");
                return Ok(Some(EXIT_USAGE));
            };
            let Some(id) = parse_cli_task_id(raw, command) else {
                return Ok(Some(EXIT_USAGE));
            };
            let db = void::db::Database::open()?;
            let mut data = db.load_app_data()?;
            let Some(task) = data.task(id) else {
                eprintln!("Task {id} not found.");
                return Ok(Some(EXIT_FAILED));
            };
            if !task.is_open() {
                eprintln!("Task {id} is already done or archived.");
                return Ok(Some(EXIT_FAILED));
            }
            if command == "start" && task.is_blocked(&data.tasks) {
                eprintln!("Task {id} is blocked; finish the tasks it depends on first.");
                return Ok(Some(EXIT_FAILED));
            }
            if command == "start" {
                void::storage::promote_task_on_activate(&db, &mut data, id)?;
                db.persist_active_task(Some(id))?;
                return Ok(None);
            }
            void::storage::mark_task_done(&db, &mut data, id)?;
            println!("Task {id} marked as done.");
            0
        }
        "help" | "h" | "-h" => {
            println!("Void {}\n", env!("CARGO_PKG_VERSION"));
            println!("{USAGE}");
            0
        }
        "version" | "-V" => {
            println!("void {}", env!("CARGO_PKG_VERSION"));
            0
        }
        "archive" => {
            if args.get(2).map(String::as_str) != Some("list") {
                eprintln!("Usage: void archive list");
                return Ok(Some(EXIT_USAGE));
            }
            let db = void::db::Database::open()?;
            let data = db.load_app_data()?;
            let archived: Vec<_> = void::storage::archived_tasks(&data).collect();
            if archived.is_empty() {
                println!("No archived tasks.");
            } else {
                for t in archived {
                    println!("{} | {}", t.id, t.title);
                }
            }
            0
        }
        "export" => {
            let db = void::db::Database::open()?;
            let dest = args.get(2);
            if dest.is_some_and(|d| is_csv_path(d)) {
                let path = export_csv_to(&db, dest)?;
                println!("Exported sessions to CSV at {}", path.display());
            } else {
                let path = match dest {
                    Some(dest) => {
                        let dest = std::path::PathBuf::from(dest);
                        db.export_json_to(&dest)?;
                        dest
                    }
                    None => db.export_json()?,
                };
                println!("Exported JSON backup to {}", path.display());
            }
            0
        }
        "export-csv" => {
            let db = void::db::Database::open()?;
            let path = export_csv_to(&db, args.get(2))?;
            println!("Exported sessions to CSV at {}", path.display());
            0
        }
        "import" => {
            let yes = args[2..].iter().any(|a| is_yes_flag(a));
            let target = args[2..].iter().find(|a| !is_yes_flag(a));
            let path = match existing_file(
                target,
                "Usage: void import <backup.json|sessions.csv> [--yes]",
            ) {
                Ok(path) => path,
                Err(code) => return Ok(Some(code)),
            };
            if is_csv_path(&path.to_string_lossy()) {
                import_csv_file(&path, yes)?
            } else if !yes
                && !confirm(
                "WARNING: This will completely overwrite your current tasks and focus history.\nAre you sure you want to proceed?",
            )? {
                println!("Import cancelled.");
                EXIT_FAILED
            } else {
                let db = void::db::Database::open()?;
                match db.import_json(&path) {
                    Ok(()) => {
                        println!("Successfully imported database from {}", path.display());
                        0
                    }
                    Err(e) => {
                        eprintln!("Import failed: {e:#}");
                        EXIT_FAILED
                    }
                }
            }
        }
        "import-csv" => {
            let yes = args[2..].iter().any(|a| is_yes_flag(a));
            let target = args[2..].iter().find(|a| !is_yes_flag(a));
            match existing_file(target, "Usage: void import-csv <sessions.csv> [--yes]") {
                Ok(path) => import_csv_file(&path, yes)?,
                Err(code) => code,
            }
        }
        unknown => {
            eprintln!("Unknown command: {unknown}\n");
            eprintln!("{USAGE}");
            EXIT_USAGE
        }
    };
    Ok(Some(code))
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste,
        crossterm::event::EnableFocusChange
    )?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Restores the terminal before the default panic handler prints.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
            crossterm::event::DisableBracketedPaste,
            crossterm::event::DisableFocusChange,
            crossterm::terminal::SetTitle("")
        );
        default_hook(info);
    }));
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
        crossterm::event::DisableFocusChange,
        crossterm::terminal::SetTitle("")
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn set_window_title(title: &str) {
    let _ = execute!(io::stdout(), crossterm::terminal::SetTitle(title));
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut last_tick = std::time::Instant::now();
    let mut needs_draw = true;
    let mut drawn_frame = 0;

    loop {
        if needs_draw {
            app.refresh_chart_if_needed();
            if let Some(title) = app.poll_window_title() {
                set_window_title(title);
            }
            terminal.draw(|f| ui::render(f, app))?;
            needs_draw = false;
            drawn_frame = app.frame_signature();
        }

        let tick_rate = app.tick_rate();
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_millis(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        app.handle_key(key);
                        needs_draw = true;
                    }
                }
                Event::Mouse(mouse) => {
                    if matches!(
                        mouse.kind,
                        crossterm::event::MouseEventKind::ScrollUp
                            | crossterm::event::MouseEventKind::ScrollDown
                    ) {
                        app.handle_mouse(mouse);
                        needs_draw = true;
                    }
                }
                Event::FocusGained => {
                    app.ui.focused = true;
                    needs_draw = true;
                }
                Event::FocusLost => app.ui.focused = false,
                Event::Paste(text) => {
                    app.handle_paste(&text);
                    needs_draw = true;
                }
                Event::Resize(_, _) => {
                    needs_draw = true;
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            app.on_tick();
            last_tick = std::time::Instant::now();
            needs_draw |= app.canvas_animating() || app.frame_signature() != drawn_frame;
        }
        if app.ui.should_quit {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn add_accepts_flags_before_or_after_the_title() {
        let parsed =
            parse_add_args(&args(&["--tags", "a,b", "Write notes", "--due", "today"])).unwrap();
        assert_eq!(parsed.title, "Write notes");
        assert_eq!(parsed.tags, vec!["a", "b"]);
        assert!(parsed.due.is_some());
    }

    #[test]
    fn add_rejects_a_missing_title_value_or_unknown_flag() {
        assert!(parse_add_args(&args(&["--due", "tomorrow"])).is_err());
        assert!(parse_add_args(&args(&["Milk", "--due"])).is_err());
        assert!(parse_add_args(&args(&["Milk", "--priority", "high"])).is_err());
        assert!(parse_add_args(&args(&["Buy", "milk"])).is_err());
    }

    #[test]
    fn a_bad_due_date_is_an_error() {
        assert!(parse_add_args(&args(&["Milk", "--due", "someday"])).is_err());
    }

    #[test]
    fn list_titles_are_padded_by_display_width() {
        use unicode_width::UnicodeWidthStr;
        for title in [
            "short",
            "日本語のタスク",
            "a very long title that will not fit at all here",
        ] {
            assert_eq!(pad_to_width(title, 20).width(), 20, "{title}");
        }
    }

    #[test]
    fn csv_paths_are_recognised_in_any_case() {
        assert!(is_csv_path("out.CSV"));
        assert!(is_csv_path("dir/sessions.csv"));
        assert!(!is_csv_path("backup.json"));
    }

    #[test]
    fn unknown_commands_fail_instead_of_opening_the_tui() {
        assert_eq!(
            handle_cli(args(&["void", "lsit"])).unwrap(),
            Some(EXIT_USAGE)
        );
        assert_eq!(handle_cli(args(&["void", "--version"])).unwrap(), Some(0));
        assert_eq!(handle_cli(args(&["void"])).unwrap(), None);
    }
}

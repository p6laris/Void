use std::io::Cursor;
use std::process::Command;
#[cfg(target_os = "windows")]
use std::time::Duration;

use notify_rust::Notification;
use rodio::{Decoder, OutputStream, Sink};

use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;

static AUDIO_TX: OnceLock<mpsc::Sender<&'static [u8]>> = OnceLock::new();

pub fn init_audio() {
    let (tx, rx) = mpsc::channel();
    if AUDIO_TX.set(tx).is_err() {
        return; // Already initialized
    }

    thread::spawn(move || {
        #[cfg(target_os = "linux")]
        let _silencer = StderrSilencer::new();

        let (_stream, stream_handle) = match OutputStream::try_default() {
            Ok(s) => s,
            Err(_) => return, // No audio device available
        };

        #[cfg(target_os = "linux")]
        drop(_silencer);

        // Keep receiving sounds as long as the app runs
        while let Ok(bytes) = rx.recv() {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                let cursor = Cursor::new(bytes);
                if let Ok(decoder) = Decoder::new(cursor) {
                    sink.append(decoder);
                    sink.detach();
                }
            }
        }
    });
}

#[cfg(target_os = "linux")]
struct StderrSilencer {
    original_stderr: libc::c_int,
    null_fd: libc::c_int,
}

#[cfg(target_os = "linux")]
impl StderrSilencer {
    fn new() -> Option<Self> {
        unsafe {
            let original_stderr = libc::dup(libc::STDERR_FILENO);
            if original_stderr < 0 {
                return None;
            }
            let null_fd = libc::open(c"/dev/null".as_ptr() as *const _, libc::O_WRONLY);
            if null_fd < 0 {
                libc::close(original_stderr);
                return None;
            }
            libc::dup2(null_fd, libc::STDERR_FILENO);
            Some(Self {
                original_stderr,
                null_fd,
            })
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for StderrSilencer {
    fn drop(&mut self) {
        unsafe {
            libc::dup2(self.original_stderr, libc::STDERR_FILENO);
            libc::close(self.original_stderr);
            libc::close(self.null_fd);
        }
    }
}

fn play_embedded_sound(bytes: &'static [u8]) -> bool {
    if let Some(tx) = AUDIO_TX.get() {
        tx.send(bytes).is_ok()
    } else {
        false
    }
}

fn play_sound(bytes: &'static [u8], fallback: fn()) {
    if play_embedded_sound(bytes) {
        return;
    }
    thread::spawn(fallback);
}

#[derive(Debug, Clone, Copy)]
pub enum NotifyKind {
    FocusComplete,
    BreakComplete,
    SessionSkipped,
    Info,
}

#[cfg(target_os = "windows")]
fn beep_windows(freq: u32, duration_ms: u32) {
    let _ = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!("[console]::Beep({},{})", freq, duration_ms),
        ])
        .output();
}

#[cfg(target_os = "macos")]
fn beep_macos(sound_name: &str) {
    let _ = Command::new("afplay")
        .args([&format!("/System/Library/Sounds/{}.aiff", sound_name)])
        .output();
}

#[cfg(all(unix, not(target_os = "macos")))]
fn beep_linux() {
    let _ = Command::new("sh").args(["-c", "printf '\\a'"]).output();
}

// -----------------------------------------------------------------------------
// Fallbacks
// -----------------------------------------------------------------------------

fn fallback_success() {
    #[cfg(target_os = "windows")]
    {
        beep_windows(880, 200);
        std::thread::sleep(Duration::from_millis(120));
        beep_windows(1175, 350);
    }
    #[cfg(target_os = "macos")]
    {
        beep_macos("Glass");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        beep_linux();
        let _ = Command::new("paplay")
            .args(["/usr/share/sounds/freedesktop/stereo/complete.oga"])
            .output();
    }
}

fn fallback_soft() {
    #[cfg(target_os = "windows")]
    {
        beep_windows(440, 120);
    }
    #[cfg(target_os = "macos")]
    {
        beep_macos("Tink");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("printf").arg("%b").arg("\u{7}").output();
    }
}

fn fallback_click() {
    #[cfg(target_os = "windows")]
    {
        beep_windows(660, 100);
    }
    #[cfg(target_os = "macos")]
    {
        beep_macos("Pop");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("printf").arg("%b").arg("\u{7}").output();
    }
}

// -----------------------------------------------------------------------------
// Rich Audio Events
// -----------------------------------------------------------------------------

pub fn play_focus_complete() {
    play_sound(
        include_bytes!("../assets/sounds/focus_complete.mp3"),
        fallback_success,
    );
}

pub fn play_break_complete() {
    play_sound(
        include_bytes!("../assets/sounds/break_complete.mp3"),
        fallback_success,
    );
}

pub fn play_task_complete() {
    play_sound(
        include_bytes!("../assets/sounds/task_complete.mp3"),
        fallback_success,
    );
}

pub fn play_start() {
    play_sound(include_bytes!("../assets/sounds/start.mp3"), fallback_click);
}

pub fn play_pause() {
    play_sound(include_bytes!("../assets/sounds/pause.mp3"), fallback_soft);
}

pub fn play_resume() {
    play_sound(include_bytes!("../assets/sounds/resume.mp3"), fallback_click);
}

pub fn play_warning() {
    play_sound(include_bytes!("../assets/sounds/warning.mp3"), fallback_soft);
}

pub fn play_skip() {
    play_sound(include_bytes!("../assets/sounds/skip.mp3"), fallback_click);
}

// -----------------------------------------------------------------------------
// Notifications
// -----------------------------------------------------------------------------

pub fn notify(title: &str, body: &str) {
    notify_typed(NotifyKind::Info, title, body);
}

pub fn notify_typed(kind: NotifyKind, title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    std::thread::spawn(move || {
        let mut n = Notification::new();
        n.summary(&title).body(&body).timeout(8000);

        #[cfg(target_os = "macos")]
        {
            let subtitle = match kind {
                NotifyKind::FocusComplete => "Focus session",
                NotifyKind::BreakComplete => "Break time",
                NotifyKind::SessionSkipped => "Session skipped",
                NotifyKind::Info => "Void",
            };
            n.subtitle(subtitle);
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        {
            use notify_rust::Urgency;
            n.appname("Void");
            match kind {
                NotifyKind::FocusComplete => {
                    n.urgency(Urgency::Normal);
                }
                NotifyKind::BreakComplete | NotifyKind::SessionSkipped => {
                    n.urgency(Urgency::Low);
                }
                NotifyKind::Info => {}
            }
        }

        #[cfg(target_os = "windows")]
        let _ = kind;

        if let Err(e) = n.show() {
            eprintln!("Void notification error: {e}");
        }
    });
}

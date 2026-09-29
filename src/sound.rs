use std::io::Cursor;
#[cfg(not(target_os = "windows"))]
use std::process::Command;
#[cfg(target_os = "windows")]
use std::time::Duration;

use notify_rust::Notification;
use rodio::{Decoder, OutputStream, Sink};

use std::sync::mpsc;
use std::sync::OnceLock;
use std::thread;

/// A sound to play, and the system-beep fallback if audio output is unavailable.
type SoundRequest = (&'static [u8], fn());

static AUDIO_TX: OnceLock<mpsc::Sender<SoundRequest>> = OnceLock::new();

pub fn init_audio() {
    // Just register the channel — audio thread starts lazily on first sound
    let (tx, rx) = mpsc::channel();
    if AUDIO_TX.set(tx).is_err() {
        return;
    }

    thread::spawn(move || {
        // Block until the very first sound request arrives
        let Ok(first) = rx.recv() else {
            return;
        };

        #[cfg(target_os = "linux")]
        let _silencer = StderrSilencer::new();

        let stream = OutputStream::try_default();

        #[cfg(target_os = "linux")]
        drop(_silencer);

        let Ok((_stream, stream_handle)) = stream else {
            // No audio device: keep serving requests with system beeps, first included.
            crate::log::log_error("audio output unavailable; using system beeps");
            for (_, fallback) in std::iter::once(first).chain(rx.iter()) {
                fallback();
            }
            return;
        };

        for (bytes, _) in std::iter::once(first).chain(rx.iter()) {
            if let Ok(sink) = Sink::try_new(&stream_handle) {
                if let Ok(decoder) = Decoder::new(Cursor::new(bytes)) {
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

fn play_sound(bytes: &'static [u8], fallback: fn()) {
    let sent = AUDIO_TX
        .get()
        .is_some_and(|tx| tx.send((bytes, fallback)).is_ok());
    if !sent {
        thread::spawn(fallback);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NotifyKind {
    FocusComplete,
    BreakComplete,
    SessionSkipped,
    Info,
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn Beep(frequency: u32, duration_ms: u32) -> i32;
}

#[cfg(target_os = "windows")]
fn beep_windows(freq: u32, duration_ms: u32) {
    // SAFETY: Beep takes two plain integers and has no memory preconditions.
    unsafe {
        Beep(freq, duration_ms);
    }
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
    play_sound(
        include_bytes!("../assets/sounds/resume.mp3"),
        fallback_click,
    );
}

pub fn play_warning() {
    play_sound(
        include_bytes!("../assets/sounds/warning.mp3"),
        fallback_soft,
    );
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
            crate::log::log_error(&format!("notification failed: {e}"));
        }
    });
}

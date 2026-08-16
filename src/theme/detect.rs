//! Zero-overhead OS appearance detection (Light vs Dark).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTheme {
    Dark,
    Light,
}

impl SystemTheme {
    pub fn is_dark(self) -> bool {
        self == SystemTheme::Dark
    }

    pub fn is_light(self) -> bool {
        self == SystemTheme::Light
    }
}

/// Detects the current operating system's light/dark mode preference.
pub fn detect_system_theme() -> SystemTheme {
    #[cfg(target_os = "windows")]
    {
        if let Some(theme) = detect_windows_theme() {
            return theme;
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(theme) = detect_macos_theme() {
            return theme;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(theme) = detect_linux_theme() {
            return theme;
        }
    }

    // Fallback: check COLORFGBG terminal environment variable (format: "fg;bg", e.g. "15;0")
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg_str) = colorfgbg.split(';').next_back() {
            if let Ok(bg_num) = bg_str.trim().parse::<u8>() {
                // Background color 0-6 or 8 is dark, 7 or 15 is light
                if bg_num == 7 || bg_num == 15 {
                    return SystemTheme::Light;
                } else {
                    return SystemTheme::Dark;
                }
            }
        }
    }

    // Default fallback is Dark
    SystemTheme::Dark
}

#[cfg(target_os = "windows")]
fn detect_windows_theme() -> Option<SystemTheme> {
    use std::process::Command;
    // Command line `reg query` is fast and requires zero extra C/Win32 dependencies
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            "/v",
            "AppsUseLightTheme",
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // "0x0" indicates Dark theme, "0x1" indicates Light theme
        if stdout.contains("0x1") {
            return Some(SystemTheme::Light);
        } else if stdout.contains("0x0") {
            return Some(SystemTheme::Dark);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn detect_macos_theme() -> Option<SystemTheme> {
    use std::process::Command;
    let output = Command::new("defaults")
        .args(["read", "-g", "AppleInterfaceStyle"])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().eq_ignore_ascii_case("Dark") {
            return Some(SystemTheme::Dark);
        }
    }
    // If key is missing or not "Dark", macOS is in Light mode
    Some(SystemTheme::Light)
}

#[cfg(target_os = "linux")]
fn detect_linux_theme() -> Option<SystemTheme> {
    use std::process::Command;
    // Try gsettings for GNOME / cosmic / cinnamon
    if let Ok(output) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("prefer-dark") || stdout.contains("dark") {
                return Some(SystemTheme::Dark);
            } else if stdout.contains("prefer-light")
                || stdout.contains("light")
                || stdout.contains("default")
            {
                return Some(SystemTheme::Light);
            }
        }
    }
    None
}

//! Readability checks for the built-in themes.

use ratatui::style::Color;

fn luminance(c: Color) -> f64 {
    let Color::Rgb(r, g, b) = c else {
        panic!("theme colours are RGB");
    };
    let channel = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

fn contrast(a: Color, b: Color) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn comment_text_is_readable_in_every_builtin_theme() {
    let catalog = void::theme::ThemeCatalog::load();
    for id in [
        "light",
        "catppuccin-latte",
        "dark",
        "catppuccin-mocha",
        "matrix",
        "polaris",
    ] {
        let theme = void::theme::resolve(id, &catalog).unwrap();
        let ratio = contrast(theme.comment, theme.bg);
        assert!(
            ratio >= 3.0,
            "{id}: comment contrast {ratio:.2} is below 3:1"
        );
    }
}

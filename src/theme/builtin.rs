use super::{Theme, ThemeVariant};

pub const BUILTINS: &[(&str, &str, ThemeVariant)] = &[
    ("matrix", "Matrix", ThemeVariant::Dark),
    ("dark", "Dark", ThemeVariant::Dark),
    ("light", "Light", ThemeVariant::Light),
    ("polaris", "Polaris", ThemeVariant::Dark),
];

pub fn builtin_theme(id: &str) -> Option<Theme> {
    match id {
        "matrix" => Some(Theme::matrix()),
        "dark" => Some(Theme::dark()),
        "light" => Some(Theme::light()),
        "polaris" => Some(Theme::polaris()),
        _ => None,
    }
}

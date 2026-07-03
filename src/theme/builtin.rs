use super::Theme;

pub const BUILTINS: &[(&str, &str)] = &[
    ("matrix", "Matrix"),
    ("dark", "Dark"),
    ("light", "Light"),
    ("polaris", "Polaris"),
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

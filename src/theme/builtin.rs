use super::tokens::ThemeTokens;
use super::Theme;

pub const BUILTINS: &[(&str, &str)] = &[
    ("matrix", "Matrix"),
    ("dark", "Dark"),
    ("light", "Light"),
    ("polaris", "Polaris"),
];

pub fn builtin_tokens(id: &str) -> Option<ThemeTokens> {
    let theme = match id {
        "matrix" => Theme::matrix(),
        "dark" => Theme::dark(),
        "light" => Theme::light(),
        "polaris" => Theme::polaris(),
        _ => return None,
    };
    Some(theme.into_tokens())
}

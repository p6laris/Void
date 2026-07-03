use chrono::Local;

pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

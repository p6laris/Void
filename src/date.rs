use chrono::{Local, NaiveDate};

pub fn today_naive() -> NaiveDate {
    Local::now().date_naive()
}

pub fn today_str() -> String {
    format_date(today_naive())
}

pub fn tomorrow_str() -> String {
    format_date(today_naive() + chrono::Duration::days(1))
}

pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

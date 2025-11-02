use chrono::NaiveDateTime;

pub fn format_date(date: &NaiveDateTime) -> String {
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}

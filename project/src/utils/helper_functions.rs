use chrono::NaiveDateTime;

pub fn format_date(date: &NaiveDateTime) -> String {
    date.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn assert_parent_dir_exists(file_path: &str) -> Result<(), String> {
    let path = std::path::Path::new(file_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }
    Ok(())
}

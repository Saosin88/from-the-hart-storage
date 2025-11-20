pub fn calculate_folder_prefix(file_path: &str) -> String {
    let segments: Vec<&str> = file_path.split('/').collect();
    if segments.len() > 1 {
        segments[..segments.len() - 1].join("/") + "/"
    } else {
        String::new()
    }
}

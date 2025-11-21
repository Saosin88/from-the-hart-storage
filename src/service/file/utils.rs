pub fn calculate_folder_prefix(file_path: &str) -> String {
    let segments: Vec<&str> = file_path.split('/').collect();
    if segments.len() > 1 {
        segments[..segments.len() - 1].join("/") + "/"
    } else {
        "".to_string()
    }
}

pub fn get_ancestor_folder_paths(file_path: &str) -> Vec<String> {
    let folder_prefix = calculate_folder_prefix(file_path);

    if folder_prefix.is_empty() {
        return vec![];
    }

    let mut ancestors = vec![];
    let segments: Vec<&str> = folder_prefix
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    for i in 1..=segments.len() {
        let path = segments[..i].join("/") + "/";
        ancestors.push(path);
    }

    ancestors
}

pub fn get_parent_folder_path(full_folder_path: &str) -> String {
    if full_folder_path.is_empty() {
        return "".to_string();
    }

    let segments: Vec<&str> = full_folder_path
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if segments.len() > 1 {
        segments[..segments.len() - 1].join("/") + "/"
    } else {
        "".to_string()
    }
}

pub fn get_folder_name(full_folder_path: &str) -> String {
    if full_folder_path.contains('/') {
        full_folder_path
            .trim_end_matches('/')
            .split('/')
            .next_back()
            .unwrap_or(full_folder_path.trim_end_matches('/'))
            .to_string()
    } else {
        full_folder_path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_folder_prefix_nested() {
        assert_eq!(
            calculate_folder_prefix("media/photos/2024/vacation/img.jpg"),
            "media/photos/2024/vacation/"
        );
    }

    #[test]
    fn test_calculate_folder_prefix_single_level() {
        assert_eq!(calculate_folder_prefix("media/img.jpg"), "media/");
    }

    #[test]
    fn test_calculate_folder_prefix_root_file() {
        assert_eq!(calculate_folder_prefix("img.jpg"), "");
    }

    #[test]
    fn test_get_ancestor_folder_paths_nested() {
        let result = get_ancestor_folder_paths("media/photos/2024/vacation/img.jpg");
        assert_eq!(
            result,
            vec![
                "media/",
                "media/photos/",
                "media/photos/2024/",
                "media/photos/2024/vacation/"
            ]
        );
    }

    #[test]
    fn test_get_ancestor_folder_paths_single_level() {
        let result = get_ancestor_folder_paths("media/img.jpg");
        assert_eq!(result, vec!["media/"]);
    }

    #[test]
    fn test_get_ancestor_folder_paths_root_file() {
        let result = get_ancestor_folder_paths("img.jpg");
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_get_parent_folder_path_nested() {
        assert_eq!(
            get_parent_folder_path("media/photos/2024/"),
            "media/photos/"
        );
    }

    #[test]
    fn test_get_parent_folder_path_single_level() {
        assert_eq!(get_parent_folder_path("media/"), "");
    }

    #[test]
    fn test_get_parent_folder_path_root() {
        assert_eq!(get_parent_folder_path(""), "");
    }

    #[test]
    fn test_get_folder_name_nested() {
        assert_eq!(get_folder_name("media/photos/2024/"), "2024");
    }

    #[test]
    fn test_get_folder_name_single_level() {
        assert_eq!(get_folder_name("media/"), "media");
    }

    #[test]
    fn test_get_folder_name_no_slash() {
        assert_eq!(get_folder_name("media"), "media");
    }
}

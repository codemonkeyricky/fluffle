use lazy_static::lazy_static;
use std::sync::RwLock;
use tracing;

lazy_static! {
    static ref APP_NAME: RwLock<String> = RwLock::new("coding".to_string());
}

/// Validate app name to prevent path traversal
pub fn is_valid_app_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Disallow path traversal components and slashes
    if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return false;
    }
    // Optional: allow alphanumeric, hyphen, underscore, dot
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Get the current app name
pub fn get_app_name() -> String {
    APP_NAME.read().unwrap().clone()
}

/// Set the app name (should be called early in startup)
pub fn set_app_name(name: &str) {
    if !is_valid_app_name(name) {
        tracing::warn!(
            "Invalid app name '{}', using default 'coding' instead",
            name
        );
        let mut app_name = APP_NAME.write().unwrap();
        *app_name = "coding".to_string();
        return;
    }
    let mut app_name = APP_NAME.write().unwrap();
    *app_name = name.to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_app_names() {
        assert!(is_valid_app_name("coding"));
        assert!(is_valid_app_name("my-app"));
        assert!(is_valid_app_name("my_app"));
        assert!(is_valid_app_name("my.app"));
        assert!(is_valid_app_name("app123"));
    }

    #[test]
    fn test_invalid_app_names() {
        assert!(!is_valid_app_name(""));
        assert!(!is_valid_app_name("."));
        assert!(!is_valid_app_name(".."));
        assert!(!is_valid_app_name("app/name"));
        assert!(!is_valid_app_name("app\\name"));
        assert!(!is_valid_app_name("app name"));
        assert!(!is_valid_app_name("app@name"));
    }

    #[test]
    fn test_set_app_name() {
        // Reset to default before test
        {
            let mut app_name = APP_NAME.write().unwrap();
            *app_name = "coding".to_string();
        }

        set_app_name("test-app");
        assert_eq!(get_app_name(), "test-app");

        // Invalid name should fall back to coding
        set_app_name("..");
        assert_eq!(get_app_name(), "coding");

        // Reset to default after test
        {
            let mut app_name = APP_NAME.write().unwrap();
            *app_name = "coding".to_string();
        }
    }
}

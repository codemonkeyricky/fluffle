use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub working_directory: std::path::PathBuf,
    pub permissions: Vec<String>,
}

#[derive(Debug)]
pub struct ToolResult {
    success: bool,
    output: String,
    error: Option<String>,
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

pub type ToolParameters = Value;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult::success("test output");
        assert!(result.is_success());
        assert_eq!(result.output(), "test output");
        assert_eq!(result.error_message(), None);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("test error");
        assert!(!result.is_success());
        assert_eq!(result.output(), "");
        assert_eq!(result.error_message(), Some("test error"));
    }

    #[test]
    fn test_tool_result_error_with_empty_string() {
        let result = ToolResult::error("");
        assert!(!result.is_success());
        assert_eq!(result.output(), "");
        assert_eq!(result.error_message(), Some(""));
    }
}

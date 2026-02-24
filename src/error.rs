use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to load config: {0}")]
    ConfigLoad(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("AI error: {0}")]
    Ai(String),

    #[error("Tool iteration limit exceeded: {0}")]
    ToolIterationLimit(u32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::ConfigLoad("test".to_string());
        assert_eq!(err.to_string(), "Failed to load config: test");
    }

    #[test]
    fn test_tool_iteration_limit_error() {
        let err = Error::ToolIterationLimit(5);
        assert_eq!(err.to_string(), "Tool iteration limit exceeded: 5");
    }
}

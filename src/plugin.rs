use std::sync::Arc;
use async_trait::async_trait;
use crate::types::{ToolContext, ToolResult, ToolParameters};

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> ToolParameters;
    async fn execute(&self, ctx: &ToolContext, params: ToolParameters) -> ToolResult;
}

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn tools(&self) -> Vec<Arc<dyn Tool>>;
}

// Inventory registration
inventory::collect!(&'static dyn Plugin);

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serde_json::json;

    #[test]
    fn test_tool_trait_methods() {
        struct TestTool;

        #[async_trait]
        impl Tool for TestTool {
            fn name(&self) -> &'static str { "test" }
            fn description(&self) -> &'static str { "test tool" }
            fn parameters(&self) -> ToolParameters { json!({}) }
            async fn execute(&self, _ctx: &ToolContext, _params: ToolParameters) -> ToolResult {
                ToolResult::success("test")
            }
        }

        let tool = TestTool;
        assert_eq!(tool.name(), "test");
        assert_eq!(tool.description(), "test tool");
    }
}
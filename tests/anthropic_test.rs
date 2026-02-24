use nanocode::ai::{AiProvider, AnthropicProvider, Message, ToolDefinition};
use serde_json::json;

#[tokio::test]
#[ignore = "Requires Anthropic API key"]
async fn test_anthropic_provider_creation() {
    let provider = AnthropicProvider::new(None);
    assert!(provider.is_ok());
}

#[tokio::test]
async fn test_anthropic_provider_trait() {
    let provider = AnthropicProvider::new(None).unwrap();
    let messages = vec![Message::user("test")];
    let tools = vec![ToolDefinition {
        name: "test_tool".to_string(),
        description: "Test tool".to_string(),
        parameters: json!({}),
    }];

    let result = provider.complete_with_tools(&messages, &tools).await;
    assert!(result.is_err());
}

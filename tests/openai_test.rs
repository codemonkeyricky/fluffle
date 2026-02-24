use nanocode::ai::{AiProvider, Message, ToolDefinition, OpenAiProvider};
use serde_json::json;

#[tokio::test]
#[ignore = "Requires OpenAI API key"]
async fn test_openai_provider_creation() {
    let provider = OpenAiProvider::new(None);
    assert!(provider.is_ok());
}

#[tokio::test]
#[ignore = "Requires OpenAI API key"]
async fn test_openai_provider_trait() {
    let provider = OpenAiProvider::new(None).unwrap();
    let messages = vec![Message::user("test")];
    let tools = vec![ToolDefinition {
        name: "test_tool".to_string(),
        description: "Test tool".to_string(),
        parameters: json!({}),
    }];
    let result = provider.complete_with_tools(&messages, &tools).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_openai_provider_base_url() {
    // Test that provider can be created with OPENAI_API_BASE environment variable
    std::env::set_var("OPENAI_API_BASE", "http://localhost:8080");

    let provider = OpenAiProvider::new(None);
    assert!(provider.is_ok());

    // Clean up
    std::env::remove_var("OPENAI_API_BASE");
}
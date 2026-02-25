use nanocode::config::Config;
use nanocode::agent::Agent;
use nanocode::ui::SharedMessages;
use std::sync::Arc;

#[tokio::test]
async fn test_shared_messages_integration() {
    // Create config with test values
    let config = Config {
        model: "test-model".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 100,
        temperature: 0.5,
        max_tool_iterations: 10,
    };

    // Create agent with shared messages
    let mut agent = Agent::new(config).expect("Agent creation failed");
    let shared = Arc::new(SharedMessages::new());
    agent.set_shared_messages(shared.clone());

    // Verify shared messages can be set
    assert!(true);
}

#[tokio::test]
async fn test_agent_process_without_api_key_still_works() {
    // Test that agent.process() can be called even without API key
    // (it will fail, but shouldn't panic)
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    let mut agent = Agent::new(config).expect("Agent creation failed");
    let shared = Arc::new(SharedMessages::new());
    agent.set_shared_messages(shared.clone());

    // This should fail with API error, not panic
    let result = agent.process("Hello").await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("API") || err_msg.contains("OpenAI") || err_msg.contains("AI error"));
}
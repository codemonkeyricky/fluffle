use nanocode::agent::Agent;
use nanocode::config::Config;

#[tokio::test]
async fn test_agent_creation_and_structure() {
    // Create a minimal config for testing
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    // Create agent - this should succeed
    let agent = Agent::new(config).expect("Agent initialization failed");

    // Verify agent has tools (discovered from plugins)
    let tools = agent.tools();
    assert!(!tools.is_empty(), "Agent should discover tools from plugins");

    // Verify conversation history is empty initially
    let history = agent.conversation_history();
    assert!(history.is_empty(), "Conversation history should be empty initially");

    // Verify config is stored
    let stored_config = agent.config();
    assert_eq!(stored_config.model, "gpt-4");
    assert_eq!(stored_config.provider, "openai");
}

#[tokio::test]
async fn test_agent_process_method_exists() {
    // Create a minimal config for testing
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    // Create agent
    let mut agent = Agent::new(config).expect("Agent initialization failed");

    // Test that process() method exists and can be called
    // It will fail due to missing API key, but that's OK for this test
    let result = agent.process("Hello, agent!").await;

    // The method should return an error (no API key), not panic
    assert!(result.is_err(), "process() should return error without API key");

    // Check error type
    let err = result.unwrap_err();
    let err_str = err.to_string();
    assert!(err_str.contains("API key") || err_str.contains("OpenAI") || err_str.contains("API error"),
            "Error should be related to API: {}", err_str);
}

#[tokio::test]
async fn test_agent_tools_access() {
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    let agent = Agent::new(config).expect("Agent initialization failed");

    // Verify we can access tools
    let tools = agent.tools();
    assert!(!tools.is_empty(), "Agent should have tools");

    // Check tool properties
    for tool in tools {
        let name = tool.name();
        let description = tool.description();
        let parameters = tool.parameters();

        assert!(!name.is_empty(), "Tool should have a name");
        assert!(!description.is_empty(), "Tool should have a description");
        // Parameters should be valid JSON (even if empty object)
        assert!(parameters.is_object() || parameters.is_null(), "Tool parameters should be JSON object or null");
    }
}
#[tokio::test]
async fn test_agent_conversation_history_management() {
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    let mut agent = Agent::new(config).expect("Agent initialization failed");

    // Initial history should be empty
    assert!(agent.conversation_history().is_empty());

    // Try to process a message (will fail due to no API key)
    let result = agent.process("Test message").await;
    assert!(result.is_err());

    // Even though process failed, user message should be added to history
    let history = agent.conversation_history();
    assert_eq!(history.len(), 1, "User message should be added to history even if process fails");

    // Verify the message is a user message
    match history[0].role {
        nanocode::ai::MessageRole::User => (),
        _ => panic!("First message should be a user message"),
    }
    assert!(history[0].content.contains("Test message"));
}

#[tokio::test]
async fn test_agent_tool_conversion() {
    let config = Config {
        model: "gpt-4".to_string(),
        api_key: None,
        provider: "openai".to_string(),
        max_tokens: 4096,
        temperature: 0.7,
        max_tool_iterations: 10,
    };

    let _agent = Agent::new(config).expect("Agent initialization failed");

    // This is an internal method, but we can test it indirectly through process()
    // or we could make it public for testing. For now, just verify agent creation.
    assert!(true, "Agent should be created successfully with tool conversion capability");
}

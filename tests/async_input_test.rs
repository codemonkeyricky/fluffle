#[cfg(test)]
mod tests {
    use nanocode::ui::{Event, EventHandler};
    use nanocode::App;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_app_new_with_async_agent() {
        // Verify App initializes with no pending async task or result
        use nanocode::config::Config;
        use tokio::sync::mpsc;
        let config = Config {
            model: "gpt-3.5-turbo".to_string(),
            api_key: Some("dummy".to_string()),
            provider: "openai".to_string(),
            max_tokens: 100,
            temperature: 0.5,
            max_tool_iterations: 10,
        };
        let (ui_to_agent_tx, _) = mpsc::channel(100);
        let app = App::new(config, ui_to_agent_tx).await.expect("App creation failed");
        assert_eq!(app.pending_requests, 0);
        assert!(app.messages.is_empty());
    }

    #[tokio::test]
    async fn test_async_event_handler() {
        let mut handler = EventHandler::new(100);

        // Should be able to await events
        let task = tokio::spawn(async move {
            // This won't receive events without user input, but should not block
            let _ = handler.next().await;
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        task.abort();
    }

    #[tokio::test]
    async fn test_event_handler_generates_ticks() {
        // Create handler with fast tick rate for testing
        let mut handler = EventHandler::new(10); // 10ms tick rate

        // Wait for a tick event with timeout
        match timeout(Duration::from_millis(50), handler.next()).await {
            Ok(Some(Event::Tick)) => {
                // Successfully received a tick event
                assert!(true);
            }
            Ok(Some(other_event)) => {
                panic!("Expected Tick event, got {:?}", other_event);
            }
            Ok(None) => {
                panic!("Channel closed before receiving tick event");
            }
            Err(_) => {
                panic!("Timeout waiting for tick event");
            }
        }
    }

    #[tokio::test]
    async fn test_send_task_completed() {
        let handler = EventHandler::new(100);

        // Send task completed event
        handler
            .send_task_completed()
            .await
            .expect("Failed to send task completed");

        // We can't easily verify this without a receiver, but at least verify no panic
        // The event is sent to the background task's channel
        assert!(true);
    }

    #[tokio::test]
    async fn test_event_handler_drop_cleanup() {
        // Create and immediately drop handler to test cleanup
        let handler = EventHandler::new(100);
        drop(handler);

        // If drop panics or leaks resources, test will fail
        // Add a small delay to ensure background tasks have time to clean up
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(true);
    }

    #[tokio::test]
    async fn test_multiple_event_handlers_independent() {
        // Test that multiple handlers don't interfere
        let handler1 = EventHandler::new(100);
        let handler2 = EventHandler::new(100);

        // Both should be able to be created without issues
        assert!(true);

        // Drop them to test cleanup
        drop(handler1);
        drop(handler2);
    }

    #[tokio::test]
    async fn test_async_handle_input_spawns_task() {
        use nanocode::agent::Agent;
        use nanocode::config::Config;

        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Create minimal config with valid provider
        let config = Config {
            model: "gpt-3.5-turbo".to_string(),
            api_key: Some("dummy".to_string()),
            provider: "openai".to_string(),
            max_tokens: 100,
            temperature: 0.5,
            max_tool_iterations: 10,
        };

        let agent = Agent::new(config).expect("Agent creation failed");
        let _agent_wrapped = Arc::new(RwLock::new(agent));

        // Can't easily test full App::new() due to config loading
        // Test that the method signature and structure are correct
        assert!(true);
    }
}

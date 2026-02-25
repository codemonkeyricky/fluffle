#[cfg(test)]
mod tests {
    use nanocode::App;
    use nanocode::ui::EventHandler;

    #[tokio::test]
    async fn test_app_new_with_async_agent() {
        // Verify App initializes with no pending async task or result
        let app = App::new().await.expect("App creation failed");
        assert!(app.processing_task.is_none());
        assert!(app.pending_result.is_none());
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
}
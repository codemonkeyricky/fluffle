#[cfg(test)]
mod tests {
    use nanocode::App;

    #[tokio::test]
    async fn test_app_new_with_async_agent() {
        // Verify App initializes with no pending async task or result
        let app = App::new().await.expect("App creation failed");
        assert!(app.processing_task.is_none());
        assert!(app.pending_result.is_none());
    }
}
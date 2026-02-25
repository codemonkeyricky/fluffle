#[cfg(test)]
mod tests {
    use nanocode::App;

    #[tokio::test]
    async fn test_app_new_with_async_agent() {
        // This should fail because Agent doesn't implement Send
        let app = App::new().await.expect("App creation failed");
        assert!(app.processing_task.is_none());
        assert!(app.pending_result.is_none());
    }
}
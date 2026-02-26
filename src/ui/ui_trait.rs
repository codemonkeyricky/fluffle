use crate::error::Result;
use async_trait::async_trait;

/// Trait for UI backends that handle bidirectional communication with agent.
#[async_trait]
pub trait Ui {
    /// Run the UI event loop, processing user input and agent messages.
    async fn run(&mut self) -> Result<()>;
}
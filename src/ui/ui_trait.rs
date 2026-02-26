use crate::error::Result;
use crate::messaging::{AgentToUi, UiToAgent};
use crate::ui::event::Event;
use async_trait::async_trait;
use tokio::sync::mpsc;

/// Trait for UI backends that handle bidirectional communication with agent.
#[async_trait]
pub trait Ui {
    /// Run the UI event loop, processing user input and agent messages.
    async fn run(&mut self) -> Result<()>;

    // Private methods for internal use by implementations
    /// Get a mutable reference to the agent-to-UI receiver.
    fn agent_rx(&mut self) -> &mut mpsc::Receiver<AgentToUi>;
    /// Get a mutable reference to the UI-to-agent sender.
    fn agent_tx(&mut self) -> &mut mpsc::Sender<UiToAgent>;

    /// Receive a message from the agent (default implementation uses agent_rx).
    async fn recv_from_agent(&mut self) -> Option<AgentToUi> {
        self.agent_rx().recv().await
    }

    /// Send a message to the agent (default implementation uses agent_tx).
    async fn send_to_agent(&mut self, msg: UiToAgent) -> std::result::Result<(), mpsc::error::SendError<UiToAgent>> {
        self.agent_tx().send(msg).await
    }

    /// Try to send a message to the agent without waiting (default implementation uses agent_tx).
    fn try_send_to_agent(&mut self, msg: UiToAgent) -> std::result::Result<(), mpsc::error::TrySendError<UiToAgent>> {
        self.agent_tx().try_send(msg)
    }

    /// Try to receive a message from the agent without waiting (default implementation uses agent_rx).
    fn try_recv_from_agent(&mut self) -> std::result::Result<AgentToUi, mpsc::error::TryRecvError> {
        self.agent_rx().try_recv()
    }

    /// Get the next user input event, if any.
    async fn next_user_event(&mut self) -> Option<Event> {
        None
    }
}
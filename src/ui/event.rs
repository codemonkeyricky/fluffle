use crossterm::event::{KeyEvent};
use tokio::sync::mpsc;
use tokio::time::interval;
use std::time::Duration;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Tick,
    TaskCompleted,  // New: Background task finished
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,  // Keep sender for task completion signals
    _task_handle: tokio::task::JoinHandle<()>,  // Background task handle for cleanup
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::channel(100);  // Reasonable buffer size
        let tick_rate = Duration::from_millis(tick_rate);

        // Clone the sender for use in the background task
        let tx_for_task = tx.clone();

        // Spawn the background event processing task
        let task_handle = tokio::spawn(async move {
            let mut interval = interval(tick_rate);

            // Optional polling task for terminal input
            #[cfg(not(test))]
            let maybe_poll_handle = {
                let key_tx = tx_for_task.clone();
                Some(tokio::task::spawn_blocking(move || {
                    loop {
                        // Poll for terminal events with proper error handling
                        match crossterm::event::poll(tick_rate) {
                            Ok(true) => {
                                // Event available, read it
                                match crossterm::event::read() {
                                    Ok(crossterm::event::Event::Key(key)) => {
                                        // Send key event, break on channel error
                                        if key_tx.blocking_send(Event::Key(key)).is_err() {
                                            break; // Channel disconnected
                                        }
                                    }
                                    Ok(_) => {
                                        // Ignore non-key events (mouse, resize) for now
                                    }
                                    Err(e) => {
                                        // Terminal read error, break the polling loop
                                        eprintln!("Terminal read error: {}", e);
                                        break;
                                    }
                                }
                            }
                            Ok(false) => {
                                // No event, continue polling
                            }
                            Err(e) => {
                                // Terminal poll error, break the polling loop
                                eprintln!("Terminal poll error: {}", e);
                                break;
                            }
                        }
                    }
                }))
            };
            #[cfg(test)]
            let maybe_poll_handle: Option<tokio::task::JoinHandle<()>> = None;

            // If we have a polling task, run loop with polling
            if let Some(mut poll_handle) = maybe_poll_handle {
                loop {
                    tokio::select! {
                        // Check if polling task completed (terminal error or channel closed)
                        _ = &mut poll_handle => {
                            // Polling task finished, stop polling but continue ticking
                            break;
                        }
                        // Send tick events
                        _ = interval.tick() => {
                            if tx_for_task.send(Event::Tick).await.is_err() {
                                break; // Channel disconnected
                            }
                        }
                    }
                }
            }

            // Continue with tick-only loop
            loop {
                interval.tick().await;
                if tx_for_task.send(Event::Tick).await.is_err() {
                    break; // Channel disconnected
                }
            }
        });

        Self { rx, tx, _task_handle: task_handle }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub async fn send_task_completed(&self) -> Result<(), mpsc::error::SendError<Event>> {
        self.tx.send(Event::TaskCompleted).await
    }
}

impl Drop for EventHandler {
    fn drop(&mut self) {
        // Abort the background task when EventHandler is dropped
        self._task_handle.abort();
    }
}
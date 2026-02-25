use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc;
use tokio::time::interval;
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    Tick,
    TaskCompleted,  // New: Background task finished
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
    tx: mpsc::Sender<Event>,  // Keep sender for task completion signals
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::channel(100);  // Reasonable buffer size

        let event_tx = tx.clone();
        let tick_rate = Duration::from_millis(tick_rate);

        tokio::spawn(async move {
            let mut interval = interval(tick_rate);

            loop {
                tokio::select! {
                    // Poll for crossterm events (needs to be in blocking thread)
                    _ = tokio::task::spawn_blocking({
                        let key_tx = event_tx.clone();
                        move || {
                            if crossterm::event::poll(tick_rate).unwrap_or(false) {
                                if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                                    let _ = key_tx.blocking_send(Event::Key(key));
                                }
                            }
                        }
                    }) => {},
                    // Send tick events
                    _ = interval.tick() => {
                        if event_tx.send(Event::Tick).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, tx }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }

    pub fn send_task_completed(&self) -> Result<(), mpsc::error::SendError<Event>> {
        self.tx.blocking_send(Event::TaskCompleted)
    }
}
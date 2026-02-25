use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    Tick,
    TaskCompleted,  // New: Background task finished
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(tick_rate);

        thread::spawn(move || loop {
            // Handle poll errors gracefully
            match event::poll(tick_rate) {
                Ok(true) => {
                    if let Ok(CrosstermEvent::Key(key)) = event::read() {
                        if tx.send(Event::Key(key)).is_err() { break; } // Channel disconnected
                    }
                }
                Ok(false) => {} // No event
                Err(_) => break, // Poll error, exit thread
            }

            // Send tick, break if channel is disconnected
            if tx.send(Event::Tick).is_err() {
                break;
            }
        });

        Self { rx }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
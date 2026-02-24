use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub enum Event {
    Key(KeyEvent),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        let tick_rate = Duration::from_millis(tick_rate);

        thread::spawn(move || loop {
            if event::poll(tick_rate).unwrap() {
                if let Ok(CrosstermEvent::Key(key)) = event::read() {
                    tx.send(Event::Key(key)).unwrap();
                }
            }
            tx.send(Event::Tick).unwrap();
        });

        Self { rx }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
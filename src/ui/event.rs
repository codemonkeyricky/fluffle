pub enum Event {
    Key,
}

pub struct EventHandler;

impl EventHandler {
    pub fn new(_tick_rate: u64) -> Self {
        Self
    }
}
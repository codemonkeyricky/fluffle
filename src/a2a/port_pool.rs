use std::collections::VecDeque;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct PortPool {
    available: VecDeque<u16>,
}

impl PortPool {
    pub fn new(range: RangeInclusive<u16>) -> Self {
        Self {
            available: range.collect(),
        }
    }

    pub fn acquire(&mut self) -> Option<u16> {
        self.available.pop_front()
    }

    pub fn release(&mut self, port: u16) {
        self.available.push_back(port);
    }
}

use std::time::{Duration, SystemTime};

pub enum Event {
    Flag(bool),
    Wait,
}
impl Event {
    pub fn wait(&mut self, time : f64) {
        let now = SystemTime::now();
        while now.elapsed().unwrap() < Duration::from_secs_f64(time) {}
    }

    pub fn is_set(self) -> bool {
        match self {
            Event::Flag(true) => return true,
            _ => return false,
        };
    }
}

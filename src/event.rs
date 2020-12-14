use std::time::{Duration, SystemTime};
use std::*;

pub enum Event {
    Flag(bool),
    Wait,
}
impl Event {
    pub fn wait(&mut self, time : i64) {
        let now = SystemTime::now();
        let wait_period = time::Duration::from_millis(50);
        match time {
            -1 => {
                let mut breaker = true;
                while breaker {
                    match self {
                        Event::Flag(true) => breaker = false,
                        _ => thread::sleep(wait_period),
                    }
                }
            },
            _ => while now.elapsed().unwrap() < Duration::from_secs_f64(time as f64) {},
        }
    }

    pub fn is_set(self) -> bool {
        match self {
            Event::Flag(true) => return true,
            _ => return false,
        };
    }
}

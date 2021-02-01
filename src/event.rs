use std::time::{Duration, SystemTime};
use std::*;

#[derive(Clone, Copy)]
pub enum EventEnum {
    Flag(bool),
    Wait,
}

#[derive(Clone, Copy)]
pub struct Event {
    pub t: EventEnum,
}
impl Event {
    /// Give a time in milliseconds to pause the current connection between threads, or give -1 for  time to wait until a different thread has set this enum
    pub fn wait(&self, time: f64) {
        println!("Waiting");
        let now = SystemTime::now();
        let wait_period = time::Duration::from_millis(50);
        if time == -1.0 {
            let mut breaker = true;
            while breaker {
                match self.t {
                    EventEnum::Flag(true) => breaker = false,
                    _ => thread::sleep(wait_period),
                }
            }
        } else {
            while now.elapsed().unwrap() > Duration::from_secs_f64(time as f64) {
                thread::sleep(wait_period);
                println!("Currently at {:?}", now.elapsed().unwrap());
            }
        }
    }

    pub fn is_set(self) -> bool {
        match self.t {
            EventEnum::Flag(true) => return true,
            _ => return false,
        };
    }

    pub fn replace_self(&mut self, replace: EventEnum) {
        self.t = replace.clone()
    }
}

use crate::error::*;
use std::collections::*;
use std::path::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct TimeIt {
    pub timers: HashMap<String, u128>,
    pub paused: HashMap<String, u128>,
}
impl TimeIt {
    pub fn new() -> TimeIt {
        TimeIt {
            timers: HashMap::<String, u128>::new(),
            paused: HashMap::<String, u128>::new(),
        }
    }

    pub fn start(&mut self, name: String) {
        let local_name = name.clone();
        self.timers.entry(name).or_insert(0);
        self.timers.insert(
            local_name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        );
    }

    pub fn pause(&mut self, name: String) {
        let name_copy = name.clone();
        let name_copy_2 = name.clone();
        if self.timers.contains_key(&name_copy) {
            self.paused.entry(name).or_insert(0);
            self.paused.insert(
                name_copy,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    - self.timers.get(&name_copy_2).unwrap(),
            );
        }
    }

    pub fn stop(&mut self, name: String, config_dir: &Path) {
        let name_copy = name.clone();
        if self.timers.contains_key(&name_copy) {
            if let Some(x) = self.timers.get(&name_copy) {
                let mut total = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
                    - *x;
                self.timers.remove(&name_copy);
                if self.paused.contains_key(&name_copy) {
                    total = total + self.paused.get(&name_copy).unwrap();
                    self.paused.remove(&name_copy);
                }
                errlog(
                    config_dir,
                    format!("{} completed in {:.6} seconds", name_copy, total),
                );
            }
        }
    }
}

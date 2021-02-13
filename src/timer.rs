use {
    crate::{config::Config, key::Key},
    once_cell::sync::OnceCell,
    std::{
        sync::Mutex,
        time::{Duration, SystemTime},
    },
};

pub struct Timer {
    pub timestamp: SystemTime,
    pub return_zero: bool,
}
impl Timer {
    pub fn new() -> Self {
        Timer {
            timestamp: SystemTime::now(),
            return_zero: false,
        }
    }

    pub fn stamp(&mut self) {
        self.timestamp = SystemTime::now();
    }

    pub fn not_zero(&mut self, CONFIG: &OnceCell<Mutex<Config>>) -> bool {
        if self.return_zero {
            self.return_zero = false;
            return false;
        }
        match self
            .timestamp
            .checked_add(Duration::from_millis(CONFIG.update_ms as u64))
            .unwrap()
            .duration_since(SystemTime::now())
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn left(&mut self, CONFIG: &OnceCell<Mutex<Config>>) -> Duration {
        match SystemTime::now().duration_since(
            self.timestamp
                .checked_add(Duration::from_millis(CONFIG.update_ms as u64))
                .unwrap(),
        ) {
            Ok(_) => Duration::from_millis(0),
            Err(_) => self
                .timestamp
                .checked_add(Duration::from_millis(CONFIG.update_ms as u64))
                .unwrap()
                .duration_since(SystemTime::now())
                .unwrap(),
        }
    }

    pub fn finish(&mut self, key: &Key, CONFIG: &Config) {
        self.return_zero = true;
        self.timestamp = SystemTime::now()
            .checked_sub(Duration::from_millis(CONFIG.update_ms as u64))
            .unwrap();
        key.break_wait();
    }
}

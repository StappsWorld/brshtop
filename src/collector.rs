use std::*;
use std::sync::mpsc::*;
use crate::event::Event;






pub struct Collector {
    pub stopping: bool,
    pub started: bool,
    pub draw_now: bool,
    pub redraw: bool,
    pub only_draw: bool,
    pub thread: std::thread,
    pub collect_run : Event,
    pub collect_idle: Event,
    pub collect_done: Event,
    pub collect_queue: Vec,
    pub collect_interrupt: bool,
    pub proc_interrupt: bool,
    pub use_draw_list: bool,
} impl Collector {

    pub fn init() {
        let mut collecter_initialize = Collector {
            stopping = false,
            started = false,
            draw_now = false,
            redraw = false,
            only_draw = false,
            thread =
        }
    }

}


use std::*;
use std::sync::mpsc::*;
use crate::event::Event;
use crate::Config;
use crate::Error::*;
use thread_control::*;






pub struct Collector {
    pub stopping: bool,
    pub started: bool,
    pub draw_now: bool,
    pub redraw: bool,
    pub only_draw: bool,
    pub tx: Sender<Event>,
    pub rx: Reciever<Event>,
    pub thread: Option<thread::JoinHandle<()>>,
    pub flag: Flag,
    pub control: Control,
    pub collect_run : Event,
    pub collect_idle: Event,
    pub collect_done: Event,
    pub collect_queue: Vec,
    pub collect_interrupt: bool,
    pub proc_interrupt: bool,
    pub use_draw_list: bool,
} impl Collector {

    pub fn init(tx_build : Sender<Event>, rx_build : Receiver<Event>) {
        let (tx_build, rx_build) = channel();
        let (flag_build, control_build) = make_pair();
        let mut collecter_initialize = Collector {
            stopping = false,
            started = false,
            draw_now = false,
            redraw = false,
            only_draw = false,
            tx = tx_build,
            rx = rx_build,
            flag = flag_build,
            control = control_build,
            thread = None,
            collect_run = Event::Flag(false),
            collect_done = Event::Flag(false),
            collect_idle = Event::Flag(true),
            collect_done = Event::Flag(false),
            collect_queue = Vec::<Collector>::new(),
            collect_interrupt = false,
            proc_interrupt = false,
            use_draw_list = false,
        };
    }

    pub fn start(&mut self, CONFIG : Config, b : Box, t : TimeIt) {
        self.stopping = false;
        self.thread = mpsc::spawn(|| _runner(&self, b, t));
        self.started = true;
    }

    pub fn stop(&mut self) {
        while !this.stopping {
            if this.started && this.flag.alive() {
                this.stopping = true;
                this.started = false;
                this.collect_queue = Vec::<Collector>::new();
                this.collect_idle = Event::Flag(true);
                this.collect_done = Event::Flag(true);
                let now = SystemTime::now();
                while this.control.is_done() {
                    if now.elapsed().unwrap() > 5 {
                        break;
                    }
                }
                
            }
        }
    }

    pub fn _runner(&mut self, b : Box, t : TimeIt) {
        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;

        while !self.stopping {
            if CONFIG.draw_clock && CONFIG.update_ms != 1000{
                b.draw_clock();
            }
            this.collect_run = Event::Wait;
            this.collect_run.wait(0.1);
            if !this.collect_run.is_set() {
                continue;
            }
            draw_buffers = Vec::<String>::new();
            self.collect_interrupt = false;
            self.collect_run = Event::Flag(false);
            self.collect_idle = Event::Flag(false);
            self.collect_done = Event::Flag(false);

            if DEBUG && !debugged {
                t.start("Collect and draw");
            }

            while self.collect_queue {
                let collector = self.collect_queue.pop();
                if !self.only_draw {
                    collector.collect();
                }
                collector.draw();
            }

        }
        

    }

}


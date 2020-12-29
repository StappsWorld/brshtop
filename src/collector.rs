use std::*;
use std::sync::mpsc::*;
use crate::event::Event;
use crate::Config;
use crate::Error::*;
use thread_control::*;
use std::path::*;





pub trait CollTrait {    
    /// Setup collect queue for runner, default: {draw_now: bool = True, interrupt: bool = False, proc_interrupt: bool = False, redraw: bool = False, only_draw: bool = False}
    fn collect<P: AsRef<Path>>(&mut self, collectors : Vec<dyn CollTrait>, CONFIG_DIR : P, draw_now : bool, interrupt : bool, proc_interrupt : bool, redraw : bool, only_draw : bool);
    
    fn draw(&mut self);
}

pub struct Collector {
    pub stopping: bool,
    pub started: bool,
    pub draw_now: bool,
    pub redraw: bool,
    pub only_draw: bool,
    pub tx: Sender<Event>,
    pub rx: Receiver<Event>,
    pub thread: Option<thread::JoinHandle<()>>,
    pub flag: Flag,
    pub control: Control,
    pub collect_run : Event,
    pub collect_idle: Event,
    pub collect_done: Event,
    pub collect_queue: Vec<Box<CollTrait>>,
    pub default_collect_queue: Vec<Box<CollTrait>>,
    pub collect_interrupt: bool,
    pub proc_interrupt: bool,
    pub use_draw_list: bool,
} impl CollTrait for Collector{

    

    

    fn collect<P: AsRef<Path>>(&mut self, collectors : Vec<Box<dyn CollTrait>>, CONFIG : Config, CONFIG_DIR : P, draw_now : bool, interrupt : bool, proc_interrupt : bool, redraw : bool, only_draw : bool) {
        self.collect_interrupt = interrupt;
        self.proc_interrupt = proc_interrupt;
        self.collect_idle = Event::Wait;
        self.collect_idle.wait(-1);
        self.collect_interrupt = false;
        self.proc_interrupt = false;
        self.use_draw_list = false;
        self.draw_now = draw_now;
        self.redraw = redraw;
        self.only_draw = only_draw;

        if collectors.capacity() > 0 {
            self.collect_queue = collectors;
            self.use_draw_list = true;
        } else {
            self.collect_queue = self.default_collect_queue.clone();
        }

        self.collect_run = Event::Flag(true);
    }

} impl Collector {

    fn init(tx_build : Sender<Event>, rx_build : Receiver<Event>) {
        let (tx_build, rx_build) = channel();
        let (flag_build, control_build) = make_pair();
        let mut collecter_initialize = Collector {
            stopping : false,
            started : false,
            draw_now : false,
            redraw : false,
            only_draw : false,
            tx : tx_build,
            rx : rx_build,
            flag : flag_build,
            control : control_build,
            thread : None,
            collect_run : Event::Flag(false),
            collect_done : Event::Flag(false),
            collect_idle : Event::Flag(true),
            collect_queue : Vec::<Box<CollTrait>>::new(),
            default_collect_queue : Vec::<Box<CollTrait>>::new(),
            collect_interrupt : false,
            proc_interrupt : false,
            use_draw_list : false,
        };
    }

    pub fn start(&mut self, CONFIG : Config, b : Box, t : TimeIt, m : Menu, d : Draw) {
        self.stopping = false;
        self.thread = thread::spawn(|| self.runner(&self, b, t));
        self.started = true;
        self.default_collect_queue = vec!{b, t, m, d};
    }

    pub fn stop(&mut self) {
        while !self.stopping {
            if self.started && self.flag.alive() {
                self.stopping = true;
                self.started = false;
                self.collect_queue = Vec::<Collector>::new();
                self.collect_idle = Event::Flag(true);
                self.collect_done = Event::Flag(true);
                let now = time::SystemTime::now();
                while self.control.is_done() {
                    if now.elapsed().unwrap() > 5 {
                        break;
                    }
                }
                
            }
        }
    }

    pub fn runner(&mut self, CONFIG : Config, DEBUG : bool, b : Box, t : TimeIt, m : Menu, d : Draw) {
        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;

        while !self.stopping {
            if CONFIG.draw_clock && CONFIG.update_ms != 1000 {
                b.draw_clock();
            }
            self.collect_run = Event::Wait;
            self.collect_run.wait(0.1);
            if !self.collect_run.is_set() {
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

            while self.collect_queue.capacity() > 0 {
                let collector = self.collect_queue.pop();
                if !self.only_draw {
                    collector.collect();
                }
                collector.draw();

                if self.use_draw_list {
                    draw_buffers.push(collector.buffer);
                }

                if self.collect_interrupt {
                    break;
                }

            }

            if DEBUG && !debugged {
                t.stop("Collect and draw");
                debugged = true;
            }

            if self.draw_now && !m.active && !self.collect_interrupt {
                if self.use_draw_list {
                    d.out(draw_buffers);
                } else {
                    d.out;
                }
            }

            if CONFIG.draw_clock && CONFIG.update_ms == 1000 {
                b.draw_clock();
            }

            self.collect_idle = Event::Flag(true);
            self.collect_done = Event::Flag(true);

        }
        

    }
}


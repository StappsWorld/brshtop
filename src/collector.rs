use {
    crate::{
        brshtop_box::BrshtopBox,
        Config, 
        cpucollector::CpuCollector,
        draw::Draw,
        event::Event, 
        Error::*,
        menu::Menu,
        timeit::TimeIt,
    },
    std::{
        *,
        path::*,
        sync::mpsc::*,
        time::Duration,
    },
    thread_control::*,
};

#[derive(Clone)]
pub enum Collectors {
    CpuCollector(CpuCollector),
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
    pub collect_run: Event,
    pub collect_idle: Event,
    pub collect_done: Event,
    pub collect_queue: Vec<Collectors>,
    pub default_collect_queue: Vec<Collectors>,
    pub collect_interrupt: bool,
    pub proc_interrupt: bool,
    pub use_draw_list: bool,
}
impl Collector {
    pub fn new(tx_build: Sender<Event>, rx_build: Receiver<Event>) {
        let (tx_build, rx_build) = channel();
        let (flag_build, control_build) = make_pair();
        let mut collecter_initialize = Collector {
            stopping: false,
            started: false,
            draw_now: false,
            redraw: false,
            only_draw: false,
            tx: tx_build,
            rx: rx_build,
            flag: flag_build,
            control: control_build,
            thread: None,
            collect_run: Event::Flag(false),
            collect_done: Event::Flag(false),
            collect_idle: Event::Flag(true),
            collect_queue: Vec::<Collectors>::new(),
            default_collect_queue: Vec::<Collectors>::new(),
            collect_interrupt: false,
            proc_interrupt: false,
            use_draw_list: false,
        };
    }

    pub fn collect<P: AsRef<Path>>(
        &mut self,
        collectors: Vec<Collectors>,
        CONFIG: Config,
        CONFIG_DIR: P,
        draw_now: bool,
        interrupt: bool,
        proc_interrupt: bool,
        redraw: bool,
        only_draw: bool,
    ) {
        self.collect_interrupt = interrupt;
        self.proc_interrupt = proc_interrupt;
        self.collect_idle = Event::Wait;
        self.collect_idle.wait(-1.0);
        self.collect_interrupt = false;
        self.proc_interrupt = false;
        self.use_draw_list = false;
        self.draw_now = draw_now;
        self.redraw = redraw;
        self.only_draw = only_draw;

        if collectors.len() > 0 {
            self.collect_queue = collectors;
            self.use_draw_list = true;
        } else {
            self.collect_queue = self.default_collect_queue.clone();
        }

        self.collect_run = Event::Flag(true);
    }

    pub fn start(&mut self, CONFIG: Config, DEBUG : bool, collectors : Vec<Collectors>, brshtop_box : BrshtopBox, timeit : TimeIt, menu : Menu, draw : Draw) {
        self.stopping = false;
        self.thread = thread::spawn(|| self.runner(&self, CONFIG, DEBUG, brshtop_box, timeit, menu, draw));
        self.started = true;
        self.default_collect_queue = collectors.clone();
    }

    pub fn stop(&mut self) {
        while !self.stopping {
            if self.started && self.flag.alive() {
                self.stopping = true;
                self.started = false;
                self.collect_queue = Vec::<Collectors>::new();
                self.collect_idle = Event::Flag(true);
                self.collect_done = Event::Flag(true);
                let now = time::SystemTime::now();
                while self.control.is_done() {
                    if now.elapsed().unwrap() > Duration::new(5, 0) {
                        break;
                    }
                }
            }
        }
    }

    pub fn runner(&mut self, CONFIG: Config, DEBUG: bool, brshtop_box: BrshtopBox, timeit: TimeIt, menu: Menu, draw: Draw) {
        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;

        while !self.stopping {
            if CONFIG.draw_clock != String::default() && CONFIG.update_ms != 1000 {
                brshtop_box.draw_clock();
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
                timeit.start("Collect and draw");
            }

            while self.collect_queue.capacity() > 0 {
                let collector = self.collect_queue.pop().unwrap();
                if !self.only_draw {
                    match collector {
                        Collectors::CpuCollector(c) => c.collect(),
                    }
                }
                match collector {
                    Collectors::CpuCollector(c) => c.draw(),
                }

                if self.use_draw_list {
                    
                    draw_buffers.push(match collector {
                        Collectors::CpuCollector(c) => c.buffer,
                    });
                }

                if self.collect_interrupt {
                    break;
                }
            }

            if DEBUG && !debugged {
                timeit.stop("Collect and draw");
                debugged = true;
            }

            if self.draw_now && !menu.active && !self.collect_interrupt {
                if self.use_draw_list {
                    draw.out(draw_buffers);
                } else {
                    draw.out();
                }
            }

            if CONFIG.draw_clock != String::default() && CONFIG.update_ms == 1000 {
                brshtop_box.draw_clock();
            }

            self.collect_idle = Event::Flag(true);
            self.collect_done = Event::Flag(true);
        }
    }
}

use crate::{cpucollector, netbox};

use {
    crate::{
        brshtop_box::BrshtopBox, config::{Config, ViewMode}, cpubox::CpuBox, cpucollector::CpuCollector,
        draw::Draw, event::Event, graph::Graphs, key::Key, menu::Menu, meter::Meters, netbox::NetBox, netcollector::NetCollector, proccollector::ProcCollector, term::Term, theme::Theme, timeit::TimeIt,
    },
    std::{path::*, sync::mpsc::*, time::Duration, *},
    thread_control::*,
};

#[derive(Clone)]
pub enum Collectors<'a> {
    CpuCollector(&'a CpuCollector),
    NetCollector(&'a NetCollector),
    ProcCollector(&'a ProcCollector),
}

pub struct Collector{
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
    pub fn new() -> Self {
        let (tx_build, rx_build) = channel();
        let (flag_build, control_build) = make_pair();
        Collector {
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
        }
    }

    /// Defaults draw_now: bool = True, interrupt: bool = False, proc_interrupt: bool = False, redraw: bool = False, only_draw: bool = False
    pub fn collect<P: AsRef<Path>>(
        &mut self,
        collectors: Vec<Collectors>,
        CONFIG: &mut Config,
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

    pub fn start(
        &'static mut self,
        CONFIG: &'static mut Config,
        DEBUG: bool,
        collectors: Vec<Collectors>,
        brshtop_box: &'static mut BrshtopBox,
        timeit: &'static mut TimeIt,
        menu: &'static mut Menu,
        draw: &'static mut Draw,
        term: &'static mut Term,
        config_dir: &'static Path,
        THREADS: u64,
        CORES: u64,
        CORE_MAP: Vec<i32>,
        cpu_box: &'static mut CpuBox,
        key: &'static mut Key,
        THEME: &'static mut Theme,
        ARG_MODE : ViewMode,
        graphs : &'static mut Graphs,
        meters : &'static mut Meters,
        netbox: &'static mut NetBox
    ) {
        self.stopping = false;
        self.thread = Some(thread::spawn(|| {
            self.runner(
                CONFIG,
                DEBUG,
                config_dir,
                THREADS,
                brshtop_box,
                timeit,
                menu,
                draw,
                term,
                CORES,
                CORE_MAP,
                cpu_box,
                key,
                THEME,
                ARG_MODE,
                graphs,
                meters,
                netbox
            )
        }));
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

    pub fn runner(
        &mut self,
        CONFIG: &mut Config,
        DEBUG: bool,
        config_dir: &Path,
        THREADS: u64,
        brshtop_box: &mut BrshtopBox,
        timeit: &mut TimeIt,
        menu: &mut Menu,
        draw: &mut Draw,
        term: &mut Term,
        CORES: u64,
        CORE_MAP: Vec<i32>,
        cpu_box: &mut CpuBox,
        key: &mut Key,
        THEME: &mut Theme,
        ARG_MODE : ViewMode,
        graphs : &mut Graphs,
        meters: &mut Meters,
        netbox: &mut NetBox

    ) {
        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;

        while !self.stopping {
            if CONFIG.draw_clock != String::default() && CONFIG.update_ms != 1000 {
                brshtop_box.draw_clock(false, term, CONFIG, THEME, menu, cpu_box, draw, key);
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
                timeit.start("Collect and draw".to_owned());
            }

            while self.collect_queue.capacity() > 0 {
                let collector = self.collect_queue.pop().unwrap();
                if !self.only_draw {
                    match collector {
                        Collectors::CpuCollector(c) => c.collect(
                            CONFIG,
                            THREADS,
                            config_dir,
                            term,
                            CORES,
                            CORE_MAP,
                            cpu_box,
                            brshtop_box,
                        ),
                        Collectors::NetCollector(n) => n.collect(),
                        Collectors::ProcCollector(p) => p.collect(),
                    }
                }
                match collector {
                    Collectors::CpuCollector(c) => c.draw(
                        cpu_box,
                        CONFIG,
                        key,
                        THEME,
                        term,
                        draw,
                        ARG_MODE,
                        graphs,
                        meters,
                        THREADS,
                        menu,
                        config_dir
                    ),
                    Collectors::NetCollector(_) => netbox.draw_fg(THEME, key, term, CONFIG, draw, graphs, menu),
                    Collectors::ProcCollector(p) => p.draw_fg(),

                }

                if self.use_draw_list {
                    draw_buffers.push(match collector {
                        Collectors::CpuCollector(c) => c.buffer,
                        Collectors::NetCollector(n) => n.buffer,
                        Collectors::ProcCollector(p) => p.buffer,
                    });
                }

                if self.collect_interrupt {
                    break;
                }
            }

            if DEBUG && !debugged {
                timeit.stop("Collect and draw".to_owned(), config_dir);
                debugged = true;
            }

            if self.draw_now && !menu.active && !self.collect_interrupt {
                if self.use_draw_list {
                    draw.out(draw_buffers, false, key);
                } else {
                    draw.out(Vec::<String>::new(), false, key);
                }
            }

            if CONFIG.draw_clock != String::default() && CONFIG.update_ms == 1000 {
                brshtop_box.draw_clock(false, term, CONFIG, THEME, menu, cpu_box, draw, key);
            }

            self.collect_idle = Event::Flag(true);
            self.collect_done = Event::Flag(true);
        }
    }
}
impl Clone for Collector {
    fn clone(&self) -> Self {
        let (tx_build, rx_build) = channel();
        let (flag_build, control_build) = make_pair();
        Collector {
            stopping: self.stopping.clone(),
            started: self.started.clone(),
            draw_now: self.draw_now.clone(),
            redraw: self.redraw.clone(),
            only_draw: self.only_draw.clone(),
            tx: tx_build,
            rx: rx_build,
            thread: None,
            flag: flag_build,
            control: control_build,
            collect_run: self.collect_run.clone(),
            collect_idle: self.collect_idle.clone(),
            collect_done: self.collect_done.clone(),
            collect_queue: self.collect_queue.clone(),
            default_collect_queue: self.default_collect_queue.clone(),
            collect_interrupt: self.collect_interrupt.clone(),
            proc_interrupt: self.proc_interrupt.clone(),
            use_draw_list: self.use_draw_list.clone(),
        }
    }
}

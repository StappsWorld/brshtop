use crate::{cpucollector, netbox, procbox, proccollector};

use {
    crate::{
        brshtop_box::BrshtopBox,
        config::{Config, ViewMode},
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        draw::Draw,
        error::errlog,
        event::{Event, EventEnum},
        graph::Graphs,
        key::Key,
        membox::MemBox,
        memcollector::MemCollector,
        menu::Menu,
        meter::Meters,
        netbox::NetBox,
        netcollector::NetCollector,
        procbox::ProcBox,
        proccollector::ProcCollector,
        term::Term,
        theme::Theme,
        timeit::TimeIt,
        CONFIG_DIR,
    },
    std::{
        path::*,
        sync::{Arc, Mutex},
        time::Duration,
        *,
    },
    thread_control::*,
};

#[derive(Clone, Copy)]
pub enum Collectors {
    CpuCollector,
    NetCollector,
    ProcCollector,
    MemCollector,
}

pub struct Collector {
    stopping: bool,
    started: bool,
    draw_now: bool,
    redraw: bool,
    only_draw: bool,
    pub flag: Flag,
    pub control: Control,
    collect_run: Event,
    collect_idle: Event,
    collect_done: Event,
    collect_queue: Vec<Collectors>,
    default_collect_queue: Vec<Collectors>,
    collect_interrupt: bool,
    proc_interrupt: bool,
    use_draw_list: bool,
}
impl Collector {
    pub fn new() -> Self {
        let (flag_build, control_build) = make_pair();
        Collector {
            stopping: false,
            started: false,
            draw_now: false,
            redraw: false,
            only_draw: false,
            flag: flag_build,
            control: control_build,
            collect_run: Event {
                t: EventEnum::Flag(false),
            },
            collect_done: Event {
                t: EventEnum::Flag(false),
            },
            collect_idle: Event {
                t: EventEnum::Flag(true),
            },
            collect_queue: Vec::<Collectors>::new(),
            default_collect_queue: Vec::<Collectors>::new(),
            collect_interrupt: false,
            proc_interrupt: false,
            use_draw_list: false,
        }
    }

    /// Defaults draw_now: bool = True, interrupt: bool = False, proc_interrupt: bool = False, redraw: bool = False, only_draw: bool = False
    pub fn collect(
        &mut self,
        collectors: Vec<Collectors>,
        draw_now: bool,
        interrupt: bool,
        proc_interrupt: bool,
        redraw: bool,
        only_draw: bool,
    ) {
        self.set_collect_interrupt(interrupt.clone());
        self.set_proc_interrupt(proc_interrupt.clone());
        self.set_collect_interrupt(false);
        self.set_proc_interrupt(false);
        self.set_use_draw_list(false);
        self.set_draw_now(draw_now.clone());
        self.set_redraw(redraw.clone());
        self.set_only_draw(only_draw.clone());

        if collectors.len() > 0 {
            self.set_collect_queue(collectors.clone());
            self.set_use_draw_list(true);
        } else {
            self.set_collect_queue(self.get_default_collect_queue().clone());
        }

        self.set_collect_run(EventEnum::Flag(true));
    }

    pub fn start(
        _self: Arc<Mutex<Collector>>,
        DEBUG: bool,
        ARG_MODE: ViewMode,
        collectors: Vec<Collectors>,
        CONFIG: Arc<Mutex<Config>>,
        brshtop_box: Arc<Mutex<BrshtopBox>>,
        timeit: Arc<Mutex<TimeIt>>,
        menu: Arc<Mutex<Menu>>,
        draw: Arc<Mutex<Draw>>,
        term: Arc<Mutex<Term>>,
        cpu_box: Arc<Mutex<CpuBox>>,
        key: Arc<Mutex<Key>>,
        THEME: Arc<Mutex<Theme>>,
        graphs: Arc<Mutex<Graphs>>,
        meters: Arc<Mutex<Meters>>,
        netbox: Arc<Mutex<NetBox>>,
        procbox: Arc<Mutex<ProcBox>>,
        membox: Arc<Mutex<MemBox>>,
        cpu_collector: Arc<Mutex<CpuCollector>>,
        mem_collector: Arc<Mutex<MemCollector>>,
        net_collector: Arc<Mutex<NetCollector>>,
        proc_collector: Arc<Mutex<ProcCollector>>,
    ) {
        let mut initial_usage = _self.lock().unwrap();
        initial_usage.set_stopping(false);
        drop(initial_usage);
        let mut self_copy = Arc::clone(&_self);
        thread::spawn(move || {
            Collector::runner(
                self_copy,
                CONFIG,
                DEBUG,
                ARG_MODE,
                brshtop_box,
                timeit,
                menu,
                draw,
                term,
                cpu_box,
                key,
                THEME,
                graphs,
                meters,
                netbox,
                procbox,
                membox,
                cpu_collector,
                net_collector,
                proc_collector,
                mem_collector,
            );
        });
        let mut after_usage = _self.lock().unwrap();

        after_usage.set_started(true);
        after_usage.set_default_collect_queue(collectors.clone());
    }

    pub fn stop(&mut self) {
        while !self.get_stopping() {
            if self.get_started() && self.flag.alive() {
                self.set_stopping(true);
                self.set_started(false);
                self.set_collect_queue(Vec::<Collectors>::new());
                self.set_collect_idle(EventEnum::Flag(true));
                self.set_collect_done(EventEnum::Flag(true));
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
        _self: Arc<Mutex<Collector>>,
        CONFIG_mutex: Arc<Mutex<Config>>,
        DEBUG: bool,
        ARG_MODE: ViewMode,
        brshtop_box_mutex: Arc<Mutex<BrshtopBox>>,
        timeit_mutex: Arc<Mutex<TimeIt>>,
        menu_mutex: Arc<Mutex<Menu>>,
        draw_mutex: Arc<Mutex<Draw>>,
        term_mutex: Arc<Mutex<Term>>,
        cpu_box_mutex: Arc<Mutex<CpuBox>>,
        key_mutex: Arc<Mutex<Key>>,
        THEME_mutex: Arc<Mutex<Theme>>,
        graphs_mutex: Arc<Mutex<Graphs>>,
        meters_mutex: Arc<Mutex<Meters>>,
        netbox_mutex: Arc<Mutex<NetBox>>,
        procbox_mutex: Arc<Mutex<ProcBox>>,
        membox_mutex: Arc<Mutex<MemBox>>,
        cpu_collector_mutex: Arc<Mutex<CpuCollector>>,
        net_collector_mutex: Arc<Mutex<NetCollector>>,
        proc_collector_mutex: Arc<Mutex<ProcCollector>>,
        mem_collector_mutex: Arc<Mutex<MemCollector>>,
    ) {
        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;
        let initial_check = _self.lock().unwrap();
        let mut stopping = initial_check.get_stopping();
        drop(initial_check);

        while !stopping {
            thread::sleep(Duration::from_millis(10));
            let mut CONFIG = match CONFIG_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut brshtop_box = match brshtop_box_mutex.try_lock() {
                Ok(b) => b,
                Err(_) => continue,
            };
            let mut timeit = match timeit_mutex.try_lock() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut menu = match menu_mutex.try_lock() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mut draw = match draw_mutex.try_lock() {
                Ok(d) => d,
                Err(_) => continue,
            };
            let mut term = match term_mutex.try_lock() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut cpu_box = match cpu_box_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut key = match key_mutex.try_lock() {
                Ok(k) => k,
                Err(_) => continue,
            };
            let mut THEME = match THEME_mutex.try_lock() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut graphs = match graphs_mutex.try_lock() {
                Ok(g) => g,
                Err(_) => continue,
            };
            let mut meters = match meters_mutex.try_lock() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let mut netbox = match netbox_mutex.try_lock() {
                Ok(n) => n,
                Err(_) => continue,
            };
            let mut procbox = match procbox_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut membox = match membox_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut cpu_collector = match cpu_collector_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut net_collector = match net_collector_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut proc_collector = match proc_collector_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut mem_collector = match mem_collector_mutex.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut self_collector = match _self.try_lock() {
                Ok(c) => c,
                Err(_) => continue,
            };
            errlog("Locked all modules in Collector::runner()".to_owned());

            if CONFIG.draw_clock != String::default() && CONFIG.update_ms != 1000 {
                brshtop_box.draw_clock(
                    false, &term, &CONFIG, &THEME, &menu, &cpu_box, &mut draw, &mut key,
                );
            }
            if !self_collector.get_collect_run().is_set() {
                continue;
            }
            draw_buffers = Vec::<String>::new();
            self_collector.set_collect_interrupt(false);
            self_collector.set_collect_run(EventEnum::Flag(false));
            self_collector.set_collect_idle(EventEnum::Flag(true));
            self_collector.set_collect_done(EventEnum::Flag(false));

            if DEBUG && !debugged {
                timeit.start("Collect and draw".to_owned());
            }

            while self_collector.get_collect_queue().len() > 0 {
                let collector = self_collector.pop_collect_queue();
                if !self_collector.get_only_draw() {
                    match collector {
                        Collectors::CpuCollector => {
                            cpu_collector.collect(&CONFIG, &term, &mut cpu_box, &mut brshtop_box);
                        }
                        Collectors::NetCollector => {
                            net_collector.collect(&CONFIG, &mut netbox);
                        }
                        Collectors::ProcCollector => {
                            proc_collector.collect(&brshtop_box, &CONFIG, &mut procbox);
                        }
                        Collectors::MemCollector => {
                            mem_collector.collect(&CONFIG, &mut membox);
                        }
                    }
                }
                match collector {
                    Collectors::CpuCollector => {
                        cpu_collector.draw(
                            &mut cpu_box,
                            &CONFIG,
                            &mut key,
                            &THEME,
                            &mut term,
                            &mut draw,
                            ARG_MODE,
                            &mut graphs,
                            &mut meters,
                            &mut menu,
                        );
                    }
                    Collectors::NetCollector => {
                        net_collector.draw(
                            &mut netbox,
                            &THEME,
                            &mut key,
                            &term,
                            &CONFIG,
                            &mut draw,
                            &mut graphs,
                            &mut menu,
                        );
                    }
                    Collectors::ProcCollector => {
                        proc_collector.draw(
                            &mut procbox,
                            &CONFIG,
                            &mut key,
                            &THEME,
                            &mut graphs,
                            &term,
                            &mut draw,
                            &mut menu,
                        );
                    }
                    Collectors::MemCollector => {
                        mem_collector.draw(
                            &mut membox,
                            &term,
                            &mut brshtop_box,
                            &CONFIG,
                            &mut meters,
                            &THEME,
                            &mut key,
                            &self_collector,
                            &mut draw,
                            &menu,
                        );
                    }
                }

                if self_collector.get_use_draw_list() {
                    draw_buffers.push(match collector {
                        Collectors::CpuCollector => cpu_collector.get_buffer().clone(),
                        Collectors::NetCollector => net_collector.get_buffer().clone(),
                        Collectors::ProcCollector => proc_collector.buffer.clone(),
                        Collectors::MemCollector => mem_collector.get_buffer().clone(),
                    });
                }

                if self_collector.get_collect_interrupt() {
                    break;
                }
            }

            if DEBUG && !debugged {
                timeit.stop("Collect and draw".to_owned());
                debugged = true;
            }

            if self_collector.get_draw_now()
                && !menu.active
                && !self_collector.get_collect_interrupt()
            {
                if self_collector.get_use_draw_list() {
                    draw.out(draw_buffers.clone(), false, &mut key);
                } else {
                    draw.out(Vec::<String>::new(), false, &mut key);
                }
            }

            if CONFIG.draw_clock != String::default() && CONFIG.update_ms == 1000 {
                brshtop_box.draw_clock(
                    false, &term, &CONFIG, &THEME, &menu, &cpu_box, &mut draw, &mut key,
                );
            }

            self_collector.set_collect_idle(EventEnum::Flag(true));
            self_collector.set_collect_done(EventEnum::Flag(true));
            stopping = self_collector.get_stopping();

        }
    }

    pub fn get_stopping(&self) -> bool {
        self.stopping.clone()
    }

    pub fn set_stopping(&mut self, stopping: bool) {
        self.stopping = stopping.clone()
    }

    pub fn get_started(&self) -> bool {
        self.started.clone()
    }

    pub fn set_started(&mut self, started: bool) {
        self.started = started.clone()
    }

    pub fn get_draw_now(&self) -> bool {
        self.draw_now.clone()
    }

    pub fn set_draw_now(&mut self, draw_now: bool) {
        self.draw_now = draw_now.clone()
    }

    pub fn get_redraw(&self) -> bool {
        self.redraw.clone()
    }

    pub fn set_redraw(&mut self, redraw: bool) {
        self.redraw = redraw.clone()
    }

    pub fn get_only_draw(&self) -> bool {
        self.only_draw.clone()
    }

    pub fn set_only_draw(&mut self, only_draw: bool) {
        self.only_draw = only_draw.clone()
    }

    pub fn get_collect_run(&self) -> Event {
        self.collect_run.clone()
    }

    pub fn set_collect_run(&mut self, collect_run: EventEnum) {
        self.collect_run.replace_self(collect_run);
    }

    pub fn get_collect_run_reference(&self) -> &Event {
        &self.collect_run
    }

    pub fn get_collect_idle(&self) -> Event {
        self.collect_idle.clone()
    }

    pub fn set_collect_idle(&mut self, collect_idle: EventEnum) {
        self.collect_idle.replace_self(collect_idle.clone())
    }

    pub fn get_collect_idle_reference(&self) -> &Event {
        &self.collect_idle
    }

    pub fn get_collect_done(&self) -> Event {
        self.collect_done.clone()
    }

    pub fn set_collect_done(&mut self, collect_done: EventEnum) {
        self.collect_done.replace_self(collect_done.clone())
    }

    pub fn get_collect_done_reference(&self) -> &Event {
        &self.collect_done
    }

    pub fn get_collect_queue(&self) -> Vec<Collectors> {
        self.collect_queue.clone()
    }

    pub fn set_collect_queue(&mut self, collect_queue: Vec<Collectors>) {
        self.collect_queue = collect_queue.clone()
    }

    pub fn push_collect_queue(&mut self, element: Collectors) {
        self.collect_queue.push(element.clone())
    }

    pub fn pop_collect_queue(&mut self) -> Collectors {
        self.collect_queue.pop().unwrap()
    }

    pub fn get_collect_queue_index(&self, index: usize) -> Option<Collectors> {
        match self.get_collect_queue().get(index) {
            Some(c) => Some(c.clone()),
            None => None,
        }
    }
    pub fn set_collect_queue_index(&mut self, index: usize, element: Collectors) -> Option<()> {
        if index > self.get_collect_queue().len() {
            None
        } else {
            self.collect_queue.insert(index.clone(), element.clone());
            Some(())
        }
    }

    pub fn get_default_collect_queue(&self) -> Vec<Collectors> {
        self.default_collect_queue.clone()
    }

    pub fn set_default_collect_queue(&mut self, default_collect_queue: Vec<Collectors>) {
        self.default_collect_queue = default_collect_queue.clone()
    }

    pub fn push_default_collect_queue(&mut self, element: Collectors) {
        self.default_collect_queue.push(element.clone())
    }

    pub fn get_default_collect_queue_index(&self, index: usize) -> Option<Collectors> {
        match self.get_default_collect_queue().get(index) {
            Some(c) => Some(c.clone()),
            None => None,
        }
    }
    pub fn set_default_collect_queue_index(
        &mut self,
        index: usize,
        element: Collectors,
    ) -> Option<()> {
        if index > self.get_default_collect_queue().len() {
            None
        } else {
            self.default_collect_queue
                .insert(index.clone(), element.clone());
            Some(())
        }
    }

    pub fn get_collect_interrupt(&self) -> bool {
        self.collect_interrupt.clone()
    }

    pub fn set_collect_interrupt(&mut self, collect_interrupt: bool) {
        self.collect_interrupt = collect_interrupt.clone()
    }

    pub fn get_proc_interrupt(&self) -> bool {
        self.proc_interrupt.clone()
    }

    pub fn set_proc_interrupt(&mut self, proc_interrupt: bool) {
        self.proc_interrupt = proc_interrupt.clone()
    }

    pub fn get_use_draw_list(&self) -> bool {
        self.use_draw_list.clone()
    }

    pub fn set_use_draw_list(&mut self, use_draw_list: bool) {
        self.use_draw_list = use_draw_list.clone()
    }
}
impl<'a> Clone for Collector {
    fn clone(&self) -> Self {
        let (flag_build, control_build) = make_pair();
        Collector {
            stopping: self.stopping.clone(),
            started: self.started.clone(),
            draw_now: self.draw_now.clone(),
            redraw: self.redraw.clone(),
            only_draw: self.only_draw.clone(),
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

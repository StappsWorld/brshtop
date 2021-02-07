use crate::{cpucollector, netbox, procbox, proccollector};

use {
    crate::{
        brshtop_box::BrshtopBox,
        config::{Config, ViewMode},
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        draw::Draw,
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
    crossbeam,
    once_cell::sync::OnceCell,
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
        CONFIG: &OnceCell<Mutex<Config>>,
        draw_now: bool,
        interrupt: bool,
        proc_interrupt: bool,
        redraw: bool,
        only_draw: bool,
    ) {
        self.set_collect_interrupt(interrupt.clone());
        self.set_proc_interrupt(proc_interrupt.clone());
        self.set_collect_idle(EventEnum::Wait);
        self.get_collect_idle_reference().wait(-1.0);
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
        &mut self,
        CONFIG: &OnceCell<Mutex<Config>>,
        DEBUG: bool,
        collectors: Vec<Collectors>,
        brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
        timeit: &OnceCell<Mutex<TimeIt>>,
        menu: &OnceCell<Mutex<Menu>>,
        draw: &OnceCell<Mutex<Draw>>,
        term: &OnceCell<Mutex<Term>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        key: &OnceCell<Mutex<Key>>,
        THEME: &OnceCell<Mutex<Theme>>,
        ARG_MODE: ViewMode,
        graphs: &OnceCell<Mutex<Graphs>>,
        meters: &OnceCell<Mutex<Meters>>,
        netbox: &OnceCell<Mutex<NetBox>>,
        procbox: &OnceCell<Mutex<ProcBox>>,
        membox: &OnceCell<Mutex<MemBox>>,
        cpu_collector: &OnceCell<Mutex<CpuCollector>>,
        mem_collector: &OnceCell<Mutex<MemCollector>>,
        net_collector: &OnceCell<Mutex<NetCollector>>,
        proc_collector: &OnceCell<Mutex<ProcCollector>>,
    ) {
        self.set_stopping(false);
        match crossbeam::scope(|s| {
            s.spawn(|_| {
                let (flag_build, control_build) = make_pair();
                self.flag = flag_build;
                self.control = control_build;
                self.runner(
                    CONFIG,
                    DEBUG,
                    brshtop_box,
                    timeit,
                    menu,
                    draw,
                    term,
                    cpu_box,
                    key,
                    THEME,
                    ARG_MODE,
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
        }) {
            _ => (),
        };

        self.set_started(true);
        self.set_default_collect_queue(collectors.clone());
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
        &mut self,
        CONFIG_p: &OnceCell<Mutex<Config>>,
        DEBUG: bool,
        brshtop_box_p: &OnceCell<Mutex<BrshtopBox>>,
        timeit_p: &OnceCell<Mutex<TimeIt>>,
        menu_p: &OnceCell<Mutex<Menu>>,
        draw_p: &OnceCell<Mutex<Draw>>,
        term_p: &OnceCell<Mutex<Term>>,
        cpu_box_p: &OnceCell<Mutex<CpuBox>>,
        key_p: &OnceCell<Mutex<Key>>,
        THEME_p: &OnceCell<Mutex<Theme>>,
        ARG_MODE: ViewMode,
        graphs_p: &OnceCell<Mutex<Graphs>>,
        meters_p: &OnceCell<Mutex<Meters>>,
        netbox_p: &OnceCell<Mutex<NetBox>>,
        procbox_p: &OnceCell<Mutex<ProcBox>>,
        membox_p: &OnceCell<Mutex<MemBox>>,
        cpu_collector_p: &OnceCell<Mutex<CpuCollector>>,
        net_collector_p: &OnceCell<Mutex<NetCollector>>,
        proc_collector_p: &OnceCell<Mutex<ProcCollector>>,
        mem_collector_p: &OnceCell<Mutex<MemCollector>>,
    ) {
        let mut CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
        let mut brshtop_box = brshtop_box_p.get().unwrap().lock().unwrap();
        let mut timeit = timeit_p.get().unwrap().lock().unwrap();
        let mut menu = menu_p.get().unwrap().lock().unwrap();
        let mut draw = draw_p.get().unwrap().lock().unwrap();
        let mut term = term_p.get().unwrap().lock().unwrap();
        let mut cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
        let mut THEME = THEME_p.get().unwrap().lock().unwrap();
        let mut graphs = graphs_p.get().unwrap().lock().unwrap();
        let mut meters = meters_p.get().unwrap().lock().unwrap();
        let mut netbox = netbox_p.get().unwrap().lock().unwrap();
        let mut procbox = procbox_p.get().unwrap().lock().unwrap();
        let mut membox = membox_p.get().unwrap().lock().unwrap();
        let mut cpu_collector = cpu_collector_p.get().unwrap().lock().unwrap();
        let mut net_collector = net_collector_p.get().unwrap().lock().unwrap();
        let mut proc_collector = proc_collector_p.get().unwrap().lock().unwrap();
        let mut mem_collector = mem_collector_p.get().unwrap().lock().unwrap();

        let mut draw_buffers = Vec::<String>::new();

        let mut debugged = false;

        while !self.get_stopping() {
            if CONFIG.draw_clock != String::default() && CONFIG.update_ms != 1000 {
                drop(term);
                drop(CONFIG);
                drop(THEME);
                drop(cpu_box);
                drop(draw);
                brshtop_box.draw_clock(
                    false, term_p, CONFIG_p, THEME_p, &menu, cpu_box_p, draw_p, key_p,
                );
                term = term_p.get().unwrap().lock().unwrap();
                CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                THEME = THEME_p.get().unwrap().lock().unwrap();
                cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
                draw = draw_p.get().unwrap().lock().unwrap();
            }
            self.set_collect_run(EventEnum::Wait);
            self.get_collect_run_reference().wait(0.1);
            if !self.get_collect_run().is_set() {
                continue;
            }
            draw_buffers = Vec::<String>::new();
            self.set_collect_interrupt(false);
            self.set_collect_run(EventEnum::Flag(false));
            self.set_collect_idle(EventEnum::Flag(true));
            self.set_collect_done(EventEnum::Flag(false));

            if DEBUG && !debugged {
                timeit.start("Collect and draw".to_owned());
            }

            while self.get_collect_queue().len() > 0 {
                let collector = self.pop_collect_queue();
                if !self.get_only_draw() {
                    match collector {
                        Collectors::CpuCollector => {
                            drop(CONFIG);
                            drop(term);
                            drop(cpu_box);
                            drop(brshtop_box);
                            cpu_collector.collect(CONFIG_p, term_p, cpu_box_p, brshtop_box_p);
                            CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                            term = term_p.get().unwrap().lock().unwrap();
                            cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
                            brshtop_box = brshtop_box_p.get().unwrap().lock().unwrap();
                        }
                        Collectors::NetCollector => {
                            drop(CONFIG);
                            drop(netbox);
                            net_collector.collect(CONFIG_p, netbox_p);
                            CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                            netbox = netbox_p.get().unwrap().lock().unwrap();
                        }
                        Collectors::ProcCollector => {
                            drop(brshtop_box);
                            drop(CONFIG);
                            drop(procbox);
                            proc_collector.collect(brshtop_box_p, CONFIG_p, procbox_p);
                            brshtop_box = brshtop_box_p.get().unwrap().lock().unwrap();
                            CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                            procbox = procbox_p.get().unwrap().lock().unwrap();
                        }
                        Collectors::MemCollector => {
                            drop(CONFIG);
                            drop(membox);
                            mem_collector.collect(CONFIG_p, membox_p);
                            CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                            membox = membox_p.get().unwrap().lock().unwrap();
                        }
                    }
                }
                match collector {
                    Collectors::CpuCollector => {
                        drop(cpu_box);
                        drop(CONFIG);
                        drop(THEME);
                        drop(term);
                        drop(draw);
                        drop(graphs);
                        drop(meters);
                        drop(menu);
                        cpu_collector.draw(
                            cpu_box_p, CONFIG_p, key_p, THEME_p, term_p, draw_p, ARG_MODE,
                            graphs_p, meters_p, menu_p,
                        );
                        cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
                        CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                        THEME = THEME_p.get().unwrap().lock().unwrap();
                        term = term_p.get().unwrap().lock().unwrap();
                        draw = draw_p.get().unwrap().lock().unwrap();
                        graphs = graphs_p.get().unwrap().lock().unwrap();
                        meters = meters_p.get().unwrap().lock().unwrap();
                        menu = menu_p.get().unwrap().lock().unwrap();
                    }
                    Collectors::NetCollector => {
                        drop(netbox);
                        drop(THEME);
                        drop(term);
                        drop(CONFIG);
                        drop(draw);
                        drop(graphs);
                        drop(menu);
                        net_collector.draw(
                            netbox_p, THEME_p, key_p, term_p, CONFIG_p, draw_p, graphs_p, menu_p,
                        );
                        netbox = netbox_p.get().unwrap().lock().unwrap();
                        THEME = THEME_p.get().unwrap().lock().unwrap();
                        term = term_p.get().unwrap().lock().unwrap();
                        CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                        draw = draw_p.get().unwrap().lock().unwrap();
                        graphs = graphs_p.get().unwrap().lock().unwrap();
                        menu = menu_p.get().unwrap().lock().unwrap();
                    }
                    Collectors::ProcCollector => {
                        drop(procbox);
                        drop(CONFIG);
                        drop(THEME);
                        drop(graphs);
                        drop(term);
                        drop(draw);
                        drop(menu);
                        proc_collector.draw(
                            procbox_p, CONFIG_p, key_p, THEME_p, graphs_p, term_p, draw_p, menu_p,
                        );
                        procbox = procbox_p.get().unwrap().lock().unwrap();
                        CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                        THEME = THEME_p.get().unwrap().lock().unwrap();
                        graphs = graphs_p.get().unwrap().lock().unwrap();
                        term = term_p.get().unwrap().lock().unwrap();
                        draw = draw_p.get().unwrap().lock().unwrap();
                        menu = menu_p.get().unwrap().lock().unwrap();
                    }
                    Collectors::MemCollector => {
                        drop(membox);
                        drop(term);
                        drop(brshtop_box);
                        drop(CONFIG);
                        drop(meters);
                        drop(THEME);
                        drop(draw);
                        drop(menu);
                        mem_collector.draw(
                            membox_p,
                            term_p,
                            brshtop_box_p,
                            CONFIG_p,
                            meters_p,
                            THEME_p,
                            key_p,
                            self,
                            draw_p,
                            menu_p,
                        );
                        membox = membox_p.get().unwrap().lock().unwrap();
                        term = term_p.get().unwrap().lock().unwrap();
                        brshtop_box = brshtop_box_p.get().unwrap().lock().unwrap();
                        CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                        meters = meters_p.get().unwrap().lock().unwrap();
                        THEME = THEME_p.get().unwrap().lock().unwrap();
                        draw = draw_p.get().unwrap().lock().unwrap();
                        menu = menu_p.get().unwrap().lock().unwrap();
                    }
                }

                if self.get_use_draw_list() {
                    draw_buffers.push(match collector {
                        Collectors::CpuCollector => cpu_collector.get_buffer().clone(),
                        Collectors::NetCollector => net_collector.get_buffer().clone(),
                        Collectors::ProcCollector => proc_collector.buffer.clone(),
                        Collectors::MemCollector => mem_collector.get_buffer().clone(),
                    });
                }

                if self.get_collect_interrupt() {
                    break;
                }
            }

            if DEBUG && !debugged {
                timeit.stop("Collect and draw".to_owned());
                debugged = true;
            }

            if self.get_draw_now() && !menu.active && !self.get_collect_interrupt() {
                let mut key = key_p.get().unwrap().lock().unwrap();
                if self.get_use_draw_list() {
                    draw.out(draw_buffers, false, &mut key);
                } else {
                    draw.out(Vec::<String>::new(), false, &mut key);
                }
                drop(key);
            }

            if CONFIG.draw_clock != String::default() && CONFIG.update_ms == 1000 {
                drop(term);
                drop(CONFIG);
                drop(THEME);
                drop(cpu_box);
                drop(draw);
                brshtop_box.draw_clock(
                    false, term_p, CONFIG_p, THEME_p, &menu, cpu_box_p, draw_p, key_p,
                );
                term = term_p.get().unwrap().lock().unwrap();
                CONFIG = CONFIG_p.get().unwrap().lock().unwrap();
                THEME = THEME_p.get().unwrap().lock().unwrap();
                cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
                draw = draw_p.get().unwrap().lock().unwrap();
            }

            self.set_collect_idle(EventEnum::Flag(true));
            self.set_collect_done(EventEnum::Flag(true));
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

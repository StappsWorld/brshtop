use {
    crate::{
        brshtop_box::BrshtopBox,
        collector::Collector,
        config::{Config, SortingOption},
        draw::Draw,
        error::{errlog, throw_error},
        floating_humanizer,
        graph::Graphs,
        key::Key,
        menu::Menu,
        procbox::ProcBox,
        term::Term,
        theme::Theme,
        SYSTEM, THREADS,
    },
    core::time::Duration,
    math::round::ceil,
    psutil::{
        process::{os::unix::ProcessExt, *},
        Bytes, Count, Pid,
    },
    std::{cmp::Ordering, collections::HashMap, convert::TryInto, fmt::Display},
};

#[derive(Clone)]
pub enum ProcCollectorDetails {
    Bool(bool),
    Status(Status),
    U32(u32),
    U64(u64),
    I32(i32),
    F32(f32),
    F64(f64),
    String(String),
    VecString(Vec<String>),
    MemoryInfo(MemoryInfo),
    Duration(Duration),
    Process(Process),
    None,
}
impl Display for ProcCollectorDetails {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            ProcCollectorDetails::Bool(b) => f.write_str(b.to_string().as_str()),
            ProcCollectorDetails::Status(s) => match s {
                Status::Running => f.write_str("Running"),
                Status::Sleeping => f.write_str("Sleeping"),
                Status::DiskSleep => f.write_str("DiskSleep"),
                Status::Stopped => f.write_str("Stopped"),
                Status::TracingStop => f.write_str("TracingStop"),
                Status::Zombie => f.write_str("Zombie"),
                Status::Dead => f.write_str("Dead"),
                Status::WakeKill => f.write_str("WakeKill"),
                Status::Waking => f.write_str("Waking"),
                Status::Parked => f.write_str("Parked"),
                Status::Idle => f.write_str("Idle"),
                Status::Locked => f.write_str("Locked"),
                Status::Waiting => f.write_str("Waiting"),
                Status::Suspended => f.write_str("Suspended"),
            },
            ProcCollectorDetails::U32(u) => f.write_str(u.to_string().as_str()),
            ProcCollectorDetails::U64(u) => f.write_str(u.to_string().as_str()),
            ProcCollectorDetails::I32(i) => f.write_str(i.to_string().as_str()),
            ProcCollectorDetails::F32(fl) => f.write_str(fl.to_string().as_str()),
            ProcCollectorDetails::F64(fl) => f.write_str(fl.to_string().as_str()),
            ProcCollectorDetails::String(s) => f.write_str(s.clone().as_str()),
            ProcCollectorDetails::VecString(v) => f.write_str(v.join(", ").as_str()),
            ProcCollectorDetails::MemoryInfo(m) => f.write_str(m.rss().to_string().as_str()),
            ProcCollectorDetails::Duration(d) => f.write_str(d.as_millis().to_string().as_str()),
            ProcCollectorDetails::Process(p) => {
                f.write_str(p.name().unwrap_or("".to_owned()).as_str())
            }
            ProcCollectorDetails::None => f.write_str(""),
        }
    }
}
impl From<ProcessInfo> for ProcCollectorDetails {
    fn from(info: ProcessInfo) -> Self {
        match info {
            ProcessInfo::Count(u) => ProcCollectorDetails::U64(u),
            ProcessInfo::F32(f) => ProcCollectorDetails::F32(f),
            ProcessInfo::MemoryInfo(m) => ProcCollectorDetails::MemoryInfo(m),
            ProcessInfo::String(s) => ProcCollectorDetails::String(s.clone()),
            ProcessInfo::U64(u) => ProcCollectorDetails::U64(u),
        }
    }
}

#[derive(Clone)]
pub enum ProcessInfo {
    U64(u64),
    String(String),
    Count(Count),
    MemoryInfo(MemoryInfo),
    F32(f32),
}
impl From<ProcCollectorDetails> for ProcessInfo {
    fn from(details: ProcCollectorDetails) -> Self {
        match details {
            ProcCollectorDetails::U64(u) => ProcessInfo::U64(u),
            ProcCollectorDetails::U32(u) => ProcessInfo::U64(u as u64),
            ProcCollectorDetails::String(s) => ProcessInfo::String(s.clone()),
            ProcCollectorDetails::MemoryInfo(m) => ProcessInfo::MemoryInfo(m),
            ProcCollectorDetails::F32(f) => ProcessInfo::F32(f),
            _ => {
                errlog("Attempted to convert some ProcCollectorDetails to ProcessInfo that doesn't exist!!!".to_owned());
                ProcessInfo::String(String::default())
            }
        }
    }
}

#[derive(Clone)]
pub struct ProcCollector<'a> {
    pub parent: Collector<'a>,
    pub buffer: String,
    pub search_filter: String,
    pub processes: HashMap<Pid, HashMap<String, ProcessInfo>>,
    pub num_procs: u32,
    pub det_cpu: f64,
    pub detailed: bool,
    pub detailed_pid: Option<Pid>,
    pub details: HashMap<String, ProcCollectorDetails>,
    pub details_cpu: Vec<u32>,
    pub details_mem: Vec<u32>,
    pub expand: u32,
    pub collapsed: HashMap<Pid, bool>,
    pub tree_counter: usize,
    pub p_values: Vec<String>,
}
impl<'a> ProcCollector<'a> {
    pub fn new(procbox : &mut ProcBox) -> Self {
        let mut proc = ProcCollector {
            parent: Collector::new(),
            buffer: procbox.buffer.clone(),
            search_filter: String::default(),
            processes: HashMap::<Pid, HashMap<String, ProcessInfo>>::new(),
            num_procs: 0,
            det_cpu: 0.0,
            detailed: false,
            detailed_pid: None,
            details: HashMap::<String, ProcCollectorDetails>::new(),
            details_cpu: vec![],
            details_mem: vec![],
            expand: 0,
            collapsed: HashMap::<Pid, bool>::new(),
            tree_counter: 0,
            p_values: [
                "pid",
                "name",
                "cmdline",
                "num_threads",
                "username",
                "memory_percent",
                "cpu_percent",
                "cpu_times",
                "create_time",
            ]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect(),
        };

        proc
    }

    /// List all processess with pid, name, arguments, threads, username, memory percent and cpu percent
    pub fn collect(
        &mut self,
        brshtop_box: &mut BrshtopBox,
        CONFIG: &mut Config,
        procbox: &mut ProcBox,
    ) {
        if brshtop_box.stat_mode {
            return;
        }

        let mut out: HashMap<Pid, HashMap<String, ProcessInfo>> =
            HashMap::<Pid, HashMap<String, ProcessInfo>>::new();
        self.det_cpu = 0.0;
        let sorting: SortingOption = CONFIG.proc_sorting;
        let reverse = !CONFIG.proc_reversed;
        let proc_per_cpu: bool = CONFIG.proc_per_core;
        let search: String = self.search_filter;
        let err: f64 = 0.0;
        let n: usize = 0;

        if CONFIG.proc_tree && sorting == SortingOption::Arguments {
            sorting = SortingOption::Program;
        }

        if CONFIG.proc_tree {
            self.tree(sorting, reverse, proc_per_cpu, search, CONFIG);
        } else {
            let processes = ProcCollector::get_sorted_processes(sorting, reverse);

            for p in processes {
                if self.parent.collect_interrupt || self.parent.proc_interrupt {
                    return;
                }
                let name: String = match p.name() {
                    Ok(s) => {
                        if s == "idle".to_owned() {
                            continue;
                        } else {
                            s
                        }
                    }
                    Err(e) => continue,
                };
                let pid: Pid = p.pid();
                let cmdline = match p.cmdline() {
                    Ok(c) => match c {
                        Some(s) => s.clone(),
                        None => String::default(),
                    },
                    Err(_) => String::default(),
                };
                let username: String = p.username().clone();
                let num_threads: u32 = p.num_threads() as u32;
                if search.len() > 0 {
                    if self.detailed && pid == self.detailed_pid.unwrap_or(0) {
                        self.det_cpu = p.cpu_percent().unwrap_or(0.0) as f64;
                    }

                    let mut search_commandline: String = String::from(" ");
                    for adder in p.cmdline() {
                        match adder {
                            Some(s) => search_commandline.push_str(s.as_str()),
                            None => continue,
                        }
                    }

                    let mut broke = false;
                    for value in vec![name, search_commandline, pid.to_string(), username]
                        .iter()
                        .map(|s| s.to_owned())
                        .collect::<Vec<String>>()
                    {
                        for s in search.split(',') {
                            if value.contains(s.trim()) {
                                break;
                            }
                        }
                        if !broke {
                            continue;
                        }
                        break;
                    }
                    if !broke {
                        continue;
                    }
                }

                let cpu = if proc_per_cpu {
                    match p.cpu_percent() {
                        Ok(c) => c,
                        Err(e) => {
                            errlog(format!(
                                "Unable to get cpu usage of pid {} (error {})",
                                pid, e
                            ));
                            0.0
                        }
                    }
                } else {
                    match p.cpu_percent() {
                        Ok(c) => ceil((c / THREADS.to_owned() as f32) as f64, 2) as f32,
                        Err(e) => {
                            errlog(format!(
                                "Unable to get cpu usage of pid {} (error {})",
                                pid, e
                            ));
                            0.0
                        }
                    }
                };
                let mem = match p.memory_info() {
                    Ok(m) => m,
                    Err(e) => {
                        errlog(format!(
                            "Unable to get memory usage of pid {} (error {})",
                            pid, e
                        ));
                        return;
                    }
                };
                let mem_b: Bytes = mem.rss();

                let cmd: String = match p.cmdline() {
                    Ok(o) => match o {
                        Some(s) => s,
                        None => "[".to_owned() + p.name().unwrap().as_str() + "]",
                    },
                    Err(e) => {
                        errlog(format!(
                            "There was an error getting the process info... (error {:?})",
                            e
                        ));
                        String::default()
                    }
                };

                out[&p.pid()] = vec![
                    (
                        "name",
                        ProcessInfo::String(p.name().unwrap_or(String::default())),
                    ),
                    (
                        "cmd",
                        ProcessInfo::String(
                            cmd.replace("\n", "").replace("\t", "").replace("\\", ""),
                        ),
                    ),
                    ("threads", ProcessInfo::Count(p.num_threads())),
                    ("username", ProcessInfo::String(p.username())),
                    ("mem", ProcessInfo::MemoryInfo(mem)),
                    ("mem_b", ProcessInfo::U64(mem_b)),
                    ("cpu", ProcessInfo::F32(cpu)),
                ]
                .iter()
                .map(|(s, p)| (s.to_owned().to_owned(), p.clone()))
                .collect::<HashMap<String, ProcessInfo>>();

                n += 1;
            }
            self.num_procs = n as u32;
            self.processes = out.clone();
        }

        if self.detailed {
            self.expand = ((procbox.parent.width - 2) - ((procbox.parent.width - 2) / 3) - 40) / 10;
            if self.expand > 5 {
                self.expand = 5;
            }
        }
        if self.detailed
            && !match self
                .details
                .get(&"killed".to_owned())
                .unwrap_or(&ProcCollectorDetails::Bool(false))
            {
                ProcCollectorDetails::Bool(b) => b.to_owned(),
                _ => {
                    errlog("Wrong type in proccollector.details['killed']".to_owned());
                    false
                }
            }
        {
            let c_pid = match self.detailed_pid {
                Some(p) => p,
                None => {
                    self.details[&"killed".to_owned()] = ProcCollectorDetails::Bool(true);
                    self.details[&"status".to_owned()] = ProcCollectorDetails::Status(Status::Dead);
                    procbox.redraw = true;
                    return;
                }
            };
            let det: Process = match psutil::process::Process::new(c_pid) {
                Ok(d) => d,
                Err(e) => {
                    errlog(format!("Unable find process {} (error {:?})", c_pid, e));
                    self.details[&"killed".to_owned()] = ProcCollectorDetails::Bool(true);
                    self.details[&"status".to_owned()] = ProcCollectorDetails::Status(Status::Dead);
                    procbox.redraw = true;
                    return;
                }
            };

            let attrs: Vec<String> = vec!["status", "memory_info", "create_time"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect();
            if SYSTEM.to_owned() != "MacOS".to_owned() {
                attrs.push("cpu_num".to_owned())
            }
            if self.expand > 0 {
                attrs.push("nice".to_owned());
                attrs.push("terminal".to_owned());
                if SYSTEM.to_owned() != "MacOS".to_owned() {
                    attrs.push("io_counters".to_owned());
                }
            }

            if !self.processes.contains_key(&c_pid) {
                for item in vec![
                    "pid",
                    "name",
                    "cmdline",
                    "num_threads",
                    "username",
                    "memory_percent",
                ]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
                {
                    attrs.push(item.clone());
                }
            }

            // cls.details = det.as_dict(attrs=attrs, ad_value="")
            let pre_parsed_keys = [
                (
                    "memory_info",
                    match det.memory_info() {
                        Ok(m) => ProcCollectorDetails::MemoryInfo(m),
                        Err(e) => {
                            errlog(format!(
                                "Error in getting memory information (error {:?})",
                                e
                            ));
                            ProcCollectorDetails::None
                        }
                    },
                ),
                (
                    "status",
                    match det.status() {
                        Ok(s) => ProcCollectorDetails::Status(s),
                        Err(e) => {
                            errlog(format!(
                                "Error in receiving status of process (error {:?})",
                                e
                            ));
                            ProcCollectorDetails::None
                        }
                    },
                ),
                (
                    "create_time",
                    ProcCollectorDetails::Duration(det.create_time()),
                ),
                ("cpu_num", ProcCollectorDetails::I32(0)), // TODO : Once implemented
                ("nice", ProcCollectorDetails::I32(det.get_nice())),
                (
                    "terminal",
                    ProcCollectorDetails::String(match det.terminal() {
                        Some(s) => s.clone(),
                        None => {
                            errlog("Unable to get process' terminal...".to_owned());
                            String::default()
                        }
                    }),
                ),
                ("io_counters", ProcCollectorDetails::None), // TODO : Once implemented in psutil
                (
                    "name",
                    ProcCollectorDetails::String(det.name().unwrap_or("".to_owned())),
                ),
                ("pid", ProcCollectorDetails::U32(det.pid())),
                (
                    "cmdline",
                    match det.cmdline_vec() {
                        Ok(o) => match o {
                            Some(v) => ProcCollectorDetails::VecString(v.clone()),
                            None => ProcCollectorDetails::None,
                        },
                        Err(e) => {
                            errlog(format!(
                                "Error getting cmdline information on process (error {:?})",
                                e
                            ));
                            ProcCollectorDetails::None
                        }
                    },
                ),
                ("num_threads", ProcCollectorDetails::U64(det.num_threads())),
                (
                    "memory_percent",
                    ProcCollectorDetails::F32(det.memory_percent().unwrap_or(0.0)),
                ),
                ("username", ProcCollectorDetails::String(det.username())),
            ]
            .iter()
            .map(|(s, p)| (s.to_owned().to_owned(), p.clone()))
            .collect::<HashMap<String, ProcCollectorDetails>>();

            self.details = pre_parsed_keys.clone();

            self.details.insert(
                "parent_name".to_owned(),
                match det.parent() {
                    Ok(o) => match o {
                        Some(p) => ProcCollectorDetails::String(p.name().unwrap_or("".to_owned())),
                        None => ProcCollectorDetails::String(String::default()),
                    },
                    Err(e) => {
                        errlog(format!("Unable to get process' parent (error {:?})", e));
                        ProcCollectorDetails::String(String::default())
                    }
                },
            );

            self.details
                .insert("pid".to_owned(), ProcCollectorDetails::U32(c_pid));
            if self.processes.contains_key(&c_pid) {
                let current_process: HashMap<String, ProcessInfo> = self.processes[&c_pid].clone();
                self.details[&"name".to_owned()] =
                    ProcCollectorDetails::from(current_process[&"name".to_owned()]);
                self.details[&"cmdline".to_owned()] =
                    ProcCollectorDetails::from(current_process[&"cmd".to_owned()]);
                self.details[&"threads".to_owned()] =
                    ProcCollectorDetails::from(current_process[&"threads".to_owned()]);
                self.details[&"username".to_owned()] =
                    ProcCollectorDetails::from(current_process[&"username".to_owned()]);
                self.details[&"memory_percent".to_owned()] =
                    ProcCollectorDetails::from(current_process[&"mem".to_owned()]);
                let cpu_percent_push = match current_process[&"cpu".to_owned()] {
                    ProcessInfo::F32(f) => f,
                    _ => {
                        errlog(format!(
                            "Malformed cpu percentage from proccollector.processes[{}]",
                            c_pid
                        ));
                        0.0
                    }
                };
                self.details.insert(
                    "cpu_percent".to_owned(),
                    ProcCollectorDetails::F32(
                        cpu_percent_push
                            * if CONFIG.proc_per_core {
                                1.0
                            } else {
                                THREADS.to_owned() as f32
                            },
                    ),
                );
            } else {
                let cmdline_pusher: String = " ".to_owned()
                    + match self.details[&"cmdline".to_owned()] {
                        ProcCollectorDetails::String(s) => s.as_str(),
                        _ => "",
                    };

                self.details[&"cmdline".to_owned()] =
                    ProcCollectorDetails::String(if cmdline_pusher.len() > 0 {
                        cmdline_pusher
                    } else {
                        "[".to_owned() + self.details[&"name".to_owned()].to_string().as_str() + "]"
                    });

                self.details.insert(
                    "threads".to_owned(),
                    self.details[&"num_threads".to_owned()],
                );
                self.details.insert(
                    "cpu_percent".to_owned(),
                    ProcCollectorDetails::F64(self.det_cpu),
                );
            }

            self.details
                .insert("killed".to_owned(), ProcCollectorDetails::Bool(false));
            if SYSTEM.to_owned() == "MacOS".to_owned() {
                self.details["cpu_num"] = ProcCollectorDetails::None;
                self.details["io_counters"] = ProcCollectorDetails::None;
            }

            self.details.insert(
                "memory_info".to_owned(),
                match self.details[&"memory_info".to_owned()] {
                    ProcCollectorDetails::MemoryInfo(m) => ProcCollectorDetails::String(
                        floating_humanizer(m.rss() as f64, false, false, 0, false),
                    ),
                    _ => ProcCollectorDetails::String("? Bytes".to_owned()),
                },
            );

            self.details.insert(
                "uptime".to_owned(),
                match self.details[&"create_time".to_owned()] {
                    ProcCollectorDetails::Duration(d) => {
                        let total_seconds = d.as_secs();
                        let days = total_seconds / (24 * 60 * 60);
                        total_seconds -= total_seconds / (24 * 60 * 60);
                        let hours = total_seconds / (60 * 60);
                        total_seconds -= total_seconds / (60 * 60);
                        let minutes = total_seconds / 60;
                        total_seconds -= total_seconds / 60;
                        let seconds = total_seconds;

                        if days > 0 {
                            ProcCollectorDetails::String(format!(
                                "{}d {}:{}:{}",
                                days, hours, minutes, seconds
                            ))
                        } else {
                            ProcCollectorDetails::String(format!(
                                "{}:{}:{}",
                                hours, minutes, seconds
                            ))
                        }
                    }
                    _ => ProcCollectorDetails::String("??:??:??".to_owned()),
                },
            );

            if self.expand > 0 {
                if self.expand > 1 {
                    self.details["nice"] =
                        ProcCollectorDetails::String(self.details["nice"].to_string());
                }
                if SYSTEM.to_owned() == "BSD".to_owned() {
                    if self.expand > 2 {
                        // TODO : Once implemented by psutil
                        self.details.insert(
                            "io_read".to_owned(),
                            ProcCollectorDetails::String("?".to_owned()),
                        );
                        if self.expand > 3 {
                            self.details.insert(
                                "io_write".to_owned(),
                                ProcCollectorDetails::String("?".to_owned()),
                            );
                        }
                    }
                } else {
                    if self.expand > 2 {
                        // TODO : Once implemented by psutil
                        self.details.insert(
                            "io_read".to_owned(),
                            ProcCollectorDetails::String("?".to_owned()),
                        );
                        if self.expand > 3 {
                            self.details.insert(
                                "io_write".to_owned(),
                                ProcCollectorDetails::String("?".to_owned()),
                            );
                        }
                    }
                }
                if self.expand > 4 {
                    self.details.insert(
                        "terminal".to_owned(),
                        ProcCollectorDetails::String(
                            self.details["terminal"].to_string().replace("/dev/", ""),
                        ),
                    );
                }
            }

            self.details_cpu
                .push(match self.details[&"cpu_percent".to_owned()] {
                    ProcCollectorDetails::String(s) => s.parse::<u32>().unwrap_or(0),
                    _ => {
                        errlog("Malformed cpu_percentage".to_owned());
                        0
                    }
                });

            let mut mem: f32 = match self.details[&"memory_percent".to_owned()] {
                ProcCollectorDetails::F32(f) => f,
                _ => {
                    errlog("Malformed memory_percent".to_owned());
                    0.0
                }
            };

            self.details_mem.push(f32::round(
                mem * if mem > 80.0 {
                    1.0
                } else if mem > 60.0 {
                    1.2
                } else if mem > 30.0 {
                    1.5
                } else if mem > 10.0 {
                    2.0
                } else if mem > 5.0 {
                    10.0
                } else {
                    20.0
                },
            ) as u32);
            if self.details_cpu.len() as u32 > procbox.parent.width {
                self.details_cpu.remove(0);
            }
            if self.details_mem.len() as u32 > procbox.parent.width {
                self.details_mem.remove(0);
            }
        }
    }

    pub fn get_sorted_processes(sort_type: SortingOption, reverse: bool) -> Vec<Process> {
        let processes: Vec<ProcessResult<Process>> = match processes() {
            Ok(p) => p,
            Err(e) => {
                throw_error(format!("UNABLE TO GET PROCESS INFORMATION (ERROR {})", e).as_str());
                return Vec::<Process>::new();
            }
        };

        let mut sorting: Vec<Process> = Vec::<Process>::new();
        for p_result in processes {
            match p_result {
                Ok(p) => sorting.push(p),
                Err(e) => errlog(format!(
                    "Unable to get current process information (error {})",
                    e
                )),
            }
        }

        match sort_type {
            SortingOption::Pid => {
                sorting.sort_by(|p1, p2| p1.pid().partial_cmp(&p2.pid()).unwrap())
            }
            SortingOption::Program => {
                sorting.sort_by(|p1, p2| p1.name().unwrap().cmp(&p2.name().unwrap()))
            }
            SortingOption::Arguments => sorting.sort_by(|p1, p2| {
                p1.cmdline()
                    .unwrap()
                    .unwrap_or(String::default())
                    .cmp(&p2.cmdline().unwrap().unwrap_or(String::default()))
            }),
            SortingOption::Threads => sorting.sort_by(|p1, p2| p1.threads().cmp(&p2.threads())),
            SortingOption::User => sorting.sort_by(|p1, p2| p1.username().cmp(&p2.username())),
            SortingOption::Memory => sorting.sort_by(|p1, p2| {
                p1.memory_info()
                    .unwrap()
                    .rss()
                    .cmp(&p2.memory_info().unwrap().rss())
            }),
            SortingOption::Cpu { lazy: b } => {
                if b {
                    sorting.sort_by(|p1, p2| {
                        let times1 = p1.cpu_times().unwrap();
                        let times2 = p2.cpu_times().unwrap();

                        let sum1 = times1.user() + times1.system();
                        let sum2 = times2.user() + times2.system();

                        if sum1 > sum2 {
                            Ordering::Greater
                        } else if sum1 == sum2 {
                            Ordering::Equal
                        } else {
                            Ordering::Less
                        }
                    });
                } else {
                    sorting.sort_by(|p1, p2| {
                        p1.cpu_percent()
                            .unwrap()
                            .partial_cmp(&p2.cpu_percent().unwrap())
                            .unwrap()
                    });
                }
            }
        }
        if reverse {
            sorting.iter().rev().map(|p| p.to_owned()).collect()
        } else {
            sorting
        }
    }

    pub fn tree(
        &mut self,
        sort_type: SortingOption,
        reverse: bool,
        proc_per_cpu: bool,
        search: String,
        CONFIG: &mut Config,
    ) {
        let mut out: HashMap<Pid, HashMap<String, ProcCollectorDetails>> =
            HashMap::<Pid, HashMap<String, ProcCollectorDetails>>::new();
        let mut err: f32 = 0.0;
        let mut det_cpu: f64 = 0.0;
        let mut infolist: HashMap<Pid, HashMap<String, ProcCollectorDetails>> =
            HashMap::<Pid, HashMap<String, ProcCollectorDetails>>::new();
        self.tree_counter += 1;
        let mut tree: HashMap<Pid, Vec<Pid>> = HashMap::<Pid, Vec<Pid>>::new(); // Default to an empty Vec!!!
        let mut n: usize = 0;

        for p in ProcCollector::get_sorted_processes(sort_type, reverse) {
            if self.parent.collect_interrupt {
                return;
            }
            match p.ppid() {
                Ok(o) => {
                    match o {
                        Some(pid) => {
                            match tree.get(&pid) {
                                Some(v) => v.push(p.pid()),
                                None => {
                                    tree.insert(pid, vec![p.pid()]);
                                    ()
                                }
                            }
                            let info: HashMap<String, ProcCollectorDetails> =
                                HashMap::<String, ProcCollectorDetails>::new();

                            if self.p_values.contains(&"cpu_percent".to_owned()) {
                                info.insert("cpu_percent".to_owned(),
                                match p.cpu_percent() {
                                    Ok(f) =>  ProcCollectorDetails::F32(f),
                                    Err(e) => {
                                        errlog(format!("Error getting cpu_percent from process (error {:?})", e));
                                        ProcCollectorDetails::F32(0.0)
                                    },
                                }
                            );
                            }
                            if self.p_values.contains(&"username".to_owned()) {
                                info.insert(
                                    "username".to_owned(),
                                    ProcCollectorDetails::String(p.username()),
                                );
                            }
                            if self.p_values.contains(&"cmdline".to_owned()) {
                                info.insert(
                                    "cmdline".to_owned(),
                                    match p.cmdline() {
                                        Ok(o) => match o {
                                            Some(s) => ProcCollectorDetails::String(s),
                                            None => {
                                                errlog(
                                                    "Error getting cmdline from process".to_owned(),
                                                );
                                                ProcCollectorDetails::F32(0.0)
                                            }
                                        },
                                        Err(e) => {
                                            errlog(format!(
                                                "Error getting cmdline from process (error {:?})",
                                                e
                                            ));
                                            ProcCollectorDetails::F32(0.0)
                                        }
                                    },
                                );
                            }
                            if self.p_values.contains(&"num_threads".to_owned()) {
                                info.insert(
                                    "num_threads".to_owned(),
                                    ProcCollectorDetails::U64(p.num_threads()),
                                );
                            }
                            if self.p_values.contains(&"name".to_owned()) {
                                info.insert(
                                    "name".to_owned(),
                                    ProcCollectorDetails::String(match p.name() {
                                        Ok(s) => s,
                                        Err(e) => {
                                            errlog(format!(
                                                "Error getting process name (error {:?})",
                                                e
                                            ));
                                            String::default()
                                        }
                                    }),
                                );
                            }
                            if self.p_values.contains(&"memory_info".to_owned()) {
                                info.insert("memory_info".to_owned(),
                                match p.memory_info() {
                                    Ok(m) =>  ProcCollectorDetails::MemoryInfo(m),
                                    Err(e) => {
                                        errlog(format!("Error getting memory_info from process (error {:?})", e));
                                        ProcCollectorDetails::F32(0.0)
                                    },
                                }
                            );
                            }

                            infolist.insert(p.pid(), info.clone());
                            n += 1;
                        }
                        None => (),
                    }
                }
                Err(e) => (),
            }
        }
        if tree.contains_key(&0) && tree.get(&0).unwrap().contains(&0) {
            let mut xs = tree.get_mut(&0).unwrap();
            let index = xs.iter().position(|x| *x == 0).unwrap();
            xs.remove(index);
        }

        if tree.len() > 0 {
            ProcCollector::create_tree(
                self,
                tree.iter().map(|(s, _)| s.to_owned()).min().unwrap(),
                tree,
                String::default(),
                String::default(),
                false,
                0,
                None,
                &mut infolist,
                &mut proc_per_cpu,
                &mut search,
                &mut out,
                &mut det_cpu,
                CONFIG,
            )
        }
        self.det_cpu = det_cpu;

        if self.parent.collect_interrupt {
            return;
        }
        if self.tree_counter >= 100 {
            self.tree_counter = 0;
            for (pid, _) in self.collapsed {
                if !psutil::process::pid_exists(pid) {
                    self.collapsed.remove_entry(&pid);
                }
            }
        }
        self.num_procs = out.len() as u32;
        self.processes = out
            .clone()
            .iter()
            .map(|(s1, h)| {
                (
                    s1.to_owned(),
                    h.iter()
                        .map(|(s, p)| (s.clone(), ProcessInfo::from(p.to_owned())))
                        .collect::<HashMap<String, ProcessInfo>>()
                )
            })
            .collect::<HashMap<u32, HashMap<String, ProcessInfo>>>();
    }

    /// Defaults indent: str = "", inindent: str = " ", found: bool = False, depth: int = 0, collapse_to: Union[None, int] = None
    pub fn create_tree(
        &mut self,
        pid: Pid,
        tree: HashMap<Pid, Vec<Pid>>,
        indent: String,
        inindent: String,
        found: bool,
        depth: u32,
        collapse_to: Option<Pid>,
        infolist: &mut HashMap<Pid, HashMap<String, ProcCollectorDetails>>,
        proc_per_cpu: &mut bool,
        search: &mut String,
        out: &mut HashMap<Pid, HashMap<String, ProcCollectorDetails>>,
        det_cpu: &mut f64,
        CONFIG: &mut Config,
    ) {
        let mut name: String = String::default();
        let mut threads: u64 = 0;
        let mut username: String = String::default();
        let mut mem: f32 = 0.0;
        let mut cpu: f32 = 0.0;
        let mut collapse: bool = false;
        let mut cont: bool = true;
        let mut getinfo: HashMap<String, ProcCollectorDetails> =
            HashMap::<String, ProcCollectorDetails>::new();
        let mut mem_b: Bytes = 0;
        let mut cmd: String = String::default();
        if self.parent.collect_interrupt {
            return;
        }
        let name: String = match psutil::process::Process::new(pid) {
            Ok(p) => match p.name() {
                Ok(s) => s,
                Err(_) => {
                    cont = false;
                    String::default()
                }
            },
            Err(e) => {
                errlog(format!(
                    "Unable to get process from PID : {} (error {})",
                    pid, e
                ));
                return;
            }
        };
        if name == "idle".to_owned() {
            return;
        }
        if infolist.contains_key(&pid) {
            getinfo = infolist.get(&pid).unwrap().clone();
        }

        if search.len() > 0 && !found {
            if self.detailed && pid == self.detailed_pid.unwrap_or(0) {
                det_cpu = match getinfo.get(&"cpu_percent".to_owned()) {
                    Some(p) => match p {
                        ProcCollectorDetails::F64(f) => &mut f,
                        _ => {
                            errlog("Malformed information found in cpu_percent".to_owned());
                            &mut 0.0
                        }
                    },
                    None => {
                        errlog("getinfo doesn't have cpu_percent!!!!".to_owned());
                        &mut 0.0
                    }
                };
            }
            let mut adder: String = String::default();
            for key in vec!["username", "cmdline"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
            {
                if !getinfo.contains_key(&key) {
                    getinfo.insert(key, ProcCollectorDetails::String(String::default()));
                    adder.push_str("");
                } else {
                    adder.push_str((adder + " ").as_str());
                }
            }
            adder = adder.trim().to_owned();
            let mut broke1: bool = false;
            for value in vec![name, pid.to_string(), adder] {
                let mut broke2: bool = false;
                for s in search.split(',') {
                    if value.contains(s.trim()) {
                        found = true;
                        broke2 = true;
                        break;
                    }
                }
                if !broke2 {
                    continue;
                }
                broke1 = true;
                break;
            }
            if !broke1 {
                cont = false;
            }
        }
        if cont {
            if getinfo.len() > 0 {
                threads = match getinfo.get(&"num_threads".to_owned()) {
                    Some(t) => match t {
                        ProcCollectorDetails::U64(u) => u.to_owned(),
                        _ => {
                            errlog("Malformed type in getinfo['num_threads']".to_owned());
                            0
                        }
                    },
                    None => 0,
                };
                username = match getinfo.get(&"username".to_owned()) {
                    Some(t) => match t {
                        ProcCollectorDetails::String(s) => s.clone(),
                        _ => {
                            errlog("Malformed type in getinfo['username']".to_owned());
                            String::default()
                        }
                    },
                    None => String::default(),
                };
                cpu = match getinfo.get(&"cpu_percent".to_owned()) {
                    Some(o) => match o {
                        ProcCollectorDetails::F32(f) => {
                            if proc_per_cpu.to_owned() {
                                f.to_owned()
                            } else {
                                format!("{:.2}", f / THREADS.to_owned() as f32).parse::<f32>().unwrap_or(0.0)
                            }
                        }
                        _ => {
                            errlog("Malformed type in getinfo['cpu_percent']".to_owned());
                            0.0
                        }
                    },
                    None => 0.0,
                };
                mem = match getinfo.get(&"memory_percent".to_owned()) {
                    Some(o) => match o {
                        ProcCollectorDetails::F32(f) => f.to_owned(),
                        _ => {
                            errlog("Malformed type in getinfo['memory_percent']".to_owned());
                            0.0
                        }
                    },
                    None => 0.0,
                };
                cmd = match getinfo.get(&"cmdline".to_owned()) {
                    Some(o) => match o {
                        ProcCollectorDetails::String(s) => {
                            let ws: String = s.clone();
                            if ws.len() > 0 {
                                (" ".to_owned() + ws.as_str()).to_owned()
                            } else {
                                ("[".to_owned()
                                    + match getinfo.get(&"name".to_owned()) {
                                        Some(p) => match p {
                                            ProcCollectorDetails::String(s) => s.clone().as_str(),
                                            _ => {
                                                errlog("Malformed type in getinfo['name']".to_owned());
                                                ""
                                            }
                                        },
                                        None => "",
                                    }
                                    + "]")
                                    .to_owned()
                            }
                        }
                        ProcCollectorDetails::VecString(v) => v.join(", ").trim().to_owned(),
                        _ => {
                            errlog("Malformed type in getinfo['cmdline']".to_owned());
                            String::default()
                        }
                    },
                    None => String::default(),
                };
                if CONFIG.proc_mem_bytes {
                    mem_b = match getinfo.get(&"memory_info".to_owned()) {
                        Some(p) => match p {
                            ProcCollectorDetails::MemoryInfo(m) => m.rss(),
                            _ => {
                                errlog("Malformed type in getinfo['memory_info']".to_owned());
                                0
                            }
                        },
                        None => 0,
                    }
                }
            } else {
                threads = 0;
                mem_b = 0;
                username = String::default();
                mem = 0.0;
                cpu = 0.0;
            }

            collapse = match self.collapsed.get(&pid) {
                Some(b) => b.clone(),
                None => {
                    let b = depth as i32 > CONFIG.tree_depth;
                    self.collapsed.insert(pid, b);
                    b
                }
            };

            let mut elser: bool = false;
            match collapse_to {
                Some(u) => {
                    if search.len() == 0 {
                        out[&u][&"threads".to_owned()] = match out[&u]
                            [&"threads".to_owned()]
                        {
                            ProcCollectorDetails::U64(n) => ProcCollectorDetails::U64(n + threads),
                            _ => {
                                errlog(format!("Malformed type in out[{}]['threads']", u));
                                ProcCollectorDetails::U64(0)
                            }
                        };
                        out[&u][&"mem".to_owned()] = match out[&u][&"mem".to_owned()]
                        {
                            ProcCollectorDetails::F32(f) => ProcCollectorDetails::F32(f + mem),
                            _ => {
                                errlog(format!("Malformed type in out[{}]['mem']", u));
                                ProcCollectorDetails::F32(0.0)
                            }
                        };
                        out[&u][&"mem_b".to_owned()] = match out[&u]
                            [&"mem_b".to_owned()]
                        {
                            ProcCollectorDetails::U64(n) => ProcCollectorDetails::U64(n + mem_b),
                            _ => {
                                errlog(format!("Malformed type in out[{}]['mem_b']", u));
                                ProcCollectorDetails::U64(0)
                            }
                        };
                        out[&u][&"cpu".to_owned()] = match out[&u][&"cpu".to_owned()]
                        {
                            ProcCollectorDetails::F32(f) => ProcCollectorDetails::F32(f + cpu),
                            _ => {
                                errlog(format!("Malformed type in out[{}]['cpu']", u));
                                ProcCollectorDetails::F32(0.0)
                            }
                        };
                    } else {
                        elser = true;
                    }
                }
                None => elser = true,
            }
            if elser {
                if tree.contains_key(&pid) && tree.get(&pid).unwrap().len() > 0 {
                    let sign: &str = if collapse { "+" } else { "-" };
                    inindent = inindent.replace(" ├─ ", ("[".to_owned() + sign + "]─").as_str());
                }
                out.insert(
                    pid,
                    vec![
                        ("indent", ProcCollectorDetails::String(inindent)),
                        ("name", ProcCollectorDetails::String(name)),
                        (
                            "cmd",
                            ProcCollectorDetails::String(
                                cmd.replace("\n", "").replace("\t", "").replace("\\", ""),
                            ),
                        ),
                        ("threads", ProcCollectorDetails::U64(threads)),
                        ("username", ProcCollectorDetails::String(username)),
                        ("mem", ProcCollectorDetails::F32(mem)),
                        ("mem_b", ProcCollectorDetails::U64(mem_b)),
                        ("cpu", ProcCollectorDetails::F32(cpu)),
                        ("depth", ProcCollectorDetails::U32(depth)),
                    ]
                    .iter()
                    .map(|(s, p)| (s.to_owned().to_owned(), p.clone()))
                    .collect::<HashMap<String, ProcCollectorDetails>>(),
                );
            }
        }

        if search.len() > 0 {
            collapse = false;
        } else {
            match collapse_to {
                Some(u) => {
                    if collapse && u == 0 {
                        collapse_to = Some(pid);
                    }
                }
                None => (),
            }
        }

        if !tree.contains_key(&pid) {
            return;
        }
        let children: Vec<u32> = tree[&pid][..tree[&pid].len() - 2].iter().map(|u| u.to_owned()).collect::<Vec<u32>>();

        for child in children {
            ProcCollector::create_tree(
                self,
                child,
                tree,
                indent + " | ",
                indent + " ├─ ",
                found,
                depth + 1,
                collapse_to,
                infolist,
                proc_per_cpu,
                search,
                out,
                det_cpu,
                CONFIG,
            )
        }
        ProcCollector::create_tree(
            self,
            tree[&pid][tree[&pid].len() - 2].to_owned(),
            tree,
            indent + " ",
            indent + " └─ ",
            false,
            depth + 1,
            collapse_to,
            infolist,
            proc_per_cpu,
            search,
            out,
            det_cpu,
            CONFIG,
        );
    }

    /// JUST CALL ProcBox.draw_fg()
    pub fn draw(
        &mut self,
        procbox: &mut ProcBox,
        CONFIG: &mut Config,
        key: &mut Key,
        THEME: &mut Theme,
        graphs: &mut Graphs,
        term: &mut Term,
        draw: &mut Draw,
        menu: &mut Menu,
    ) {
        procbox.draw_fg(CONFIG, key, THEME, graphs, term, draw, self, menu)
    }
}

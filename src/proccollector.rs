use crate::{brshtop_box, floating_humanizer};

use {
    core::time::Duration,
    crate::{
        brshtop_box::{
            BrshtopBox,
            Boxes
        },
        collector::Collector,
        config::{Config, SortingOption},
        error::{errlog, throw_error},
        procbox::ProcBox,
        SYSTEM,
    },
    math::round::ceil,
    psutil::{
        Count,
        disk::DiskIoCounters,
        network::NetIoCounters,
        process::{
            *, 
            os::{
                linux::IoCounters,
                unix::ProcessExt,
            },
        }, 
        Pid
    },
    std::{
        cmp::Ordering, 
        collections::HashMap,
        convert::TryInto, 
        path::Path},
    users::get_user_by_uid,
};

#[derive(Clone, Display)]
// TODO : Fix rest of detail types
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
    IoCounters(IoCounters),
    Process(Process),
    None
} impl Display for ProcCollectorDetails {
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
            ProcCollectorDetails::F32(f) => f.write_str(f.to_string().as_str()),
            ProcCollectorDetails::F64(f) => f.write_str(f.to_string().as_str()),
            ProcCollectorDetails::String(s) => s.clone(),
            ProcCollectorDetails::VecString(v) => v.join(", "),
            ProcCollectorDetails::MemoryInfo(m) => f.write_fmt(m.rss()),
            ProcCollectorDetails::Duration(d) => f.write_fmt(d.as_millis()),
            ProcCollectorDetails::IoCounters(_) => f.write_fmt(String::default()),
            ProcCollectorDetails::Process(p) => f.write_fmt(p.name().unwrap_or("".to_owned())),
            ProcCollectorDetails::None => f.write_str(""),
        }
    }
} impl From<ProcessInfo> for ProcCollectorDetails {
    fn from(info : ProcessInfo) -> Self {
        match info {
            ProcessInfo::Count(u) => ProcCollectorDetails::U64(u),
            ProcessInfo::F32(f) => ProcCollectorDetails::F32(f),
            ProcessInfo::MemoryInfo(_) => {
                errlog("Attempted to convert ProcessInfo::MemoryInfo to ProcCollectorDetails! This is impossible and needs to be patched!");
                ProcCollectorDetails::None
            },
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

#[derive(Clone)]
pub struct ProcCollector {
    pub parent: Collector,
    pub buffer: String,
    pub search_filter: String,
    pub processes: HashMap<Pid, HashMap<String, ProcessInfo>>,
    pub num_procs: u32,
    pub det_cpu: f64,
    pub detailed: bool,
    pub detailed_pid: Option<Pid>,
    pub details: HashMap<String, ProcCollectorDetails>, // TODO : Fix types
    pub details_cpu: Vec<u32>,
    pub details_mem: Vec<u32>,
    pub expand: u32,
    pub collapsed: HashMap<String, String>, // TODO : Fix types
    pub tree_counter: usize,
    pub p_values: Vec<String>,
}
impl ProcCollector {

    pub fn new(buffer: String) -> Self {
        let mut proc = ProcCollector {
            parent: Collector::new(),
            buffer: buffer.clone(),
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
            collapsed: HashMap::<String, String>::new(),
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
    pub fn collect(&mut self, brshtop_box : &mut BrshtopBox, CONFIG : &mut Config, THREADS : u64, procbox : &mut ProcBox) {
        if brshtop_box.stat_mode {
            return
        }

        let mut out : HashMap<Pid, HashMap<String, ProcessInfo>> = HashMap::<Pid, HashMap<String, ProcessInfo>>::new();
        self.det_cpu = 0.0;
        let sorting : SortingOption = CONFIG.proc_sorting;
        let reverse = !CONFIG.proc_reversed;
        let proc_per_cpu : bool = CONFIG.proc_per_core;
        let search : String = self.search_filter;
        let err : f64 = 0.0;
        let n : usize = 0;

        if CONFIG.proc_tree && sorting == SortingOption::Arguments {
            sorting = SortingOption::Program;
        }

        if CONFIG.proc_tree {
            self.tree();
        } else {
            let processes = ProcCollector::get_sorted_processes(sorting);
            if reverse {
                processes = processes.iter().rev().map(|p| p.to_owned()).collect();
            }

            for p in processes {
                if self.parent.collect_interrupt || self.parent.proc_interrupt {
                    return;
                }
                let name : String = match p.name() {
                    Ok(s) => if s == "idle".to_owned() {continue} else {s},
                    Err(e) => continue,
                };
                let pid : Pid = p.pid();
                let cmdline = match p.cmdline() {
                    Ok(c) => match c {
                        Some(s) => s.clone(),
                        None => String::default(),
                    },
                    Err(_) => String::default(),
                };
                let username : String = p.username().clone();
                let num_threads : u32 = p.num_threads() as u32;
                if search.len() > 0 {
                    if self.detailed && pid == self.detailed_pid.unwrap_or(0) {
                        self.det_cpu = p.cpu_percent().unwrap_or(0.0) as f64;
                    }

                    let mut search_commandline : String = String::from(" ");
                    for adder in p.cmdline() {
                        match adder {
                            Some(s) => search_commandline.push_str(s.as_str()),
                            None => continue,
                        }
                    }

                    let mut broke = false;
                    for value in vec![name, search_commandline, pid.to_string(), username].iter().map(|s| s.to_owned()).collect::<Vec<String>>() {
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
                            errlog(format!("Unable to get cpu usage of pid {} (error {})", pid, e));
                            0.0
                        }
                    }
                } else {
                    match p.cpu_percent() {
                        Ok(c) => ceil((c / THREADS as f32) as f64, 2) as f32,
                        Err(e) => {
                            errlog(format!("Unable to get cpu usage of pid {} (error {})", pid, e));
                            0.0
                        }
                    }
                };
                let mem =  match p.memory_info() {
                    Ok(m) => m,
                    Err(e) => {
                        errlog(format!("Unable to get memory usage of pid {} (error {})", pid, e));
                        return
                    }
                };
                let mem_b : Bytes = mem.rss();

                let cmd : String = match p.cmdline() {
                    Ok(o) => match o {
                        Some(s) => s,
                        None => "[".to_owned() + p.name().unwrap() + "]",
                    },
                    Err(e) => {
                        errlog(format!("There was an error getting the process info... (error {:?})", e));
                        String::default()
                    }
                };

                out[p.pid()] = vec![
                    ("name", ProcessInfo::String(p.name().unwrap_or(String::default()))),
                    ("cmd", ProcessInfo::String(cmd.replace("\n", "").replace("\t", "").replace("\\", ""))),
                    ("threads", ProcessInfo::Count(p.num_threads())),
                    ("username", ProcessInfo::String(p.username())),
                    ("mem", ProcessInfo::MemoryInfo(mem)),
                    ("mem_b", ProcessInfo::U64(mem_b)),
                    ("cpu", ProcessInfo::F32(cpu)),
                ].iter().map(|(s,p)| (s.to_owned().to_owned(), p.clone())).collect::<HashMap<String, ProcessInfo>>();

                n += 1;
            }
            self.num_procs = n;
            self.processes = out.copy();
        }

        if self.detailed {
            self.expand = ((procbox.parent.width - 2) - ((procbox.parent.width - 2) / 3) - 40) / 10;
            if self.expand > 5 {
                self.expand = 5;
            }
        }
        if self.detailed && ! match self.details.get("killed".to_owned()).unwrap_or(ProcCollectorDetails::Bool(false)) {
            ProcCollectorDetails::Bool(b) => b,
            _ => {
                errlog("Wrong type in proccollector.details['killed']");
                false
            },
        } {
            let c_pid = match self.detailed_pid {
                Some(p) => p,
                None => {
                    self.details["killed".to_owned()] = ProcCollectorDetails::Bool(true);
                    self.details["status".to_owned()] = ProcCollectorDetails::Status(Status::Dead);
                    procbox.redraw = true;
                    return;
                }
            };
            let det : Process = match psutil::process::Process::new(c_pid) {
                Some(d) => d,
                None => {
                    self.details["killed".to_owned()] = ProcCollectorDetails::Bool(true);
                    self.details["status".to_owned()] = ProcCollectorDetails::Status(Status::Dead);
                    procbox.redraw = true;
                    return;
                }
            };

            let attrs : Vec<String> = vec!["status", "memory_info", "create_time"].iter().map(|s| s.to_owned().to_owned()).collect();
            if SYSTEM != "MacOS".to_owned() {
                attrs.push("cpu_num".to_owned())
            }
            if self.expand > 0 {
                attrs.push("nice".to_owned());
                attrs.push("terminal".to_owned());
                if SYSTEM != "MacOS".to_owned() {
                    attrs.push("io_counters".to_owned());
                }
            }

            if !self.processes.contains_key(c_pid) {
                for item in vec!["pid", "name", "cmdline",
                "num_threads", "username", "memory_percent"].iter().map(|s| s.to_owned().to_owned()).collect() {
                    attrs.push(item.clone());
                }
            }

            // cls.details = det.as_dict(attrs=attrs, ad_value="")
            let pre_parsed_keys = [
                ("memory_info", match det.memory_info() {
                    Ok(m) => ProcCollectorDetails::MemoryInfo(m),
                    Err(e) => {
                        errlog(format!("Error in getting memory information (error {:?})", e));
                        ProcCollectorDetails::None
                    },
                }),
                ("status", match det.status(){
                    Ok(s) => ProcCollectorDetails::Status(s),
                    Err(e) => {
                        errlog(format!("Error in receiving status of process (error {:?})", e));
                        ProcCollectorDetails::None
                    },
                }),
                ("create_time", ProcCollectorDetails::Duration(det.create_time())),
                ("cpu_num", ProcCollectorDetails::I32(det.cpu_num())),
                ("nice", ProcCollectorDetails::I32(det.get_nice())),
                ("terminal", ProcCollectorDetails::String(match det.terminal() {
                    Some(s) => s,
                    None => {
                        errlog("Unable to get process' terminal...");
                        String::default()
                    }
                })),
                ("io_counters", ProcCollectorDetails::None), // TODO : Once implemented in psutil
                ("name", ProcCollectorDetails::String(det.name().unwrap_or("".to_owned()))),
                ("pid", ProcCollectorDetails::U32(det.pid())),
                ("cmdline", match det.cmdline_vec() {
                    Ok(o) => match o {
                        Some(v) => ProcCollectorDetails::VecString(v.clone()),
                        None => ProcCollectorDetails::None,
                    },
                    Err(e) => {
                        errlog(format!("Error getting cmdline information on process (error {:?})", e));
                        ProcCollectorDetails::None
                    }
                }),
                ("num_threads", ProcCollectorDetails::U64(det.num_threads())),
                ("memory_percent", ProcCollectorDetails::F32(det.memory_percent().unwrap_or(0.0))),
                ("username", ProcCollectorDetails::String(det.username())),
            ].iter().map(|s,p| (s.to_owned(),p.clone())).collect::<HashMap<String, ProcCollectorDetails>>();

            self.details = pre_parsed_keys.clone();

            self.details.insert("parent_name".to_owned(), match det.parent() {
                Ok(o) => match o {
                    Some(p) => ProcCollectorDetails::String(p.name().unwrap_or("".to_owned())),
                    None => ProcCollectorDetails::String(String::default()),
                },
                Err(e) => {
                    errlog(format!("Unable to get process' parent (error {:?})", e));
                    ProcCollectorDetails::String(String::default())
                }
            });

            self.details.insert("pid".to_owned(), ProcCollectorDetails::U32(c_pid));
            if self.processes.contains_key(c_pid) {
                let current_process : HashMap<Pid, HashMap<String, ProcessInfo>> = &self.processes[c_pid];
                self.details["name".to_owned()] = ProcCollectorDetails::from(current_process["name".to_owned()]);
                self.details["cmdline".to_owned()] = ProcCollectorDetails::from(current_process["cmd".to_owned()]);
                self.details["threads".to_owned()] = ProcCollectorDetails::from(current_process["threads".to_owned()]);
                self.details["username".to_owned()] = ProcCollectorDetails::from(current_process["username".to_owned()]);
                self.details["memory_percent".to_owned()] = ProcCollectorDetails::from(current_process["mem".to_owned()]);
                let cpu_percent_push = match current_process["cpu".to_owned()] {
                    ProcessInfo::F32(f) => f,
                    _ => {
                        errlog(format!("Malformed cpu percentage from proccollector.processes[{}]", c_pid));
                        0.0
                    },
                };
                self.details["cpu_percent".to_owned()] = ProcCollectorDetails::F32(cpu_percent_push * if CONFIG.proc_per_core {1.0} else {THREADS as f32});
            } else {
                let cmdline_pusher : String = " ".to_owned() + match self.details["cmdline".to_owned()] {
                    ProcCollectorDetails::String(s) => s.as_str(),
                    _ => "",
                };

                self.details["cmdline".to_owned()] = ProcCollectorDetails::String(
                    if cmdline_pusher.len() > 0 {
                        cmdline_pusher
                    } else {
                        "[".to_owned() + self.details["name".to_owned()].as_str() + "]"
                    }
                );

                self.details["threads".to_owned()] = self.details["num_threads".to_owned()];
                self.details["cpu_percent".to_owned()] = ProcCollectorDetails::F64(self.det_cpu);
            }

            self.details["killed".to_owned()] = ProcCollectorDetails::Bool(false);
            if SYSTEM == "MacOS".to_owned() {
                self.details["cpu_num"] = ProcCollectorDetails::None;
                self.details["io_counters"] = ProcCollectorDetails::None;
            }

            self.details["memory_info".to_owned()] = match self.details["memory_info".to_owned()] {
                ProcCollectorDetails::MemoryInfo(m) => ProcCollectorDetails::String(floating_humanizer(m.rss() as f64, false, false, 0, false)),
                _ => ProcCollectorDetails::String("? Bytes".to_owned()),
            }

        }

    }

    

    pub fn get_sorted_processes(
        sort_type: SortingOption,
    ) -> Vec<Process> {
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
                Err(e) => errlog(
                    format!("Unable to get current process information (error {})", e),
                ),
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
        sorting
    }
    
}
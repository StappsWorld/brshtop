use crate::brshtop_box;

use {
    crate::{
        brshtop_box::{
            BrshtopBox,
            Boxes
        },
        collector::Collector,
        config::{Config, SortingOption},
        error::{errlog, throw_error},
    },
    math::round::ceil,
    psutil::{process::*, Pid},
    std::{cmp::Ordering, collections::HashMap, path::Path},
    users::get_user_by_uid,
};

#[derive(Clone)]
// TODO : Fix rest of detail types
pub enum ProcCollectorDetails {
    Bool(bool),
}

#[derive(Clone)]
pub struct ProcCollector {
    pub parent: Collector,
    pub buffer: String,
    pub search_filter: String,
    pub processes: HashMap<String, String>,
    pub num_procs: u32,
    pub det_cpu: f64,
    pub detailed: bool,
    pub detailed_pid: Option<Pid>,
    pub details: HashMap<String, ProcCollectorDetails>, // TODO : Fix types
    pub details_cpu: Vec<u32>,
    pub details_mem: Vec<u32>,
    pub expanded: u32,
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
            processes: HashMap::<String, String>::new(),
            num_procs: 0,
            det_cpu: 0.0,
            detailed: false,
            detailed_pid: None,
            details: HashMap::<String, ProcCollectorDetails>::new(),
            details_cpu: vec![],
            details_mem: vec![],
            expanded: 0,
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
    pub fn collect<P: AsRef<Path>>(&mut self, brshtop_box : &mut BrshtopBox, CONFIG : &mut Config, CONFIG_DIR : P, THREADS : u64) {
        if brshtop_box.stat_mode {
            return
        }

        let mut out : HashMap<String, String> = HashMap::<String, String>::new();
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
            let processes = ProcCollector::get_sorted_processes(sorting, CONFIG_DIR);
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
                            errlog(CONFIG_DIR, format!("Unable to get cpu usage of pid {} (error {})", pid, e));
                            0.0
                        }
                    }
                } else {
                    match p.cpu_percent() {
                        Ok(c) => ceil((c / THREADS as f32) as f64, 2) as f32,
                        Err(e) => {
                            errlog(CONFIG_DIR, format!("Unable to get cpu usage of pid {} (error {})", pid, e));
                            0.0
                        }
                    }
                };
                let mem =  match p.memory_info() {
                    Ok(m) => m,
                    Err(e) => {
                        errlog(CONFIG_DIR, format!("Unable to get memory usage of pid {} (error {})", pid, e));
                        return
                    }
                };
                let mem_b = mem.rss();

            }
        }

    }

    

    pub fn get_sorted_processes<P: AsRef<Path>>(
        sort_type: SortingOption,
        config_dir: P,
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
                    config_dir,
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

use std::{collections::HashMap, path::PathBuf};

use sysinfo::{DiskUsage, Pid, Process, ProcessExt, ProcessStatus, System, SystemExt};

#[derive(Debug, Clone)]
pub struct ProcessT {
    name: String,
    cmd: Vec<String>,
    exe: PathBuf,
    pid: Pid,
    environ: Vec<String>,
    cwd: PathBuf,
    root: PathBuf,
    memory: u64,
    virtual_memory: u64,
    parent: Option<Pid>,
    status: ProcessStatus,
    start_time: u64,
    cpu_usage: f32,
    disk_usage: DiskUsage,
}
impl From<&Process> for ProcessT {
    fn from(proc: &Process) -> Self {
        let name = proc.name().to_string();
        let cmd = proc.cmd().to_vec();
        let exe = proc.exe().to_path_buf();
        let pid = proc.pid();
        let environ = proc.environ().to_vec();
        let cwd = proc.cwd().to_path_buf();
        let root = proc.root().to_path_buf();
        let memory = proc.memory();
        let virtual_memory = proc.virtual_memory();
        let parent = proc.parent();
        let status = proc.status();
        let start_time = proc.start_time();
        let cpu_usage = proc.cpu_usage();
        let disk_usage = proc.disk_usage();

        Self {
            name,
            cmd,
            exe,
            pid,
            environ,
            cwd,
            root,
            memory,
            virtual_memory,
            parent,
            status,
            start_time,
            cpu_usage,
            disk_usage,
        }
    }
}

#[derive(Debug)]
pub struct ProcessTreeNode {
    parent: Option<Pid>,
    children: Vec<Pid>,
    this: ProcessT,
}
impl ProcessTreeNode {
    fn new(process: ProcessT) -> Self {
        Self {
            parent: None,
            children: vec![],
            this: process,
        }
    }

    fn with_parent(mut self, parent: Pid) -> Self {
        self.parent = Some(parent);
        self
    }

    fn push_child(&mut self, child: Pid) {
        self.children.push(child);
    }
}

#[derive(Debug)]
pub struct ProcessTree {
    // TODO(Charlie): Remove pub
    pub tree: HashMap<Pid, ProcessTreeNode>,
    system: System,
}
impl ProcessTree {
    pub fn new() -> Self {
        let system = System::new_all();
        let mut missing_parents: HashMap<Pid, Vec<Pid>> = HashMap::new();

        let processes = system.processes();

        let mut tree: HashMap<Pid, ProcessTreeNode> = HashMap::new();

        for (pid, process) in processes.iter() {
            let parent_pid = process.parent();

            // Skip duplicates
            if tree.contains_key(pid) {
                continue;
            }

            let mut process_node = ProcessTreeNode::new(process.into());

            if let Some(parent_pid) = parent_pid {
                if let Some(parent) = tree.get_mut(&parent_pid) {
                    parent.push_child(*pid);
                } else {
                    missing_parents
                        .entry(parent_pid)
                        .or_insert_with(Vec::new)
                        .push(*pid);
                }

                process_node = process_node.with_parent(parent_pid);
            }

            if let Some(children) = missing_parents.remove(pid) {
                process_node.children = children;
            }

            tree.insert(*pid, process_node);
        }
        Self { tree, system }
    }
}

pub fn collect(system: &System) -> HashMap<Pid, ProcessTreeNode> {
    let mut missing_parents: HashMap<Pid, Vec<Pid>> = HashMap::new();

    let processes = system.processes();

    let mut tree: HashMap<Pid, ProcessTreeNode> = HashMap::new();

    for (pid, process) in processes.iter() {
        let parent_pid = process.parent();

        // Skip duplicates
        if tree.contains_key(pid) {
            continue;
        }

        let mut process_node = ProcessTreeNode::new(process.into());

        if let Some(parent_pid) = parent_pid {
            if let Some(parent) = tree.get_mut(&parent_pid) {
                parent.push_child(*pid);
            } else {
                missing_parents
                    .entry(parent_pid)
                    .or_insert_with(Vec::new)
                    .push(*pid);
            }

            process_node = process_node.with_parent(parent_pid);
        }

        if let Some(children) = missing_parents.remove(pid) {
            process_node.children = children;
        }

        tree.insert(*pid, process_node);
    }

    tree
}

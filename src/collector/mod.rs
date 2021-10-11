use std::collections::HashMap;

use sysinfo::{Pid, System, SystemExt};

use self::{
    cpu::CpuData, disk::DiskData, memory::MemoryData, network::Network, process::ProcessTreeNode,
};

pub mod cpu;
pub mod disk;
pub mod memory;
pub mod network;
pub mod process;

#[derive(Debug)]
pub struct Collector {
    process_tree: HashMap<Pid, ProcessTreeNode>,
    cpu: CpuData,
    memory: MemoryData,
    disk: DiskData,
    networks: Vec<Network>,
    system: System,
}
impl Collector {
    pub async fn new() -> heim::Result<Self> {
        let system = System::new_all();

        let process_tree = process::collect(&system);
        let networks = network::collect(&system);

        let (cpu, memory, disk) =
            tokio::try_join!(cpu::collect(), memory::collect(), disk::collect())?;

        Ok(Self {
            process_tree,
            cpu,
            memory,
            disk,
            networks,
            system,
        })
    }

    pub async fn update(&mut self) -> heim::Result<()> {
        self.system.refresh_all();

        let process_tree = process::collect(&self.system);
        let networks = network::collect(&self.system);

        let (cpu, memory, disk) =
            tokio::try_join!(cpu::collect(), memory::collect(), disk::collect())?;

        self.process_tree = process_tree;
        self.cpu = cpu;
        self.memory = memory;
        self.disk = disk;
        self.networks = networks;

        Ok(())
    }
}

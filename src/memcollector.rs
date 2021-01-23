use std::fs::File;

use {
    crate::{
        brshtop_box::BrshtopBox,
        collector::Collector,
        config::Config,
        draw::Draw,
        error::{errlog, throw_error},
        floating_humanizer,
        key::Key,
        membox::MemBox,
        menu::Menu,
        meter::Meters,
        SYSTEM,
        term::Term,
        theme::Theme
    },
    futures::{future, stream::StreamExt},
    heim::{
        disk::{io_counters, IoCounters},
    },
    psutil::{
        disk::{DiskUsage, FileSystem},
        memory::{os::linux::VirtualMemoryExt, swap_memory, virtual_memory, SwapMemory, VirtualMemory},
        Bytes,
    },
    std::{collections::HashMap, convert::TryFrom, fmt, path::Path, time::SystemTime},
};

#[derive(Clone)]
pub enum DiskInfo {
    String(String),
    U32(u32),
    U64(u64),
    None,
} impl fmt::Display for DiskInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiskInfo::String(s) => write!(f, "{}", s.to_owned()),
            DiskInfo::U32(u) => write!(f, "{}", u.to_owned()),
            DiskInfo::U64(u) => write!(f, "{}", u.to_owned()),
            DiskInfo::None => write!(f, ""),
        }
    }
}

pub struct MemCollector<'a> {
    parent: Collector<'a>,
    values: HashMap<String, Bytes>,
    vlist: HashMap<String, Vec<Bytes>>,
    percent: HashMap<String, Bytes>,
    string: HashMap<String, String>,
    swap_values: HashMap<String, Bytes>,
    swap_vlist: HashMap<String, Vec<Bytes>>,
    swap_percent: HashMap<String, Bytes>,
    swap_string: HashMap<String, String>,
    disks: HashMap<String, HashMap<String, DiskInfo>>,
    disk_hist: HashMap<String, Vec<Bytes>>,
    timestamp: SystemTime,
    io_error: bool,
    old_disks: Vec<String>,
    excludes: Vec<FileSystem>,
    buffer: String,
}
impl<'a> MemCollector<'a> {
    pub fn new(membox: &mut MemBox) -> Self {
        let mem = MemCollector {
            parent: Collector::new(),
            values: HashMap::<String, Bytes>::new(),
            vlist: HashMap::<String, Vec<Bytes>>::new(),
            percent: HashMap::<String, Bytes>::new(),
            string: HashMap::<String, String>::new(),
            swap_values: HashMap::<String, Bytes>::new(),
            swap_vlist: HashMap::<String, Vec<Bytes>>::new(),
            swap_percent: HashMap::<String, Bytes>::new(),
            swap_string: HashMap::<String, String>::new(),
            disks: HashMap::<String, HashMap<String, DiskInfo>>::new(),
            disk_hist: HashMap::<String, Vec<Bytes>>::new(),
            timestamp: SystemTime::now(),
            io_error: false,
            old_disks: Vec::<String>::new(),
            excludes: vec![FileSystem::Other("squashfs".to_owned())],
            buffer: membox.get_buffer().clone(),
        };
        if SYSTEM.to_owned() == "BSD".to_owned() {
            for s in vec!["devfs", "tmpfs", "procfs", "linprocfs", "gvfs", "fusefs"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
            {
                let mut new_v : Vec<FileSystem> = mem.get_excludes();
                new_v.push(FileSystem::Other(s));
                mem.set_excludes(new_v);
            }
        }
        mem
    }

    pub fn collect(&mut self, CONFIG: &mut Config, membox: &mut MemBox) {
        // * Collect memory
        let mem: VirtualMemory = match virtual_memory() {
            Ok(v) => v,
            Err(e) => {
                errlog(format!(
                    "There was a problem collecting the virtual memory! (error {:?})",
                    e
                ));
                VirtualMemory {
                    total: 0,
                    available: 0,
                    used: 0,
                    free: 0,
                    percent: 0.0,
                    active: 0,
                    inactive: 0,
                    buffers: 0,
                    cached: 0,
                    shared: 0,
                    slab: 0,
                }
            }
        };

        self.set_values_index("cached".to_owned(), mem.cached());
        self.set_values_index("total".to_owned(), mem.total());
        self.set_values_index("free".to_owned(), mem.free());
        self.set_values_index("available".to_owned(), mem.available());
        self.set_values_index("used".to_owned(), u64::try_from(mem.total() as i64 - mem.available() as i64).unwrap_or(0));

        for (key, value) in self.get_values() {
            self.set_string_index(key, floating_humanizer(value as f64, false, false, 0, false));
            if key == "total".to_owned() {
                continue;
            }
            self.set_percent_index(key.clone(), value * 100 / self.get_values_index("total".to_owned()).unwrap_or(1));
            if CONFIG.mem_graphs {
                if !self.get_vlist().contains_key(&key.clone()) {
                    self.vlist.insert(key, vec![]);
                }
                self.vlist
                    .get_mut(&key)
                    .unwrap()
                    .push(self.percent.get(&key).unwrap_or(&0).clone());
                if self.vlist[&key].len() as u32 > membox.get_parent().get_width() {
                    self.vlist.get_mut(&key).unwrap().remove(0);
                }
            }
        }

        // * Collect swap
        if CONFIG.show_swap || CONFIG.swap_disk {
            let swap: SwapMemory = match swap_memory() {
                Ok(s) => s,
                Err(e) => {
                    errlog(format!(
                        "There was a problem collecting the swap memory! (error {:?})",
                        e
                    ));
                    SwapMemory {
                        total: 0,
                        used: 0,
                        free: 0,
                        percent: 0.0,
                        swapped_in: 0,
                        swapped_out: 0,
                    }
                }
            };

            self.swap_values.insert("total".to_owned(), swap.total());
            self.swap_values.insert("free".to_owned(), swap.free());
            self.swap_values
                .insert("used".to_owned(), swap.total() / swap.free());

            if swap.total() > 0 {
                if !membox.swap_on {
                    membox.redraw = true;
                    membox.swap_on = true;
                }
                for (key, value) in self.swap_values {
                    self.swap_string
                        .insert(key, floating_humanizer(value as f64, false, false, 0, false));
                    if key == "total".to_owned() {
                        continue;
                    }
                    self.swap_percent
                        .insert(key, value * 100 / self.swap_values[&key]);
                    if CONFIG.mem_graphs {
                        if !self.swap_vlist.contains_key(&key) {
                            self.swap_vlist.insert(key, vec![]);
                        }
                        self.swap_vlist
                            .get_mut(&key)
                            .unwrap()
                            .push(self.swap_percent.get(&key).unwrap_or(&0).clone());
                        if self.swap_vlist.get(&key).unwrap().len() as u32 > membox.parent.width {
                            self.vlist.get_mut(&key).unwrap().remove(0);
                        }
                    }
                }
            } else {
                if membox.swap_on {
                    membox.redraw = true;
                }
                membox.swap_on = false;
            }
        } else {
            if membox.swap_on {
                membox.redraw = true;
            }
            membox.swap_on = false;
        }

        if !CONFIG.show_disks {
            return;
        }

        // * Collect disks usage
        let mut disk_read: Bytes = 0;
        let mut disk_write: Bytes = 0;
        let mut dev_name: String = String::default();
        let mut disk_name: String = String::default();
        let mut filtering: Vec<String> = Vec::<String>::new();
        let mut filter_exclude: bool = false;
        let mut io_string: String = String::default();
        let mut u_percent: u32 = 0;
        let mut disk_list: Vec<String> = Vec::<String>::new();
        self.disks = HashMap::<String, HashMap<String, DiskInfo>>::new();

        if CONFIG.disks_filter.len() > 0 {
            if CONFIG.disks_filter.starts_with("exclude=") {
                filter_exclude = true;
                let mut adder: Vec<String> = Vec::<String>::new();
                for v in CONFIG
                    .disks_filter
                    .replace("exclude=", "")
                    .trim()
                    .split(',')
                {
                    adder.push(v.trim().to_owned());
                }
                filtering = adder.clone();
            } else {
                let mut adder: Vec<String> = Vec::<String>::new();
                for v in CONFIG.disks_filter.trim().split(',') {
                    adder.push(v.trim().to_owned());
                }
                filtering = adder.clone();
            }
        }

        let io_stream = io_counters();
        let io_counters: HashMap<String, IoCounters> = HashMap::<String, IoCounters>::new();
        io_stream.for_each(|o| match o {
            Ok(counter) => {
                io_counters.insert(counter.device_name().to_str().unwrap().to_owned(), counter);
                future::ready(())
            },
            Err(e) => future::ready(()),
        });

        match psutil::disk::partitions() {
            Ok(disks) => {
                for disk in disks {
                    let mut disk_io: &IoCounters;
                    let mut io_string: String = String::default();
                    let mut disk_name: String = if disk.mountpoint().is_file() {
                        match disk.mountpoint().file_name() {
                            Some(s) => s.to_str().unwrap_or("").to_owned(),
                            None => String::default(),
                        }
                    } else {
                        "root".to_owned()
                    };

                    while disk_list.contains(&disk_name) {
                        disk_name.push_str("_");
                    }

                    disk_list.push(disk_name);
                    if self.excludes.len() > 0 && self.excludes.contains(disk.filesystem()) {
                        continue;
                    }

                    let mut ender : String = String::default();
                    for s in filtering {
                        ender.push_str(s.as_str());
                    }
                    if filtering.len() > 0
                        && ((!filter_exclude && !disk_name.ends_with(ender.as_str()))
                            || (filter_exclude && disk_name.ends_with(ender.as_str())))
                    {
                        continue;
                    }
                    if SYSTEM.to_owned() == "MacOS".to_owned()
                        && disk.mountpoint() == Path::new("/private/var/vm")
                    {
                        continue;
                    }
                    let disk_u: DiskUsage = match psutil::disk::disk_usage(disk.mountpoint()) {
                        Ok(d) => d,
                        Err(e) => {
                            errlog(format!("Unable to get disk usage of disk {:?}", e));
                            DiskUsage {
                                total: 0,
                                used: 0,
                                free: 0,
                                percent: 0.0,
                            }
                        }
                    };
                    let u_percent: u32 = disk_u.percent().round() as u32;
                    self.disks.insert(
                        disk.device().to_owned(),
                        vec![
                            ("name", DiskInfo::String(disk_name)),
                            ("used_percent", DiskInfo::U32(u_percent)),
                            ("free_percent", DiskInfo::U32(100 - u_percent)),
                        ]
                        .iter()
                        .map(|(s, d)| (s.to_owned().to_owned(), d.clone()))
                        .collect::<HashMap<String, DiskInfo>>(),
                    );
                    for (name, val) in vec![
                        ("total", disk_u.total()),
                        ("used", disk_u.used()),
                        ("free", disk_u.free()),
                    ]
                    .iter()
                    .map(|(s, d)| (s.to_owned().to_owned(), d.clone()))
                    .collect::<HashMap<String, Bytes>>()
                    {
                        self.disks
                            .get_mut(&disk.device().to_owned())
                            .unwrap()
                            .insert(
                                name,
                                DiskInfo::String(floating_humanizer(
                                    val as f64, false, false, 0, false,
                                )),
                            );
                    }

                    // * Collect disk io
                    if io_counters.len() > 0 {
                        if SYSTEM.to_owned() == "Linux".to_owned() {
                            dev_name = disk.mountpoint().file_name().unwrap().to_str().unwrap_or("").to_owned();
                            if dev_name.starts_with("md") {
                                match dev_name.find('p') {
                                    Some(u) => dev_name = dev_name[..u].to_owned(),
                                    None => (),
                                }
                            }
                            disk_io = io_counters.get(&dev_name).unwrap().clone();
                        } else if disk.mountpoint() == Path::new("/") {
                            //Not sure if this is called with the heim library :/
                            disk_io = io_counters.get(&"/".to_owned()).unwrap();
                        } else {
                            throw_error("OS disk IO issue... Please post this as a problem in the GitHub with your current OS!!!")
                        }
                        match self.timestamp.elapsed() {
                            Ok(d) => {
                                disk_read = (disk_io.read_bytes().value
                                    - self.disk_hist[disk.device()][0])
                                    / d.as_secs();
                                disk_write = (disk_io.write_bytes().value
                                    - self.disk_hist[disk.device()][1])
                                    / d.as_secs();
                            }
                            Err(e) => {
                                errlog(format!("Error with system time... (error {:?})", e));
                                disk_read = 0;
                                disk_write = 0;
                            }
                        };
                    } else {
                        errlog("No disks???".to_owned());
                        disk_read = 0;
                        disk_write = 0;
                    }


                    match self.disk_hist.get_mut(&disk.device().to_owned()) {
                        Some(v) => v = &mut vec![disk_io.read_bytes().value, disk_io.write_bytes().value],
                        None => errlog(format!(
                            "disk_hist did not have {}...",
                            disk.device().to_owned()
                        )),
                    }
                    if membox.disks_width > 30 {
                        if disk_read > 0 {
                            io_string.push_str(format!(
                                "▲{}",
                                floating_humanizer(disk_read as f64, false, false, 0, true)
                            ).as_str());
                        }
                        if disk_write > 0 {
                            io_string.push_str(format!(
                                "▼{}",
                                floating_humanizer(
                                    disk_write as f64,
                                    false,
                                    false,
                                    0,
                                    true
                                )
                            ).as_str());
                        }
                    } else if disk_read + disk_write > 0 {
                        io_string.push_str(format!(
                            "▼▲{}",
                            floating_humanizer(
                                (disk_read + disk_write) as f64,
                                false,
                                false,
                                0,
                                true
                            )
                        ).as_str());
                    }
                


                    match self.disks.get_mut(&disk.device().to_owned()) {
                        Some(h) => {
                            h.insert("io".to_owned(), DiskInfo::String(io_string.clone()));
                            ()
                        }
                        None => errlog(format!("Unable to get {} from disks...", disk.device())),
                    }
                }

                if CONFIG.swap_disk && membox.swap_on {
                    match self.disks.get_mut(&"__swap".to_owned()) {
                        Some(h) => {
                            h = &mut vec![
                                ("name", DiskInfo::String("swap".to_owned())),
                                (
                                    "used_percent",
                                    DiskInfo::U64(self.swap_percent[&"used".to_owned()]),
                                ),
                                (
                                    "free_percent",
                                    DiskInfo::U64(self.swap_percent[&"free".to_owned()]),
                                ),
                                ("io", DiskInfo::None),
                            ]
                            .iter()
                            .map(|(s, d)| (s.to_owned().to_owned(), d.clone()))
                            .collect::<HashMap<String, DiskInfo>>();

                            for name in vec!["total", "used", "free"]
                                .iter()
                                .map(|s| s.to_owned().to_owned())
                                .collect::<Vec<String>>()
                            {
                                h.insert(name, DiskInfo::String(self.swap_string.get(&name).unwrap_or(&String::default()).clone()));
                            }

                            if self.disks.len() > 2 {
                                let new: HashMap<String, HashMap<String, DiskInfo>> = vec![(
                                    self.disks.keys().cloned().collect::<Vec<String>>()[0].clone(),
                                    self.disks
                                        .get(&self.disks.keys().map(|k| k.to_string()).collect::<Vec<String>>()[0].clone())
                                        .unwrap()
                                        .clone()
                                )]
                                .iter()
                                .cloned()
                                .collect::<HashMap<String, HashMap<String, DiskInfo>>>();

                                new.insert("__swap".to_owned(), h.clone());
                                self.disks.remove(&"__swap".to_owned());

                                for (key, map) in self.disks {
                                    new.insert(key, map);
                                }
                                self.disks = new.clone();
                            }
                        }
                        None => (),
                    };
                }

                if disk_list != self.old_disks {
                    membox.redraw = true;
                    self.old_disks = disk_list.clone();
                }

                self.timestamp = SystemTime::now();
            }
            Err(e) => errlog(format!(
                "Unable to get a disk partitions... (error {:?})",
                e
            )),
        }
    }

    /// JUST CALL MemBox.draw_fg()
    pub fn draw(&mut self, membox: &mut MemBox, term : &mut Term, brshtop_box : &mut BrshtopBox, CONFIG : &mut Config, meters : &mut Meters, THEME : &mut Theme, key : &mut Key, collector : &mut Collector, draw : &mut Draw, menu : &mut Menu) {
        membox.draw_fg(self, term, brshtop_box, CONFIG, meters, THEME, key, collector, draw, menu);
    }

    pub fn get_parent(&self) -> Collector<'a> {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent : Collector<'a>) {
        self.parent = parent.clone()
    }

    pub fn get_values(&self) -> HashMap<String, Bytes> {
        self.values.clone()
    }

    pub fn set_values(&mut self, values : HashMap<String, Bytes>) {
        self.values = values.clone()
    }

    pub fn get_values_index(&self, index : String) -> Option<Bytes> {
        match self.values.get(&index.clone()) {
            Some(u) => Some(u.to_owned().clone()),
            None => None 
        }
    }

    pub fn set_values_index(&mut self, index : String, element : Bytes) {
        self.values.insert(index.clone(), element.clone());
    }

    pub fn get_vlist(&self) -> HashMap<String, Vec<Bytes>> {
        self.vlist.clone()
    }

    pub fn set_vlist(&mut self, vlist : HashMap<String, Vec<Bytes>>) {
        self.vlist = vlist.clone()
    }

    pub fn get_vlist_index(&self, index : String) -> Option<Vec<u64>> {
        match self.get_vlist().get(&index.clone()) {
            Some(u) => Some(u.iter().cloned().collect()),
            None => None,
        }
    }

    pub fn set_vlist_index(&mut self, index : String, element : Vec<Bytes>) {
        self.vlist.insert(index.clone(), element.clone());
    }

    pub fn get_vlist_inner_index(&self, index1 : String, index2 : usize) -> Option<Bytes> {
        match self.get_vlist().get(&index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_vlist_inner_index(&mut self, index1 : String, index2 : usize, element : Bytes) {
        self.set_vlist_index(index1.clone(), match self.get_vlist_index(index1.clone()) {
            Some(v) => {
                let mut new_v = v.clone();
                new_v.insert(index2, element.clone());
                new_v
            },
            None => {
                let mut new_v : Vec<Bytes> = Vec::<Bytes>::new();
                for i in 0..index2 {
                    new_v.push(0);
                }
                new_v.push(element.clone());
                new_v
            },
        })
    }

    pub fn get_percent(&self) -> HashMap<String, Bytes> {
        self.percent.clone()
    }

    pub fn set_percent(&mut self, percent : HashMap<String, Bytes>) {
        self.percent = percent.clone()
    }

    pub fn get_percent_index(&self, index : String) -> Option<Bytes> {
        match self.get_percent().get(&index.clone()) {
            Some(b) => Some(b.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_percent_index(&mut self, index : String, element : Bytes) {
        self.percent.insert(index.clone(), element.clone());
    }

    pub fn get_string(&self) -> HashMap<String, String> {
        self.string.clone()
    }

    pub fn set_string(&mut self, string : HashMap<String, String>) {
        self.string = string.clone()
    }

    pub fn get_string_index(&self, index : String) -> Option<String> {
        match self.get_string().get(&index.to_owned().clone()) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_string_index(&mut self, index : String, element : String) {
        self.string.insert(index.clone(), element.clone());
    }

    pub fn get_swap_values(&self) -> HashMap<String, Bytes> {
        self.swap_values.clone()
    }

    pub fn set_swap_values(&mut self, swap_values : HashMap<String, Bytes>) {
        self.swap_values = swap_values.clone()
    }

    pub fn get_swap_values_index(&self, index : String) -> Option<Bytes> {
        match self.get_swap_values().get(&index.to_owned().clone()) {
            Some(u ) => Some(u.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_values_index(&mut self, index : String, element : Bytes) {
        self.swap_values.insert(index.clone(), element.clone());
    }

    pub fn get_swap_vlist(&self) -> HashMap<String, Vec<Bytes>> {
        self.swap_vlist.clone()
    }

    pub fn set_swap_vlist(&mut self, swap_vlist : HashMap<String, Vec<Bytes>>) {
        self.swap_vlist = swap_vlist.clone()
    }

    pub fn get_swap_vlist_index(&self, index : String) -> Option<Vec<u64>> {
        match self.get_swap_vlist().get(&index.clone()) {
            Some(u) => Some(u.iter().cloned().collect()),
            None => None,
        }
    }

    pub fn set_swap_vlist_index(&mut self, index : String, element : Vec<Bytes>) {
        self.swap_vlist.insert(index.clone(), element.clone());
    }

    pub fn get_swap_vlist_inner_index(&self, index1 : String, index2 : usize) -> Option<Bytes> {
        match self.get_swap_vlist().get(&index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_swap_vlist_inner_index(&mut self, index1 : String, index2 : usize, element : Bytes) {
        self.set_swap_vlist_index(index1.clone(), match self.get_swap_vlist_index(index1.clone()) {
            Some(v) => {
                let mut new_v = v.clone();
                new_v.insert(index2, element.clone());
                new_v
            },
            None => {
                let mut new_v : Vec<Bytes> = Vec::<Bytes>::new();
                for _ in 0..index2 {
                    new_v.push(0);
                }
                new_v.push(element.clone());
                new_v
            },
        })
    }

    pub fn get_swap_percent(&self) -> HashMap<String, Bytes> {
        self.swap_percent.clone()
    }

    pub fn set_swap_percent(&mut self, swap_percent : HashMap<String, Bytes>) {
        self.swap_percent = swap_percent.clone()
    }

    pub fn get_swap_percent_index(&self, index : String) -> Option<Bytes> {
        match self.get_swap_percent().get(&index.clone()) {
            Some(b) => Some(b.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_percent_index(&mut self, index : String, element : Bytes) {
        self.swap_percent.insert(index.clone(), element.clone());
    }

    pub fn get_swap_string(&self) -> HashMap<String, String> {
        self.swap_string.clone()
    }

    pub fn set_swap_string(&mut self, swap_string : HashMap<String, String>) {
        self.swap_string = swap_string.clone()
    }

    pub fn get_swap_string_index(&self, index : String) -> Option<String> {
        match self.get_swap_string().get(&index.to_owned().clone()) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_string_index(&mut self, index : String, element : String) {
        self.swap_string.insert(index.clone(), element.clone());
    }

    pub fn get_disks(&self) -> HashMap<String, HashMap<String, DiskInfo>> {
        self.disks.clone()
    }

    pub fn set_disks(&mut self, disks : HashMap<String, HashMap<String, DiskInfo>>) {
        self.disks = disks.clone()
    }

    pub fn get_disks_index(&self, index : String) -> Option<HashMap<String, DiskInfo>> {
        match self.get_disks().get(&index.clone()) {
            Some(h) => Some(h.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_disks_index(&mut self, index : String, element : HashMap<String, DiskInfo>) {
        self.disks.insert(index.clone(), element.clone());
    }

    pub fn get_disks_inner_index(&self, index1 : String, index2 : String) -> Option<DiskInfo> {
        match self.get_disks_index(index1.clone()) {
            Some(h) => match h.to_owned().get(&index2.clone()) {
                Some(d) => Some(d.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_disks_inner_index(&mut self, index1 : String, index2 : String, element : DiskInfo) {
        self.set_disks_index(index1.clone(), match self.get_disks_index(index1.clone()) {
            Some(h) => {
                let mut new_h : HashMap<String, DiskInfo> = h.clone();
                new_h.insert(index2.clone(), element.clone());
                new_h
            },
            None => {
                let mut new_h : HashMap<String, DiskInfo> = HashMap::<String, DiskInfo>::new();
                new_h.insert(index2.clone(), element.clone());
                new_h
            },
        })
    }

    pub fn get_disk_hist(&self) -> HashMap<String, Vec<Bytes>> {
        self.disk_hist.clone()
    }

    pub fn set_disk_hist(&mut self, disk_hist : HashMap<String, Vec<Bytes>>) {
        self.disk_hist = disk_hist.clone()
    }

    pub fn get_disk_hist_index(&self, index : String) -> Option<Vec<Bytes>> {
        match self.get_disk_hist().get(&index.clone()) {
            Some(v) => Some(v.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_disk_hist_index(&mut self, index : String, element : Vec<Bytes>) {
        self.disk_hist.insert(index.clone(), element.clone());
    }

    pub fn get_disk_hist_inner_index(&self, index1 : String, index2 : usize) -> Option<Bytes> {
        match self.get_disk_hist_index(index1.clone()) {
            Some(v) => match v.get(index2.clone()) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_disk_hist_inner_index(&mut self, index1 : String, index2 : usize, element : Bytes) {
        self.set_disk_hist_index(index1.clone(), match self.get_disk_hist_index(index1.clone()) {
            Some(v) => {
                let mut new_v = v.clone();
                new_v.insert(index2, element.clone());
                new_v
            },
            None => {
                let mut new_v : Vec<Bytes> = Vec::<Bytes>::new();
                for i in 0..index2 {
                    new_v.push(0);
                }
                new_v.push(element.clone());
                new_v
            },
        })
    }

    pub fn get_timestamp(&self) -> SystemTime {
        self.timestamp.clone()
    }

    pub fn set_timestamp(&mut self, timestamp : SystemTime) {
        self.timestamp = timestamp.clone()
    }

    pub fn get_io_error(&self) -> bool {
        self.io_error.clone()
    }

    pub fn set_io_error(&mut self, io_error : bool) {
        self.io_error = io_error.clone()
    }

    pub fn get_old_disks(&self) -> Vec<String> {
        self.old_disks.clone()
    }

    pub fn set_old_disks(&mut self, old_disks : Vec<String>) {
        self.old_disks = old_disks.clone()
    }

    pub fn get_old_disks_index(&self, index : usize) -> Option<String> {
        match self.get_old_disks().get(index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_old_disks_index(&mut self, index : usize, element : String) {
        self.old_disks.insert(index, element.clone())
    }

    pub fn get_excludes(&self) -> Vec<FileSystem> {
        self.excludes.clone()
    }

    pub fn set_excludes(&mut self, excludes : Vec<FileSystem>) {
        self.excludes = excludes.clone()
    }

    pub fn get_excludes_index(&self, index : usize) -> Option<FileSystem> {
        match self.excludes.get(index) {
            Some(f) => Some(f.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_excludes_index(&mut self, index : usize, element : FileSystem) {
        self.excludes.insert(index, element.clone());
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer : String) {
        self.buffer = buffer.clone()
    }

}

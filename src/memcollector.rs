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
    futures::{Stream, task::Poll},
    heim::{
        disk::{io_counters, IoCounters},
        units::information::Conversion,
    },
    psutil::{
        disk::{DiskUsage, FileSystem},
        memory::{os::linux::VirtualMemoryExt, swap_memory, virtual_memory, SwapMemory, VirtualMemory},
        Bytes,
    },
    std::{collections::HashMap, fmt, path::Path, time::SystemTime},
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
    pub parent: Collector<'a>,
    pub values: HashMap<String, Bytes>,
    pub vlist: HashMap<String, Vec<Bytes>>,
    pub percent: HashMap<String, Bytes>,
    pub string: HashMap<String, String>,
    pub swap_values: HashMap<String, Bytes>,
    pub swap_vlist: HashMap<String, Vec<Bytes>>,
    pub swap_percent: HashMap<String, Bytes>,
    pub swap_string: HashMap<String, String>,
    pub disks: HashMap<String, HashMap<String, DiskInfo>>,
    pub disk_hist: HashMap<String, Vec<Bytes>>,
    pub timestamp: SystemTime,
    pub io_error: bool,
    pub old_disks: Vec<String>,
    pub excludes: Vec<FileSystem>,
    pub buffer: String,
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
            buffer: membox.buffer.clone(),
        };
        if SYSTEM.to_owned() == "BSD".to_owned() {
            for s in vec!["devfs", "tmpfs", "procfs", "linprocfs", "gvfs", "fusefs"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
            {
                mem.excludes.push(FileSystem::Other(s));
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

        self.values.insert("cached".to_owned(), mem.cached());
        self.values.insert("total".to_owned(), mem.total());
        self.values.insert("free".to_owned(), mem.free());
        self.values.insert("available".to_owned(), mem.available());
        self.values
            .insert("used".to_owned(), mem.total() - mem.available());

        for (key, value) in self.values {
            self.string
                .insert(key, floating_humanizer(value as f64, false, false, 0, false));
            if key == "total".to_owned() {
                continue;
            }
            self.percent[&key] = value * 100 / self.values[&"total".to_owned()];
            if CONFIG.mem_graphs {
                if !self.vlist.contains_key(&key) {
                    self.vlist.insert(key, vec![]);
                }
                self.vlist
                    .get_mut(&key)
                    .unwrap()
                    .push(self.percent.get(&key).unwrap_or(&0).clone());
                if self.vlist[&key].len() as u32 > membox.parent.width {
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
        let mut looping = true;
        while looping {
            match io_stream.poll_next() {
                Poll::Pending => (),
                Poll::Ready(o) => match o {
                    Some(counter) => {
                        io_counters.insert(counter.device_name.to_str().to_owned(), counter)
                    }
                    None => looping = false,
                },
            };
        }

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
                                        .get(self.disks.keys().cloned().collect()[0].clone())
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

}

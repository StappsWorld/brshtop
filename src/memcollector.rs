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
        term::Term,
        theme::Theme,
        SYSTEM,
    },
    futures::{future, stream::StreamExt},
    heim::disk::{io_counters, IoCounters},
    once_cell::sync::OnceCell,
    psutil::{
        disk::{DiskUsage, FileSystem},
        memory::{
            os::linux::VirtualMemoryExt, swap_memory, virtual_memory, SwapMemory, VirtualMemory,
        },
        Bytes,
    },
    std::{
        collections::HashMap,
        convert::TryFrom,
        fmt,
        path::Path,
        sync::{Arc, Mutex},
        time::SystemTime,
    },
};

#[derive(Clone)]
pub enum DiskInfo {
    String(String),
    U32(u32),
    U64(u64),
    None,
}
impl fmt::Display for DiskInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiskInfo::String(s) => write!(f, "{}", s.to_owned()),
            DiskInfo::U32(u) => write!(f, "{}", u.to_owned()),
            DiskInfo::U64(u) => write!(f, "{}", u.to_owned()),
            DiskInfo::None => write!(f, ""),
        }
    }
}

pub struct MemCollector {
    parent: Collector,
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
impl MemCollector {
    pub fn new(membox: &MemBox) -> Self {
        let mut mem = MemCollector {
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
                let mut new_v: Vec<FileSystem> = mem.get_excludes();
                new_v.push(FileSystem::Other(s));
                mem.set_excludes(new_v);
            }
        }
        mem
    }

    pub fn collect(&mut self, CONFIG: &Config, membox: &mut MemBox) {
        // * Collect memory
        let mem: VirtualMemory = match virtual_memory() {
            Ok(v) => v,
            Err(e) => {
                errlog(format!(
                    "There was a problem collecting the virtual memory! (error {:?})",
                    e
                ));
                return;
            }
        };

        self.set_values_index("cached".to_owned(), mem.cached());
        self.set_values_index("total".to_owned(), mem.total());
        self.set_values_index("free".to_owned(), mem.free());
        self.set_values_index("available".to_owned(), mem.available());
        self.set_values_index(
            "used".to_owned(),
            u64::try_from(mem.total() as i64 - mem.available() as i64).unwrap_or(0),
        );

        for (key, value) in self.get_values() {
            self.set_string_index(
                key.clone(),
                floating_humanizer(value as f64, false, false, 0, false),
            );
            if key.clone() == "total".to_owned() {
                continue;
            }
            self.set_percent_index(
                key.clone(),
                value * 100 / self.get_values_index("total".to_owned()).unwrap_or(1),
            );
            if CONFIG.mem_graphs {
                if !self.get_vlist().contains_key(&key.clone()) {
                    self.vlist.insert(key.clone(), vec![]);
                }
                self.push_vlist_inner_index(
                    key.clone(),
                    self.get_percent_index(key.clone()).unwrap_or(0),
                );
                if self.get_vlist_index(key.clone()).unwrap_or(vec![]).len() as u32
                    > membox
                        .get_parent()
                        .get_width()
                {
                    match self.remove_vlist_inner_index(key.clone(), 0) {
                        Err(s) => errlog(format!(
                            "There was a problem removing an index in vlist (error: {})",
                            s.clone()
                        )),
                        _ => (),
                    }
                }
            }
        }

        // * Collect swap
        if CONFIG.show_swap
            || CONFIG.swap_disk
        {
            let swap: SwapMemory = match swap_memory() {
                Ok(s) => s,
                Err(e) => {
                    errlog(format!(
                        "There was a problem collecting the swap memory! (error {:?})",
                        e
                    ));
                    return;
                }
            };

            self.set_swap_values_index("total".to_owned(), swap.total());
            self.set_swap_values_index("free".to_owned(), swap.free());
            self.set_swap_values_index("used".to_owned(), swap.total() / swap.free());

            if swap.total() > 0 {
                if !membox.get_swap_on() {
                    membox.set_redraw(true);
                    membox.set_swap_on(true);
                }
                for (key, value) in self.get_swap_values() {
                    self.set_swap_string_index(
                        key.clone(),
                        floating_humanizer(value.clone() as f64, false, false, 0, false),
                    );
                    if key.clone() == "total".to_owned() {
                        continue;
                    }
                    self.set_swap_percent_index(
                        key.clone(),
                        value.clone() * 100 / self.get_swap_values_index(key.clone()).unwrap_or(1),
                    );
                    if CONFIG.mem_graphs {
                        if !self.get_swap_vlist().contains_key(&key.clone()) {
                            self.set_swap_vlist_index(key.clone(), vec![]);
                        }
                        self.push_swap_vlist_inner_index(
                            key.clone(),
                            self.get_swap_percent_index(key.clone()).unwrap_or(0),
                        );
                        if self
                            .get_swap_vlist_index(key.clone())
                            .unwrap_or(vec![])
                            .len() as u32
                            > membox
                                .get_parent()
                                .get_width()
                        {
                            self.remove_vlist_inner_index(key.clone(), 0);
                        }
                    }
                }
            } else {
                if membox.get_swap_on() {
                    membox.set_redraw(true);
                    membox.set_swap_on(false);
                }
            }
        } else {
            if membox.get_swap_on() {
                membox.set_redraw(true);
                membox.set_swap_on(false);
            }
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
        self.set_disks(HashMap::<String, HashMap<String, DiskInfo>>::new());

        if CONFIG.disks_filter.len() > 0 {
            if CONFIG
                .disks_filter
                .starts_with("exclude=")
            {
                filter_exclude = true;
                let mut adder: Vec<String> = Vec::<String>::new();
                for v in CONFIG
                    .disks_filter
                    .clone()
                    .replace("exclude=", "")
                    .trim()
                    .split(',')
                {
                    adder.push(v.trim().to_owned());
                }
                filtering = adder.clone();
            } else {
                let mut adder: Vec<String> = Vec::<String>::new();
                for v in CONFIG
                    .disks_filter
                    .clone()
                    .trim()
                    .split(',')
                {
                    adder.push(v.trim().to_owned());
                }
                filtering = adder.clone();
            }
        }

        let io_stream = io_counters();
        let mut io_counters: HashMap<String, IoCounters> = HashMap::<String, IoCounters>::new();
        io_stream.for_each(|o| match o {
            Ok(counter) => {
                io_counters.insert(counter.device_name().to_str().unwrap().to_owned(), counter);
                future::ready(())
            }
            Err(e) => future::ready(()),
        });

        match psutil::disk::partitions() {
            Ok(disks) => {
                for disk in disks {
                    let mut disk_io: &IoCounters;
                    let mut io_string: String = String::default();
                    let mut disk_name: String = if disk.clone().mountpoint().is_file() {
                        match disk.clone().mountpoint().file_name() {
                            Some(s) => s.to_str().unwrap_or("").to_owned(),
                            None => String::default(),
                        }
                    } else {
                        "root".to_owned()
                    };

                    while disk_list.contains(&disk_name.clone()) {
                        disk_name.push_str("_");
                    }

                    disk_list.push(disk_name.clone());
                    if self.get_excludes().len() > 0
                        && self.get_excludes().contains(disk.clone().filesystem())
                    {
                        continue;
                    }

                    let mut ender: String = String::default();
                    for s in filtering.clone() {
                        ender.push_str(s.as_str());
                    }
                    if filtering.len() > 0
                        && ((!filter_exclude && !disk_name.ends_with(ender.as_str()))
                            || (filter_exclude && disk_name.ends_with(ender.as_str())))
                    {
                        continue;
                    }
                    if SYSTEM.to_owned() == "MacOS".to_owned()
                        && disk.clone().mountpoint() == Path::new("/private/var/vm")
                    {
                        continue;
                    }
                    let disk_u: DiskUsage =
                        match psutil::disk::disk_usage(disk.clone().mountpoint()) {
                            Ok(d) => d,
                            Err(e) => {
                                errlog(format!("Unable to get disk usage of disk {:?}", e));
                                return;
                            }
                        };
                    let u_percent: u32 = disk_u.clone().percent().round() as u32;
                    self.set_disks_index(
                        disk.clone().device().to_owned(),
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
                        self.set_disks_inner_index(
                            disk.clone().device().to_owned(),
                            name.clone(),
                            DiskInfo::String(floating_humanizer(
                                val as f64, false, false, 0, false,
                            )),
                        );
                    }

                    // * Collect disk io
                    if io_counters.len() > 0 {
                        if SYSTEM.to_owned() == "Linux".to_owned() {
                            dev_name = disk
                                .clone()
                                .mountpoint()
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap_or("")
                                .to_owned();
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
                            throw_error("OS disk IO issue... Please post this as a problem in the GitHub with your current OS!!!");
                            return;
                        }
                        match self.get_timestamp().elapsed() {
                            Ok(d) => {
                                if d.as_secs() > 0 {
                                    disk_read = (disk_io.read_bytes().value
                                        - self
                                            .get_disk_hist_inner_index(
                                                disk.clone().device().to_owned(),
                                                0,
                                            )
                                            .unwrap_or(0))
                                        / d.as_secs();
                                    disk_write = (disk_io.write_bytes().value
                                        - self
                                            .get_disk_hist_inner_index(
                                                disk.clone().device().to_owned(),
                                                1,
                                            )
                                            .unwrap_or(0))
                                        / d.as_secs();
                                } else {
                                    errlog(
                                        "No time has passed since last disk read/write!!!!"
                                            .to_owned(),
                                    );
                                    disk_read = 0;
                                    disk_write = 0;
                                }
                            }
                            Err(e) => {
                                errlog(format!("Error with system time... (error {:?})", e));
                                disk_read = 0;
                                disk_write = 0;
                            }
                        };
                    } else {
                        errlog("No disks???".to_owned());
                        return;
                    }

                    self.set_disk_hist_index(
                        disk.clone().device().to_owned(),
                        vec![disk_io.read_bytes().value, disk_io.write_bytes().value],
                    );

                    if membox.get_disks_width() > 30 {
                        if disk_read > 0 {
                            io_string.push_str(
                                format!(
                                    "▲{}",
                                    floating_humanizer(
                                        disk_read.clone() as f64,
                                        false,
                                        false,
                                        0,
                                        true
                                    )
                                )
                                .as_str(),
                            );
                        }
                        if disk_write > 0 {
                            io_string.push_str(
                                format!(
                                    "▼{}",
                                    floating_humanizer(
                                        disk_write.clone() as f64,
                                        false,
                                        false,
                                        0,
                                        true
                                    )
                                )
                                .as_str(),
                            );
                        }
                    } else if disk_read + disk_write > 0 {
                        io_string.push_str(
                            format!(
                                "▼▲{}",
                                floating_humanizer(
                                    (disk_read.clone() + disk_write.clone()) as f64,
                                    false,
                                    false,
                                    0,
                                    true
                                )
                            )
                            .as_str(),
                        );
                    }

                    self.set_disks_inner_index(
                        disk.clone().device().to_owned(),
                        "io".to_owned(),
                        DiskInfo::String(io_string.clone()),
                    );
                }

                if CONFIG.swap_disk
                    && membox.get_swap_on()
                {
                    self.set_disks_index("__swap".to_owned(), {
                        let mut h = vec![
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
                            h.insert(
                                name.clone(),
                                DiskInfo::String(
                                    self.get_swap_string_index(name.clone())
                                        .unwrap_or(String::default())
                                        .clone(),
                                ),
                            );
                        }

                        h
                    });
                    if self.get_disks().len() > 2 {
                        let mut new: HashMap<String, HashMap<String, DiskInfo>> = vec![(
                            self.get_disks().keys().cloned().collect::<Vec<String>>()[0].clone(),
                            self.get_disks_index(
                                self.get_disks()
                                    .keys()
                                    .map(|k| k.to_string())
                                    .collect::<Vec<String>>()[0]
                                    .clone(),
                            )
                            .unwrap(),
                        )]
                        .iter()
                        .cloned()
                        .collect::<HashMap<String, HashMap<String, DiskInfo>>>();

                        new.insert(
                            "__swap".to_owned(),
                            self.get_disks_index("__swap".to_owned()).unwrap().clone(),
                        );

                        let remover = "__swap".to_owned();

                        self.remove_disks_index(remover);

                        for (key, map) in self.get_disks() {
                            new.insert(key, map);
                        }
                        self.set_disks(new.clone());
                    }
                }

                if disk_list != self.get_old_disks() {
                    membox.set_redraw(true);
                    self.set_old_disks(disk_list.clone());
                }

                self.set_timestamp(SystemTime::now());
            }
            Err(e) => errlog(format!(
                "Unable to get a disk partitions... (error {:?})",
                e
            )),
        }
    }

    /// JUST CALL MemBox.draw_fg()
    pub fn draw(
        &mut self,
        membox: &mut MemBox,
        term: &Term,
        brshtop_box: &mut BrshtopBox,
        CONFIG: &Config,
        meters: &mut Meters,
        THEME: &Theme,
        key: &mut Key,
        collector: &Collector,
        draw: &mut Draw,
        menu: &Menu,
    ) {
        membox.draw_fg(
            self,
            term,
            brshtop_box,
            CONFIG,
            meters,
            THEME,
            key,
            collector,
            draw,
            menu,
        );
    }

    pub fn get_parent(&self) -> Collector {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent: Collector) {
        self.parent = parent.clone()
    }

    pub fn get_values(&self) -> HashMap<String, Bytes> {
        self.values.clone()
    }

    pub fn set_values(&mut self, values: HashMap<String, Bytes>) {
        self.values = values.clone()
    }

    pub fn get_values_index(&self, index: String) -> Option<Bytes> {
        match self.values.get(&index.clone()) {
            Some(u) => Some(u.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_values_index(&mut self, index: String, element: Bytes) {
        self.values.insert(index.clone(), element.clone());
    }

    pub fn get_vlist(&self) -> HashMap<String, Vec<Bytes>> {
        self.vlist.clone()
    }

    pub fn set_vlist(&mut self, vlist: HashMap<String, Vec<Bytes>>) {
        self.vlist = vlist.clone()
    }

    pub fn get_vlist_index(&self, index: String) -> Option<Vec<u64>> {
        match self.get_vlist().get(&index.clone()) {
            Some(u) => Some(u.iter().cloned().collect()),
            None => None,
        }
    }

    pub fn set_vlist_index(&mut self, index: String, element: Vec<Bytes>) {
        self.vlist.insert(index.clone(), element.clone());
    }

    pub fn get_vlist_inner_index(&self, index1: String, index2: usize) -> Option<Bytes> {
        match self.get_vlist().get(&index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_vlist_inner_index(&mut self, index1: String, index2: usize, element: Bytes) {
        self.set_vlist_index(
            index1.clone(),
            match self.get_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    new_v.insert(index2, element.clone());
                    new_v
                }
                None => {
                    let mut new_v: Vec<Bytes> = Vec::<Bytes>::new();
                    for i in 0..index2 {
                        new_v.push(0);
                    }
                    new_v.push(element.clone());
                    new_v
                }
            },
        )
    }

    pub fn push_vlist_inner_index(&mut self, index1: String, element: Bytes) -> Result<(), String> {
        self.set_vlist_index(
            index1.clone(),
            match self.get_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    new_v.push(element.clone());
                    new_v
                }
                None => return Err(format!("No vec at index '{}'", index1.clone())),
            },
        );
        Ok(())
    }

    pub fn remove_vlist_inner_index(
        &mut self,
        index1: String,
        remove_index: usize,
    ) -> Result<(), String> {
        self.set_vlist_index(
            index1.clone(),
            match self.get_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    if remove_index > new_v.len() {
                        return Err(format!("Index {} is out of bounds", remove_index));
                    }
                    new_v.remove(remove_index);
                    new_v
                }
                None => return Err(format!("No vec at index '{}'", index1.clone())),
            },
        );
        Ok(())
    }

    pub fn get_percent(&self) -> HashMap<String, Bytes> {
        self.percent.clone()
    }

    pub fn set_percent(&mut self, percent: HashMap<String, Bytes>) {
        self.percent = percent.clone()
    }

    pub fn get_percent_index(&self, index: String) -> Option<Bytes> {
        match self.get_percent().get(&index.clone()) {
            Some(b) => Some(b.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_percent_index(&mut self, index: String, element: Bytes) {
        self.percent.insert(index.clone(), element.clone());
    }

    pub fn get_string(&self) -> HashMap<String, String> {
        self.string.clone()
    }

    pub fn set_string(&mut self, string: HashMap<String, String>) {
        self.string = string.clone()
    }

    pub fn get_string_index(&self, index: String) -> Option<String> {
        match self.get_string().get(&index.to_owned().clone()) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_string_index(&mut self, index: String, element: String) {
        self.string.insert(index.clone(), element.clone());
    }

    pub fn get_swap_values(&self) -> HashMap<String, Bytes> {
        self.swap_values.clone()
    }

    pub fn set_swap_values(&mut self, swap_values: HashMap<String, Bytes>) {
        self.swap_values = swap_values.clone()
    }

    pub fn get_swap_values_index(&self, index: String) -> Option<Bytes> {
        match self.get_swap_values().get(&index.to_owned().clone()) {
            Some(u) => Some(u.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_values_index(&mut self, index: String, element: Bytes) {
        self.swap_values.insert(index.clone(), element.clone());
    }

    pub fn get_swap_vlist(&self) -> HashMap<String, Vec<Bytes>> {
        self.swap_vlist.clone()
    }

    pub fn set_swap_vlist(&mut self, swap_vlist: HashMap<String, Vec<Bytes>>) {
        self.swap_vlist = swap_vlist.clone()
    }

    pub fn get_swap_vlist_index(&self, index: String) -> Option<Vec<u64>> {
        match self.get_swap_vlist().get(&index.clone()) {
            Some(u) => Some(u.iter().cloned().collect()),
            None => None,
        }
    }

    pub fn set_swap_vlist_index(&mut self, index: String, element: Vec<Bytes>) {
        self.swap_vlist.insert(index.clone(), element.clone());
    }

    pub fn get_swap_vlist_inner_index(&self, index1: String, index2: usize) -> Option<Bytes> {
        match self.get_swap_vlist().get(&index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_swap_vlist_inner_index(&mut self, index1: String, index2: usize, element: Bytes) {
        self.set_swap_vlist_index(
            index1.clone(),
            match self.get_swap_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    new_v.insert(index2, element.clone());
                    new_v
                }
                None => {
                    let mut new_v: Vec<Bytes> = Vec::<Bytes>::new();
                    for _ in 0..index2 {
                        new_v.push(0);
                    }
                    new_v.push(element.clone());
                    new_v
                }
            },
        )
    }

    pub fn push_swap_vlist_inner_index(
        &mut self,
        index1: String,
        element: Bytes,
    ) -> Result<(), String> {
        self.set_swap_vlist_index(
            index1.clone(),
            match self.get_swap_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    new_v.push(element.clone());
                    new_v
                }
                None => return Err(format!("No vec at index '{}'", index1.clone())),
            },
        );
        Ok(())
    }

    pub fn remove_swap_vlist_inner_index(
        &mut self,
        index1: String,
        remove_index: usize,
    ) -> Result<(), String> {
        self.set_swap_vlist_index(
            index1.clone(),
            match self.get_swap_vlist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    if remove_index > new_v.len() {
                        return Err(format!("Index {} is out of bounds", remove_index));
                    }
                    new_v.remove(remove_index);
                    new_v
                }
                None => return Err(format!("No vec at index '{}'", index1.clone())),
            },
        );
        Ok(())
    }

    pub fn get_swap_percent(&self) -> HashMap<String, Bytes> {
        self.swap_percent.clone()
    }

    pub fn set_swap_percent(&mut self, swap_percent: HashMap<String, Bytes>) {
        self.swap_percent = swap_percent.clone()
    }

    pub fn get_swap_percent_index(&self, index: String) -> Option<Bytes> {
        match self.get_swap_percent().get(&index.clone()) {
            Some(b) => Some(b.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_percent_index(&mut self, index: String, element: Bytes) {
        self.swap_percent.insert(index.clone(), element.clone());
    }

    pub fn get_swap_string(&self) -> HashMap<String, String> {
        self.swap_string.clone()
    }

    pub fn set_swap_string(&mut self, swap_string: HashMap<String, String>) {
        self.swap_string = swap_string.clone()
    }

    pub fn get_swap_string_index(&self, index: String) -> Option<String> {
        match self.get_swap_string().get(&index.to_owned().clone()) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_string_index(&mut self, index: String, element: String) {
        self.swap_string.insert(index.clone(), element.clone());
    }

    pub fn get_disks(&self) -> HashMap<String, HashMap<String, DiskInfo>> {
        self.disks.clone()
    }

    pub fn set_disks(&mut self, disks: HashMap<String, HashMap<String, DiskInfo>>) {
        self.disks = disks.clone()
    }

    pub fn get_disks_index(&self, index: String) -> Option<HashMap<String, DiskInfo>> {
        match self.get_disks().get(&index.clone()) {
            Some(h) => Some(h.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_disks_index(&mut self, index: String, element: HashMap<String, DiskInfo>) {
        self.disks.insert(index.clone(), element.clone());
    }

    pub fn remove_disks_index(&mut self, index: String) {
        self.disks.remove(&index.clone());
    }

    pub fn get_disks_inner_index(&self, index1: String, index2: String) -> Option<DiskInfo> {
        match self.get_disks_index(index1.clone()) {
            Some(h) => match h.to_owned().get(&index2.clone()) {
                Some(d) => Some(d.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_disks_inner_index(&mut self, index1: String, index2: String, element: DiskInfo) {
        self.set_disks_index(
            index1.clone(),
            match self.get_disks_index(index1.clone()) {
                Some(h) => {
                    let mut new_h: HashMap<String, DiskInfo> = h.clone();
                    new_h.insert(index2.clone(), element.clone());
                    new_h
                }
                None => {
                    let mut new_h: HashMap<String, DiskInfo> = HashMap::<String, DiskInfo>::new();
                    new_h.insert(index2.clone(), element.clone());
                    new_h
                }
            },
        )
    }

    pub fn get_disk_hist(&self) -> HashMap<String, Vec<Bytes>> {
        self.disk_hist.clone()
    }

    pub fn set_disk_hist(&mut self, disk_hist: HashMap<String, Vec<Bytes>>) {
        self.disk_hist = disk_hist.clone()
    }

    pub fn get_disk_hist_index(&self, index: String) -> Option<Vec<Bytes>> {
        match self.get_disk_hist().get(&index.clone()) {
            Some(v) => Some(v.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_disk_hist_index(&mut self, index: String, element: Vec<Bytes>) {
        self.disk_hist.insert(index.clone(), element.clone());
    }

    pub fn get_disk_hist_inner_index(&self, index1: String, index2: usize) -> Option<Bytes> {
        match self.get_disk_hist_index(index1.clone()) {
            Some(v) => match v.get(index2.clone()) {
                Some(b) => Some(b.to_owned().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_disk_hist_inner_index(&mut self, index1: String, index2: usize, element: Bytes) {
        self.set_disk_hist_index(
            index1.clone(),
            match self.get_disk_hist_index(index1.clone()) {
                Some(v) => {
                    let mut new_v = v.clone();
                    new_v.insert(index2, element.clone());
                    new_v
                }
                None => {
                    let mut new_v: Vec<Bytes> = Vec::<Bytes>::new();
                    for i in 0..index2 {
                        new_v.push(0);
                    }
                    new_v.push(element.clone());
                    new_v
                }
            },
        )
    }

    pub fn get_timestamp(&self) -> SystemTime {
        self.timestamp.clone()
    }

    pub fn set_timestamp(&mut self, timestamp: SystemTime) {
        self.timestamp = timestamp.clone()
    }

    pub fn get_io_error(&self) -> bool {
        self.io_error.clone()
    }

    pub fn set_io_error(&mut self, io_error: bool) {
        self.io_error = io_error.clone()
    }

    pub fn get_old_disks(&self) -> Vec<String> {
        self.old_disks.clone()
    }

    pub fn set_old_disks(&mut self, old_disks: Vec<String>) {
        self.old_disks = old_disks.clone()
    }

    pub fn get_old_disks_index(&self, index: usize) -> Option<String> {
        match self.get_old_disks().get(index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_old_disks_index(&mut self, index: usize, element: String) {
        self.old_disks.insert(index, element.clone())
    }

    pub fn get_excludes(&self) -> Vec<FileSystem> {
        self.excludes.clone()
    }

    pub fn set_excludes(&mut self, excludes: Vec<FileSystem>) {
        self.excludes = excludes.clone()
    }

    pub fn get_excludes_index(&self, index: usize) -> Option<FileSystem> {
        match self.excludes.get(index) {
            Some(f) => Some(f.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_excludes_index(&mut self, index: usize, element: FileSystem) {
        self.excludes.insert(index, element.clone());
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer.clone()
    }
}

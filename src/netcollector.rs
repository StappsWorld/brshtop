use {
    crate::{
        collector::{Collector, Collectors},
        config::Config,
        draw::Draw,
        error::errlog,
        floating_humanizer,
        graph::Graphs,
        key::Key,
        menu::Menu,
        netbox::NetBox,
        term::Term,
        theme::Theme,
        units_to_bytes,
    },
    heim::net::{io_counters, nic, IoCounters, Nic},
    std::{
        collections::HashMap,
        fmt,
        time::{Duration, SystemTime},
    },
};

#[derive(Clone, Debug, PartialEq)]
pub enum NetCollectorStat {
    U64(u64),
    Vec(Vec<u64>),
    Bool(bool),
    I32(i32),
    String(String),
}
impl fmt::Display for NetCollectorStat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetCollectorStat::U64(u) => write!(f, "{}", u.to_owned()),
            NetCollectorStat::Vec(v) => write!(
                f,
                "{}",
                v.iter()
                    .map(|i| i.to_owned().to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            NetCollectorStat::Bool(b) => write!(f, "{}", b.to_owned()),
            NetCollectorStat::I32(i) => write!(f, "{}", i.to_owned()),
            NetCollectorStat::String(s) => write!(f, "{}", s.clone()),
        }
    }
}

#[derive(Clone)]
pub struct NetCollector<'a> {
    pub parent: Collector<'a>,
    pub buffer: String,
    pub nics: Vec<&'a Nic>,
    pub nic_i: i32,
    pub nic: Option<&'a Nic>,
    pub new_nic: Option<&'a Nic>,
    pub nic_error: bool,
    pub reset: bool,
    pub graph_raise: HashMap<String, i32>,
    pub graph_lower: HashMap<String, i32>,
    pub stats: HashMap<String, HashMap<String, HashMap<String, NetCollectorStat>>>,
    pub strings: HashMap<String, HashMap<String, HashMap<String, String>>>,
    pub switched: bool,
    pub timestamp: SystemTime,
    pub net_min: HashMap<String, i32>,
    pub auto_min: bool,
    pub sync_top: i32,
    pub sync_string: String,
}
impl<'a> NetCollector<'a> {
    pub fn new(netbox: &mut NetBox, CONFIG: &mut Config) -> Self {
        NetCollector {
            parent: Collector::new(),
            buffer: netbox.buffer.clone(),
            nics: Vec::<&'a Nic>::new(),
            nic_i: 0,
            nic: None,
            new_nic: None,
            nic_error: false,
            reset: false,
            graph_raise: [("download", 5), ("upload", 5)]
                .iter()
                .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                .collect::<HashMap<String, i32>>(),
            graph_lower: [("download", 5), ("upload", 5)]
                .iter()
                .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                .collect::<HashMap<String, i32>>(),
            stats: HashMap::<String, HashMap<String, HashMap<String, NetCollectorStat>>>::new(),
            strings: HashMap::<String, HashMap<String, HashMap<String, String>>>::new(),
            switched: false,
            timestamp: SystemTime::now(),
            net_min: [("download", -1), ("upload", -1)]
                .iter()
                .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                .collect::<HashMap<String, i32>>(),
            auto_min: CONFIG.net_auto,
            sync_top: 0,
            sync_string: String::default(),
        }
    }

    /// Get a list of all network devices sorted by highest throughput
    pub fn get_nics(&mut self) {
        self.nic_i = 0;
        self.nic = None;
        let io_all_stream = io_counters();
        let mut io_all: HashMap<String, IoCounters> = HashMap::<String, IoCounters>::new();
        let mut looping = true;
        while looping {
            match io_all_stream.poll_next() {
                Poll::Pending => (),
                Poll::Ready(o) => match o {
                    Some(res) => match res {
                        Ok(val) => {
                            io_all.insert(val.interface().to_owned(), val);
                            ()
                        }
                        Err(e) => {
                            if !self.nic_error {
                                self.nic_error = true;
                                errlog(format!("Nic error : {:?}", e));
                            }
                        }
                    },
                    None => looping = false,
                },
            }
        }

        if io_all.len() == 0 {
            return;
        }

        let up_stat_stream = nic();
        let mut up_stat: HashMap<String, Nic> = HashMap::<String, Nic>::new();
        looping = true;
        while looping {
            match up_stat_stream.poll_next() {
                Poll::Pending => (),
                Poll::Ready(o) => match o {
                    Some(res) => match res {
                        Ok(val) => {
                            up_stat.insert(val.name().to_owned(), val);
                            ()
                        }
                        Err(e) => {
                            if !self.nic_error {
                                self.nic_error = true;
                                errlog(format!("Nic error : {:?}", e));
                            }
                        }
                    },
                    None => looping = false,
                },
            }
        }

        for (nic, _) in io_all {
            match up_stat.get(&nic) {
                Some(n) => {
                    if n.is_up() {
                        self.nics.push(n.to_owned())
                    } else {
                        ()
                    }
                }
                None => (),
            };
        }
        if self.nics.len() == 0 {
            self.nics = vec![];
        }
        self.nic = Some(self.nics[self.nic_i as usize]);
    }

    pub fn switch(&'a mut self, key: String, collector: &'a mut Collector<'a>, CONFIG: &mut Config) {
        if self.nics.len() < 2 {
            return;
        }
        self.nic_i += if key == "n".to_owned() { 1 } else { -1 };
        if self.nic_i >= self.nics.len() as i32 {
            self.nic_i = 0;
        } else if self.nic_i < 0 {
            self.nic_i = self.nics.len() as i32 - 1;
        }
        self.new_nic = Some(self.nics[self.nic_i as usize]);
        self.switched = true;

        unsafe {
            collector.collect(
                vec![Collectors::<'a>::NetCollector(self)],
                CONFIG,
                true,
                false,
                false,
                true,
                false,
            );
        }
    }

    pub fn collect(&mut self, CONFIG: &mut Config, netbox: &mut NetBox) {
        let mut speed: i32 = 0;
        let mut stat: HashMap<String, NetCollectorStat> =
            HashMap::<String, NetCollectorStat>::new();
        let up_stat_stream = nic();
        let mut up_stat: HashMap<String, &'a Nic> = HashMap::<String, &'a Nic>::new();
        let mut looping = true;
        while looping {
            match up_stat_stream.poll_next() {
                Poll::Pending => (),
                Poll::Ready(o) => match o {
                    Some(res) => match res {
                        Ok(val) => {
                            up_stat.insert(val.name().to_owned(), val);
                            ()
                        }
                        Err(e) => {
                            if !self.nic_error {
                                self.nic_error = true;
                                errlog(format!("Nic error : {:?}", e));
                            }
                        }
                    },
                    None => looping = false,
                },
            }
        }

        if self.switched {
            self.nic = self.new_nic;
            self.switched = false;
        }

        if self.nic.is_none()
            || !up_stat.contains_key(&self.nic.unwrap().name().to_owned())
            || !up_stat
                .get(&self.nic.unwrap().name().to_owned())
                .unwrap()
                .is_up()
        {
            self.get_nics();
            if self.nic.is_none() {
                return;
            }
        }

        let io_all_stream = io_counters();
        let mut io_all_hash: HashMap<String, IoCounters> = HashMap::<String, IoCounters>::new();
        let mut looping = true;
        while looping {
            match io_all_stream.poll_next() {
                Poll::Pending => (),
                Poll::Ready(o) => match o {
                    Some(res) => match res {
                        Ok(val) => {
                            io_all_hash.insert(val.interface().to_owned(), val);
                            ()
                        }
                        Err(e) => {
                            if !self.nic_error {
                                self.nic_error = true;
                                errlog(format!("Nic error : {:?}", e));
                            }
                        }
                    },
                    None => looping = false,
                },
            }
        }

        let mut io_all: &IoCounters = match io_all_hash.get(&self.nic.unwrap().name().to_owned()) {
            Some(i) => i,
            None => return,
        };

        if !self
            .stats
            .contains_key(&self.nic.unwrap().name().to_owned())
        {
            self.stats.insert(
                self.nic.unwrap().name().to_owned(),
                HashMap::<String, HashMap<String, NetCollectorStat>>::new(),
            );
            self.strings.insert(
                self.nic.unwrap().name().to_owned(),
                vec![
                    ("download", HashMap::<String, String>::new()),
                    ("upload", HashMap::<String, String>::new()),
                ]
                .iter()
                .map(|(s, h)| (s.to_owned().to_owned(), h.clone()))
                .collect::<HashMap<String, HashMap<String, String>>>(),
            );
            for (direction, value) in vec![
                ("download", io_all.bytes_recv().value),
                ("upload", io_all.bytes_sent().value),
            ]
            .iter()
            .map(|(s, b)| (s.to_owned().to_owned(), b.clone()))
            .collect::<HashMap<String, u64>>()
            {
                self.stats
                    .get_mut(&self.nic.unwrap().name().to_owned())
                    .unwrap()
                    .insert(
                        direction,
                        vec![
                            ("total", NetCollectorStat::U64(value)),
                            ("last", NetCollectorStat::U64(value)),
                            ("top", NetCollectorStat::U64(0)),
                            ("graph_top", NetCollectorStat::U64(0)),
                            ("offset", NetCollectorStat::U64(0)),
                            ("speed", NetCollectorStat::Vec(Vec::<u64>::new())),
                            ("redraw", NetCollectorStat::Bool(true)),
                            ("graph_raise", NetCollectorStat::U64(0)),
                            ("graph_lower", NetCollectorStat::U64(7)),
                        ]
                        .iter()
                        .map(|(s, n)| (s.to_owned().to_owned(), n.clone()))
                        .collect::<HashMap<String, NetCollectorStat>>(),
                    );
                for v in vec!["total", "byte_ps", "bit_ps", "top", "graph_top"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                {
                    match self.strings.get_mut(&self.nic.unwrap().name().to_owned()) {
                        Some(h) => {
                            h.insert(v, HashMap::<String, String>::new());
                            ()
                        }
                        None => (),
                    }
                }
            }
        }

        match self.stats.get_mut(&self.nic.unwrap().name().to_owned()) {
            Some(h) => {
                match h.get_mut(&"download".to_owned()) {
                    Some(hash) => {
                        hash.insert(
                            "total".to_owned(),
                            NetCollectorStat::U64(io_all.bytes_recv().value),
                        );
                        ()
                    }
                    None => (),
                }
                match h.get_mut(&"upload".to_owned()) {
                    Some(hash) => {
                        hash.insert(
                            "total".to_owned(),
                            NetCollectorStat::U64(io_all.bytes_sent().value),
                        );
                        ()
                    }
                    None => (),
                }
                for direction in vec!["download", "upload"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                {
                    stat = h.get(&direction).unwrap().clone();
                    let mut strings: HashMap<String, NetCollectorStat> = self
                        .strings
                        .get(&self.nic.unwrap().name().to_owned())
                        .unwrap()
                        .get(&direction)
                        .unwrap()
                        .iter()
                        .map(|(s1, s2)| (s1.clone(), NetCollectorStat::String(s2.clone())))
                        .collect()
                        .clone();
                    // * Calculate current speed
                    let speed_vec = match stat.get(&"speed".to_owned()).unwrap() {
                        NetCollectorStat::Vec(v) => v.clone(),
                        _ => {
                            errlog("Malformed type in stat['speed']".to_owned());
                            vec![]
                        }
                    };
                    let total = match stat.get(&"total".to_owned()).unwrap() {
                        NetCollectorStat::U64(u) => u.to_owned(),
                        _ => {
                            errlog("Malformed type in stat['total']".to_owned());
                            0
                        }
                    };
                    let last = match stat.get(&"last".to_owned()).unwrap() {
                        NetCollectorStat::U64(u) => u.to_owned(),
                        _ => {
                            errlog("Malformed type in stat['last']".to_owned());
                            0
                        }
                    };
                    speed_vec.push(
                        (total - last)
                            / self
                                .timestamp
                                .elapsed()
                                .unwrap_or(Duration::from_secs(1))
                                .as_secs(),
                    );
                    last = total;
                    speed = speed_vec[speed_vec.len() - 2] as i32;

                    if self.net_min.get(&direction).unwrap_or(&0).to_owned() == -1 {
                        self.net_min.insert(
                            direction.clone(),
                            units_to_bytes(match direction.as_str() {
                                "download" => CONFIG.net_download,
                                "upload" => CONFIG.net_upload,
                            }) as i32,
                        );
                        stat.insert(
                            "graph_top".to_owned(),
                            NetCollectorStat::I32(self.net_min.get(&direction).unwrap().to_owned()),
                        );
                        stat.insert("graph_lower".to_owned(), NetCollectorStat::I32(7));
                        if !self.auto_min {
                            stat.insert("redraw".to_owned(), NetCollectorStat::Bool(true));
                            strings.insert(
                                "graph_top".to_owned(),
                                NetCollectorStat::String(floating_humanizer(
                                    match stat.get(&"graph_top".to_owned()).unwrap() {
                                        NetCollectorStat::I32(i) => i.to_owned() as f64,
                                        NetCollectorStat::U64(u) => u.to_owned() as f64,
                                        _ => {
                                            errlog(
                                                "Malformed type in strings['graph_top']".to_owned(),
                                            );
                                            0.0
                                        }
                                    },
                                    false,
                                    false,
                                    0,
                                    true,
                                )),
                            );
                        }
                    }
                    let stat_offset = match stat.get(&"offset".to_owned()) {
                        Some(n) => match n {
                            NetCollectorStat::I32(i) => i.to_owned() as u64,
                            NetCollectorStat::U64(u) => u.to_owned(),
                            _ => {
                                errlog("Malformed type in stat['offset']".to_owned());
                                0
                            }
                        },
                        None => {
                            errlog("Error getting stat['offset']".to_owned());
                            0
                        }
                    };
                    if stat_offset != 0 && stat_offset > total {
                        self.reset = true;
                    }

                    if self.reset {
                        if stat_offset == 0 {
                            stat_offset = total;
                        } else {
                            stat_offset = 0;
                        }
                        if direction == "upload".to_owned() {
                            self.reset = false;
                            netbox.redraw = true;
                        }
                    }

                    if speed_vec.len() as u32 > netbox.parent.width * 2 {
                        speed_vec.remove(0);
                    }

                    strings.insert(
                        "total".to_owned(),
                        NetCollectorStat::String(floating_humanizer(
                            (total - stat_offset) as f64,
                            false,
                            false,
                            0,
                            false,
                        )),
                    );
                    strings.insert(
                        "byte_ps".to_owned(),
                        NetCollectorStat::String(floating_humanizer(
                            speed_vec[speed_vec.len() - 2] as f64,
                            false,
                            true,
                            0,
                            false,
                        )),
                    );
                    strings.insert(
                        "bit_ps".to_owned(),
                        NetCollectorStat::String(floating_humanizer(
                            speed_vec[speed_vec.len() - 2] as f64,
                            true,
                            true,
                            0,
                            false,
                        )),
                    );

                    let top: i32 = match stat.get(&"top".to_owned()).unwrap() {
                        NetCollectorStat::I32(i) => i.to_owned(),
                        NetCollectorStat::U64(u) => u.to_owned() as i32,
                        _ => {
                            errlog("Malformed type in stat['top']".to_owned());
                            0
                        }
                    };

                    if speed > top || top == 0 {
                        top = speed;
                        strings.insert(
                            "top".to_owned(),
                            NetCollectorStat::String(floating_humanizer(
                                top as f64, true, true, 0, false,
                            )),
                        );
                    }

                    if self.auto_min {
                        let graph_top: i32 = match stat.get(&"graph_top".to_owned()).unwrap() {
                            NetCollectorStat::I32(i) => i.to_owned(),
                            NetCollectorStat::U64(u) => u.to_owned() as i32,
                            _ => {
                                errlog("Malformed type in stat['graph_top']".to_owned());
                                0
                            }
                        };
                        let graph_raise: i32 = match stat.get(&"graph_raise".to_owned()).unwrap() {
                            NetCollectorStat::I32(i) => i.to_owned(),
                            NetCollectorStat::U64(u) => u.to_owned() as i32,
                            _ => {
                                errlog("Malformed type in stat['graph_raise']".to_owned());
                                0
                            }
                        };
                        let graph_lower: i32 = match stat.get(&"graph_lower".to_owned()).unwrap() {
                            NetCollectorStat::I32(i) => i.to_owned(),
                            NetCollectorStat::U64(u) => u.to_owned() as i32,
                            _ => {
                                errlog("Malformed type in stat['graph_lower']".to_owned());
                                0
                            }
                        };

                        if speed > graph_top {
                            graph_raise += 1;
                            if graph_lower > 0 {
                                graph_lower -= 1;
                            }
                        } else if speed < graph_top / 10 {
                            graph_lower += 1;
                            if graph_raise > 0 {
                                graph_raise -= 1;
                            }
                        }

                        if graph_raise >= 5 || graph_lower >= 5 {
                            let max: u64 = speed_vec[speed_vec.len() - 6..]
                                .iter()
                                .max()
                                .unwrap()
                                .to_owned();
                            if graph_raise >= 5 {
                                graph_top = (max as f32 / 0.8) as i32;
                            } else if graph_lower >= 5 {
                                graph_top = if (10 << 10) > max * 3 {
                                    10 << 10
                                } else {
                                    max as i32 * 3
                                };
                            }
                            graph_raise = 0;
                            graph_lower = 0;
                            stat.insert("redraw".to_owned(), NetCollectorStat::Bool(true));
                            strings.insert(
                                "graph_top".to_owned(),
                                NetCollectorStat::String(floating_humanizer(
                                    graph_top as f64,
                                    false,
                                    false,
                                    0,
                                    true,
                                )),
                            );
                        }
                        stat.insert("graph_top".to_owned(), NetCollectorStat::I32(graph_top));
                        stat.insert("graph_raise".to_owned(), NetCollectorStat::I32(graph_raise));
                        stat.insert("graph_lower".to_owned(), NetCollectorStat::I32(graph_lower));
                    }

                    stat.insert("top".to_owned(), NetCollectorStat::I32(top));
                    stat.insert("offset".to_owned(), NetCollectorStat::U64(stat_offset));
                    stat.insert("last".to_owned(), NetCollectorStat::U64(last));
                    stat.insert("total".to_owned(), NetCollectorStat::U64(total));
                    stat.insert("speed".to_owned(), NetCollectorStat::Vec(speed_vec));

                    self.strings
                        .get_mut(&self.nic.unwrap().name().to_owned())
                        .unwrap()
                        .insert(
                            direction,
                            strings
                                .clone()
                                .iter()
                                .map(|(s1, s2)| {
                                    (
                                        s1.clone(),
                                        match s2 {
                                            NetCollectorStat::String(s) => s.clone(),
                                            _ => "".to_owned(),
                                        },
                                    )
                                })
                                .collect(),
                        );
                    h.insert(direction, stat.clone());
                }

                self.timestamp = SystemTime::now();

                if CONFIG.net_sync {
                    let download_top = self
                        .stats
                        .get(&self.nic.unwrap().name().to_owned())
                        .unwrap()
                        .get(&"download".to_owned())
                        .unwrap()
                        .get(&"graph_top".to_owned())
                        .unwrap();
                    let upload_top = self
                        .stats
                        .get(&self.nic.unwrap().name().to_owned())
                        .unwrap()
                        .get(&"upload".to_owned())
                        .unwrap()
                        .get(&"graph_top".to_owned())
                        .unwrap();

                    let dtu : i32 = match download_top {
                        NetCollectorStat::I32(i) => i.to_owned(),
                        NetCollectorStat::U64(u) => u.to_owned() as i32,
                        NetCollectorStat::String(s) => s.to_owned().parse::<i32>().unwrap_or(0),
                        _ => 0,
                    };
                    let dut : i32 = match upload_top {
                        NetCollectorStat::I32(i) => i.to_owned(),
                        NetCollectorStat::U64(u) => u.to_owned() as i32,
                        NetCollectorStat::String(s) => s.to_owned().parse::<i32>().unwrap_or(0),
                        _ => 0,
                    };

                    let c_max: i32 = if dtu > dut {
                        dtu
                    } else {
                        dut
                    };

                    if c_max != self.sync_top {
                        self.sync_top = c_max;
                        self.sync_string =
                            floating_humanizer(self.sync_top as f64, false, false, 0, false);
                        netbox.redraw = true;
                    }
                }
            }
            None => errlog(format!(
                "Unable to access nic in self.stats (nic : {})",
                self.nic.unwrap().name()
            )),
        }
    }

    /// JUST CALL NETBOX.draw_fg()
    pub fn draw(
        &mut self,
        netbox: &mut NetBox,
        theme: &mut Theme,
        key: &mut Key,
        term: &mut Term,
        CONFIG: &mut Config,
        draw: &mut Draw,
        graphs: &mut Graphs,
        menu: &mut Menu,
    ) {
        netbox.draw_fg(theme, key, term, CONFIG, draw, graphs, menu)
    }
}

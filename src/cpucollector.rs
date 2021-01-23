use {
    crate::{
        brshtop_box::BrshtopBox,
        collector::{Collector, Collectors},
        config::{Config, ViewMode},
        cpubox::CpuBox,
        draw::Draw,
        error,
        graph::Graphs,
        key::Key,
        menu::Menu,
        meter::Meters,
        term::Term,
        theme::Theme,
        CONFIG_DIR, CORES, CORE_MAP, SYSTEM,
        THREADS,
    },
    hhmmss::Hhmmss,
    psutil::sensors::*,
    std::time::SystemTime,
    std::{collections::HashMap, iter::Enumerate, path::*},
    subprocess::Exec,
    sys_info::*,
    which::which,
};

#[derive(Clone)]
pub struct CpuCollector<'a> {
    pub parent: Collector<'a>,
    pub cpu_usage: Vec<Vec<u32>>,
    pub cpu_temp: Vec<Vec<u32>>,
    pub cpu_temp_high: i32,
    pub cpu_temp_crit: i32,
    pub freq_error: bool,
    pub cpu_freq: f64,
    pub load_avg: Vec<f64>,
    pub uptime: String,
    pub buffer: String,
    pub sensor_method: String,
    pub got_sensors: bool,
    pub sensor_swap: bool,
    pub cpu_temp_only: bool,
}
impl<'a> CpuCollector<'a> {
    pub fn new() -> Self {
        let mut cpu_usage_mut = Vec::<Vec<u32>>::new();
        let mut cpu_temp_mut = Vec::<Vec<u32>>::new();
        for _ in 0..THREADS.to_owned() + 1 {
            cpu_usage_mut.push(Vec::new());
            cpu_temp_mut.push(Vec::new());
        }

        let mut CpuCollector_initialize = CpuCollector {
            parent: Collector::new(),
            cpu_usage: cpu_usage_mut,
            cpu_temp: cpu_temp_mut,
            cpu_temp_high: 0,
            cpu_temp_crit: 0,
            freq_error: false,
            cpu_freq: 0.0,
            load_avg: Vec::<f64>::new(),
            uptime: String::from(""),
            buffer: String::from(""),
            sensor_method: String::from(""),
            got_sensors: false,
            sensor_swap: false,
            cpu_temp_only: false,
        };

        CpuCollector_initialize
    }
    pub fn collect(
        &mut self,
        CONFIG: &mut Config,
        term: &mut Term,
        cpu_box: &mut CpuBox,
        brshtop_box: &mut BrshtopBox,
    ) {
        match psutil::cpu::CpuPercentCollector::new()
            .unwrap()
            .cpu_percent()
        {
            Ok(p) => self.cpu_usage[0].push(format!("{:.2}", p).parse::<u32>().unwrap()),
            Err(_) => (),
        }

        if self.cpu_usage[0].len() > (term.width * 4) as usize {
            self.cpu_usage[0].remove(0);
        }

        let cpu_percentages = match psutil::cpu::CpuPercentCollector::new() {
            Ok(p) => match p.cpu_percent_percpu() {
                Ok(p) => p,
                Err(e) => {
                    error::errlog(format!("Unable to collect CPU percentages! (error {})", e));
                    self.got_sensors = false;
                    return;
                }
            },
            Err(e) => {
                error::errlog(format!("Unable to collect CPU percentages! (error {})", e));
                vec![-1.0]
            }
        };

        for (n, thread) in cpu_percentages.iter().enumerate() {
            self.cpu_usage[n].push(format!("{:.2}", *thread as u32).parse::<u32>().unwrap());
            if self.cpu_usage[n].len() > (term.width * 2) as usize {
                self.cpu_usage[n].remove(0);
            }
        }

        let cpu_frequency = match psutil::cpu::cpu_freq() {
            Ok(f) => f.current(),
            Err(e) => {
                error::errlog(format!("Unable to collect CPU frequency! (error {})", e));
                -1.0
            }
        };

        self.cpu_freq = cpu_frequency;

        let lavg: Vec<f64> = match sys_info::loadavg() {
            Ok(l) => vec![
                format!("{:.2}", l.one).parse::<f64>().unwrap(),
                format!("{:.2}", l.five).parse::<f64>().unwrap(),
                format!("{:.2}", l.fifteen).parse::<f64>().unwrap(),
            ],
            Err(e) => {
                error::errlog(format!("Unable to collect load average! (error {})", e));
                vec![-1.0, -1.0, -1.0]
            }
        };

        self.load_avg = lavg;

        let now = SystemTime::now();
        self.uptime = match psutil::host::boot_time() {
            Ok(t) => {
                let mut ela = t.elapsed().unwrap().hhmmss();
                ela.pop();
                ela.pop();
                ela.pop();
                ela
            }
            Err(e) => {
                error::errlog(format!(
                    "Error finding the boot time of this system... (error {})",
                    e
                ));
                String::from("99:99:99")
            }
        };

        if CONFIG.check_temp && self.got_sensors {
            self.collect_temps(CONFIG, cpu_box, brshtop_box, term);
        }
    }

    pub fn draw(
        &mut self,
        cpu_box: &mut CpuBox,
        CONFIG: &mut Config,
        key: &mut Key,
        THEME: &mut Theme,
        term: &mut Term,
        draw: &mut Draw,
        ARG_MODE: ViewMode,
        graphs: &mut Graphs,
        meters: &mut Meters,
        menu: &mut Menu,
    ) {
        cpu_box.draw_fg(
            self, CONFIG, key, THEME, term, draw, ARG_MODE, graphs, meters, menu, THEME,
        );
    }

    pub fn get_sensors(&mut self, CONFIG: &mut Config) {
        self.sensor_method = String::from("");

        if SYSTEM.to_owned() == "MacOS".to_owned() {
            match which("coretemp") {
                Ok(_) => {
                    let output = Exec::shell("coretemp -p")
                        .capture()
                        .unwrap()
                        .stdout_str()
                        .to_owned();
                    match output.trim().replace("-", "").parse::<f64>() {
                        Ok(n) => self.sensor_method = String::from("coretemp"),
                        Err(_) => match which("osx-cpu-temp") {
                            Ok(_) => {
                                let output = Exec::shell("osx-cpu-temp")
                                    .capture()
                                    .unwrap()
                                    .stdout_str()
                                    .to_owned();
                                if output.trim_end().ends_with("°C") {
                                    self.sensor_method = String::from("osx-cpu-temp");
                                }
                            }
                            Err(_) => (),
                        },
                    }
                }
                Err(_) => (),
            }
        } else if CONFIG.cpu_sensor != "Auto" && CONFIG.cpu_sensors.contains(&CONFIG.cpu_sensor) {
            self.sensor_method = String::from("psutil");
        } else {
            for res in temperatures() {
                match res {
                    Ok(temp) => {
                        if temp.unit().to_lowercase().starts_with("cpu") {
                            self.sensor_method = String::from("psutil");
                            break;
                        }
                        match temp.label() {
                            Some(label) => {
                                let arr = vec!["Package", "Core 0", "Tdie", "CPU"];

                                for test in arr {
                                    if label.starts_with(test) {
                                        self.sensor_method = String::from("psutil");
                                        break;
                                    }
                                }
                            }
                            None => (),
                        };
                    }
                    Err(e) => (),
                };
            }
        }

        if self.sensor_method == "" && SYSTEM.to_owned() == "Linux".to_owned() {
            let output: Option<String> = match which("vcgencmd") {
                Ok(s) => Some(
                    Exec::shell("vcgencmd measure_temp")
                        .capture()
                        .unwrap()
                        .stdout_str()
                        .to_owned(),
                ),
                Err(e) => None,
            };

            match output {
                Some(s) => {
                    if s.trim().to_owned().ends_with("'C") {
                        self.sensor_method = String::from("vcgencmd");
                    }
                }
                None => (),
            };

            self.got_sensors = self.sensor_method.chars().count() > 0;
        }
    }

    pub fn collect_temps(
        &mut self,
        CONFIG: &mut Config,
        cpu_box: &mut CpuBox,
        brshtop_box: &mut BrshtopBox,
        term: &mut Term,
    ) {
        let mut temp: i32 = 1000;
        let mut cores: Vec<String> = Vec::<String>::new();
        let mut core_dict: HashMap<i32, i32> = HashMap::<i32, i32>::new();
        let mut entry_int: i32 = 0;
        let mut cpu_type: String = String::from("");
        let mut c_max: i32 = 0;
        let mut s_name: String = String::from("_-_");
        let mut s_label: String = String::from("_-_");

        if self.sensor_method == "psutil" {
            if CONFIG.cpu_sensor != "Auto" {
                let mut splitter = CONFIG.cpu_sensor.splitn(2, ":");
                s_name = String::from(splitter.next().unwrap());
                s_label = String::from(splitter.next().unwrap());
            }

            let mut num = 1;
            for res in psutil::sensors::temperatures() {
                match res {
                    Ok(s) => {
                        let mut sensor = s.clone();
                        let mut name = sensor.unit();

                        let label = sensor.label().unwrap();

                        if name == s_name
                            && (sensor.label().unwrap_or("error_in_label") == s_label
                                || num.to_string() == s_label.to_owned())
                            && sensor.current().celsius() > 0.0
                        {
                            if label.starts_with("Package") {
                                cpu_type = String::from("intel");
                            } else if label.starts_with("Tdie") {
                                cpu_type = String::from("ryzen");
                            } else {
                                cpu_type = String::from("other");
                            }

                            // TODO : Allow for fahrenheit and celsius
                            match sensor.high() {
                                Some(t) => {
                                    if t.celsius() > 1.0 {
                                        self.cpu_temp_high = t.celsius().round() as i32;
                                    } else {
                                        self.cpu_temp_high = 80;
                                    }
                                }
                                None => self.cpu_temp_high = 80,
                            }

                            match sensor.critical() {
                                Some(t) => {
                                    if t.celsius() > 1.0 {
                                        self.cpu_temp_crit = t.celsius().round() as i32;
                                    } else {
                                        self.cpu_temp_crit = 95;
                                    }
                                }
                                None => self.cpu_temp_crit = 95,
                            }
                        } else if (label.starts_with("Package") || label.starts_with("Tdie"))
                            && vec!["", "other"].iter().any(|&s| s.to_owned() == cpu_type)
                            && s_name == "_-_"
                            && sensor.current().celsius().round() > 0.0
                        {
                            if self.cpu_temp_high == 0 || self.sensor_swap || cpu_type == "other" {
                                self.sensor_swap = false;
                                match sensor.high() {
                                    Some(t) => {
                                        if t.celsius() > 1.0 {
                                            self.cpu_temp_high = t.celsius().round() as i32;
                                        } else {
                                            self.cpu_temp_high = 80
                                        }
                                    }
                                    None => self.cpu_temp_high = 80,
                                }

                                match sensor.critical() {
                                    Some(t) => {
                                        if t.celsius() > 1.0 {
                                            self.cpu_temp_crit = t.celsius().round() as i32;
                                        } else {
                                            self.cpu_temp_crit = 95;
                                        }
                                    }
                                    None => self.cpu_temp_crit = 95,
                                }

                                if label.starts_with("Package") {
                                    cpu_type = String::from("intel");
                                } else {
                                    cpu_type = String::from("ryzen");
                                }
                            }
                        } else if (label.starts_with("Core")
                            || label.starts_with("Tccd")
                            || label.starts_with("CPU")
                            || name.to_owned().to_lowercase().starts_with("cpu"))
                            && sensor.current().celsius() > 0.0
                        {
                            if label.starts_with("Core") || label.starts_with("Tccd") {
                                entry_int = label
                                    .replace("Core", "")
                                    .replace("Tccd", "")
                                    .parse::<i32>()
                                    .unwrap();

                                if core_dict.contains_key(&entry_int) && cpu_type != "ryzen" {
                                    if c_max == 0 {
                                        let mut largest = 0;
                                        for (key, val) in core_dict {
                                            if key > largest {
                                                largest = key.clone();
                                            }
                                        }
                                        c_max = largest + 1;
                                    }
                                    if c_max < (THREADS.to_owned() / 2) as i32
                                        && !core_dict.contains_key(&(entry_int + c_max))
                                    {
                                        core_dict.insert(
                                            entry_int + c_max,
                                            sensor.current().celsius().round() as i32,
                                        );
                                    }
                                    continue;
                                } else if core_dict.contains_key(&entry_int) {
                                    continue;
                                }
                                core_dict
                                    .insert(entry_int, sensor.current().celsius().round() as i32);
                                continue;
                            } else if vec!["intel", "ryzen"].contains(&(cpu_type.as_str())) {
                                continue;
                            }

                            if cpu_type == "" {
                                cpu_type = String::from("other");
                                if self.cpu_temp_high == 0 || self.sensor_swap {
                                    self.sensor_swap = false;

                                    match sensor.high() {
                                        Some(t) => {
                                            if t.celsius() > 1.0 {
                                                self.cpu_temp_high = t.celsius().round() as i32;
                                            } else {
                                                self.cpu_temp_high = match name {
                                                    "cpu_thermal" => 60,
                                                    _ => 80,
                                                };
                                            }
                                        }
                                        None => {
                                            self.cpu_temp_high = match name {
                                                "cpu_thermal" => 60,
                                                _ => 80,
                                            }
                                        }
                                    }

                                    match sensor.critical() {
                                        Some(t) => {
                                            if t.celsius() > 1.0 {
                                                self.cpu_temp_crit = t.celsius().round() as i32;
                                            } else {
                                                self.cpu_temp_crit = match name {
                                                    "cpu_thermal" => 80,
                                                    _ => 95,
                                                };
                                            }
                                        }
                                        None => {
                                            self.cpu_temp_crit = match name {
                                                "cpu_thermal" => 80,
                                                _ => 95,
                                            }
                                        }
                                    }
                                }
                                temp = sensor.current().celsius().round() as i32;
                            }
                            cores.push(sensor.current().celsius().round().to_string());
                        }
                    }
                    Err(e) => (),
                }
                num += 1;
            }

            if core_dict.len() > 0 {
                if temp == 1000 {
                    temp = core_dict.values().into_iter().sum::<i32>() / core_dict.len() as i32;
                }
                if self.cpu_temp_high == 0 || self.cpu_temp_crit == 0 {
                    self.cpu_temp_high = 80;
                    self.cpu_temp_crit = 95;
                }
                self.cpu_temp.get(0).unwrap().push(temp as u32);
                if cpu_type == String::from("ryzen") {
                    let ccds: i32 = core_dict.len() as i32;
                    let cores_per_ccd: i32 = (CORES.to_owned() / ccds as u64) as i32;
                    let mut z: i32 = 1;

                    for x in 0..THREADS.to_owned() as usize {
                        if x == CORES.to_owned() as usize {
                            z = 1;
                        }
                        if CORE_MAP.to_owned()[x] + 1 > cores_per_ccd * z {
                            z += 1;
                        }
                        if core_dict.contains_key(&z) {
                            self.cpu_temp[x + 1].push(core_dict[&CORE_MAP.to_owned()[x]] as u32);
                        }
                    }
                } else {
                    for x in 0..THREADS.to_owned() as usize {
                        if core_dict.contains_key(&CORE_MAP.to_owned()[x]) {
                            self.cpu_temp[x + 1].push(core_dict[&CORE_MAP.to_owned()[x]] as u32);
                        }
                    }
                }
            } else if cores.len() == (THREADS.to_owned() / 2) as usize {
                self.cpu_temp[0].push(temp as u32);
                let mut n = 1;
                for t in cores {
                    match self.cpu_temp.get(n) {
                        Some(u) => u.push(t.parse::<u32>().unwrap()),
                        None => break,
                    }
                    match self.cpu_temp.get((THREADS.to_owned() / 2) as usize + n) {
                        Some(u) => u.push(t.parse::<u32>().unwrap()),
                        None => break,
                    }

                    n += 1;
                }
            } else {
                self.cpu_temp[0].push(temp as u32);
                if cores.len() > 1 {
                    let mut n = 1;
                    for t in cores {
                        match self.cpu_temp.get(n) {
                            Some(u) => u.push(t.parse::<u32>().unwrap()),
                            None => break,
                        }
                        n += 1;
                    }
                }
            }
        } else {
            match self.sensor_method.as_str() {
                "coretemp" => {
                    let coretemp_p = match Exec::shell("coretemp -p").capture() {
                        Ok(o) => o.stdout_str().to_owned().trim().parse::<u32>().unwrap(),
                        Err(e) => {
                            error::errlog(format!(
                                "Error getting temperature data for this system... (error {})",
                                e
                            ));
                            self.got_sensors = false;
                            cpu_box.calc_size(term, brshtop_box, self);
                            return;
                        }
                    };

                    temp = if coretemp_p > 0 { coretemp_p as i32 } else { 0 };

                    let coretemp: Vec<u32> = match Exec::shell("coretemp").capture() {
                        Ok(o) => o
                            .stdout_str()
                            .to_owned()
                            .trim()
                            .split(" ")
                            .map(|s: &str| s.parse::<u32>().unwrap_or(0))
                            .collect::<Vec<u32>>(),
                        Err(e) => {
                            error::errlog(format!(
                                "Error getting temperature data for this system... (error {})",
                                e
                            ));
                            self.got_sensors = false;
                            cpu_box.calc_size(term, brshtop_box, self);
                            return;
                        }
                    };

                    cores = coretemp.iter().map(|u| u.to_string()).collect();

                    if cores.len() == (THREADS.to_owned() as usize / 2) {
                        self.cpu_temp[0].push(temp as u32);

                        let mut n = 1;
                        for t in cores {
                            match self.cpu_temp.get(n) {
                                Some(u) => u.push(t.parse::<u32>().unwrap()),
                                None => break,
                            }
                            match self.cpu_temp.get((THREADS.to_owned() / 2) as usize + n) {
                                Some(u) => u.push(t.parse::<u32>().unwrap()),
                                None => break,
                            }

                            n += 1;
                        }
                    } else {
                        cores.insert(0, temp.to_string());

                        for (n, t) in cores.iter().enumerate() {
                            match self.cpu_temp.get(n) {
                                Some(u) => u.push(t.parse::<u32>().unwrap()),
                                None => break,
                            }
                        }
                    }

                    if self.cpu_temp_high == 0 {
                        self.cpu_temp_high = 85;
                        self.cpu_temp_crit = 100;
                    }
                }
                "osx-cpu-temp" => {
                    let cpu_temp = match Exec::shell("osx-cpu-temp").capture() {
                        Ok(o) => {
                            let mut setter = o.stdout_str().to_owned().trim().to_owned();
                            setter.pop();
                            setter.pop();
                            setter.parse::<f64>().unwrap().round() as u32
                        }
                        Err(e) => {
                            error::errlog(format!(
                                "Error getting temperature data for this system... (error {})",
                                e
                            ));
                            self.got_sensors = false;
                            cpu_box.calc_size(term, brshtop_box ,self);
                            return;
                        }
                    };

                    temp = if cpu_temp > 0 { cpu_temp as i32 } else { 0 };

                    if self.cpu_temp_high == 0 {
                        self.cpu_temp_high = 85;
                        self.cpu_temp_crit = 100;
                    }
                }
                "vcgencmd" => {
                    let vcgencmd = match Exec::shell("vcgencmd measure_temp").capture() {
                        Ok(o) => {
                            let mut setter = o.stdout_str().to_owned().trim()[5..].to_owned();
                            setter.pop();
                            setter.pop();
                            setter.parse::<f64>().unwrap().round() as u32
                        }
                        Err(e) => {
                            error::errlog(format!(
                                "Error getting temperature data for this system... (error {})",
                                e
                            ));
                            self.got_sensors = false;
                            cpu_box.calc_size(term, brshtop_box ,self);
                            return;
                        }
                    };

                    temp = if vcgencmd > 0 { vcgencmd as i32 } else { 0 };

                    if self.cpu_temp_high == 0 {
                        self.cpu_temp_high = 60;
                        self.cpu_temp_crit = 80;
                    }
                }
            }

            if cores.len() == 0 {
                self.cpu_temp[0].push(temp as u32);
            }
        }

        if core_dict.len() == 0 && cores.len() <= 1 {
            self.cpu_temp_only = true;
        }
        if self.cpu_temp[0].len() > 5 {
            for n in 0..self.cpu_temp.len() {
                if self.cpu_temp[n].len() == 0 {
                    self.cpu_temp.remove(n);
                }
            }
        }
    }
}

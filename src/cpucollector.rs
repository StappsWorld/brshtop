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
        CONFIG_DIR, CORES, CORE_MAP, SYSTEM, THREADS,
    },
    hhmmss::Hhmmss,
    once_cell::sync::OnceCell,
    psutil::sensors::*,
    std::time::SystemTime,
    std::{collections::HashMap, iter::Enumerate, path::*, sync::Mutex},
    subprocess::Exec,
    sys_info::*,
    which::which,
};

#[derive(Clone)]
pub struct CpuCollector {
    parent: Collector,
    cpu_usage: Vec<Vec<u32>>,
    cpu_temp: Vec<Vec<u32>>,
    cpu_temp_high: i32,
    cpu_temp_crit: i32,
    freq_error: bool,
    cpu_freq: f64,
    load_avg: Vec<f64>,
    uptime: String,
    buffer: String,
    sensor_method: String,
    got_sensors: bool,
    sensor_swap: bool,
    cpu_temp_only: bool,
}
impl CpuCollector {
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
        CONFIG: &OnceCell<Mutex<Config>>,
        term: &OnceCell<Mutex<Term>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
    ) {
        match psutil::cpu::CpuPercentCollector::new()
            .unwrap()
            .cpu_percent()
        {
            Ok(p) => self.cpu_usage[0].push(format!("{:.2}", p).parse::<u32>().unwrap()),
            Err(_) => (),
        }

        if self.cpu_usage[0].len() > (term.get().unwrap().try_lock().unwrap().get_width() * 4) as usize
        {
            self.cpu_usage[0].remove(0);
        }

        let mut cpu_percent_collector = psutil::cpu::CpuPercentCollector::new().unwrap();
        let cpu_percentages = match cpu_percent_collector.cpu_percent_percpu() {
            Ok(p) => p,
            Err(e) => {
                error::errlog(format!("Unable to collect CPU percentages! (error {})", e));
                self.got_sensors = false;
                return;
            }
        };

        for (n, thread) in cpu_percentages.iter().enumerate() {
            self.cpu_usage[n].push(format!("{:.2}", *thread as u32).parse::<u32>().unwrap());
            if self.cpu_usage[n].len()
                > (term.get().unwrap().try_lock().unwrap().get_width() * 2) as usize
            {
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

        if CONFIG.get().unwrap().try_lock().unwrap().check_temp && self.got_sensors {
            self.collect_temps(CONFIG, cpu_box, brshtop_box, term);
        }
    }

    pub fn draw(
        &mut self,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        CONFIG: &OnceCell<Mutex<Config>>,
        key: &OnceCell<Mutex<Key>>,
        THEME: &OnceCell<Mutex<Theme>>,
        term: &OnceCell<Mutex<Term>>,
        draw: &OnceCell<Mutex<Draw>>,
        ARG_MODE: ViewMode,
        graphs: &OnceCell<Mutex<Graphs>>,
        meters: &OnceCell<Mutex<Meters>>,
        menu: &OnceCell<Mutex<Menu>>,
    ) {
        cpu_box.get().unwrap().try_lock().unwrap().draw_fg(
            self,
            CONFIG,
            key,
            THEME,
            term,
            draw,
            ARG_MODE,
            graphs,
            meters,
            menu,
            THEME,
        );
    }

    pub fn get_sensors(&mut self, CONFIG_p: &OnceCell<Mutex<Config>>) {
        let mut CONFIG = CONFIG_p.get().unwrap().try_lock().unwrap();
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
        } else if CONFIG.cpu_sensor != "Auto"
            && CONFIG.cpu_sensors.contains(&CONFIG.cpu_sensor.clone())
        {
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
        CONFIG: &OnceCell<Mutex<Config>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
        term: &OnceCell<Mutex<Term>>,
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
            if CONFIG.get().unwrap().try_lock().unwrap().cpu_sensor != "Auto" {
                let cpu_sensor_string = CONFIG.get().unwrap().try_lock().unwrap().cpu_sensor.clone();
                let mut splitter = cpu_sensor_string.splitn(2, ":");
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

                                if core_dict.contains_key(&entry_int.clone())
                                    && cpu_type != "ryzen".to_owned()
                                {
                                    if c_max == 0 {
                                        let mut largest = 0;
                                        for (key, _) in core_dict.clone() {
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
                self.cpu_temp.get_mut(0).unwrap().push(temp as u32);
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
                for t in cores.clone() {
                    match self.cpu_temp.get_mut(n) {
                        Some(u) => u.push(t.parse::<u32>().unwrap()),
                        None => break,
                    }
                    match self.cpu_temp.get_mut((THREADS.to_owned() / 2) as usize + n) {
                        Some(u) => u.push(t.parse::<u32>().unwrap()),
                        None => break,
                    }

                    n += 1;
                }
            } else {
                self.cpu_temp[0].push(temp as u32);
                if cores.len() > 1 {
                    let mut n = 1;
                    for t in cores.clone() {
                        match self.cpu_temp.get_mut(n) {
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
                            brshtop_box.get().unwrap().try_lock().unwrap().set_b_cpu_h(
                                cpu_box.get().unwrap().try_lock().unwrap().calc_size(
                                    term,
                                    brshtop_box.get().unwrap().try_lock().unwrap().get_b_cpu_h(),
                                    self,
                                ),
                            );
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
                            brshtop_box.get().unwrap().try_lock().unwrap().set_b_cpu_h(
                                cpu_box.get().unwrap().try_lock().unwrap().calc_size(
                                    term,
                                    brshtop_box.get().unwrap().try_lock().unwrap().get_b_cpu_h(),
                                    self,
                                ),
                            );
                            return;
                        }
                    };

                    cores = coretemp.iter().map(|u| u.to_string()).collect();

                    if cores.len() == (THREADS.to_owned() as usize / 2) {
                        self.cpu_temp[0].push(temp as u32);

                        let mut n = 1;
                        for t in cores.clone() {
                            match self.cpu_temp.get_mut(n) {
                                Some(u) => u.push(t.parse::<u32>().unwrap()),
                                None => break,
                            }
                            match self.cpu_temp.get_mut((THREADS.to_owned() / 2) as usize + n) {
                                Some(u) => u.push(t.parse::<u32>().unwrap()),
                                None => break,
                            }

                            n += 1;
                        }
                    } else {
                        cores.insert(0, temp.to_string());

                        for (n, t) in cores.iter().enumerate() {
                            match self.cpu_temp.get_mut(n) {
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
                            brshtop_box.get().unwrap().try_lock().unwrap().set_b_cpu_h(
                                cpu_box.get().unwrap().try_lock().unwrap().calc_size(
                                    term,
                                    brshtop_box.get().unwrap().try_lock().unwrap().get_b_cpu_h(),
                                    self,
                                ),
                            );
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
                            brshtop_box.get().unwrap().try_lock().unwrap().set_b_cpu_h(
                                cpu_box.get().unwrap().try_lock().unwrap().calc_size(
                                    term,
                                    brshtop_box.get().unwrap().try_lock().unwrap().get_b_cpu_h(),
                                    self,
                                ),
                            );
                            return;
                        }
                    };

                    temp = if vcgencmd > 0 { vcgencmd as i32 } else { 0 };

                    if self.cpu_temp_high == 0 {
                        self.cpu_temp_high = 60;
                        self.cpu_temp_crit = 80;
                    }
                }
                _ => error::errlog(format!(
                    "Invalid sensor_method {} found in CpuCollector",
                    self.sensor_method.clone()
                )),
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

    pub fn get_parent(&self) -> Collector {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent: Collector) {
        self.parent = parent.clone()
    }

    pub fn get_cpu_usage(&self) -> Vec<Vec<u32>> {
        self.cpu_usage.clone()
    }

    pub fn set_cpu_usage(&mut self, cpu_usage: Vec<Vec<u32>>) {
        self.cpu_usage = cpu_usage.clone()
    }

    pub fn get_cpu_usage_index(&self, index: usize) -> Option<Vec<u32>> {
        match self.get_cpu_usage().get(index) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }

    /// Sets element at index to the argument provided, and if this index is larger than the size of the Vec, pushes it on the Vec
    pub fn set_cpu_usage_index(&mut self, index: usize, element: Vec<u32>) {
        if index < self.cpu_usage.len() {
            let mut setter: Vec<Vec<u32>> = vec![];
            for i in 0..self.cpu_usage.len() {
                if i == index {
                    setter.push(element.clone());
                } else {
                    setter.push(self.get_cpu_usage_index(index.clone()).unwrap())
                }
            }
            self.cpu_usage = setter.clone();
        } else {
            self.cpu_usage.push(element.clone());
        }
    }

    pub fn get_cpu_usage_inner_index(&self, index1: usize, index2: usize) -> Option<u32> {
        match self.get_cpu_usage_index(index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(u) => Some(u.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_cpu_usage_inner_index(&mut self, index1: usize, index2: usize, element: u32) {
        self.set_cpu_usage_index(
            index1.clone(),
            match self.get_cpu_usage_index(index1.clone()) {
                Some(v) => {
                    let mut returnable: Vec<u32> = v.clone();
                    if index2 > v.len() {
                        returnable = v.clone();
                    } else {
                        returnable = v.clone();
                        returnable.push(element);
                    }
                    returnable.clone()
                }
                None => vec![element.clone()],
            },
        );
    }

    pub fn get_cpu_temp(&self) -> Vec<Vec<u32>> {
        self.cpu_temp.clone()
    }

    pub fn set_cpu_temp(&mut self, cpu_temp: Vec<Vec<u32>>) {
        self.cpu_temp = cpu_temp.clone()
    }

    pub fn get_cpu_temp_index(&self, index: usize) -> Option<Vec<u32>> {
        match self.get_cpu_temp().get(index) {
            Some(v) => Some(v.clone()),
            None => None,
        }
    }

    /// Sets element at index to the argument provided, and if this index is larger than the size of the Vec, pushes it on the Vec
    pub fn set_cpu_temp_index(&mut self, index: usize, element: Vec<u32>) {
        if index < self.cpu_temp.len() {
            let mut setter: Vec<Vec<u32>> = vec![];
            for i in 0..self.cpu_temp.len() {
                if i == index {
                    setter.push(element.clone());
                } else {
                    setter.push(self.get_cpu_temp_index(index.clone()).unwrap())
                }
            }
            self.cpu_temp = setter.clone();
        } else {
            self.cpu_temp.push(element.clone());
        }
    }

    pub fn get_cpu_temp_inner_index(&self, index1: usize, index2: usize) -> Option<u32> {
        match self.get_cpu_temp_index(index1.clone()) {
            Some(v) => match v.get(index2) {
                Some(u) => Some(u.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn set_cpu_temp_inner_index(&mut self, index1: usize, index2: usize, element: u32) {
        self.set_cpu_temp_index(
            index1.clone(),
            match self.get_cpu_temp_index(index1.clone()) {
                Some(v) => {
                    let mut returnable: Vec<u32> = v.clone();
                    if index2 > v.len() {
                        returnable = v.clone();
                    } else {
                        returnable = v.clone();
                        returnable.push(element);
                    }
                    returnable.clone()
                }
                None => vec![element.clone()],
            },
        );
    }

    pub fn get_cpu_temp_high(&self) -> i32 {
        self.cpu_temp_high.clone()
    }

    pub fn set_cpu_temp_high(&mut self, cpu_temp_high: i32) {
        self.cpu_temp_high = cpu_temp_high.clone();
    }

    pub fn get_cpu_temp_crit(&self) -> i32 {
        self.cpu_temp_crit.clone()
    }

    pub fn set_cpu_temp_crit(&mut self, cpu_temp_crit: i32) {
        self.cpu_temp_crit = cpu_temp_crit.clone()
    }

    pub fn get_freq_error(&self) -> bool {
        self.freq_error.clone()
    }

    pub fn set_freq_error(&mut self, freq_error: bool) {
        self.freq_error = freq_error.clone()
    }

    pub fn get_cpu_freq(&self) -> f64 {
        self.cpu_freq.clone()
    }

    pub fn set_cpu_freq(&mut self, cpu_freq: f64) {
        self.cpu_freq = cpu_freq.clone()
    }

    pub fn get_load_avg(&self) -> Vec<f64> {
        self.load_avg.clone()
    }

    pub fn set_load_avg(&mut self, load_avg: Vec<f64>) {
        self.load_avg = load_avg.clone()
    }

    pub fn get_load_avg_index(&self, index: usize) -> Option<f64> {
        match self.load_avg.get(index) {
            Some(f) => Some(f.clone()),
            None => None,
        }
    }

    pub fn set_load_avg_index(&mut self, index: usize, element: f64) {
        let mut setter: Vec<f64> = vec![];

        if index > self.get_load_avg().len() {
            self.load_avg.push(element);
        } else {
            for i in 0..self.get_load_avg().len() {
                if i == index {
                    setter.push(element)
                } else {
                    setter.push(self.get_load_avg_index(i).unwrap())
                }
            }

            self.load_avg = setter;
        }
    }

    pub fn get_uptime(&self) -> String {
        self.uptime.clone()
    }

    pub fn set_uptime(&mut self, uptime: String) {
        self.uptime = uptime.clone()
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer.clone()
    }

    pub fn get_sensor_method(&self) -> String {
        self.sensor_method.clone()
    }

    pub fn set_sensor_method(&mut self, sensor_method: String) {
        self.sensor_method = sensor_method.clone()
    }

    pub fn get_got_sensors(&self) -> bool {
        self.got_sensors.clone()
    }

    pub fn set_got_sensors(&mut self, got_sensors: bool) {
        self.got_sensors = got_sensors.clone()
    }

    pub fn get_sensor_swap(&self) -> bool {
        self.sensor_swap.clone()
    }

    pub fn set_sensor_swap(&mut self, sensor_swap: bool) {
        self.sensor_swap = sensor_swap.clone()
    }

    pub fn get_cpu_temp_only(&self) -> bool {
        self.cpu_temp_only.clone()
    }

    pub fn set_cpu_temp_only(&mut self, cpu_temp_only: bool) {
        self.cpu_temp_only = cpu_temp_only.clone()
    }
}

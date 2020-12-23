use crate::collector::*;
use crate::error;
use crate::Config;
use hhmmss::Hhmmss;
use psutil::sensors::*;
use std::time::SystemTime;
use std::{collections::HashMap, iter::Enumerate, path::*};
use subprocess::Exec;
use sys_info::*;
use which::which;

pub struct CpuCollector {
    pub cpu_usage: Vec<Vec<u32>>,
    pub cpu_temp: Vec<Vec<u32>>,
    pub cpu_temp_high: i32,
    pub cpu_temp_crit: i32,
    pub freq_error: bool,
    pub cpu_freq: f64,
    pub load_avg: Vec<f32>,
    pub uptime: String,
    pub buffer: String,
    pub sensor_method: String,
    pub got_sensors: bool,
    pub sensor_swap: bool,
    pub cpu_temp_only: bool,
}
impl CollTrait for CpuCollector {
    fn init(THREADS: u64) {
        let mut cpu_usage_mut = Vec::<Vec<u32>>::new();
        let mut cpu_temp_mut = Vec::<Vec<u32>>::new();
        for _ in 0..THREADS + 1 {
            cpu_usage_mut.push(Vec::new());
            cpu_temp_mut.push(Vec::new());
        }

        let mut CpuCollector_initialize = CpuCollector {
            cpu_usage: cpu_usage_mut,
            cpu_temp: cpu_temp_mut,
            cpu_temp_high: 0,
            cpu_temp_crit: 0,
            freq_error: false,
            cpu_freq: 0,
            load_avg: Vec::<f64>::new(),
            uptime: String::from(""),
            buffer: String::from(""),
            sensor_method: String::from(""),
            got_sensors: false,
            sensor_swap: false,
            cpu_temp_only: false,
        };
    }

    fn collect<P: AsRef<Path>>(
        &mut self,
        collectors: Vec<dyn CollTrait>,
        CONFIG: Config,
        CONFIG_DIR: P,
        draw_now: bool,
        interrupt: bool,
        proc_interrupt: bool,
        redraw: bool,
        only_draw: bool,
        t: Term,
    ) {
        match psutil::cpu::CpuPercentCollector::cpu_percent() {
            Some(p) => self.cpu_usage[0].push(format!("{:.2}", p)),
            None => (),
        }

        if self.cpu_usage[0] > t.width * 4 {
            self.cpu_usage[0].remove(0);
        }

        let cpu_percentages = match psutil::cpu::CpuPercentCollector::new() {
            Ok(p) => match p.cpu_percent_percpu() {
                Ok(p) => p,
                Err(e) => error::errlog(
                    CONFIG_DIR,
                    format!("Unable to collect CPU percentages! (error {})", e).as_str(),
                ),
            },
            Err(e) => error::errlog(
                CONFIG_DIR,
                format!("Unable to collect CPU percentages! (error {})", e).as_str(),
            ),
        };

        for (n, thread) in cpu_percentages.iter().enumerate() {
            self.cpu_usage[n].push(format!("{:.2}", thread as u32));
            if self.cpu_usage[n].capacity() > t.width * 2 {
                self.cpu_usage[n].remove(0);
            }
        }

        let cpu_frequency = match psutil::cpu::cpu_freq() {
            Ok(f) => f.current(),
            Err(e) => error::errlog(
                CONFIG_DIR,
                format!("Unable to collect CPU frequency! (error {})", e).as_str(),
            ),
        };

        self.cpu_freq = cpu_frequency;

        let lavg = match sys_info::loadavg() {
            Ok(l) => [
                format!("{:.2}", l.one).parse::<f64>().unwrap(),
                format!("{:.2}", l.five).parse::<f64>().unwrap(),
                format!("{:.2}", l.fifteen).parse::<f64>().unwrap(),
            ],
            Err(e) => error::errlog(
                CONFIG_DIR,
                format!("Unable to collect load average! (error {})", e).as_str(),
            ),
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
            Err(e) => error::errlog(
                CONFIG_DIR,
                "Error finding the boot time of this system...".to_owned(),
            ),
        };

        if CONFIG.check_temp && self.got_sensors {
            self.collect_temps();
        }
    }
}
impl CpuCollector {
    pub fn get_sensors(&mut self, CONFIG: Config, SYSTEM: String) {
        self.sensor_method = String::from("");

        if SYSTEM == "MacOS" {
            match which("coretemp") {
                Ok() => {
                    let output = Exec::shell("coretemp -p")
                        .capture()?
                        .stdout_str()
                        .to_owned();
                    match output.trim().replace("-", "").parse::<f64>() {
                        Some(n) => self.sensor_method = "coretemp",
                        None => match which("osx-cpu-temp") {
                            Ok() => {
                                let output = Exec::shell("osx-cpu-temp")
                                    .capture()?
                                    .stdout_str()
                                    .to_owned();
                                match output.trim_end() {
                                    Some(s) => {
                                        if s.ends_with("°C") {
                                            self.sensor_method = "osx-cpu-temp";
                                        }
                                    }
                                    None => (),
                                };
                            }
                            Err() => (),
                        },
                    }
                }
                Err() => (),
            }
        } else if CONFIG.cpu_sensor != "Auto" && CONFIG.cpu_sensor.contains(CONFIG.cpu_sensor) {
            self.sensor_method = "psutil";
        } else {
            for res in temperatures() {
                match res {
                    Ok(temp) => {
                        if temp.unit().to_lowercase().starts_with("cpu") {
                            self.sensor_method = "psutil";
                            break;
                        }
                        match temp.label {
                            Some(label) => {
                                let arr: &str = vec!["Package", "Core 0", "Tdie", "CPU"];

                                for test in arr {
                                    if label.starts_with(test) {
                                        self.sensor_method = "psutil";
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

        if self.sensor_method == "" && SYSTEM == "Linux" {
            let output: Option<String> = match which("vcgencmd") {
                Some(s) => Some(
                    Exec::shell("vcgencmd measure_temp")
                        .capture()?
                        .stdout_str()
                        .to_owned(),
                ),
                None => None,
            };

            match output {
                Some(s) => {
                    if s.trim().endswith("'C") {
                        self.sensor_method = "vcgencmd";
                    }
                }
                None => (),
            };

            self.got_sensors = self.sensor_method.chars.count() > 0;
        }
    }

    pub fn collect_temps(&mut self, CONFIG: Config, THREADS : u64) {
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
                s_name = splitter.next().unwrap();
                s_label = splitter.next().unwrap();
            }

            let mut num = 1;
            for res in psutil::sensors::temperatures() {
                match res {
                    Ok(s) => {
                        let mut sensor = s.clone();

                        match sensor.unit() {
                            Some(name) => {
                                let label = sensor.label().unwrap();

                                if name == s_name
                                    && (sensor.label().unwrap_or("error_in_label") == s_label || String::from(num) == s_label.to_owned())
                                    && sensor.current() > 0 {
                                    
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
                                                if t.celsius() > 1 {
                                                    self.cpu_temp_high = t.celsius().round() as i32;
                                                } else {
                                                    self.cpu_temp_high = 80;
                                                }
                                            }
                                            None => self.cpu_temp_high = 80,
                                        }

                                        match sensor.critical() {
                                            Some(t) => {
                                                if t.celsius() > 1 {
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
                                    && sensor.current().celsius().round() > 0 {
                                        
                                        if self.cpu_temp_high == 0 || self.sensor_swap || cpu_type == "other" {
                                            self.sensor_swap = false;
                                            match sensor.high() {
                                                Some(t) => {
                                                    if t > 1 {
                                                        self.cpu_temp_high = t.celsius().round();
                                                    } else {
                                                        self.cpu_temp_high = 80
                                                    }
                                                }
                                                None => self.cpu_temp_high = 80,
                                            }

                                            match sensor.critical() {
                                                Some(t) => {
                                                    if t > 1 {
                                                        self.cpu_temp_crit = t.celsius().round();
                                                    } else {
                                                        self.cpu_temp_crit = 95;
                                                    }
                                                },
                                                None =>  self.cpu_temp_crit = 95,
                                            }

                                            if label.starts_with("Package") {
                                                cpu_type = "intel";
                                            } else {
                                                cpu_type = "ryzen";
                                            }
                                        }
                                } else if (label.starts_with("Core")
                                || label.starts_with("Tccd")
                                || label.starts_with("CPU")
                                || name.to_owned().to_lowercase().starts_with("cpu"))
                                && sensor.current().celsius() > 0 {
                                    if label.starts_with("Core")
                                    || label.starts_with("Tccd") {
                                        entry_int = label.replace("Core", "").replace("Tccd", "").parse::<i32>();

                                        if core_dict.contains_key(entry_int) && cpu_type != "ryzen" {
                                            if c_max == 0 {
                                                let mut largest = 0;
                                                for (key, val) in core_dict {
                                                    if key > largest{
                                                        largest = key.clone();
                                                    }
                                                }
                                                c_max == largest + 1;
                                            }
                                            if c_max < (THREADS / 2).floor() && !core_dict.contains_key(entry_int + c_max) {
                                                core_dict.insert(entry_int + c_max, sensor.current().celsius().round());
                                            }
                                            continue;
                                        } else if core_dict.contains(entry_int) {
                                            continue;
                                        }
                                        core_dict.set(entry_int, sensor.current().celsius().round());
                                        continue;
                                    } else if vec!["intel", "ryzen"].contains(cpu_type) {
                                        continue;
                                    }

                                    if cpu_type == "" {
                                        cpu_type = String::from("other");
                                        if self.cpu_temp_high == 0 || self.sensor_swap {
                                            self.sensor_swap = false;
                                            
                                            match sensor.high() {
                                                Some(t) => {
                                                    if t.celsius() > 1 {
                                                        self.cpu_temp_high = t.celsius().round();
                                                    } else {
                                                        self.cpu_temp_high = match name {
                                                            "cpu_thermal" => 60,
                                                            _ => 80,
                                                        };
                                                    }
                                                },
                                                None => self.cpu_temp_high = match name {
                                                    "cpu_thermal" => 60,
                                                    _ => 80,
                                                },
                                            }

                                            match sensor.critical() {
                                                Some(t) => {
                                                    if t.celsius() > 1 {
                                                        self.cpu_temp_crit = t.celsius().round();
                                                    } else {
                                                        self.cpu_temp_crit = match name {
                                                            "cpu_thermal" => 80,
                                                            _ => 95,
                                                        };
                                                    }
                                                },
                                                None => self.cpu_temp_crit = match name {
                                                    "cpu_thermal" => 80,
                                                    _ => 95,
                                                },
                                            }
                                        }
                                        temp = sensor.current().celsius().round();
                                    }
                                    cores.append(sensor.current().celsius().round());
                                }
                            }
                            None => (),
                        }
                    }
                    Err(e) => (),
                }
                num += 1;
            }
            
            if core_dict.len() > 0 {
                if temp == 1000 {
                    temp = (core_dict.values().into_iter().sum() / core_dict.len()).floor();
                }
                if self.cpu_temp_high == 0 || self.cpu_temp_crit == 0 {
                    (self.cpu_temp_high, self.cpu_temp_crit) = (80, 95);
                }
                self.cpu_temp[0].append(temp);
            }
        }
    }
}

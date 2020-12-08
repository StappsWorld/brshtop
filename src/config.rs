use std::collections::*;
use std::path::*;
use psutil::sensors::*;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use lenient_bool::LenientBool;


// TODO : Fix macro scope
pub enum ConfigItem {
    Str(String),
    Int(i64),
    Bool(bool),
    ViewMode(ViewMode),
    LogLevel(LogLevel),
    SortingOption(SortingOption),
    Error,
}

#[derive(Clone, Copy)]
pub enum ViewMode {
    Full,
    Proc,
    Stat,
}

#[derive(Clone, Copy)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

#[derive(Clone, Copy)]
pub enum SortingOption {
    Pid,
    Program,
    Arguments,
    Threads,
    User,
    Memory,
    Cpu { lazy: bool },
}



pub struct Config {
    keys: Vec<String>,
    conf_dict: HashMap<String, ConfigItem>,
    attr: HashMap<String, ConfigItem>,
    color_theme: String,
    theme_background: bool,
    update_ms: i64,
    proc_sorting: SortingOption,
    proc_reversed: bool,
    proc_tree: bool,
    tree_depth: i32,
    proc_colors: bool,
    proc_gradient: bool,
    proc_per_core: bool,
    proc_mem_bytes: bool,
    check_temp: bool,
    cpu_sensor: String,
    show_coretemp: bool,
    draw_clock: String,
    background_update: bool,
    custom_cpu_name: String,
    disks_filter: String,
    update_check: bool,
    mem_graphs: bool,
    show_swap: bool,
    swap_disk: bool,
    show_disks: bool,
    net_download: String,
    net_upload: String,
    net_color_fixed: bool,
    net_auto: bool,
    net_sync: bool,
    show_battery: bool,
    show_init: bool,
    view_mode: ViewMode,
    log_level: LogLevel,
    warnings: Vec<String>,
    info: Vec<String>,
    changed: bool,
    config_file: PathBuf,
    recreate: bool,
    sorting_options: Vec<SortingOption>,
    log_levels: Vec<LogLevel>,
    view_modes: Vec<ViewMode>,
    cpu_sensors: Vec<String>,
    _initialized: bool,
} impl Config {

    pub fn new( path : PathBuf, version : String) -> Result<Self, &'static str> {

        let mut cpu_sensors_mut : Vec::<String> = vec!["Auto"].iter().map(|s| s.to_string()).collect();
        let _temps = temperatures();
        let mut num = 1;
        for res in _temps{
            match res {
                Ok(t) =>{
                    let name = t.unit().to_owned();
                    let label_option = t.label();
                    match label_option {
                        Some(l) => {
                            cpu_sensors_mut.push(format!("{}:{}",name, l));
                        },
                        None => {
                            cpu_sensors_mut.push(format!("{}:{}",name, num));
                        },
                    };

                    num += 1;
                }
                Err(e) => (),
            };
        }

        let keys_unconverted = vec!["color_theme", "update_ms", "proc_sorting", "proc_reversed", "proc_tree", "check_temp", "draw_clock", "background_update", "custom_cpu_name",
        "proc_colors", "proc_gradient", "proc_per_core", "proc_mem_bytes", "disks_filter", "update_check", "log_level", "mem_graphs", "show_swap",
        "swap_disk", "show_disks", "net_download", "net_upload", "net_auto", "net_color_fixed", "show_init", "view_mode", "theme_background",
        "net_sync", "show_battery", "tree_depth", "cpu_sensor", "show_coretemp"];

        let mut initializing_config = Config {
            keys: keys_unconverted.iter().map(|s| s.to_string()).collect(),
            conf_dict: HashMap::<String, ConfigItem>::new(),
            attr: HashMap::<String, ConfigItem>::new(),
            color_theme: "Default".to_string(),
            theme_background: true,
            update_ms: 2000,
            proc_sorting: SortingOption::Cpu{lazy : true},
            proc_reversed: false,
            proc_tree: false,
            tree_depth: 3,
            proc_colors: true,
            proc_gradient: true,
            proc_per_core: false,
            proc_mem_bytes: true,
            check_temp: true,
            cpu_sensor: "Auto".to_string(),
            show_coretemp: true,
            draw_clock: "%X".to_string(),
            background_update: true,
            custom_cpu_name: "".to_string(),
            disks_filter: "".to_string(),
            update_check: true,
            mem_graphs: true,
            show_swap: true,
            swap_disk: true,
            show_disks: true,
            net_download: "10M".to_string(),
            net_upload: "10M".to_string(),
            net_color_fixed: false,
            net_auto: true,
            net_sync: false,
            show_battery: true,
            show_init: true,
            view_mode: ViewMode::Full,
            log_level: LogLevel::Warning,
            warnings: Vec::<String>::new(),
            info: Vec::<String>::new(),
            sorting_options: vec![SortingOption::Pid, SortingOption::Program, SortingOption::Arguments, SortingOption::Threads, SortingOption::User, SortingOption::Memory, SortingOption::Cpu {lazy : true}, SortingOption::Cpu {lazy : false}],
            log_levels: vec![LogLevel::Error, LogLevel::Warning, LogLevel::Error, LogLevel::Debug],
            view_modes: vec![ViewMode::Full, ViewMode::Proc, ViewMode::Stat],
            cpu_sensors: cpu_sensors_mut,
            changed: false,
            recreate: false,
            config_file: path,
            _initialized: false,
        };

        let mut conf = match Config::load_config(&mut initializing_config) {
            Ok(d) => d,
            Err(e) => return Err(e)
        };

        if !conf.contains_key(&"version".to_owned()) {
            initializing_config.recreate = true;
            initializing_config.info.push("Config file malformatted or mossing, will be recreated on exit!".to_owned());
        } else  {
            match conf.get(&"version".to_owned()).unwrap() {
                ConfigItem::Str(s) => {
                    if *s != version {
                        initializing_config.recreate = true;
                        initializing_config.warnings.push("Config file version and brshtop version missmatch, will be recreated on exit!".to_owned())
                    }
                }
                _ => {
                    initializing_config.recreate = true;
                    initializing_config.warnings.push("Config file is malformed, will be recreated on exit!".to_owned())
                }
            }
        }

        let keys_for_loop : Vec<String> = initializing_config.keys.iter().map(|c| c.clone()).collect();

        for key in  keys_for_loop{
            if conf.contains_key(&key) {
                match conf.get(&key).unwrap() {
                    ConfigItem::Error => {
                        
                        initializing_config.recreate = true;

                        let sender = match initializing_config.attr.get(&key).unwrap() {
                            ConfigItem::Str(s) => ConfigItem::Str(String::from(s)),
                            ConfigItem::Int(i) => ConfigItem::Int(*i),
                            ConfigItem::Bool(b) => ConfigItem::Bool(*b),
                            ConfigItem::ViewMode(v) => ConfigItem::ViewMode(*v),
                            ConfigItem::LogLevel(l) => ConfigItem::LogLevel(*l),
                            ConfigItem::SortingOption(s) => ConfigItem::SortingOption(*s),
                            ConfigItem::Error => ConfigItem::Error,
                            _ => continue,
                        };

                        initializing_config.conf_dict.insert(key, sender);
                    },
                    _ => {

                        let sender = match conf.get(&key).unwrap() {
                            ConfigItem::Str(s) => ConfigItem::Str(String::from(s)),
                            ConfigItem::Int(i) => ConfigItem::Int(*i),
                            ConfigItem::Bool(b) => ConfigItem::Bool(*b),
                            ConfigItem::ViewMode(v) => ConfigItem::ViewMode(*v),
                            ConfigItem::LogLevel(l) => ConfigItem::LogLevel(*l),
                            ConfigItem::SortingOption(s) => ConfigItem::SortingOption(*s),
                            ConfigItem::Error => ConfigItem::Error,
                            _ => continue,
                        };

                        initializing_config.attr.insert(key, sender);
                    }
                };
            }
        }
        initializing_config._initialized = true;
        


        
        Ok(initializing_config)

    }

    /// Returns a HashMap<String, ConfigItem> from the configuration file
    pub fn load_config(&mut self) -> Result<HashMap<String, ConfigItem>, &'static str> {
        let mut new_config = HashMap::<String, ConfigItem>::new();

        let mut conf_file = PathBuf::new();

        if self.config_file.is_file() {
            conf_file = self.config_file.clone();
        } else if PathBuf::from("/etc/brshtop.conf").is_file() {
            conf_file = PathBuf::from("/etc/brshtop.conf");
        } else {
            return Err("Could not find config file.");
        }

        let file = match File::open(conf_file) {
            Ok(f) => f,
            Err(e) => return Err("Unable to read config file."),
        };
        let mut buf_reader = BufReader::new(file);

       for line in buf_reader.lines(){
            match line {
                Ok(l) => {
                    let mut l_stripped_before = l.clone();
                    l_stripped_before = l_stripped_before.trim_start().to_owned();
                    l_stripped_before = l_stripped_before.trim_end().to_owned();
                    let mut l_stripped_config = l_stripped_before.clone();

                    if l_stripped_config.starts_with("#? Config") {

                        let index_of_version = match l_stripped_config.find("v. ") {
                            Some(i) => i,
                            None => return Err("Malformed configuration file."),
                        };

                        new_config.insert(String::from("version"), ConfigItem::Str(l_stripped_config[(index_of_version + 3 as usize)..].to_owned()));
                        continue;
                    }

                    for key in &self.keys {
                        let mut l_stripped = l_stripped_before.clone();
                        if l_stripped.starts_with(key) {
                            l_stripped = l_stripped.replace(&(key.to_owned() + "="), "");
                            if l_stripped.starts_with('"') {
                                l_stripped.retain(|c| c != '"');
                            }

                            match key.as_str() {
                                "proc_sorting" => {
                                    let mut to_insert : SortingOption;
                                    match l_stripped.as_str() {
                                        "pid" => to_insert = SortingOption::Pid,
                                        "program" => to_insert = SortingOption::Program,
                                        "arguments" => to_insert = SortingOption::Arguments,
                                        "threads" => to_insert = SortingOption::Threads,
                                        "user" => to_insert = SortingOption::User,
                                        "memory" => to_insert = SortingOption::Memory,
                                        "cpu" => to_insert = SortingOption::Cpu{lazy : false},
                                        "cpu lazy" => to_insert = SortingOption::Cpu{lazy : true},
                                        _ => {
                                            self.warnings.push("Config key \"proc_sorted\" didn\'t get an acceptable value!".to_owned());
                                            new_config.insert(key.to_owned(), ConfigItem::Error);
                                            continue;
                                        },
                                    };
                                    new_config.insert(key.to_owned(), ConfigItem::SortingOption(to_insert));
                                    continue;
                                },
                                "log_level" => {
                                    let mut to_insert : LogLevel;
                                    match l_stripped.as_str() {
                                        "error" => to_insert = LogLevel::Error,
                                        "warning" => to_insert = LogLevel::Warning,
                                        "info" => to_insert = LogLevel::Info,
                                        "debug" => to_insert = LogLevel::Debug,
                                        _ => {
                                            self.warnings.push("Config key \"log_level\" didn\'t get an acceptable value!".to_owned());
                                            new_config.insert(key.to_owned(), ConfigItem::Error);
                                            continue;
                                        }
                                    };
                                    new_config.insert(key.to_owned(), ConfigItem::LogLevel(to_insert));
                                    continue;
                                },
                                "view_mode" => {
                                    let mut to_insert : ViewMode;
                                    match l_stripped.as_str() {
                                    "full" => to_insert = ViewMode::Full,
                                    "proc" => to_insert = ViewMode::Proc,
                                    "stat" => to_insert = ViewMode::Stat,
                                    _ => {
                                        self.warnings.push("Config key \"view_mode\" didn\'t get an acceptable value!".to_owned());
                                        new_config.insert(key.to_owned(), ConfigItem::Error);
                                        continue;
                                    }
                                    };
                                    new_config.insert(key.to_owned(), ConfigItem::ViewMode(to_insert));
                                    continue;
                                },
                                _ => (),
                            }

                            let check_numeric : Vec<bool> = l_stripped.chars().map(|c| c.is_numeric()).collect();
                            if !check_numeric.contains(&false) {
                                let i = match l_stripped.parse::<i64>() {
                                    Ok(i) => i,
                                    Err(_e) => {
                                        self.warnings.push(format!("Config key \"{}\" should be an integer (was \"{}\")!", key, l_stripped));
                                        continue;
                                    },
                                  };
                                
                                if key == "update_ms" && i < 100 {
                                    self.warnings.push("Config key \"update_ms\" can\'t be lower than 100!".to_owned());
                                    new_config.insert(key.to_owned(), ConfigItem::Int(100));
                                    continue;
                                }

                                new_config.insert(key.to_owned(), ConfigItem::Int(i));
                                continue;
                            }

                            match l_stripped.parse::<LenientBool>(){
                                Ok(b) => {
                                    new_config.insert(key.to_owned(), ConfigItem::Bool(b.into()));
                                    continue;
                                },
                                Err(e) => (),
                            };

                            new_config.insert(key.to_owned(), ConfigItem::Str(l_stripped));
                           
                        }
                    }
                }
                Err(e) => return Err("Unable to read config file."),
            };
        }

        for net_name in ["net_download", "net_upload"].iter() {
            if new_config.contains_key(net_name.to_owned()) {
                match new_config.get(net_name.to_owned()).unwrap() {
                    ConfigItem::Str(s) => {
                        match s.chars().next() {
                            Some(c) => {
                                if !c.is_numeric() {
                                    new_config.insert(net_name.to_owned().to_string(), ConfigItem::Error);
                                    self.warnings.push(format!("Config key \"{}\" didn\'t get an acceptable value!", net_name));
                                }
                            }
                            None => {
                                new_config.insert(net_name.to_owned().to_string(), ConfigItem::Error);
                                self.warnings.push(format!("Config key \"{}\" didn\'t get an acceptable value!", net_name));
                            }
                        };
                    }
                    _ => {
                        new_config.insert(net_name.to_owned().to_string(), ConfigItem::Error);
                        self.warnings.push(format!("Config key \"{}\" didn\'t get an acceptable value!", net_name));
                    }
                }
            }
        }

        match new_config.get("cpu_sensor") {
            Some(c) => {
                match c {
                    ConfigItem::Str(s) => {
                        if !self.cpu_sensor.contains(s) {
                            new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                            self.warnings.push(format!("Config key \"cpu_sensor\" does not contain an available sensor!"));
                        }
                    },
                    _ => {
                        new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                        self.warnings.push(format!("Config key \"cpu_sensor\" has a malformed value!"));
                    }
                }
            },
            None => {
                new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                self.warnings.push(format!("Config key \"cpu_sensor\" has a malformed value or does not exist!"));
            }
        }



        return Ok(new_config);
    }

}
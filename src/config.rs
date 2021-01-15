use {
    crate::{error::{errlog, throw_error}, VERSION},
    lenient_bool::LenientBool,
    psutil::sensors::*,
    std::{
        collections::*,
        fmt::{self, Debug, Display, Formatter},
        fs::{write, File},
        io::{prelude::*, BufReader},
        path::*,
    },
};

// TODO : Fix macro scope
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigItem {
    Str(String),
    Int(i64),
    Bool(bool),
    ViewMode(ViewMode),
    LogLevel(LogLevel),
    SortingOption(SortingOption),
    Error,
}
impl Display for ConfigItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConfigItem::Str(s) => write!(f, "{:?}", s),
            ConfigItem::Int(i) => write!(f, "{:?}", i),
            ConfigItem::Bool(b) => write!(f, "{:?}", b),
            ConfigItem::Error => write!(f, "{:?}", "_error_"),
            ConfigItem::ViewMode(v) => write!(f, "{:?}", v),
            ConfigItem::LogLevel(l) => write!(f, "{:?}", l),
            ConfigItem::SortingOption(s) => write!(f, "{:?}", s),
        }
    }
}
impl ConfigItem {
    fn sorting_option(s: &String) -> Result<Self, String> {
        Ok(ConfigItem::SortingOption(match s.to_string().as_str() {
            "pid" => SortingOption::Pid,
            "program" => SortingOption::Program,
            "arguments" => SortingOption::Arguments,
            "threads" => SortingOption::Threads,
            "user" => SortingOption::User,
            "memory" => SortingOption::Memory,
            "cpu" => SortingOption::Cpu { lazy: false },
            "cpu lazy" => SortingOption::Cpu { lazy: true },
            bad => {
                return Err(format!(
                    r#"Config key "proc_sorted" had an unknown value: {}"#,
                    bad
                ));
            }
        }))
    }

    fn log_level(s: &String) -> Result<Self, String> {
        Ok(match s.to_string().as_str() {
            "error" => ConfigItem::LogLevel(LogLevel::Error),
            "warning" => ConfigItem::LogLevel(LogLevel::Warning),
            "info" => ConfigItem::LogLevel(LogLevel::Info),
            "debug" => ConfigItem::LogLevel(LogLevel::Debug),
            bad => {
                return Err(format!(
                    r#"Config key "log_level" had an unknown value: {}"#,
                    bad
                ));
            }
        })
    }

    fn view_mode(s: &String) -> Result<Self, String> {
        Ok(match s.to_string().as_str() {
            "full" => ConfigItem::ViewMode(ViewMode::Full),
            "proc" => ConfigItem::ViewMode(ViewMode::Proc),
            "stat" => ConfigItem::ViewMode(ViewMode::Stat),
            bad => {
                return Err(format!(
                    r#"Config key "view_mode" had an unknown value: {}"#,
                    bad
                ));
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewMode {
    Full,
    Proc,
    Stat,
    None,
}
impl Display for ViewMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ViewMode::Full => write!(f, "{:?}", "full"),
            ViewMode::Proc => write!(f, "{:?}", "proc"),
            ViewMode::Stat => write!(f, "{:?}", "stat"),
            ViewMode::None => write!(f, "{:?}", "None"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}
impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "{:?}", "error"),
            LogLevel::Warning => write!(f, "{:?}", "warning"),
            LogLevel::Info => write!(f, "{:?}", "info"),
            LogLevel::Debug => write!(f, "{:?}", "debug"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SortingOption {
    Pid,
    Program,
    Arguments,
    Threads,
    User,
    Memory,
    Cpu { lazy: bool },
}
impl Display for SortingOption {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SortingOption::Pid => write!(f, "{:?}", "pid"),
            SortingOption::Program => write!(f, "{:?}", "program"),
            SortingOption::Arguments => write!(f, "{:?}", "arguments"),
            SortingOption::Threads => write!(f, "{:?}", "threads"),
            SortingOption::User => write!(f, "{:?}", "user"),
            SortingOption::Memory => write!(f, "{:?}", "memory"),
            SortingOption::Cpu { lazy: b } => match b {
                true => write!(f, "{:?}", "cpu lazy"),
                false => write!(f, "{:?}", "cpu"),
            },
        }
    }
}
impl From<String> for SortingOption {
    fn from(x : String) -> SortingOption {
        match x.to_lowercase().as_str() {
            "pid" => SortingOption::Pid,
            "program" => SortingOption::Program,
            "arguments" => SortingOption::Arguments,
            "threads" => SortingOption::Threads,
            "user" => SortingOption::User,
            "memory" => SortingOption::Memory,
            "cpu" => SortingOption::Cpu{lazy : false},
            "cpu lazy" => SortingOption::Cpu{lazy : true},
            _ => {
                errlog(format!("Unable to convert {} to sortingoption. Defaulting to Arguments", x.clone()));
                SortingOption::Arguments
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfigAttr {
    String(String),
    Bool(bool),
    ViewMode(ViewMode),
    Int64(i64),
    SortingOption(SortingOption),
    LogLevel(LogLevel),
}
impl Display for ConfigAttr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ConfigAttr::Bool(b) => write!(f, "{}", b),
            ConfigAttr::Int64(i) => write!(f, "{}", i),
            ConfigAttr::LogLevel(l) => write!(f, "{}", l.to_string()),
            ConfigAttr::SortingOption(s) => write!(f, "{}", s.to_string()),
            ConfigAttr::String(s) => write!(f, "{}", s),
            ConfigAttr::ViewMode(v) => write!(f, "{}", v.to_string()),
        }
    }
}

pub struct Config {
    pub keys: Vec<String>,
    pub conf_dict: HashMap<String, ConfigItem>,
    pub attr: HashMap<String, ConfigItem>,
    pub color_theme: String,
    pub theme_background: bool,
    pub update_ms: i64,
    pub proc_sorting: SortingOption,
    pub proc_reversed: bool,
    pub proc_tree: bool,
    pub tree_depth: i32,
    pub proc_colors: bool,
    pub proc_gradient: bool,
    pub proc_per_core: bool,
    pub proc_mem_bytes: bool,
    pub check_temp: bool,
    pub cpu_sensor: String,
    pub show_coretemp: bool,
    pub draw_clock: String,
    pub background_update: bool,
    pub custom_cpu_name: String,
    pub disks_filter: String,
    pub update_check: bool,
    pub mem_graphs: bool,
    pub show_swap: bool,
    pub swap_disk: bool,
    pub show_disks: bool,
    pub net_download: String,
    pub net_upload: String,
    pub net_color_fixed: bool,
    pub net_auto: bool,
    pub net_sync: bool,
    pub show_battery: bool,
    pub show_init: bool,
    pub view_mode: ViewMode,
    pub log_level: LogLevel,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
    pub changed: bool,
    pub config_file: PathBuf,
    pub recreate: bool,
    // TODO: We probably don't need these
    pub sorting_options: Vec<SortingOption>,
    pub log_levels: Vec<LogLevel>,
    pub view_modes: Vec<ViewMode>,
    pub cpu_sensors: Vec<String>,
    pub _initialized: bool,
}
impl Config {
    pub fn new(path: PathBuf) -> Result<Self, &'static str> {
        let mut cpu_sensors_mut: Vec<String> = vec!["Auto".into()];
        let mut num = 1;
        for res in temperatures() {
            match res {
                Ok(t) => {
                    cpu_sensors_mut.push(format!(
                        "{}-{}",
                        t.unit(),
                        t.label().unwrap_or(&num.to_string())
                    ));

                    num += 1;
                }
                Err(e) => (),
            };
        }

        let keys_unconverted = vec![
            "color_theme",
            "update_ms",
            "proc_sorting",
            "proc_reversed",
            "proc_tree",
            "check_temp",
            "draw_clock",
            "background_update",
            "custom_cpu_name",
            "proc_colors",
            "proc_gradient",
            "proc_per_core",
            "proc_mem_bytes",
            "disks_filter",
            "update_check",
            "log_level",
            "mem_graphs",
            "show_swap",
            "swap_disk",
            "show_disks",
            "net_download",
            "net_upload",
            "net_auto",
            "net_color_fixed",
            "show_init",
            "view_mode",
            "theme_background",
            "net_sync",
            "show_battery",
            "tree_depth",
            "cpu_sensor",
            "show_coretemp",
        ];

        let mut initializing_config = Config {
            keys: keys_unconverted.iter().map(|s| s.to_string()).collect(),
            conf_dict: HashMap::<String, ConfigItem>::new(),
            attr: HashMap::<String, ConfigItem>::new(),
            color_theme: "Default".to_string(),
            theme_background: true,
            update_ms: 2000,
            proc_sorting: SortingOption::Cpu { lazy: true },
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
            // TODO: We probably don't need these
            sorting_options: vec![
                SortingOption::Pid,
                SortingOption::Program,
                SortingOption::Arguments,
                SortingOption::Threads,
                SortingOption::User,
                SortingOption::Memory,
                SortingOption::Cpu { lazy: true },
                SortingOption::Cpu { lazy: false },
            ],
            log_levels: vec![
                LogLevel::Error,
                LogLevel::Warning,
                LogLevel::Error,
                LogLevel::Debug,
            ],
            view_modes: vec![ViewMode::Full, ViewMode::Proc, ViewMode::Stat],
            cpu_sensors: cpu_sensors_mut,
            changed: false,
            recreate: false,
            config_file: path,
            _initialized: false,
        };

        let conf = match Config::load_config(&mut initializing_config) {
            Ok(d) => d,
            Err(e) => return Err(e),
        };

        match conf.get(&"version".to_owned()) {
            Some(ConfigItem::Str(s)) => {
                if s.clone() != VERSION.to_owned() {
                    initializing_config.recreate = true;
                    initializing_config.warnings.push("Config file version and brshtop version mismatch, will be recreated on exit!".to_owned())
                }
            }
            _ => {
                initializing_config.recreate = true;
                initializing_config
                    .warnings
                    .push("Config file is or missing, will be recreated on exit!".to_owned())
            }
        }

        for key in initializing_config.keys.clone() {
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
                        };

                        initializing_config.conf_dict.insert(key, sender);
                    }
                    _ => {
                        let sender = match conf.get(&key).unwrap() {
                            ConfigItem::Str(s) => ConfigItem::Str(String::from(s)),
                            ConfigItem::Int(i) => ConfigItem::Int(*i),
                            ConfigItem::Bool(b) => ConfigItem::Bool(*b),
                            ConfigItem::ViewMode(v) => ConfigItem::ViewMode(*v),
                            ConfigItem::LogLevel(l) => ConfigItem::LogLevel(*l),
                            ConfigItem::SortingOption(s) => ConfigItem::SortingOption(*s),
                            ConfigItem::Error => ConfigItem::Error,
                        };

                        initializing_config.setattr(key, sender);
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

        let conf_file = if self.config_file.is_file() {
            self.config_file.clone()
        } else if PathBuf::from("/etc/brshtop.conf").is_file() {
            PathBuf::from("/etc/brshtop.conf")
        } else {
            return Err("Could not find config file.");
        };

        let file = match File::open(conf_file) {
            Ok(f) => f,
            Err(e) => return Err("Unable to read config file."),
        };
        let buf_reader = BufReader::new(file);

        for line in buf_reader.lines() {
            match line {
                Ok(l) => {
                    // TODO: split into a separate function please and thank you @me
                    let stripped = l.trim();

                    if stripped.starts_with("#? Config") {
                        let index_of_version = match stripped.find("v. ") {
                            Some(i) => i,
                            None => return Err("Malformed configuration file."),
                        };

                        new_config.insert(
                            String::from("version"),
                            ConfigItem::Str(stripped.chars().skip(index_of_version + 3).collect()),
                        );
                        continue;
                    }

                    for key in &self.keys {
                        let mut l_stripped = stripped.clone();
                        if l_stripped.starts_with(key) {
                            l_stripped = l_stripped.to_owned().replace(&(key.clone() + "="), "").as_str();
                            if l_stripped.starts_with('"') {
                                l_stripped.to_owned().retain(|c| c != '"');
                            }

                            type ConversionFunction = fn(&String) -> Result<ConfigItem, String>;

                            let conversion_function: Option<ConversionFunction> = match key.as_str()
                            {
                                "proc_sorting" => Some(ConfigItem::sorting_option),
                                "log_level" => Some(ConfigItem::log_level),
                                "view_mode" => Some(ConfigItem::view_mode),
                                _ => None,
                            };

                            if let Some(f) = conversion_function {
                                let config_item = match f(&(l_stripped.to_owned())) {
                                    Ok(item) => item,
                                    Err(e) => {
                                        self.warnings.push(e);
                                        ConfigItem::Error
                                    }
                                };

                                new_config.insert(key.clone(), config_item);
                            } else if !l_stripped.chars().all(char::is_numeric) {
                                let i = match l_stripped.parse::<i64>() {
                                    Ok(i) => i,
                                    Err(_e) => {
                                        self.warnings.push(format!(
                                            "Config key {:?} should be an integer (was {:?})!",
                                            key, l_stripped
                                        ));
                                        continue;
                                    }
                                };

                                if key == "update_ms" && i < 100 {
                                    self.warnings
                                        .push("Config key \"update_ms\" can\'t be lower than 100!".to_owned());
                                    new_config.insert(key.to_owned(), ConfigItem::Int(100));
                                    continue;
                                }

                                new_config.insert(key.to_owned(), ConfigItem::Int(i));
                                continue;
                            } else {
                                match l_stripped.parse::<LenientBool>() {
                                    Ok(b) => {
                                        new_config.insert(key.to_owned(), ConfigItem::Bool(b.into()));
                                        continue;
                                    }
                                    Err(e) => (),
                                };

                                new_config.insert(key.to_owned(), ConfigItem::Str(l_stripped.to_owned()));
                            }
                        }
                    }
                }
                Err(e) => return Err("Unable to read config file."),
            };
        }

        for net_name in ["net_download", "net_upload"].iter().map(|s| s.to_owned().to_owned()).collect::<Vec<String>>() {
            if new_config.contains_key(&net_name) {
                match new_config.get(&net_name).unwrap() {
                    ConfigItem::Str(s) => {
                        match s.chars().next() {
                            Some(c) => {
                                if !c.is_numeric() {
                                    new_config.insert(net_name, ConfigItem::Error);
                                    self.warnings.push(format!(
                                        "Config key \"{}\" didn\'t get an acceptable value!",
                                        net_name
                                    ));
                                }
                            }
                            None => {
                                new_config.insert(net_name, ConfigItem::Error);
                                self.warnings.push(format!(
                                    "Config key \"{}\" didn\'t get an acceptable value!",
                                    net_name
                                ));
                            }
                        };
                    }
                    _ => {
                        new_config.insert(net_name, ConfigItem::Error);
                        self.warnings.push(format!(
                            "Config key \"{}\" didn\'t get an acceptable value!",
                            net_name
                        ));
                    }
                }
            }
        }

        match new_config.get("cpu_sensor") {
            Some(c) => match c {
                ConfigItem::Str(s) => {
                    if !self.cpu_sensor.contains(s) {
                        new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                        self.warnings.push(format!(
                            "Config key \"cpu_sensor\" does not contain an available sensor!"
                        ));
                    }
                }
                _ => {
                    new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                    self.warnings
                        .push(format!("Config key \"cpu_sensor\" has a malformed value!"));
                }
            },
            None => {
                new_config.insert("cpu_sensor".to_owned(), ConfigItem::Error);
                self.warnings.push(format!(
                    "Config key \"cpu_sensor\" has a malformed value or does not exist!"
                ));
            }
        };

        return Ok(new_config);
    }

    pub fn setattr(&mut self, name: String, value: ConfigItem) {
        if self._initialized {
            self.changed = true;
        }

        self.attr.insert(name.clone(), value.clone());

        let test_values = vec!["_initialized", "recreate", "changed"];
        let test_values_converted: Vec<String> =
            test_values.iter().map(<_ as ToString>::to_string).collect();

        if test_values_converted.contains(&name) {
            self.conf_dict.insert(name.clone(), value.clone());
        }
    }

    pub fn getattr(&mut self, attr: String) -> ConfigAttr {
        match attr.as_str() {
            "color_theme" => ConfigAttr::String(self.color_theme.clone()),
            "theme_background" => ConfigAttr::Bool(self.theme_background),
            "view_mode" => ConfigAttr::ViewMode(self.view_mode),
            "update_ms" => ConfigAttr::Int64(self.update_ms),
            "proc_sorting" => ConfigAttr::SortingOption(self.proc_sorting),
            "proc_reversed" => ConfigAttr::Bool(self.proc_reversed),
            "proc_tree" => ConfigAttr::Bool(self.proc_tree),
            "tree_depth" => ConfigAttr::Int64(self.tree_depth as i64),
            "proc_colors" => ConfigAttr::Bool(self.proc_colors),
            "proc_gradient" => ConfigAttr::Bool(self.proc_gradient),
            "proc_per_core" => ConfigAttr::Bool(self.proc_per_core),
            "proc_mem_bytes" => ConfigAttr::Bool(self.proc_mem_bytes),
            "check_temp" => ConfigAttr::Bool(self.check_temp),
            "cpu_sensor" => ConfigAttr::String(self.cpu_sensor.clone()),
            "show_coretemp" => ConfigAttr::Bool(self.show_coretemp),
            "draw_clock" => ConfigAttr::String(self.draw_clock.clone()),
            "background_update" => ConfigAttr::Bool(self.background_update),
            "custom_cpu_name" => ConfigAttr::String(self.custom_cpu_name.clone()),
            "disks_filter" => ConfigAttr::String(self.disks_filter.clone()),
            "mem_graphs" => ConfigAttr::Bool(self.mem_graphs),
            "show_swap" => ConfigAttr::Bool(self.show_swap),
            "swap_disk" => ConfigAttr::Bool(self.swap_disk),
            "show_disks" => ConfigAttr::Bool(self.show_disks),
            "net_download" => ConfigAttr::String(self.net_download.clone()),
            "net_upload" => ConfigAttr::String(self.net_upload.clone()),
            "net_auto" => ConfigAttr::Bool(self.net_auto),
            "net_sync" => ConfigAttr::Bool(self.net_sync),
            "net_color_fixed" => ConfigAttr::Bool(self.net_color_fixed),
            "show_battery" => ConfigAttr::Bool(self.show_battery),
            "show_init" => ConfigAttr::Bool(self.show_init),
            "update_check" => ConfigAttr::Bool(self.update_check),
            "log_level" => ConfigAttr::LogLevel(self.log_level),
        }
    }

    pub fn setattr_configattr(&mut self, attr: String, to_set: ConfigAttr) {
        match attr.as_str() {
            "color_theme" => {
                self.color_theme = match to_set {
                    ConfigAttr::String(s) => s.clone(),
                    _ => {
                        throw_error("Illegal attribute set in CONFIG");
                        String::default()
                    },
                }
            }
            "theme_background" => {
                self.theme_background = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "view_mode" => {
                self.view_mode = match to_set {
                    ConfigAttr::ViewMode(v) => v.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    ViewMode::None
                    },
                }
            }
            "update_ms" => {
                self.update_ms = match to_set {
                    ConfigAttr::Int64(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    0
                    },
                }
            }
            "proc_sorting" => {
                self.proc_sorting = match to_set {
                    ConfigAttr::SortingOption(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    SortingOption::Arguments
                    },
                }
            }
            "proc_reversed" => {
                self.proc_reversed = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "proc_tree" => {
                self.proc_tree = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "tree_depth" => {
                self.tree_depth = match to_set {
                    ConfigAttr::Int64(b) => b.clone() as i32,
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    0
                    },
                }
            }
            "proc_colors" => {
                self.proc_colors = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "proc_gradient" => {
                self.proc_gradient = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "proc_per_core" => {
                self.proc_per_core = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "proc_mem_bytes" => {
                self.proc_mem_bytes = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "check_temp" => {
                self.check_temp = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "cpu_sensor" => {
                self.cpu_sensor = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "show_coretemp" => {
                self.show_coretemp = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                        throw_error("Illegal attribute set in CONFIG");
                        false
                        },
                }
            }
            "draw_clock" => {
                self.draw_clock = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "background_update" => {
                self.background_update = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "custom_cpu_name" => {
                self.custom_cpu_name = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "disks_filter" => {
                self.disks_filter = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "mem_graphs" => {
                self.mem_graphs = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "show_swap" => {
                self.show_swap = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                throw_error("Illegal attribute set in CONFIG");
                false
                },
                }
            }
            "swap_disk" => {
                self.swap_disk = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "show_disks" => {
                self.show_disks = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "net_download" => {
                self.net_download = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "net_upload" => {
                self.net_upload = match to_set {
                    ConfigAttr::String(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    String::default()
                    },
                }
            }
            "net_auto" => {
                self.net_auto = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "net_sync" => {
                self.net_sync = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "net_color_fixed" => {
                self.net_color_fixed = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "show_battery" => {
                self.show_battery = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "show_init" => {
                self.show_init = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "update_check" => {
                self.update_check = match to_set {
                    ConfigAttr::Bool(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    false
                    },
                }
            }
            "log_level" => {
                self.log_level = match to_set {
                    ConfigAttr::LogLevel(b) => b.clone(),
                    _ => {
                    throw_error("Illegal attribute set in CONFIG");
                    LogLevel::Debug
                    },
                }
            }
        }
    }

    pub fn save_config(&mut self) -> std::io::Result<String> {
        if !self.changed && !self.recreate {
            return Ok("Nothing needs to be changed".into());
        }

        let vals: HashMap<String, String> = self
            .conf_dict
            .iter()
            .map(|(key, value)| (key.clone(), value.clone().to_string()))
            .collect();
        write(
            self.config_file.clone(),
            crate::DEFAULT_CONF.render(
                &vals
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect(),
            ),
        )?;
        Ok("Saved Successfully".into())
    }
}

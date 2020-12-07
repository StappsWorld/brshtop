use std::collections::*;
use std::path::*;
use psutil::*;
use psutil::sensors::*;

pub enum ConfigItem {
    Str(String),
    Int(i64),
    Bool(bool),
}

pub enum ViewMode {
    Full,
    Proc,
    Stat,
}
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
}

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
    config_file: String,
    recreate: bool,
    sorting_options: Vec<SortingOption>,
    log_levels: Vec<LogLevel>,
    view_modes: Vec<ViewMode>,
    cpu_sensors: Vec<String>,
    _initialized: bool,
} impl Config {

    pub fn new( path : PathBuf) -> Self {

        let mut cpu_sensors_mut : Vec::<String> = vec!["Auto"].iter().map(|s| s.to_string()).collect();

        /*if hasattr(psutil, "sensors_temperatures"):
                try:
                    _temps = psutil.sensors_temperatures()
                    if _temps:
                        for _name, _entries in _temps.items():
                            for _num, _entry in enumerate(_entries, 1):
                                if hasattr(_entry, "current"):
                                    cpu_sensors.append("{_name}:{_num if _entry.label == "" else _entry.label}")
                except:
                    pass
        */
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
        
        Config {
            keys: keys_unconverted.iter().map(|s| s.to_string()).collect(),
            conf_dict: HashMap::<String, ConfigItem>::new(),
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
            config_file: String::from(""),
            _initialized: false,
        }

    }

}
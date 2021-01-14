mod banner;
mod brshtop;
mod brshtop_box;
mod collector;
mod config;
mod consts;
mod cpubox;
mod cpucollector;
mod draw;
mod error;
mod event;
mod fx;
mod graph;
mod init;
mod key;
mod membox;
mod memcollector;
mod menu;
mod meter;
mod mv;
mod netbox;
mod netcollector;
mod nonblocking;
mod procbox;
mod proccollector;
mod raw;
mod subbox;
mod symbol;
mod term;
mod theme;
mod timer;
mod timeit;
mod updatechecker;

use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox, SubBoxes},
        collector::Collector,
        draw::Draw,
        key::Key,
        term::Term,
    },
    clap::{App, Arg},
    config::{Config, ViewMode},
    consts::*,
    cpuid,
    error::{errlog, throw_error},
    expanduser::expanduser,
    lazy_static::lazy_static,
    math::round,
    std::{
        collections::HashMap,
        env, fs,
        fs::{metadata, File},
        io::{prelude::*, BufReader},
        path::{Path, PathBuf},
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
    theme::{Color, Theme},
};

lazy_static! {
    static ref CONFIG_DIR: &'static Path = &Path::new(".");
    static ref SYSTEM: String = String::default();
    static ref CPU_NAME: String = match cpuid::identify() {
        Ok(info) => info.codename,
        Err(e) => {
            errlog(format!("Unable to get CPU name... (error {:?}", e));
            String::default()
        }
    };
    static ref UNITS : HashMap<String, Vec<String>> = HashMap::<String, Vec<String>>::new();
    static ref THREADS : u64 = 0;
    static ref VERSION : String = clap::crate_version!();
}

pub fn main() {
    let errors = Vec::<String>::new();

    let SELF_START = SystemTime::now();

    //Getting system information from env:consts:OS
    match env::consts::OS {
        "linux" => SYSTEM = String::from("Linux"),
        "netbsd" => SYSTEM = String::from("BSD"),
        "macos" => SYSTEM = String::from("MacOS"),
        &_ => SYSTEM = String::from("Other"),
    }

    if SYSTEM == "Other".to_owned() {
        print!("\nUnsupported platform!\n");
        std::process::exit(1);
    }

    //Argument Parsing
    let matches = App::new("brshtop")
        .version(VERSION)
        .author(
            ("Aristocratos (jakob@qvantnet.com)\n".to_owned()
                + "Samuel Rembisz <sjrembisz07@gmail.com)\n"
                + "Charlie Thomson <charliecthomson@gmail.com")
                .as_str(),
        )
        .about("A Rust implementation of a Python implementation of Bashtop")
        .arg(
            Arg::new("Full Mode")
                .short('f')
                .long("full")
                .takes_value(false)
                .about("Start in full mode showing all boxes [default]"),
        )
        .arg(
            Arg::new("Minimal Mode (proc)")
                .short('p')
                .long("proc")
                .takes_value(false)
                .about("Start in minimal mode without memory and net boxes"),
        )
        .arg(
            Arg::new("Minimal Mode (stat)")
                .short('s')
                .long("stat")
                .takes_value(false)
                .about("Start in minimal mode without process box"),
        )
        .arg(
            Arg::new("Version")
                .short('v')
                .long("version")
                .takes_value(false)
                .about("Show version info and exit"),
        )
        .arg(
            Arg::new("Debug")
                .long("debug")
                .takes_value(false)
                .about("Start with loglevel set to DEBUG overriding value set in config"),
        )
        .get_matches();

    let mut ARG_MODE = ViewMode::None;
    let arg_full = matches.value_of("Full Mode");
    let arg_proc = matches.value_of("Minimal Mode (proc)");
    let arg_stat = matches.value_of("Minimal Mode (stat)");
    let arg_version = matches.value_of("Version");
    let arg_debug = matches.value_of("Debug");

    if arg_full.is_some() {
        ARG_MODE = ViewMode::Full;
    } else if arg_proc.is_some() {
        ARG_MODE = ViewMode::Proc
    } else if arg_stat.is_some() {
        ARG_MODE = ViewMode::Stat;
    }

    let DEBUG = arg_debug.is_some();

    // Variables

    let config_dir_builder =
        expanduser("~").unwrap().to_str().unwrap().to_owned() + "/.config/brshtop";
    CONFIG_DIR = Path::new(config_dir_builder.as_str());

    if !CONFIG_DIR.exists() {
        match fs::create_dir(CONFIG_DIR) {
            Err(_) => throw_error(
                format!(
                    "ERROR!\nNo permission to write to \"{}\" directory!",
                    CONFIG_DIR.to_str().unwrap()
                )
                .as_str(),
            ),
            _ => (),
        }
        match fs::create_dir(CONFIG_DIR.join("themes")) {
            Err(_) => throw_error(
                format!(
                    "ERROR!\nNo permission to write to \"{}\" directory!",
                    CONFIG_DIR.join("themes").to_str().unwrap()
                )
                .as_str(),
            ),
            _ => (),
        }
    }

    let CONFIG_FILE = CONFIG_DIR.join("bpytop.conf");
    let mut EXECUTE_PATH = PathBuf::new();
    match std::env::current_exe() {
        Ok(p) => EXECUTE_PATH = p,
        Err(_) => throw_error("ERROR!\n Could not read this applications directory!"),
    }

    let theme_dir_builder = format!("{}/bpytop-themes", EXECUTE_PATH.to_str().unwrap());
    let theme_dir_check = Path::new(theme_dir_builder.as_str());
    let mut THEME_DIR;

    if theme_dir_check.exists() {
        THEME_DIR = theme_dir_check.clone();
    } else {
        let test_directories = vec!["/usr/local/", "/usr/", "/snap/bpytop/current/usr/"];

        for directory in test_directories {
            let test_directory_builder = directory.to_owned() + "share/bpytop/themes";
            let test_directory = Path::new(test_directory_builder.as_str());

            if test_directory.exists() {
                THEME_DIR = test_directory.clone();
                break;
            }
        }
    }

    let USER_THEME_DIR = CONFIG_DIR.join("themes");

    let CORES = psutil::cpu::cpu_count_physical();
    THREADS = psutil::cpu::cpu_count();

    let THREAD_ERROR = 0;

    let mut DEFAULT_THEME: HashMap<String, String> = [
        ("main_bg", ""),
        ("main_fg", "#cc"),
        ("title", "#ee"),
        ("hi_fg", "#969696"),
        ("selected_bg", "#7e2626"),
        ("selected_fg", "#ee"),
        ("inactive_fg", "#40"),
        ("graph_text", "#60"),
        ("meter_bg", "#40"),
        ("proc_misc", "#0de756"),
        ("cpu_box", "#3d7b46"),
        ("mem_box", "#8a882e"),
        ("net_box", "#423ba5"),
        ("proc_box", "#923535"),
        ("div_line", "#30"),
        ("temp_start", "#4897d4"),
        ("temp_mid", "#5474e8"),
        ("temp_end", "#ff40b6"),
        ("cpu_start", "#50f095"),
        ("cpu_mid", "#f2e266"),
        ("cpu_end", "#fa1e1e"),
        ("free_start", "#223014"),
        ("free_mid", "#b5e685"),
        ("free_end", "#dcff85"),
        ("cached_start", "#0b1a29"),
        ("cached_mid", "#74e6fc"),
        ("cached_end", "#26c5ff"),
        ("available_start", "#292107"),
        ("available_mid", "#ffd77a"),
        ("available_end", "#ffb814"),
        ("used_start", "#3b1f1c"),
        ("used_mid", "#d9626d"),
        ("used_end", "#ff4769"),
        ("download_start", "#231a63"),
        ("download_mid", "#4f43a3"),
        ("download_end", "#b0a9de"),
        ("upload_start", "#510554"),
        ("upload_mid", "#7d4180"),
        ("upload_end", "#dcafde"),
        ("process_start", "#80d0a3"),
        ("process_mid", "#dcd179"),
        ("process_end", "#d45454"),
    ]
    .iter()
    .cloned()
    .map(|(a, b)| (a.to_owned(), b.to_owned()))
    .collect();

    let mut MENUS = HashMap::new();

    let mut options_hash = HashMap::new();
    options_hash.insert(
        "normal",
        (
            "┌─┐┌─┐┌┬┐┬┌─┐┌┐┌┌─┐",
            "│ │├─┘ │ ││ ││││└─┐",
            "└─┘┴   ┴ ┴└─┘┘└┘└─┘",
        ),
    );
    options_hash.insert(
        "selected",
        (
            "╔═╗╔═╗╔╦╗╦╔═╗╔╗╔╔═╗",
            "║ ║╠═╝ ║ ║║ ║║║║╚═╗",
            "╚═╝╩   ╩ ╩╚═╝╝╚╝╚═╝",
        ),
    );
    MENUS.insert("options", options_hash);
    let mut help_hash = HashMap::new();
    help_hash.insert("normal", ("┬ ┬┌─┐┬  ┌─┐", "├─┤├┤ │  ├─┘", "┴ ┴└─┘┴─┘┴  "));
    help_hash.insert("selected", ("╦ ╦╔═╗╦  ╔═╗", "╠═╣║╣ ║  ╠═╝", "╩ ╩╚═╝╩═╝╩  "));
    MENUS.insert("help", help_hash);

    let mut quit_hash = HashMap::new();
    quit_hash.insert("normal", ("┌─┐ ┬ ┬ ┬┌┬┐", "│─┼┐│ │ │ │ ", "└─┘└└─┘ ┴ ┴ "));
    quit_hash.insert(
        "selected",
        ("╔═╗ ╦ ╦ ╦╔╦╗ ", "║═╬╗║ ║ ║ ║  ", "╚═╝╚╚═╝ ╩ ╩  "),
    );

    MENUS.insert("quit", quit_hash);
    let mut MENU_COLORS: HashMap<String, Vec<String>> = HashMap::<String, Vec<String>>::new();
    MENU_COLORS.insert(
        "normal".to_owned(),
        vec!["#0fd7ff", "#00bfe6", "#00a6c7", "#008ca8"]
            .iter()
            .map(|s| s.clone().to_owned())
            .collect::<Vec<String>>(),
    );
    MENU_COLORS.insert(
        "selected".to_owned(),
        vec!["#ffa50a", "#f09800", "#db8b00", "#c27b00"]
            .iter()
            .map(|s| s.clone().to_owned())
            .collect::<Vec<String>>(),
    );
    //Units for floating_humanizer function
    UNITS = vec![
        (
            "bit".to_owned(),
            [
                "bit", "Kib", "Mib", "Gib", "Tib", "Pib", "Eib", "Zib", "Yib", "Bib", "GEb",
            ].iter().map(|s| s.to_owned()).collect::<Vec<String>>()
        ),
        (
            "byte".to_owned(),
            [
                "Byte", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "BiB", "GEB",
            ].iter().map(|s| s.to_owned()).collect::<Vec<String>>()
        ),
    ].iter(|(s,v)| (s.clone(), v.iter().cloned().collect())).collect::<HashMap<String, Vec<String>>>();

    let CONFIG = match Config::new(CONFIG_FILE.clone()) {
        Ok(c) => c,
        Err(e) => {
            throw_error(e);
            Config::new(CONFIG_FILE.clone()).unwrap() //Never reached, but compiler is unhappy, so I bend
        }
    };

    errlog(format!(
        "New instance of brshtop version {} started with pid {}",
        VERSION,
        std::process::id()
    ));
    errlog(format!(
        "Loglevel set to {} (even though, currently, this doesn't work)",
        CONFIG.log_level
    ));

    let mut arg_output = String::new();
    for arg in env::args() {
        arg_output.push_str((arg + " ").as_str());
    }
    // errlog(CONFIG_DIR, format!("CMD: {}", arg_output));

    let mut b = brshtop::Brshtop::new();
    b._init(DEFAULT_THEME);
}

/// Defaults x: int = 0, y: int = 0, width: int = 0, height: int = 0, title: str = "", title2: str = "", line_color: Color = None, title_color: Color = None, fill: bool = True, box=None
pub fn create_box(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    title: Option<String>,
    title2: Option<String>,
    line_color: Option<Color>,
    title_color: Option<Color>,
    fill: bool,
    box_to_use: Option<Boxes>,
) -> String {
    String::default()
}

pub fn readfile(file: File) -> Option<String> {
    match file.metadata() {
        Ok(m) => {
            if m.is_file() {
                let mut out: String = String::new();
                let mut buf_reader = BufReader::new(file);

                match buf_reader.read_to_string(&mut out) {
                    Ok(_) => Some(out),
                    Err(e) => None,
                }
            } else {
                None
            }
        }
        Err(e) => None,
    }
}

pub fn min_max(value: i32, min_value: i32, max_value: i32) -> i32 {
    let min = if value > max_value { max_value } else { value };

    if min_value > min {
        min_value
    } else {
        min
    }
}

pub fn clean_quit(
    errcode: Option<i32>,
    errmsg: Option<String>,
    key: &mut Key,
    collector: &mut Collector,
    draw: &mut Draw,
    term: &mut Term,
    CONFIG: &mut Config,
    SELF_START: Option<SystemTime>,
) {
    key.stop();
    collector.stop();
    if errcode == None {
        CONFIG.save_config();
    }
    draw.now(
        vec![
            term.clear,
            term.normal_screen,
            term.show_cursor,
            term.mouse_off,
            term.mouse_direct_off,
            Term::title(String::default()),
        ],
        key,
    );
    Term::echo(true);
    let now = SystemTime::now();
    match errcode {
        Some(0) => errlog(
            CONFIG_DIR,
            format!(
                "Exiting, Runtime {} \n",
                now.duration_since(SELF_START.unwrap())
                    .unwrap()
                    .as_secs_f64()
            ),
        ),
        Some(n) => {
            errlog(
                CONFIG_DIR,
                format!(
                    "Exiting with errorcode {}, Runtime {} \n",
                    n,
                    now.duration_since(SELF_START.unwrap())
                        .unwrap()
                        .as_secs_f64()
                ),
            );
            print!(
                "Brshtop exted with errorcode ({}). See {}/error.log for more information!",
                errcode.unwrap(),
                CONFIG_DIR.to_string_lossy()
            );
        }
        None => (),
    };
    std::process::exit(errcode.unwrap_or(0));
}

pub fn first_letter_to_upper_case(s1: String) -> String {
    let mut c = s1.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Scales up in steps of 1024 to highest possible unit and returns string with unit suffixed. Defaults bit: bool = False, per_second: bool = False, start: int = 0, short: bool = False
pub fn floating_humanizer(
    value: f64,
    bit: bool,
    per_second: bool,
    start: usize,
    short: bool,
) -> String {

    let mut out : String = String::default();
    let mut mult : f64 = if bit {8.0} else {1.0};
    let mut selector : usize = start;
    let mut unit : Vec<String> = if bit {
        UNITS["bit".to_owned()]
    } else {
        UNITS["byte".to_owned()]
    };

    let mut working_val : f64 = f64::round(value * 100 * mult);
    if working_val < 0.0 {
        working_val = 0.0; 
    }

    let mut broke : bool = false;
    while working_val.to_string().len() > 5 && working_val >= 102400 {
        working_val >>= 10;
        if working_val < 100 {
            out = working_val.to_string();
            broke = true;
            break;
        }
        selector += 1;
    }
    if !broke {
        if working_val.to_string().len() == 4 && selector > 0 {
            out = working_val.to_string()[..working_val.to_string().len() - 3] + "." + working_val.to_string()[working_val.to_string().len() - 3];
        } else if working_val.to_string().len() == 3 && selector > 0 {
            out = working_val.to_string()[..working_val.to_string().len() - 3] + "." + working_val.to_string()[(working_val.to_string().len() - 3)..];
        } else if working_val.to_string().len() >= 2 {
            out = working_val.to_string()[..working_val.to_string().len() - 3];
        } else {
            out = working_val.to_string();
        }
    }

    if short {
        if out.contains('.') {
            out = f64::round(out.parse::<f64>()).to_string();
        }
        if out.len() > 3 {
            out = (out[0] as i64 + 1).to_string();
            selector += 1;
        }
    }
    out.push_str(format!("{}{}",
            if short {
                ""
            } else {
                " "
            },
            if short {
                unit[selector][0]
            } else {
                unit[selector]
            }
        )
        .as_str()
    );
    if per_second {
        out.push_str(if bit {"ps"} else {"/s"});
    }

    out
}

pub fn units_to_bytes(value : String) -> u64 {
    if value.len() == 0 {
        return 0;
    }
    let mut out : u32 = 0;
    let mut mult : u32 = 0;
    let mut bit : bool = false;
    let mut value_i : u64 = 0;
    let mut units : HashMap<String, u32> = HashMap::<String, u32>::new();
    if value.to_ascii_lowercase().ends_with('s') {
        value = value[..value.len() - 2];
    } 
    if value.to_ascii_lowercase().ends_with("bit") {
        bit = true;
        value = value[..value.len() - 4];
    } else if value.to_ascii_lowercase().ends_with("byte") {
        value = value[..value.len() - 5];
    }

    if units.contains_key(value[value.len() - 2].to_ascii_lowercase()) {
        mult = units.get(value[value.len() - 2].to_ascii_lowercase()).unwrap();
        value = value[..value.len() - 2];
    }

    if value.contains('.') && match value.replace(".", "").parse::<u64>() {
        Ok(_) => true,
        Err(_) => false,
    } {
        if mult > 0 {
            value_i = ((value.parse::<u64>() as f64) * 1024.0) as u64;
            mult -= 1;
        } else {
            value_i = value.parse::<u64>();
        }
    } else {
        match value.parse::<u64>() {
            Ok(u) => value_i = u,
            Err(_) => false,
        }
    }

    if bit {
        value_i = value_i / 8;
    }
    out = value_i << (10 * mult);

    out
}
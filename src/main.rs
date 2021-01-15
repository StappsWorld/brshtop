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
mod timeit;
mod timer;
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

#[macro_use]
lazy_static! {
    pub static ref CONFIG_DIR: &'static Path = {
        let config_dir_builder =
            expanduser("~").unwrap().to_str().unwrap().to_owned() + "/.config/brshtop";
        Path::new(config_dir_builder.as_str());
    };
    pub static ref SYSTEM: String = match env::consts::OS {
        "linux" => String::from("Linux"),
        "netbsd" => String::from("BSD"),
        "macos" => String::from("MacOS"),
        &_ => String::from("Other"),
    };
    pub static ref CPU_NAME: String = match cpuid::identify() {
        Ok(info) => info.codename,
        Err(e) => {
            errlog(format!("Unable to get CPU name... (error {:?}", e));
            String::default()
        }
    };
    pub static ref UNITS: HashMap<String, Vec<String>> = vec![
        (
            "bit".to_owned(),
            ["bit", "Kib", "Mib", "Gib", "Tib", "Pib", "Eib", "Zib", "Yib", "Bib", "GEb",]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>(),
        ),
        (
            "byte".to_owned(),
            ["Byte", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "BiB", "GEB",]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>(),
        ),
    ]
    .iter()
    .map(|(s, v)| (s.clone(), v.iter().cloned().collect()))
    .collect::<HashMap<String, Vec<String>>>();
    pub static ref THREADS: u64 = psutil::cpu::cpu_count();
    pub static ref VERSION: String = clap::crate_version!().to_owned();
}

pub fn main() {
    let errors = Vec::<String>::new();

    let SELF_START = SystemTime::now();

    //Getting system information from env:consts:OS

    if SYSTEM.to_string() == "Other".to_owned() {
        print!("\nUnsupported platform!\n");
        std::process::exit(1);
    }

    //Argument Parsing
    let matches = App::new("brshtop")
        .version(VERSION.as_str())
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

    if !CONFIG_DIR.exists() {
        match fs::create_dir(CONFIG_DIR.to_path_buf()) {
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

    let CONFIG = match Config::new(CONFIG_FILE.clone()) {
        Ok(c) => c,
        Err(e) => {
            throw_error(e);
            Config::new(CONFIG_FILE.clone()).unwrap() //Never reached, but compiler is unhappy, so I bend
        }
    };

    errlog(format!(
        "New instance of brshtop version {} started with pid {}",
        VERSION.to_owned(),
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
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    title: Option<String>,
    title2: Option<String>,
    line_color: Option<Color>,
    title_color: Option<Color>,
    fill: bool,
    box_to_use: Option<Boxes>,
    term: &mut Term,
    THEME: &mut Theme,
) -> String {
    let mut out: String = format!("{}{}", term.fg, term.bg);
    let mut lc: Color = match line_color {
        Some(c) => c,
        None => THEME.colors.div_line,
    };
    let mut tc: Color = match title_color {
        Some(c) => c,
        None => THEME.colors.title,
    };

    let mut wx: u32 = x;
    let mut wy: u32 = y;
    let mut ww: u32 = width;
    let mut wh: u32 = height;
    let mut wt: String = match title {
        Some(s) => s.clone(),
        None => String::default(),
    };
    // * Get values from box class if given
    match box_to_use {
        Some(o) => match o {
            Boxes::BrshtopBox(b) => {
                wx = b.x;
                wy = b.y;
                ww = b.width;
                wh = b.height;
                wt = b.name.clone();
            }
            Boxes::CpuBox(b) => {
                wx = b.x;
                wy = b.y;
                ww = b.parent.width;
                wh = b.parent.height;
                wt = b.name.clone();
            }
            Boxes::MemBox(b) => {
                wx = b.x as u32;
                wy = b.y as u32;
                ww = b.parent.width;
                wh = b.parent.height;
                wt = b.name.clone();
            }
            Boxes::NetBox(b) => {
                wx = b.x as u32;
                wy = b.y as u32;
                ww = b.parent.width;
                wh = b.parent.height;
                wt = b.name.clone();
            }
            Boxes::ProcBox(b) => {
                wx = b.parent.x;
                wy = b.parent.y;
                ww = b.parent.width;
                wh = b.parent.height;
                wt = b.name.clone();
            }
        },
        None => (),
    };
    let hlines: Vec<u32> = vec![wy, wy + wh - 1];

    out.push_str(lc.to_string().as_str());

    // * Draw all horizontal lines
    for hpos in hlines {
        out.push_str(
            format!(
                "{}{}",
                mv::to(hpos, wx),
                symbol::h_line.repeat((ww - 1) as usize)
            )
            .as_str(),
        );
    }

    // * Draw all vertical lines and fill if enabled
    for hpos in hlines[0] + 1..hlines[1] {
        out.push_str(
            format!(
                "{}{}{}{}",
                mv::to(hpos, wx),
                symbol::v_line,
                if fill {
                    " ".repeat((ww - 2) as usize)
                } else {
                    mv::right(ww - 2)
                },
                symbol::v_line
            )
            .as_str(),
        );
    }

    // * Draw corners
    out.push_str(
        format!(
            "{}{}{}{}{}{}{}{}",
            mv::to(wy, wx),
            symbol::left_up,
            mv::to(wy, wx + ww - 1),
            symbol::right_up,
            mv::to(wy + wh - 1, wx),
            symbol::left_down,
            mv::to(wy + wh - 1, wx + ww - 1),
            symbol::right_down,
        )
        .as_str(),
    );

    // * Draw titles if enabled
    match title {
        Some(s) => out.push_str(
            format!(
                "{}{}{}{}{}{}{}{}",
                mv::to(wy, wx + 2),
                symbol::title_left,
                tc,
                fx::b,
                s,
                fx::ub,
                lc,
                symbol::title_right
            )
            .as_str(),
        ),
        None => (),
    };

    match title2 {
        Some(s) => {
            out.push_str(
                format!(
                    "{}{}{}{}{}{}{}{}",
                    mv::to(hlines[1], wx + 2),
                    symbol::title_left,
                    tc,
                    fx::b,
                    s,
                    fx::ub,
                    lc,
                    symbol::title_right,
                )
                .as_str(),
            );
            ()
        }
        None => (),
    }

    out
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
            format!(
                "Exiting, Runtime {} \n",
                now.duration_since(SELF_START.unwrap())
                    .unwrap()
                    .as_secs_f64()
            ),
        ),
        Some(n) => {
            errlog(
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
    let mut out: String = String::default();
    let mut mult: f64 = if bit { 8.0 } else { 1.0 };
    let mut selector: usize = start;
    let mut unit: Vec<String> = if bit {
        UNITS[&"bit".to_owned()]
    } else {
        UNITS[&"byte".to_owned()]
    };

    let mut working_val: i64 = f64::round(value * 100.0 * mult) as i64;
    if working_val < 0 {
        working_val = 0;
    }

    let mut broke: bool = false;
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
            out = working_val.to_string()[..working_val.to_string().len() - 3].to_owned()
                + "."
                + (working_val.to_string().as_bytes()[working_val.to_string().len() - 3] as char).to_string().as_str();
        } else if working_val.to_string().len() == 3 && selector > 0 {
            out = working_val.to_string()[..working_val.to_string().len() - 3].to_string()
                + "."
                + working_val.to_string()[(working_val.to_string().len() - 3)..].to_string().as_str();
        } else if working_val.to_string().len() >= 2 {
            out = working_val.to_string()[..working_val.to_string().len() - 3].to_owned();
        } else {
            out = working_val.to_string();
        }
    }

    if short {
        if out.contains('.') {
            out = f64::round(out.parse::<f64>().unwrap()).to_string();
        }
        if out.len() > 3 {
            out = ((out.as_bytes()[0] as char).to_string().parse::<i64>().unwrap() + 1).to_string();
            selector += 1;
        }
    }
    out.push_str(
        format!(
            "{}{}",
            if short { "" } else { " " },
            if short {
                (unit[selector].as_bytes()[0] as char).to_string()
            } else {
                unit[selector]
            }
        )
        .as_str(),
    );
    if per_second {
        out.push_str(if bit { "ps" } else { "/s" });
    }

    out
}

pub fn units_to_bytes(value: String) -> u64 {
    if value.len() == 0 {
        return 0;
    }
    let mut out: u64 = 0;
    let mut mult: u32 = 0;
    let mut bit: bool = false;
    let mut value_i: u64 = 0;
    let mut units: HashMap<String, u32> = HashMap::<String, u32>::new();
    if value.to_ascii_lowercase().ends_with('s') {
        value = value[..value.len() - 2].to_owned();
    }
    if value.to_ascii_lowercase().ends_with("bit") {
        bit = true;
        value = value[..value.len() - 4].to_owned();
    } else if value.to_ascii_lowercase().ends_with("byte") {
        value = value[..value.len() - 5].to_owned();
    }

    if units.contains_key(&(value.as_bytes()[value.len() - 2] as char).to_string().to_ascii_lowercase()) {
        mult = units
            .get(&(value.as_bytes()[value.len() - 2] as char).to_string().to_ascii_lowercase())
            .unwrap().to_owned();
        value = value[..value.len() - 2].to_owned();
    }

    if value.contains('.')
        && match value.replace(".", "").parse::<u64>() {
            Ok(_) => true,
            Err(_) => false,
        }
    {
        if mult > 0 {
            value_i = ((value.parse::<u64>().unwrap() as f64) * 1024.0) as u64;
            mult -= 1;
        } else {
            value_i = value.parse::<u64>().unwrap();
        }
    } else {
        match value.parse::<u64>() {
            Ok(u) => value_i = u,
            Err(_) => (),
        }
    }

    if bit {
        value_i = value_i / 8;
    }
    out = value_i << (10 * mult);

    out
}

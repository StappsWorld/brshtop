mod banner;
mod brshtop;
mod config;
mod consts;
mod error;
mod event;
mod mv;
mod symbol;
mod term;
mod theme;
mod timeit;

use {
    config::Config,
    consts::*,
    error::{errlog, throw_error},
};

use clap::{App, Arg};
use expanduser::expanduser;
use log::LevelFilter;
use psutil::*;
use std::{
    collections::HashMap,
    env, fs,
    fs::metadata,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use string_template::*;
use theme::Theme;

pub fn main() {
    let mut b = brshtop::Brshtop::new();
    b._init();

    let errors = Vec::<String>::new();

    let SELF_START = SystemTime::now();

    //Getting system information from env:consts:OS
    let mut SYSTEM = String::new();
    match env::consts::OS {
        "linux" => SYSTEM = String::from("Linux"),
        "netbsd" => SYSTEM = String::from("BSD"),
        "macos" => SYSTEM = String::from("MacOS"),
        &_ => SYSTEM = String::from("Other"),
    }

    if SYSTEM == "Other" {
        print!("\nUnsupported platform!\n");
        std::process::exit(1);
    }

    //Argument Parsing
    let matches = App::new("brshtop")
        .version(clap::crate_version!())
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

    let mut ARG_MODE = String::new();
    let arg_full = matches.value_of("Full Mode");
    let arg_proc = matches.value_of("Minimal Mode (proc)");
    let arg_stat = matches.value_of("Minimal Mode (stat)");
    let arg_version = matches.value_of("Version");
    let arg_debug = matches.value_of("Debug");

    if arg_full.is_some() {
        ARG_MODE = String::from("full");
    } else if arg_proc.is_some() {
        ARG_MODE = String::from("proc");
    } else if arg_stat.is_some() {
        ARG_MODE = String::from("stat");
    }

    let DEBUG = arg_debug.is_some();

    let VERSION = clap::crate_version!();

    // Variables

    let config_dir_builder =
        expanduser("~").unwrap().to_str().unwrap().to_owned() + "/.config/brshtop";
    let CONFIG_DIR = Path::new(config_dir_builder.as_str());

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
    let THREADS = psutil::cpu::cpu_count();

    let THREAD_ERROR = 0;

    let mut DEFAULT_THEME: HashMap<&str, &str> = [
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
    let mut MENU_COLORS = HashMap::new();
    MENU_COLORS.insert("normal", ("#0fd7ff", "#00bfe6", "#00a6c7", "#008ca8"));
    MENU_COLORS.insert("selected", ("#ffa50a", "#f09800", "#db8b00", "#c27b00"));
    //Units for floating_humanizer function
    let mut UNITS = HashMap::new();
    UNITS.insert(
        "bit",
        (
            "bit", "Kib", "Mib", "Gib", "Tib", "Pib", "Eib", "Zib", "Yib", "Bib", "GEb",
        ),
    );
    UNITS.insert(
        "byte",
        (
            "Byte", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "BiB", "GEB",
        ),
    );

    let CONFIG = match Config::new(CONFIG_FILE.clone(), VERSION.to_owned()) {
        Ok(c) => c,
        Err(e) => {
            throw_error(e.to_string().as_str());
            Config::new(CONFIG_FILE.clone(), VERSION.to_owned()).unwrap() //Never reached, but compiler is unhappy, so I bend
        }
    };

    errlog(
        CONFIG_DIR,
        format!(
            "New instance of brshtop version {} started with pid {}",
            VERSION.to_owned(),
            std::process::id()
        ),
    );
    errlog(
        CONFIG_DIR,
        format!(
            "Loglevel set to {} (even though, currently, this doesn't work)",
            CONFIG.log_level
        ),
    );

    let mut arg_output = String::new();
    for arg in env::args() {
        arg_output.push_str((arg + " ").as_str());
    }
    // errlog(CONFIG_DIR, format!("CMD: {}", arg_output));
}

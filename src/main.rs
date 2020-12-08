mod config;
use config::*;
mod term;
use term::*;
mod timeit;
use timeit::*;
mod error;
use error::*;

use std::collections::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::*;
use clap::{Arg, App};
use psutil::*;
use string_template::*;
use expanduser::expanduser;
use std::fs::metadata;
use std::path::*;
use log::LevelFilter;



pub fn main() {
    let errors = vec::Vec::<String>::new();

    let SELF_START = SystemTime::now();

    //Getting system information from env:consts:OS
    let mut SYSTEM = String::new();
    match env::consts::OS {
        "linux" => SYSTEM = String::from("Linux"),
        "netbsd" => SYSTEM = String::from("BSD"),
        "macos" => SYSTEM = String::from("MacOS"),
        &_ => SYSTEM = String::from("Other")
    }

    if SYSTEM == "Other"{
        print!("\nUnsupported platform!\n");
        std::process::exit(1);
    }

    //Argument Parsing
    let matches = App::new("brshtop")
    .version(clap::crate_version!())
    .author(("Aristocratos (jakob@qvantnet.com)\n".to_owned() +
        "Samuel Rembisz <sjrembisz07@gmail.com)" +
        "Charlie Thomson <charliecthomson@gmail.com").as_str())
    .about("A Rust implementation of a Python implementation of Bashtop")
    .arg(Arg::new("Full Mode")
            .short('f')
            .long("full")
            .takes_value(false)
            .about("Start in full mode showing all boxes [default]"))
    .arg(Arg::new("Minimal Mode (proc)")
            .short('p')
            .long("proc")
            .takes_value(false)
            .about("Start in minimal mode without memory and net boxes"))
    .arg(Arg::new("Minimal Mode (stat)")
            .short('s')
            .long("stat")
            .takes_value(false)
            .about("Start in minimal mode without process box"))
    .arg(Arg::new("Version")
            .short('v')
            .long("version")
            .takes_value(false)
            .about("Show version info and exit"))
    .arg(Arg::new("Debug")
            .long("debug")
            .takes_value(false)
            .about("Start with loglevel set to DEBUG overriding value set in config"))
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
    } else if arg_stat.is_some(){
        ARG_MODE = String::from("stat");
    }

    let DEBUG = arg_debug.is_some();

    let VERSION = clap::crate_version!();

    // Variables

    let BANNER_SRC = vec![
	("#ffa50a", "#0fd7ff", "██████╗ ██████╗ ███████╗██╗  ██╗████████╗ ██████╗ ██████╗"),
	("#f09800", "#00bfe6", "██╔══██╗██╔══██╗██╔════╝██║  ██║╚══██╔══╝██╔═══██╗██╔══██╗"),
	("#db8b00", "#00a6c7", "██████╔╝██████╔╝███████╗███████║   ██║   ██║   ██║██████╔╝"),
	("#c27b00", "#008ca8", "██╔══██╗██╔══██╗╚════██║██╔══██║   ██║   ██║   ██║██╔═══╝"),
	("#a86b00", "#006e85", "██████╔╝██║  ██║███████║██║  ██║   ██║   ╚██████╔╝██║"),
	("#000000", "#000000", "╚═════╝ ╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝"),
    ];

    let DEFAULT_CONF = string_template::Template::new("#? Config file for bpytop v. {{version}}

    #* Color theme, looks for a .theme file in \"/usr/[local/]share/bpytop/themes\" and \"~/.config/bpytop/themes\", \"Default\" for builtin default theme.
    #* Prefix name by a plus sign (+) for a theme located in user themes folder, i.e. color_theme=\"+monokai\"
    color_theme=\"{{color_theme}}\"

    #* If the theme set background should be shown, set to False if you want terminal background transparency
    theme_background={{theme_background}}

    #* Set bpytop view mode, \"full\" for everything shown, \"proc\" for cpu stats and processes, \"stat\" for cpu, mem, disks and net stats shown.
    view_mode={{view_mode}}

    #* Update time in milliseconds, increases automatically if set below internal loops processing time, recommended 2000 ms or above for better sample times for graphs.
    update_ms={{update_ms}}

    #* Processes sorting, \"pid\" \"program\" \"arguments\" \"threads\" \"user\" \"memory\" \"cpu lazy\" \"cpu responsive\",
    #* \"cpu lazy\" updates top process over time, \"cpu responsive\" updates top process directly.
    proc_sorting=\"{{proc_sorting}}\"

    #* Reverse sorting order, True or False.
    proc_reversed={{proc_reversed}}

    #* Show processes as a tree
    proc_tree={{proc_tree}}

    #* Which depth the tree view should auto collapse processes at
    tree_depth={{tree_depth}}

    #* Use the cpu graph colors in the process list.
    proc_colors={{proc_colors}}

    #* Use a darkening gradient in the process list.
    proc_gradient={{proc_gradient}}

    #* If process cpu usage should be of the core it's running on or usage of the total available cpu power.
    proc_per_core={{proc_per_core}}

    #* Show process memory as bytes instead of percent
    proc_mem_bytes={{proc_mem_bytes}}

    #* Check cpu temperature, needs \"osx-cpu-temp\" on MacOS X.
    check_temp={{check_temp}}

    #* Which sensor to use for cpu temperature, use options menu to select from list of available sensors.
    cpu_sensor={{cpu_sensor}}

    #* Show temperatures for cpu cores also if check_temp is True and sensors has been found
    show_coretemp={{show_coretemp}}

    #* Draw a clock at top of screen, formatting according to strftime, empty string to disable.
    draw_clock=\"{{draw_clock}}\"

    #* Update main ui in background when menus are showing, set this to false if the menus is flickering too much for comfort.
    background_update={{background_update}}

    #* Custom cpu model name, empty string to disable.
    custom_cpu_name=\"{{custom_cpu_name}}\"

    #* Optional filter for shown disks, should be last folder in path of a mountpoint, \"root\" replaces \"/\", separate multiple values with comma.
    #* Begin line with \"exclude=\" to change to exclude filter, oterwise defaults to \"most include\" filter. Example: disks_filter=\"exclude=boot, home\"
    disks_filter=\"{{disks_filter}}\"

    #* Show graphs instead of meters for memory values.
    mem_graphs={{mem_graphs}}

    #* If swap memory should be shown in memory box.
    show_swap={{show_swap}}

    #* Show swap as a disk, ignores show_swap value above, inserts itself after first disk.
    swap_disk={{swap_disk}}

    #* If mem box should be split to also show disks info.
    show_disks={{show_disks}}

    #* Set fixed values for network graphs, default \"10M\" = 10 Mibibytes, possible units \"K\", \"M\", \"G\", append with \"bit\" for bits instead of bytes, i.e \"100mbit\"
    net_download=\"{{net_download}}\"
    net_upload=\"{{net_upload}}\"

    #* Start in network graphs auto rescaling mode, ignores any values set above and rescales down to 10 Kibibytes at the lowest.
    net_auto={{net_auto}}

    #* Sync the scaling for download and upload to whichever currently has the highest scale
    net_sync={{net_sync}}

    #* If the network graphs color gradient should scale to bandwith usage or auto scale, bandwith usage is based on \"net_download\" and \"net_upload\" values
    net_color_fixed={{net_color_fixed}}

    #* Show battery stats in top right if battery is present
    show_battery={{show_battery}}

    #* Show init screen at startup, the init screen is purely cosmetical
    show_init={{show_init}}

    #* Enable check for new version from github.com/aristocratos/bpytop at start.
    update_check={{update_check}}

    #* Set loglevel for \"~/.config/bpytop/error.log\" levels are: \"ERROR\" \"WARNING\" \"INFO\" \"DEBUG\".
    #* The level set includes all lower levels, i.e. \"DEBUG\" will show all logging info.
    log_level={{log_level}}
    ");

    let config_dir_builder = expanduser("~").unwrap().to_str().unwrap().to_owned() + "/.config/brshtop";
    let CONFIG_DIR = Path::new(config_dir_builder.as_str());

    if !CONFIG_DIR.exists() {
        match fs::create_dir(CONFIG_DIR){
            Err(_) => throw_error(format!("ERROR!\nNo permission to write to \"{}\" directory!", CONFIG_DIR.to_str().unwrap()).as_str()),
            _ => (),
        }
        match fs::create_dir(CONFIG_DIR.join("themes")){
            Err(_) => throw_error(format!("ERROR!\nNo permission to write to \"{}\" directory!", CONFIG_DIR.join("themes").to_str().unwrap()).as_str()),
            _ => (),
        }
    }

    let CONFIG_FILE = CONFIG_DIR.join("bpytop.conf");
    
    let mut EXECUTE_PATH = PathBuf::new();
    match std::env::current_exe() {
        Ok(p) => EXECUTE_PATH = p,
        Err(_) => throw_error("ERROR!\n Could not read this applications directory!")
    }

    let theme_dir_builder = format!("{}/bpytop-themes", EXECUTE_PATH.to_str().unwrap());
    let theme_dir_check = Path::new(theme_dir_builder.as_str());
    let mut THEME_DIR;

    if theme_dir_check.exists(){
        THEME_DIR = theme_dir_check.clone();
    } else {
        let test_directories = vec!["/usr/local/", "/usr/", "/snap/bpytop/current/usr/"];

        for directory in test_directories {
            let test_directory_builder = directory.to_owned() + "share/bpytop/themes";
            let test_directory = Path::new(test_directory_builder.as_str());

            if test_directory.exists(){
                THEME_DIR = test_directory.clone();
                break;
            }
        }

    }

    let USER_THEME_DIR = CONFIG_DIR.join("themes");


    let CORES = psutil::cpu::cpu_count_physical();
    let THREADS = psutil::cpu::cpu_count();

    let THREAD_ERROR = 0;

    let mut DEFAULT_THEME: HashMap<&str, &str> = 
    [
        ("main_bg" , ""),
        ("main_fg" , "#cc"),
        ("title" , "#ee"),
        ("hi_fg" , "#969696"),
        ("selected_bg" , "#7e2626"),
        ("selected_fg" , "#ee"),
        ("inactive_fg" , "#40"),
        ("graph_text" , "#60"),
        ("meter_bg" , "#40"),
        ("proc_misc" , "#0de756"),
        ("cpu_box" , "#3d7b46"),
        ("mem_box" , "#8a882e"),
        ("net_box" , "#423ba5"),
        ("proc_box" , "#923535"),
        ("div_line" , "#30"),
        ("temp_start" , "#4897d4"),
        ("temp_mid" , "#5474e8"),
        ("temp_end" , "#ff40b6"),
        ("cpu_start" , "#50f095"),
        ("cpu_mid" , "#f2e266"),
        ("cpu_end" , "#fa1e1e"),
        ("free_start" , "#223014"),
        ("free_mid" , "#b5e685"),
        ("free_end" , "#dcff85"),
        ("cached_start" , "#0b1a29"),
        ("cached_mid" , "#74e6fc"),
        ("cached_end" , "#26c5ff"),
        ("available_start" , "#292107"),
        ("available_mid" , "#ffd77a"),
        ("available_end" , "#ffb814"),
        ("used_start" , "#3b1f1c"),
        ("used_mid" , "#d9626d"),
        ("used_end" , "#ff4769"),
        ("download_start" , "#231a63"),
        ("download_mid" , "#4f43a3"),
        ("download_end" , "#b0a9de"),
        ("upload_start" , "#510554"),
        ("upload_mid" , "#7d4180"),
        ("upload_end" , "#dcafde"),
        ("process_start" , "#80d0a3"),
        ("process_mid" , "#dcd179"),
        ("process_end" , "#d45454"),
    ].iter().cloned().collect();



    let mut MENUS = HashMap::new();

    let mut options_hash = HashMap::new();
        options_hash.insert("normal", (
            "┌─┐┌─┐┌┬┐┬┌─┐┌┐┌┌─┐",
            "│ │├─┘ │ ││ ││││└─┐",
            "└─┘┴   ┴ ┴└─┘┘└┘└─┘"));
        options_hash.insert("selected", (
            "╔═╗╔═╗╔╦╗╦╔═╗╔╗╔╔═╗",
            "║ ║╠═╝ ║ ║║ ║║║║╚═╗",
            "╚═╝╩   ╩ ╩╚═╝╝╚╝╚═╝"));
    MENUS.insert("options", options_hash);
        
    let mut help_hash = HashMap::new();
        help_hash.insert("normal", (
            "┬ ┬┌─┐┬  ┌─┐",
            "├─┤├┤ │  ├─┘",
            "┴ ┴└─┘┴─┘┴  "));
        help_hash.insert("selected", (
            "╦ ╦╔═╗╦  ╔═╗",
            "╠═╣║╣ ║  ╠═╝",
            "╩ ╩╚═╝╩═╝╩  "));
    
    MENUS.insert("help", help_hash);

    let mut quit_hash = HashMap::new();
        quit_hash.insert("normal", (
            "┌─┐ ┬ ┬ ┬┌┬┐",
            "│─┼┐│ │ │ │ ",
            "└─┘└└─┘ ┴ ┴ "));
        quit_hash.insert("selected", (
            "╔═╗ ╦ ╦ ╦╔╦╗ ",
            "║═╬╗║ ║ ║ ║  ",
            "╚═╝╚╚═╝ ╩ ╩  "));

    MENUS.insert("quit", quit_hash);
        
    let mut MENU_COLORS = HashMap::new();
    MENU_COLORS.insert("normal", ("#0fd7ff", "#00bfe6", "#00a6c7", "#008ca8"));
    MENU_COLORS.insert("selected", ("#ffa50a", "#f09800", "#db8b00", "#c27b00"));
    
    //Units for floating_humanizer function
    let mut UNITS = HashMap::new();
    UNITS.insert("bit", ("bit", "Kib", "Mib", "Gib", "Tib", "Pib", "Eib", "Zib", "Yib", "Bib", "GEb"));
    UNITS.insert("byte", ("Byte", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB", "BiB", "GEB"));
    

    let CONFIG = match Config::new(CONFIG_FILE.clone(), VERSION.to_owned()) {
        Ok(c) => c,
        Err(e) => {
            throw_error(e.to_string().as_str());
            Config::new(CONFIG_FILE.clone(), VERSION.to_owned()).unwrap() //Never reached, but compiler is unhappy, so I bend
        },
    };

    errlog(CONFIG_DIR, format!("New instance of brshtop version {} started with pid {}", VERSION.to_owned(), std::process::id()));
    errlog(CONFIG_DIR, format!("Loglevel set to {} (even though, currently, this doesn't work)", CONFIG.log_level));

    let mut arg_output = String::new();
    for arg in env::args() {
        arg_output.push_str((arg + " ").as_str());
    }
    errlog(CONFIG_DIR, format!("CMD: {}", arg_output));




}


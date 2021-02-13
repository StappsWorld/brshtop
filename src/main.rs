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
        collector::{Collector, Collectors},
        config::{Config, SortingOption, ViewMode, ViewModeEnum},
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        draw::Draw,
        event::EventEnum,
        fx::Fx,
        graph::Graphs,
        init::Init,
        key::Key,
        membox::MemBox,
        memcollector::MemCollector,
        menu::Menu,
        meter::Meters,
        netbox::NetBox,
        netcollector::NetCollector,
        procbox::ProcBox,
        proccollector::{ProcCollector, ProcCollectorDetails},
        term::Term,
        timeit::TimeIt,
        timer::Timer,
        updatechecker::UpdateChecker,
    },
    clap::{App, Arg},
    consts::*,
    cpuid, crossbeam,
    error::{errlog, throw_error},
    expanduser::expanduser,
    lazy_static::lazy_static,
    math::round,
    once_cell::sync::OnceCell,
    psutil::process::Signal,
    signal_hook::{consts::signal::*, iterator::Signals},
    std::{
        collections::HashMap,
        env, fs,
        fs::{metadata, File},
        io,
        io::{prelude::*, BufReader},
        mem::drop,
        ops::{Deref, DerefMut},
        path::{Path, PathBuf},
        process,
        sync::Mutex,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
    terminal_size::{terminal_size, Height, Width},
    theme::{Color, Theme},
};

#[macro_use]
lazy_static! {
    pub static ref CONFIG_DIR: PathBuf = Path::new((expanduser("~").unwrap().to_str().unwrap().to_owned() + "/.config/brshtop").as_str()).to_owned();
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
    pub static ref DEFAULT_THEME: HashMap<String, String> = vec![
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
    pub static ref EXECUTE_PATH : PathBuf = match std::env::current_exe() {
        Ok(p) => p.as_path().to_owned(),
        Err(_) => {
            throw_error("ERROR!\n Could not read this applications directory!");
            Path::new("").to_owned() //NEVER REACHED
        },
    };
    pub static ref THEME_DIR : PathBuf = {
        let mut theme_dir_str = EXECUTE_PATH.to_owned().to_str().unwrap().to_string();
        theme_dir_str.push_str("/bpytop-themes");
        let theme_dir_check = Path::new(&theme_dir_str);
        let mut out : PathBuf = PathBuf::default();
        if theme_dir_check.to_owned().exists() {
            out = theme_dir_check.to_owned();
        } else {
            let test_directories = vec!["/usr/local/", "/usr/", "/snap/bpytop/current/usr/"];
            let mut broke : bool = false;
            for directory in test_directories {
                let test_directory_builder = directory.to_owned() + "share/bpytop/themes";
                let test_directory = Path::new(test_directory_builder.as_str());

                if test_directory.exists() {
                    out = test_directory.to_owned();
                    broke = true;
                    break;
                }
            }
            if !broke {
                throw_error("Unable to find theme directory!!");
            }
        }
        out.to_owned()
    };
    pub static ref USER_THEME_DIR : PathBuf = CONFIG_DIR.to_owned().join("themes").as_path().to_owned();
    pub static ref CORES : u64 = psutil::cpu::cpu_count_physical();
    pub static ref CORE_MAP : Vec<i32> = get_cpu_core_mapping();
    pub static ref SELF_START : SystemTime = SystemTime::now();
}

pub fn main() {
    let errors = Vec::<String>::new();

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

    let mut ARG_MODE: ViewMode = ViewMode {
        t: ViewModeEnum::None,
    };
    let arg_full = matches.value_of("Full Mode");
    let arg_proc = matches.value_of("Minimal Mode (proc)");
    let arg_stat = matches.value_of("Minimal Mode (stat)");
    let arg_version = matches.value_of("Version");
    let arg_debug = matches.value_of("Debug");

    if arg_full.is_some() {
        ARG_MODE = ViewMode {
            t: ViewModeEnum::Full,
        };
    } else if arg_proc.is_some() {
        ARG_MODE = ViewMode {
            t: ViewModeEnum::Proc,
        }
    } else if arg_stat.is_some() {
        ARG_MODE = ViewMode {
            t: ViewModeEnum::Stat,
        };
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

    let THREAD_ERROR = 0;

    let mut MENUS = HashMap::new();

    let mut options_hash = HashMap::new();
    options_hash.insert(
        "normal".to_owned(),
        (
            "┌─┐┌─┐┌┬┐┬┌─┐┌┐┌┌─┐".to_owned(),
            "│ │├─┘ │ ││ ││││└─┐".to_owned(),
            "└─┘┴   ┴ ┴└─┘┘└┘└─┘".to_owned(),
        ),
    );
    options_hash.insert(
        "selected".to_owned(),
        (
            "╔═╗╔═╗╔╦╗╦╔═╗╔╗╔╔═╗".to_owned(),
            "║ ║╠═╝ ║ ║║ ║║║║╚═╗".to_owned(),
            "╚═╝╩   ╩ ╩╚═╝╝╚╝╚═╝".to_owned(),
        ),
    );
    MENUS.insert("options".to_owned(), options_hash);
    let mut help_hash = HashMap::new();
    help_hash.insert(
        "normal".to_owned(),
        (
            "┬ ┬┌─┐┬  ┌─┐".to_owned(),
            "├─┤├┤ │  ├─┘".to_owned(),
            "┴ ┴└─┘┴─┘┴  ".to_owned(),
        ),
    );
    help_hash.insert(
        "selected".to_owned(),
        (
            "╦ ╦╔═╗╦  ╔═╗".to_owned(),
            "╠═╣║╣ ║  ╠═╝".to_owned(),
            "╩ ╩╚═╝╩═╝╩  ".to_owned(),
        ),
    );
    MENUS.insert("help".to_owned(), help_hash);

    let mut quit_hash = HashMap::new();
    quit_hash.insert(
        "normal".to_owned(),
        (
            "┌─┐ ┬ ┬ ┬┌┬┐".to_owned(),
            "│─┼┐│ │ │ │ ".to_owned(),
            "└─┘└└─┘ ┴ ┴ ".to_owned(),
        ),
    );
    quit_hash.insert(
        "selected".to_owned(),
        (
            "╔═╗ ╦ ╦ ╦╔╦╗ ".to_owned(),
            "║═╬╗║ ║ ║ ║  ".to_owned(),
            "╚═╝╚╚═╝ ╩ ╩  ".to_owned(),
        ),
    );

    MENUS.insert("quit".to_owned(), quit_hash);
    let mut MENU_COLORS: HashMap<String, Vec<String>> = HashMap::<String, Vec<String>>::new();
    MENU_COLORS.insert(
        "normal".to_owned(),
        vec!["#0fd7ff", "#00bfe6", "#00a6c7", "#008ca8"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>(),
    );
    MENU_COLORS.insert(
        "selected".to_owned(),
        vec!["#ffa50a", "#f09800", "#db8b00", "#c27b00"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>(),
    );

    let mut CONFIG: Config = match Config::new(CONFIG_FILE.clone()) {
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
    b._init();

    let mut THEME: Theme = match Theme::from_file(THEME_DIR.to_owned().as_path()) {
        Ok(r) => match r {
            Ok(t) => t,
            Err(e) => {
                errlog(format!(
                    "Unable to read Theme in directory '{}' (error {}), falling back to default",
                    THEME_DIR.to_owned().to_str().unwrap(),
                    e
                ));
                Theme::default()
            }
        },
        Err(e) => {
            errlog(format!(
                "Unable to read Theme in directory '{}' (error {}), falling back to default",
                THEME_DIR.to_owned().to_str().unwrap(),
                e
            ));
            Theme::default()
        }
    };
    let mut mutex_THEME: Mutex<Theme> = Mutex::new(THEME);
    let mut THEME: OnceCell<Mutex<Theme>> = OnceCell::new();
    THEME.set(mutex_THEME);

    //println!("Made it through global variables");

    // Pre main ---------------------------------------------------------------------------------------------
    let mut term: Term = Term::new();

    let mut key: Key = Key::new();

    let mut draw: Draw = Draw::new();

    let mut brshtop_box: BrshtopBox = BrshtopBox::new(&CONFIG, ARG_MODE);

    let mut cpu_box: CpuBox = CpuBox::new(&brshtop_box, &CONFIG, ARG_MODE);

    let mut mem_box: MemBox = MemBox::new(&brshtop_box, &CONFIG, ARG_MODE);

    let mut net_box: NetBox = NetBox::new(&CONFIG, ARG_MODE, &brshtop_box);

    let mut proc_box: ProcBox = ProcBox::new(&brshtop_box, &CONFIG, ARG_MODE);

    let mut collector: Collector = Collector::new();

    let mut cpu_collector: CpuCollector = CpuCollector::new();

    let mut mem_collector: MemCollector = MemCollector::new(&mem_box);

    let mut net_collector: NetCollector = NetCollector::new(&net_box, &CONFIG);

    let mut proc_collector: ProcCollector = ProcCollector::new(&proc_box);

    let mut menu: Menu = Menu::new(MENUS, MENU_COLORS);

    let mut timer: Timer = Timer::new();

    let mut timeit: TimeIt = TimeIt::new();

    let mut init: Init = Init::new();

    let mut updatechecker: UpdateChecker = UpdateChecker::new();

    let mut collectors: Vec<Collectors> = vec![
        Collectors::MemCollector,
        Collectors::NetCollector,
        Collectors::ProcCollector,
        Collectors::CpuCollector,
    ];

    let mut boxes: Vec<Boxes> = vec![Boxes::CpuBox, Boxes::MemBox, Boxes::NetBox, Boxes::ProcBox];

    let mut graphs: Graphs = Graphs::default();

    let mut meters: Meters = Meters::default();

    //println!("Made it through pre-main");

    // Main -----------------------------------------------------------------------------------------------

    let term_size = terminal_size();
    match term_size {
        Some((Width(w), Height(h))) => {
            term.set_width(w);
            term.set_height(h);
        }
        None => error::throw_error("Unable to get size of terminal!"),
    };

    // Init ----------------------------------------------------------------------------------

    if DEBUG {
        timeit.start("Init".to_owned());
    }

    // Switch to alternate screen, clear screen, hide cursor, enable mouse reporting and disable input echo
    draw.now(
        vec![
            term.get_alt_screen(),
            term.get_clear(),
            term.get_hide_cursor(),
            term.get_mouse_on(),
            Term::title("BRShtop".to_owned()),
        ],
        &mut key,
    );

    Term::echo(false);

    term.refresh(
        vec![],
        boxes.clone(),
        &mut collector,
        &init,
        &cpu_box,
        &draw,
        true,
        &key,
        &mut menu,
        &brshtop_box,
        &timer,
        &CONFIG,
        &THEME,
        &cpu_collector,
        &mem_box,
        &net_box,
        &proc_box,
    );

    // Start a thread checking for updates while running init
    if CONFIG.update_check {
        updatechecker.run();
    }

    // Draw banner and init status
    if CONFIG.show_init && !init.resized {
        init.start(&passable_draw, &passable_key, &passable_term);
    }

    // Load theme
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Loading theme and creating colors... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }

    THEME = match Theme::from_str(init_CONFIG.color_theme.clone()) {
        Ok(t) => {
            drop(init_CONFIG);
            init_init.success(
                &CONFIG,
                &passable_draw,
                &passable_term,
                &passable_key,
            );
            t
        }
        Err(e) => {
            errlog(format!("Unable to read theme from config (error {})...", e));
            drop(init_CONFIG);
            Init::fail(
                e,
                &CONFIG,
                &passable_draw,
                &passable_collector,
                &passable_key,
                &passable_term,
            );
            Theme::default()
        }
    };
    init_CONFIG = CONFIG;

    // Setup boxes
    if init_CONFIG.show_init {
        println!("Showing init");
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Doing some maths and drawing... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
        if init_CONFIG.check_temp {
            let mut init_cpu_collector = cpu_collector;

            drop(init_CONFIG);
            init_cpu_collector.get_sensors(&CONFIG);
            init_CONFIG = CONFIG;
        }
        let mut init_brshtop_box = brshtop_box;

        drop(init_CONFIG);
        init_brshtop_box.calc_sizes(
            boxes.clone(),
            &passable_term,
            &CONFIG,
            &cpu_collector,
            &passable_cpu_box,
            &mem_box,
            &net_box,
            &proc_box,
        );
        init_brshtop_box.draw_bg(
            false,
            &passable_draw,
            boxes.clone(),
            &init_menu,
            &CONFIG,
            &passable_cpu_box,
            &mem_box,
            &net_box,
            &proc_box,
            &passable_key,
            &THEME,
            &passable_term,
        );
        init_init.success(
            &CONFIG,
            &passable_draw,
            &passable_term,
            &passable_key,
        );
    }

    // Setup signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH
    init_CONFIG = CONFIG;
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Setting up signal handlers... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }

    drop(init_CONFIG);
    let mut signals = match Signals::new(&[SIGTSTP, SIGCONT, SIGINT, SIGWINCH]) {
        //Handling ctrl-z, resume, ctrl-c, terminal resized
        Ok(s) => s,
        Err(e) => {
            Init::fail(
                e.to_string(),
                &CONFIG,
                &passable_draw,
                &passable_collector,
                &passable_key,
                &passable_term,
            );
            return;
        }
    };
    drop(init_menu);
    match crossbeam::scope(|s| {
        s.spawn(|_| {
            for sig in signals.forever() {
                match sig {
                    SIGTSTP => match now_sleeping(
                        &passable_key,
                        &passable_collector,
                        &passable_draw,
                        &passable_term,
                    ) {
                        Some(_) => (),
                        None => clean_quit(
                            None,
                            Some("Failed to pause program".to_owned()),
                            &passable_key,
                            &passable_collector,
                            &passable_draw,
                            &passable_term,
                            &CONFIG,
                        ),
                    },
                    SIGCONT => now_awake(
                        &passable_draw,
                        &passable_term,
                        &passable_key,
                        &brshtop_box,
                        &passable_collector,
                        boxes.clone(),
                        &passable_init,
                        &passable_cpu_box,
                        &passable_menu,
                        &timer,
                        &CONFIG,
                        &THEME,
                        DEBUG,
                        collectors.clone(),
                        &passable_timeit,
                        ARG_MODE,
                        &passable_graphs,
                        &passable_meters,
                        &net_box,
                        &proc_box,
                        &mem_box,
                        &cpu_collector,
                        &passable_mem_collector,
                        &passable_net_collector,
                        &passable_proc_collector,
                    ),
                    SIGINT => clean_quit(
                        None,
                        None,
                        &passable_key,
                        &passable_collector,
                        &passable_draw,
                        &passable_term,
                        &CONFIG,
                    ),
                    SIGWINCH => {
                        let mut SIG_term = passable_term;
                        let mut SIG_menu = passable_menu;
                        SIG_term.refresh(
                            vec![],
                            boxes.clone(),
                            &passable_collector,
                            &passable_init,
                            &passable_cpu_box,
                            &passable_draw,
                            true,
                            &passable_key,
                            &mut SIG_menu,
                            &brshtop_box,
                            &timer,
                            &CONFIG,
                            &THEME,
                            &cpu_collector,
                            &mem_box,
                            &net_box,
                            &proc_box,
                        );
                        drop(SIG_term);
                    }
                    _ => unreachable!(),
                }
            }
        });
    }) {
        _ => (),
    };
    init_menu = passable_menu;

    init_init.success(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Start a separate thread for reading keyboard input
    init_CONFIG = CONFIG;
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Starting input reader thread... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }
    let mut init_key = passable_key;

    init_key.start(&passable_draw, &passable_menu);

    drop(init_CONFIG);
    drop(init_key);
    init_init.success(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Start a separate thread for data collection and drawing
    init_CONFIG = CONFIG;
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Starting data collection and drawer thread... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }

    let mut init_collector = passable_collector;
    init_collector.start(
        &CONFIG,
        DEBUG,
        collectors.clone(),
        &brshtop_box,
        &passable_timeit,
        &passable_menu,
        &passable_draw,
        &passable_term,
        &passable_cpu_box,
        &passable_key,
        &THEME,
        ARG_MODE,
        &passable_graphs,
        &passable_meters,
        &net_box,
        &proc_box,
        &mem_box,
        &cpu_collector,
        &passable_mem_collector,
        &passable_net_collector,
        &passable_proc_collector,
    );
    init_init = passable_init;
    init_init.success(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Collect data and draw to buffer
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Collecting data and drawing... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }
    drop(init_CONFIG);
    init_collector.collect(
        collectors.clone(),
        &CONFIG,
        false,
        false,
        false,
        false,
        false,
    );
    init_init.success(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Draw to screen
    init_CONFIG = CONFIG;
    if init_CONFIG.show_init {
        init_draw = passable_draw;
        init_draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}",
                mv::restore,
                Fx::trans("Finishing up... ".to_owned()),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            &passable_key,
        );
        drop(init_draw);
    }
    init_collector = passable_collector;
    init_collector.set_collect_done(EventEnum::Wait);
    init_collector.get_collect_done_reference().wait(-1.0);
    drop(init_CONFIG);
    init_init.success(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    init_init.done(
        &CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );
    init_term = passable_term;
    drop(init_init);
    drop(init_collector);
    init_term.refresh(
        vec![],
        boxes.clone(),
        &passable_collector,
        &passable_init,
        &passable_cpu_box,
        &passable_draw,
        false,
        &passable_key,
        &mut init_menu,
        &brshtop_box,
        &timer,
        &CONFIG,
        &THEME,
        &cpu_collector,
        &mem_box,
        &net_box,
        &proc_box,
    );
    init_key = passable_key;
    init_draw = passable_draw;
    init_draw.out(vec![], true, &mut init_key);
    drop(init_key);
    init_CONFIG = CONFIG;
    if init_CONFIG.draw_clock.len() > 0 {
        let mut init_brshtop_box = brshtop_box;
        init_brshtop_box.set_clock_on(true);
        drop(init_brshtop_box);
    }
    if DEBUG {
        let mut init_timeit = passable_timeit;
        init_timeit.stop("Init".to_owned());
        drop(init_timeit);
    }

    // Main loop ------------------------------------------------------------------------------------->

    drop(init_term);
    drop(init_draw);
    drop(init_menu);
    drop(init_CONFIG);
    drop(init_updatechecker);
    run(
        &passable_term,
        &passable_key,
        &timer,
        &passable_collector,
        boxes.clone(),
        &passable_init,
        &passable_cpu_box,
        &passable_draw,
        &passable_menu,
        &brshtop_box,
        &CONFIG,
        &THEME,
        &mut ARG_MODE,
        &proc_box,
        &passable_proc_collector,
        &passable_net_collector,
        &cpu_collector,
        &net_box,
        &passable_updatechecker,
        collectors.clone(),
        &passable_mem_collector,
        &passable_graphs,
        &mem_box,
    );
}

pub fn run(
    term_p: &OnceCell<Mutex<Term>>,
    key_p: &OnceCell<Mutex<Key>>,
    timer_p: &OnceCell<Mutex<Timer>>,
    collector_p: &OnceCell<Mutex<Collector>>,
    boxes: Vec<Boxes>,
    init_p: &OnceCell<Mutex<Init>>,
    cpu_box_p: &OnceCell<Mutex<CpuBox>>,
    draw_p: &OnceCell<Mutex<Draw>>,
    menu_p: &OnceCell<Mutex<Menu>>,
    brshtop_box_p: &OnceCell<Mutex<BrshtopBox>>,
    CONFIG_p: &OnceCell<Mutex<Config>>,
    THEME_p: &OnceCell<Mutex<Theme>>,
    ARG_MODE: &mut ViewMode,
    procbox_p: &OnceCell<Mutex<ProcBox>>,
    proccollector_p: &OnceCell<Mutex<ProcCollector>>,
    netcollector_p: &OnceCell<Mutex<NetCollector>>,
    cpucollector_p: &OnceCell<Mutex<CpuCollector>>,
    netbox_p: &OnceCell<Mutex<NetBox>>,
    update_checker_p: &OnceCell<Mutex<UpdateChecker>>,
    collectors: Vec<Collectors>,
    memcollector_p: &OnceCell<Mutex<MemCollector>>,
    graphs_p: &OnceCell<Mutex<Graphs>>,
    mem_box_p: &OnceCell<Mutex<MemBox>>,
) {
    loop {
        let mut term = term_p;
        let mut key = key_p;
        let mut timer = timer_p;
        let mut collector = collector_p;
        let mut init = init_p;
        let mut cpu_box = cpu_box_p;
        let mut draw = draw_p;
        let mut menu = menu_p;
        let mut brshtop_box = brshtop_box_p;
        let mut CONFIG = CONFIG_p;
        let mut THEME = THEME_p;
        let mut procbox = procbox_p;
        let mut proccollector = proccollector_p;
        let mut netcollector = netcollector_p;
        let mut cpucollector = cpucollector_p;
        let mut netbox = netbox_p;
        let mut update_checker = update_checker_p;
        let mut memcollector = memcollector_p;
        let mut graphs = graphs_p;
        let mut mem_box = mem_box_p;

        drop(collector);
        drop(init);
        drop(cpu_box);
        drop(draw);
        drop(key);
        drop(brshtop_box);
        drop(timer);
        drop(CONFIG);
        drop(THEME);
        drop(cpucollector);
        drop(mem_box);
        drop(netbox);
        drop(procbox);
        term.refresh(
            vec![],
            boxes.clone(),
            collector_p,
            init_p,
            cpu_box_p,
            draw_p,
            false,
            key_p,
            &mut menu,
            brshtop_box_p,
            timer_p,
            CONFIG_p,
            THEME_p,
            cpucollector_p,
            mem_box_p,
            netbox_p,
            procbox_p,
        );

        collector = collector_p;
        init = init_p;
        cpu_box = cpu_box_p;
        draw = draw_p;
        key = key_p;
        brshtop_box = brshtop_box_p;
        timer = timer_p;
        THEME = THEME_p;
        cpucollector = cpucollector_p;
        mem_box = mem_box_p;
        netbox = netbox_p;
        procbox = procbox_p;

        timer.stamp();

        drop(draw);
        drop(term);
        while timer.not_zero(&CONFIG_p) {
            if key.input_wait(timer.left(CONFIG_p).as_secs_f64(), false, draw_p, term_p) {
                drop(key);
                drop(procbox);
                drop(collector);
                drop(proccollector);
                drop(brshtop_box);
                drop(cpu_box);
                drop(menu);
                drop(THEME);
                drop(netcollector);
                drop(init);
                drop(cpucollector);
                drop(netbox);
                drop(update_checker);
                drop(timer);
                drop(memcollector);
                drop(graphs);
                drop(mem_box);
                process_keys(
                    ARG_MODE,
                    key_p,
                    procbox_p,
                    collector_p,
                    proccollector_p,
                    CONFIG_p,
                    draw_p,
                    term_p,
                    brshtop_box_p,
                    cpu_box_p,
                    menu_p,
                    THEME_p,
                    netcollector_p,
                    init_p,
                    cpucollector_p,
                    boxes.clone(),
                    netbox_p,
                    update_checker_p,
                    collectors.clone(),
                    timer_p,
                    memcollector_p,
                    graphs_p,
                    mem_box_p,
                    procbox_p,
                );
                key = key_p;
                procbox = procbox_p;
                collector = collector_p;
                proccollector = proccollector_p;
                draw = draw_p;
                term = term_p;
                brshtop_box = brshtop_box_p;
                cpu_box = cpu_box_p;
                menu = menu_p;
                THEME = THEME_p;
                netcollector = netcollector_p;
                init = init_p;
                cpucollector = cpucollector_p;
                netbox = netbox_p;
                update_checker = update_checker_p;
                timer = timer_p;
                memcollector = memcollector_p;
                graphs = graphs_p;
                mem_box = mem_box_p;
                procbox = procbox_p;
            }
        }

        collector.collect(
            collectors.clone(),
            CONFIG_p,
            true,
            false,
            false,
            false,
            false,
        );
        drop(key);
        drop(timer);
        drop(collector);
        drop(init);
        drop(cpu_box);
        drop(menu);
        drop(brshtop_box);
        drop(THEME);
        drop(procbox);
        drop(proccollector);
        drop(netcollector);
        drop(cpucollector);
        drop(netbox);
        drop(update_checker);
        drop(memcollector);
        drop(graphs);
        drop(mem_box);
    }
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
    term: &Term,
    THEME: &Theme,
    brshtop_box: Option<&OnceCell<Mutex<BrshtopBox>>>,
    cpu_box: Option<&CpuBox>,
    mem_box: Option<&MemBox>,
    net_box: Option<&NetBox>,
    proc_box: Option<&ProcBox>,
) -> String {
    let mut out: String = format!("{}{}", term.get_fg(), term.get_bg());
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
    let mut wt: String = match title.clone() {
        Some(s) => s.clone(),
        None => String::default(),
    };
    // * Get values from box class if given
    match box_to_use {
        Some(o) => match o {
            Boxes::BrshtopBox => {
                wx = brshtop_box.unwrap().get_x();
                wy = brshtop_box.unwrap().get_y();
                ww = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .get_width();
                wh = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .get_height();
                wt = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .get_name();
            }
            Boxes::CpuBox => {
                let parent_box = cpu_box.unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::MemBox => {
                let parent_box = mem_box.unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::NetBox => {
                let parent_box = net_box.unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::ProcBox => {
                let parent_box = proc_box.unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
        },
        None => (),
    };
    let hlines: Vec<u32> = vec![wy, wy + wh - 1];

    out.push_str(lc.to_string().as_str());

    // * Draw all horizontal lines
    for hpos in hlines.clone() {
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
    for hpos in hlines.clone()[0] + 1..hlines.clone()[1] {
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
    match title.clone() {
        Some(st) => out.push_str(
            format!(
                "{}{}{}{}{}{}{}{}",
                mv::to(wy, wx + 2),
                symbol::title_left,
                tc,
                fx::b,
                st.clone(),
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
    collector: &Collector,
    draw: &Draw,
    term: &Term,
    CONFIG: &Config,
) {
    key.stop();
    collector.stop();
    if errcode == None {
        CONFIG.save_config();
    }
    draw.now(
        vec![
            term.get_clear(),
            term.get_normal_screen(),
            term.get_show_cursor(),
            term.get_mouse_off(),
            term.get_mouse_direct_off(),
            Term::title(String::default()),
        ],
        &mut key,
    );
    Term::echo(true);
    let now = SystemTime::now();
    match errcode {
        Some(0) => errlog(format!(
            "Exiting, Runtime {} \n",
            now.duration_since(SELF_START.to_owned())
                .unwrap()
                .as_secs_f64()
        )),
        Some(n) => {
            errlog(format!(
                "Exiting with errorcode {}, Runtime {} \n",
                n,
                now.duration_since(SELF_START.to_owned())
                    .unwrap()
                    .as_secs_f64()
            ));
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
        UNITS.to_owned().get(&"bit".to_owned()).unwrap().to_owned()
    } else {
        UNITS.to_owned().get(&"byte".to_owned()).unwrap().to_owned()
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
                + (working_val.to_string().as_bytes()[working_val.to_string().len() - 3] as char)
                    .to_string()
                    .as_str();
        } else if working_val.to_string().len() == 3 && selector > 0 {
            out = working_val.to_string()[..working_val.to_string().len() - 3].to_string()
                + "."
                + working_val.to_string()[(working_val.to_string().len() - 3)..]
                    .to_string()
                    .as_str();
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
            out = ((out.as_bytes()[0] as char)
                .to_string()
                .parse::<i64>()
                .unwrap()
                + 1)
            .to_string();
            selector += 1;
        }
    }
    out.push_str(
        format!(
            "{}{}",
            if short { "" } else { " " },
            if short {
                (unit[selector].clone().as_bytes()[0] as char).to_string()
            } else {
                unit[selector].clone()
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

    let mut mutable_value: String = value.clone();
    if mutable_value.to_ascii_lowercase().ends_with('s') {
        mutable_value = mutable_value[..mutable_value.len() - 2].to_owned();
    }
    if mutable_value.to_ascii_lowercase().ends_with("bit") {
        bit = true;
        mutable_value = mutable_value[..mutable_value.len() - 4].to_owned();
    } else if mutable_value.to_ascii_lowercase().ends_with("byte") {
        mutable_value = mutable_value[..mutable_value.len() - 5].to_owned();
    }

    if units.contains_key(
        &(mutable_value.as_bytes()[mutable_value.len() - 2] as char)
            .to_string()
            .to_ascii_lowercase(),
    ) {
        mult = units
            .get(
                &(mutable_value.as_bytes()[mutable_value.len() - 2] as char)
                    .to_string()
                    .to_ascii_lowercase(),
            )
            .unwrap()
            .to_owned();
        mutable_value = mutable_value[..mutable_value.len() - 2].to_owned();
    }

    if mutable_value.contains('.')
        && match mutable_value.replace(".", "").parse::<u64>() {
            Ok(_) => true,
            Err(_) => false,
        }
    {
        if mult > 0 {
            value_i = ((mutable_value.parse::<u64>().unwrap() as f64) * 1024.0) as u64;
            mult -= 1;
        } else {
            value_i = mutable_value.parse::<u64>().unwrap();
        }
    } else {
        match mutable_value.parse::<u64>() {
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

pub fn process_keys<'a>(
    ARG_MODE: &mut ViewMode,
    key_class_p: &OnceCell<Mutex<Key>>,
    procbox_p: &OnceCell<Mutex<ProcBox>>,
    collector_p: &OnceCell<Mutex<Collector>>,
    proccollector_p: &OnceCell<Mutex<ProcCollector>>,
    CONFIG_p: &OnceCell<Mutex<Config>>,
    draw_p: &OnceCell<Mutex<Draw>>,
    term_p: &OnceCell<Mutex<Term>>,
    brshtop_box_p: &OnceCell<Mutex<BrshtopBox>>,
    cpu_box_p: &OnceCell<Mutex<CpuBox>>,
    menu_p: &OnceCell<Mutex<Menu>>,
    THEME_p: &OnceCell<Mutex<Theme>>,
    netcollector_p: &OnceCell<Mutex<NetCollector>>,
    init_p: &OnceCell<Mutex<Init>>,
    cpucollector_p: &OnceCell<Mutex<CpuCollector>>,
    boxes: Vec<Boxes>,
    netbox_p: &OnceCell<Mutex<NetBox>>,
    update_checker_p: &OnceCell<Mutex<UpdateChecker>>,
    collectors: Vec<Collectors>,
    timer_p: &OnceCell<Mutex<Timer>>,
    memcollector_p: &OnceCell<Mutex<MemCollector>>,
    graphs_p: &OnceCell<Mutex<Graphs>>,
    mem_box_p: &OnceCell<Mutex<MemBox>>,
    proc_box_p: &OnceCell<Mutex<ProcBox>>,
) {
    let mut key_class = key_class_p;
    let mut procbox = procbox_p;
    let mut collector = collector_p;
    let mut proccollector = proccollector_p;
    let mut CONFIG = CONFIG_p;
    let mut draw = draw_p;
    let mut term = term_p;
    let mut brshtop_box = brshtop_box_p;
    let mut cpu_box = cpu_box_p;
    let mut menu = menu_p;
    let mut THEME = THEME_p;
    let mut netcollector = netcollector_p;
    let mut init = init_p;
    let mut cpucollector = cpucollector_p;
    let mut netbox = netbox_p;
    let mut update_checker = update_checker_p;
    let mut timer = timer_p;
    let mut memcollector = memcollector_p;
    let mut graphs = graphs_p;
    let mut mem_box = mem_box_p;
    let mut proc_box = proc_box_p;

    let mut mouse_pos: (i32, i32) = (0, 0);
    let mut filtered: bool = false;
    while key_class.has_key() {
        let mut key = match key_class.get() {
            Some(k) => k.clone(),
            None => return,
        };
        if vec!["mouse_scroll_up", "mouse_scroll_down", "mouse_click"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            mouse_pos = key_class.get_mouse();
            if mouse_pos.0 >= procbox.get_parent().get_x() as i32
                && procbox.get_current_y() as i32 + 1 <= mouse_pos.1
                && mouse_pos.1 < procbox.get_current_y() as i32 + procbox.get_current_h() as i32 - 1
            {
                ()
            } else if key == "mouse_click".to_owned() {
                key = "mouse_unselect".to_owned()
            } else {
                key = "_null".to_owned()
            }
        }
        if procbox.get_filtering() {
            if vec!["enter", "mouse_click", "mouse_unselect"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
                .contains(&key)
            {
                procbox.set_filtering(false);
                collector.collect(
                    vec![Collectors::ProcCollector],
                    CONFIG_p,
                    true,
                    false,
                    false,
                    true,
                    true,
                );
                continue;
            } else if vec!["escape", "delete"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
                .contains(&key)
            {
                proccollector.search_filter = String::default();
                procbox.set_filtering(false);
            } else if key.len() == 1 {
                proccollector.search_filter.push_str(key.as_str());
            } else if key == "backspace".to_owned() && proccollector.search_filter.len() > 0 {
                proccollector.search_filter =
                    proccollector.search_filter[..proccollector.search_filter.len() - 2].to_owned();
            } else {
                continue;
            }
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                false,
                true,
                true,
                false,
            );
            if filtered {
                collector.set_collect_done(EventEnum::Wait);
                collector.get_collect_done_reference().wait(0.1);
                collector.set_collect_done(EventEnum::Flag(false));
            }
            filtered = true;
            continue;
        }

        CONFIG = CONFIG_p;
        if key == "_null".to_owned() {
            continue;
        } else if key == "q".to_owned() {
            drop(key_class);
            drop(collector);
            drop(draw);
            drop(term);
            drop(CONFIG);
            clean_quit(
                None,
                None,
                key_class_p,
                collector_p,
                draw_p,
                term_p,
                CONFIG_p,
            );
            key_class = key_class_p; // NEVER REACHED
            collector = collector_p; // NEVER REACHED
            draw = draw_p; // NEVER REACHED
            term = term_p; // NEVER REACHED
            CONFIG = CONFIG_p; // NEVER REACHED
        } else if key == "+" && CONFIG.update_ms + 100 <= 86399900 {
            CONFIG.update_ms += 100;
            drop(key_class);
            drop(collector);
            drop(draw);
            drop(term);
            drop(CONFIG);
            drop(cpu_box);
            drop(THEME);
            brshtop_box.draw_update_ms(
                false,
                CONFIG_p,
                cpu_box_p,
                key_class_p,
                draw_p,
                &menu,
                THEME_p,
                term_p,
            );
            key_class = key_class_p;
            collector = collector_p;
            draw = draw_p;
            term = term_p;
            CONFIG = CONFIG_p;
            cpu_box = cpu_box_p;
            THEME = THEME_p;
        } else if key == "-".to_owned() && CONFIG.update_ms - 100 >= 100 {
            CONFIG.update_ms -= 100;
            drop(key_class);
            drop(collector);
            drop(draw);
            drop(term);
            drop(CONFIG);
            drop(cpu_box);
            drop(THEME);
            brshtop_box.draw_update_ms(
                false,
                CONFIG_p,
                cpu_box_p,
                key_class_p,
                draw_p,
                &menu,
                THEME_p,
                term_p,
            );
            key_class = key_class_p;
            collector = collector_p;
            draw = draw_p;
            term = term_p;
            CONFIG = CONFIG_p;
            cpu_box = cpu_box_p;
            THEME = THEME_p;
        } else if vec!["b", "n"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            drop(collector);
            drop(CONFIG);
            netcollector.switch(key, collector_p, CONFIG_p);
            collector = collector_p;
            CONFIG = CONFIG_p;
        } else if vec!["M", "escape"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            drop(THEME);
            drop(draw);
            drop(term);
            drop(update_checker);
            drop(key_class);
            drop(timer);
            drop(collector);
            drop(CONFIG);
            drop(netcollector);
            drop(brshtop_box);
            drop(init);
            drop(cpu_box);
            drop(cpucollector);
            drop(netbox);
            drop(proccollector);
            drop(mem_box);
            drop(proc_box);
            menu.main(
                draw_p,
                term_p,
                update_checker_p,
                THEME_p,
                key_class_p,
                timer_p,
                collector_p,
                collectors.clone(),
                CONFIG_p,
                ARG_MODE,
                netcollector_p,
                brshtop_box_p,
                init_p,
                cpu_box_p,
                cpucollector_p,
                boxes.clone(),
                netbox_p,
                proccollector_p,
                mem_box_p,
                proc_box_p,
            );
            draw = draw_p;
            term = term_p;
            update_checker = update_checker_p;
            THEME = THEME_p;
            key_class = key_class_p;
            timer = timer_p;
            collector = collector_p;
            CONFIG = CONFIG_p;
            netcollector = netcollector_p;
            brshtop_box = brshtop_box_p;
            init = init_p;
            cpu_box = cpu_box_p;
            cpucollector = cpucollector_p;
            netbox = netbox_p;
            proccollector = proccollector_p;
            mem_box = mem_box_p;
            proc_box = proc_box_p;
        } else if vec!["o", "f2"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            drop(THEME);
            drop(draw);
            drop(term);
            drop(CONFIG);
            drop(key_class);
            drop(timer);
            drop(netcollector);
            drop(brshtop_box);
            drop(collector);
            drop(init);
            drop(cpu_box);
            drop(cpucollector);
            drop(netbox);
            drop(proccollector);
            drop(proc_box);
            drop(mem_box);
            menu.options(
                ARG_MODE,
                THEME_p,
                draw_p,
                term_p,
                CONFIG_p,
                key_class_p,
                timer_p,
                netcollector_p,
                brshtop_box_p,
                boxes.clone(),
                collector_p,
                init_p,
                cpu_box_p,
                cpucollector_p,
                netbox_p,
                proccollector_p,
                collectors.clone(),
                proc_box_p,
                mem_box_p,
            );
            THEME = THEME_p;
            draw = draw_p;
            term = term_p;
            CONFIG = CONFIG_p;
            key_class = key_class_p;
            timer = timer_p;
            netcollector = netcollector_p;
            brshtop_box = brshtop_box_p;
            collector = collector_p;
            init = init_p;
            cpu_box = cpu_box_p;
            cpucollector = cpucollector_p;
            netbox = netbox_p;
            proccollector = proccollector_p;
            proc_box = proc_box_p;
            mem_box = mem_box_p;
        } else if vec!["h", "f1"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            drop(THEME);
            drop(draw);
            drop(term);
            drop(key_class);
            drop(collector);
            drop(CONFIG);
            drop(timer);
            menu.help(
                THEME_p,
                draw_p,
                term_p,
                key_class_p,
                collector_p,
                collectors.clone(),
                CONFIG_p,
                timer_p,
            );
            THEME = THEME_p;
            draw = draw_p;
            term = term_p;
            key_class = key_class_p;
            collector = collector_p;
            CONFIG = CONFIG_p;
            timer = timer_p;
        } else if key == "z".to_owned() {
            let inserter = netcollector.get_reset();
            netcollector.set_reset(!inserter);
            drop(CONFIG);
            collector.collect(
                vec![Collectors::NetCollector],
                CONFIG_p,
                true,
                false,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "y".to_owned() {
            CONFIG.net_sync = !CONFIG.net_sync;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::NetCollector],
                CONFIG_p,
                true,
                false,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "a".to_owned() {
            let inserter = netcollector.get_auto_min();
            netcollector.set_auto_min(!inserter);
            netcollector.set_net_min(
                vec![("download", -1), ("upload", -1)]
                    .iter()
                    .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                    .collect::<HashMap<String, i32>>(),
            );
            drop(CONFIG);
            collector.collect(
                vec![Collectors::NetCollector],
                CONFIG_p,
                true,
                false,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if vec!["left", "right"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            // TODO : Fix this...
            //proccollector.sorting(key);
        } else if key == " ".to_owned() && CONFIG.proc_tree && procbox.get_selected() > 0 {
            if proccollector
                .collapsed
                .contains_key(&procbox.get_selected_pid())
            {
                let inserter = proccollector
                    .collapsed
                    .get(&procbox.get_selected_pid().clone())
                    .unwrap()
                    .to_owned();
                proccollector
                    .collapsed
                    .insert(procbox.get_selected_pid().clone(), !inserter);
            }
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "e".to_owned() {
            CONFIG.proc_tree = !CONFIG.proc_tree;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "r".to_owned() {
            CONFIG.proc_reversed = !CONFIG.proc_reversed;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "c".to_owned() {
            CONFIG.proc_per_core = !CONFIG.proc_per_core;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "g".to_owned() {
            CONFIG.mem_graphs = !CONFIG.mem_graphs;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::MemCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "s".to_owned() {
            collector.set_collect_idle(EventEnum::Wait);
            collector.get_collect_idle_reference().wait(-1.0);
            CONFIG.swap_disk = !CONFIG.swap_disk;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::MemCollector],
                CONFIG_p,
                true,
                true,
                false,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "f".to_owned() {
            procbox.set_filtering(true);
            if proccollector.search_filter.len() == 0 {
                procbox.set_start(0);
            }
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                false,
                false,
                true,
                true,
            );
            CONFIG = CONFIG_p;
        } else if key == "m".to_owned() {
            if ARG_MODE.t != ViewModeEnum::None {
                ARG_MODE.replace_self(ViewModeEnum::None);
            } else if CONFIG
                .view_modes
                .iter()
                .position(|v| *v == CONFIG.view_mode)
                .unwrap()
                + 1
                > CONFIG.view_modes.len() - 1
            {
                CONFIG.view_mode = CONFIG.view_modes[0];
            } else {
                CONFIG.view_mode = CONFIG.view_modes[CONFIG
                    .view_modes
                    .iter()
                    .position(|v| *v == CONFIG.view_mode)
                    .unwrap()
                    + 1];
            }
            brshtop_box.set_proc_mode(CONFIG.view_mode.t == ViewModeEnum::Proc);
            brshtop_box.set_stat_mode(CONFIG.view_mode.t == ViewModeEnum::Stat);
            draw.clear(vec![], true);
            drop(collector);
            drop(init);
            drop(cpu_box);
            drop(draw);
            drop(key_class);
            drop(brshtop_box);
            drop(timer);
            drop(CONFIG);
            drop(THEME);
            drop(cpucollector);
            drop(mem_box);
            drop(netbox);
            drop(procbox);
            term.refresh(
                vec![],
                boxes.clone(),
                collector_p,
                init_p,
                cpu_box_p,
                draw_p,
                true,
                key_class_p,
                &mut menu,
                brshtop_box_p,
                timer_p,
                CONFIG_p,
                THEME_p,
                cpucollector_p,
                mem_box_p,
                netbox_p,
                procbox_p,
            );

            collector = collector_p;
            init = init_p;
            cpu_box = cpu_box_p;
            draw = draw_p;
            key_class = key_class_p;
            brshtop_box = brshtop_box_p;
            timer = timer_p;
            THEME = THEME_p;
            cpucollector = cpucollector_p;
            mem_box = mem_box_p;
            netbox = netbox_p;
            procbox = procbox_p;
        } else if vec!["t", "k", "i"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key.to_ascii_lowercase())
        {
            let pid: u32 = if procbox.get_selected() > 0 {
                procbox.get_selected_pid()
            } else {
                proccollector.detailed_pid.unwrap()
            };
            let lower = key.to_ascii_lowercase();
            if psutil::process::pid_exists(pid) {
                let sig = if lower == "t".to_owned() {
                    Signal::SIGTERM
                } else if lower == "k".to_owned() {
                    Signal::SIGKILL
                } else {
                    Signal::SIGINT
                };
                match psutil::process::Process::new(pid).unwrap().send_signal(sig) {
                    Ok(_) => (),
                    Err(e) => errlog(format!(
                        "Execption when sending signal {} to pid {}",
                        sig, pid
                    )),
                };
            }
        } else if key == "delete".to_owned() && proccollector.search_filter.len() > 0 {
            proccollector.search_filter = String::default();
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                false,
                true,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if key == "enter".to_owned() {
            if procbox.get_selected() > 0
                && proccollector.detailed_pid.unwrap_or(0) != procbox.get_selected_pid()
                && psutil::process::pid_exists(procbox.get_selected_pid())
            {
                proccollector.detailed = true;
                let inserter = procbox.get_selected();
                procbox.set_last_selection(inserter);
                procbox.set_selected(0);
                proccollector.detailed_pid = Some(procbox.get_selected_pid());
                procbox.set_parent_resized(true);
            } else if proccollector.detailed {
                let inserter = procbox.get_last_selection();
                procbox.set_selected(inserter);
                procbox.set_last_selection(0);
                proccollector.detailed = false;
                proccollector.detailed_pid = None;
                procbox.set_parent_resized(true);
            } else {
                continue;
            }
            proccollector.details = HashMap::<String, ProcCollectorDetails>::new();
            proccollector.details_cpu = vec![];
            proccollector.details_mem = vec![];
            graphs.detailed_cpu.NotImplemented = true;
            graphs.detailed_mem.NotImplemented = true;
            drop(CONFIG);
            collector.collect(
                vec![Collectors::ProcCollector],
                CONFIG_p,
                true,
                false,
                true,
                true,
                false,
            );
            CONFIG = CONFIG_p;
        } else if vec![
            "up",
            "down",
            "mouse_scroll_up",
            "mouse_scroll_down",
            "page_up",
            "page_down",
            "home",
            "end",
            "mouse_click",
            "mouse_unselect",
        ]
        .iter()
        .map(|s| s.to_owned().to_owned())
        .collect::<Vec<String>>()
        .contains(&key)
        {
            drop(proccollector);
            drop(key_class);
            drop(collector);
            drop(CONFIG);
            procbox.selector(
                key.clone(),
                mouse_pos,
                proccollector_p,
                key_class_p,
                collector_p,
                CONFIG_p,
            );
            proccollector = proccollector_p;
            key_class = key_class_p;
            collector = collector_p;
            CONFIG = CONFIG_p;
        }
    }
}

pub fn get_cpu_core_mapping() -> Vec<i32> {
    let mut mapping: Vec<i32> = vec![];
    let map_file = Path::new("/proc/cpuinfo");
    if SYSTEM.to_owned() == "Linux".to_owned() && map_file.exists() {
        for _ in 0..THREADS.to_owned() {
            mapping.push(0);
        }
        let mut num: i32 = 0;
        for l in read_lines(map_file).unwrap() {
            if let Ok(line) = l {
                if line.starts_with("processor") {
                    num = line.trim()[line.find(": ").unwrap() + 2..]
                        .to_owned()
                        .parse::<i32>()
                        .unwrap_or(0);
                    if num > THREADS.to_owned() as i32 - 1 || num < 0 {
                        break;
                    }
                } else if line.starts_with("core id") {
                    mapping[num as usize] = line.trim()[line.find(": ").unwrap() + 2..]
                        .to_owned()
                        .parse::<i32>()
                        .unwrap_or(0);
                }
            }
        }
        if num < THREADS.to_owned() as i32 - 1 {
            throw_error("Error getting cpu core mapping!!!");
        }
    }

    if mapping.len() == 0 {
        mapping = vec![];
        for _ in 0..THREADS.to_owned() / CORES.to_owned() {
            let mut appender: Vec<i32> = vec![];
            for x in 0..CORES.to_owned() as i32 {
                appender.push(x);
            }
            mapping.append(&mut appender);
        }
    }

    mapping
}

fn read_lines<P: AsRef<Path>>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Reset terminal settings and stop background input read before putting to sleep
pub fn now_sleeping(
    key_p: &OnceCell<Mutex<Key>>,
    collector_p: &OnceCell<Mutex<Collector>>,
    draw_p: &OnceCell<Mutex<Draw>>,
    term_p: &OnceCell<Mutex<Term>>,
) -> Option<()> {
    let mut key = key_p;
    let mut collector = collector_p;
    let mut draw = draw_p;
    let mut term = term_p;

    key.stop();
    collector.stop();
    draw.now(
        vec![
            term.get_clear(),
            term.get_normal_screen(),
            term.get_show_cursor(),
            term.get_mouse_off(),
            Term::title("".to_owned()),
        ],
        &mut key,
    );
    Term::echo(true);
    match psutil::process::Process::new(process::id())
        .unwrap()
        .send_signal(Signal::SIGTSTP)
    {
        Ok(_) => Some(()),
        Err(e) => None,
    }
}

/// Set terminal settings and restart background input read
pub fn now_awake(
    draw_p: &OnceCell<Mutex<Draw>>,
    term_p: &OnceCell<Mutex<Term>>,
    key_p: &OnceCell<Mutex<Key>>,
    brshtop_box_p: &OnceCell<Mutex<BrshtopBox>>,
    collector_p: &OnceCell<Mutex<Collector>>,
    boxes: Vec<Boxes>,
    init_p: &OnceCell<Mutex<Init>>,
    cpu_box_p: &OnceCell<Mutex<CpuBox>>,
    menu_p: &OnceCell<Mutex<Menu>>,
    timer_p: &OnceCell<Mutex<Timer>>,
    CONFIG_p: &OnceCell<Mutex<Config>>,
    THEME_p: &OnceCell<Mutex<Theme>>,
    DEBUG: bool,
    collectors: Vec<Collectors>,
    timeit_p: &OnceCell<Mutex<TimeIt>>,
    ARG_MODE: ViewMode,
    graphs_p: &OnceCell<Mutex<Graphs>>,
    meters_p: &OnceCell<Mutex<Meters>>,
    netbox_p: &OnceCell<Mutex<NetBox>>,
    procbox_p: &OnceCell<Mutex<ProcBox>>,
    membox_p: &OnceCell<Mutex<MemBox>>,
    cpu_collector_p: &OnceCell<Mutex<CpuCollector>>,
    mem_collector_p: &OnceCell<Mutex<MemCollector>>,
    net_collector_p: &OnceCell<Mutex<NetCollector>>,
    proc_collector_p: &OnceCell<Mutex<ProcCollector>>,
) {
    let mut draw = draw_p;
    let mut term = term_p;
    let mut key = key_p;
    let mut brshtop_box = brshtop_box_p;
    let mut collector = collector_p;
    let mut init = init_p;
    let mut cpu_box = cpu_box_p;
    let mut menu = menu_p;
    let mut timer = timer_p;
    let mut CONFIG = CONFIG_p;
    let mut THEME = THEME_p;
    let mut timeit = timeit_p;
    let mut graphs = graphs_p;
    let mut meters = meters_p;
    let mut netbox = netbox_p;
    let mut procbox = procbox_p;
    let mut membox = membox_p;
    let mut cpu_collector = cpu_collector_p;
    let mut mem_collector = mem_collector_p;
    let mut net_collector = net_collector_p;
    let mut proc_collector = proc_collector_p;

    draw.now(
        vec![
            term.get_alt_screen(),
            term.get_clear(),
            term.get_hide_cursor(),
            term.get_mouse_on(),
            Term::title("BRShtop".to_owned()),
        ],
        &mut key,
    );
    Term::echo(false);
    drop(draw);
    drop(menu);
    key.start(draw_p, menu_p);
    drop(collector);
    drop(init);
    drop(cpu_box);
    drop(key);
    drop(brshtop_box);
    drop(timer);
    drop(CONFIG);
    drop(THEME);
    drop(cpu_collector);
    drop(netbox);
    drop(procbox);
    menu = menu_p;
    term.refresh(
        vec![],
        boxes.clone(),
        collector_p,
        init_p,
        cpu_box_p,
        draw_p,
        false,
        key_p,
        &mut menu,
        brshtop_box_p,
        timer_p,
        CONFIG_p,
        THEME_p,
        cpu_collector_p,
        membox_p,
        netbox_p,
        procbox_p,
    );
    collector = collector_p;
    init = init_p;
    brshtop_box = brshtop_box_p;
    timer = timer_p;

    drop(term);
    brshtop_box.calc_sizes(
        boxes.clone(),
        term_p,
        CONFIG_p,
        cpu_collector_p,
        cpu_box_p,
        membox_p,
        netbox_p,
        procbox_p,
    );
    brshtop_box.draw_bg(
        true,
        draw_p,
        boxes.clone(),
        &menu,
        CONFIG_p,
        cpu_box_p,
        membox_p,
        netbox_p,
        procbox_p,
        key_p,
        THEME_p,
        term_p,
    );

    drop(brshtop_box);
    drop(timeit);
    drop(menu);
    drop(graphs);
    drop(meters);
    drop(membox);
    drop(mem_collector);
    drop(net_collector);
    drop(proc_collector);
    collector.start(
        CONFIG_p,
        DEBUG,
        collectors,
        brshtop_box_p,
        timeit_p,
        menu_p,
        draw_p,
        term_p,
        cpu_box_p,
        key_p,
        THEME_p,
        ARG_MODE,
        graphs_p,
        meters_p,
        netbox_p,
        procbox_p,
        membox_p,
        cpu_collector_p,
        mem_collector_p,
        net_collector_p,
        proc_collector_p,
    )
}

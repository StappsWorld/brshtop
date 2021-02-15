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

    //println!("Made it through global variables");

    // Pre main ---------------------------------------------------------------------------------------------
    let mut term: Term = Term::new();

    let mut key: Key = Key::new();

    let mut draw: Draw = Draw::new();

    let mut brshtop_box: BrshtopBox = BrshtopBox::new(&CONFIG, ARG_MODE);

    let mut cpu_box: CpuBox = CpuBox::new(&mut brshtop_box, &CONFIG, ARG_MODE);

    let mut mem_box: MemBox = MemBox::new(&mut brshtop_box, &CONFIG, ARG_MODE);

    let mut net_box: NetBox = NetBox::new(&CONFIG, ARG_MODE, &mut brshtop_box);

    let mut proc_box: ProcBox = ProcBox::new(&mut brshtop_box, &CONFIG, ARG_MODE);

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
        &mut init,
        &mut cpu_box,
        &mut draw,
        true,
        &mut key,
        &mut menu,
        &mut brshtop_box,
        &mut timer,
        &CONFIG,
        &THEME,
        &cpu_collector,
        &mut mem_box,
        &mut net_box,
        &mut proc_box,
    );

    // Start a thread checking for updates while running init
    if CONFIG.update_check {
        updatechecker.run();
    }

    // Draw banner and init status
    if CONFIG.show_init && !init.resized {
        init.start(&mut draw, &mut key, &term);
    }

    // Load theme
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }

    THEME = match Theme::from_str(CONFIG.color_theme.clone()) {
        Ok(t) => {
            init.success(&CONFIG, &mut draw, &term, &mut key);
            t
        }
        Err(e) => {
            errlog(format!("Unable to read theme from config (error {})...", e));
            Init::fail(e, &CONFIG, &mut draw, &mut collector, &mut key, &term);
            Theme::default()
        }
    };

    // Setup boxes
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
        if CONFIG.check_temp {
            cpu_collector.get_sensors(&CONFIG);
        }

        brshtop_box.calc_sizes(
            boxes.clone(),
            &term,
            &CONFIG,
            &cpu_collector,
            &mut cpu_box,
            &mut mem_box,
            &mut net_box,
            &mut proc_box,
        );
        brshtop_box.draw_bg(
            false,
            &mut draw,
            boxes.clone(),
            &menu,
            &CONFIG,
            &cpu_box,
            &mem_box,
            &net_box,
            &proc_box,
            &mut key,
            &THEME,
            &term,
        );
        init.success(&CONFIG, &mut draw, &term, &mut key);
    }

    // Setup signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }

    let mut signals = match Signals::new(&[SIGTSTP, SIGCONT, SIGINT, SIGWINCH]) {
        //Handling ctrl-z, resume, ctrl-c, terminal resized
        Ok(s) => s,
        Err(e) => {
            Init::fail(
                e.to_string(),
                &CONFIG,
                &mut draw,
                &mut collector,
                &mut key,
                &term,
            );
            return;
        }
    };

    match crossbeam::scope(|s| {
        s.spawn(|_| {
            for sig in signals.forever() {
                match sig {
                    SIGTSTP => match now_sleeping(&mut key, &mut collector, &mut draw, &mut term) {
                        Some(_) => (),
                        None => clean_quit(
                            None,
                            Some("Failed to pause program".to_owned()),
                            &mut key,
                            &mut collector,
                            &mut draw,
                            &term,
                            &CONFIG,
                        ),
                    },
                    SIGCONT => now_awake(
                        &mut draw,
                        &mut term,
                        &mut key,
                        &mut brshtop_box,
                        &mut collector,
                        boxes.clone(),
                        &mut init,
                        &mut cpu_box,
                        &mut menu,
                        &mut timer,
                        &CONFIG,
                        &THEME,
                        DEBUG,
                        collectors.clone(),
                        &mut timeit,
                        ARG_MODE,
                        &mut graphs,
                        &mut meters,
                        &mut net_box,
                        &mut proc_box,
                        &mut mem_box,
                        &mut cpu_collector,
                        &mut mem_collector,
                        &mut net_collector,
                        &mut proc_collector,
                    ),
                    SIGINT => clean_quit(
                        None,
                        None,
                        &mut key,
                        &mut collector,
                        &mut draw,
                        &term,
                        &CONFIG,
                    ),
                    SIGWINCH => {
                        term.refresh(
                            vec![],
                            boxes.clone(),
                            &mut collector,
                            &mut init,
                            &mut cpu_box,
                            &mut draw,
                            true,
                            &mut key,
                            &mut menu,
                            &mut brshtop_box,
                            &mut timer,
                            &CONFIG,
                            &THEME,
                            &cpu_collector,
                            &mut mem_box,
                            &mut net_box,
                            &mut proc_box,
                        );
                    }
                    _ => unreachable!(),
                }
            }
        });
    }) {
        _ => (),
    };

    init.success(&CONFIG, &mut draw, &term, &mut key);

    // Start a separate thread for reading keyboard input
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }

    key.start(&mut draw, &menu);

    init.success(&CONFIG, &mut draw, &term, &mut key);

    // Start a separate thread for data collection and drawing
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }

    collector.start(
        &CONFIG,
        DEBUG,
        collectors.clone(),
        &mut brshtop_box,
        &mut timeit,
        &menu,
        &mut draw,
        &term,
        &mut cpu_box,
        &mut key,
        &THEME,
        ARG_MODE,
        &mut graphs,
        &mut meters,
        &mut net_box,
        &mut proc_box,
        &mut mem_box,
        &mut cpu_collector,
        &mut mem_collector,
        &mut net_collector,
        &mut proc_collector,
    );
    init.success(&CONFIG, &mut draw, &term, &mut key);

    // Collect data and draw to buffer
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }
    collector.collect(collectors.clone(), false, false, false, false, false);
    init.success(&CONFIG, &mut draw, &term, &mut key);

    // Draw to screen
    if CONFIG.show_init {
        draw.buffer(
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
            &mut key,
        );
    }
    collector.set_collect_done(EventEnum::Wait);
    collector.get_collect_done_reference().wait(1.0);
    init.success(&CONFIG, &mut draw, &term, &mut key);

    init.done(&CONFIG, &mut draw, &term, &mut key);
    term.refresh(
        vec![],
        boxes.clone(),
        &mut collector,
        &mut init,
        &mut cpu_box,
        &mut draw,
        false,
        &mut key,
        &mut menu,
        &mut brshtop_box,
        &mut timer,
        &CONFIG,
        &THEME,
        &cpu_collector,
        &mut mem_box,
        &mut net_box,
        &mut proc_box,
    );
    draw.out(vec![], true, &mut key);
    if CONFIG.draw_clock.len() > 0 {
        brshtop_box.set_clock_on(true);
    }
    if DEBUG {
        timeit.stop("Init".to_owned());
    }

    // Main loop ------------------------------------------------------------------------------------->

    run(
        &mut term,
        &mut key,
        &mut timer,
        &mut collector,
        boxes.clone(),
        &mut init,
        &mut cpu_box,
        &mut draw,
        &mut menu,
        &mut brshtop_box,
        &mut CONFIG,
        &mut THEME,
        &mut ARG_MODE,
        &mut proc_box,
        &mut proc_collector,
        &mut net_collector,
        &mut cpu_collector,
        &mut net_box,
        &updatechecker,
        collectors.clone(),
        &mut graphs,
        &mut mem_box,
    );
}

pub fn run(
    term: &mut Term,
    key: &mut Key,
    timer: &mut Timer,
    collector: &mut Collector,
    boxes: Vec<Boxes>,
    init: &mut Init,
    cpu_box: &mut CpuBox,
    draw: &mut Draw,
    menu: &mut Menu,
    brshtop_box: &mut BrshtopBox,
    CONFIG: &mut Config,
    THEME: &mut Theme,
    ARG_MODE: &mut ViewMode,
    procbox: &mut ProcBox,
    proccollector: &mut ProcCollector,
    netcollector: &mut NetCollector,
    cpucollector: &mut CpuCollector,
    netbox: &mut NetBox,
    update_checker: &UpdateChecker,
    collectors: Vec<Collectors>,
    graphs: &mut Graphs,
    mem_box: &mut MemBox,
) {
    loop {
        term.refresh(
            vec![],
            boxes.clone(),
            collector,
            init,
            cpu_box,
            draw,
            false,
            key,
            menu,
            brshtop_box,
            timer,
            CONFIG,
            THEME,
            cpucollector,
            mem_box,
            netbox,
            procbox,
        );

        timer.stamp();

        while timer.not_zero(CONFIG) {
            if key.input_wait(timer.left(CONFIG).as_secs_f64(), false, draw, term) {
                process_keys(
                    ARG_MODE,
                    key,
                    procbox,
                    collector,
                    proccollector,
                    CONFIG,
                    draw,
                    term,
                    brshtop_box,
                    cpu_box,
                    menu,
                    THEME,
                    netcollector,
                    init,
                    cpucollector,
                    boxes.clone(),
                    netbox,
                    update_checker,
                    collectors.clone(),
                    timer,
                    graphs,
                    mem_box,
                );
            }
        }

        collector.collect(collectors.clone(), true, false, false, false, false);
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
    brshtop_box: Option<&BrshtopBox>,
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
                ww = brshtop_box.unwrap().get_width();
                wh = brshtop_box.unwrap().get_height();
                wt = brshtop_box.unwrap().get_name();
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
    collector: &mut Collector,
    draw: &mut Draw,
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
        key,
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
    key_class: &mut Key,
    procbox: &mut ProcBox,
    collector: &mut Collector,
    proccollector: &mut ProcCollector,
    CONFIG: &mut Config,
    draw: &mut Draw,
    term: &mut Term,
    brshtop_box: &mut BrshtopBox,
    cpu_box: &mut CpuBox,
    menu: &mut Menu,
    THEME: &mut Theme,
    netcollector: &mut NetCollector,
    init: &mut Init,
    cpucollector: &mut CpuCollector,
    boxes: Vec<Boxes>,
    netbox: &mut NetBox,
    update_checker: &UpdateChecker,
    collectors: Vec<Collectors>,
    timer: &mut Timer,
    graphs: &mut Graphs,
    mem_box: &mut MemBox,
) {
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

        if key == "_null".to_owned() {
            continue;
        } else if key == "q".to_owned() {
            clean_quit(None, None, key_class, collector, draw, term, CONFIG);
        } else if key == "+" && CONFIG.update_ms + 100 <= 86399900 {
            CONFIG.update_ms += 100;
            brshtop_box.draw_update_ms(false, CONFIG, cpu_box, key_class, draw, menu, THEME, term);
        } else if key == "-".to_owned() && CONFIG.update_ms - 100 >= 100 {
            CONFIG.update_ms -= 100;
            brshtop_box.draw_update_ms(false, CONFIG, cpu_box, key_class, draw, menu, THEME, term);
        } else if vec!["b", "n"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            netcollector.switch(key, collector);
        } else if vec!["M", "escape"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.main(
                draw,
                term,
                update_checker,
                THEME,
                key_class,
                timer,
                collector,
                collectors.clone(),
                CONFIG,
                ARG_MODE,
                netcollector,
                brshtop_box,
                init,
                cpu_box,
                cpucollector,
                boxes.clone(),
                netbox,
                proccollector,
                mem_box,
                procbox,
            );
        } else if vec!["o", "f2"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.options(
                ARG_MODE,
                THEME,
                draw,
                term,
                CONFIG,
                key_class,
                timer,
                netcollector,
                brshtop_box,
                boxes.clone(),
                collector,
                init,
                cpu_box,
                cpucollector,
                netbox,
                proccollector,
                collectors.clone(),
                procbox,
                mem_box,
            );
        } else if vec!["h", "f1"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.help(
                THEME,
                draw,
                term,
                key_class,
                collector,
                collectors.clone(),
                CONFIG,
                timer,
            );
        } else if key == "z".to_owned() {
            let inserter = netcollector.get_reset();
            netcollector.set_reset(!inserter);
            collector.collect(
                vec![Collectors::NetCollector],
                true,
                false,
                false,
                true,
                false,
            );
        } else if key == "y".to_owned() {
            let switch = CONFIG.net_sync.clone();
            CONFIG.net_sync = !switch;
            collector.collect(
                vec![Collectors::NetCollector],
                true,
                false,
                false,
                true,
                false,
            );
        } else if key == "a".to_owned() {
            let inserter = netcollector.get_auto_min();
            netcollector.set_auto_min(!inserter);
            netcollector.set_net_min(
                vec![("download", -1), ("upload", -1)]
                    .iter()
                    .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                    .collect::<HashMap<String, i32>>(),
            );
            collector.collect(
                vec![Collectors::NetCollector],
                true,
                false,
                false,
                true,
                false,
            );
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
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "e".to_owned() {
            let switch = CONFIG.proc_tree;
            CONFIG.proc_tree = !switch;
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "r".to_owned() {
            let switch = CONFIG.proc_reversed;
            CONFIG.proc_reversed = !switch;
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "c".to_owned() {
            let switch = CONFIG.proc_per_core;
            CONFIG.proc_per_core = !switch;
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "g".to_owned() {
            let switch = CONFIG.mem_graphs;
            CONFIG.mem_graphs = !switch;
            collector.collect(
                vec![Collectors::MemCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "s".to_owned() {
            collector.set_collect_idle(EventEnum::Wait);
            collector.get_collect_idle_reference().wait(1.0);
            let switch = CONFIG.swap_disk;
            CONFIG.swap_disk = !switch;
            collector.collect(
                vec![Collectors::MemCollector],
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "f".to_owned() {
            procbox.set_filtering(true);
            if proccollector.search_filter.len() == 0 {
                procbox.set_start(0);
            }
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                false,
                false,
                true,
                true,
            );
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
            term.refresh(
                vec![],
                boxes.clone(),
                collector,
                init,
                cpu_box,
                draw,
                true,
                key_class,
                menu,
                brshtop_box,
                timer,
                CONFIG,
                THEME,
                cpucollector,
                mem_box,
                netbox,
                procbox,
            );
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
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                false,
                true,
                true,
                false,
            );
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
            collector.collect(
                vec![Collectors::ProcCollector],
                true,
                false,
                true,
                true,
                false,
            );
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
            procbox.selector(
                key.clone(),
                mouse_pos,
                proccollector,
                key_class,
                collector,
                CONFIG,
            );
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
    key: &mut Key,
    collector: &mut Collector,
    draw: &mut Draw,
    term: &mut Term,
) -> Option<()> {
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
        key,
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
    draw: &mut Draw,
    term: &mut Term,
    key: &mut Key,
    brshtop_box: &mut BrshtopBox,
    collector: &mut Collector,
    boxes: Vec<Boxes>,
    init: &mut Init,
    cpu_box: &mut CpuBox,
    menu: &mut Menu,
    timer: &mut Timer,
    CONFIG: &Config,
    THEME: &Theme,
    DEBUG: bool,
    collectors: Vec<Collectors>,
    timeit: &mut TimeIt,
    ARG_MODE: ViewMode,
    graphs: &mut Graphs,
    meters: &mut Meters,
    netbox: &mut NetBox,
    procbox: &mut ProcBox,
    membox: &mut MemBox,
    cpu_collector: &mut CpuCollector,
    mem_collector: &mut MemCollector,
    net_collector: &mut NetCollector,
    proc_collector: &mut ProcCollector,
) {
    draw.now(
        vec![
            term.get_alt_screen(),
            term.get_clear(),
            term.get_hide_cursor(),
            term.get_mouse_on(),
            Term::title("BRShtop".to_owned()),
        ],
        key,
    );
    Term::echo(false);
    key.start(draw, menu);
    term.refresh(
        vec![],
        boxes.clone(),
        collector,
        init,
        cpu_box,
        draw,
        false,
        key,
        menu,
        brshtop_box,
        timer,
        CONFIG,
        THEME,
        cpu_collector,
        membox,
        netbox,
        procbox,
    );

    brshtop_box.calc_sizes(
        boxes.clone(),
        term,
        CONFIG,
        cpu_collector,
        cpu_box,
        membox,
        netbox,
        procbox,
    );
    brshtop_box.draw_bg(
        true,
        draw,
        boxes.clone(),
        menu,
        CONFIG,
        cpu_box,
        membox,
        netbox,
        procbox,
        key,
        THEME,
        term,
    );

    collector.start(
        CONFIG,
        DEBUG,
        collectors,
        brshtop_box,
        timeit,
        menu,
        draw,
        term,
        cpu_box,
        key,
        THEME,
        ARG_MODE,
        graphs,
        meters,
        netbox,
        procbox,
        membox,
        cpu_collector,
        mem_collector,
        net_collector,
        proc_collector,
    )
}

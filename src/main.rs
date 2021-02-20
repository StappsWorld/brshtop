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
    cpuid,
    error::{errlog, throw_error},
    expanduser::expanduser,
    lazy_static::lazy_static,
    log::{debug, LevelFilter},
    math::round,
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
        sync::{Arc, Mutex, MutexGuard},
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

    // Setting up error logging

    let error_file = "brshtop.log";
    let error_dir = CONFIG_DIR.join(PathBuf::from(error_file));
    let dir = error_dir.to_str().unwrap();

    match simple_logging::log_to_file(dir, LevelFilter::Debug) {
        Err(e) => throw_error(
            format!(
                "ERROR!\nNo permission to write to \"{}\" directory with error {}!",
                CONFIG_DIR.to_str().unwrap(),
                e
            )
            .as_str(),
        ),
        _ => (),
    };

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

    let mut ARG_MODE_raw: ViewMode = ViewMode {
        t: ViewModeEnum::None,
    };
    let arg_full = matches.value_of("Full Mode");
    let arg_proc = matches.value_of("Minimal Mode (proc)");
    let arg_stat = matches.value_of("Minimal Mode (stat)");
    let arg_version = matches.value_of("Version");
    let arg_debug = matches.value_of("Debug");

    if arg_full.is_some() {
        ARG_MODE_raw = ViewMode {
            t: ViewModeEnum::Full,
        };
    } else if arg_proc.is_some() {
        ARG_MODE_raw = ViewMode {
            t: ViewModeEnum::Proc,
        }
    } else if arg_stat.is_some() {
        ARG_MODE_raw = ViewMode {
            t: ViewModeEnum::Stat,
        };
    }
    let mut ARG_MODE_parent: Arc<Mutex<ViewMode>> = Arc::new(Mutex::new(ARG_MODE_raw));
    let mut ARG_MODE_mutex: Arc<Mutex<ViewMode>> = Arc::clone(&ARG_MODE_parent);
    let mut ARG_MODE: MutexGuard<ViewMode> = ARG_MODE_mutex.lock().unwrap();

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

    let mut CONFIG_raw: Config = match Config::new(CONFIG_FILE.clone()) {
        Ok(c) => c,
        Err(e) => {
            throw_error(e);
            Config::new(CONFIG_FILE.clone()).unwrap() //Never reached, but compiler is unhappy, so I bend
        }
    };
    let mut CONFIG_parent: Arc<Mutex<Config>> = Arc::new(Mutex::new(CONFIG_raw));
    let mut CONFIG_mutex: Arc<Mutex<Config>> = Arc::clone(&CONFIG_parent);
    let mut CONFIG: MutexGuard<Config> = CONFIG_mutex.lock().unwrap();

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

    let mut THEME_raw: Theme = match Theme::from_file(CONFIG.color_theme.clone()) {
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
    let mut THEME_parent: Arc<Mutex<Theme>> = Arc::new(Mutex::new(THEME_raw));
    let mut THEME_mutex: Arc<Mutex<Theme>> = Arc::clone(&THEME_parent);
    let mut THEME: MutexGuard<Theme> = THEME_mutex.lock().unwrap();

    errlog("Made it through global variables".to_owned());

    // Pre main ---------------------------------------------------------------------------------------------
    let mut term_raw: Term = Term::new();
    let mut term_parent: Arc<Mutex<Term>> = Arc::new(Mutex::new(term_raw));
    let mut term_mutex: Arc<Mutex<Term>> = Arc::clone(&term_parent);
    let mut term: MutexGuard<Term> = term_mutex.lock().unwrap();

    let mut key_raw: Key = Key::new();
    let mut key_parent: Arc<Mutex<Key>> = Arc::new(Mutex::new(key_raw));
    let mut key_mutex: Arc<Mutex<Key>> = Arc::clone(&key_parent);
    let mut key: MutexGuard<Key> = key_mutex.lock().unwrap();

    let mut draw_raw: Draw = Draw::new();
    let mut draw_parent: Arc<Mutex<Draw>> = Arc::new(Mutex::new(draw_raw));
    let mut draw_mutex: Arc<Mutex<Draw>> = Arc::clone(&draw_parent);
    let mut draw: MutexGuard<Draw> = draw_mutex.lock().unwrap();

    let mut brshtop_box_raw: BrshtopBox = BrshtopBox::new(&CONFIG, ARG_MODE.to_owned());
    let mut brshtop_box_parent: Arc<Mutex<BrshtopBox>> = Arc::new(Mutex::new(brshtop_box_raw));
    let mut brshtop_box_mutex: Arc<Mutex<BrshtopBox>> = Arc::clone(&brshtop_box_parent);
    let mut brshtop_box: MutexGuard<BrshtopBox> = brshtop_box_mutex.lock().unwrap();

    let mut cpu_box_raw: CpuBox = CpuBox::new(&mut brshtop_box, &CONFIG, ARG_MODE.to_owned());
    let mut cpu_box_parent: Arc<Mutex<CpuBox>> = Arc::new(Mutex::new(cpu_box_raw));
    let mut cpu_box_mutex: Arc<Mutex<CpuBox>> = Arc::clone(&cpu_box_parent);
    let mut cpu_box: MutexGuard<CpuBox> = cpu_box_mutex.lock().unwrap();

    let mut mem_box_raw: MemBox = MemBox::new(&mut brshtop_box, &CONFIG, ARG_MODE.to_owned());
    let mut mem_box_parent: Arc<Mutex<MemBox>> = Arc::new(Mutex::new(mem_box_raw));
    let mut mem_box_mutex: Arc<Mutex<MemBox>> = Arc::clone(&mem_box_parent);
    let mut mem_box: MutexGuard<MemBox> = mem_box_mutex.lock().unwrap();

    let mut net_box_raw: NetBox = NetBox::new(&CONFIG, ARG_MODE.to_owned(), &mut brshtop_box);
    let mut net_box_parent: Arc<Mutex<NetBox>> = Arc::new(Mutex::new(net_box_raw));
    let mut net_box_mutex: Arc<Mutex<NetBox>> = Arc::clone(&net_box_parent);
    let mut net_box: MutexGuard<NetBox> = net_box_mutex.lock().unwrap();

    let mut proc_box_raw: ProcBox = ProcBox::new(&mut brshtop_box, &CONFIG, ARG_MODE.to_owned());
    let mut proc_box_parent: Arc<Mutex<ProcBox>> = Arc::new(Mutex::new(proc_box_raw));
    let mut proc_box_mutex: Arc<Mutex<ProcBox>> = Arc::clone(&proc_box_parent);
    let mut proc_box: MutexGuard<ProcBox> = proc_box_mutex.lock().unwrap();

    let mut collector_raw: Collector = Collector::new();
    let mut collector_parent: Arc<Mutex<Collector>> = Arc::new(Mutex::new(collector_raw));
    let mut collector_mutex: Arc<Mutex<Collector>> = Arc::clone(&collector_parent);
    let mut collector: MutexGuard<Collector> = collector_mutex.lock().unwrap();

    let mut cpu_collector_raw: CpuCollector = CpuCollector::new();
    let mut cpu_collector_parent: Arc<Mutex<CpuCollector>> =
        Arc::new(Mutex::new(cpu_collector_raw));
    let mut cpu_collector_mutex: Arc<Mutex<CpuCollector>> = Arc::clone(&cpu_collector_parent);
    let mut cpu_collector: MutexGuard<CpuCollector> = cpu_collector_mutex.lock().unwrap();

    let mut mem_collector_raw: MemCollector = MemCollector::new(&mem_box);
    let mut mem_collector_parent: Arc<Mutex<MemCollector>> =
        Arc::new(Mutex::new(mem_collector_raw));
    let mut mem_collector_mutex: Arc<Mutex<MemCollector>> = Arc::clone(&mem_collector_parent);
    let mut mem_collector: MutexGuard<MemCollector> = mem_collector_mutex.lock().unwrap();

    let mut net_collector_raw: NetCollector = NetCollector::new(&net_box, &CONFIG);
    let mut net_collector_parent: Arc<Mutex<NetCollector>> =
        Arc::new(Mutex::new(net_collector_raw));
    let mut net_collector_mutex: Arc<Mutex<NetCollector>> = Arc::clone(&net_collector_parent);
    let mut net_collector: MutexGuard<NetCollector> = net_collector_mutex.lock().unwrap();

    let mut proc_collector_raw: ProcCollector = ProcCollector::new(&proc_box);
    let mut proc_collector_parent: Arc<Mutex<ProcCollector>> =
        Arc::new(Mutex::new(proc_collector_raw));
    let mut proc_collector_mutex: Arc<Mutex<ProcCollector>> = Arc::clone(&proc_collector_parent);
    let mut proc_collector: MutexGuard<ProcCollector> = proc_collector_mutex.lock().unwrap();

    let mut menu_raw: Menu = Menu::new(MENUS, MENU_COLORS);
    let mut menu_parent: Arc<Mutex<Menu>> = Arc::new(Mutex::new(menu_raw));
    let mut menu_mutex: Arc<Mutex<Menu>> = Arc::clone(&menu_parent);
    let mut menu: MutexGuard<Menu> = menu_mutex.lock().unwrap();

    let mut timer_raw: Timer = Timer::new();
    let mut timer_parent: Arc<Mutex<Timer>> = Arc::new(Mutex::new(timer_raw));
    let mut timer_mutex: Arc<Mutex<Timer>> = Arc::clone(&timer_parent);
    let mut timer: MutexGuard<Timer> = timer_mutex.lock().unwrap();

    let mut timeit_raw: TimeIt = TimeIt::new();
    let mut timeit_parent: Arc<Mutex<TimeIt>> = Arc::new(Mutex::new(timeit_raw));
    let mut timeit_mutex: Arc<Mutex<TimeIt>> = Arc::clone(&timeit_parent);
    let mut timeit: MutexGuard<TimeIt> = timeit_mutex.lock().unwrap();

    let mut init_raw: Init = Init::new();
    let mut init_parent: Arc<Mutex<Init>> = Arc::new(Mutex::new(init_raw));
    let mut init_mutex: Arc<Mutex<Init>> = Arc::clone(&init_parent);
    let mut init: MutexGuard<Init> = init_mutex.lock().unwrap();

    let mut updatechecker_raw: UpdateChecker = UpdateChecker::new();
    let mut updatechecker_parent: Arc<Mutex<UpdateChecker>> =
        Arc::new(Mutex::new(updatechecker_raw));

    let mut collectors: Vec<Collectors> = vec![
        Collectors::MemCollector,
        Collectors::NetCollector,
        Collectors::ProcCollector,
        Collectors::CpuCollector,
    ];

    let mut boxes: Vec<Boxes> = vec![Boxes::CpuBox, Boxes::MemBox, Boxes::NetBox, Boxes::ProcBox];

    let mut graphs_raw: Graphs = Graphs::default();
    let mut graphs_parent: Arc<Mutex<Graphs>> = Arc::new(Mutex::new(graphs_raw));
    let mut graphs_mutex: Arc<Mutex<Graphs>> = Arc::clone(&graphs_parent);
    let mut graphs: MutexGuard<Graphs> = graphs_mutex.lock().unwrap();

    let mut meters_raw: Meters = Meters::default();
    let mut meters_parent: Arc<Mutex<Meters>> = Arc::new(Mutex::new(meters_raw));
    let mut meters_mutex: Arc<Mutex<Meters>> = Arc::clone(&meters_parent);
    let mut meters: MutexGuard<Meters> = meters_mutex.lock().unwrap();

    errlog("Made it through pre-main".to_owned());

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
    errlog("Switch to alternate screen, clear screen, hide cursor, enable mouse reporting and disable input echo".to_owned());
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
    errlog("Start a thread checking for updates while running init".to_owned());
    if CONFIG.update_check {
        UpdateChecker::checker(Arc::clone(&updatechecker_parent));
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

    errlog("Load theme".to_owned());

    THEME.replace_self(match Theme::from_str(CONFIG.color_theme.clone()) {
        Ok(t) => {
            init.success(&CONFIG, &mut draw, &term, &mut key);
            t
        }
        Err(e) => {
            errlog(format!("Unable to read theme from config (error {})...", e));
            Init::fail(e, &CONFIG, &mut draw, &mut collector, &mut key, &term);
            Theme::default()
        }
    });

    // Setup boxes
    errlog("Setting up boxes".to_owned());
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
    errlog("Setup boxes successfully".to_owned());

    // Setup signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH
    errlog("Setting up signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH".to_owned());
    if CONFIG.show_init {
        errlog("Showing init".to_owned());
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
        errlog("Finished showing init".to_owned());
    }

    let mut signals_unimportant = match Signals::new(&[SIGTSTP, SIGCONT, SIGWINCH]) {
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

    let mut signals_important = match Signals::new(&[SIGINT]) {
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

    let mut ARG_MODE_signal = Arc::clone(&ARG_MODE_parent);
    let mut CONFIG_signal = Arc::clone(&CONFIG_parent);
    let mut THEME_signal = Arc::clone(&THEME_parent);
    let mut term_signal = Arc::clone(&term_parent);
    let mut key_signal = Arc::clone(&key_parent);
    let mut draw_signal = Arc::clone(&draw_parent);
    let mut brshtop_box_signal = Arc::clone(&brshtop_box_parent);
    let mut cpu_box_signal = Arc::clone(&cpu_box_parent);
    let mut mem_box_signal = Arc::clone(&mem_box_parent);
    let mut net_box_signal = Arc::clone(&net_box_parent);
    let mut proc_box_signal = Arc::clone(&proc_box_parent);
    let mut collector_signal = Arc::clone(&collector_parent);
    let mut cpu_collector_signal = Arc::clone(&cpu_collector_parent);
    let mut mem_collector_signal = Arc::clone(&mem_collector_parent);
    let mut net_collector_signal = Arc::clone(&net_collector_parent);
    let mut proc_collector_signal = Arc::clone(&proc_collector_parent);
    let mut menu_signal = Arc::clone(&menu_parent);
    let mut timer_signal = Arc::clone(&timer_parent);
    let mut init_signal = Arc::clone(&init_parent);
    let mut graphs_signal = Arc::clone(&graphs_parent);
    let mut meters_signal = Arc::clone(&meters_parent);
    let mut timeit_signal = Arc::clone(&timeit_parent);
    let mut boxes_signal = boxes.clone();
    let mut collectors_signal = collectors.clone();
    let mut DEBUG_signal = DEBUG.clone();

    thread::spawn(move || {
        for sig in signals_unimportant.forever() {
            match sig {
                SIGTSTP => {
                    match now_sleeping(
                        key_signal.clone(),
                        collector_signal.clone(),
                        draw_signal.clone(),
                        term_signal.clone(),
                    ) {
                        Some(_) => (),
                        None => clean_quit_mutex(
                            None,
                            Some("Failed to pause program".to_owned()),
                            key_signal.clone(),
                            collector_signal.clone(),
                            draw_signal.clone(),
                            term_signal.clone(),
                            CONFIG_signal.clone(),
                        ),
                    }
                }
                SIGCONT => {
                    let ARG_MODE_cont = ARG_MODE_signal.lock().unwrap();
                    now_awake(
                        boxes_signal.clone(),
                        collectors_signal.clone(),
                        DEBUG_signal,
                        ARG_MODE_cont.to_owned(),
                        draw_signal.clone(),
                        term_signal.clone(),
                        key_signal.clone(),
                        brshtop_box_signal.clone(),
                        collector_signal.clone(),
                        init_signal.clone(),
                        cpu_box_signal.clone(),
                        menu_signal.clone(),
                        timer_signal.clone(),
                        CONFIG_signal.clone(),
                        THEME_signal.clone(),
                        timeit_signal.clone(),
                        graphs_signal.clone(),
                        meters_signal.clone(),
                        net_box_signal.clone(),
                        proc_box_signal.clone(),
                        mem_box_signal.clone(),
                        cpu_collector_signal.clone(),
                        mem_collector_signal.clone(),
                        net_collector_signal.clone(),
                        proc_collector_signal.clone(),
                    );
                }
                SIGWINCH => {
                    let mut term_signal_clone = term_signal.clone();
                    let mut collector_signal_clone = collector_signal.clone();
                    let mut init_signal_clone = init_signal.clone();
                    let mut cpu_box_signal_clone = cpu_box_signal.clone();
                    let mut draw_signal_clone = draw_signal.clone();
                    let mut key_signal_clone = key_signal.clone();
                    let mut menu_signal_clone = menu_signal.clone();
                    let mut brshtop_box_signal_clone = brshtop_box_signal.clone();
                    let mut timer_signal_clone = timer_signal.clone();
                    let mut CONFIG_signal_clone = CONFIG_signal.clone();
                    let mut THEME_signal_clone = THEME_signal.clone();
                    let mut cpu_collector_signal_clone = cpu_collector_signal.clone();
                    let mut mem_box_signal_clone = mem_box_signal.clone();
                    let mut net_box_signal_clone = net_box_signal.clone();
                    let mut proc_box_signal_clone = proc_box_signal.clone();
                    let mut term_winch = term_signal_clone.lock().unwrap();
                    let mut collector_winch = collector_signal_clone.lock().unwrap();
                    let mut init_winch = init_signal_clone.lock().unwrap();
                    let mut cpu_box_winch = cpu_box_signal_clone.lock().unwrap();
                    let mut draw_winch = draw_signal_clone.lock().unwrap();
                    let mut key_winch = key_signal_clone.lock().unwrap();
                    let mut menu_winch = menu_signal_clone.lock().unwrap();
                    let mut brshtop_box_winch = brshtop_box_signal_clone.lock().unwrap();
                    let mut timer_winch = timer_signal_clone.lock().unwrap();
                    let mut CONFIG_winch = CONFIG_signal_clone.lock().unwrap();
                    let mut THEME_winch = THEME_signal_clone.lock().unwrap();
                    let mut cpu_collector_winch = cpu_collector_signal_clone.lock().unwrap();
                    let mut mem_box_winch = mem_box_signal_clone.lock().unwrap();
                    let mut net_box_winch = net_box_signal_clone.lock().unwrap();
                    let mut proc_box_winch = proc_box_signal_clone.lock().unwrap();
                    term_winch.refresh(
                        vec![],
                        boxes_signal.clone(),
                        &mut collector_winch,
                        &mut init_winch,
                        &mut cpu_box_winch,
                        &mut draw_winch,
                        true,
                        &mut key_winch,
                        &mut menu_winch,
                        &mut brshtop_box_winch,
                        &mut timer_winch,
                        &mut CONFIG_winch,
                        &mut THEME_winch,
                        &mut cpu_collector_winch,
                        &mut mem_box_winch,
                        &mut net_box_winch,
                        &mut proc_box_winch,
                    );
                }
                _ => unreachable!(),
            }
        }
    });

    let mut key_signal_important = Arc::clone(&key_parent);
    let mut collector_signal_important = Arc::clone(&collector_parent);
    let mut draw_signal_important = Arc::clone(&draw_parent);
    let mut term_signal_important = Arc::clone(&term_parent);
    let mut CONFIG_signal_important = Arc::clone(&CONFIG_parent);

    thread::spawn(move || {
        for sig in signals_important.forever() {
            match sig {
                SIGINT => clean_quit_mutex(
                    None,
                    Some("SIGINT received".to_owned()),
                    key_signal_important.clone(),
                    collector_signal_important.clone(),
                    draw_signal_important.clone(),
                    term_signal_important.clone(),
                    CONFIG_signal_important.clone(),
                ),
                _ => unreachable!(),
            }
        }
    });

    errlog("Setup signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH succesfully".to_owned());

    init.success(&CONFIG, &mut draw, &term, &mut key);

    // Start a separate thread for reading keyboard input
    errlog("Starting a separate thread for reading keyboard input".to_owned());
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

    drop(key);
    drop(draw);
    drop(menu);
    Key::start(
        Arc::clone(&key_parent),
        Arc::clone(&draw_parent),
        Arc::clone(&menu_parent),
    );
    key = key_mutex.lock().unwrap();
    draw = draw_mutex.lock().unwrap();
    menu = menu_mutex.lock().unwrap();

    init.success(&CONFIG, &mut draw, &term, &mut key);
    errlog("Started a separate thread for reading keyboard input successfully".to_owned());

    // Start a separate thread for data collection and drawing
    errlog("Starting a separate thread for data collection and drawing".to_owned());
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

    drop(collector);
    Collector::start(
        Arc::clone(&collector_parent),
        DEBUG,
        ARG_MODE.to_owned(),
        collectors.clone(),
        Arc::clone(&CONFIG_parent),
        Arc::clone(&brshtop_box_parent),
        Arc::clone(&timeit_parent),
        Arc::clone(&menu_parent),
        Arc::clone(&draw_parent),
        Arc::clone(&term_parent),
        Arc::clone(&cpu_box_parent),
        Arc::clone(&key_parent),
        Arc::clone(&THEME_parent),
        Arc::clone(&graphs_parent),
        Arc::clone(&meters_parent),
        Arc::clone(&net_box_parent),
        Arc::clone(&proc_box_parent),
        Arc::clone(&mem_box_parent),
        Arc::clone(&cpu_collector_parent),
        Arc::clone(&mem_collector_parent),
        Arc::clone(&net_collector_parent),
        Arc::clone(&proc_collector_parent),
    );
    collector = collector_mutex.lock().unwrap();
    init.success(&CONFIG, &mut draw, &term, &mut key);
    errlog("Started a separate thread for data collection and drawing successfully".to_owned());

    // Collect data and draw to buffer
    errlog("Collecting data and draw to buffer".to_owned());
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
    errlog("Collected data and drew to buffer successfully".to_owned());

    // Draw to screen
    errlog("Drawing to screen".to_owned());
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

    errlog("Main finished successfully! Moving to main loop.".to_owned());

    // Main loop ------------------------------------------------------------------------------------->

    drop(term);
    drop(key);
    drop(timer);
    drop(collector);
    drop(init);
    drop(cpu_box);
    drop(draw);
    drop(menu);
    drop(brshtop_box);
    drop(CONFIG);
    drop(THEME);
    drop(ARG_MODE);
    drop(proc_box);
    drop(proc_collector);
    drop(net_collector);
    drop(cpu_collector);
    drop(net_box);
    drop(graphs);
    drop(mem_box);

    run(
        boxes.clone(),
        collectors.clone(),
        Arc::clone(&term_parent),
        Arc::clone(&key_parent),
        Arc::clone(&timer_parent),
        Arc::clone(&collector_parent),
        Arc::clone(&init_parent),
        Arc::clone(&cpu_box_parent),
        Arc::clone(&draw_parent),
        Arc::clone(&menu_parent),
        Arc::clone(&brshtop_box_parent),
        Arc::clone(&CONFIG_parent),
        Arc::clone(&THEME_parent),
        Arc::clone(&ARG_MODE_parent),
        Arc::clone(&proc_box_parent),
        Arc::clone(&proc_collector_parent),
        Arc::clone(&net_collector_parent),
        Arc::clone(&cpu_collector_parent),
        Arc::clone(&net_box_parent),
        Arc::clone(&updatechecker_parent),
        Arc::clone(&graphs_parent),
        Arc::clone(&mem_box_parent),
    );
}

pub fn run(
    boxes: Vec<Boxes>,
    collectors: Vec<Collectors>,
    term_mutex: Arc<Mutex<Term>>,
    key_mutex: Arc<Mutex<Key>>,
    timer_mutex: Arc<Mutex<Timer>>,
    collector_mutex: Arc<Mutex<Collector>>,
    init_mutex: Arc<Mutex<Init>>,
    cpu_box_mutex: Arc<Mutex<CpuBox>>,
    draw_mutex: Arc<Mutex<Draw>>,
    menu_mutex: Arc<Mutex<Menu>>,
    brshtop_box_mutex: Arc<Mutex<BrshtopBox>>,
    CONFIG_mutex: Arc<Mutex<Config>>,
    THEME_mutex: Arc<Mutex<Theme>>,
    ARG_MODE_mutex: Arc<Mutex<ViewMode>>,
    procbox_mutex: Arc<Mutex<ProcBox>>,
    proccollector_mutex: Arc<Mutex<ProcCollector>>,
    netcollector_mutex: Arc<Mutex<NetCollector>>,
    cpucollector_mutex: Arc<Mutex<CpuCollector>>,
    netbox_mutex: Arc<Mutex<NetBox>>,
    update_checker_mutex: Arc<Mutex<UpdateChecker>>,
    graphs_mutex: Arc<Mutex<Graphs>>,
    mem_box_mutex: Arc<Mutex<MemBox>>,
) {
    let mut count : u64 = 0;
    loop {
        let mut term = match term_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut key = match key_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut timer = match timer_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut collector = match collector_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut init = match init_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut cpu_box = match cpu_box_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut draw = match draw_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut menu = match menu_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut brshtop_box = match brshtop_box_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut CONFIG = match CONFIG_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut THEME = match THEME_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut ARG_MODE = match ARG_MODE_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut procbox = match procbox_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut proccollector = match proccollector_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut netcollector = match netcollector_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut cpucollector = match cpucollector_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut netbox = match netbox_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut update_checker = match update_checker_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut graphs = match graphs_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        let mut mem_box = match mem_box_mutex.try_lock() {
            Ok(m) => m,
            _ => continue,
        };
        errlog("Locked all modules in main loop successfully...".to_owned());
        count += 1;
        errlog(format!("Successfully locked {} times!", count));

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
            &cpucollector,
            &mut mem_box,
            &mut netbox,
            &mut procbox,
        );

        timer.stamp();

        while timer.not_zero(&CONFIG) {
            if key.input_wait(
                timer.left(&CONFIG).as_secs_f64(),
                false,
                &mut draw,
                &mut term,
            ) {
                process_keys_mutex_guard(
                    boxes.clone(),
                    collectors.clone(),
                    &mut ARG_MODE,
                    &mut key,
                    &mut procbox,
                    &mut collector,
                    &mut proccollector,
                    &mut CONFIG,
                    &mut draw,
                    &mut term,
                    &mut brshtop_box,
                    &mut cpu_box,
                    &mut menu,
                    &mut THEME,
                    &mut netcollector,
                    &mut init,
                    &mut cpucollector,
                    &mut netbox,
                    &mut update_checker,
                    &mut timer,
                    &mut graphs,
                    &mut mem_box,
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

pub fn clean_quit_mutex(
    errcode: Option<i32>,
    errmsg: Option<String>,
    key_mutex: Arc<Mutex<Key>>,
    collector_mutex: Arc<Mutex<Collector>>,
    draw_mutex: Arc<Mutex<Draw>>,
    term_mutex: Arc<Mutex<Term>>,
    CONFIG_mutex: Arc<Mutex<Config>>,
) {
    let mut term = match term_mutex.try_lock() {
        Ok(t) => t,
        Err(_) => {
            emergency_quit(errcode, errmsg);
            unreachable!()
        }
    };
    let mut draw: MutexGuard<Draw> = match draw_mutex.try_lock() {
        Ok(d) => d,
        Err(_) => {
            emergency_quit(errcode, errmsg);
            unreachable!()
        },
    };
    let mut key = key_mutex.lock().unwrap();
    let mut collector = collector_mutex.lock().unwrap();
    let CONFIG = CONFIG_mutex.lock().unwrap();

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

pub fn clean_quit_mutex_guard(
    errcode: Option<i32>,
    errmsg: Option<String>,
    key: &mut MutexGuard<Key>,
    collector: &mut MutexGuard<Collector>,
    draw: &mut MutexGuard<Draw>,
    term: &mut MutexGuard<Term>,
    CONFIG: &mut MutexGuard<Config>,
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

pub fn emergency_quit(
    errcode: Option<i32>,
    errmsg: Option<String>,
) {
    let mut draw = Draw::new();
    let mut term = Term::new();
    draw.now_without_key(
        vec![
            term.get_clear(),
            term.get_normal_screen(),
            term.get_show_cursor(),
            term.get_mouse_off(),
            term.get_mouse_direct_off(),
            Term::title(String::default()),
        ]
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

pub fn process_keys_mutex(
    boxes: Vec<Boxes>,
    collectors: Vec<Collectors>,
    ARG_MODE_mutex: Arc<Mutex<ViewMode>>,
    key_class_mutex: Arc<Mutex<Key>>,
    procbox_mutex: Arc<Mutex<ProcBox>>,
    collector_mutex: Arc<Mutex<Collector>>,
    proccollector_mutex: Arc<Mutex<ProcCollector>>,
    CONFIG_mutex: Arc<Mutex<Config>>,
    draw_mutex: Arc<Mutex<Draw>>,
    term_mutex: Arc<Mutex<Term>>,
    brshtop_box_mutex: Arc<Mutex<BrshtopBox>>,
    cpu_box_mutex: Arc<Mutex<CpuBox>>,
    menu_mutex: Arc<Mutex<Menu>>,
    THEME_mutex: Arc<Mutex<Theme>>,
    netcollector_mutex: Arc<Mutex<NetCollector>>,
    init_mutex: Arc<Mutex<Init>>,
    cpucollector_mutex: Arc<Mutex<CpuCollector>>,
    netbox_mutex: Arc<Mutex<NetBox>>,
    update_checker_mutex: Arc<Mutex<UpdateChecker>>,
    timer_mutex: Arc<Mutex<Timer>>,
    graphs_mutex: Arc<Mutex<Graphs>>,
    mem_box_mutex: Arc<Mutex<MemBox>>,
) {
    let mut ARG_MODE = ARG_MODE_mutex.lock().unwrap();
    let mut key_class = key_class_mutex.lock().unwrap();
    let mut procbox = procbox_mutex.lock().unwrap();
    let mut collector = collector_mutex.lock().unwrap();
    let mut proccollector = proccollector_mutex.lock().unwrap();
    let mut CONFIG = CONFIG_mutex.lock().unwrap();
    let mut draw = draw_mutex.lock().unwrap();
    let mut term = term_mutex.lock().unwrap();
    let mut brshtop_box = brshtop_box_mutex.lock().unwrap();
    let mut cpu_box = cpu_box_mutex.lock().unwrap();
    let mut menu = menu_mutex.lock().unwrap();
    let mut THEME = THEME_mutex.lock().unwrap();
    let mut netcollector = netcollector_mutex.lock().unwrap();
    let mut init = init_mutex.lock().unwrap();
    let mut cpucollector = cpucollector_mutex.lock().unwrap();
    let mut netbox = netbox_mutex.lock().unwrap();
    let mut update_checker = update_checker_mutex.lock().unwrap();
    let mut timer = timer_mutex.lock().unwrap();
    let mut graphs = graphs_mutex.lock().unwrap();
    let mut mem_box = mem_box_mutex.lock().unwrap();

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
            if filtered {}
            filtered = true;
            continue;
        }

        if key == "_null".to_owned() {
            continue;
        } else if key == "q".to_owned() {
            clean_quit_mutex_guard(
                None,
                None,
                &mut key_class,
                &mut collector,
                &mut draw,
                &mut term,
                &mut CONFIG,
            );
        } else if key == "+" && CONFIG.update_ms + 100 <= 86399900 {
            CONFIG.update_ms += 100;
            brshtop_box.draw_update_ms(
                false,
                &mut CONFIG,
                &mut cpu_box,
                &mut key_class,
                &mut draw,
                &menu,
                &THEME,
                &term,
            );
        } else if key == "-".to_owned() && CONFIG.update_ms - 100 >= 100 {
            CONFIG.update_ms -= 100;
            brshtop_box.draw_update_ms(
                false,
                &mut CONFIG,
                &mut cpu_box,
                &mut key_class,
                &mut draw,
                &menu,
                &THEME,
                &term,
            );
        } else if vec!["b", "n"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            netcollector.switch(key, &mut collector);
        } else if vec!["M", "escape"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.main(
                &mut draw,
                &mut term,
                &update_checker,
                &mut THEME,
                &mut key_class,
                &mut timer,
                &mut collector,
                collectors.clone(),
                &mut CONFIG,
                &mut ARG_MODE,
                &mut netcollector,
                &mut brshtop_box,
                &mut init,
                &mut cpu_box,
                &mut cpucollector,
                boxes.clone(),
                &mut netbox,
                &mut proccollector,
                &mut mem_box,
                &mut procbox,
            );
        } else if vec!["o", "f2"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.options(
                &mut ARG_MODE,
                &mut THEME,
                &mut draw,
                &mut term,
                &mut CONFIG,
                &mut key_class,
                &mut timer,
                &mut netcollector,
                &mut brshtop_box,
                boxes.clone(),
                &mut collector,
                &mut init,
                &mut cpu_box,
                &mut cpucollector,
                &mut netbox,
                &mut proccollector,
                collectors.clone(),
                &mut procbox,
                &mut mem_box,
            );
        } else if vec!["h", "f1"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.help(
                &THEME,
                &mut draw,
                &term,
                &mut key_class,
                &mut collector,
                collectors.clone(),
                &CONFIG,
                &mut timer,
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
                &mut collector,
                &mut init,
                &mut cpu_box,
                &mut draw,
                true,
                &mut key_class,
                &mut menu,
                &mut brshtop_box,
                &mut timer,
                &mut CONFIG,
                &mut THEME,
                &mut cpucollector,
                &mut mem_box,
                &mut netbox,
                &mut procbox,
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
                &mut proccollector,
                &mut key_class,
                &mut collector,
                &mut CONFIG,
            );
        }
    }
}

pub fn process_keys_mutex_guard(
    boxes: Vec<Boxes>,
    collectors: Vec<Collectors>,
    ARG_MODE: &mut MutexGuard<ViewMode>,
    key_class: &mut MutexGuard<Key>,
    procbox: &mut MutexGuard<ProcBox>,
    collector: &mut MutexGuard<Collector>,
    proccollector: &mut MutexGuard<ProcCollector>,
    CONFIG: &mut MutexGuard<Config>,
    draw: &mut MutexGuard<Draw>,
    term: &mut MutexGuard<Term>,
    brshtop_box: &mut MutexGuard<BrshtopBox>,
    cpu_box: &mut MutexGuard<CpuBox>,
    menu: &mut MutexGuard<Menu>,
    THEME: &mut MutexGuard<Theme>,
    netcollector: &mut MutexGuard<NetCollector>,
    init: &mut MutexGuard<Init>,
    cpucollector: &mut MutexGuard<CpuCollector>,
    netbox: &mut MutexGuard<NetBox>,
    update_checker: &mut MutexGuard<UpdateChecker>,
    timer: &mut MutexGuard<Timer>,
    graphs: &mut MutexGuard<Graphs>,
    mem_box: &mut MutexGuard<MemBox>,
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
            if filtered {}
            filtered = true;
            continue;
        }

        if key == "_null".to_owned() {
            continue;
        } else if key == "q".to_owned() {
            clean_quit_mutex_guard(None, None, key_class, collector, draw, term, CONFIG);
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
    key_mutex: Arc<Mutex<Key>>,
    collector_mutex: Arc<Mutex<Collector>>,
    draw_mutex: Arc<Mutex<Draw>>,
    term_mutex: Arc<Mutex<Term>>,
) -> Option<()> {
    let mut key = key_mutex.lock().unwrap();
    let mut collector = collector_mutex.lock().unwrap();
    let mut draw = draw_mutex.lock().unwrap();
    let mut term = term_mutex.lock().unwrap();

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
    boxes: Vec<Boxes>,
    collectors: Vec<Collectors>,
    DEBUG: bool,
    ARG_MODE: ViewMode,
    draw_mutex: Arc<Mutex<Draw>>,
    term_mutex: Arc<Mutex<Term>>,
    key_mutex: Arc<Mutex<Key>>,
    brshtop_box_mutex: Arc<Mutex<BrshtopBox>>,
    collector_mutex: Arc<Mutex<Collector>>,
    init_mutex: Arc<Mutex<Init>>,
    cpu_box_mutex: Arc<Mutex<CpuBox>>,
    menu_mutex: Arc<Mutex<Menu>>,
    timer_mutex: Arc<Mutex<Timer>>,
    CONFIG_mutex: Arc<Mutex<Config>>,
    THEME_mutex: Arc<Mutex<Theme>>,
    timeit_mutex: Arc<Mutex<TimeIt>>,
    graphs_mutex: Arc<Mutex<Graphs>>,
    meters_mutex: Arc<Mutex<Meters>>,
    netbox_mutex: Arc<Mutex<NetBox>>,
    procbox_mutex: Arc<Mutex<ProcBox>>,
    membox_mutex: Arc<Mutex<MemBox>>,
    cpu_collector_mutex: Arc<Mutex<CpuCollector>>,
    mem_collector_mutex: Arc<Mutex<MemCollector>>,
    net_collector_mutex: Arc<Mutex<NetCollector>>,
    proc_collector_mutex: Arc<Mutex<ProcCollector>>,
) {
    let mut draw = draw_mutex.lock().unwrap();
    let mut term = term_mutex.lock().unwrap();
    let mut key = key_mutex.lock().unwrap();
    let mut brshtop_box = brshtop_box_mutex.lock().unwrap();
    let mut collector = collector_mutex.lock().unwrap();
    let mut init = init_mutex.lock().unwrap();
    let mut cpu_box = cpu_box_mutex.lock().unwrap();
    let mut menu = menu_mutex.lock().unwrap();
    let mut timer = timer_mutex.lock().unwrap();
    let mut CONFIG = CONFIG_mutex.lock().unwrap();
    let mut THEME = THEME_mutex.lock().unwrap();
    let mut netbox = netbox_mutex.lock().unwrap();
    let mut procbox = procbox_mutex.lock().unwrap();
    let mut membox = membox_mutex.lock().unwrap();
    let mut cpu_collector = cpu_collector_mutex.lock().unwrap();

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

    drop(key);
    drop(draw);
    drop(menu);
    Key::start(
        Arc::clone(&key_mutex),
        Arc::clone(&draw_mutex),
        Arc::clone(&menu_mutex),
    );
    key = key_mutex.lock().unwrap();
    draw = draw_mutex.lock().unwrap();
    menu = menu_mutex.lock().unwrap();

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
        &mut CONFIG,
        &mut THEME,
        &mut cpu_collector,
        &mut membox,
        &mut netbox,
        &mut procbox,
    );

    brshtop_box.calc_sizes(
        boxes.clone(),
        &mut term,
        &mut CONFIG,
        &mut cpu_collector,
        &mut cpu_box,
        &mut membox,
        &mut netbox,
        &mut procbox,
    );
    brshtop_box.draw_bg(
        true,
        &mut draw,
        boxes.clone(),
        &mut menu,
        &mut CONFIG,
        &mut cpu_box,
        &mut membox,
        &mut netbox,
        &mut procbox,
        &mut key,
        &mut THEME,
        &mut term,
    );

    Collector::start(
        Arc::clone(&collector_mutex),
        DEBUG,
        ARG_MODE.to_owned(),
        collectors.clone(),
        Arc::clone(&CONFIG_mutex),
        Arc::clone(&brshtop_box_mutex),
        Arc::clone(&timeit_mutex),
        Arc::clone(&menu_mutex),
        Arc::clone(&draw_mutex),
        Arc::clone(&term_mutex),
        Arc::clone(&cpu_box_mutex),
        Arc::clone(&key_mutex),
        Arc::clone(&THEME_mutex),
        Arc::clone(&graphs_mutex),
        Arc::clone(&meters_mutex),
        Arc::clone(&netbox_mutex),
        Arc::clone(&procbox_mutex),
        Arc::clone(&membox_mutex),
        Arc::clone(&cpu_collector_mutex),
        Arc::clone(&mem_collector_mutex),
        Arc::clone(&net_collector_mutex),
        Arc::clone(&proc_collector_mutex),
    );
}

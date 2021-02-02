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
    let mut mutex_CONFIG: Mutex<Config> = Mutex::new(CONFIG);
    let mut passable_CONFIG: OnceCell<Mutex<Config>> = OnceCell::new();
    passable_CONFIG.set(mutex_CONFIG);

    errlog(format!(
        "New instance of brshtop version {} started with pid {}",
        VERSION.to_owned(),
        std::process::id()
    ));
    errlog(format!(
        "Loglevel set to {} (even though, currently, this doesn't work)",
        passable_CONFIG.get().unwrap().lock().unwrap().log_level
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
    let mut passable_THEME: OnceCell<Mutex<Theme>> = OnceCell::new();
    passable_THEME.set(mutex_THEME);

    //println!("Made it through global variables");

    // Pre main ---------------------------------------------------------------------------------------------
    let mut term: Term = Term::new();
    let mut mutex_term: Mutex<Term> = Mutex::new(term);
    let mut passable_term: OnceCell<Mutex<Term>> = OnceCell::new();
    passable_term.set(mutex_term);

    let mut key: Key = Key::new();
    let mut mutex_key: Mutex<Key> = Mutex::new(key);
    let mut passable_key: OnceCell<Mutex<Key>> = OnceCell::new();
    passable_key.set(mutex_key);

    let mut draw: Draw = Draw::new();
    let mut mutex_draw: Mutex<Draw> = Mutex::new(draw);
    let mut passable_draw: OnceCell<Mutex<Draw>> = OnceCell::new();
    passable_draw.set(mutex_draw);

    let mut brshtop_box: BrshtopBox = BrshtopBox::new(&passable_CONFIG, ARG_MODE);
    let mut mutex_brshtop_box: Mutex<BrshtopBox> = Mutex::new(brshtop_box);
    let mut passable_brshtop_box: OnceCell<Mutex<BrshtopBox>> = OnceCell::new();
    passable_brshtop_box.set(mutex_brshtop_box);

    let mut cpu_box: CpuBox = CpuBox::new(&passable_brshtop_box, &passable_CONFIG, ARG_MODE);
    let mut mutex_cpu_box: Mutex<CpuBox> = Mutex::new(cpu_box);
    let mut passable_cpu_box: OnceCell<Mutex<CpuBox>> = OnceCell::new();
    passable_cpu_box.set(mutex_cpu_box);

    let mut mem_box: MemBox = MemBox::new(&passable_brshtop_box, &passable_CONFIG, ARG_MODE);
    let mut mutex_mem_box: Mutex<MemBox> = Mutex::new(mem_box);
    let mut passable_mem_box: OnceCell<Mutex<MemBox>> = OnceCell::new();
    passable_mem_box.set(mutex_mem_box);

    let mut net_box: NetBox = NetBox::new(&passable_CONFIG, ARG_MODE, &passable_brshtop_box);
    let mut mutex_net_box: Mutex<NetBox> = Mutex::new(net_box);
    let mut passable_net_box: OnceCell<Mutex<NetBox>> = OnceCell::new();
    passable_net_box.set(mutex_net_box);

    let mut proc_box: ProcBox = ProcBox::new(&passable_brshtop_box, &passable_CONFIG, ARG_MODE);
    let mut mutex_proc_box: Mutex<ProcBox> = Mutex::new(proc_box);
    let mut passable_proc_box: OnceCell<Mutex<ProcBox>> = OnceCell::new();
    passable_proc_box.set(mutex_proc_box);

    let mut collector: Collector = Collector::new();
    let mut mutex_collector: Mutex<Collector> = Mutex::new(collector);
    let mut passable_collector: OnceCell<Mutex<Collector>> = OnceCell::new();
    passable_collector.set(mutex_collector);

    let mut cpu_collector: CpuCollector = CpuCollector::new();
    let mut mutex_cpu_collector: Mutex<CpuCollector> = Mutex::new(cpu_collector);
    let mut passable_cpu_collector: OnceCell<Mutex<CpuCollector>> = OnceCell::new();
    passable_cpu_collector.set(mutex_cpu_collector);

    let mut mem_collector: MemCollector = MemCollector::new(&passable_mem_box);
    let mut mutex_mem_collector: Mutex<MemCollector> = Mutex::new(mem_collector);
    let mut passable_mem_collector: OnceCell<Mutex<MemCollector>> = OnceCell::new();
    passable_mem_collector.set(mutex_mem_collector);

    let mut net_collector: NetCollector = NetCollector::new(&passable_net_box, &passable_CONFIG);
    let mut mutex_net_collector: Mutex<NetCollector> = Mutex::new(net_collector);
    let mut passable_net_collector: OnceCell<Mutex<NetCollector>> = OnceCell::new();
    passable_net_collector.set(mutex_net_collector);

    let mut proc_collector: ProcCollector = ProcCollector::new(&passable_proc_box);
    let mut mutex_proc_collector: Mutex<ProcCollector> = Mutex::new(proc_collector);
    let mut passable_proc_collector: OnceCell<Mutex<ProcCollector>> = OnceCell::new();
    passable_proc_collector.set(mutex_proc_collector);

    let mut menu: Menu = Menu::new(MENUS, MENU_COLORS);
    let mut mutex_menu: Mutex<Menu> = Mutex::new(menu);
    let mut passable_menu: OnceCell<Mutex<Menu>> = OnceCell::new();
    passable_menu.set(mutex_menu);

    let mut timer: Timer = Timer::new();
    let mut mutex_timer: Mutex<Timer> = Mutex::new(timer);
    let mut passable_timer: OnceCell<Mutex<Timer>> = OnceCell::new();
    passable_timer.set(mutex_timer);

    let mut timeit: TimeIt = TimeIt::new();
    let mut mutex_timeit: Mutex<TimeIt> = Mutex::new(timeit);
    let mut passable_timeit: OnceCell<Mutex<TimeIt>> = OnceCell::new();
    passable_timeit.set(mutex_timeit);

    let mut init: Init = Init::new();
    let mut mutex_init: Mutex<Init> = Mutex::new(init);
    let mut passable_init: OnceCell<Mutex<Init>> = OnceCell::new();
    passable_init.set(mutex_init);

    let mut updatechecker: UpdateChecker = UpdateChecker::new();
    let mut mutex_updatechecker: Mutex<UpdateChecker> = Mutex::new(updatechecker);
    let mut passable_updatechecker: OnceCell<Mutex<UpdateChecker>> = OnceCell::new();
    passable_updatechecker.set(mutex_updatechecker);

    let mut collectors: Vec<Collectors> = vec![
        Collectors::MemCollector,
        Collectors::NetCollector,
        Collectors::ProcCollector,
        Collectors::CpuCollector,
    ];

    let mut boxes: Vec<Boxes> = vec![Boxes::CpuBox, Boxes::MemBox, Boxes::NetBox, Boxes::ProcBox];

    let mut graphs: Graphs = Graphs::default();
    let mut mutex_graphs: Mutex<Graphs> = Mutex::new(graphs);
    let mut passable_graphs: OnceCell<Mutex<Graphs>> = OnceCell::new();
    passable_graphs.set(mutex_graphs);

    let mut meters: Meters = Meters::default();
    let mut mutex_meters: Mutex<Meters> = Mutex::new(meters);
    let mut passable_meters: OnceCell<Mutex<Meters>> = OnceCell::new();
    passable_meters.set(mutex_meters);

    //println!("Made it through pre-main");

    // Main -----------------------------------------------------------------------------------------------

    let term_size = terminal_size();
    match term_size {
        Some((Width(w), Height(h))) => {
            &passable_term.get().unwrap().lock().unwrap().set_width(w);
            &passable_term.get().unwrap().lock().unwrap().set_height(h);
        }
        None => error::throw_error("Unable to get size of terminal!"),
    };

    // Init ----------------------------------------------------------------------------------

    if DEBUG {
        let mut init_timeit = passable_timeit.get().unwrap().lock().unwrap();
        init_timeit.start("Init".to_owned());
        drop(init_timeit);
    }

    // Switch to alternate screen, clear screen, hide cursor, enable mouse reporting and disable input echo
    let mut init_term = passable_term.get().unwrap().lock().unwrap();
    let mut init_draw = passable_draw.get().unwrap().lock().unwrap();

    init_draw.now(
        vec![
            init_term.get_alt_screen(),
            init_term.get_clear(),
            init_term.get_hide_cursor(),
            init_term.get_mouse_on(),
            Term::title("BRShtop".to_owned()),
        ],
        &passable_key,
    );

    Term::echo(false);

    drop(init_draw);
    init_term.refresh(
        vec![],
        boxes.clone(),
        &passable_collector,
        &passable_init,
        &passable_cpu_box,
        &passable_draw,
        true,
        &passable_key,
        &passable_menu,
        &passable_brshtop_box,
        &passable_timer,
        &passable_CONFIG,
        &passable_THEME,
        &passable_cpu_collector,
        &passable_mem_box,
        &passable_net_box,
        &passable_proc_box,
    );

    // Start a thread checking for updates while running init
    let mut init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
    let mut init_updatechecker = passable_updatechecker.get().unwrap().lock().unwrap();
    if init_CONFIG.update_check {
        init_updatechecker.run();
    }

    // Draw banner and init status
    let mut init_init = passable_init.get().unwrap().lock().unwrap();
    if init_CONFIG.show_init && !init_init.resized {
        drop(init_term);
        init_init.start(&passable_draw, &passable_key, &passable_term);
    }

    // Load theme
    if init_CONFIG.show_init {
        init_draw = passable_draw.get().unwrap().lock().unwrap();
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
    }
    THEME = match Theme::from_str(init_CONFIG.color_theme.clone()) {
        Ok(t) => {
            drop(init_CONFIG);
            init_init.success(
                &passable_CONFIG,
                &passable_draw,
                &passable_term,
                &passable_key,
            );
            init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
            t
        }
        Err(e) => {
            errlog(format!("Unable to read theme from config (error {})...", e));
            Init::fail(
                e,
                &passable_CONFIG,
                &passable_draw,
                &passable_collector,
                &passable_key,
                &passable_term,
            );
            Theme::default()
        }
    };

    // Setup boxes
    if init_CONFIG.show_init {
        init_draw = passable_draw.get().unwrap().lock().unwrap();
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
        if init_CONFIG.check_temp {
            let mut init_cpu_collector = passable_cpu_collector.get().unwrap().lock().unwrap();

            drop(init_CONFIG);
            init_cpu_collector.get_sensors(&passable_CONFIG);
            init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
        }
        let mut init_brshtop_box = passable_brshtop_box.get().unwrap().lock().unwrap();

        drop(init_CONFIG);
        init_brshtop_box.calc_sizes(
            boxes.clone(),
            &passable_term,
            &passable_CONFIG,
            &passable_cpu_collector,
            &passable_cpu_box,
            &passable_mem_box,
            &passable_net_box,
            &passable_proc_box,
        );
        drop(init_draw);
        init_brshtop_box.draw_bg(
            false,
            &passable_draw,
            boxes.clone(),
            &passable_menu,
            &passable_CONFIG,
            &passable_cpu_box,
            &passable_mem_box,
            &passable_net_box,
            &passable_proc_box,
            &passable_key,
            &passable_THEME,
            &passable_term,
        );
        init_init.success(
            &passable_CONFIG,
            &passable_draw,
            &passable_term,
            &passable_key,
        );
    }

    // Setup signal handlers for SIGSTP, SIGCONT, SIGINT and SIGWINCH
    init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
    if init_CONFIG.show_init {
        init_draw = passable_draw.get().unwrap().lock().unwrap();
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
    }

    drop(init_CONFIG);
    let mut signals = match Signals::new(&[SIGTSTP, SIGCONT, SIGINT, SIGWINCH]) {
        //Handling ctrl-z, resume, ctrl-c, terminal resized
        Ok(s) => s,
        Err(e) => {
            Init::fail(
                e.to_string(),
                &passable_CONFIG,
                &passable_draw,
                &passable_collector,
                &passable_key,
                &passable_term,
            );
            return;
        }
    };
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
                            &passable_CONFIG,
                            None,
                        ),
                    },
                    SIGCONT => now_awake(
                        &passable_draw,
                        &passable_term,
                        &passable_key,
                        &passable_brshtop_box,
                        &passable_collector,
                        boxes.clone(),
                        &passable_init,
                        &passable_cpu_box,
                        &passable_menu,
                        &passable_timer,
                        &passable_CONFIG,
                        &passable_THEME,
                        DEBUG,
                        collectors.clone(),
                        &passable_timeit,
                        ARG_MODE,
                        &passable_graphs,
                        &passable_meters,
                        &passable_net_box,
                        &passable_proc_box,
                        &passable_mem_box,
                        &passable_cpu_collector,
                        &passable_mem_collector,
                        &passable_net_collector,
                        &passable_proc_collector,
                        &passable_mem_box,
                    ),
                    SIGINT => clean_quit(
                        None,
                        None,
                        &passable_key,
                        &passable_collector,
                        &passable_draw,
                        &passable_term,
                        &passable_CONFIG,
                        None,
                    ),
                    SIGWINCH => {
                        let mut SIG_term = passable_term.get().unwrap().lock().unwrap();
                        SIG_term.refresh(
                        vec![],
                        boxes.clone(),
                        &passable_collector,
                        &passable_init,
                        &passable_cpu_box,
                        &passable_draw,
                        true,
                        &passable_key,
                        &passable_menu,
                        &passable_brshtop_box,
                        &passable_timer,
                        &passable_CONFIG,
                        &passable_THEME,
                        &passable_cpu_collector,
                        &passable_mem_box,
                        &passable_net_box,
                        &passable_proc_box,
                    );
                    drop(SIG_term);
                },
                    _ => unreachable!(),
                }
            }
        });
    }) {
        _ => (),
    };

    init_init.success(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Start a separate thread for reading keyboard input
    init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
    if init_CONFIG.show_init {
        init_draw = passable_draw.get().unwrap().lock().unwrap();
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
    }
    let mut init_key = passable_key
    .get()
    .unwrap()
    .lock()
    .unwrap();
    
    init_key.start(&passable_draw, &passable_menu);

    drop(init_CONFIG);
    drop(init_key);
    init_init.success(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Start a separate thread for data collection and drawing
    init_CONFIG = passable_CONFIG.get().unwrap().lock().unwrap();
    if init_CONFIG.show_init {
        init_draw = passable_draw.get().unwrap().lock().unwrap();
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
    }
    passable_collector.get().unwrap().lock().unwrap().start(
        &passable_CONFIG,
        DEBUG,
        collectors.clone(),
        &passable_brshtop_box,
        &passable_timeit,
        &passable_menu,
        &passable_draw,
        &passable_term,
        &passable_cpu_box,
        &passable_key,
        &passable_THEME,
        ARG_MODE,
        &passable_graphs,
        &passable_meters,
        &passable_net_box,
        &passable_proc_box,
        &passable_mem_box,
        &passable_cpu_collector,
        &passable_mem_collector,
        &passable_net_collector,
        &passable_proc_collector,
        &passable_collector,
    );
    passable_init.get().unwrap().lock().unwrap().success(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Collect data and draw to buffer
    if passable_CONFIG.get().unwrap().lock().unwrap().show_init {
        passable_draw.get().unwrap().lock().unwrap().buffer(
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
    }
    passable_collector.get().unwrap().lock().unwrap().collect(
        collectors.clone(),
        &passable_CONFIG,
        false,
        false,
        false,
        false,
        false,
    );
    passable_init.get().unwrap().lock().unwrap().success(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    // Draw to screen
    if passable_CONFIG.get().unwrap().lock().unwrap().show_init {
        passable_draw.get().unwrap().lock().unwrap().buffer(
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
    }
    passable_collector
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .set_collect_done(EventEnum::Wait);
    passable_collector
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .get_collect_done_reference()
        .wait(-1.0);
    passable_init.get().unwrap().lock().unwrap().success(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );

    passable_init.get().unwrap().lock().unwrap().done(
        &passable_CONFIG,
        &passable_draw,
        &passable_term,
        &passable_key,
    );
    &passable_term.get().unwrap().lock().unwrap().refresh(
        vec![],
        boxes.clone(),
        &passable_collector,
        &passable_init,
        &passable_cpu_box,
        &passable_draw,
        false,
        &passable_key,
        &passable_menu,
        &passable_brshtop_box,
        &passable_timer,
        &passable_CONFIG,
        &passable_THEME,
        &passable_cpu_collector,
        &passable_mem_box,
        &passable_net_box,
        &passable_proc_box,
    );
    passable_draw
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .out(vec![], true, &passable_key);
    if passable_CONFIG
        .get()
        .unwrap()
        .lock()
        .unwrap()
        .draw_clock
        .len()
        > 0
    {
        passable_brshtop_box
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .set_clock_on(true);
    }
    if DEBUG {
        passable_timeit
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .stop("Init".to_owned());
    }

    // Main loop ------------------------------------------------------------------------------------->
    run(
        &passable_term,
        &passable_key,
        &passable_timer,
        &passable_collector,
        boxes.clone(),
        &passable_init,
        &passable_cpu_box,
        &passable_draw,
        &passable_menu,
        &passable_brshtop_box,
        &passable_CONFIG,
        &passable_THEME,
        &mut ARG_MODE,
        &passable_proc_box,
        &passable_proc_collector,
        &passable_net_collector,
        &passable_cpu_collector,
        &passable_net_box,
        &passable_updatechecker,
        collectors.clone(),
        &passable_mem_collector,
        &passable_graphs,
        &passable_mem_box,
    );
}

pub fn run(
    term: &OnceCell<Mutex<Term>>,
    key: &OnceCell<Mutex<Key>>,
    timer: &OnceCell<Mutex<Timer>>,
    collector: &OnceCell<Mutex<Collector>>,
    boxes: Vec<Boxes>,
    init: &OnceCell<Mutex<Init>>,
    cpu_box: &OnceCell<Mutex<CpuBox>>,
    draw: &OnceCell<Mutex<Draw>>,
    menu: &OnceCell<Mutex<Menu>>,
    brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
    CONFIG: &OnceCell<Mutex<Config>>,
    THEME: &OnceCell<Mutex<Theme>>,
    ARG_MODE: &mut ViewMode,
    procbox: &OnceCell<Mutex<ProcBox>>,
    proccollector: &OnceCell<Mutex<ProcCollector>>,
    netcollector: &OnceCell<Mutex<NetCollector>>,
    cpucollector: &OnceCell<Mutex<CpuCollector>>,
    netbox: &OnceCell<Mutex<NetBox>>,
    update_checker: &OnceCell<Mutex<UpdateChecker>>,
    collectors: Vec<Collectors>,
    memcollector: &OnceCell<Mutex<MemCollector>>,
    graphs: &OnceCell<Mutex<Graphs>>,
    mem_box: &OnceCell<Mutex<MemBox>>,
) {
    loop {
        term.get().unwrap().lock().unwrap().refresh(
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
        timer.get().unwrap().lock().unwrap().stamp();

        while timer.get().unwrap().lock().unwrap().not_zero(&CONFIG) {
            if key.get().unwrap().lock().unwrap().input_wait(
                timer
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .left(CONFIG)
                    .as_secs_f64(),
                false,
                draw,
                term,
                key,
            ) {
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
                    memcollector,
                    graphs,
                    mem_box,
                    procbox,
                )
            }
        }

        collector.get().unwrap().lock().unwrap().collect(
            collectors.clone(),
            CONFIG,
            true,
            false,
            false,
            false,
            false,
        );
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
    term: &OnceCell<Mutex<Term>>,
    THEME: &OnceCell<Mutex<Theme>>,
    brshtop_box: Option<&OnceCell<Mutex<BrshtopBox>>>,
    cpu_box: Option<&OnceCell<Mutex<CpuBox>>>,
    mem_box: Option<&OnceCell<Mutex<MemBox>>>,
    net_box: Option<&OnceCell<Mutex<NetBox>>>,
    proc_box: Option<&OnceCell<Mutex<ProcBox>>>,
) -> String {
    let mut out: String = format!(
        "{}{}",
        term.get().unwrap().lock().unwrap().get_fg(),
        term.get().unwrap().lock().unwrap().get_bg()
    );
    let mut lc: Color = match line_color {
        Some(c) => c,
        None => THEME.get().unwrap().lock().unwrap().colors.div_line,
    };
    let mut tc: Color = match title_color {
        Some(c) => c,
        None => THEME.get().unwrap().lock().unwrap().colors.title,
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
                wx = brshtop_box.unwrap().get().unwrap().lock().unwrap().get_x();
                wy = brshtop_box.unwrap().get().unwrap().lock().unwrap().get_y();
                ww = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_width();
                wh = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_height();
                wt = brshtop_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_name();
            }
            Boxes::CpuBox => {
                let parent_box = cpu_box.unwrap().get().unwrap().lock().unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::MemBox => {
                let parent_box = mem_box.unwrap().get().unwrap().lock().unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::NetBox => {
                let parent_box = net_box.unwrap().get().unwrap().lock().unwrap().get_parent();
                wx = parent_box.get_x();
                wy = parent_box.get_y();
                ww = parent_box.get_width();
                wh = parent_box.get_height();
                wt = parent_box.get_name();
            }
            Boxes::ProcBox => {
                let parent_box = proc_box
                    .unwrap()
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_parent();
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
    key: &OnceCell<Mutex<Key>>,
    collector: &OnceCell<Mutex<Collector>>,
    draw: &OnceCell<Mutex<Draw>>,
    term: &OnceCell<Mutex<Term>>,
    CONFIG: &OnceCell<Mutex<Config>>,
    SELF_START: Option<SystemTime>,
) {
    key.get().unwrap().lock().unwrap().stop();
    collector.get().unwrap().lock().unwrap().stop();
    if errcode == None {
        CONFIG.get().unwrap().lock().unwrap().save_config();
    }
    draw.get().unwrap().lock().unwrap().now(
        vec![
            term.get().unwrap().lock().unwrap().get_clear(),
            term.get().unwrap().lock().unwrap().get_normal_screen(),
            term.get().unwrap().lock().unwrap().get_show_cursor(),
            term.get().unwrap().lock().unwrap().get_mouse_off(),
            term.get().unwrap().lock().unwrap().get_mouse_direct_off(),
            Term::title(String::default()),
        ],
        key,
    );
    Term::echo(true);
    let now = SystemTime::now();
    match errcode {
        Some(0) => errlog(format!(
            "Exiting, Runtime {} \n",
            now.duration_since(SELF_START.unwrap())
                .unwrap()
                .as_secs_f64()
        )),
        Some(n) => {
            errlog(format!(
                "Exiting with errorcode {}, Runtime {} \n",
                n,
                now.duration_since(SELF_START.unwrap())
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
    key_class: &OnceCell<Mutex<Key>>,
    procbox: &OnceCell<Mutex<ProcBox>>,
    collector: &OnceCell<Mutex<Collector>>,
    proccollector: &OnceCell<Mutex<ProcCollector>>,
    CONFIG: &OnceCell<Mutex<Config>>,
    draw: &OnceCell<Mutex<Draw>>,
    term: &OnceCell<Mutex<Term>>,
    brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
    cpu_box: &OnceCell<Mutex<CpuBox>>,
    menu: &OnceCell<Mutex<Menu>>,
    THEME: &OnceCell<Mutex<Theme>>,
    netcollector: &OnceCell<Mutex<NetCollector>>,
    init: &OnceCell<Mutex<Init>>,
    cpucollector: &OnceCell<Mutex<CpuCollector>>,
    boxes: Vec<Boxes>,
    netbox: &OnceCell<Mutex<NetBox>>,
    update_checker: &OnceCell<Mutex<UpdateChecker>>,
    collectors: Vec<Collectors>,
    timer: &OnceCell<Mutex<Timer>>,
    memcollector: &OnceCell<Mutex<MemCollector>>,
    graphs: &OnceCell<Mutex<Graphs>>,
    mem_box: &OnceCell<Mutex<MemBox>>,
    proc_box: &OnceCell<Mutex<ProcBox>>,
) {
    let mut mouse_pos: (i32, i32) = (0, 0);
    let mut filtered: bool = false;
    while key_class.get().unwrap().lock().unwrap().has_key() {
        let mut key = match key_class.get().unwrap().lock().unwrap().get() {
            Some(k) => k.clone(),
            None => return,
        };
        if vec!["mouse_scroll_up", "mouse_scroll_down", "mouse_click"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            mouse_pos = key_class.get().unwrap().lock().unwrap().get_mouse();
            if mouse_pos.0 >= procbox.get().unwrap().lock().unwrap().get_parent().get_x() as i32
                && procbox.get().unwrap().lock().unwrap().get_current_y() as i32 + 1 <= mouse_pos.1
                && mouse_pos.1
                    < procbox.get().unwrap().lock().unwrap().get_current_y() as i32
                        + procbox.get().unwrap().lock().unwrap().get_current_h() as i32
                        - 1
            {
                ()
            } else if key == "mouse_click".to_owned() {
                key = "mouse_unselect".to_owned()
            } else {
                key = "_null".to_owned()
            }
        }
        if procbox.get().unwrap().lock().unwrap().get_filtering() {
            if vec!["enter", "mouse_click", "mouse_unselect"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
                .contains(&key)
            {
                procbox.get().unwrap().lock().unwrap().set_filtering(false);
                collector.get().unwrap().lock().unwrap().collect(
                    vec![Collectors::ProcCollector],
                    CONFIG,
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
                proccollector.get().unwrap().lock().unwrap().search_filter = String::default();
                procbox.get().unwrap().lock().unwrap().set_filtering(false);
            } else if key.len() == 1 {
                proccollector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .search_filter
                    .push_str(key.as_str());
            } else if key == "backspace".to_owned()
                && proccollector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .search_filter
                    .len()
                    > 0
            {
                proccollector.get().unwrap().lock().unwrap().search_filter =
                    proccollector.get().unwrap().lock().unwrap().search_filter[..proccollector
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .search_filter
                        .len()
                        - 2]
                        .to_owned();
            } else {
                continue;
            }
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                false,
                true,
                true,
                false,
            );
            if filtered {
                collector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_collect_done(EventEnum::Wait);
                collector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_collect_done_reference()
                    .wait(0.1);
                collector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_collect_done(EventEnum::Flag(false));
            }
            filtered = true;
            continue;
        }

        if key == "_null".to_owned() {
            continue;
        } else if key == "q".to_owned() {
            clean_quit(None, None, key_class, collector, draw, term, CONFIG, None);
        } else if key == "+" && CONFIG.get().unwrap().lock().unwrap().update_ms + 100 <= 86399900 {
            CONFIG.get().unwrap().lock().unwrap().update_ms += 100;
            brshtop_box
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .draw_update_ms(false, CONFIG, cpu_box, key_class, draw, menu, THEME, &term);
        } else if key == "-".to_owned()
            && CONFIG.get().unwrap().lock().unwrap().update_ms - 100 >= 100
        {
            CONFIG.get().unwrap().lock().unwrap().update_ms -= 100;
            brshtop_box
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .draw_update_ms(false, CONFIG, cpu_box, key_class, draw, menu, THEME, &term);
        } else if vec!["b", "n"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            netcollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .switch(key, collector, CONFIG);
        } else if vec!["M", "escape"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.get().unwrap().lock().unwrap().main(
                &THEME,
                &draw,
                term,
                &update_checker,
                &THEME,
                &key_class,
                &timer,
                &collector,
                collectors.clone(),
                &CONFIG,
                ARG_MODE,
                &netcollector,
                &brshtop_box,
                &init,
                &cpu_box,
                &cpucollector,
                boxes.clone(),
                &netbox,
                &proccollector,
                mem_box,
                &proc_box,
                &menu,
            );
        } else if vec!["o", "f2"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.get().unwrap().lock().unwrap().options(
                ARG_MODE,
                THEME,
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
                proc_box,
                mem_box,
                &menu,
            );
        } else if vec!["h", "f1"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key)
        {
            menu.get().unwrap().lock().unwrap().help(
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
            netcollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .set_reset(!netcollector.get().unwrap().lock().unwrap().get_reset());
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::NetCollector],
                CONFIG,
                true,
                false,
                false,
                true,
                false,
            );
        } else if key == "y".to_owned() {
            CONFIG.get().unwrap().lock().unwrap().net_sync =
                !CONFIG.get().unwrap().lock().unwrap().net_sync;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::NetCollector],
                CONFIG,
                true,
                false,
                false,
                true,
                false,
            );
        } else if key == "a".to_owned() {
            netcollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .set_auto_min(!netcollector.get().unwrap().lock().unwrap().get_auto_min());
            netcollector.get().unwrap().lock().unwrap().set_net_min(
                vec![("download", -1), ("upload", -1)]
                    .iter()
                    .map(|(s, i)| (s.to_owned().to_owned(), i.to_owned()))
                    .collect::<HashMap<String, i32>>(),
            );
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::NetCollector],
                CONFIG,
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
            //proccollector.get().unwrap().lock().unwrap().sorting(key);
        } else if key == " ".to_owned()
            && CONFIG.get().unwrap().lock().unwrap().proc_tree
            && procbox.get().unwrap().lock().unwrap().get_selected() > 0
        {
            if proccollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .collapsed
                .contains_key(&procbox.get().unwrap().lock().unwrap().get_selected_pid())
            {
                proccollector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .collapsed
                    .insert(
                        procbox
                            .get()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .get_selected_pid()
                            .clone(),
                        !proccollector
                            .get()
                            .unwrap()
                            .lock()
                            .unwrap()
                            .collapsed
                            .get(
                                &procbox
                                    .get()
                                    .unwrap()
                                    .lock()
                                    .unwrap()
                                    .get_selected_pid()
                                    .clone(),
                            )
                            .unwrap()
                            .to_owned(),
                    );
            }
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "e".to_owned() {
            CONFIG.get().unwrap().lock().unwrap().proc_tree =
                !CONFIG.get().unwrap().lock().unwrap().proc_tree;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "r".to_owned() {
            CONFIG.get().unwrap().lock().unwrap().proc_reversed =
                !CONFIG.get().unwrap().lock().unwrap().proc_reversed;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "c".to_owned() {
            CONFIG.get().unwrap().lock().unwrap().proc_per_core =
                !CONFIG.get().unwrap().lock().unwrap().proc_per_core;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "g".to_owned() {
            CONFIG.get().unwrap().lock().unwrap().mem_graphs =
                !CONFIG.get().unwrap().lock().unwrap().mem_graphs;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::MemCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "s".to_owned() {
            collector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .set_collect_idle(EventEnum::Wait);
            collector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .get_collect_idle_reference()
                .wait(-1.0);
            CONFIG.get().unwrap().lock().unwrap().swap_disk =
                !CONFIG.get().unwrap().lock().unwrap().swap_disk;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::MemCollector],
                CONFIG,
                true,
                true,
                false,
                true,
                false,
            );
        } else if key == "f".to_owned() {
            procbox.get().unwrap().lock().unwrap().set_filtering(true);
            if proccollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .search_filter
                .len()
                == 0
            {
                procbox.get().unwrap().lock().unwrap().set_start(0);
            }
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
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
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .view_modes
                .iter()
                .position(|v| *v == CONFIG.get().unwrap().lock().unwrap().view_mode)
                .unwrap()
                + 1
                > CONFIG.get().unwrap().lock().unwrap().view_modes.len() - 1
            {
                CONFIG.get().unwrap().lock().unwrap().view_mode =
                    CONFIG.get().unwrap().lock().unwrap().view_modes[0];
            } else {
                CONFIG.get().unwrap().lock().unwrap().view_mode =
                    CONFIG.get().unwrap().lock().unwrap().view_modes[CONFIG
                        .get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .view_modes
                        .iter()
                        .position(|v| *v == CONFIG.get().unwrap().lock().unwrap().view_mode)
                        .unwrap()
                        + 1];
            }
            brshtop_box.get().unwrap().lock().unwrap().set_proc_mode(
                CONFIG.get().unwrap().lock().unwrap().view_mode.t == ViewModeEnum::Proc,
            );
            brshtop_box.get().unwrap().lock().unwrap().set_stat_mode(
                CONFIG.get().unwrap().lock().unwrap().view_mode.t == ViewModeEnum::Stat,
            );
            draw.get().unwrap().lock().unwrap().clear(vec![], true);
            term.get().unwrap().lock().unwrap().refresh(
                vec![],
                vec![],
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
                proc_box,
            );
        } else if vec!["t", "k", "i"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
            .contains(&key.to_ascii_lowercase())
        {
            let pid: u32 = if procbox.get().unwrap().lock().unwrap().get_selected() > 0 {
                procbox.get().unwrap().lock().unwrap().get_selected_pid()
            } else {
                proccollector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .detailed_pid
                    .unwrap()
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
        } else if key == "delete".to_owned()
            && proccollector
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .search_filter
                .len()
                > 0
        {
            proccollector.get().unwrap().lock().unwrap().search_filter = String::default();
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
                true,
                false,
                true,
                true,
                false,
            );
        } else if key == "enter".to_owned() {
            if procbox.get().unwrap().lock().unwrap().get_selected() > 0
                && proccollector
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .detailed_pid
                    .unwrap_or(0)
                    != procbox.get().unwrap().lock().unwrap().get_selected_pid()
                && psutil::process::pid_exists(
                    procbox.get().unwrap().lock().unwrap().get_selected_pid(),
                )
            {
                proccollector.get().unwrap().lock().unwrap().detailed = true;
                procbox
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_last_selection(procbox.get().unwrap().lock().unwrap().get_selected());
                procbox.get().unwrap().lock().unwrap().set_selected(0);
                proccollector.get().unwrap().lock().unwrap().detailed_pid =
                    Some(procbox.get().unwrap().lock().unwrap().get_selected_pid());
                procbox
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_parent_resized(true);
            } else if proccollector.get().unwrap().lock().unwrap().detailed {
                procbox
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_selected(procbox.get().unwrap().lock().unwrap().get_last_selection());
                procbox.get().unwrap().lock().unwrap().set_last_selection(0);
                proccollector.get().unwrap().lock().unwrap().detailed = false;
                proccollector.get().unwrap().lock().unwrap().detailed_pid = None;
                procbox
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .set_parent_resized(true);
            } else {
                continue;
            }
            proccollector.get().unwrap().lock().unwrap().details =
                HashMap::<String, ProcCollectorDetails>::new();
            proccollector.get().unwrap().lock().unwrap().details_cpu = vec![];
            proccollector.get().unwrap().lock().unwrap().details_mem = vec![];
            graphs
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .detailed_cpu
                .NotImplemented = true;
            graphs
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .detailed_mem
                .NotImplemented = true;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
                CONFIG,
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
            procbox.get().unwrap().lock().unwrap().selector(
                key,
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
    key: &OnceCell<Mutex<Key>>,
    collector: &OnceCell<Mutex<Collector>>,
    draw: &OnceCell<Mutex<Draw>>,
    term: &OnceCell<Mutex<Term>>,
) -> Option<()> {
    key.get().unwrap().lock().unwrap().stop();
    collector.get().unwrap().lock().unwrap().stop();
    draw.get().unwrap().lock().unwrap().now(
        vec![
            term.get().unwrap().lock().unwrap().get_clear(),
            term.get().unwrap().lock().unwrap().get_normal_screen(),
            term.get().unwrap().lock().unwrap().get_show_cursor(),
            term.get().unwrap().lock().unwrap().get_mouse_off(),
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
    draw: &OnceCell<Mutex<Draw>>,
    term: &OnceCell<Mutex<Term>>,
    key: &OnceCell<Mutex<Key>>,
    brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
    collector: &OnceCell<Mutex<Collector>>,
    boxes: Vec<Boxes>,
    init: &OnceCell<Mutex<Init>>,
    cpu_box: &OnceCell<Mutex<CpuBox>>,
    menu: &OnceCell<Mutex<Menu>>,
    timer: &OnceCell<Mutex<Timer>>,
    CONFIG: &OnceCell<Mutex<Config>>,
    THEME: &OnceCell<Mutex<Theme>>,
    DEBUG: bool,
    collectors: Vec<Collectors>,
    timeit: &OnceCell<Mutex<TimeIt>>,
    ARG_MODE: ViewMode,
    graphs: &OnceCell<Mutex<Graphs>>,
    meters: &OnceCell<Mutex<Meters>>,
    netbox: &OnceCell<Mutex<NetBox>>,
    procbox: &OnceCell<Mutex<ProcBox>>,
    membox: &OnceCell<Mutex<MemBox>>,
    cpu_collector: &OnceCell<Mutex<CpuCollector>>,
    mem_collector: &OnceCell<Mutex<MemCollector>>,
    net_collector: &OnceCell<Mutex<NetCollector>>,
    proc_collector: &OnceCell<Mutex<ProcCollector>>,
    mem_box: &OnceCell<Mutex<MemBox>>,
) {
    draw.get().unwrap().lock().unwrap().now(
        vec![
            term.get().unwrap().lock().unwrap().get_alt_screen(),
            term.get().unwrap().lock().unwrap().get_clear(),
            term.get().unwrap().lock().unwrap().get_hide_cursor(),
            term.get().unwrap().lock().unwrap().get_mouse_on(),
            Term::title("BRShtop".to_owned()),
        ],
        key,
    );
    Term::echo(false);
    key.get().unwrap().lock().unwrap().start(draw, menu);
    term.get().unwrap().lock().unwrap().refresh(
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
        mem_box,
        netbox,
        procbox,
    );
    brshtop_box.get().unwrap().lock().unwrap().calc_sizes(
        boxes.clone(),
        term,
        CONFIG,
        cpu_collector,
        cpu_box,
        membox,
        netbox,
        procbox,
    );
    brshtop_box.get().unwrap().lock().unwrap().draw_bg(
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
    collector.get().unwrap().lock().unwrap().start(
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
        collector,
    )
}

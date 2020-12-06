use std::collections::*;
use std::time::{Duration, SystemTime};
use std::*;
use clap::{Arg, App};
use psutil::*;
use string_template::*;


fn main() {
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
    let matches = App::new("brstop")
    .version(clap::crate_version!())
    .author(("Aristocratos (jakob@qvantnet.com)\n".to_owned() +
        "Samuel Rembisz <sjrembisz07@gmail.com)").as_str())
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

    // Variables

    let BANNER_SRC = vec![
	("#ffa50a", "#0fd7ff", "██████╗ ██████╗ ██╗   ██╗████████╗ ██████╗ ██████╗"),
	("#f09800", "#00bfe6", "██╔══██╗██╔══██╗╚██╗ ██╔╝╚══██╔══╝██╔═══██╗██╔══██╗"),
	("#db8b00", "#00a6c7", "██████╔╝██████╔╝ ╚████╔╝    ██║   ██║   ██║██████╔╝"),
	("#c27b00", "#008ca8", "██╔══██╗██╔═══╝   ╚██╔╝     ██║   ██║   ██║██╔═══╝ "),
	("#a86b00", "#006e85", "██████╔╝██║        ██║      ██║   ╚██████╔╝██║"),
	("#000000", "#000000", "╚═════╝ ╚═╝        ╚═╝      ╚═╝    ╚═════╝ ╚═╝"),
    ];

    let DEFAULT_CONF = string_template::Template::new("#? Config file for bpytop v. {{version}}

    #* Color theme, looks for a .theme file in \"/usr/[local/]share/bpytop/themes\\" and \"~/.config/bpytop/themes\", \"Default\" for builtin default theme.
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
    ")

}

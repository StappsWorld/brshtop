use std::collections::*;
use std::time::{Duration, SystemTime};
use std::*;
use clap::{Arg, App};
use psutil::*;


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
        process::exit(1);
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

}

use std::path::*;
use log::LevelFilter;


pub fn errlog(config_dir : &Path, message : String) {
    let error_file = "log.log";
    let error_dir = config_dir.join(PathBuf::from(error_file));
    let dir = error_dir.to_str().unwrap();

    match simple_logging::log_to_file(dir, LevelFilter::Debug) {
        Err(e) => throw_error(format!("ERROR!\nNo permission to write to \"{}\" directory with error {}!", config_dir.to_str().unwrap(), e).as_str()),
        _ => (),
    };

    return;
}

pub fn throw_error(message : &str){
    print!("{}", message);
    std::process::exit(1);
}

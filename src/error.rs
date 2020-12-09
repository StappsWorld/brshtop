use log::{debug, LevelFilter};
use std::path::*;

pub fn errlog<P: AsRef<Path>>(config_dir: P, message: String) {
    let error_file = "log.log";
    let error_dir = config_dir.as_ref().join(PathBuf::from(error_file));
    let dir = error_dir.to_str().unwrap();

    match simple_logging::log_to_file(dir, LevelFilter::Debug) {
        Err(e) => throw_error(
            format!(
                "ERROR!\nNo permission to write to \"{}\" directory with error {}!",
                config_dir.as_ref().to_str().unwrap(),
                e
            )
            .as_str(),
        ),
        _ => (),
    };
    debug!("{}", message);

    return;
}

pub fn throw_error(message: &str) {
    print!("{}", message);
    std::process::exit(1);
}

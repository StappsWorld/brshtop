use log::{debug, LevelFilter};
use std::path::*;
use crate::{clean_quit, CONFIG_DIR};

pub fn errlog(message: String) {
    // TODO: I know there's a better way to do this
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
    debug!("{}", message);

    return;
}

pub fn throw_error(message: &str) {
    print!("{}", message);
    std::process::exit(1);
}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Theme(String),
}
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

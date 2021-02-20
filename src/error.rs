use log::{debug, LevelFilter};
use std::path::*;
use crate::{CONFIG_DIR, clean_quit, emergency_quit};

pub fn errlog(message: String) {
    // TODO: I know there's a better way to do this
    debug!("{}", message);

    return;
}

pub fn throw_error(message: &str) {
    print!("{}", message);
    emergency_quit(None, Some(message.to_owned()));
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

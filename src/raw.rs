use crate::error::*;
use std::fs::File;
use nix::fcntl;
use termios::*;
use std::os::unix::io::{RawFd, AsRawFd};


pub struct Raw {
    pub stream : File,
    pub fd : RawFd,
    pub original_stty : Termios,
}
impl Raw {
    pub fn new(s : File) -> Self {
        let usable_fd = s.as_raw_fd();
        let tty = match Termios::from_fd(usable_fd) {
            Ok(t) => t,
            Err(e) => {
                throw_error(format!("Unable to create TermIOS from given in Raw (error {})", e).as_str());
                Termios::from_fd(usable_fd).unwrap() //This is never reached, but compiler doesn't trust me :)
            },
        };
        Raw {
            stream : s,
            fd : s.as_raw_fd().clone(),
            original_stty : tty,
        }
    }

    pub fn enter(&mut self) {
        termios::tcgetattr(self.fd, &mut self.original_stty);

    }

    pub fn exit(&mut self) {
        
    }
}
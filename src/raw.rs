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
    pub fn new(s : File) {
        let usable_fd = s.as_raw_fd();
        Raw {
            stream : s,
            fd : s.as_raw_fd(),
            original_stty : Termios::from_fd(usable_fd),
        }
    }

    pub fn enter(&mut self) {
        termios::tcgetattr(self.fd, self.original_stty);

    }

    pub fn exit(&mut self) {
        
    }
}
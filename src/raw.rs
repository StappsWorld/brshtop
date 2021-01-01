use crate::error::*;
use std::io::Stdin;
use nix::fcntl;
use termios::*;
use std::os::unix::io::{RawFd, AsRawFd};
use pancurses;


pub struct Raw<'a> {
    pub stream : &'a mut Stdin,
    pub fd : RawFd,
    pub original_stty : Termios,
}
impl<'a> Raw<'a> {
    pub fn new(s : &'a mut Stdin) -> Self {
        let usable_fd = s.as_raw_fd().clone();
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
        pancurses::cbreak();
    }

    pub fn exit(&mut self) {
        termios::tcsetattr(self.fd, termios::os::linux::TCSANOW, &self.original_stty);
    }
}
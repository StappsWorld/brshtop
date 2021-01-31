use crate::error::*;
use std::io::{stdin, Stdin};
use nix::fcntl;
use termios::*;
use pancurses;


pub struct Raw {
    pub stream : Stdin,
    pub fd : i32,
    pub original_stty : Termios,
}
impl Raw {
    pub fn new() -> Self {
        let usable_fd = libc::STDIN_FILENO.clone();
        let tty = match Termios::from_fd(usable_fd) {
            Ok(t) => t,
            Err(e) => {
                throw_error(format!("Unable to create TermIOS from given in Raw (error {})", e).as_str());
                Termios::from_fd(usable_fd).unwrap() //This is never reached, but compiler doesn't trust me :)
            },
        };
        Raw {
            stream : stdin(),
            fd : libc::STDIN_FILENO.clone(),
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
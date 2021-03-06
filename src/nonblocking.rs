use crate::error;
use nix::{fcntl, libc::O_NONBLOCK};
use std::io::Stdin;
use std::path::Path;

// TODO : Figure out STDIN
pub struct Nonblocking /*<'a>*/ {
    //pub stream : &'a mut Stdin,
    pub fd: i32,
    pub orig_fl: Option<i32>,
}
impl<'a> Nonblocking /*<'a>*/ {
    pub fn new(/*s : &'a mut Stdin*/) -> Self {
        Nonblocking {
            //stream : s,
            fd: libc::STDIN_FILENO.clone(),
            orig_fl: None,
        }
    }

    pub fn enter(&mut self) {
        self.orig_fl = match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_GETFL) {
            Ok(o) => Some(o),
            Err(e) => {
                error::errlog(format!("Error getting fcntl data... (error {})", e));
                return;
            }
        };

        match fcntl::fcntl(
            self.fd,
            fcntl::FcntlArg::F_SETFL(
                fcntl::OFlag::from_bits(self.orig_fl.unwrap() | O_NONBLOCK).unwrap(),
            ),
        ) {
            Ok(_) => (),
            Err(e) => {
                error::errlog(format!("Error setting fcntl data... (error {})", e));
                return;
            }
        }
    }

    pub fn exit(&mut self) {
        match fcntl::fcntl(
            self.fd,
            fcntl::FcntlArg::F_SETFL(fcntl::OFlag::from_bits(self.orig_fl.unwrap()).unwrap()),
        ) {
            Ok(_) => (),
            Err(e) => {
                error::errlog(format!("Error setting fcntl data... (error {})", e));
                return;
            }
        }
    }
}

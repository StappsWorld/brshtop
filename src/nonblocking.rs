use crate::error;
use std::io::Stdin;
use std::path::Path;
use std::os::unix::io::{RawFd, AsRawFd};
use nix::{fcntl, libc::O_NONBLOCK};

pub struct Nonblocking<'a> {
    pub stream : &'a mut Stdin,
    pub fd : RawFd,
    pub orig_fl : Option<RawFd>,
}
impl<'a> Nonblocking<'a> {

    pub fn new(s : &'a mut Stdin) -> Self {
        Nonblocking {
            stream : s,
            fd : s.as_raw_fd().clone(),
            orig_fl : None,
        }
    }


    pub fn enter(&mut self){
        self.orig_fl = match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_GETFL){
            Ok(o) => Some(o),
            Err(e) => {
                error::errlog(
                    format!(
                        "Error getting fcntl data... (error {})",
                        e
                    ),
                );
                return;
            }
        };

        match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_SETFL(fcntl::OFlag{bits : self.orig_fl.unwrap() | O_NONBLOCK as i32})) {
            Ok(_) => (),
            Err(e) => {
                error::errlog(
                    format!(
                        "Error setting fcntl data... (error {})",
                        e
                    ),
                );
                return;
            }
        }
    }

    pub fn exit(&mut self) {
        match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_SETFL(fcntl::OFlag{bits : self.orig_fl.unwrap()})) {
            Ok(_) => (),
            Err(e) => {
                error::errlog(
                    format!(
                        "Error setting fcntl data... (error {})",
                        e
                    ),
                );
                return;
            }
        }
    }
    
}
use std::fs::File;
use std::os::unix::io::RawFd;
use nix::{fcntl, libc::O_NONBLOCK};

pub struct Nonblocking {
    pub stream : File,
    pub fd : RawFd,
    pub orig_fl : Option<RawFd>,
}
impl Nonblocking {

    pub fn new(s : File) {
        Nonblocking {
            stream : s,
            fd : s.as_raw_fd(),
            orig_fl : None,
        }
    }


    pub fn enter(&mut self){
        self.orig_fl = match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_GETFL){
            Ok(o) => Some(o),
            Err(e) => {
                error::errlog(
                    CONFIG_DIR,
                    format!(
                        "Error getting fcntl data... (error {})",
                        e
                    ),
                );
                return;
            }
        };

        match fcntl::fcntl(self.fd, fcntl::FcntlArg::F_SETFL(fcntl::OFlag{bits : self.orig_fl.unwrap() | O_NONBLOCK})) {
            Ok(_) => (),
            Err(e) => {
                error::errlog(
                    CONFIG_DIR,
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
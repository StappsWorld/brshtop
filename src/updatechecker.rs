use {
    crate::{error::errlog, VERSION},
    crossbeam::scope,
    error_chain::error_chain,
    reqwest,
    std::{process::Command, str, thread},
    which::which,
};

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

pub struct UpdateChecker {
    pub version: String,
}
impl<'a> UpdateChecker {
    pub fn new() -> Self {
        UpdateChecker {
            version: VERSION.clone(),
        }
    }

    pub fn run(&mut self) {
        scope(|s| {
            s.spawn(|_| self.checker());
        });
    }

    // TODO : Implement for Brshtop github
    pub fn checker(&mut self) {
        let source: String = match reqwest::blocking::get(
            "https://github.com/aristocratos/bpytop/raw/master/bpytop.py",
        ) {
            Ok(s) => match s.text() {
                Ok(text) => text,
                Err(e) => {
                    errlog(format!("Unable to get version info (error {:?})", e));
                    return;
                }
            },
            Err(e) => {
                errlog(format!("Unable to get version info (error {:?})", e));
                return;
            }
        };

        for line in source.lines() {
            let mut mut_line = line.clone();
            mut_line = match str::from_utf8(line.as_bytes()) {
                Ok(s) => s,
                Err(e) => {
                    errlog(format!(
                        "Unable to convert current line to utf-8 (error {:?})",
                        e
                    ));
                    continue;
                }
            };
            if mut_line.starts_with("VERSION: str = ") {
                self.version = mut_line[(mut_line.find('=').unwrap()) + 1..]
                    .strip_prefix("\" \n")
                    .unwrap_or(&mut_line[(mut_line.find('=').unwrap()) + 1..])
                    .strip_suffix("\" \n")
                    .unwrap_or(&mut_line[(mut_line.find('=').unwrap()) + 1..])
                    .to_owned();
                break;
            }
        }

        if self.version != VERSION.to_owned()
            && match which::which("notify_send") {
                Ok(p) => p.exists(),
                Err(e) => false,
            }
        {

            match Command::new("notify_send").args(&vec!["-u", "normal", "BpyTop Update!", format!("New version of BpyTop available!\nCurrent version: {}\nNew version: {}\nDownload at github.com/aristocratos/bpytop", VERSION.to_owned(), self.version).as_str(), "-i", "update-notifier", "-t", "10000"]).output() {
                Ok(_) => (),
                Err(e) => errlog(format!("Unable to execute notify_send (error {})", e)),
            };
        }
    }
}

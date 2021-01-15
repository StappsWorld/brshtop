use {
    crate::{error::errlog, VERSION},
    reqwest::blocking::get,
    std::{process::Command, str, thread},
    which::which,
};

pub struct UpdateChecker {
    pub version: String,
    pub thread: Option<thread::JoinHandle<()>>,
}
impl<'a> UpdateChecker {
    pub fn new() -> Self {
        UpdateChecker {
            version: VERSION.clone(),
            thread: None,
        }
    }

    pub fn run(&'static mut self) {
        self.thread = Some(thread::spawn(|| self.checker()));
    }

    // TODO : Implement for Brshtop github
    pub fn checker(&mut self) {
        let source: String =
            match get("https://github.com/aristocratos/bpytop/raw/master/bpytop.py") {
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
            line = match str::from_utf8(line.as_bytes()) {
                Ok(s) => s,
                Err(e) => {
                    errlog(format!(
                        "Unable to convert current line to utf-8 (error {:?})",
                        e
                    ));
                    continue;
                }
            };
            if line.starts_with("VERSION: str = ") {
                self.version = line[(line.find('=').unwrap()) + 1..]
                    .strip_prefix("\" \n")
                    .unwrap_or(&line[(line.find('=').unwrap()) + 1..])
                    .strip_suffix("\" \n")
                    .unwrap_or(&line[(line.find('=').unwrap()) + 1..])
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
            let command = Command::new("notify_send").args(&["-u", "normal", "BpyTop Update!", format!("New version of BpyTop available!\nCurrent version: {}\nNew version: {}\nDownload at github.com/aristocratos/bpytop", VERSION.to_owned(), self.version).as_str(), "-i", "update-notifier", "-t", "10000"]);

            match command.output() {
                Ok(_) => (),
                Err(e) => errlog(format!("Unable to execute notify_send (error {})", e)),
            };
        }
    }
}

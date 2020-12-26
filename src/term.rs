use crate::error::*;
use crate::event::Event;
use crate::theme::Color;
use crate::mv::*;
use crate::fx::*;
use crate::collector::*;
use crate::*;
use std::sync::mpsc::*;
use std::{thread, io};
use terminal_size::{terminal_size, Height, Width};
use termios::*;
use std::os::unix::io::{RawFd, AsRawFd};


pub struct Term {
    pub width: u16,
    pub height: u16,
    pub resized: bool,
    pub _w: u16,
    pub _h: u16,
    pub fg: Color,
    pub bg: Color,
    pub hide_cursor: String,
    pub show_cursor: String,
    pub alt_screen: String,
    pub normal_screen: String,
    pub clear: String,
    pub mouse_on: String,
    pub mouse_off: String,
    pub mouse_direct_on: String,
    pub mouse_direct_off: String,
    pub winch: Event,
}
impl Term {

    pub fn new() {
        let (winch_s_mut, winch_r_mut) = channel::<Event>();
        Term {
            width: 0,
            height: 0,
            resized: false,
            _w : 0,
            _h : 0,
            fg : Color::Default(),  // Default foreground color,
            bg : Color::Default(),  // Default background color,
            hide_cursor : "\033[?25l",  // Hide terminal cursor,
            show_cursor : "\033[?25h",  // Show terminal cursor,
            alt_screen : "\033[?1049h", // Switch to alternate screen,
            normal_screen : "\033[?1049l",  // Switch to normal screen,
            clear : "\033[2J\033[0;0f",  // Clear screen and set cursor to position 0,0,
            // Enable reporting of mouse position on click and release,
            mouse_on : "\033[?1002h\033[?1015h\033[?1006h",
            mouse_off : "\033[?1002l",  // Disable mouse reporting,
            // Enable reporting of mouse position at any movement,
            mouse_direct_on : "\033[?1003h",
            mouse_direct_off : "\033[?1003l",  // Disable direct mouse reporting,
            winch : Event::Flag(false),
        }
    }

    ///Updates width and height and sets resized flag if terminal has been resized
    pub fn refresh(&mut self, args: Vec<String>, collector : Collector, init : Init, cpu_box : CpuBox, draw : Draw, force: bool, mv : Mv, fx : Fx, key : Key, menu : Menu, box_class : Box, timer : Timer) {
        if self.resized {
            self.winch = Event::Flag(true);
            return;
        }

        let term_size = terminal_size();
        match term_size {
            Some((Width(w), Height(h))) => {
                self._w = w;
                self._h = h;
            }
            None => throw_error("Unable to get size of terminal!"),
        };

        if (self._w == self.width && self._h == self.height) && !force {
            return;
        }
        if force {
            collector.collect_interrupt = true;
        }

        while (self._w != self.width && self._h != self.height) || (self._w < 80 || self._h < 24) {
            if init.running {
                init.resized = true;
            }

            cpu_box.clock_block = true;
            self.resized = true;
            collector.collect_interrupt = true;
            self.width = self._w;
            self.height = self._h;
            draw.now(term.clear());
            draw.now(
                create_box((self._w / 2) as i32 - 25, (self._h / 2) as i32 - 2, 50, 3, String::from("resizing"), "".to_owned(), Color::Green(), Color::White(), true, Box::None),
                format!("{}{}{}{}Width : {}   Height: {}{}{}{}",
                    mv::right(120),
                    Color::Default(),
                    Color::BlackBg(),
                    fx::bold,
                    self._w,
                    self._h,
                    fx::ub,
                    self.bg,
                    self.fg));

            while self._w < 80 || self._h < 24 {
                draw.now(self.clear);
                draw.now(
                    create_box((self._w / 2) as i32 - 25, (self._h / 2) as i32 - 2, 50, 5, String::from("warning"), "".to_owned(), Color::Red(), Color::White(), true, Box::None),
                    format!("{}{}{}{}Width: {}{}   ", Mv::right(12), Color::default(), Color::BlackBg(), Fx::b, if self._w < 80 {Color::Red()} else {Color::Green()}, self._w),
                    format!("{}Height: {}{}{}{}", Color::Default(), if self._h < 24 {Color::Red()} else {Color::Green()}, self._h, self::bg, self::fg),
                    format!("{}{}{}Width and Height needs to be at least 80 x 24 !{}{}{}", Mv::to((self._h / 2) as i32, (self._w / 2) as i32 - 23), Color::Default(), Color::BlackBg(), Fx::ub, self.bg, self.fg)
                );
                self.winch = Event::Wait;
                self.winch.wait(0.3);
                self.winch = Event::Flag(false);

                let term_size_check = terminal_size();
                match term_size_check {
                    Some((Width(w), Height(h))) => {
                        self._w = w;
                        self._h = h;
                    }
                    None => throw_error("Unable to get size of terminal!"),
                };
            }
            
            self.winch = Event::Wait;
            self.winch.wait(0.3);
            self.winch = Event::Flag(false);

            let term_size_check = terminal_size();
            match term_size_check {
                Some((Width(w), Height(h))) => {
                    self._w = w;
                    self._h = h;
                }
                None => throw_error("Unable to get size of terminal!"),
            };
        }

        key.mouse = HashMap::new();
        box_class.calc_size();
        if init.running {
            self.resized = false;
            return;
        }

        if menu.active {
            menu.resize = true;
        }

        box_class.draw_bg(false);
        self.resized = false;
        timer.finish();

        return;
    }

    /// Toggle input echo
    pub fn echo<P: AsRef<Path>>(on : bool, CONFIG_DIR : P) {
        let fd = io::stdin().as_raw_fd();

        let mut termios = match Termios::from_fd(fd) {
            Ok(t) => t,
            Err(e) => {
                error::errlog(
                    CONFIG_DIR,
                    format!(
                        "Error getting Termios data... (error {})",
                        e
                    ),
                );
            }
        };

        if on {
            termios.c_lflag |= termios::os::linux::ECHO;
        } else {
            termios.c_lflag &= !termios::os::linux::ECHO;
        }

        match tcsetattr(fd, os::target::TCSANOW, &termios) {
            Ok(s) => (),
            Err(e) => error::errlog(
                CONFIG_DIR,
                format!(
                    "Error setting Termios data... (error {})",
                    e
                ),
            )
        }
    }

    pub fn title(text : String) -> String {
        let out : String = match os::env::var("TERMINAL_TITLE") {
            Ok(o) => o,
            Err(e) => {
                error::errlog(
                    CONFIG_DIR,
                    format!(
                        "Error setting Termios data... (error {})",
                        e
                    )
                );
                return;
            }
        };

        if text == String::from("") {
            out.push_str(" ");
        } else {
            out.push_str(text);
        }
        
        format!("\033]0;{}\a", out)
    }
    
}

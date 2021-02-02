use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::*,
        config::Config,
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        create_box,
        draw::Draw,
        error,
        event::{Event, EventEnum},
        fx,
        init::Init,
        key::Key,
        membox::MemBox,
        menu::Menu,
        mv,
        netbox::NetBox,
        procbox::ProcBox,
        theme::{Color, Theme},
        timer::Timer,
    },
    once_cell::sync::OnceCell,
    std::{
        collections::HashMap,
        io,
        mem::drop,
        ops::{Deref, DerefMut},
        os::unix::io::AsRawFd,
        sync::Mutex,
    },
    terminal_size::{terminal_size, Height, Width},
    termios::*,
};

#[derive(Clone)]
pub struct Term {
    width: u16,
    height: u16,
    resized: bool,
    _w: u16,
    _h: u16,
    fg: Color,
    bg: Color,
    hide_cursor: String,
    show_cursor: String,
    alt_screen: String,
    normal_screen: String,
    clear: String,
    mouse_on: String,
    mouse_off: String,
    mouse_direct_on: String,
    mouse_direct_off: String,
    winch: Event,
}
impl Term {
    pub fn new() -> Self {
        Term {
            width: 1,
            height: 1,
            resized: false,
            _w: 1,
            _h: 1,
            fg: Color::Default(),                    // Default foreground color,
            bg: Color::Default(),                    // Default background color,
            hide_cursor: String::from("\x1b[?25l"),  // Hide terminal cursor,
            show_cursor: String::from("\x1b[?25h"),  // Show terminal cursor,
            alt_screen: String::from("\x1b[?1049h"), // Switch to alternate screen,
            normal_screen: String::from("\x1b[?1049l"), // Switch to normal screen,
            clear: String::from("\x1b[2J\x1b[0;0f"), // Clear screen and set cursor to position 0,0,
            // Enable reporting of mouse position on click and release,
            mouse_on: String::from("\x1b[?1002h\x1b[?1015h\x1b[?1006h"),
            mouse_off: String::from("\x1b[?1002l"), // Disable mouse reporting,
            // Enable reporting of mouse position at any movement,
            mouse_direct_on: String::from("\x1b[?1003h"),
            mouse_direct_off: String::from("\x1b[?1003l"), // Disable direct mouse reporting,
            winch: Event {
                t: EventEnum::Flag(false),
            },
        }
    }

    ///Updates width and height and sets resized flag if terminal has been resized, defaults : force = false
    pub fn refresh(
        &mut self,
        args: Vec<String>,
        boxes: Vec<Boxes>,
        collector_p: &OnceCell<Mutex<Collector>>,
        init_p: &OnceCell<Mutex<Init>>,
        cpu_box_p: &OnceCell<Mutex<CpuBox>>,
        draw_p: &OnceCell<Mutex<Draw>>,
        force: bool,
        key_p: &OnceCell<Mutex<Key>>,
        menu_p: &OnceCell<Mutex<Menu>>,
        brshtop_box_p: &OnceCell<Mutex<BrshtopBox>>,
        timer_p: &OnceCell<Mutex<Timer>>,
        config_p: &OnceCell<Mutex<Config>>,
        theme_p: &OnceCell<Mutex<Theme>>,
        cpu_p: &OnceCell<Mutex<CpuCollector>>,
        mem_box_p: &OnceCell<Mutex<MemBox>>,
        net_box_p: &OnceCell<Mutex<NetBox>>,
        proc_box_p: &OnceCell<Mutex<ProcBox>>,
    ) {
        let mut collector = collector_p.get().unwrap().lock().unwrap();
        let mut init = init_p.get().unwrap().lock().unwrap();
        let mut cpu_box = cpu_box_p.get().unwrap().lock().unwrap();
        let mut draw = draw_p.get().unwrap().lock().unwrap();
        let mut key = key_p.get().unwrap().lock().unwrap();
        drop(key);
        let mut menu = menu_p.get().unwrap().lock().unwrap();
        let mut brshtop_box = brshtop_box_p.get().unwrap().lock().unwrap();
        let mut timer = timer_p.get().unwrap().lock().unwrap();

        if self.resized {
            self.winch.replace_self(EventEnum::Flag(true));
            return;
        }

        let term_size = terminal_size();
        match term_size {
            Some((Width(w), Height(h))) => {
                self._w = w;
                self._h = h;
            }
            None => error::throw_error("Unable to get size of terminal!"),
        };

        if (self._w == self.width && self._h == self.height) && !force {
            return;
        }
        if force {
            collector.set_collect_interrupt(true);
        }

        while (self._w != self.width && self._h != self.height) || (self._w < 80 || self._h < 24) {
            if init.running {
                init.resized = true;
            }

            cpu_box.set_clock_block(true);
            self.resized = true;
            collector.set_collect_interrupt(true);
            self.width = self._w;
            self.height = self._h;
            draw.now(vec![self.clear.clone()], key_p);
            let mut mutex_self: Mutex<Term> = Mutex::new(self.clone());
            let mut passable_self: OnceCell<Mutex<Term>> = OnceCell::new();
            passable_self.set(mutex_self);
            draw.now(
                vec![
                    create_box(
                        (self._w as u32 / 2) - 25,
                        (self._h as u32 / 2) - 2,
                        50,
                        3,
                        Some(String::from("resizing")),
                        None,
                        Some(Color::Green()),
                        Some(Color::White()),
                        true,
                        None,
                        &passable_self,
                        theme_p,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ),
                    format!(
                        "{}{}{}{}Width : {}   Height: {}{}{}{}",
                        mv::right(120),
                        Color::Default(),
                        Color::BlackBg(),
                        fx::bold,
                        self.get_w(),
                        self.get_h(),
                        fx::ub,
                        self.get_bg(),
                        self.get_fg()
                    ),
                ],
                key_p,
            );

            while self._w < 80 || self._h < 24 {
                draw.now(vec![self.clear.clone()], key_p);
                draw.now(
                    vec![
                        create_box(
                            (self._w as u32 / 2) - 25,
                            (self._h as u32 / 2) - 2,
                            50,
                            5,
                            Some(String::from("warning")),
                            None,
                            Some(Color::Red()),
                            Some(Color::White()),
                            true,
                            None,
                            &passable_self,
                            theme_p,
                            None,
                            None,
                            None,
                            None,
                            None,
                        ),
                        format!(
                            "{}{}{}{}Width: {}{}   ",
                            mv::right(12),
                            Color::default(),
                            Color::BlackBg(),
                            fx::b,
                            if self._w < 80 {
                                Color::Red()
                            } else {
                                Color::Green()
                            },
                            self.get_w()
                        ),
                        format!(
                            "{}Height: {}{}{}{}",
                            Color::Default(),
                            if self.get_h() < 24 {
                                Color::Red()
                            } else {
                                Color::Green()
                            },
                            self.get_h(),
                            self.get_bg(),
                            self.get_fg()
                        ),
                        format!(
                            "{}{}{}Width and Height needs to be at least 80 x 24 !{}{}{}",
                            mv::to((self._h / 2) as u32, (self._w / 2) as u32 - 23),
                            Color::Default(),
                            Color::BlackBg(),
                            fx::ub,
                            self.get_bg(),
                            self.get_fg()
                        ),
                    ],
                    key_p,
                );
                self.winch.replace_self(EventEnum::Wait);
                self.winch.wait(0.3);
                self.winch.replace_self(EventEnum::Flag(false));

                let term_size_check = terminal_size();
                match term_size_check {
                    Some((Width(w), Height(h))) => {
                        self._w = w;
                        self._h = h;
                    }
                    None => error::throw_error("Unable to get size of terminal!"),
                };
            }
            self.winch.replace_self(EventEnum::Wait);
            self.winch.wait(0.3);
            self.winch.replace_self(EventEnum::Flag(false));

            let term_size_check = terminal_size();
            match term_size_check {
                Some((Width(w), Height(h))) => {
                    self._w = w;
                    self._h = h;
                }
                None => error::throw_error("Unable to get size of terminal!"),
            };
        }

        key = key_p.get().unwrap().lock().unwrap();
        key.mouse = HashMap::<String, Vec<Vec<i32>>>::new();
        drop(cpu_box);
        let mut mutex_self: Mutex<Term> = Mutex::new(self.clone());
        let mut passable_self: OnceCell<Mutex<Term>> = OnceCell::new();
        passable_self.set(mutex_self);
        brshtop_box.calc_sizes(
            boxes.clone(),
            &passable_self,
            config_p,
            cpu_p,
            cpu_box_p,
            mem_box_p,
            net_box_p,
            proc_box_p,
        );
        if init.running {
            self.resized = false;
            return;
        }

        if menu.active {
            menu.resized = true;
        }

        brshtop_box.draw_bg(
            false,
            draw_p,
            boxes.clone(),
            menu_p,
            config_p,
            cpu_box_p,
            mem_box_p,
            net_box_p,
            proc_box_p,
            key_p,
            theme_p,
            &passable_self,
        );
        self.resized = false;
        timer.finish(key_p, config_p);
    }

    /// Toggle input echo
    pub fn echo(on: bool) {
        let fd = io::stdin().as_raw_fd().clone();

        let mut termios = match Termios::from_fd(fd) {
            Ok(t) => t,
            Err(e) => {
                error::errlog(format!("Error getting Termios data... (error {})", e));
                return;
            }
        };

        if on {
            termios.c_lflag |= termios::os::linux::ECHO;
        } else {
            termios.c_lflag &= !termios::os::linux::ECHO;
        }

        match tcsetattr(fd, os::target::TCSANOW, &termios) {
            Ok(_) => (),
            Err(e) => error::errlog(format!("Error setting Termios data... (error {})", e)),
        }
    }

    pub fn title(text: String) -> String {
        //println!("Making title");
        let mut out: String = match std::env::var("TERMINAL_TITLE") {
            Ok(o) => o,
            Err(e) => {
                error::errlog(format!("Error setting Termios data... (error {})", e));
                return String::default();
            }
        };

        if text == String::from("") {
            out.push_str(" ");
        } else {
            out.push_str(text.as_str());
        }
        format!("\x1b]0;{}{}", out, ascii_utils::table::BEL)
    }

    pub fn get_width(&self) -> u16 {
        self.width.clone()
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = width.clone()
    }

    pub fn get_height(&self) -> u16 {
        self.height.clone()
    }

    pub fn set_height(&mut self, height: u16) {
        self.height = height.clone()
    }

    pub fn get_resized(&self) -> bool {
        self.resized.clone()
    }

    pub fn set_resized(&mut self, resized: bool) {
        self.resized = resized.clone()
    }

    pub fn get_w(&self) -> u16 {
        self._w.clone()
    }

    pub fn set_w(&mut self, _w: u16) {
        self._w = _w.clone()
    }

    pub fn get_h(&self) -> u16 {
        self._h.clone()
    }

    pub fn set_h(&mut self, _h: u16) {
        self._h = _h.clone()
    }

    pub fn get_fg(&self) -> Color {
        self.fg.clone()
    }

    pub fn set_fg(&mut self, fg: Color) {
        self.fg = fg.clone()
    }

    pub fn get_bg(&self) -> Color {
        self.bg.clone()
    }

    pub fn set_bg(&mut self, bg: Color) {
        self.bg = bg.clone()
    }

    pub fn get_hide_cursor(&self) -> String {
        self.hide_cursor.clone()
    }

    pub fn set_hide_cursor(&mut self, hide_cursor: String) {
        self.hide_cursor = hide_cursor.clone()
    }

    pub fn get_show_cursor(&self) -> String {
        self.show_cursor.clone()
    }

    pub fn set_show_cursor(&mut self, show_cursor: String) {
        self.show_cursor = show_cursor.clone()
    }

    pub fn get_alt_screen(&self) -> String {
        self.alt_screen.clone()
    }

    pub fn set_alt_screen(&mut self, alt_screen: String) {
        self.alt_screen = alt_screen.clone()
    }

    pub fn get_normal_screen(&self) -> String {
        self.normal_screen.clone()
    }

    pub fn set_normal_screen(&mut self, normal_screen: String) {
        self.normal_screen = normal_screen.clone()
    }

    pub fn get_clear(&self) -> String {
        self.clear.clone()
    }

    pub fn set_clear(&mut self, clear: String) {
        self.clear = clear.clone()
    }

    pub fn get_mouse_on(&self) -> String {
        self.mouse_on.clone()
    }

    pub fn set_mouse_on(&mut self, mouse_on: String) {
        self.mouse_on = mouse_on.clone()
    }

    pub fn get_mouse_off(&self) -> String {
        self.mouse_off.clone()
    }

    pub fn set_mouse_off(&mut self, mouse_off: String) {
        self.mouse_off = mouse_off.clone()
    }

    pub fn get_mouse_direct_on(&self) -> String {
        self.mouse_direct_on.clone()
    }

    pub fn set_mouse_direct_on(&mut self, mouse_direct_on: String) {
        self.mouse_direct_on = mouse_direct_on.clone()
    }

    pub fn get_mouse_direct_off(&self) -> String {
        self.mouse_direct_off.clone()
    }

    pub fn set_mouse_direct_off(&mut self, mouse_direct_off: String) {
        self.mouse_direct_off = mouse_direct_off.clone()
    }

    pub fn get_winch(&self) -> Event {
        self.winch.clone()
    }

    pub fn set_winch(&mut self, winch: Event) {
        self.winch = winch.clone()
    }
}

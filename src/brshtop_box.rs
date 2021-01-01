use {
    crate::{
        config::{Config, ViewMode},
        cpubox::CpuBox,
        draw::Draw,
        error::*,
        fx,
        key::Key,
        membox::MemBox,
        menu::Menu,
        mv, symbol,
        term::Term,
        theme::Theme,
    },
    battery::Manager,
    chrono::{offset::Local, DateTime},
    std::{collections::HashMap, time::SystemTime},
    uname::uname,
};

pub enum Boxes<'a> {
    BrshtopBox(&'a mut BrshtopBox),
    CpuBox(&'a mut CpuBox),
    MemBox(&'a mut MemBox),
}

pub enum SubBoxes<'a> {
    CpuBox(&'a mut CpuBox),
}

pub struct BrshtopBox {
    pub name: String,
    pub height_p: u32,
    pub width_p: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub proc_mode: bool,
    pub stat_mode: bool,
    pub out: String,
    pub bg: String,
    pub _b_cpu_h: i32,
    pub _b_mem_h: i32,
    pub redraw_all: bool,
    pub buffers: Vec<String>,
    pub clock_on: bool,
    pub clock: String,
    pub clock_len: u32,
    pub resized: bool,
    pub clock_custom_format: HashMap<String, String>,
}
impl BrshtopBox {
    pub fn new(config: &mut Config, ARG_MODE: ViewMode) -> Self {
        let proc_mode_mut = (config.view_mode == ViewMode::Proc && ARG_MODE == ViewMode::None)
            || ARG_MODE == ViewMode::Proc;
        let stat_mode_mut = (config.view_mode == ViewMode::Stat && ARG_MODE == ViewMode::None)
            || ARG_MODE == ViewMode::Stat;

        let ccfm = HashMap::<String, String>::new();
        let u = match uname() {
            Ok(info) => info,
            Err(e) => {
                throw_error("Unable to get uname info!");
                uname().unwrap()
            }
        };
        ccfm.insert(String::from("/host"), u.nodename.replace(".local", ""));

        ccfm.insert(
            String::from("/user"),
            match std::env::var("USER") {
                Ok(user) => user,
                Err(_) => {
                    throw_error("Unable to get username info!");
                    String::default()
                }
            },
        );

        BrshtopBox {
            name: String::from(""),
            height_p: 0,
            width_p: 0,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            proc_mode: proc_mode_mut,
            stat_mode: false,
            out: String::from(""),
            bg: String::from(""),
            _b_cpu_h: 0,
            _b_mem_h: 0,
            redraw_all: false,
            buffers: Vec::<String>::new(),
            clock_on: false,
            clock: String::from(""),
            clock_len: 0,
            resized: false,
            clock_custom_format: HashMap::<String, String>::new(),
        }
    }

    pub fn calc_sizes(&mut self, boxes: Vec<Boxes>, THREADS: u64, term: &mut Term) {
        for sub in boxes {
            //TODO : Fill in rest of sub-boxes
            match sub {
                Boxes::BrshtopBox(b) => (),
                Boxes::CpuBox(b) => {
                    b.calc_size(THREADS, term, self);
                    b.resized = true;
                }
                Boxes::MemBox(b) => {
                    b.calc_sizes(boxes);
                    b.resized = true;
                }
            }
        }
    }

    /// Defaults now = true
    pub fn draw_update_ms(
        &mut self,
        now: bool,
        config: &mut Config,
        cpu_box: &mut CpuBox,
        key: &mut Key,
        draw: &mut Draw,
        menu: &mut Menu,
        theme: &mut Theme,
        term: &mut Term,
    ) {
        let mut update_string: String = format!("{}ms", config.update_ms);
        let xpos: u32 = cpu_box.x + cpu_box.parent.width - (update_string.len() as u32) - 15;

        if !key.mouse.contains("+".to_owned()) {
            let mut add_for_mouse_parent = Vec::<Vec<u32>>::new();
            let mut add_for_mouse = Vec::<u32>::new();
            for i in 0..3 {
                add_for_mouse.push(xpos + 7 + i);
                add_for_mouse.push(cpu_box.y);
            }
            add_for_mouse_parent.push(add_for_mouse);
            key.mouse.set("+".to_owned(), add_for_mouse_parent);
            let mut sub_for_mouse_parent = Vec::<Vec<u32>>::new();
            let mut sub_for_mouse = Vec::<u32>::new();
            for i in 0..3 {
                sub_for_mouse.push(cpu_box.x + cpu_box.parent.width - 4 + i);
                sub_for_mouse.push(cpu_box.y);
            }
            sub_for_mouse_parent.push(sub_for_mouse);
            key.mouse.set("-".to_owned(), sub_for_mouse_parent);
        }

        draw.buffer(
            if now && !menu.active {
                String::from("update_ms!")
            } else {
                String::from("update_ms")
            },
            vec![
                format!(
                    "{}{}{}{} ",
                    mv::to(cpu_box.y, xpos),
                    theme.colors.cpu_box.call(
                        format!("{}{}", symbol::h_line.repeat(7), symbol::title_left),
                        term
                    ),
                    fx::b,
                    theme.colors.hi_fg.call("+".to_owned(), term)
                ),
                format!(
                    "{} {}{}{}",
                    theme.colors.title.call(update_string, term),
                    theme.colors.hi_fg.call("-".to_owned(), term),
                    fx::ub,
                    theme.colors.cpu_box.call(symbol::title_right.to_owned(), term)
                ),
            ],
            false,
            100,
            menu.active,
            false,
            true,
        );

        if now && !menu.active {
            draw.clear(vec!["update_ms".to_owned()], false);
            if config.show_battery {
                match Manager::new() {
                    Ok(m) => match m.batteries() {
                        Ok(b) => match b.into_iter().size_hint() {
                            (0, Some(_)) => draw.out("battery".to_owned()),
                            _ => (),
                        },
                        _ => (),
                    },
                    Err(e) => (),
                };
            }
        }
    }

    /// Defaults force : bool = false
    pub fn draw_clock(
        &mut self,
        force: bool,
        term: &mut Term,
        config: &mut Config,
        theme: &mut Theme,
        menu: &mut Menu,
        cpu_box: &mut CpuBox,
        draw: &mut Draw,
    ) {
        let mut out: String = String::default();

        let system_time = SystemTime::now();
        let datetime: DateTime<Local> = system_time.into();

        if !force
            && (!self.clock_on
                || term.resized
                || datetime.format(config.draw_clock.as_str()).to_string() == self.clock)
        {
            return;
        }

        let mut clock_string: String = datetime
            .format(config.draw_clock.as_str())
            .to_string()
            .clone();
        self.clock = datetime
            .format(config.draw_clock.as_str())
            .to_string()
            .clone();
        for (custom, value) in self.clock_custom_format {
            if clock_string.contains(custom.as_str()) {
                clock_string = clock_string.replace(custom.as_str(), value.as_str())
            }
        }

        let clock_len = clock_string[..cpu_box.parent.width as usize - 56].len();

        if self.clock_len != clock_len as u32 && !cpu_box.resized {
            out = format!(
                "{}{}{}{}",
                mv::to(
                    cpu_box.y,
                    ((cpu_box.parent.width) / 2) as u32 - (clock_len / 2) as u32
                ),
                fx::ub,
                theme.colors.cpu_box,
                symbol::h_line.repeat(self.clock_len as usize)
            );
        }
        self.clock_len = clock_len.clone() as u32;
        let now: bool = if menu.active { false } else { !force };

        out.push_str(
            format!(
                "{}{}{}{}{}{}{}{}{}{}",
                mv::to(
                    cpu_box.y,
                    (cpu_box.parent.width / 2) as u32 - (clock_len / 2) as u32
                ),
                fx::ub,
                theme.colors.cpu_box,
                symbol::title_left,
                fx::b,
                theme
                    .colors.title
                    .call(clock_string[..clock_len as usize].to_string(), term),
                fx::ub,
                theme.colors.cpu_box,
                symbol::title_right,
                term.fg
            )
            .as_str(),
        );

        draw.buffer(
            "clock".to_owned(),
            vec![out],
            false,
            now,
            100,
            menu.active,
            false,
            !force,
        );

        if now && !menu.active && config.show_battery {
            match Manager::new() {
                Ok(m) => match m.batteries() {
                    Ok(b) => match b.into_iter().size_hint() {
                        (0, Some(_)) => draw.out("battery".to_owned()),
                        _ => (),
                    },
                    _ => (),
                },
                Err(e) => (),
            };
        }
    }

    /// Draw all boxes outlines and titles -> Default now : bool = true
    pub fn draw_bg(
        &mut self,
        now: bool,
        draw: &mut Draw,
        subclasses: Vec<Boxes>,
        menu: &mut Menu,
        config: &mut Config,
        cpu_box: &mut CpuBox,
        key: &mut Key,
        theme: &mut Theme,
        term: &mut Term,
    ) {
        // TODO : Handle the rest of the possible boxes...
        draw.buffer(
            "bg".to_owned(),
            subclasses
                .into_iter()
                .map(|b| match b {
                    Boxes::CpuBox(cb) => cb.draw_bg(),
                    _ => String::default(),
                })
                .collect(),
            false,
            now,
            1000,
            menu.active,
            false,
            true,
        );

        self.draw_update_ms(now, config, cpu_box, key, draw, menu, theme, term);

        if config.draw_clock != String::default() {
            self.draw_clock(true, term, config, theme, menu, cpu_box, draw);
        }
    }
}

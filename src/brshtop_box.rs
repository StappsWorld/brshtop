use {
    crate::{
        config::{Config, ViewMode, ViewModeEnum},
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        draw::Draw,
        error::*,
        fx,
        key::Key,
        membox::MemBox,
        menu::Menu,
        mv,
        netbox::NetBox,
        procbox::ProcBox,
        symbol,
        term::Term,
        theme::Theme,
        CPU_NAME,
    },
    battery::Manager,
    chrono::{offset::Local, DateTime},
    once_cell::sync::OnceCell,
    std::{collections::HashMap, sync::Mutex, time::SystemTime},
    uname::uname,
};

#[derive(Clone)]
pub enum Boxes {
    BrshtopBox,
    CpuBox,
    MemBox,
    NetBox,
    ProcBox,
}

#[derive(Clone)]
pub enum SubBoxes {
    CpuBox,
    NetBox,
}

#[derive(Clone)]
pub struct BrshtopBox {
    name: String,
    height_p: u32,
    width_p: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    proc_mode: bool,
    stat_mode: bool,
    out: String,
    bg: String,
    _b_cpu_h: i32,
    _b_mem_h: i32,
    redraw_all: bool,
    buffers: Vec<String>,
    clock_on: bool,
    clock: String,
    clock_len: u32,
    resized: bool,
    clock_custom_format: HashMap<String, String>,
}
impl BrshtopBox {
    pub fn new(config: &OnceCell<Mutex<Config>>, ARG_MODE: ViewMode) -> Self {
        let proc_mode_mut = (config.get().unwrap().try_lock().unwrap().view_mode.t
            == ViewModeEnum::Proc
            && ARG_MODE.t == ViewModeEnum::None)
            || ARG_MODE.t == ViewModeEnum::Proc;
        let stat_mode_mut = (config.get().unwrap().try_lock().unwrap().view_mode.t
            == ViewModeEnum::Stat
            && ARG_MODE.t == ViewModeEnum::None)
            || ARG_MODE.t == ViewModeEnum::Stat;

        let mut ccfm = HashMap::<String, String>::new();
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

    pub fn calc_sizes(
        &mut self,
        boxes: Vec<Boxes>,
        term: &OnceCell<Mutex<Term>>,
        CONFIG: &OnceCell<Mutex<Config>>,
        cpu: &OnceCell<Mutex<CpuCollector>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        mem_box: &OnceCell<Mutex<MemBox>>,
        net_box: &OnceCell<Mutex<NetBox>>,
        proc_box: &OnceCell<Mutex<ProcBox>>,
    ) {
        for sub in boxes {
            match sub {
                Boxes::BrshtopBox => (),
                Boxes::CpuBox => {
                    self._b_cpu_h = cpu_box.get().unwrap().try_lock().unwrap().calc_size(
                        term,
                        self._b_cpu_h.clone(),
                        cpu,
                    );
                    cpu_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .set_parent_resized(true);
                }
                Boxes::MemBox => {
                    let mut _b_mem_h_mutable: i32 = self._b_mem_h.clone();
                    _b_mem_h_mutable = mem_box.get().unwrap().try_lock().unwrap().calc_size(
                        term,
                        _b_mem_h_mutable,
                        self.get_b_cpu_h(),
                        CONFIG,
                    );
                    self._b_mem_h = _b_mem_h_mutable.clone();
                    mem_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .set_parent_resized(true);
                }
                Boxes::NetBox => {
                    net_box.get().unwrap().try_lock().unwrap().calc_size(
                        term,
                        self.get_b_cpu_h(),
                        self.get_b_mem_h(),
                    );
                    net_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .set_parent_resized(true);
                }
                Boxes::ProcBox => {
                    proc_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .calc_size(term, self.get_b_cpu_h());
                    proc_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .set_parent_resized(true);
                }
            }
        }
    }

    /// Defaults now = true
    pub fn draw_update_ms(
        &mut self,
        now: bool,
        config: &OnceCell<Mutex<Config>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        key: &OnceCell<Mutex<Key>>,
        draw: &OnceCell<Mutex<Draw>>,
        menu: &OnceCell<Mutex<Menu>>,
        theme: &OnceCell<Mutex<Theme>>,
        term: &OnceCell<Mutex<Term>>,
    ) {
        let mut update_string: String =
            format!("{}ms", config.get().unwrap().try_lock().unwrap().update_ms);
        let xpos: u32 = cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_x()
            + cpu_box
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .get_parent()
                .get_width()
            - (update_string.len() as u32)
            - 15;

        if !key
            .get()
            .unwrap()
            .try_lock()
            .unwrap()
            .mouse
            .contains_key(&"+".to_owned())
        {
            let mut add_for_mouse_parent = Vec::<Vec<i32>>::new();
            let mut add_for_mouse = Vec::<i32>::new();
            for i in 0..3 {
                add_for_mouse.push((xpos + 7 + i) as i32);
                add_for_mouse
                    .push((cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_y()) as i32);
            }
            add_for_mouse_parent.push(add_for_mouse);
            key.get()
                .unwrap()
                .try_lock()
                .unwrap()
                .mouse
                .insert("+".to_owned(), add_for_mouse_parent);
            let mut sub_for_mouse_parent = Vec::<Vec<i32>>::new();
            let mut sub_for_mouse = Vec::<i32>::new();
            for i in 0..3 {
                sub_for_mouse.push(
                    (cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_x()
                        + cpu_box
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .get_parent()
                            .get_width()
                        - 4
                        + i) as i32,
                );
                sub_for_mouse
                    .push(cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_y() as i32);
            }
            sub_for_mouse_parent.push(sub_for_mouse);
            key.get()
                .unwrap()
                .try_lock()
                .unwrap()
                .mouse
                .insert("-".to_owned(), sub_for_mouse_parent);
        }

        draw.get().unwrap().try_lock().unwrap().buffer(
            if now && !menu.get().unwrap().try_lock().unwrap().active {
                String::from("update_ms!")
            } else {
                String::from("update_ms")
            },
            vec![
                format!(
                    "{}{}{}{} ",
                    mv::to(
                        cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_y(),
                        xpos
                    ),
                    theme.get().unwrap().try_lock().unwrap().colors.cpu_box.call(
                        format!("{}{}", symbol::h_line.repeat(7), symbol::title_left),
                        term
                    ),
                    fx::b,
                    theme
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .hi_fg
                        .call("+".to_owned(), term)
                ),
                format!(
                    "{} {}{}{}",
                    theme
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .title
                        .call(update_string, term),
                    theme
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .hi_fg
                        .call("-".to_owned(), term),
                    fx::ub,
                    theme
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .cpu_box
                        .call(symbol::title_right.to_owned(), term)
                ),
            ],
            false,
            false,
            100,
            menu.get().unwrap().try_lock().unwrap().active,
            false,
            true,
            key,
        );

        if now && !menu.get().unwrap().try_lock().unwrap().active {
            draw.get()
                .unwrap()
                .try_lock()
                .unwrap()
                .clear(vec!["update_ms".to_owned()], false);
            if config.get().unwrap().try_lock().unwrap().show_battery {
                match Manager::new() {
                    Ok(m) => match m.batteries() {
                        Ok(b) => match b.into_iter().size_hint() {
                            (0, Some(_)) => draw.get().unwrap().try_lock().unwrap().out(
                                vec!["battery".to_owned()],
                                false,
                                key,
                            ),
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
        term: &OnceCell<Mutex<Term>>,
        config: &OnceCell<Mutex<Config>>,
        theme: &OnceCell<Mutex<Theme>>,
        menu: &OnceCell<Mutex<Menu>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        draw: &OnceCell<Mutex<Draw>>,
        key: &OnceCell<Mutex<Key>>,
    ) {
        let mut out: String = String::default();

        let system_time = SystemTime::now();
        let datetime: DateTime<Local> = system_time.into();

        if !force
            && (!self.get_clock_on()
                || term.get().unwrap().try_lock().unwrap().get_resized()
                || datetime
                    .format(config.get().unwrap().try_lock().unwrap().draw_clock.as_str())
                    .to_string()
                    == self.get_clock())
        {
            return;
        }

        let mut clock_string: String = datetime
            .format(config.get().unwrap().try_lock().unwrap().draw_clock.as_str())
            .to_string()
            .clone();
        self.clock = datetime
            .format(config.get().unwrap().try_lock().unwrap().draw_clock.as_str())
            .to_string()
            .clone();
        for (custom, value) in self.clock_custom_format.clone() {
            if clock_string.contains(custom.as_str()) {
                clock_string = clock_string.replace(custom.as_str(), value.as_str())
            }
        }

        let clock_len = clock_string[..cpu_box
            .get()
            .unwrap()
            .try_lock()
            .unwrap()
            .get_parent()
            .get_width() as usize
            - 56]
            .len();

        if self.clock_len != clock_len as u32
            && !cpu_box
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .get_parent()
                .get_resized()
        {
            out = format!(
                "{}{}{}{}",
                mv::to(
                    cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_y(),
                    ((cpu_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .get_parent()
                        .get_width())
                        / 2) as u32
                        - (clock_len / 2) as u32
                ),
                fx::ub,
                theme.get().unwrap().try_lock().unwrap().colors.cpu_box,
                symbol::h_line.repeat(self.clock_len as usize)
            );
        }
        self.clock_len = clock_len.clone() as u32;
        let now: bool = if menu.get().unwrap().try_lock().unwrap().active {
            false
        } else {
            !force
        };

        out.push_str(
            format!(
                "{}{}{}{}{}{}{}{}{}{}",
                mv::to(
                    cpu_box.get().unwrap().try_lock().unwrap().get_parent().get_y(),
                    (cpu_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .get_parent()
                        .get_width()
                        / 2) as u32
                        - (clock_len / 2) as u32
                ),
                fx::ub,
                theme.get().unwrap().try_lock().unwrap().colors.cpu_box,
                symbol::title_left,
                fx::b,
                theme
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .colors
                    .title
                    .call(clock_string[..clock_len as usize].to_string(), term),
                fx::ub,
                theme.get().unwrap().try_lock().unwrap().colors.cpu_box,
                symbol::title_right,
                term.get().unwrap().try_lock().unwrap().get_fg()
            )
            .as_str(),
        );

        draw.get().unwrap().try_lock().unwrap().buffer(
            "clock".to_owned(),
            vec![out],
            false,
            now,
            100,
            menu.get().unwrap().try_lock().unwrap().active,
            false,
            !force,
            key,
        );

        if now
            && !menu.get().unwrap().try_lock().unwrap().active
            && config.get().unwrap().try_lock().unwrap().show_battery
        {
            match Manager::new() {
                Ok(m) => match m.batteries() {
                    Ok(b) => match b.into_iter().size_hint() {
                        (0, Some(_)) => draw.get().unwrap().try_lock().unwrap().out(
                            vec!["battery".to_owned()],
                            false,
                            key,
                        ),
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
        draw: &OnceCell<Mutex<Draw>>,
        subclasses: Vec<Boxes>,
        menu: &OnceCell<Mutex<Menu>>,
        config: &OnceCell<Mutex<Config>>,
        cpu_box: &OnceCell<Mutex<CpuBox>>,
        mem_box: &OnceCell<Mutex<MemBox>>,
        net_box: &OnceCell<Mutex<NetBox>>,
        proc_box: &OnceCell<Mutex<ProcBox>>,
        key: &OnceCell<Mutex<Key>>,
        theme: &OnceCell<Mutex<Theme>>,
        term: &OnceCell<Mutex<Term>>,
    ) {
        draw.get().unwrap().try_lock().unwrap().buffer(
            "bg".to_owned(),
            subclasses
                .into_iter()
                .map(|b| match b {
                    Boxes::CpuBox => cpu_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .draw_bg(key, theme, term, config, cpu_box),
                    Boxes::MemBox => mem_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .draw_bg(theme, config, term, mem_box),
                    Boxes::NetBox => net_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .draw_bg(theme, term, net_box),
                    Boxes::ProcBox => proc_box
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .draw_bg(theme, term, proc_box),
                    _ => String::default(),
                })
                .collect(),
            false,
            now,
            1000,
            menu.get().unwrap().try_lock().unwrap().active,
            false,
            true,
            key,
        );

        self.draw_update_ms(now, config, cpu_box, key, draw, menu, theme, term);

        if config.get().unwrap().try_lock().unwrap().draw_clock != String::default() {
            self.draw_clock(true, term, config, theme, menu, cpu_box, draw, key);
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name.clone()
    }

    pub fn get_height_p(&self) -> u32 {
        self.height_p.clone()
    }

    pub fn set_height_p(&mut self, height_p: u32) {
        self.height_p = height_p.clone()
    }

    pub fn get_width_p(&self) -> u32 {
        self.width_p.clone()
    }

    pub fn set_width_p(&mut self, width_p: u32) {
        self.width_p = width_p
    }

    pub fn get_x(&self) -> u32 {
        self.x.clone()
    }

    pub fn set_x(&mut self, x: u32) {
        self.x = x.clone()
    }

    pub fn get_y(&self) -> u32 {
        self.y.clone()
    }

    pub fn set_y(&mut self, y: u32) {
        self.y = y.clone()
    }

    pub fn get_width(&self) -> u32 {
        self.width.clone()
    }

    pub fn set_width(&mut self, width: u32) {
        self.width = width.clone()
    }

    pub fn get_height(&self) -> u32 {
        self.height.clone()
    }

    pub fn set_height(&mut self, height: u32) {
        self.height = height.clone()
    }

    pub fn get_proc_mode(&self) -> bool {
        self.proc_mode.clone()
    }

    pub fn set_proc_mode(&mut self, proc_mode: bool) {
        self.proc_mode = proc_mode.clone()
    }

    pub fn get_stat_mode(&self) -> bool {
        self.stat_mode.clone()
    }

    pub fn set_stat_mode(&mut self, stat_mode: bool) {
        self.stat_mode = stat_mode.clone()
    }

    pub fn get_out(&self) -> String {
        self.out.clone()
    }

    pub fn set_out(&mut self, out: String) {
        self.out = out.clone()
    }

    pub fn get_bg(&self) -> String {
        self.bg.clone()
    }

    pub fn set_bg(&mut self, bg: String) {
        self.bg = bg.clone()
    }

    pub fn get_b_cpu_h(&self) -> i32 {
        self._b_cpu_h.clone()
    }

    pub fn set_b_cpu_h(&mut self, _b_cpu_h: i32) {
        self._b_cpu_h = _b_cpu_h.clone()
    }

    pub fn get_b_cpu_h_mut(&mut self) -> &mut i32 {
        &mut self._b_cpu_h
    }

    pub fn get_b_mem_h(&self) -> i32 {
        self._b_mem_h.clone()
    }

    pub fn set_b_mem_h(&mut self, _b_mem_h: i32) {
        self._b_mem_h = _b_mem_h.clone()
    }

    pub fn get_b_mem_h_mut(&mut self) -> &mut i32 {
        &mut self._b_mem_h
    }

    pub fn get_redraw_all(&self) -> bool {
        self.redraw_all.clone()
    }

    pub fn set_redraw_all(&mut self, redraw_all: bool) {
        self.redraw_all = redraw_all.clone()
    }

    pub fn get_buffers(&self) -> Vec<String> {
        self.buffers.clone()
    }

    pub fn set_buffers(&mut self, buffers: Vec<String>) {
        self.buffers = buffers.clone()
    }

    pub fn push_buffers(&mut self, element: String) {
        self.buffers.push(element.clone())
    }

    pub fn get_buffers_index(&self, index: usize) -> Option<String> {
        match self.buffers.get(index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_buffers_index(&mut self, index: usize, element: String) {
        self.buffers.insert(index, element.clone())
    }

    pub fn get_clock_on(&self) -> bool {
        self.clock_on.clone()
    }

    pub fn set_clock_on(&mut self, clock_on: bool) {
        self.clock_on = clock_on.clone()
    }

    pub fn get_clock(&self) -> String {
        self.clock.clone()
    }

    pub fn set_clock(&mut self, clock: String) {
        self.clock = clock.clone()
    }

    pub fn get_clock_len(&self) -> u32 {
        self.clock_len.clone()
    }

    pub fn set_clock_len(&mut self, clock_len: u32) {
        self.clock_len = clock_len.clone()
    }

    pub fn get_resized(&self) -> bool {
        self.resized.clone()
    }

    pub fn set_resized(&mut self, resized: bool) {
        self.resized = resized.clone()
    }

    pub fn get_clock_custom_format(&self) -> HashMap<String, String> {
        self.clock_custom_format.clone()
    }

    pub fn set_clock_custom_format(&mut self, clock_custom_format: HashMap<String, String>) {
        self.clock_custom_format = clock_custom_format.clone()
    }

    pub fn get_clock_custom_format_index(&self, index: String) -> Option<String> {
        match self.clock_custom_format.get(&index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_clock_custom_format_index(&mut self, index: String, element: String) {
        self.clock_custom_format
            .insert(index.clone(), element.clone());
    }
}

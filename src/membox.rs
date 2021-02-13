use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::Collector,
        config::{Config, ViewMode},
        create_box,
        draw::Draw,
        fx,
        fx::Fx,
        graph::{ColorSwitch, Graph},
        key::Key,
        memcollector::{DiskInfo, MemCollector},
        menu::Menu,
        meter::{Meter, MeterUnion, Meters},
        mv, symbol,
        term::Term,
        theme::Theme,
    },
    inflector::Inflector,
    math::round::ceil,
    once_cell::sync::OnceCell,
    std::{collections::HashMap, convert::TryFrom, sync::Mutex},
};

pub struct MemBox {
    parent: BrshtopBox,
    mem_meter: i32,
    mem_size: usize,
    disk_meter: i32,
    divider: i32,
    mem_width: u32,
    disks_width: u32,
    graph_height: u32,
    redraw: bool,
    buffer: String,
    swap_on: bool,
    mem_names: Vec<String>,
    swap_names: Vec<String>,
}
impl MemBox {
    pub fn new(brshtop_box: &BrshtopBox, CONFIG: &Config, ARG_MODE: ViewMode) -> Self {
        let mut membox = MemBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
            mem_meter: 0,
            mem_size: 0,
            disk_meter: 0,
            divider: 0,
            mem_width: 0,
            disks_width: 0,
            graph_height: 0,
            redraw: false,
            buffer: "mem".to_owned(),
            swap_on: CONFIG.show_swap,
            mem_names: vec!["used", "available", "cached", "free"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect(),
            swap_names: vec!["used", "free"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect(),
        };
        brshtop_box.push_buffers(membox.buffer.clone());

        membox.set_parent_name("mem".to_owned());
        membox.set_parent_height_p(38);
        membox.set_parent_width_p(45);
        membox.set_parent_x(1);
        membox.set_parent_y(1);
        membox.set_parent_resized(true);
        membox
    }

    pub fn calc_size(&mut self, term: &Term, b_mem_h: i32, b_cpu_h: i32, CONFIG: &Config) -> i32 {
        let mut width_p: u32 = 0;
        let mut height_p: u32 = 0;

        if self.get_parent().get_stat_mode() {
            width_p = 100;
            height_p = self.get_parent().get_height_p();
        } else {
            width_p = self.get_parent().get_width_p();
            height_p = self.get_parent().get_height_p();
        }
        self.set_parent_width(term.get_width() as u32 * width_p / 100);
        self.set_parent_height((term.get_height() as u32 * height_p / 100) + 1);
        let mut set_b_mem_h = b_mem_h.clone();
        set_b_mem_h = self.get_parent().get_height() as i32;
        self.set_parent_y(u32::try_from(b_cpu_h + 1).unwrap_or(0));
        if CONFIG.show_disks {
            self.set_mem_width(
                u32::try_from(
                    ceil((self.get_parent().get_width() as i32 - 3) as f64 / 2.0, 0) as i32,
                )
                .unwrap_or(0),
            );
            self.set_disks_width(
                u32::try_from(
                    self.get_parent().get_width() as i32 - self.get_mem_width() as i32 - 3,
                )
                .unwrap_or(0),
            );
            if ((self.get_mem_width() + self.get_disks_width()) as i32)
                < self.get_parent().get_width() as i32 - 2
            {
                self.set_mem_width(self.get_mem_width() + 1);
            }
            self.set_divider(self.get_parent().get_x() as i32 + self.get_mem_width() as i32);
        } else {
            self.set_mem_width(
                u32::try_from(self.get_parent().get_width() as i32 - 1).unwrap_or(0),
            );
        }

        let mut item_height: u32 = if self.get_swap_on() && !CONFIG.swap_disk {
            6
        } else {
            4
        };
        self.set_mem_width(
            if self.get_parent().get_height()
                - if self.get_swap_on() && !CONFIG.swap_disk {
                    3
                } else {
                    2
                }
                > 2 * item_height
            {
                3
            } else if self.get_mem_width() > 25 {
                2
            } else {
                1
            },
        );

        self.set_mem_meter(
            (self.get_parent().get_width()
                - if CONFIG.show_disks {
                    self.get_disks_width()
                } else {
                    0
                }
                - if self.get_mem_size() > 2 { 9 } else { 20 }) as i32,
        );

        if self.get_mem_size() == 1 {
            self.set_mem_meter(self.get_mem_meter() + 6);
        }
        if self.get_mem_meter() < 1 {
            self.set_mem_meter(0);
        }

        if CONFIG.mem_graphs {
            self.set_graph_height(
                ((self.get_parent().get_height()
                    - if self.get_swap_on() && !CONFIG.swap_disk {
                        2
                    } else {
                        1
                    })
                    - if self.mem_size == 3 { 2 } else { 1 } * item_height)
                    / item_height,
            );
            if self.get_graph_height() == 0 {
                self.set_graph_height(1);
            }
            if self.get_graph_height() > 1 {
                self.set_mem_meter(self.get_mem_meter() + 6);
            }
        } else {
            self.set_graph_height(0);
        }

        if CONFIG.show_disks {
            self.set_disk_meter(
                self.get_parent().get_width() as i32 - self.get_mem_width() as i32 - 23,
            );
            if self.get_disks_width() < 25 {
                self.set_disk_meter(self.get_disk_meter() + 10);
            }
            if self.get_disk_meter() < 1 {
                self.set_disk_meter(0);
            }
        }
        set_b_mem_h
    }

    pub fn draw_bg(&self, THEME: &Theme, CONFIG: &Config, term: &Term) -> String {
        if self.get_parent().get_proc_mode() {
            String::default()
        } else {
            let mut out: String = String::default();
            out.push_str(
                create_box(
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    Some(THEME.colors.mem_box),
                    None,
                    true,
                    Some(Boxes::MemBox),
                    &term.to_owned(),
                    &THEME.to_owned(),
                    None,
                    None,
                    Some(self),
                    None,
                    None,
                )
                .as_str(),
            );
            if CONFIG.show_disks {
                let mut adder: String = String::default();
                for i in 1..self.get_parent().get_height() - 1 {
                    adder.push_str(
                        format!(
                            "{}{}",
                            mv::to(
                                self.get_parent().get_y() as u32 + i,
                                self.get_divider() as u32
                            ),
                            symbol::v_line
                        )
                        .as_str(),
                    );
                }

                out.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}{}{}{}{}",
                        mv::to(self.get_parent().get_y(), self.get_divider() as u32 + 2),
                        THEME
                            .colors
                            .mem_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        THEME.colors.title.call("disks".to_owned(), term),
                        fx::ub,
                        THEME
                            .colors
                            .mem_box
                            .call(symbol::title_right.to_owned(), term),
                        mv::to(self.get_parent().get_y(), self.get_divider() as u32),
                        THEME.colors.mem_box.call(symbol::div_up.to_owned(), term),
                        mv::to(
                            self.get_parent().get_y() as u32 + self.get_parent().get_height() - 1,
                            self.get_divider() as u32
                        ),
                        THEME.colors.mem_box.call(symbol::div_down.to_owned(), term),
                        THEME.colors.div_line,
                        adder
                    )
                    .as_str(),
                );
            }
            out
        }
    }

    pub fn draw_fg(
        &mut self,
        mem: &MemCollector,
        term: &OnceCell<Mutex<Term>>,
        brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
        CONFIG: &OnceCell<Mutex<Config>>,
        meters: &OnceCell<Mutex<Meters>>,
        THEME: &OnceCell<Mutex<Theme>>,
        key: &OnceCell<Mutex<Key>>,
        collector: &Collector,
        draw: &OnceCell<Mutex<Draw>>,
        menu: &OnceCell<Mutex<Menu>>,
    ) {
        if self.get_parent().get_proc_mode() {
            return;
        }

        if mem.get_parent().get_redraw() {
            self.set_redraw(true);
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut gbg: String = String::default();
        let mut gmv: String = String::default();
        let mut gli: String = String::default();

        let parent_box: BrshtopBox = self.get_parent();
        let mut x = parent_box.get_x() + 1;
        let mut y = parent_box.get_y() + 1;
        let mut w = parent_box.get_width() - 2;
        let mut h = parent_box.get_height() - 2;

        if parent_box.get_resized() || self.get_redraw() {
            brshtop_box
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .set_b_mem_h(self.calc_size(
                    term,
                    brshtop_box.get_b_mem_h(),
                    brshtop_box.get_b_cpu_h(),
                    CONFIG,
                ));
            out_misc.push_str(self.draw_bg(THEME, CONFIG, term).as_str());
            meters
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .set_mem(HashMap::<String, MeterUnion>::new());
            meters
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .set_swap(HashMap::<String, MeterUnion>::new());
            meters
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .set_disks_used(HashMap::<String, Meter>::new());
            meters
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .set_disks_free(HashMap::<String, Meter>::new());
            if self.get_mem_meter() > 0 {
                for name in self.get_mem_names() {
                    if CONFIG.mem_graphs {
                        meters.set_mem_index(
                            name.clone(),
                            MeterUnion::Graph(Graph::new(
                                self.get_mem_meter(),
                                self.get_graph_height() as i32,
                                Some(ColorSwitch::VecString(
                                    THEME
                                        .get()
                                        .unwrap()
                                        .try_lock()
                                        .unwrap()
                                        .gradient
                                        .get(&name.clone())
                                        .unwrap()
                                        .clone(),
                                )),
                                mem.get_vlist_index(name.clone())
                                    .unwrap()
                                    .iter()
                                    .map(|u| u.to_owned() as i32)
                                    .collect(),
                                term,
                                false,
                                0,
                                0,
                                None,
                            )),
                        );
                    } else {
                        meters.set_mem_index(
                            name.clone(),
                            MeterUnion::Meter(Meter::new(
                                mem.get_percent_index(name.clone()).unwrap_or(0) as i32,
                                self.get_mem_meter() as u32,
                                name.clone(),
                                false,
                                THEME,
                                term,
                            )),
                        );
                    }
                }
                if self.get_swap_on() {
                    for name in self.get_swap_names() {
                        if CONFIG.mem_graphs && !CONFIG.swap_disk {
                            meters.set_swap_index(
                                name.clone(),
                                MeterUnion::Graph(Graph::new(
                                    self.get_mem_meter(),
                                    self.get_graph_height() as i32,
                                    Some(ColorSwitch::VecString(
                                        THEME
                                            .get()
                                            .unwrap()
                                            .try_lock()
                                            .unwrap()
                                            .gradient
                                            .get(&name.clone())
                                            .unwrap()
                                            .clone(),
                                    )),
                                    mem.get_vlist_index(name.clone())
                                        .unwrap_or(vec![])
                                        .iter()
                                        .map(|u| u.to_owned() as i32)
                                        .collect(),
                                    term,
                                    false,
                                    0,
                                    0,
                                    None,
                                )),
                            );
                        } else if CONFIG.swap_disk && CONFIG.show_disks {
                            meters.set_disks_used_index(
                                "__swap".to_owned(),
                                Meter::new(
                                    mem.get_swap_percent_index("used".to_owned()).unwrap_or(0)
                                        as i32,
                                    self.get_disk_meter() as u32,
                                    "used".to_owned(),
                                    false,
                                    THEME,
                                    term,
                                ),
                            );
                            if mem.get_disks().len() * 3 <= h as usize + 1 {
                                meters.set_disks_free_index(
                                    "__swap".to_owned(),
                                    Meter::new(
                                        mem.get_swap_percent_index("free".to_owned()).unwrap_or(0)
                                            as i32,
                                        self.get_mem_meter() as u32,
                                        "free".to_owned(),
                                        false,
                                        THEME,
                                        term,
                                    ),
                                );
                            }
                            break;
                        } else {
                            meters.set_swap_index(
                                name.clone(),
                                MeterUnion::Meter(Meter::new(
                                    mem.get_swap_percent_index(name.clone()).unwrap_or(0) as i32,
                                    self.get_mem_meter() as u32,
                                    name,
                                    false,
                                    THEME,
                                    term,
                                )),
                            );
                        }
                    }
                }
            }
            if self.get_disk_meter() > 0 {
                for (n, name) in mem.get_disks().keys().enumerate() {
                    if n * 2 > h as usize {
                        break;
                    }
                    meters.set_disks_used_index(
                        name.clone(),
                        Meter::new(
                            match mem
                                .get_disks_inner_index(name.clone(), "used_percent".to_owned())
                                .unwrap_or(DiskInfo::U64(0))
                            {
                                DiskInfo::U64(u) => u as i32,
                                DiskInfo::U32(u) => u as i32,
                                DiskInfo::String(s) => s.parse::<i32>().unwrap_or(0),
                                DiskInfo::None => 0,
                            },
                            self.get_disk_meter() as u32,
                            "used".to_owned(),
                            false,
                            THEME,
                            term,
                        ),
                    );
                    if mem.get_disks().len() * 3 <= h as usize + 1 {
                        meters.set_disks_free_index(
                            name.clone(),
                            Meter::new(
                                match mem
                                    .get_disks_inner_index(name.clone(), "free_percent".to_owned())
                                    .unwrap_or(DiskInfo::U64(0))
                                {
                                    DiskInfo::U64(u) => u as i32,
                                    DiskInfo::U32(u) => u as i32,
                                    DiskInfo::String(s) => s.parse::<i32>().unwrap_or(0),
                                    DiskInfo::None => 0,
                                },
                                self.get_disk_meter() as u32,
                                "free".to_owned(),
                                false,
                                THEME,
                                term,
                            ),
                        );
                    }
                }
            }
            if !key
                .get()
                .unwrap()
                .try_lock()
                .unwrap()
                .mouse
                .contains_key(&"g".to_owned())
            {
                let mut top = Vec::<Vec<i32>>::new();
                for i in 0..5 {
                    let mut adder: Vec<i32> = Vec::<i32>::new();
                    adder.push(x as i32 + self.get_mem_width() as i32 - 8);
                    adder.push(y as i32 - 1);
                    top.push(adder);
                }
                key.get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .mouse
                    .insert("g".to_owned(), top);
            }
            out_misc.push_str(
                format!(
                    "{}{}{}{}{}{}{}",
                    mv::to(y as u32 - 1, x as u32 + w - 7),
                    THEME
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .mem_box
                        .call(symbol::title_left.to_owned(), term),
                    if CONFIG.mem_graphs { fx::b } else { "" },
                    THEME
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .hi_fg
                        .call("g".to_owned(), term),
                    THEME
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .title
                        .call("wap".to_owned(), term),
                    fx::ub,
                    THEME
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .mem_box
                        .call(symbol::title_right.to_owned(), term),
                )
                .as_str(),
            );
            if CONFIG.show_disks {
                if !key
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .mouse
                    .contains_key(&"s".to_owned())
                {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
                    for i in 0..4 {
                        let mut adder: Vec<i32> = Vec::<i32>::new();
                        adder.push(x as i32 + w as i32 - 6 + i);
                        adder.push(y as i32 - 1);
                        top.push(adder);
                    }
                    key.get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .mouse
                        .insert("s".to_owned(), top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}",
                        mv::to(y as u32 - 1, x as u32 + w - 7),
                        THEME
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .colors
                            .mem_box
                            .call(symbol::title_left.to_owned(), term),
                        if CONFIG.swap_disk { fx::b } else { "" },
                        THEME
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .colors
                            .hi_fg
                            .call("s".to_owned(), term),
                        THEME
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .colors
                            .title
                            .call("raph".to_owned(), term),
                        fx::ub,
                        THEME
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .colors
                            .mem_box
                            .call(symbol::title_right.to_owned(), term),
                    )
                    .as_str(),
                );
            }
            if collector.get_collect_interrupt() {
                return;
            }
            draw.buffer(
                "mem_misc".to_owned(),
                vec![out_misc.clone()],
                false,
                false,
                100,
                true,
                false,
                false,
                key,
            );
        }
        let mut cx: u32 = 1;
        let mut cy: u32 = 1;

        out.push_str(
            format!(
                "{}{}{}Total:{:>width$}{}{}",
                mv::to(y as u32, x as u32 + 1),
                THEME.colors.title,
                fx::b,
                mem.get_string_index("total".to_owned())
                    .unwrap_or(String::default()),
                fx::ub,
                THEME.colors.main_fg,
                width = self.get_mem_width() as usize - 9,
            )
            .as_str(),
        );
        if self.get_graph_height() > 0 {
            gli = format!(
                "{}{}{}{}{}{}{}{}",
                mv::left(2),
                THEME
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .colors
                    .mem_box
                    .call(symbol::title_right.to_owned(), term),
                THEME.colors.div_line,
                symbol::h_line.repeat(self.get_mem_width() as usize - 1),
                if CONFIG.show_disks {
                    "".to_owned()
                } else {
                    THEME
                        .get()
                        .unwrap()
                        .try_lock()
                        .unwrap()
                        .colors
                        .mem_box
                        .to_string()
                },
                symbol::title_left,
                mv::left(self.get_mem_width() - 1),
                THEME.colors.title,
            );
            if self.get_graph_height() >= 2 {
                gbg = mv::left(1);
                gmv = format!(
                    "{}{}",
                    mv::left(u32::try_from(self.get_mem_width() as i32 - 2).unwrap_or(0)),
                    mv::up(u32::try_from(self.get_graph_height() as i32 - 1).unwrap_or(0))
                );
            }
        }

        let big_mem: bool = false;
        for name in self.get_mem_names() {
            if collector.get_collect_interrupt() {
                return;
            }
            if self.get_mem_size() > 2 {
                out.push_str(
                    format!(
                        "{}{}{:<width1$.width2$}{}{}{}{}{}{}{:>4}",
                        mv::to(y + cy, x + cx),
                        gli,
                        name.to_title_case()[if big_mem { ..name.len() } else { ..5 }].to_owned()
                            + ":",
                        mv::to(
                            y + cy,
                            u32::try_from(
                                x as i32 + cx as i32 + self.get_mem_width() as i32
                                    - 3
                                    - mem
                                        .get_string_index(name.clone())
                                        .unwrap_or(String::default())
                                        .len() as i32
                            )
                            .unwrap_or(0)
                        ),
                        Fx::trans(
                            mem.get_string_index(name.clone())
                                .unwrap_or(String::default())
                        ),
                        mv::to(y + cy + 1, x + cx),
                        gbg,
                        match meters
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .get_mem_index(name.clone())
                            .unwrap_or(MeterUnion::Meter(Meter::default()))
                        {
                            MeterUnion::Meter(m) => {
                                let mut m_callable = m.clone(); // TODO : May need to implement mutable references to meters and graphs
                                let save =
                                    m_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(mem.get_percent_index(name.clone()).unwrap_or(0)
                                                as i32)
                                        },
                                        term,
                                    );
                                meters
                                    .get()
                                    .unwrap()
                                    .try_lock()
                                    .unwrap()
                                    .set_mem_index(name.clone(), MeterUnion::Meter(m_callable));
                                save
                            }
                            MeterUnion::Graph(g) => {
                                let mut g_callable = g.clone(); // TODO : May need to implement mutable references to meters and graphs
                                let save =
                                    g_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(mem.get_percent_index(name.clone()).unwrap_or(0)
                                                as i32)
                                        },
                                        term,
                                    );
                                meters
                                    .get()
                                    .unwrap()
                                    .try_lock()
                                    .unwrap()
                                    .set_mem_index(name.clone(), MeterUnion::Graph(g_callable));
                                save
                            }
                        },
                        gmv,
                        mem.get_percent_index(name.clone()).unwrap_or(0).to_string() + "%",
                        width1 = if big_mem { 1 } else { 6 },
                        width2 = if big_mem { 0 } else { 6 },
                    )
                    .as_str(),
                );
                cy += if self.get_graph_height() == 0 {
                    2
                } else {
                    self.get_graph_height() + 1
                };
            } else {
                let mem_check = self.mem_size > 1;
                out.push_str(
                    format!(
                        "{}{:width1$.width2$} {}{}{:width3$}",
                        mv::to(y + cy, x + cx),
                        name.to_title_case(),
                        gbg,
                        match meters
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .get_mem_index(name.clone())
                            .unwrap_or(MeterUnion::Meter(Meter::default()))
                        {
                            MeterUnion::Graph(g) => {
                                let mut g_callable = g.clone();
                                g_callable.call(
                                    if self.get_parent().get_resized() {
                                        None
                                    } else {
                                        Some(
                                            mem
                                                .get_percent_index(name.clone())
                                                .unwrap_or(0)
                                                as i32,
                                        )
                                    },
                                    term,
                                )
                            }
                            MeterUnion::Meter(m) => {
                                let mut m_callable = m.clone();
                                m_callable.call(
                                    if self.get_parent().get_resized() {
                                        None
                                    } else {
                                        Some(
                                            mem
                                                .get_percent_index(name.clone())
                                                .unwrap_or(0)
                                                as i32,
                                        )
                                    },
                                    term,
                                )
                            }
                        },
                        match mem.get_string_index(name.clone()) {
                            Some(s) => (s.clone()[if mem_check {
                                ..s.len()
                            } else {
                                if s.len() as i32 - 3 > 0 {
                                    ..s.len() - 3
                                } else {
                                    ..0
                                }
                            }])
                            .to_owned(),
                            None => String::default(),
                        },
                        width1 = if mem_check { 5 } else { 1 },
                        width2 = if mem_check { 5 } else { 1 },
                        width3 = if mem_check { 9 } else { 7 },
                    )
                    .as_str(),
                );
                cy += if self.get_graph_height() == 0 {
                    1
                } else {
                    self.get_graph_height()
                };
            }
        }

        // * Swap
        if self.get_swap_on()
            && CONFIG.show_swap
            && !CONFIG.swap_disk
            && mem.get_swap_string().len() > 0
        {
            if h - cy > 5 {
                out.push_str(format!("{}{}", mv::to(y + cy, x + cx), gli).as_str());
            }
            cy += 1;
            out.push_str(
                format!(
                    "{}{}{}Swap:{:>width$}{}{}",
                    mv::to(y + cy, x + cx),
                    THEME.colors.title,
                    fx::b,
                    mem.get_swap_string_index("total".to_owned())
                        .unwrap_or(String::default()),
                    fx::ub,
                    THEME.colors.main_fg,
                    width = self.get_mem_width() as usize - 8,
                )
                .as_str(),
            );
            cy += 1;
            for name in self.get_swap_names() {
                if collector.get_collect_interrupt() {
                    return;
                }
                if self.get_mem_size() > 2 {
                    out.push_str(
                        format!(
                            "{}{}{:<width1$.width2$}{}{}{}{}{}{}{:>4}",
                            mv::to(y + cy, x + cx),
                            gli,
                            name.to_title_case()[..if big_mem { name.len() } else { 5 }].to_owned()
                                + ":",
                            mv::to(
                                y + cy,
                                u32::try_from(
                                    x as i32 + cx as i32 + self.get_mem_width() as i32
                                        - 3
                                        - mem
                                            .get_swap_string_index(name.clone())
                                            .unwrap_or(String::default())
                                            .len() as i32
                                )
                                .unwrap_or(0)
                            ),
                            Fx::trans(
                                mem.get_swap_string_index(name.clone())
                                    .unwrap_or(String::default())
                            ),
                            mv::to(y + cy + 1, x + cx),
                            gbg,
                            match meters
                                .get()
                                .unwrap()
                                .try_lock()
                                .unwrap()
                                .get_swap_index(name.clone())
                                .unwrap_or(MeterUnion::Meter(Meter::default()))
                            {
                                MeterUnion::Graph(g) => {
                                    let mut g_callable = g.clone();
                                    g_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(
                                                mem.get_swap_percent_index(name.clone())
                                                    .unwrap_or(0)
                                                    as i32,
                                            )
                                        },
                                        term,
                                    )
                                }
                                MeterUnion::Meter(m) => {
                                    let mut m_callable = m.clone();
                                    m_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(
                                                mem.get_swap_percent_index(name.clone())
                                                    .unwrap_or(0)
                                                    as i32,
                                            )
                                        },
                                        term,
                                    )
                                }
                            },
                            gmv,
                            mem.get_swap_percent_index(name.clone())
                                .unwrap_or(0)
                                .to_string()
                                + "%",
                            width1 = if big_mem { 1 } else { 6 },
                            width2 = if big_mem { 0 } else { 6 },
                        )
                        .as_str(),
                    );
                    cy += if self.get_graph_height() == 0 {
                        1
                    } else {
                        self.get_graph_height()
                    }
                } else {
                    let mem_check = self.get_mem_size() > 1;
                    out.push_str(
                        format!(
                            "{}{:width1$.width2$} {}{}{:width3$}",
                            mv::to(y + cy, x + cx),
                            name.to_title_case(),
                            gbg,
                            match meters
                                .get()
                                .unwrap()
                                .try_lock()
                                .unwrap()
                                .get_swap_index(name.clone())
                                .unwrap_or(MeterUnion::Meter(Meter::default()))
                            {
                                MeterUnion::Graph(g) => {
                                    let mut g_callable = g.clone();
                                    let save = g_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(mem.get_percent_index(name.clone()).unwrap_or(0)
                                                as i32)
                                        },
                                        term,
                                    );
                                    meters.set_swap_index(
                                        name.clone(),
                                        MeterUnion::Graph(g_callable),
                                    );
                                    save
                                }
                                MeterUnion::Meter(m) => {
                                    let mut m_callable = m.clone();
                                    let save = m_callable.call(
                                        if self.get_parent().get_resized() {
                                            None
                                        } else {
                                            Some(mem.get_percent_index(name.clone()).unwrap_or(0)
                                                as i32)
                                        },
                                        term,
                                    );
                                    meters.set_swap_index(
                                        name.clone(),
                                        MeterUnion::Meter(m_callable),
                                    );
                                    save
                                }
                            },
                            match mem.get_swap_string_index(name.clone()) {
                                Some(s) => s.clone()[if mem_check {
                                    ..s.len()
                                } else {
                                    if s.len() as i32 - 3 > 0 {
                                        ..s.len() - 3
                                    } else {
                                        ..0
                                    }
                                }]
                                .to_owned(),
                                None => String::default(),
                            },
                            width1 = if mem_check { 5 } else { 1 },
                            width2 = if mem_check { 5 } else { 1 },
                            width3 = if mem_check { 9 } else { 7 },
                        )
                        .as_str(),
                    );
                    cy += if self.get_graph_height() == 0 {
                        1
                    } else {
                        self.get_graph_height()
                    };
                }
            }
        }

        if self.get_graph_height() > 0 && cy != h {
            out.push_str(format!("{}{}", mv::to(y + cy, x + cx), gli).as_str());
        }

        // * Disks
        if CONFIG.show_disks && mem.get_disks().len() > 0 {
            cx = u32::try_from(x as i32 + self.mem_width as i32 - 1).unwrap_or(0);
            cy = 0;
            let mut big_disk: bool = self.get_disks_width() >= 25;
            let gli: String = format!(
                "{}{}{}{}{}{}{}",
                mv::left(2),
                THEME.colors.div_line,
                symbol::title_right,
                symbol::h_line.repeat(self.get_disks_width() as usize),
                THEME.colors.mem_box,
                symbol::title_left,
                mv::left(u32::try_from(self.get_disks_width() as i32 - 1).unwrap_or(0)),
            );

            for (name, item) in mem.get_disks() {
                if collector.get_collect_interrupt() {
                    return;
                }
                if !meters
                    .get()
                    .unwrap()
                    .try_lock()
                    .unwrap()
                    .get_disks_used()
                    .contains_key(&name)
                {
                    continue;
                }
                if cy > h - 2 {
                    break;
                }
                let item_s: String = item[&"total".to_owned()].to_string();
                let item_len: usize = item_s.len();
                let insert: String =
                    item_s[if big_disk { ..item_len } else { ..item_len - 3 }].to_owned();

                out.push_str(
                    Fx::trans(format!(
                        "{}{}{}{}{:width$.12}{}{:>9}",
                        mv::to(y + cy, x + cx),
                        gli,
                        THEME.colors.title,
                        fx::b,
                        item_s,
                        mv::to(
                            y + cy,
                            u32::try_from(
                                x as i32 + cx as i32 + self.get_disks_width() as i32 - 11
                            )
                            .unwrap_or(0)
                        ),
                        insert,
                        width = usize::try_from(self.get_disks_width() as i32 - 2).unwrap_or(0),
                    ))
                    .as_str(),
                );

                out.push_str(
                    format!(
                        "{}{}{}{}{}{}{}",
                        mv::to(
                            y + cy,
                            u32::try_from(
                                x as i32 + cx as i32 + (self.get_disks_width() / 2) as i32
                                    - (item[&"io".to_owned()].to_string().len() / 2) as i32
                                    - 2
                            )
                            .unwrap_or(0)
                        ),
                        fx::ub,
                        THEME.colors.main_fg,
                        item[&"io".to_owned()],
                        fx::ub,
                        THEME.colors.main_fg,
                        mv::to(y + cy + 1, x + cx),
                    )
                    .as_str(),
                );
                let inserter_used = item.get(&"used_percent".to_owned()).unwrap().to_string() + "%";
                out.push_str(
                    if big_disk {
                        format!("Used:{:>4}", inserter_used)
                    } else {
                        "U ".to_owned()
                    }
                    .as_str(),
                );

                let used: String = item[&"used".to_owned()].to_string();
                let used_len: usize = used.len();
                let insert: String =
                    used[..if big_disk { used_len } else { used_len - 3 }].to_owned();

                out.push_str(
                    format!(
                        "{}{:>width$}",
                        meters
                            .get()
                            .unwrap()
                            .try_lock()
                            .unwrap()
                            .get_disks_used_index(name.clone())
                            .unwrap_or(Meter::default()),
                        insert,
                        width = if big_disk { 9 } else { 7 },
                    )
                    .as_str(),
                );
                cy += 2;

                if mem.get_disks().len() as u32 * 3 <= h + 1 {
                    if cy > h - 1 {
                        break;
                    }
                    out.push_str(mv::to(y + cy, x + cx).as_str());
                    let inserter_free = item[&"free_percent".to_owned()].to_string() + "%";
                    out.push_str(
                        if big_disk {
                            format!("Free:{:>4} ", inserter_free)
                        } else {
                            "F ".to_owned()
                        }
                        .as_str(),
                    );
                    let free_s: String = item[&"free".to_owned()].to_string();
                    let free_len: usize = free_s.len();
                    let insert: String =
                        free_s[..if big_disk { free_len } else { free_len - 3 }].to_owned();
                    out.push_str(
                        format!(
                            "{}{:>width$}",
                            meters
                                .get()
                                .unwrap()
                                .try_lock()
                                .unwrap()
                                .get_disks_free_index(name.clone())
                                .unwrap_or(Meter::default()),
                            insert,
                            width = if big_disk { 9 } else { 7 }
                        )
                        .as_str(),
                    );
                    cy += 1;
                    if mem.get_disks().len() as u32 * 4 <= h + 1 {
                        cy += 1;
                    }
                }
            }
        }
        draw.buffer(
            self.get_buffer(),
            vec![format!("{}{}{}", out_misc, out, term.get_fg())],
            false,
            false,
            100,
            menu.active,
            false,
            false,
            key,
        );
        self.set_parent_resized(false);
        self.set_redraw(false);
    }

    pub fn get_parent(&self) -> BrshtopBox {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent: BrshtopBox) {
        self.parent = parent.clone()
    }

    pub fn set_parent_name(&mut self, name: String) {
        self.parent.set_name(name.clone())
    }

    pub fn set_parent_height_p(&mut self, height_p: u32) {
        self.parent.set_height_p(height_p.clone())
    }

    pub fn set_parent_width_p(&mut self, width_p: u32) {
        self.parent.set_width_p(width_p.clone())
    }

    pub fn set_parent_x(&mut self, x: u32) {
        self.parent.set_x(x.clone())
    }

    pub fn set_parent_y(&mut self, y: u32) {
        self.parent.set_y(y.clone())
    }

    pub fn set_parent_resized(&mut self, resized: bool) {
        self.parent.set_resized(resized.clone())
    }

    pub fn set_parent_width(&mut self, width: u32) {
        self.parent.set_width(width.clone())
    }

    pub fn set_parent_height(&mut self, height: u32) {
        self.parent.set_height(height.clone())
    }

    pub fn get_mem_meter(&self) -> i32 {
        self.mem_meter.clone()
    }

    pub fn set_mem_meter(&mut self, mem_meter: i32) {
        self.mem_meter = mem_meter.clone()
    }

    pub fn get_mem_size(&self) -> usize {
        self.mem_size.clone()
    }

    pub fn set_mem_size(&mut self, mem_size: usize) {
        self.mem_size = mem_size.clone()
    }

    pub fn get_disk_meter(&self) -> i32 {
        self.disk_meter.clone()
    }

    pub fn set_disk_meter(&mut self, disk_meter: i32) {
        self.disk_meter = disk_meter.clone()
    }

    pub fn get_divider(&self) -> i32 {
        self.divider.clone()
    }

    pub fn set_divider(&mut self, divider: i32) {
        self.divider = divider.clone()
    }

    pub fn get_mem_width(&self) -> u32 {
        self.mem_width.clone()
    }

    pub fn set_mem_width(&mut self, mem_width: u32) {
        self.mem_width = mem_width.clone()
    }

    pub fn get_disks_width(&self) -> u32 {
        self.disks_width.clone()
    }

    pub fn set_disks_width(&mut self, disks_width: u32) {
        self.disks_width = disks_width.clone()
    }

    pub fn get_graph_height(&self) -> u32 {
        self.graph_height.clone()
    }

    pub fn set_graph_height(&mut self, graph_height: u32) {
        self.graph_height = graph_height.clone()
    }

    pub fn get_redraw(&self) -> bool {
        self.redraw.clone()
    }

    pub fn set_redraw(&mut self, redraw: bool) {
        self.redraw = redraw.clone()
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer.clone()
    }

    pub fn get_swap_on(&self) -> bool {
        self.swap_on.clone()
    }

    pub fn set_swap_on(&mut self, swap_on: bool) {
        self.swap_on = swap_on.clone()
    }

    pub fn get_mem_names(&self) -> Vec<String> {
        self.mem_names.clone()
    }

    pub fn set_mem_names(&mut self, mem_names: Vec<String>) {
        self.mem_names = mem_names.clone()
    }

    pub fn get_mem_names_index(&self, index: usize) -> Option<String> {
        match self.get_mem_names().get(index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_mem_names_index(&mut self, index: usize, element: String) {
        self.get_mem_names().insert(index, element.clone())
    }

    pub fn get_swap_names(&self) -> Vec<String> {
        self.swap_names.clone()
    }

    pub fn set_swap_names(&mut self, swap_names: Vec<String>) {
        self.swap_names = swap_names.clone()
    }

    pub fn get_swap_names_index(&self, index: usize) -> Option<String> {
        match self.get_swap_names().get(index) {
            Some(s) => Some(s.to_owned().clone()),
            None => None,
        }
    }

    pub fn set_swap_names_index(&mut self, index: usize, element: String) {
        self.get_mem_names().insert(index, element.clone())
    }
}

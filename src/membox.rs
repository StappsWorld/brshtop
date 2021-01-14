use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::Collector,
        config::{
            Config, 
            ViewMode
        },
        create_box, 
        draw::Draw,
        fx,
        fx::Fx,
        graph::{Graph, Graphs},
        key::Key,
        memcollector::MemCollector,
        meter::{Meter, MeterUnion, Meters},
        mv, symbol,
        term::Term,
        theme::{
            Theme,
            Color
        },
    },
    math::round::ceil,
    inflector::Inflector,
    std::collections::HashMap,
};

pub struct MemBox {
    pub parent: BrshtopBox,
    pub name: String,
    pub height_p: u32,
    pub width_p: u32,
    pub x: i32,
    pub y: i32,
    pub mem_meter: i32,
    pub mem_size: usize,
    pub disk_meter: i32,
    pub divider: i32,
    pub mem_width: u32,
    pub disks_width: i32,
    pub graph_height: u32,
    pub redraw: bool,
    pub buffer: String,
    pub swap_on: bool,
    pub mem_names: Vec<String>,
    pub swap_names: Vec<String>,
}
impl MemBox {
    pub fn new(brshtop_box: &mut BrshtopBox, CONFIG: &mut Config, ARG_MODE: ViewMode) -> Self {
        let membox = MemBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
            name: "mem".to_owned(),
            height_p: 38,
            width_p: 45,
            x: 1,
            y: 1,
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
        brshtop_box.buffers.push(membox.buffer.clone());
        membox.parent.resized = true;
        membox
    }

    pub fn calc_size(
        &mut self,
        term: &mut Term,
        brshtop_box: &mut BrshtopBox,
        CONFIG: &mut Config,
    ) {
        let mut width_p: u32 = 0;
        let mut height_p: u32 = 0;

        if self.parent.stat_mode {
            width_p = 100;
            height_p = self.height_p;
        } else {
            width_p = self.width_p;
            height_p = self.height_p;
        }
        self.width = term.width as u32 * width_p / 100;
        self.height = (term.height as u32 * height_p / 100) + 1;
        brshtop_box._b_mem_h = self.parent.height;
        self.y = brshtop_box._b_cpu_h + 1;
        if CONFIG.show_disks {
            self.mem_width = ceil((self.parent.width - 3) as f64 / 2.0, 0);
            self.disk_width = self.parent.width - self.mem_width - 3;
            if self.mem_width as i32 + self.disks_width < self.parent.width as i32 - 2 {
                self.mem_width += 1;
            }
            self.divider = self.x + self.mem_width as i32;
        } else {
            self.mem_width = self.parent.width - 1;
        }

        let mut item_height: u32 = if self.swap_on && !CONFIG.swap_disk {
            6
        } else {
            4
        };
        self.mem_width = if self.parent.height
            - if self.swap_on && !CONFIG.swap_disk {
                3
            } else {
                2
            }
            > 2 * item_height
        {
            3
        } else if self.mem_width > 25 {
            2
        } else {
            1
        };

        self.mem_meter = self.parent.width as i32
            - if CONFIG.show_disks {
                self.disks_width
            } else {
                0
            }
            - if self.mem_size > 2 { 9 } else { 20 };

        if self.mem_size == 1 {
            self.mem_meter += 6;
        }
        if self.mem_meter < 1 {
            self.mem_meter = 0;
        }

        if CONFIG.mem_graphs {
            self.graph_height = ((self.parent.height
                - if self.swap_on && !CONFIG.swap_disk {
                    2
                } else {
                    1
                })
                - if self.mem_size == 3 { 2 } else { 1 } * item_height)
                / item_height;
            if self.graph_height == 0 {
                self.graph_height = 1;
            }
            if self.graph_height > 1 {
                self.mem_meter += 6;
            }
        } else {
            self.graph_height = 0;
        }

        if CONFIG.show_disks {
            self.disk_meter = self.parent.width - self.mem_width - 23;
            if self.disks_width < 25 {
                self.disk_meter += 10;
            }
            if self.disk_meter < 1 {
                self.disk_meter = 0;
            }
        }
    }

    pub fn draw_bg(&mut self, THEME: &mut Theme, CONFIG: &mut Config, term: &Term) -> String {
        if self.parent.proc_mode {
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
                    Boxes::MemBox(self),
                    term,
                    THEME,
                )
                .as_str(),
            );
            if CONFIG.show_disks {
                let mut adder: String = String::default();
                for i in 1..self.parent.height - 1 {
                    adder.push_str(
                        format!(
                            "{}{}",
                            mv::to(self.y as u32 + i, self.divider as u32),
                            symbol::v_line
                        )
                        .as_str(),
                    );
                }

                out.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}{}{}{}{}",
                        mv::to(self.y as u32, self.divider as u32 + 2),
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
                        mv::to(self.y as u32, self.divider as u32),
                        THEME.colors.mem_box.call(symbol::div_up.to_owned(), term),
                        mv::to(self.y as u32 + self.height - 1, self.divider),
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
        mem: &mut MemCollector,
        term: &mut Term,
        brshtop_box: &mut BrshtopBox,
        CONFIG: &mut Config,
        meters: &mut Meters,
        THEME: &mut Theme,
        key : &mut Key,
        collector : &mut Collector,
        draw : &mut Draw,
    ) {
        if self.parent.proc_mode {
            return;
        }

        if mem.redraw {
            self.redraw = true;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut gbg: String = String::default();
        let mut gmv: String = String::default();
        let mut gli: String = String::default();

        let mut x = self.x + 1;
        let mut y = self.y + 1;
        let mut w = self.width - 2;
        let mut h = self.height - 2;

        if self.parent.resized || self.redraw {
            self.calc_size(term, brshtop_box, CONFIG);
            out_misc.push_str(self.draw_bg(THEME, CONFIG, term).as_str());
            meters.mem = HashMap::<String, MeterUnion>::new();
            meters.swap = HashMap::<String, MeterUnion>::new();
            meters.disks_used = HashMap::<String, Meter>::new();
            meters.disks_free = HashMap::<String, Meter>::new();
            if self.mem_meter > 0 {
                for name in self.mem_names {
                    if CONFIG.mem_graphs {
                        meters.mem[name] = MeterUnion::Graph(Graph::new(
                            self.mem_meter,
                            self.graph_height as i32,
                            THEME.gradient[name],
                            mem.vlist[name],
                            term,
                            false,
                            0,
                            0,
                            None,
                        ));
                    } else {
                        meters.mem[name] = MeterUnion::Meter(Meter::new(
                            mem.percent[name],
                            self.mem_meter as u32,
                            name.clone(),
                            false,
                            THEME,
                            term,
                        ));
                    }
                }
                if self.swap_on {
                    for name in self.swap_names {
                        if CONFIG.mem_graphs && !CONFIG.swap_disk {
                            meters.swap[name] = MeterUnion::Graph(Graph::new(self.mem_meter, self.graph_height as i32, Some(Color::new(THEME.gradient[name])), mem.vlist[name], term, false, 0, 0, None));
                        } else if CONFIG.swap_disk && CONFIG.show_disks {
                            meters.disks_used["__swap".to_owned()] = MeterUnion::Meter(Meter::new(mem.swap_percent["used".to_owned()], self.disk_meter as u32, "used".to_owned(), false, THEME, term));
                            if mem.disks.len() * 3 <= h + 1 {
                                meters.disks_free["__swap"] = MeterUnion::Meter(Meter::new(mem.swap_percent["free".to_owned()], self.mem_meter as u32, "free".to_owned(), false, THEME, term));
                            }
                            break;
                        } else {
                            meters.swap[name] = MeterUnion::Meter(Meter::new(mem.swap_percent[name], self.mem_meter as u32, name, false, THEME, term));
                        }
                    }
                }
            }
            if self.disk_meter > 0 {
                for (n, name) in mem.disks.keys().enumerate() {
                    if n * 2 > h {
                        break;
                    }
                    meters.disks_used[name] = MeterUnion::Meter(Meter::new(mem.disks[name]["used_percent".to_owned()], self.disk_meter as u32, "used".to_owned(), false, THEME, term));
                    if mem.disks.len() * 3 <= h + 1 {
                        meters.disks_free[name] = MeterUnion::Meter(Meter::new(mem.disks[name]["free_percent".as_bytes()], self.disk_meter as u32, "free".to_owned(), false, THEME, term));
                    }
                }
            }
            if !key.mouse.contains_key(&"g".to_owned()) {
                let mut top = Vec::<Vec<i32>>::new();
                for i in 0..5 {
                    let mut adder : Vec<i32> = Vec::<i32>::new();
                    adder.push(x + self.mem_width as i32 - 8);
                    adder.push(y - 1);
                    top.push(adder);
                }
                key.mouse.insert("g".to_owned(), top);
            }
            out_misc.push_str(format!("{}{}{}{}{}{}{}",
                    mv::to(y as u32 - 1, (x + w - 7) as u32),
                    THEME.colors.mem_box.call(symbol::title_left.to_owned(), term),
                    if CONFIG.mem_graphs {
                        fx::b
                    } else {
                        ""
                    },
                    THEME.colors.hi_fg.call("g".to_owned(), term),
                    THEME.colors.title("wap".to_owned(), term),
                    fx::ub,
                    THEME.colors.mem_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );
            if CONFIG.show_disks {
                if !key.mouse.contains_key(&"s") {
                    let mut top : Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
                    for i in 0..4 {
                        let mut adder : Vec<i32> = Vec::<i32>::new();
                        adder.push(x + w - 6 + i);
                        adder.push(y - 1);
                        top.push(adder);
                    }
                    key.mouse.insert("s", top);
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}",
                        mv::to(y as u32 - 1, (x + w - 7) as u32),
                        THEME.colors.mem_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.swap_disk {
                            fx::b
                        } else {
                            ""
                        },
                        THEME.colors.hi_fg.call("s".to_owned(), term),
                        THEME.colors.title.call("raph".to_owned(), term),
                        fx::ub,
                        THEME.colors.mem_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }
            if collector.collect_interrupt {
                return;
            }
            draw.buffer("mem_misc".to_owned(), vec![out_misc.clone()], false, false, 100, true, false, false, key);
        }
        let mut cx : u32 = 1;
        let mut cy : u32 = 1;

        out.push_str(format!("{}{}{}Total:{:>width$}{}{}",
                mv::to(y, x + 1),
                THEME.colors.title,
                fx::b,
                mem.string["total".to_owned()],
                fx::ub,
                THEME.colors.main_fg,
                width = self.mem_width - 9,
            )
            .as_str()
        );
        if self.graph_height > 0 {
            gli = format!("{}{}{}{}{}{}{}{}",
                mv::left(2),
                THEME.colors.mem_box.call(symbol::title_right.to_owned(), term),
                THEME.colors.div_line,
                symbol::h_line.repeat(self.mem_width - 1),
                if CONFIG.show_disks {
                    "".to_owned()
                } else {
                    THEME.colors.mem_box.to_string()
                },
                symbol::title_left,
                mv::l(self.mem_width - 1),
                THEME.colors.title,
            );
            if self.graph_height >= 2 {
                gbg = mv::left(1);
                gmv = format!("{}{}", mv::left(self.mem_width - 2), mv::up(self.graph_height - 1));
            }
        }

        let big_mem : bool = false;
        for name in self.mem_names {
            if collector.collect_interrupt {
                return;
            }
            if self.mem_size > 2 {
                out.push_str(format!("{}{}{:<width$}{}{}{}{}{}{}{:>4}",
                        mv::to(y + cy, x + cx),
                        gli,
                        name.to_title_case()[if big_mem {
                            
                        } else {
                            ..5
                        }] + ":",
                        mv::to(y + cy, x + cx + self.mem_width - 3 - mem.string[name].len() as u32),
                        Fx::trans(mem.string[name]),
                        mv::to(y + cy + 1, x + cx),
                        gbg,
                        match meters.mem[name] {
                            MeterUnion::Meter(m) => m.call(if self.parent.resized {
                                None
                            } else {
                                Some(mem.percent[name])
                            }, term),
                            MeterUnion::Graph(g) => g.call(if self.parent.resized {
                                None
                            } else {
                                Some(mem.percent[name])
                            }, term),
                        },
                        gmv,
                        mem.percent[name].to_string() + "%",
                        width = if big_mem {
                            1.0
                        } else {
                            6.6
                        }
                    )
                    .as_str()
                );
                cy += if self.graph_height == 0 {
                    2
                } else {
                    self.graph_height + 1
                };
            } else {
                out.push_str( format!("{}{:width$} {}{}{:width2$}",
                        mv::to(y + cy, x + cx),
                        name.to_title_case()[if big_mem {

                        } else {
                            ..5
                        }] + ":",
                        
                    )
                    .as_str()
                );
            }
        }
        
    }
}

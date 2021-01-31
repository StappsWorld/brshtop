use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        config::{Config, ViewMode},
        create_box,
        draw::Draw,
        error::errlog,
        fx,
        graph::{Graph, Graphs},
        key::Key,
        menu::Menu,
        mv,
        netcollector::{NetCollector, NetCollectorStat},
        subbox::SubBox,
        symbol,
        term::Term,
        theme::{Color, Theme},
    },
    once_cell::sync::OnceCell,
    std::{collections::HashMap, convert::TryFrom, sync::Mutex},
};

pub struct NetBox {
    parent: BrshtopBox,
    sub: SubBox,
    redraw: bool,
    graph_height: HashMap<String, u32>,
    symbols: HashMap<String, String>,
    buffer: String,
}
impl NetBox {
    pub fn new(
        CONFIG: &OnceCell<Mutex<Config>>,
        ARG_MODE: ViewMode,
        brshtop_box: &OnceCell<Mutex<BrshtopBox>>,
    ) -> Self {
        let net = NetBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
            sub: SubBox::new(),
            redraw: true,
            graph_height: HashMap::<String, u32>::new(),
            symbols: [("download", "▼"), ("upload", "▲")]
                .iter()
                .map(|(s1, s2)| (s1.to_owned().to_owned(), s2.to_owned().to_owned()))
                .collect(),
            buffer: "net".to_owned(),
        };

        brshtop_box
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .push_buffers(net.buffer.clone());

        net.set_parent_name("net".to_owned());
        net.set_parent_height_p(30);
        net.set_parent_width_p(45);
        net.set_parent_x(1);
        net.set_parent_y(1);
        net.set_parent_resized(true);
        net
    }

    pub fn calc_size(&mut self, term: &OnceCell<Mutex<Term>>, b_cpu_h: i32, b_mem_h: i32) {
        let mut width_p: u32 = 0;

        if self.get_parent().get_stat_mode() {
            width_p = 100;
        } else {
            width_p = self.get_parent().get_width_p();
        }
        self.set_parent_width(
            ((term.get().unwrap().lock().unwrap().get_width() as u32) * width_p / 100) as u32,
        );
        self.set_parent_height(
            u32::try_from(
                term.get().unwrap().lock().unwrap().get_height() as i32 - b_cpu_h - b_mem_h,
            )
            .unwrap_or(0),
        );
        self.set_parent_y(
            u32::try_from(
                (term.get().unwrap().lock().unwrap().get_height() as i32)
                    - self.parent.get_height() as i32
                    + 1,
            )
            .unwrap_or(0),
        );
        self.set_sub_box_width(if self.parent.get_width() > 45 { 27 } else { 19 });
        self.set_sub_box_height(if self.parent.get_height() > 10 {
            9
        } else {
            u32::try_from(self.parent.get_height() as i32 - 2).unwrap_or(0)
        });
        self.set_sub_box_x(
            u32::try_from(self.parent.get_width() as i32 - self.sub.get_box_width() as i32 - 1)
                .unwrap_or(0),
        );
        self.set_sub_box_y(
            self.get_parent().get_y()
                + u32::try_from(
                    ((self.parent.get_height() as i32 - 2) / 2)
                        - (self.sub.get_box_height() / 2) as i32,
                )
                .unwrap_or(0)
                + 1,
        );
        self.set_graph_height_index(
            "download".to_owned(),
            ((self.parent.get_height() as i32 - 2) as f64 / 2.0).round() as u32,
        );
        self.set_graph_height_index(
            "upload".to_owned(),
            u32::try_from(
                self.parent.get_height() as i32
                    - 2
                    - self
                        .get_graph_height_index("download".to_owned())
                        .unwrap_or(0) as i32,
            )
            .unwrap_or(0),
        );
        self.set_redraw(true);
    }

    pub fn draw_bg(
        &self,
        theme: &Theme,
        term: &OnceCell<Mutex<Term>>,
        passable_self: &OnceCell<Mutex<NetBox>>,
    ) -> String {
        if self.parent.get_proc_mode() {
            return String::default();
        }
        format!(
            "{}{}",
            create_box(
                0,
                0,
                0,
                0,
                None,
                None,
                Some(theme.colors.net_box),
                None,
                true,
                Some(Boxes::NetBox),
                term,
                theme,
                None,
                None,
                None,
                Some(passable_self),
                None,
            ),
            create_box(
                self.sub.get_box_x(),
                self.sub.get_box_y(),
                self.sub.get_box_width(),
                self.sub.get_box_height(),
                Some("Download".to_owned()),
                Some("Upload".to_owned()),
                Some(theme.colors.div_line),
                None,
                false,
                None,
                term,
                theme,
                None,
                None,
                None,
                Some(passable_self),
                None,
            )
        )
    }

    pub fn draw_fg(
        &mut self,
        theme: &Theme,
        key: &OnceCell<Mutex<Key>>,
        term: &OnceCell<Mutex<Term>>,
        CONFIG: &OnceCell<Mutex<Config>>,
        draw: &OnceCell<Mutex<Draw>>,
        graphs: &OnceCell<Mutex<Graphs>>,
        menu: &OnceCell<Mutex<Menu>>,
        net: &OnceCell<Mutex<NetCollector>>,
        passable_self: &OnceCell<Mutex<NetBox>>,
    ) {
        if self.get_parent().get_proc_mode() {
            return;
        }

        if net.get().unwrap().lock().unwrap().parent.get_redraw() {
            self.redraw = true;
        }
        if net.get().unwrap().lock().unwrap().nic.is_none() {
            return;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let x: u32 = self.get_parent().get_x() + 1;
        let y: u32 = self.get_parent().get_y() + 1;
        let w: u32 = u32::try_from(self.get_parent().get_width() as i32 - 2).unwrap_or(0);
        let h: u32 = u32::try_from(self.get_parent().get_height() as i32 - 2).unwrap_or(0);
        let bx: u32 = self.get_sub().get_box_x() + 1;
        let by: u32 = self.get_sub().get_box_y() + 1;
        let bw: u32 = u32::try_from(self.get_sub().get_box_width() as i32 - 2).unwrap_or(0);
        let bh: u32 = u32::try_from(self.get_sub().get_box_height() as i32 - 2).unwrap_or(0);
        let nic_name: String = net
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .nic
            .unwrap()
            .name()
            .to_owned();
        let reset: bool = match net.get().unwrap().lock().unwrap().stats[&nic_name]
            [&"download".to_owned()][&"offset".to_owned()]
        {
            NetCollectorStat::Bool(b) => b,
            NetCollectorStat::I32(i) => i > 0,
            NetCollectorStat::U64(u) => u > 0,
            NetCollectorStat::Vec(v) => v.len() > 0,
            NetCollectorStat::String(s) => {
                errlog(format!(
                    "Malformed type in net.get().unwrap().lock().unwrap().stats[{}]['download']['offset']",
                    nic_name
                ));
                s.parse::<i64>().unwrap_or(0) > 0
            }
        };

        if self.get_parent().get_resized() || self.get_redraw() {
            out_misc.push_str(self.draw_bg(theme, term, passable_self).as_str());
            if key
                .get()
                .unwrap()
                .lock()
                .unwrap()
                .mouse
                .contains_key(&"b".to_owned())
            {
                let mut b_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut b_insert: Vec<i32> = Vec::<i32>::new();
                    b_insert.push(x as i32 + w as i32 - nic_name[..10].len() as i32 - 9 + i);
                    b_insert.push(y as i32 - 1);
                    b_vec_top.push(b_insert);
                }

                key.get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .insert("b".to_owned(), b_vec_top);

                let mut n_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut n_insert: Vec<i32> = Vec::<i32>::new();
                    n_insert.push(x as i32 + w as i32 - 5 + i);
                    n_insert.push(y as i32 - 1);
                    n_vec_top.push(n_insert);
                }

                key.get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .insert("n".to_owned(), n_vec_top);

                let mut z_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut z_insert: Vec<i32> = Vec::<i32>::new();
                    z_insert.push(x as i32 + w as i32 - nic_name[..10].len() as i32 - 14 + i);
                    z_insert.push(y as i32 - 1);
                    z_vec_top.push(z_insert);
                }

                key.get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .insert("z".to_owned(), z_vec_top);
            }
            out_misc.push_str(
                format!(
                    "{}{}{}{}{}{}{}{}{}{}{}{}{}{} {} {}{}{}{}",
                    mv::to(y as u32 - 1, x as u32 + w - 25),
                    theme.colors.net_box,
                    symbol::h_line.repeat(10 - nic_name[..10].len()),
                    symbol::title_left,
                    if reset { fx::bold } else { "" },
                    theme.colors.hi_fg.call("z".to_owned(), term),
                    theme.colors.title.call("ero".to_owned(), term),
                    fx::ub,
                    theme
                        .colors
                        .net_box
                        .call(symbol::title_right.to_owned(), term),
                    term.get().unwrap().lock().unwrap().get_fg(),
                    theme.colors.net_box,
                    symbol::title_left,
                    fx::b,
                    theme.colors.hi_fg.call("<b".to_owned(), term),
                    theme.colors.title.call(nic_name[..10].to_owned(), term),
                    theme.colors.hi_fg.call("n>".to_owned(), term),
                    fx::ub,
                    theme
                        .colors
                        .net_box
                        .call(symbol::title_right.to_owned(), term),
                    term.get().unwrap().lock().unwrap().get_fg(),
                )
                .as_str(),
            );

            if (w as usize) - nic_name[..10].len() - 20 > 6 {
                if !key
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .contains_key(&"a".to_owned())
                {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();

                        inserter.push(x as i32 + w as i32 - 20 - nic_name[..10].len() as i32 + i);
                        inserter.push(y as i32 - 1);
                        inserter_top.push(inserter);
                    }
                    key.get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .mouse
                        .insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}",
                        mv::to(
                            u32::try_from(y as i32 - 1).unwrap_or(0),
                            u32::try_from(x as i32 + w as i32 - 21 - nic_name[..10].len() as i32)
                                .unwrap_or(0)
                        ),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if net.get().unwrap().lock().unwrap().auto_min {
                            fx::b
                        } else {
                            ""
                        },
                        theme.colors.hi_fg.call("a".to_owned(), term),
                        theme.colors.title.call("uto".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.get().unwrap().lock().unwrap().get_fg(),
                    )
                    .as_str(),
                );
            }
            if w as i32 - nic_name[..10].len() as i32 - 20 > 6 {
                if !key
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .contains_key(&"a".to_owned())
                {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();
                        inserter.push(x as i32 + w as i32 - 20 - nic_name[..10].len() as i32 + i);
                        inserter.push(y as i32 - 1);
                        inserter_top.push(inserter);
                    }
                    key.get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .mouse
                        .insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}",
                        mv::to(
                            u32::try_from(y as i32 - 1).unwrap_or(0),
                            u32::try_from(x as i32 + w as i32 - 21 - nic_name[..10].len() as i32)
                                .unwrap_or(0)
                        ),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if net.get().unwrap().lock().unwrap().auto_min {
                            fx::b
                        } else {
                            ""
                        },
                        theme.colors.hi_fg.call("a".to_owned(), term),
                        theme.colors.title.call("auto".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.get().unwrap().lock().unwrap().get_fg(),
                    )
                    .as_str(),
                );
            }

            if w - nic_name[..10].len() as u32 - 20 > 13 {
                if !key
                    .get()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .mouse
                    .contains_key(&"y".to_owned())
                {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();
                        inserter.push(x as i32 + w as i32 - 26 - nic_name[..10].len() as i32 + i);
                        inserter.push(y as i32 - 1);
                        inserter_top.push(inserter);
                    }
                    key.get()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .mouse
                        .insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}{}",
                        mv::to(
                            u32::try_from(y as i32 - 1).unwrap_or(0),
                            u32::try_from(x as i32 + w as i32 - 27 - nic_name[..10].len() as i32)
                                .unwrap_or(0)
                        ),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if CONFIG.get().unwrap().lock().unwrap().net_sync {
                            fx::b
                        } else {
                            ""
                        },
                        theme.colors.title.call("s".to_owned(), term),
                        theme.colors.hi_fg.call("y".to_owned(), term),
                        theme.colors.title.call("nc".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.get().unwrap().lock().unwrap().get_fg(),
                    )
                    .as_str(),
                );
            }

            draw.get().unwrap().lock().unwrap().buffer(
                "net_misc".to_owned(),
                vec![out_misc],
                false,
                false,
                100,
                true,
                false,
                false,
                key,
            );
        }

        let mut cy = 0;

        for direction in ["download", "upload"]
            .iter()
            .map(|s| s.to_owned().to_owned())
            .collect::<Vec<String>>()
        {
            let mut strings =
                net.get().unwrap().lock().unwrap().strings[&nic_name][&direction].clone();
            let mut stats = net.get().unwrap().lock().unwrap().stats[&nic_name][&direction].clone();

            if self.get_redraw() {
                stats[&"redraw".to_owned()] = NetCollectorStat::Bool(true);
            }
            if match stats["redraw"] {
                NetCollectorStat::Bool(b) => b,
                NetCollectorStat::I32(i) => i > 0,
                NetCollectorStat::Vec(v) => v.len() > 0,
                NetCollectorStat::U64(u) => u > 0,
                NetCollectorStat::String(s) => {
                    errlog("Malformed type in stats['redraw']".to_owned());
                    s.parse::<i32>().unwrap_or(0) > 0
                }
            } || self.get_parent().get_resized()
            {
                graphs.get().unwrap().lock().unwrap().net[&direction] = Graph::new_with_vec::<Color>(
                    w - bw - 3,
                    self.graph_height[&direction],
                    theme.gradient[&direction],
                    match stats[&"speed".to_owned()] {
                        NetCollectorStat::Vec(v) => v.iter().map(|u| u.to_owned() as i32).collect(),
                        _ => vec![],
                    },
                    term,
                    direction != "download".to_owned(),
                    if CONFIG.get().unwrap().lock().unwrap().net_sync {
                        net.get().unwrap().lock().unwrap().sync_top
                    } else {
                        match stats[&"graph_top".to_owned()] {
                            NetCollectorStat::Bool(b) => {
                                if b {
                                    1
                                } else {
                                    0
                                }
                            }
                            NetCollectorStat::I32(i) => i,
                            NetCollectorStat::Vec(v) => 0,
                            NetCollectorStat::U64(u) => u as i32,
                            NetCollectorStat::String(s) => {
                                errlog("Malformed type in stats['graph_top']".to_owned());
                                s.parse::<i32>().unwrap_or(0)
                            }
                        }
                    },
                    0,
                    if CONFIG.get().unwrap().lock().unwrap().net_color_fixed {
                        Some(net.get().unwrap().lock().unwrap().net_min[&direction])
                    } else {
                        None
                    },
                );
            }

            out.push_str(
                format!(
                    "{}{}",
                    mv::to(
                        if direction == "download".to_owned() {
                            y
                        } else {
                            y + self
                                .get_graph_height_index("download".to_owned())
                                .unwrap_or(0)
                        },
                        x as u32
                    ),
                    graphs.get().unwrap().lock().unwrap().net[&direction].call(
                        if match stats[&"redraw".to_owned()] {
                            NetCollectorStat::Bool(b) => b,
                            NetCollectorStat::I32(i) => i > 0,
                            NetCollectorStat::Vec(v) => v.len() > 0,
                            NetCollectorStat::U64(u) => u > 0,
                            NetCollectorStat::String(s) => {
                                errlog("Malformed type in stats['redraw']".to_owned());
                                s.parse::<i32>().unwrap_or(0) > 0
                            }
                        } {
                            None
                        } else {
                            Some(match stats[&"speed".to_owned()] {
                                NetCollectorStat::Vec(v) => v[v.len() - 2] as i32,
                                _ => 0,
                            })
                        },
                        term
                    ),
                )
                .as_str(),
            );

            out.push_str(
                format!(
                    "{}{}{} {:<10.10}{}",
                    mv::to(by + cy, bx),
                    theme.colors.main_fg,
                    self.symbols[&direction],
                    strings[&"byte_ps".to_owned()],
                    if bw < 20 {
                        "".to_owned()
                    } else {
                        format!(
                            "{}{:>12.12}",
                            mv::to(by + cy, bx + bw - 12),
                            "(".to_owned() + strings[&"bit_ps".to_owned()].as_str() + ")",
                        )
                    },
                )
                .as_str(),
            );

            cy += if bh != 3 { 1 } else { 2 };

            if bh >= 6 {
                out.push_str(
                    format!(
                        "{}{} Top:{}{:>12.12}",
                        mv::to(by + cy, bx),
                        self.get_symbols_index(direction.clone())
                            .unwrap_or(String::default()),
                        mv::to(
                            by + cy,
                            u32::try_from(bx as i32 + bw as i32 - 12).unwrap_or(0)
                        ),
                        "(".to_owned() + strings[&"top".to_owned()].as_str() + ")",
                    )
                    .as_str(),
                );
                cy += 1;
            }
            if bh >= 4 {
                out.push_str(
                    format!(
                        "{}{} Total:{}{:>10.10}",
                        mv::to(by + cy, bx),
                        self.symbols[&direction],
                        mv::to(by + cy, bx + bw - 10),
                        strings[&"total".to_owned()],
                    )
                    .as_str(),
                );
                if bh > 2 && bh % 2 != 0 {
                    cy += 2;
                } else {
                    cy += 1;
                }
            }
            stats["redraw"] = NetCollectorStat::Bool(false);
            net.get().unwrap().lock().unwrap().strings[&nic_name][&direction] = strings;
            net.get().unwrap().lock().unwrap().stats[&nic_name][&direction] = stats;
        }

        out.push_str(
            format!(
                "{}{}{}{}",
                mv::to(y, x),
                theme.colors.graph_text.call(
                    if CONFIG.get().unwrap().lock().unwrap().net_sync {
                        net.get().unwrap().lock().unwrap().sync_string
                    } else {
                        net.get().unwrap().lock().unwrap().strings[&nic_name]
                            [&"download".to_owned()][&"graph_top".to_owned()]
                    },
                    term
                ),
                mv::to(u32::try_from(y as i32 + h as i32 - 1).unwrap_or(0), x),
                theme.colors.graph_text.call(
                    if CONFIG.get().unwrap().lock().unwrap().net_sync {
                        net.get().unwrap().lock().unwrap().sync_string
                    } else {
                        net.get().unwrap().lock().unwrap().strings[&nic_name][&"upload".to_owned()]
                            [&"graph_top".to_owned()]
                    },
                    term
                ),
            )
            .as_str(),
        );

        draw.get().unwrap().lock().unwrap().buffer(
            self.get_buffer(),
            vec![format!(
                "{}{}{}",
                out_misc,
                out,
                term.get().unwrap().lock().unwrap().get_fg()
            )],
            false,
            false,
            100,
            menu.get().unwrap().lock().unwrap().active,
            false,
            false,
            key,
        );

        self.set_redraw(false);
        self.set_parent_resized(false);
    }

    pub fn get_parent(&self) -> BrshtopBox {
        self.parent.clone()
    }

    pub fn set_parent(&self, parent: BrshtopBox) {
        self.parent = parent.clone()
    }

    pub fn set_parent_name(&mut self, name: String) {
        self.parent.set_name(name.clone())
    }

    pub fn set_parent_width(&mut self, width: u32) {
        self.parent.set_width(width.clone())
    }

    pub fn set_parent_height(&mut self, height: u32) {
        self.parent.set_height(height.clone())
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

    pub fn get_sub(&self) -> SubBox {
        self.sub.clone()
    }

    pub fn set_sub(&mut self, sub: SubBox) {
        self.sub = sub.clone()
    }

    pub fn set_sub_box_width(&mut self, box_width: u32) {
        self.sub.set_box_width(box_width.clone())
    }

    pub fn set_sub_box_height(&mut self, box_height: u32) {
        self.sub.set_box_height(box_height.clone())
    }

    pub fn set_sub_box_x(&mut self, box_x: u32) {
        self.sub.set_box_x(box_x.clone())
    }

    pub fn set_sub_box_y(&mut self, box_y: u32) {
        self.sub.set_box_y(box_y.clone())
    }

    pub fn get_redraw(&self) -> bool {
        self.redraw.clone()
    }

    pub fn set_redraw(&mut self, redraw: bool) {
        self.redraw = redraw.clone()
    }

    pub fn get_graph_height(&self) -> HashMap<String, u32> {
        self.graph_height.clone()
    }

    pub fn set_graph_height(&mut self, graph_height: HashMap<String, u32>) {
        self.graph_height = graph_height.clone()
    }

    pub fn get_graph_height_index(&self, index: String) -> Option<u32> {
        match self.get_graph_height().get(&index.clone()) {
            Some(u) => Some(u.to_owned()),
            None => None,
        }
    }

    pub fn set_graph_height_index(&mut self, index: String, element: u32) {
        self.graph_height.insert(index.clone(), element.clone());
    }

    pub fn get_symbols(&self) -> HashMap<String, String> {
        self.symbols.clone()
    }

    pub fn set_symbols(&mut self, symbols: HashMap<String, String>) {
        self.symbols = symbols.clone()
    }

    pub fn get_symbols_index(&self, index: String) -> Option<String> {
        match self.get_symbols().get(&index.clone()) {
            Some(s) => Some(s.to_owned()),
            None => None,
        }
    }

    pub fn set_symbols_index(&mut self, index: String, element: String) {
        self.symbols.insert(index.clone(), element.clone());
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer.clone()
    }
}

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
    std::{collections::HashMap, convert::TryFrom},
};

pub struct NetBox {
    pub parent: BrshtopBox,
    pub sub: SubBox,
    pub name: String,
    pub height_p: u32,
    pub width_p: u32,
    pub x: i32,
    pub y: i32,
    pub resized: bool,
    pub redraw: bool,
    pub graph_height: HashMap<String, u32>,
    pub symbols: HashMap<String, String>,
    pub buffer: String,
}
impl NetBox {
    pub fn new(CONFIG: &mut Config, ARG_MODE: ViewMode, brshtop_box: &mut BrshtopBox) -> Self {
        let net = NetBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
            sub: SubBox::new(),
            name: "net".to_owned(),
            height_p: 30,
            width_p: 45,
            x: 1,
            y: 1,
            resized: true,
            redraw: true,
            graph_height: HashMap::<String, u32>::new(),
            symbols: [("download", "▼"), ("upload", "▲")]
                .iter()
                .map(|(s1, s2)| (s1.to_owned().to_owned(), s2.to_owned().to_owned()))
                .collect(),
            buffer: "net".to_owned(),
        };

        brshtop_box.buffers.push(net.buffer);

        net
    }

    pub fn calc_size(&mut self, term: &mut Term, brshtop_box: &mut BrshtopBox) {
        let mut width_p: u32 = 0;

        if self.parent.stat_mode {
            width_p = 100;
        } else {
            width_p = self.width_p;
        }
        self.parent.width = ((term.width as u32) * width_p / 100) as u32;
        self.parent.height =
            u32::try_from(term.height as i32 - brshtop_box._b_cpu_h - brshtop_box._b_mem_h)
                .unwrap_or(0);
        self.y = ((term.height as u32) - self.parent.height + 1) as i32;
        self.sub.box_width = if self.parent.width > 45 { 27 } else { 19 };
        self.sub.box_height = if self.parent.height > 10 {
            9
        } else {
            self.parent.height - 2
        };
        self.sub.box_x = self.parent.width - self.sub.box_width - 1;
        self.sub.box_y = self.y as u32 + ((self.parent.height - 2) / 2) as u32
            - (self.sub.box_height / 2) as u32
            + 1;
        self.graph_height.insert(
            "download".to_owned(),
            ((self.parent.height - 2) as f64 / 2.0).round() as u32,
        );
        self.graph_height.insert(
            "upload".to_owned(),
            self.parent.height - 2 - self.graph_height[&"download".to_owned()],
        );
        self.redraw = true;
    }

    pub fn draw_bg(&mut self, theme: &mut Theme, term : &mut Term) -> String {
        if self.parent.proc_mode {
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
                Some(Boxes::NetBox(self)),
                term,
                theme
            ),
            create_box(
                self.sub.box_x,
                self.sub.box_y,
                self.sub.box_width,
                self.sub.box_height,
                Some("Download".to_owned()),
                Some("Upload".to_owned()),
                Some(theme.colors.div_line),
                None,
                false,
                None,
                term,
                theme,
            )
        )
    }

    pub fn draw_fg(
        &mut self,
        theme: &mut Theme,
        key: &mut Key,
        term: &mut Term,
        CONFIG: &mut Config,
        draw: &mut Draw,
        graphs: &mut Graphs,
        menu: &mut Menu,
    ) {
        if self.parent.proc_mode {
            return;
        }

        let mut net: NetCollector = NetCollector::new(self, CONFIG);
        if net.parent.redraw {
            self.redraw = true;
        }
        if net.nic.is_none() {
            return;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let x = self.x + 1;
        let y = self.y + 1;
        let w = self.parent.width - 2;
        let h = self.parent.height - 2;
        let bx = self.sub.box_x + 1;
        let by = self.sub.box_y + 1;
        let bw = self.sub.box_width - 2;
        let bh = self.sub.box_height - 2;
        let nic_name : String = net.nic.unwrap().name().to_owned();
        let reset: bool = match net.stats[&nic_name][&"download".to_owned()][&"offset".to_owned()] {
            NetCollectorStat::Bool(b) => b,
            NetCollectorStat::I32(i) => i > 0,
            NetCollectorStat::U64(u) => u > 0,
            NetCollectorStat::Vec(v) => v.len() > 0,
            NetCollectorStat::String(s) => {
                errlog(format!("Malformed type in net.stats[{}]['download']['offset']", nic_name));
                s.parse::<i64>().unwrap_or(0) > 0
            }
        };


        if self.resized || self.redraw {
            out_misc.push_str(self.draw_bg(theme, term).as_str());
            if key.mouse.contains_key(&"b".to_owned()) {
                let mut b_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut b_insert: Vec<i32> = Vec::<i32>::new();
                    b_insert.push(x + w as i32 - nic_name[..10].len() as i32 - 9 + i);
                    b_insert.push(y - 1);
                    b_vec_top.push(b_insert);
                }

                key.mouse.insert("b".to_owned(), b_vec_top);

                let mut n_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut n_insert: Vec<i32> = Vec::<i32>::new();
                    n_insert.push(x + w as i32 - 5 + i);
                    n_insert.push(y - 1);
                    n_vec_top.push(n_insert);
                }

                key.mouse.insert("n".to_owned(), n_vec_top);

                let mut z_vec_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut z_insert: Vec<i32> = Vec::<i32>::new();
                    z_insert.push(x + w as i32 - nic_name[..10].len() as i32 - 14 + i);
                    z_insert.push(y - 1);
                    z_vec_top.push(z_insert);
                }

                key.mouse.insert("z".to_owned(), z_vec_top);
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
                    term.fg,
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
                    term.fg,
                )
                .as_str(),
            );

            if (w as usize) - nic_name[..10].len() - 20 > 6 {
                if !key.mouse.contains_key(&"a".to_owned()) {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();

                        inserter.push(x + w as i32 - 20 - nic_name[..10].len() as i32 + i);
                        inserter.push(y - 1);
                        inserter_top.push(inserter);
                    }
                    key.mouse.insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}",
                        mv::to(
                            (y as u32) - 1,
                            (x as u32) + w - 21 - nic_name[..10].len() as u32
                        ),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if net.auto_min { fx::b } else { "" },
                        theme.colors.hi_fg.call("a".to_owned(), term),
                        theme.colors.title.call("uto".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.fg,
                    )
                    .as_str(),
                );
            }
            if w - nic_name[..10].len() as u32 - 20 > 6 {
                if !key.mouse.contains_key(&"a".to_owned()) {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();
                        inserter.push(x + w as i32 - 20 - nic_name[..10].len() as i32 + i);
                        inserter.push(y - 1);
                        inserter_top.push(inserter);
                    }
                    key.mouse.insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}",
                        mv::to(y as u32 - 1, x as u32 + w - 21 - nic_name[..10].len() as u32),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if net.auto_min { fx::b } else { "" },
                        theme.colors.hi_fg.call("a".to_owned(), term),
                        theme.colors.title.call("auto".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.fg,
                    )
                    .as_str(),
                );
            }

            if w - nic_name[..10].len() as u32 - 20 > 13 {
                if !key.mouse.contains_key(&"y".to_owned()) {
                    let mut inserter_top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut inserter: Vec<i32> = Vec::<i32>::new();
                        inserter.push(x + w as i32 - 26 - nic_name[..10].len() as i32 + i);
                        inserter.push(y - 1);
                        inserter_top.push(inserter);
                    }
                    key.mouse.insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}{}",
                        mv::to(y as u32 - 1, x as u32 + w - 27 - nic_name[..10].len() as u32),
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_left.to_owned(), term),
                        if CONFIG.net_sync { fx::b } else { "" },
                        theme.colors.title.call("s".to_owned(), term),
                        theme.colors.hi_fg.call("y".to_owned(), term),
                        theme.colors.title.call("nc".to_owned(), term),
                        fx::ub,
                        theme
                            .colors
                            .net_box
                            .call(symbol::title_right.to_owned(), term),
                        term.fg,
                    )
                    .as_str(),
                );
            }

            draw.buffer(
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
            let mut strings = net.strings[&nic_name][&direction].clone();
            let mut stats = net.stats[&nic_name][&direction].clone();

            if self.redraw {
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
                },
            } || self.resized
            {
                graphs.net[&direction] = Graph::new_with_vec::<Color>(
                    w - bw - 3,
                    self.graph_height[&direction],
                    theme.gradient[&direction],
                    match stats[&"speed".to_owned()] {
                        NetCollectorStat::Vec(v) => v.iter().map(|u| u.to_owned() as i32).collect(),
                        _ => vec![],
                    },
                    term,
                    direction != "download".to_owned(),
                    if CONFIG.net_sync {
                        net.sync_top
                    } else {
                        match stats[&"graph_top".to_owned()] {
                            NetCollectorStat::Bool(b) => if b {1} else {0},
                            NetCollectorStat::I32(i) => i,
                            NetCollectorStat::Vec(v) => 0,
                            NetCollectorStat::U64(u) => u as i32,
                            NetCollectorStat::String(s) => {
                                errlog("Malformed type in stats['graph_top']".to_owned());
                                s.parse::<i32>().unwrap_or(0)
                            },
                        }
                    },
                    0,
                    if CONFIG.net_color_fixed {
                        Some(net.net_min[&direction])
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
                            y as u32
                        } else {
                            y as u32 + self.graph_height[&"download".to_owned()]
                        },
                        x as u32
                    ),
                    graphs.net[&direction].call(
                        if match stats[&"redraw".to_owned()] {
                            NetCollectorStat::Bool(b) => b,
                            NetCollectorStat::I32(i) => i > 0,
                            NetCollectorStat::Vec(v) => v.len() > 0,
                            NetCollectorStat::U64(u) => u > 0,
                            NetCollectorStat::String(s) => {
                                errlog("Malformed type in stats['redraw']".to_owned());
                                s.parse::<i32>().unwrap_or(0) > 0
                            },
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
                        self.symbols[&direction],
                        mv::to(by + cy, bx + bw - 12),
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
            net.strings[&nic_name][&direction] = strings;
            net.stats[&nic_name][&direction] = stats;
        }

        out.push_str(
            format!(
                "{}{}{}{}",
                mv::to(y as u32, x as u32),
                theme.colors.graph_text.call(
                    if CONFIG.net_sync {
                        net.sync_string
                    } else {
                        net.strings[&nic_name][&"download".to_owned()][&"graph_top".to_owned()]
                    },
                    term
                ),
                mv::to(y as u32 + h - 1, x as u32),
                theme.colors.graph_text.call(
                    if CONFIG.net_sync {
                        net.sync_string
                    } else {
                        net.strings[&nic_name][&"upload".to_owned()][&"graph_top".to_owned()]
                    },
                    term
                ),
            )
            .as_str(),
        );

        draw.buffer(
            self.buffer,
            vec![format!("{}{}{}", out_misc, out, term.fg)],
            false,
            false,
            100,
            menu.active,
            false,
            false,
            key,
        );

        self.redraw = false;
        self.resized = false;
    }
}

use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::{Collector, Collectors},
        config::{Config, ViewMode, SortingOption},
        create_box,
        draw::Draw,
        errlog,
        fx,
        graph::{Graph, Graphs},
        key::Key,
        mv,
        proccollector::{
            ProcCollector,
            ProcCollectorDetails,
        },
        symbol,
        SYSTEM,
        term::Term,
        theme::{Color, Theme},
    },
    inflector::Inflector,
    psutil::process::Status,
    std::{
        collections::HashMap,
        iter::Enumerate,
        path::*
    },
};

pub struct ProcBox {
    parent: BrshtopBox,
    name: String,
    current_y: u32,
    current_h: u32,
    select_max: usize,
    selected: usize,
    selected_pid: u32,
    last_selection: usize,
    filtering: bool,
    moved: bool,
    start: i32,
    count: i32,
    s_len: usize,
    detailed: bool,
    detailed_x: u32,
    detailed_y: u32,
    detailed_width: u32,
    detailed_height: u32,
    buffer: String,
    pid_counter: HashMap<i32, i32>,
    redraw : bool,
}
impl ProcBox {
    pub fn new(brshtop_box: &mut BrshtopBox, CONFIG: &mut Config, ARG_MODE: ViewMode) -> Self {
        brshtop_box.buffers.push("proc".to_owned());
        let procbox = ProcBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
            name: "proc".to_owned(),
            current_y: 0,
            current_h: 0,
            select_max: 0,
            selected: 0,
            selected_pid: 0,
            last_selection: 0,
            filtering: false,
            moved: false,
            start: 1,
            count: 0,
            s_len: 0,
            detailed: false,
            detailed_x: 0,
            detailed_y: 0,
            detailed_width: 0,
            detailed_height: 8,
            buffer: "proc".to_owned(),
            pid_counter: HashMap::<i32, i32>::new(),
            redraw : true,
        };
        procbox.parent.x = 1;
        procbox.parent.y = 1;
        procbox.parent.height_p = 68;
        procbox.parent.width_p = 55;
        procbox.parent.resized = true;
        procbox
    }

    pub fn calc_size(&mut self, term: &mut Term, brshtop_box: &mut BrshtopBox) {
        let (width_p, height_p) = (self.parent.width_p, self.parent.height_p);

        if self.parent.proc_mode {
            width_p = 100;
            height_p = 80;
        }

        self.parent.width = (term.width as f64 * width_p as f64 / 100.0).round() as u32;
        self.parent.width = (term.height as f64 * height_p as f64 / 100.0).round() as u32;
        if self.parent.height + brshtop_box._b_cpu_h as u32 > term.height as u32 {
            self.parent.height = term.height as u32 - brshtop_box._b_cpu_h as u32;
        }
        self.parent.x = term.width as u32 - self.parent.width + 1;
        self.parent.y = brshtop_box._b_cpu_h as u32 + 1;
        self.select_max = self.parent.height as usize - 3;
        self.redraw = true;
        self.parent.resized = true;
    }

    pub fn draw_bg(&mut self, theme: &mut Theme) -> String {
        if self.parent.stat_mode {
            return String::default();
        }
        return create_box(
            0,
            0,
            0,
            0,
            None,
            None,
            Some(theme.colors.proc_box),
            None,
            true,
            Some(Boxes::ProcBox(self)),
        );
    }

    /// Default mouse_pos = (0, 0)
    pub fn selector(
        &mut self,
        key: String,
        mouse_pos: (i32, i32),
        proc_collector: &mut ProcCollector,
        key_class: &mut Key,
        collector: &mut Collector,
        CONFIG: &mut Config,
    ) {
        let old = (self.start, self.selected);

        let mut new_sel: usize = 0;

        if key == "up".to_owned() {
            if self.selected == 1 && self.start > 1 {
                self.start -= 1;
            } else if self.selected == 1 {
                self.selected = 0;
            } else if self.selected > 1 {
                self.selected -= 1;
            }
        } else if key == "down".to_owned() {
            if self.selected == 0 && proc_collector.detailed && self.last_selection > 0 {
                self.selected = self.last_selection;
                self.last_selection = 0;
            }
            if self.selected == self.select_max
                && self.start < proc_collector.num_procs as i32 - self.select_max as i32 + 1
            {
                self.start += 1;
            } else if self.selected < self.select_max {
                self.selected += 1;
            }
        } else if key == "mouse_scroll_up".to_owned() && self.start > 1 {
            self.start -= 5;
        } else if key == "mouse_scroll_down".to_owned()
            && self.start < proc_collector.num_procs as i32 - self.select_max as i32 + 1
        {
            self.start += 5;
        } else if key == "page_up".to_owned() && self.start > 1 {
            self.start -= self.select_max as i32;
        } else if key == "page_down".to_owned()
            && self.start < proc_collector.num_procs as i32 - self.select_max as i32 + 1
        {
            self.start += self.select_max as i32;
        } else if key == "home".to_owned() {
            if self.start > 1 {
                self.start = 1;
            } else if self.selected > 0 {
                self.selected = 0;
            }
        } else if key == "end".to_owned() {
            if self.start < proc_collector.num_procs as i32 - self.select_max as i32 + 1 {
                self.start = proc_collector.num_procs as i32 - self.select_max as i32 + 1;
            } else if self.selected < self.select_max {
                self.selected = self.select_max;
            }
        } else if key == "mouse_click".to_owned() {
            if mouse_pos.0 > (self.parent.x + self.parent.width - 4) as i32
                && self.current_y as i32 + 1 < mouse_pos.1
                && mouse_pos.1 < self.current_y as i32 + 1 + self.select_max as i32 + 1
            {
                if mouse_pos.1 == self.current_y as i32 + 2 {
                    self.start = 1;
                } else if mouse_pos.1 == (self.current_y + 1 + self.select_max as u32) as i32 {
                    self.start = proc_collector.num_procs as i32 - self.select_max as i32 + 1;
                } else {
                    self.start = ((mouse_pos.1 - self.current_y as i32) as f64
                        * ((proc_collector.num_procs as u32 - self.select_max as u32 - 2) as f64
                            / (self.select_max - 2) as f64))
                        .round() as i32;
                }
            } else {
                new_sel = (mouse_pos.1
                    - self.current_y as i32
                    - if mouse_pos.1 >= self.current_y as i32 - 1 {
                        1
                    } else {
                        0
                    }) as usize;

                if new_sel > 0 && new_sel == self.selected {
                    key_class.list.insert(0, "enter".to_owned());
                    return;
                } else if new_sel > 0 && new_sel != self.selected {
                    if self.last_selection != 0 {
                        self.last_selection = 0;
                    }
                    self.selected = new_sel;
                }
            }
        } else if key == "mouse_unselect".to_owned() {
            self.selected = 0;
        }

        if self.start > (proc_collector.num_procs - self.select_max as u32 + 1) as i32
            && proc_collector.num_procs > self.select_max as u32
        {
            self.start = (proc_collector.num_procs - self.select_max as u32 + 1) as i32;
        } else if self.start > proc_collector.num_procs as i32 {
            self.start = proc_collector.num_procs as i32;
        }
        if self.start < 1 {
            self.start = 1;
        }
        if self.selected as u32 > proc_collector.num_procs
            && proc_collector.num_procs < self.select_max as u32
        {
            self.selected = proc_collector.num_procs as usize;
        } else if self.selected > self.select_max {
            self.selected = self.select_max;
        }
        if self.selected < 0 {
            self.selected = 0;
        }

        if old != (self.start, self.selected) {
            self.moved = true;
            collector.collect(
                vec![Collectors::ProcCollector(proc_collector)],
                CONFIG,
                true,
                false,
                true,
                true,
                true,
            );
        }
    }

    pub fn draw_fg(
        &mut self,
        CONFIG: &mut Config,
        key: &mut Key,
        THEME: &mut Theme,
        graphs: &mut Graphs,
        term: &mut Term,
        draw : &mut Draw,
    ) {
        if self.parent.stat_mode {
            return;
        }
        let mut proc = ProcCollector::new(self.buffer.clone());

        if proc.parent.proc_interrupt {
            return;
        }

        if proc.parent.redraw {
            self.redraw = true;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut n: u32 = 0;
        let mut x: u32 = self.parent.x + 1;
        let mut y: u32 = self.current_y + 1;
        let mut w: u32 = self.width - 2;
        let mut h: u32 = self.current_h - 2;
        let mut prog_len: usize = 0;
        let mut arg_len: usize = 0;
        let mut val: i32 = 0;
        let mut c_color: String = String::default();
        let mut m_color: String = String::default();
        let mut t_color: String = String::default();
        let mut sort_pos: usize = 0;
        let mut tree_len: usize = 0;
        let mut is_selected: bool = false;
        let mut calc: u32 = 0;
        let mut dgx: u32 = 0;
        let mut dgw: u32 = 0;
        let mut dx: u32 = 0;
        let mut dw: u32 = 0;
        let mut dy: u32 = 0;
        let mut l_count: usize = 0;
        let mut scroll_pos: u32 = 0;
        let mut killed: bool = true;
        let mut indent: String = String::default();
        let mut offset: u32 = 0;
        let mut tr_show: bool = true;
        let mut usr_show: bool = true;
        let mut vals: Vec<String> = Vec::<String>::new();
        let mut g_color: String = String::default();
        let mut s_len: usize = 0;

        if proc.search_filter.len() > 0 {
            s_len = proc.search_filter[..10].len();
        }
        let mut loc_string: String = format!(
            "{}/{}",
            self.start + self.selected as i32 - 1,
            proc.num_procs
        );
        let mut end: String = String::default();

        if proc.detailed {
            dgx = x;
            dgw = w / 3;
            dw = w - dgw - 1;

            if dw > 120 {
                dw = 120;
                dgw = w - 121;
            }
            dx = x + dgw + 2;
            dy = self.parent.y + 1;
        }

        if w > 67 {
            arg_len = w
                - 53
                - if proc.num_procs > self.select_max as u32 {
                    1
                } else {
                    0
                };
            prog_len = 15;
        } else {
            arg_len = 0;
            prog_len = w
                - 38
                - if proc.num_procs > self.select_max as u32 {
                    1
                } else {
                    0
                };

            if prog_len < 15 {
                tr_show = false;
                prog_len += 5;
            }
            if prog_len < 12 {
                usr_show = false;
                prog_len += 9;
            }
        }

        if CONFIG.proc_tree {
            tree_len = arg_len + prog_len + 6;
            arg_len = 0;
        }

        // * Buttons and titles only redrawn if needed
        if self.parent.resized || self.redraw {
            s_len += CONFIG.proc_sorting.to_string().len();
            if self.parent.resized || s_len != self.s_len || proc.detailed {
                self.s_len = s_len;
                for k in [
                    "e", "r", "c", "t", "k", "i", "enter", "left", " ", "f", "delete",
                ]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect()
                {
                    if key.mouse.contains_key(&k) {
                        key.mouse.remove(&k);
                    }
                }
            }
            if proc.detailed {
                let mut killed: bool = match proc.details[&"killed".to_owned()] {
                    ProcCollectorDetails::Bool(b) => b,
                    _ => {
                        errlog("ProcCollectorDetails contained non-numeric value for 'killed'".to_owned())
                        false
                    },
                };
                let mut main: Color = if self.selected == 0 && !killed {
                    THEME.colors.main_fg
                } else {
                    THEME.colors.inactive_fg
                };
                let mut hi: Color = if self.selected == 0 && !killed {
                    THEME.colors.hi_fg
                } else {
                    THEME.colors.inactive_fg
                };
                let mut title: Color = if self.selected == 0 && !killed {
                    THEME.colors.title
                } else {
                    THEME.colors.inactive_fg
                };
                if self.current_y != self.parent.y + 8
                    || self.parent.resized
                    || graphs.detailed_cpu.NotImplemented
                {
                    self.current_y = self.y + 8;
                    self.current_h = self.parent.height - 8;
                    for i in 0..7 as u32 {
                        out_misc.push_str(
                            format!("{}{}", mv::to(dy + i, x), " ".repeat(w as usize)).as_str(),
                        );
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}{}{}{}{}{}{}{}{}{}",
                            mv::to(dy + 7, x - 1),
                            THEME.colors.proc_box,
                            symbol::title_right,
                            symbol::h_line.repeat(w),
                            symbol::title_left,
                            mv::to(dy + 7, x + 1),
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            THEME.colors.title.call(self.name.clone(), term),
                            fx::ub,
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                            THEME.colors.div_line,
                        )
                        .as_str(),
                    );
                    for i in 0..7 as u32 {
                        out_misc.push_str(
                            format!("{}{}", mv::to(dy + i, dgx + dgw + 1), symbol::v_line,)
                                .as_str(),
                        );
                    }
                }

                out_misc.push_str(
                    format!(
                        "{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                        mv::to(dy - 1, x - 1),
                        THEME.colors.proc_box,
                        symbol::left_up,
                        symbol::h_line.repeat(w),
                        symbol::right_up,
                        mv::to(dy - 1, dgx + dgw + 1),
                        symbol::div_up,
                        mv::to(dy - 1, x + 1),
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        THEME
                            .colors
                            .title
                            .call(proc.details["pid".to_owned()].to_string()),
                        fx::ub,
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        THEME.colors.title.call(
                            proc.details["name"].to_string()[..dgw as usize - 11].to_owned(),
                            term
                        ),
                        fx::ub,
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                    )
                    .as_str(),
                );

                if self.selected == 0 {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..7 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((dx + dw) as i32 - 10 + i);
                        pusher.push(dy as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["enter".to_owned()] = top.clone();
                }

                if self.selected == 0 && !killed {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((dx + 2) as i32 + i);
                        pusher.push(dy as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["t".to_owned()] = top.clone();
                }

                out_misc.push_str(
                    format!(
                        "{}{}{}{}close{} {}{}{}{}{}{}{}t{}erminate{}{}",
                        mv::to(dy - 1, dx + dw - 11),
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        if self.selected > 0 {
                            title
                        } else {
                            THEME.colors.title
                        },
                        fx::ub,
                        if self.selected > 0 {
                            main
                        } else {
                            THEME.colors.main_fg
                        },
                        symbol::enter,
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                        mv::to(dy - 1, dx + 1),
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                    )
                    .as_str(),
                );
                if dw > 28 {
                    if self.selected == 0 && !killed && !key.mouse.contains_key(&"k".to_owned()) {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                        for i in 0..4 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push((dx + 13) as i32 + i);
                            pusher.push(dy as i32 - 1);
                            top.push(pusher);
                        }

                        key.mouse["k".to_owned()] = top.clone();
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}k{}ill{}{}",
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                        )
                        .as_str(),
                    );
                }

                if dw > 39 {
                    if self.selected == 0 && !killed && !key.mouse.contains_key(&"i".to_owned()) {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                        for i in 0..9 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push((dx + 19) as i32 + i);
                            pusher.push(dy as i32 - 1);
                            top.push(pusher);
                        }

                        key.mouse["i".to_owned()] = top.clone();
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}i{}nterrupt{}{}",
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                        )
                        .as_str(),
                    );
                }

                if graphs.detailed_cpu.NotImplemented || self.parent.resized {
                    graphs.detailed_cpu = Graph::new(
                        (dgw + 1) as i32,
                        7,
                        Some(Color::new(THEME.gradient["cpu".to_owned()])),
                        proc.details_cpu.iter().map(|i| i as i32).collect(),
                        term,
                        false,
                        0,
                        0,
                        None,
                    );
                    graphs.detailed_mem = Graph::new(
                        (dw / 3) as i32,
                        1,
                        None,
                        proc.details_mem.iter().map(|i| i as i32).collect(),
                        term,
                        false,
                        0,
                        0,
                        None
                    );
                }
                self.select_max = self.parent.height as usize - 11;
                y = self.parent.y + 9;
                h = self.parent.height - 10;
            } else {
                if self.current_y != self.parent.y || self.parent.resized {
                    self.current_y = self.parent.y;
                    self.current_h = self.parent.height;
                    y = self.parent.y + 1;
                    h = self.parent.height - 2;
                    out_misc.push_str(format!("{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                            mv::to(y - 1, x - 1),
                            THEME.colors.proc_box,
                            symbol::left_up,
                            symbol::h_line.repeat(w),
                            symbol::right_up,
                            mv::to(y - 1, x + 1),
                            THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                            fx::b,
                            THEME.colors.title.call(self.name.clone(), term),
                            fx::ub,
                            THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                            mv::to(y + 7, x - 1),
                            THEME.colors.proc_box.call(symbol::v_line.to_owned(), term),
                            mv::right(w),
                            THEME.colors.proc_box.call(symbol::v_line.to_owned(), term),
                        )
                        .as_str()
                    );
                }
                self.select_max = self.parent.height as usize - 3;
            }

            sort_pos = (x + w) as usize - CONFIG.proc_sorting.to_string().len() - 7;
            if !key.mouse.contains_key(&"left".to_owned()) {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..3 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push(sort_pos as i32 + i);
                    pusher.push(y as i32 - 1);
                    top.push(pusher);
                }

                key.mouse["left".to_owned()] = top.clone();

                top = Vec::<Vec<i32>>::new();

                for i in 0..3 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push(sort_pos as i32 + CONFIG.proc_sorting.to_string().len() as i32 + 3 + i);
                    pusher.push(y as i32 - 1);
                    top.push(pusher);
                }

                key.mouse["right".to_owned()] = top.clone();
            }

            out_misc.push_str(format!("{}{}{}{}{}{}{} {} {}{}{}",
                    mv::to(y - 1, x + 8),
                    THEME.colors.proc_box.call(symbol::h_line.repeat(w - 9).to_owned(), term),
                    if !proc.detailed {
                        "".to_owned()
                    } else {
                        format!("{}{}", 
                            mv::to(dy + 7, dgx + dgw + 1),
                            THEME.colors.proc_box.call(symbol::div_down.to_owned(), term)
                        )
                    },
                    mv::to(y - 1, sort_pos as u32),
                    THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                    fx::b,
                    THEME.colors.hi_fg.call("<".to_owned(), term),
                    THEME.colors.title.call(CONFIG.proc_sorting.to_string(), term),
                    THEME.colors.hi_fg.call(">".to_owned(), term),
                    fx::ub,
                    THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );

            if w > 29 + s_len as u32 {
                if !key.mouse.contains_key(&"e".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sorting_pos - 5) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["e".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}",
                        mv::to(y - 1, sort_pos as u32 - 6),
                        THEME.colors.call(symbol::title_left.to_owned(), term),
                        if CONFIG.proc_tree {
                            fx::b
                        } else {
                            "".to_owned()
                        },
                        THEME.colors.title.call("tre".to_owned(), term),
                        THEME.colors.hi_fg.call("e".to_owned(), term),
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if w > 37 + s_len as u32 {
                if !key.mouse.contains_key(&"r".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..7 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sorting_pos - 14) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["r".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}",
                        mv::to(y - 1, sorting_pos as u32 - 15),
                        THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.proc_reversed {
                            fx::b
                        } else {
                            "".to_owned()
                        },
                        THEME.colors.hi_fg.call("r".to_owned(), term),
                        THEME.colors.title.call("everse".to_owned(), term),
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if w > 47 + s_len as u32 {
                if !key.mouse.contains_key(&"c".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0.. if proc.search_filter.len() == 0 {6} else {2 + proc.search_filter[(proc.search_filter.len() - 11)..].len()} {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sorting_pos - 24) as i32 + i as i32);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["c".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}{}",
                        mv::to(y - 1, sort_pos as u32 - 25),
                        THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.proc_per_core {
                            fx::b
                        } else {
                            ""
                        },
                        THEME.colors.title.call("per-".to_owned(), term),
                        THEME.colors.hi_fg.call("c".to_owned(), term),
                        THEME.colors.title.call("ore".to_owned(), term),
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if !key.mouse.contains_key(&"f".to_owned()) || self.parent.resized {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0.. if proc.search_filter.len() == 0 {6} else {2 + proc.search_filter[(proc.search_filter.len() - 11)..].len()} {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((x + 5) as i32 + i as i32);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["f".to_owned()] = top.clone();
            }
            if proc.search_filter.len() > 0 {
                if !key.mouse.contains_key(&"delete".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..3 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((x + 11 + proc.search_filter[(proc.search_filter.len() - 11)..].len() as u32) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.mouse["delete".to_owned()] = top.clone();
                }
            } else if key.mouse.contains_key(&"delete".to_owned()) {
                key.mouse.remove(&"delete".to_owned());
            }

            out_misc.push_str(format!("{}{}{}{}",
                    mv::to(y - 1, x + 7),
                    THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                    if self.filtering || proc.search_filter.len() > 0 {
                        fx::b
                    } else {
                        ""
                    },
                    THEME.colors.hi_fg.call("f".to_owned(), term),
                    THEME.colors.title,
                    if proc.search_filter.len() == 0 && !self.filtering {
                        "ilter".to_owned()
                    } else {
                        format!(" {}{}",
                            proc.search_filter[(proc.search_filter.len() - 1 + (if w < 83 {10} else {w - 74}))..],
                            if self.filtering {
                                fx::bl.to_owned() + "█" + fx::ubl
                            } else {
                                THEME.colors.hi_fg.call(" del".to_owned(), term),
                            }
                        )
                    },
                    THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );

            main = if self.selected == 0 {
                THEME.colors.inactive_fg
            } else {
                THEME.colors.main_fg
            };
            hi = if self.selected == 0 {
                THEME.colors.inactive_fg
            } else {
                THEME.colors.hi_fg
            };
            title = if self.selected == 0 {
                THEME.colors.inactive_fg
            } else {
                THEME.colors.title
            };

            out_misc.push_str(format!("{}{}{}{}{}{}{} {}{} {}{}{}{}{}{}{}info {}{}{}{}",
                    mv::to(y + h, x + 1),
                    THEME.colors.proc_box,
                    symbol::h_line.repeat(w-4),
                    mv::to(y + h, x + 1),
                    THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                    main,
                    symbol::up,
                    fx::b,
                    THEME.colors.main_fg.call("select".to_owned(), term),
                    fx::ub,
                    if self.selected == self.select_max {
                        THEME.colors.inactive_fg
                    } else {
                        THEME.colors.main_fg
                    },
                    symbol::down,
                    THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                    title,
                    fx::b,
                    fx::ub,
                    main,
                    symbol::enter,
                    THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );
            if !key.mouse.contains_key(&"enter".to_owned()) {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..6 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push((x + 14) as i32 + i);
                    pusher.push((y + h) as i32);
                    top.push(pusher);
                }

                key.mouse["enter".to_owned()] = top.clone();
            }
            if w - loc_string.len() as u32 > 34 {
                if !key.mouse.contains_key(&"t".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 22 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.mouse["t".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}t{}erminate{}{}",
                        THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }
            if w - loc_string.len() as u32 > 40 {
                if !key.mouse.contains_key(&"k") {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 33 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.mouse["k".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}k{}ill{}{}",
                        THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                );
            }
            if w - loc_string.len() as u32 > 51 {
                if !key.mouse.contains_key(&"i") {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 39 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.mouse["i".to_owned()] = top.clone();
                }
                out_misc.push_str(format!("{}{}{}i{}terrupt{}{}",
                        THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                );
            }
            if CONFIG.proc_tree && w - loc_string.len() as u32 > 65 {
                if w - loc_string.len() as u32 > 40 {
                    if !key.mouse.contains_key(&" ") {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
    
                        for i in 0..12 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push(x as i32 + 50 + i);
                            pusher.push((y + h) as i32);
                            top.push(pusher);
                        }
    
                        key.mouse[" ".to_owned()] = top.clone();
                    }
                    out_misc.push_str(format!("{}{}{}spc {}collapse{}{}",
                            THEME.colors.proc_box.call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME.colors.proc_box.call(symbol::title_right.to_owned(), term),
                        )
                    );
                }
            }

            // * Processes labels
            let mut selected : String = String::default();
            let mut label : String = String::default();
            selected = match CONFIG.proc_sorting {
                SortingOption::Memory => String::from("mem"),
                SortingOption::Threads => if !CONFIG.proc_tree && arg_len == 0 {
                        String::from("tr")
                    } else {
                        String::default()
                    },
                _ => {
                    errlog("Wrong sorting option in CONFIG.proc_sorting when processing lables...");
                    String::default()
                },
            };

            if CONFIG.proc_tree {
                label = format!("{}{}{}{:<width$}{}{}Mem%{:>11}{}{} {}",
                    THEME.colors.title,
                    fx::b,
                    mv::to(y , x),
                    " Tree:",
                    if tr_show {
                        format!("{:>9}", "Threads: ")
                    } else {
                        " ".repeat(4).to_owned()
                    },
                    if usr_show {
                        format!("{:<9}", "User:")
                    } else {
                        String::default()
                    },
                    "Cpu%",
                    fx::ub,
                    THEME.colors.main_fg,
                    width : tree_len - 2,
                );
                if ["pid", "program", "arguments"].iter().map(|s| s.to_owned().to_owned()).collect().contains(selected) {
                    selected = String::from("tree");
                }
            } else {
                label = format!("{}{}{:>7} {}{}{}{}Mem%{:>11}{}{} ",
                    THEME.colors.title,
                    fx::b,
                    mv::to(y, x),
                    "Pid:",
                    if prog_len > 8 {
                        "Program:".to_owned()
                    } else {
                        format!("{:<width$}", "Prg:", width : prog_len)
                    },
                    if arg_len {
                        format!("{:<width$}", "Arguments:", width : arg_len - 4)
                    } else {
                        "".to_owned()
                    },
                    if tr_show {
                        if arg_len > 0 {
                            format!("{:>9}", "Threads:")
                        } else {
                            format!("{:^5}", "Tr:")
                        }
                    } else {
                        "".to_owned()
                    },
                    if usr_show {
                        format!("{:<9}", "User:")
                    } else {
                        "".to_owned()
                    },
                    "Cpu%",
                    fx::ub,
                    THEME.colors.main_fg
                    if proc.num_procs > self.select_max as u32 {
                        " "
                    } else {
                        ""
                    },
                );

                if selected == String::from("program") && prog_len <= 8 {
                    selected = String::from("prg");
                }
            }

            selected = selected.split(" ")[0]..to_title_case();
            if CONFIG.proc_mem_bytes {
                label = label.replace("Mem%", "MemB");
            }
            label = label.replace(selected, format!("{}{}{}", fx::u, selected, fx::uu).as_str());
            out_misc.push_str(label.as_str());
            draw.buffer("proc_misc".to_owned(), [out_misc], false, false, 100, true, false, false, key);
        }

        // * Detailed box draw
        if proc.detailed {
            let mut stat_color : String = match proc.details[&"status".to_owned()] {
                ProcCollectorDetails::Status(s) => if s == Status::Running {
                    fx::b.to_owned()
                } else if [Status::Dead, Status::Stopped, Status::Zombie].contains(s) {
                    THEME.colors.inactive_fg.to_string()
                } else {
                    String::default()
                },
                _ => {
                    errlog("Wrong ProcCollectorDetails type when assigning stat_color");
                    String::default()
                },
            };
            let expand : u32 = proc.expand;
            let iw : u32 = (dw - 3) / (4 + expand);
            let iw : u32 = iw - 1;

            out.push_str(format!("{}{}{}{}{}%{}{}",
                    mv::to(dy, dgx),
                    graphs.detailed_cpu.call(
                        if self.moved || match proc.details["killed".to_owned()] {
                            ProcCollectorDetails::Bool(b) => b,
                            _ => {
                                errlog("Wrong ProcCollectorDetails type from proc.details['killed']");
                                false
                            },
                        } {
                            None
                        } else {
                            Some(proc.details_cpu[proc.details_cpu.len() - 2])
                        }, 
                        term
                    ),
                    mv::to(dy , dgx),
                    THEME.colors.title,
                    fx::b,
                    if match proc.details["killed".to_owned()] {
                        ProcCollectorDetails::Bool(b) => b,
                        _ => {
                            errlog("Wrong ProcCollectorDetails type from proc.details['killed']");
                            false
                        },
                    } {
                        0
                    } else {
                        match proc.details["cpu_percent".to_owned()] {
                            ProcCollectorDetails::U32(u) => u,
                            _ => {
                                errlog("Wrong ProcCollectorDetails type from proc.details['cpu_percent']");
                                0
                            },
                        }
                    },
                    mv::right(1),
                    (if SYSTEM == "MacOS".to_owned() {
                        ""
                    } else {
                        if dgw < 20 {
                            "C"
                        } else {
                            "Core"
                        }
                    }).to_owned() + proc.details["cpu_name".to_owned()].to_string().as_str(),
                )
                .as_str()
            );

            for (i, l) in vec!["C", "P", "U"].iter().map(|s| s.to_owned().to_owned()).enumerate() {
                out.push_str(format!("{}{}", mv::to(dy + 2 + i as u32, dgx), l).as_str());
            }
            for (i, l) in vec!["C", "M", "D"].iter().map(|s| s.to_owned().to_owned()).enumerate() {
                out.push_str(format!("{}{}", mv::to(dy + 4 + i as u32, dx + 1), l).as_str());
            }
            out.push_str(format!("{} {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} {}{}{}{} {}{}{}{}{}{}{}{}{}{}",
                    mv::to(dy, dx + 1),
                    format!("{:^first$.second$}", "Status:", first = iw, second = iw2)
                    format!("{:^first$.second$}", "Elapsed:", first = iw, second = iw2)
                    if dw > 28 {
                        format!("{:^first$.second$}", "Parent:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if dw > 38 {
                        format!("{:^first$.second$}", "User:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 0 {
                        format!("{:^first$.second$}", "Threads:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 1 {
                        format!("{:^first$.second$}", "Nice:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 2 {
                        format!("{:^first$.second$}", "IO Read:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 3 {
                        format!("{:^first$.second$}", "IO Write:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 4 {
                        format!("{:^first$.second$}", "TTY:", first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    mv::to(dy + 3, dx),
                    THEME.colors.title,
                    fx::ub,
                    THEME.colors.main_fg,
                    stat_color,
                    proc.details["status".to_owned()],
                    fx::ub,
                    THEME.colors.main_fg,
                    proc.details["uptime".to_owned()],
                    if dw > 28 {
                        format!("{:^first$.second$}", proc.details["parent_name".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if dw > 38 {
                        format!("{:^first$.second$}", proc.details["username".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 0 {
                        format!("{:^first$.second$}", proc.details["threads".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 1 {
                        format!("{:^first$.second$}", proc.details["nice".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 2 {
                        format!("{:^first$.second$}", proc.details["io_read".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 3 {
                        format!("{:^first$.second$}", proc.details["io_write".to_owned()], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    if expand > 4 {
                        format!("{:^first$.second$}", proc.details["terminal".to_owned()].to_string()[(proc.details["terminal".to_owned()].to_string().len() - 1 - iw2)..], first = iw, second = iw2)
                    } else {
                        String::default()
                    },
                    mv::to(dy + 3, dx),
                    THEME.colors.title,
                    fx::b,
                    format!("{:>width$}",
                        (if dw > 42 {
                            "Memory: "
                        } else {
                            "M:"
                        }).to_owned() + proc.details["memory_percent"].to_string().as_str() + "%",
                        width = (dw / 3) - 1,
                    ),
                    fx::ub,
                    THEME.colors.inactive_fg,
                    ". ".repeat(dw / 3),
                    mv::left(dw / 3),
                    THEME.colors.proc_misc,
                    graphs.detailed_mem.call(
                        if self.moved 
                        {
                            None
                        } else {
                            Some(
                                match proc.details["memory_percent".to_owned()] {
                                    ProcCollectorDetails::Bool(b) => if b {1} else {0},
                                    ProcCollectorDetails::U32(u) => u,
                                    _ => {
                                        errlog("ProcCollectorDetails contained non-numeric value for 'memory_percent'".to_owned())
                                        0
                                    }
                                }
                            )
                        }, 
                        term
                    ),
                    THEME.colors.title,
                    fx::b,
                    format!("{:.width$}", proc.details["memory_bytes".to_owned()], width = (dw / 3) - 2),
                    THEME.colors.main_fg,
                    fx::ub,
                )
                .as_str()
            );
            let cy = dy + if match proc.details["cmdline".to_owned()] {
                ProcCollectorDetails::Bool(b) => if b {1} else {0},
                ProcCollectorDetails::U32(u) => u,
                ProcCollectorDetails::String(s) => s.len() as u32,
                ProcCollectorDetails::VecString(v) => v.len() as u32,
                _ => {
                    errlog("Wrong type in proc.details['cmdline']");
                    0
                },
            } > d2 - 5 {
                4
            } else {
                5
            };
            for i in 0..(proc.details["cmdline"].len() as u32 / (dw - 5)) {
                out.push_str(format!("{}{}",
                        mv::to(cy + i, dx + 3),
                        format!("{:direction$width}",
                            if dw - 5 >= 0 {
                                proc.details["cmdline".to_owned()][((dw-5)*i)..][..(dw-5)]
                            } else {
                                proc.details["cmdline".to_owned()][((proc.details["cmdline".to_owned()].len() - 1 - 5)*i)..][..(proc.details["cmdline".to_owned()].len() - 1 -5)]
                            },
                            direction = if i == 0 {
                                "^"
                            } else {
                                "<"
                            },
                            width = dw - 5,
                        ),
                    )
                );
                if i == 2 {
                    break;
                }
            }
        }

        // * Checking for selection out of bounds
        if self.start > (proc.num_procs - self.select_max as u32 + 1) as i32 && proc.num_procs > self.select_max as u32 {
            self.start = (proc.num_procs - self.select_max as u32 + 1) as i32;
        } else if self.start > proc.num_procs as i32 {
            self.start = proc.num_procs as i32;
        }
        if self.start < 1 {
            self.start = 1;
        }
        if self.selected as u32 > proc.num_procs && proc.num_procs < self.select_max as u32 {
            self.selected = proc.num_procs as usize;
        } else if self.selected > self.select_max {
            self.selected = self.select_max;
        }

        // * Start iteration over all processes and info
        let cy: u32 = 1;

        for (n, (pid, items)) in proc.processes.enumerate() {
            if n < self.start {
                continue;
            }
            l_count += 1;
            if l_count == self.selected {
                is_selected = true;
                self.selected_pid = pid;
            } else {
                is_selected = false;
            }

            indent = items.get
        }
        
    }
}

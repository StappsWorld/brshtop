use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::{Collector, Collectors},
        config::{Config, ViewMode, SortingOption},
        create_box,
        draw::Draw,
        errlog,
        floating_humanizer,
        fx,
        graph::{ColorSwitch, Graph, Graphs},
        key::Key,
        menu::Menu,
        mv,
        proccollector::{
            ProcCollector,
            ProcCollectorDetails,
            ProcessInfo,
        },
        symbol,
        SYSTEM,
        term::Term,
        theme::{Color, Theme},
    },
    inflector::Inflector,
    once_cell::sync::OnceCell,
    psutil::{
        Bytes,
        process::{
            Status,
            MemoryInfo,
        },
    },
    std::{
        collections::HashMap,
        convert::TryFrom,
        iter::Enumerate,
        path::*,
        sync::Mutex,
    },
};

pub struct ProcBox {
    parent: BrshtopBox,
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
    pid_counter: HashMap<u32, u32>,
    redraw : bool,
}
impl<'a> ProcBox {
    pub fn new(brshtop_box: &OnceCell<Mutex<BrshtopBox>>, CONFIG: &OnceCell<Mutex<Config>>, ARG_MODE: ViewMode) -> Self {
        brshtop_box.get().unwrap().lock().unwrap().push_buffers("proc".to_owned());
        let mut procbox = ProcBox {
            parent: BrshtopBox::new(CONFIG, ARG_MODE),
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
            pid_counter: HashMap::<u32, u32>::new(),
            redraw : true,
        };
        procbox.set_parent_x(1);
        procbox.set_parent_y(1);
        procbox.set_parent_height_p(68);
        procbox.set_parent_width_p(55);
        procbox.set_parent_resized(true);
        procbox.set_parent_name("proc".to_owned());
        procbox
    }

    pub fn calc_size(&mut self, term: &OnceCell<Mutex<Term>>, _b_cpu_h : i32) {
        let mut width_p = self.parent.get_width_p();
        let mut height_p = self.parent.get_height_p();

        if self.parent.get_proc_mode() {
            width_p = 100;
            height_p = 80;
        }

        self.parent.set_width((term.get().unwrap().lock().unwrap().get_width() as f64 * width_p as f64 / 100.0).round() as u32);
        self.parent.set_width((term.get().unwrap().lock().unwrap().get_height() as f64 * height_p as f64 / 100.0).round() as u32);
        if self.parent.get_height() + _b_cpu_h as u32 > term.get().unwrap().lock().unwrap().get_height() as u32 {
            self.parent.set_height(u32::try_from(term.get().unwrap().lock().unwrap().get_height() as i32 - _b_cpu_h as i32).unwrap_or(0));
        }
        self.parent.set_x(u32::try_from(term.get().unwrap().lock().unwrap().get_width() as i32 - self.parent.get_width() as i32 + 1).unwrap_or(0));
        self.parent.set_y(_b_cpu_h as u32 + 1);
        self.select_max = usize::try_from(self.parent.get_height() as i32 - 3).unwrap_or(0);
        self.redraw = true;
        self.parent.set_resized(true);
    }

    pub fn draw_bg(&self, theme: &OnceCell<Mutex<Theme>>, term : &OnceCell<Mutex<Term>>) -> String {
        if self.parent.get_stat_mode() {
            return String::default();
        }
        return create_box(
            0,
            0,
            0,
            0,
            None,
            None,
            Some(theme.get().unwrap().lock().unwrap().colors.proc_box),
            None,
            true,
            Some(Boxes::ProcBox),
            term,
            theme,
            None,
            None,
            None,
            None,
            Some(self),
        );
    }

    /// Default mouse_pos = (0, 0)
    pub fn selector(
        &mut self,
        key: String,
        mouse_pos: (i32, i32),
        proc_collector: &OnceCell<Mutex<ProcCollector>>,
        key_class: &OnceCell<Mutex<Key>>,
        collector: &OnceCell<Mutex<Collector>>,
        CONFIG: &OnceCell<Mutex<Config>>,
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
            if self.selected == 0 && proc_collector.get().unwrap().lock().unwrap().detailed && self.last_selection > 0 {
                self.selected = self.last_selection;
                self.last_selection = 0;
            }
            if self.selected == self.select_max
                && self.start < proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1
            {
                self.start += 1;
            } else if self.selected < self.select_max {
                self.selected += 1;
            }
        } else if key == "mouse_scroll_up".to_owned() && self.start > 1 {
            self.start -= 5;
        } else if key == "mouse_scroll_down".to_owned()
            && self.start < proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1
        {
            self.start += 5;
        } else if key == "page_up".to_owned() && self.start > 1 {
            self.start -= self.select_max as i32;
        } else if key == "page_down".to_owned()
            && self.start < proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1
        {
            self.start += self.select_max as i32;
        } else if key == "home".to_owned() {
            if self.start > 1 {
                self.start = 1;
            } else if self.selected > 0 {
                self.selected = 0;
            }
        } else if key == "end".to_owned() {
            if self.start < proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1 {
                self.start = proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1;
            } else if self.selected < self.select_max {
                self.selected = self.select_max;
            }
        } else if key == "mouse_click".to_owned() {
            if mouse_pos.0 > self.parent.get_x() as i32 + self.parent.get_width() as i32 - 4
                && self.current_y as i32 + 1 < mouse_pos.1
                && mouse_pos.1 < self.current_y as i32 + 1 + self.select_max as i32 + 1
            {
                if mouse_pos.1 == self.current_y as i32 + 2 {
                    self.start = 1;
                } else if mouse_pos.1 == (self.current_y + 1 + self.select_max as u32) as i32 {
                    self.start = proc_collector.get().unwrap().lock().unwrap().num_procs as i32 - self.select_max as i32 + 1;
                } else {
                    self.start = ((mouse_pos.1 - self.current_y as i32) as f64
                        * ((proc_collector.get().unwrap().lock().unwrap().num_procs as u32 - self.select_max as u32 - 2) as f64
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
                    key_class.get().unwrap().lock().unwrap().list.insert(0, "enter".to_owned());
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

        if self.start > (proc_collector.get().unwrap().lock().unwrap().num_procs - self.select_max as u32 + 1) as i32
            && proc_collector.get().unwrap().lock().unwrap().num_procs > self.select_max as u32
        {
            self.start = (proc_collector.get().unwrap().lock().unwrap().num_procs - self.select_max as u32 + 1) as i32;
        } else if self.start > proc_collector.get().unwrap().lock().unwrap().num_procs as i32 {
            self.start = proc_collector.get().unwrap().lock().unwrap().num_procs as i32;
        }
        if self.start < 1 {
            self.start = 1;
        }
        if self.selected as u32 > proc_collector.get().unwrap().lock().unwrap().num_procs
            && proc_collector.get().unwrap().lock().unwrap().num_procs < self.select_max as u32
        {
            self.selected = proc_collector.get().unwrap().lock().unwrap().num_procs as usize;
        } else if self.selected > self.select_max {
            self.selected = self.select_max;
        }
        if self.selected < 0 {
            self.selected = 0;
        }

        if old != (self.start, self.selected) {
            self.moved = true;
            collector.get().unwrap().lock().unwrap().collect(
                vec![Collectors::ProcCollector],
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
        CONFIG: &OnceCell<Mutex<Config>>,
        key: & OnceCell<Mutex<Key>>,
        THEME: &OnceCell<Mutex<Theme>>,
        graphs: &OnceCell<Mutex<Graphs>>,
        term: &OnceCell<Mutex<Term>>,
        draw : &OnceCell<Mutex<Draw>>,
        proc : &ProcCollector,
        menu : &OnceCell<Mutex<Menu>>,
    ) {
        if self.parent.get_stat_mode() {
            return;
        }

        if proc.parent.get_proc_interrupt() {
            return;
        }

        if proc.parent.get_redraw() {
            self.redraw = true;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut n: u32 = 0;
        let mut x: u32 = self.parent.get_x() + 1;
        let mut y: u32 = self.current_y + 1;
        let mut w: u32 = u32::try_from(self.parent.get_width() as i32 - 2).unwrap_or(0);
        let mut h: u32 = u32::try_from(self.current_h as i32 - 2).unwrap_or(0);
        let mut prog_len: usize = 0;
        let mut arg_len: usize = 0;
        let mut val: u64 = 0;
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
            dy = self.parent.get_y() + 1;
        }

        if w > 67 {
            arg_len = (w
                - 53
                - if proc.num_procs > self.select_max as u32 {
                    1
                } else {
                    0
                }) as usize;
            prog_len = 15;
        } else {
            arg_len = 0;
            prog_len = (w
                - 38
                - if proc.num_procs > self.select_max as u32 {
                    1
                } else {
                    0
                }) as usize;

            if prog_len < 15 {
                tr_show = false;
                prog_len += 5;
            }
            if prog_len < 12 {
                usr_show = false;
                prog_len += 9;
            }
        }

        if CONFIG.get().unwrap().lock().unwrap().proc_tree {
            tree_len = arg_len + prog_len + 6;
            arg_len = 0;
        }

        // * Buttons and titles only redrawn if needed
        if self.parent.get_resized() || self.redraw {
            s_len += CONFIG.get().unwrap().lock().unwrap().proc_sorting.to_string().len();
            if self.parent.get_resized() || s_len != self.s_len || proc.detailed {
                self.s_len = s_len;
                for k in [
                    "e", "r", "c", "t", "k", "i", "enter", "left", " ", "f", "delete",
                ]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect::<Vec<String>>()
                {
                    if key.get().unwrap().lock().unwrap().mouse.contains_key(&k) {
                        key.get().unwrap().lock().unwrap().mouse.remove(&k);
                    }
                }
            }
            if proc.detailed {
                let mut killed: bool = match proc.details[&"killed".to_owned()] {
                    ProcCollectorDetails::Bool(b) => b,
                    _ => {
                        errlog("ProcCollectorDetails contained non-numeric value for 'killed'".to_owned());
                        false
                    },
                };
                let mut main: Color = if self.selected == 0 && !killed {
                    THEME.get().unwrap().lock().unwrap().colors.main_fg
                } else {
                    THEME.get().unwrap().lock().unwrap().colors.inactive_fg
                };
                let mut hi: Color = if self.selected == 0 && !killed {
                    THEME.get().unwrap().lock().unwrap().colors.hi_fg
                } else {
                    THEME.get().unwrap().lock().unwrap().colors.inactive_fg
                };
                let mut title: Color = if self.selected == 0 && !killed {
                    THEME.get().unwrap().lock().unwrap().colors.title
                } else {
                    THEME.get().unwrap().lock().unwrap().colors.inactive_fg
                };
                if self.current_y != self.parent.get_y() + 8
                    || self.parent.get_resized()
                    || graphs.get().unwrap().lock().unwrap().detailed_cpu.NotImplemented
                {
                    self.current_y = self.parent.get_y() + 8;
                    self.current_h = u32::try_from(self.parent.get_height() as i32 - 8).unwrap_or(0);
                    for i in 0..7 as u32 {
                        out_misc.push_str(
                            format!("{}{}", mv::to(dy + i, x), " ".repeat(w as usize)).as_str(),
                        );
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}{}{}{}{}{}{}{}{}{}",
                            mv::to(dy + 7, x - 1),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box,
                            symbol::title_right,
                            symbol::h_line.repeat(w as usize),
                            symbol::title_left,
                            mv::to(dy + 7, x + 1),
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            THEME.get().unwrap().lock().unwrap().colors.title.call(self.get_parent().get_name().clone(), term),
                            fx::ub,
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                            THEME.get().unwrap().lock().unwrap().colors.div_line,
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
                        THEME.get().unwrap().lock().unwrap().colors.proc_box,
                        symbol::left_up,
                        symbol::h_line.repeat(w as usize),
                        symbol::right_up,
                        mv::to(dy - 1, dgx + dgw + 1),
                        symbol::div_up,
                        mv::to(dy - 1, x + 1),
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .title
                            .call(proc.details[&"pid".to_owned()].to_string(), term),
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        THEME.get().unwrap().lock().unwrap().colors.title.call(
                            proc.details[&"name".to_owned()].to_string()[..dgw as usize - 11].to_owned(),
                            term
                        ),
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap()
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

                    key.get().unwrap().lock().unwrap().mouse.insert("enter".to_owned(), top.clone());
                }

                if self.selected == 0 && !killed {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((dx + 2) as i32 + i);
                        pusher.push(dy as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("t".to_owned(), top.clone());
                }

                out_misc.push_str(
                    format!(
                        "{}{}{}{}close{} {}{}{}{}{}{}{}t{}erminate{}{}",
                        mv::to(dy - 1, dx + dw - 11),
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        if self.selected > 0 {
                            title
                        } else {
                            THEME.get().unwrap().lock().unwrap().colors.title
                        },
                        fx::ub,
                        if self.selected > 0 {
                            main
                        } else {
                            THEME.get().unwrap().lock().unwrap().colors.main_fg
                        },
                        symbol::enter,
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                        mv::to(dy - 1, dx + 1),
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap()
                            .colors
                            .proc_box
                            .call(symbol::title_right.to_owned(), term),
                    )
                    .as_str(),
                );
                if dw > 28 {
                    if self.selected == 0 && !killed && !key.get().unwrap().lock().unwrap().mouse.contains_key(&"k".to_owned()) {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                        for i in 0..4 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push((dx + 13) as i32 + i);
                            pusher.push(dy as i32 - 1);
                            top.push(pusher);
                        }

                        key.get().unwrap().lock().unwrap().mouse.insert("k".to_owned(), top.clone());
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}k{}ill{}{}",
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                        )
                        .as_str(),
                    );
                }

                if dw > 39 {
                    if self.selected == 0 && !killed && !key.get().unwrap().lock().unwrap().mouse.contains_key(&"i".to_owned()) {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                        for i in 0..9 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push((dx + 19) as i32 + i);
                            pusher.push(dy as i32 - 1);
                            top.push(pusher);
                        }

                        key.get().unwrap().lock().unwrap().mouse.insert("i".to_owned(), top.clone());
                    }
                    out_misc.push_str(
                        format!(
                            "{}{}{}i{}nterrupt{}{}",
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME.get().unwrap().lock().unwrap()
                                .colors
                                .proc_box
                                .call(symbol::title_right.to_owned(), term),
                        )
                        .as_str(),
                    );
                }

                if graphs.get().unwrap().lock().unwrap().detailed_cpu.NotImplemented || self.parent.get_resized() {
                    graphs.get().unwrap().lock().unwrap().detailed_cpu = Graph::new(
                        (dgw + 1) as i32,
                        7,
                        Some(ColorSwitch::VecString(THEME.get().unwrap().lock().unwrap().gradient.get(&"cpu".to_owned()).unwrap().clone())),
                        proc.details_cpu.iter().map(|i| i.to_owned() as i32).collect(),
                        term,
                        false,
                        0,
                        0,
                        None,
                    );
                    graphs.get().unwrap().lock().unwrap().detailed_mem = Graph::new(
                        (dw / 3) as i32,
                        1,
                        None,
                        proc.details_mem.iter().map(|i| i.to_owned() as i32).collect(),
                        term,
                        false,
                        0,
                        0,
                        None
                    );
                }
                self.select_max = usize::try_from(self.parent.get_height() as i32 - 11).unwrap_or(0);
                y = u32::try_from(self.parent.get_y() as i32 + 9).unwrap_or(0);
                h = u32::try_from(self.parent.get_height() as i32 - 10).unwrap_or(0);
            } else {
                if self.current_y != self.parent.get_y() || self.parent.get_resized() {
                    self.current_y = self.parent.get_y();
                    self.current_h = self.parent.get_height();
                    y = self.parent.get_y() + 1;
                    h = u32::try_from(self.parent.get_height() as i32 - 2).unwrap_or(0);
                    out_misc.push_str(format!("{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
                            mv::to(y - 1, x - 1),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box,
                            symbol::left_up,
                            symbol::h_line.repeat(w as usize),
                            symbol::right_up,
                            mv::to(y - 1, x + 1),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                            fx::b,
                            THEME.get().unwrap().lock().unwrap().colors.title.call(self.get_parent().get_name().clone(), term),
                            fx::ub,
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                            mv::to(y + 7, x - 1),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::v_line.to_owned(), term),
                            mv::right(w),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::v_line.to_owned(), term),
                        )
                        .as_str()
                    );
                }
                self.select_max = usize::try_from(self.parent.get_height() as i32 - 3).unwrap_or(0);
            }

            sort_pos = (x + w) as usize - CONFIG.get().unwrap().lock().unwrap().proc_sorting.to_string().len() - 7;
            if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"left".to_owned()) {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..3 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push(sort_pos as i32 + i);
                    pusher.push(y as i32 - 1);
                    top.push(pusher);
                }

                key.get().unwrap().lock().unwrap().mouse.insert("left".to_owned(), top.clone());

                top = Vec::<Vec<i32>>::new();

                for i in 0..3 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push(sort_pos as i32 + CONFIG.get().unwrap().lock().unwrap().proc_sorting.to_string().len() as i32 + 3 + i);
                    pusher.push(y as i32 - 1);
                    top.push(pusher);
                }

                key.get().unwrap().lock().unwrap().mouse.insert("right".to_owned(), top.clone());
            }

            out_misc.push_str(format!("{}{}{}{}{}{}{} {} {}{}{}",
                    mv::to(y - 1, x + 8),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::h_line.repeat(w as usize - 9).to_owned(), term),
                    if !proc.detailed {
                        "".to_owned()
                    } else {
                        format!("{}{}", 
                            mv::to(dy + 7, dgx + dgw + 1),
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::div_down.to_owned(), term)
                        )
                    },
                    mv::to(y - 1, sort_pos as u32),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                    fx::b,
                    THEME.get().unwrap().lock().unwrap().colors.hi_fg.call("<".to_owned(), term),
                    THEME.get().unwrap().lock().unwrap().colors.title.call(CONFIG.get().unwrap().lock().unwrap().proc_sorting.to_string(), term),
                    THEME.get().unwrap().lock().unwrap().colors.hi_fg.call(">".to_owned(), term),
                    fx::ub,
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );

            if w > 29 + s_len as u32 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"e".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sort_pos - 5) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("e".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}",
                        mv::to(y - 1, sort_pos as u32 - 6),
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.get().unwrap().lock().unwrap().proc_tree {
                            fx::b
                        } else {
                            ""
                        },
                        THEME.get().unwrap().lock().unwrap().colors.title.call("tre".to_owned(), term),
                        THEME.get().unwrap().lock().unwrap().colors.hi_fg.call("e".to_owned(), term),
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if w > 37 + s_len as u32 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"r".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..7 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sort_pos - 14) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("r".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}",
                        mv::to(y - 1, sort_pos as u32 - 15),
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.get().unwrap().lock().unwrap().proc_reversed {
                            fx::b
                        } else {
                            ""
                        },
                        THEME.get().unwrap().lock().unwrap().colors.hi_fg.call("r".to_owned(), term),
                        THEME.get().unwrap().lock().unwrap().colors.title.call("everse".to_owned(), term),
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if w > 47 + s_len as u32 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"c".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0.. if proc.search_filter.len() == 0 {6} else {2 + proc.search_filter[(proc.search_filter.len() - 11)..].len()} {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((sort_pos - 24) as i32 + i as i32);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("c".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}{}",
                        mv::to(y - 1, sort_pos as u32 - 25),
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        if CONFIG.get().unwrap().lock().unwrap().proc_per_core {
                            fx::b
                        } else {
                            ""
                        },
                        THEME.get().unwrap().lock().unwrap().colors.title.call("per-".to_owned(), term),
                        THEME.get().unwrap().lock().unwrap().colors.hi_fg.call("c".to_owned(), term),
                        THEME.get().unwrap().lock().unwrap().colors.title.call("ore".to_owned(), term),
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }

            if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"f".to_owned()) || self.parent.get_resized() {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0.. if proc.search_filter.len() == 0 {6} else {2 + proc.search_filter[(proc.search_filter.len() - 11)..].len()} {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((x + 5) as i32 + i as i32);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("f".to_owned(), top.clone());
            }
            if proc.search_filter.len() > 0 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"delete".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..3 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push((x + 11 + proc.search_filter[(proc.search_filter.len() - 11)..].len() as u32) as i32 + i);
                        pusher.push(y as i32 - 1);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("delete".to_owned(), top.clone());
                }
            } else if key.get().unwrap().lock().unwrap().mouse.contains_key(&"delete".to_owned()) {
                key.get().unwrap().lock().unwrap().mouse.remove(&"delete".to_owned());
            }

            out_misc.push_str(format!("{}{}{}{}{}{}{}",
                    mv::to(y - 1, x + 7),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                    if self.filtering || proc.search_filter.len() > 0 {
                        fx::b
                    } else {
                        ""
                    },
                    THEME.get().unwrap().lock().unwrap().colors.hi_fg.call("f".to_owned(), term),
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    if proc.search_filter.len() == 0 && !self.filtering {
                        "ilter".to_owned()
                    } else {
                        let adder = if w < 83 {10} else {w as usize - 74};
                        let proc_insert : String = proc.search_filter[proc.search_filter.len() - 1 + adder..].to_owned();
                        format!(" {}{}",
                            proc_insert,
                            if self.filtering {
                                fx::bl.to_owned() + "█" + fx::ubl
                            } else {
                                THEME.get().unwrap().lock().unwrap().colors.hi_fg.call(" del".to_owned(), term).to_string()
                            }
                        )
                    },
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );

            let main : Color = if self.selected == 0 {
                THEME.get().unwrap().lock().unwrap().colors.inactive_fg
            } else {
                THEME.get().unwrap().lock().unwrap().colors.main_fg
            };
            let hi : Color = if self.selected == 0 {
                THEME.get().unwrap().lock().unwrap().colors.inactive_fg
            } else {
                THEME.get().unwrap().lock().unwrap().colors.hi_fg
            };
            let title : Color = if self.selected == 0 {
                THEME.get().unwrap().lock().unwrap().colors.inactive_fg
            } else {
                THEME.get().unwrap().lock().unwrap().colors.title
            };

            out_misc.push_str(format!("{}{}{}{}{}{}{} {}{} {}{}{}{}{}{}{}info {}{}{}{}",
                    mv::to(y + h, x + 1),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box,
                    symbol::h_line.repeat(w as usize - 4),
                    mv::to(y + h, x + 1),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                    main,
                    symbol::up,
                    fx::b,
                    THEME.get().unwrap().lock().unwrap().colors.main_fg.call("select".to_owned(), term),
                    fx::ub,
                    if self.selected == self.select_max {
                        THEME.get().unwrap().lock().unwrap().colors.inactive_fg
                    } else {
                        THEME.get().unwrap().lock().unwrap().colors.main_fg
                    },
                    symbol::down,
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                    title,
                    fx::b,
                    fx::ub,
                    main,
                    symbol::enter,
                    THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );
            if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"enter".to_owned()) {
                let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..6 {
                    let mut pusher: Vec<i32> = Vec::<i32>::new();
                    pusher.push((x + 14) as i32 + i);
                    pusher.push((y + h) as i32);
                    top.push(pusher);
                }

                key.get().unwrap().lock().unwrap().mouse.insert("enter".to_owned(), top.clone());
            }
            if w - loc_string.len() as u32 > 34 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"t".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 22 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("t".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}t{}erminate{}{}",
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }
            if w - loc_string.len() as u32 > 40 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"k".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..4 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 33 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("k".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}k{}ill{}{}",
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }
            if w - loc_string.len() as u32 > 51 {
                if !key.get().unwrap().lock().unwrap().mouse.contains_key(&"i".to_owned()) {
                    let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                    for i in 0..9 {
                        let mut pusher: Vec<i32> = Vec::<i32>::new();
                        pusher.push(x as i32 + 39 + i);
                        pusher.push((y + h) as i32);
                        top.push(pusher);
                    }

                    key.get().unwrap().lock().unwrap().mouse.insert("i".to_owned(), top.clone());
                }
                out_misc.push_str(format!("{}{}{}i{}terrupt{}{}",
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                        fx::b,
                        hi,
                        title,
                        fx::ub,
                        THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                    )
                    .as_str()
                );
            }
            if CONFIG.get().unwrap().lock().unwrap().proc_tree && w - loc_string.len() as u32 > 65 {
                if w - loc_string.len() as u32 > 40 {
                    if !key.get().unwrap().lock().unwrap().mouse.contains_key(&" ".to_owned()) {
                        let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
    
                        for i in 0..12 {
                            let mut pusher: Vec<i32> = Vec::<i32>::new();
                            pusher.push(x as i32 + 50 + i);
                            pusher.push((y + h) as i32);
                            top.push(pusher);
                        }
    
                        key.get().unwrap().lock().unwrap().mouse.insert(" ".to_owned(), top.clone());
                    }
                    out_misc.push_str(format!("{}{}{}spc {}collapse{}{}",
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_left.to_owned(), term),
                            fx::b,
                            hi,
                            title,
                            fx::ub,
                            THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
                        )
                        .as_str()
                    );
                }
            }

            // * Processes labels
            let mut selected : String = String::default();
            let mut label : String = String::default();
            selected = match CONFIG.get().unwrap().lock().unwrap().proc_sorting {
                SortingOption::Memory => String::from("mem"),
                SortingOption::Threads => if !CONFIG.get().unwrap().lock().unwrap().proc_tree && arg_len == 0 {
                        String::from("tr")
                    } else {
                        String::default()
                    },
                _ => {
                    errlog("Wrong sorting option in CONFIG.get().unwrap().lock().unwrap().proc_sorting when processing lables...".to_owned());
                    String::default()
                },
            };

            if CONFIG.get().unwrap().lock().unwrap().proc_tree {
                label = format!("{}{}{}{:<width$}{}{}Mem%{:>11}{}{} {}",
                    THEME.get().unwrap().lock().unwrap().colors.title,
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
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    width = tree_len - 2,
                );
                if ["pid", "program", "arguments"].iter().map(|s| s.to_owned().to_owned()).collect::<Vec<String>>().contains(&selected) {
                    selected = String::from("tree");
                }
            } else {
                label = format!("{}{}{}{:>7} {}{}{}{}Mem%{:>11}{}{} {}",
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    fx::b,
                    mv::to(y, x),
                    "Pid:",
                    if prog_len > 8 {
                        "Program:".to_owned()
                    } else {
                        format!("{:<width$}", "Prg:", width = prog_len)
                    },
                    if arg_len > 0 {
                        format!("{:<width$}", "Arguments:", width = arg_len - 4)
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
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
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

            selected = selected.split(" ").map(|s| s.to_owned().to_owned()).collect::<Vec<String>>()[0].to_title_case();
            if CONFIG.get().unwrap().lock().unwrap().proc_mem_bytes {
                label = label.replace("Mem%", "MemB");
            }
            label = label.replace(selected.as_str(), format!("{}{}{}", fx::u, selected, fx::uu).as_str());
            out_misc.push_str(label.as_str());
            draw.get().unwrap().lock().unwrap().buffer("proc_misc".to_owned(), vec![out_misc.clone()], false, false, 100, true, false, false, key);
        }

        // * Detailed box draw
        if proc.detailed {
            let mut stat_color : String = match proc.details[&"status".to_owned()] {
                ProcCollectorDetails::Status(s) => match s {
                    Status::Running => fx::b.to_owned(),
                    Status::Dead => THEME.get().unwrap().lock().unwrap().colors.inactive_fg.to_string(),
                    Status::Stopped => THEME.get().unwrap().lock().unwrap().colors.inactive_fg.to_string(),
                    Status::Zombie => THEME.get().unwrap().lock().unwrap().colors.inactive_fg.to_string(),
                    _ => String::default(),
                },
                _ => {
                    errlog("Wrong ProcCollectorDetails type when assigning stat_color".to_owned());
                    String::default()
                },
            };
            let expand : u32 = proc.expand;
            let iw : u32 = (dw - 3) / (4 + expand);
            let iw2 : u32 = iw - 1;

            out.push_str(format!("{}{}{}{}{}%{}{}{}",
                    mv::to(dy, dgx),
                    graphs.get().unwrap().lock().unwrap().detailed_cpu.call(
                        if self.moved || match proc.details[&"killed".to_owned()] {
                            ProcCollectorDetails::Bool(b) => b,
                            _ => {
                                errlog("Wrong ProcCollectorDetails type from proc.details['killed']".to_owned());
                                false
                            },
                        } {
                            None
                        } else {
                            Some(proc.details_cpu[proc.details_cpu.len() - 2] as i32)
                        }, 
                        term
                    ),
                    mv::to(dy , dgx),
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    fx::b,
                    if match proc.details[&"killed".to_owned()] {
                        ProcCollectorDetails::Bool(b) => b,
                        _ => {
                            errlog("Wrong ProcCollectorDetails type from proc.details['killed']".to_owned());
                            false
                        },
                    } {
                        0
                    } else {
                        match proc.details[&"cpu_percent".to_owned()] {
                            ProcCollectorDetails::U32(u) => u,
                            _ => {
                                errlog("Wrong ProcCollectorDetails type from proc.details['cpu_percent']".to_owned());
                                0
                            },
                        }
                    },
                    mv::right(1),
                    (if SYSTEM.to_owned() == "MacOS".to_owned() {
                        ""
                    } else {
                        if dgw < 20 {
                            "C"
                        } else {
                            "Core"
                        }
                    }).to_owned() + proc.details[&"cpu_name".to_owned()].to_string().as_str(),
                )
                .as_str()
            );

            for (i, l) in vec!["C", "P", "U"].iter().map(|s| s.to_owned().to_owned()).enumerate() {
                out.push_str(format!("{}{}", mv::to(dy + 2 + i as u32, dgx), l).as_str());
            }
            for (i, l) in vec!["C", "M", "D"].iter().map(|s| s.to_owned().to_owned()).enumerate() {
                out.push_str(format!("{}{}", mv::to(dy + 4 + i as u32, dx + 1), l).as_str());
            }


            let inserter : String = proc.details[&"terminal".to_owned()].to_string()[(proc.details[&"terminal".to_owned()].to_string().len() - 1 - iw2 as usize)..].to_owned();
            let expand_4 = format!("{:^first$.second$}", inserter, first = iw as usize, second = iw2 as usize);
            
            
            out.push_str(format!("{} {}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{} {}{}{}{} {}{}{}{}{}{}{}{}{}{}",
                    mv::to(dy, dx + 1),
                    format!("{:^first$.second$}", "Status:", first = iw as usize, second = iw2 as usize),
                    format!("{:^first$.second$}", "Elapsed:", first = iw as usize, second = iw2 as usize),
                    if dw > 28 {
                        format!("{:^first$.second$}", "Parent:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if dw > 38 {
                        format!("{:^first$.second$}", "User:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 0 {
                        format!("{:^first$.second$}", "Threads:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 1 {
                        format!("{:^first$.second$}", "Nice:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 2 {
                        format!("{:^first$.second$}", "IO Read:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 3 {
                        format!("{:^first$.second$}", "IO Write:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 4 {
                        format!("{:^first$.second$}", "TTY:", first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    mv::to(dy + 3, dx),
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    fx::ub,
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    stat_color,
                    proc.details[&"status".to_owned()],
                    fx::ub,
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    proc.details[&"uptime".to_owned()],
                    if dw > 28 {
                        format!("{:^first$.second$}", proc.details[&"parent_name".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if dw > 38 {
                        format!("{:^first$.second$}", proc.details[&"username".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 0 {
                        format!("{:^first$.second$}", proc.details[&"threads".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 1 {
                        format!("{:^first$.second$}", proc.details[&"nice".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 2 {
                        format!("{:^first$.second$}", proc.details[&"io_read".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 3 {
                        format!("{:^first$.second$}", proc.details[&"io_write".to_owned()], first = iw as usize, second = iw2 as usize)
                    } else {
                        String::default()
                    },
                    if expand > 4 {
                        expand_4
                    } else {
                        String::default()
                    },
                    mv::to(dy + 3, dx),
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    fx::b,
                    format!("{:>width$}",
                        (if dw > 42 {
                            "Memory: "
                        } else {
                            "M:"
                        }).to_owned() + proc.details["memory_percent"].to_string().as_str() + "%",
                        width = (dw as usize / 3) - 1,
                    ),
                    fx::ub,
                    THEME.get().unwrap().lock().unwrap().colors.inactive_fg,
                    ". ".repeat(dw as usize / 3),
                    mv::left(dw / 3),
                    THEME.get().unwrap().lock().unwrap().colors.proc_misc,
                    graphs.get().unwrap().lock().unwrap().detailed_mem.call(
                        if self.moved 
                        {
                            None
                        } else {
                            Some(
                                match proc.details[&"memory_percent".to_owned()] {
                                    ProcCollectorDetails::Bool(b) => if b {1} else {0},
                                    ProcCollectorDetails::U32(u) => u as i32,
                                    ProcCollectorDetails::F32(f) => f as i32,
                                    ProcCollectorDetails::F64(f) => f as i32,
                                    ProcCollectorDetails::U64(u) => u as i32,
                                    _ => {
                                        errlog("ProcCollectorDetails contained non-numeric value for 'memory_percent'".to_owned());
                                        0
                                    }
                                }
                            )
                        }, 
                        term
                    ),
                    THEME.get().unwrap().lock().unwrap().colors.title,
                    fx::b,
                    format!("{:.width$}", proc.details[&"memory_bytes".to_owned()], width = (dw as usize / 3) - 2),
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    fx::ub,
                )
                .as_str()
            );

            let cmdline : String = match proc.details.get(&"cmdline".to_owned()).unwrap() {
                ProcCollectorDetails::String(s) => s.clone(),
                ProcCollectorDetails::VecString(v) => v.clone().join(", ").clone(),
                _ => {
                    errlog("Wrong type in proc.details['cmdline']".to_owned());
                    String::default()
                },
            };
            let cmdline_len : u32 = cmdline.len() as u32;

            let cy = dy + if cmdline_len > dw - 5 {
                4
            } else {
                5
            };
            for i in 0..(cmdline_len / (dw - 5)) {
                if i == 0 {
                    let to_insert : String = if dw as i32 - 5 >= 0 {
                        let first = cmdline[((dw-5)*i) as usize..].to_owned();
                        let second = first[..(dw-5) as usize].to_owned();
                        second
                    } else {
                        let first = cmdline[((cmdline_len - 1 - 5) * i) as usize..].to_owned();
                        let second = first[..(cmdline_len - 1 - 5) as usize].to_owned();
                        second
                    };
                    out.push_str(format!("{}{}",
                        mv::to(cy + i, dx + 3),
                        format!("{:^width$}",
                            to_insert,
                            width = dw as usize- 5,
                        ),
                    )
                    .as_str()
                );
                } else {
                    let inserter : String = if dw as i32 - 5 >= 0 {
                        let first = cmdline[((dw-5)*i) as usize..].to_owned();
                        let second = first[..(dw-5) as usize].to_owned();
                        second
                    } else {
                        let first = cmdline[((cmdline_len - 1 - 5) * i) as usize..].to_owned();
                        let second = first[..(cmdline_len - 1 - 5) as usize].to_owned();
                        second
                    };
                    out.push_str(format!("{}{}",
                        mv::to(cy + i, dx + 3),
                        format!("{:<width$}",
                            inserter,
                            width = dw as usize - 5,
                        ),
                    )
                    .as_str()
                );
                }
                if i == 0 {
                    let formatter : String = if dw - 5 >= 0 {
                        cmdline[((dw-5)*i) as usize ..].to_owned()[..(dw-5) as usize].to_owned()
                    } else {
                        cmdline[((cmdline_len - 1 - 5) * i) as usize ..].to_owned()[..(cmdline_len - 1 - 5) as usize].to_owned()
                    };
                    let to_insert : String = format!("{:^width$}",
                    formatter,
                    width = (dw - 5) as usize,
                );
                    out.push_str(format!("{}{}",
                        mv::to(cy + i, dx + 3),
                        to_insert,
                    )
                    .as_str()
                );
                }
                let to_insert : String = if dw as i32 - 5 >= 0 {
                    let first =cmdline[((dw-5)*i) as usize..].to_owned();
                    let second = first[..(dw-5) as usize].to_owned();
                    second
                } else {
                    let first = cmdline[((cmdline_len - 1 - 5) * i) as usize..].to_owned();
                    let second = first[..(cmdline_len - 1 - 5) as usize].to_owned();
                    second
                };
                out.push_str(format!("{}{}",
                        mv::to(cy + i, dx + 3),
                        format!("{:<width$}",
                            to_insert,
                            width = dw as usize - 5,
                        ),
                    )
                    .as_str()
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
        let mut cy: u32 = 1;

        for (n, (pid, items)) in proc.processes.iter().enumerate() {
            if (n as i32) < self.start {
                continue;
            }
            l_count += 1;
            if l_count == self.selected {
                is_selected = true;
                self.selected_pid = pid.clone();
            } else {
                is_selected = false;
            }

            let mut indent = match items.get(&"indent".to_owned()).unwrap() {
                ProcessInfo::String(s) => s.clone(),
                _ => {
                    errlog("Malformed type in items['indent']".to_owned());
                    String::default()
                }
            };

            let name = match items.get(&"name".to_owned()).unwrap() {
                ProcessInfo::String(s) => s.clone(),
                _ => {
                    errlog("Malformed type in items['name']".to_owned());
                    String::default()
                }
            };

            let mut cmd = match items.get(&"cmd".to_owned()).unwrap() {
                ProcessInfo::String(s) => s.clone(),
                _ => {
                    errlog("Malformed type in items['cmd']".to_owned());
                    String::default()
                }
            };

            let threads : u64 = match items.get(&"threads".to_owned()).unwrap() {
                ProcessInfo::U64(u) => u.clone(),
                ProcessInfo::Count(u) => u.clone(),
                _ => {
                    errlog("Malformed type in items['threads']".to_owned());
                    0
                }
            };

            let username : String = match items.get(&"username".to_owned()).unwrap() {
                ProcessInfo::String(s) => s.clone(),
                _ => {
                    errlog("Malformed type in items['username']".to_owned());
                    String::default()
                }
            };

            let mem : MemoryInfo = match items.get(&"mem".to_owned()).unwrap() {
                ProcessInfo::MemoryInfo(m) => m.clone(),
                _ => {
                    errlog("Malformed type in items['mem']".to_owned());
                    return;
                }
            };

            let mem_b : Bytes = match items.get(&"mem_b".to_owned()).unwrap() {
                ProcessInfo::U64(u) => u.clone(),
                ProcessInfo::Count(u) => u.clone(),
                _ => {
                    errlog("Malformed type in items['mem_b']".to_owned());
                    0
                }
            };

            let cpu : f32 = match items.get(&"cpu".to_owned()).unwrap() {
                ProcessInfo::F32(f) => f.clone(),
                _ => {
                    errlog("Malformed type in items['cpu']".to_owned());
                    0.0
                }
            };

            if CONFIG.get().unwrap().lock().unwrap().proc_tree {
                arg_len = 0;
                let size_set = format!("{}{}", indent, pid).len();
                offset = size_set as u32;
                tree_len = size_set;

                indent = format!("{:.width$}", indent, width = tree_len - pid.to_string().len());
                if offset - name.len() as u32 > 12 {
                    let cmd_splitter = cmd.split(" ").map(|s| s.to_owned()).collect::<Vec<String>>()[0].split("/").map(|s| s.to_owned()).collect::<Vec<String>>();
                    cmd = cmd_splitter[cmd.len() - 2].clone();
                    if !cmd.starts_with(name.as_str()) {
                        offset = name.len() as u32;
                        arg_len = (tree_len - format!("{}{} {} ", indent, pid, name).len() + 2) as usize;
                        let set_cmd : String = cmd[..arg_len - 4].to_owned();
                        cmd = format!("({})", set_cmd);
                    }
                }
            } else {
                offset = prog_len as u32 - 1;
            }
            if cpu > 1.0 || graphs.get().unwrap().lock().unwrap().pid_cpu.contains_key(pid) {
                if !graphs.get().unwrap().lock().unwrap().pid_cpu.contains_key(pid) {
                    graphs.get().unwrap().lock().unwrap().pid_cpu.insert(pid.to_owned(),Graph::new(5, 1, None, vec![0], term, false, 0, 0, None));
                    self.pid_counter.insert(pid.to_owned(), 0);
                } else if cpu < 1.0 {
                    let mut pcp : u32 = self.pid_counter.get(pid).unwrap().to_owned();
                    self.pid_counter.insert(pid.to_owned(), pcp + 1);
                    if self.pid_counter[pid] > 10 {
                        self.pid_counter.remove(pid);
                        graphs.get().unwrap().lock().unwrap().pid_cpu.remove(pid);
                    }
                } else {
                    self.pid_counter.insert(pid.to_owned(), 0);
                }
            }

            end = if CONFIG.get().unwrap().lock().unwrap().proc_colors {
                format!("{}{}", THEME.get().unwrap().lock().unwrap().colors.main_fg, fx::ub)
            } else {
                fx::ub.to_owned()
            };
            if self.selected as u32 > cy {
                calc = self.selected as u32 - cy;
            } else if 0 < self.selected && self.selected as u32 <= cy {
                calc = cy - self.selected as u32;
            } else {
                calc = cy;
            }
            if CONFIG.get().unwrap().lock().unwrap().proc_colors && !is_selected {
                vals = Vec::<String>::new();
                for v in vec![cpu as u64, mem.rss(), (threads / 3)] {
                    if CONFIG.get().unwrap().lock().unwrap().proc_gradient {
                        val = (if v <= 100 {
                            v
                        } else {
                            100
                        } + 100) - calc as u64 * 100 / self.select_max as u64;
                        vals.push(
                            THEME.get().unwrap().lock().unwrap().gradient[
                                &(if v < 100 {
                                    "proc_color".to_owned()
                                } else {
                                    "process".to_owned()
                                })
                            ][
                                if val < 100 {
                                    val
                                } else {
                                    val - 100
                                } as usize
                            ]
                            .clone()
                        );
                    } else {
                        vals.push(
                            THEME.get().unwrap().lock().unwrap().gradient.get(&"process".to_owned()).unwrap().get(
                                if v <= 100 {
                                    v
                                } else {
                                    100
                                } as usize
                            ).unwrap().clone()
                        );
                    }
                }
                c_color = vals.join(" ");
                m_color = vals.join(" ");
                t_color = vals.join(" ");
            } else {
                c_color = fx::b.to_owned();
                m_color = fx::b.to_owned();
                t_color = fx::b.to_owned();
            }
            if CONFIG.get().unwrap().lock().unwrap().proc_gradient && !is_selected {
                g_color = THEME.get().unwrap().lock().unwrap().gradient[&"proc".to_owned()][calc as usize * 100 / self.select_max].clone();
            }
            if is_selected {
                c_color = String::default();
                m_color = String::default();
                t_color = String::default();
                g_color = String::default();
                end = String::default();
                out.push_str(format!("{}{}{}", THEME.get().unwrap().lock().unwrap().colors.selected_bg, THEME.get().unwrap().lock().unwrap().colors.selected_fg, fx::b).as_str());
            }

            // * Creates one line for a process with all gathered information
            out.push_str(format!("{}{}{}{:>width$} {}{:<offset1$.offset2$} {}{}{}{}{}{}{}",
                    mv::to(y + cy, x),
                    g_color,
                    indent,
                    pid,
                    c_color,
                    name,
                    end,
                    if arg_len > 0 {
                        format!("{}{:<arg_len1$.arg_len2$}", g_color, cmd, arg_len1 = arg_len, arg_len2 = arg_len - 1)
                    } else {
                        String::default()
                    },
                    if tr_show {
                        t_color + if threads < 1000 {
                            format!("{:>4}", threads)
                        } else {
                            "999> ".to_owned()
                        }.as_str() + end.as_str()
                    } else {
                        String::default()
                    },
                    if usr_show {
                        g_color.clone() + if username.len() < 10 {
                            format!("{:<9.9}", username)
                        } else {
                            let insert : String = username[..8].to_owned();
                            format!("{:<8}", insert)
                        }.as_str()
                    } else {
                        String::default()
                    },
                    m_color + (
                        if !CONFIG.get().unwrap().lock().unwrap().proc_mem_bytes {
                            if mem.rss() < 100 {
                                format!("{mem:>width$.*}", 1, mem = mem.rss(), width = 4)
                            } else {
                                format!("{mem:>width$.*}", 0, mem = mem.rss(), width = 4)
                            }
                        } else {
                            format!("{:>4.4}", floating_humanizer(mem_b as f64, true, false, 0, false))
                        }
                    ).as_str() + end.as_str(),
                    format!(" {}{}{}{}{}", THEME.get().unwrap().lock().unwrap().colors.inactive_fg, ".".repeat(5), THEME.get().unwrap().lock().unwrap().colors.main_fg, g_color, c_color) + if cpu < 100.0 {
                        format!(" {cpu:>width$.*} ",1, cpu = cpu, width = 4)
                    } else {
                        format!("{cpu:>width$.*} ", 0, cpu = cpu, width = 5)
                    }.as_str() + end.as_str(),
                    if proc.num_procs > self.select_max as u32 {
                        " "
                    } else {
                        ""
                    },
                    width = if CONFIG.get().unwrap().lock().unwrap().proc_tree {
                        1
                    } else {
                        7
                    },
                    offset1 = offset as usize,
                    offset2 = offset as usize,
                )
                .as_str()
            );

            // * Draw small cpu graph for process if cpu usage was above 1% in the last 10 updates
            if graphs.get().unwrap().lock().unwrap().pid_cpu.contains_key(&pid) {
                out.push_str(format!("{}{}{}{}",
                        mv::to(y + cy, x + w - if proc.num_procs > self.select_max as u32 {
                            12
                        } else {
                            11
                        }),
                        if CONFIG.get().unwrap().lock().unwrap().proc_colors {
                            c_color
                        } else {
                            THEME.get().unwrap().lock().unwrap().colors.proc_misc.to_string()
                        },
                        graphs.get().unwrap().lock().unwrap().pid_cpu.get_mut(&pid.clone()).unwrap().call(if self.moved {
                            None
                        } else {
                            Some(cpu.round() as i32)
                        }, term),
                        THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    )
                    .as_str()
                );
            }

            if is_selected {
                out.push_str(format!("{}{}{}{}{}",
                        fx::ub,
                        term.get().unwrap().lock().unwrap().get_fg(),
                        term.get().unwrap().lock().unwrap().get_bg(),
                        mv::to(y + cy, x + w - 1),
                        if proc.num_procs > self.select_max as u32 {
                            " "
                        } else {
                            ""
                        },
                    )
                    .as_str()
                );
            }

            cy += 1;
            if cy == h {
                break;
            }
        }
        if cy < h {
            for i in 0..h-cy {
                out.push_str(format!("{}{}", mv::to(y + cy + i, x), " ".repeat(w as usize)).as_str())
            }
        }

        // * Draw scrollbar if needed
        if proc.num_procs > self.select_max as u32 {
            if self.parent.get_resized() {
                match key.get().unwrap().lock().unwrap().mouse.get(&"mouse_scroll_up".to_owned()) {
                    Some(_) => {
                        let mut top = Vec::<Vec<i32>>::new();
                        for i in 0..3 {
                            let mut adder = Vec::<i32>::new();
                            adder.push((x + w - 2 + i) as i32);
                            adder.push(y as i32);
                            top.push(adder);
                        }
                        key.get().unwrap().lock().unwrap().mouse.insert("mouse_scroll_up".to_owned(), top.clone());
                    },
                    None => {
                        errlog("key.mouse does not have 'mouse_scroll_up'!".to_owned());
                        ()
                    }
                };
                match key.get().unwrap().lock().unwrap().mouse.get_mut(&"mouse_scroll_down".to_owned()) {
                    Some(v) => {
                        let mut top = Vec::<Vec<i32>>::new();
                        for i in 0..3 {
                            let mut adder = Vec::<i32>::new();
                            adder.push((x + w - 2 + i) as i32);
                            adder.push((y + h - 1) as i32);
                            top.push(adder);
                        }
                        key.get().unwrap().lock().unwrap().mouse.insert("mouse_scroll_down".to_owned(), top.clone());
                    },
                    None => {
                        errlog("key.mouse does not have 'mouse_scroll_down'!".to_owned());
                        ()
                    }
                };
            }
            scroll_pos = (self.start * (self.select_max as i32 - 2) / (proc.num_procs - (self.select_max as u32 - 2)) as i32) as u32;
            if scroll_pos > h - 3 || self.start >= (proc.num_procs - self.select_max as u32) as i32 {
                scroll_pos = h - 3;
            }
            out.push_str(format!("{}{}{}↑{}↓{}{}█",
                    mv::to(y, x + w - 1),
                    fx::b,
                    THEME.get().unwrap().lock().unwrap().colors.main_fg,
                    mv::to(y + h - 1, x + w - 1),
                    fx::ub,
                    mv::to(y + 1 + scroll_pos, x + w - 1),
                )
                .as_str()
            );
        } else if key.get().unwrap().lock().unwrap().mouse.contains_key(&"scroll_up".to_owned()) {
            key.get().unwrap().lock().unwrap().mouse.remove(&"scroll_up".to_owned());
            key.get().unwrap().lock().unwrap().mouse.remove(&"scroll_down".to_owned());
        }

        // * Draw current selection and number of processes
        out.push_str(format!("{}{}{}{}{}{}{}{}",
                mv::to(y + h, x + w - 3 - loc_string.len() as u32),
                THEME.get().unwrap().lock().unwrap().colors.proc_box,
                symbol::title_left,
                THEME.get().unwrap().lock().unwrap().colors.title,
                fx::b,
                loc_string,
                fx::ub,
                THEME.get().unwrap().lock().unwrap().colors.proc_box.call(symbol::title_right.to_owned(), term),
            )
            .as_str()
        );

        // * Clean up dead processes graphs and counters
        self.count += 1;
        if self.count == 100 {
            self.count = 0;
            for (pid, _) in self.pid_counter.clone() {
                if !psutil::process::pid_exists(pid) {
                    self.pid_counter.remove(&pid);
                    graphs.get().unwrap().lock().unwrap().pid_cpu.remove(&pid);
                }
            }
        }

        draw.get().unwrap().lock().unwrap().buffer(self.buffer.clone(), vec![format!("{}{}{}", out_misc.clone(), out, term.get().unwrap().lock().unwrap().get_fg())], false, false, 100, menu.get().unwrap().lock().unwrap().active, false, false, key);
        self.redraw = false;
        self.parent.set_resized(false);
        self.moved = false;
    }

    pub fn get_parent(&self) -> BrshtopBox {
        self.parent.clone()
    }

    pub fn set_parent(&mut self, parent : BrshtopBox) {
        self.parent = parent.clone()
    }

    pub fn set_parent_name(&mut self, name : String) {
        self.parent.set_name(name.clone())
    }

    pub fn set_parent_x(&mut self, x : u32) {
        self.parent.set_x(x.clone())
    }

    pub fn set_parent_y(&mut self, y : u32) {
        self.parent.set_y(y.clone())
    }

    pub fn set_parent_height_p(&mut self, height_p : u32) {
        self.parent.set_height_p(height_p.clone())
    }

    pub fn set_parent_width_p(&mut self, width_p : u32) {
        self.parent.set_width_p(width_p.clone())
    }

    pub fn set_parent_resized(&mut self, resized : bool) {
        self.parent.set_resized(resized.clone())
    }

    pub fn get_current_y(&self) -> u32 {
        self.current_y.clone()
    }

    pub fn get_current_h(&self) -> u32 {
        self.current_h.clone()
    }

    pub fn set_current_h(&mut self, current_h : u32) {
        self.current_h = current_h.clone()
    }

    pub fn get_select_max(&self) -> usize {
        self.select_max.clone()
    }

    pub fn set_select_max(&mut self, select_max : usize) {
        self.select_max = select_max.clone()
    }

    pub fn get_selected(&self) -> usize {
        self.selected.clone()
    }

    pub fn set_selected(&mut self, selected : usize) {
        self.selected = selected.clone()
    }

    pub fn get_selected_pid(&self) -> u32 {
        self.selected_pid.clone()
    }

    pub fn set_selected_pid(&mut self, selected_pid : u32) {
        self.selected_pid = selected_pid.clone()
    }

    pub fn get_last_selection(&self) -> usize {
        self.last_selection.clone()
    }

    pub fn set_last_selection(&mut self, last_selection : usize) {
        self.last_selection = last_selection.clone()
    }

    pub fn get_filtering(&self) -> bool {
        self.filtering.clone()
    }

    pub fn set_filtering(&mut self, filtering : bool) {
        self.filtering = filtering.clone()
    }

    pub fn get_moved(&self) -> bool {
        self.moved.clone()
    }

    pub fn set_moved(&mut self, moved : bool) {
        self.moved = moved.clone();
    }

    pub fn get_start(&self) -> i32 {
        self.start.clone()
    }

    pub fn set_start(&mut self, start : i32) {
        self.start = start.clone()
    }

    pub fn get_count(&self) -> i32 {
        self.count.clone()
    }

    pub fn set_count(&mut self, count : i32) {
        self.count = count.clone()
    }

    pub fn get_s_len(&self) -> usize {
        self.s_len.clone()
    }

    pub fn set_s_len(&mut self, s_len : usize) {
        self.s_len = s_len.clone()
    }

    pub fn get_detailed(&self) -> bool {
        self.detailed.clone()
    }

    pub fn set_detailed(&mut self, detailed : bool) {
        self.detailed = detailed.clone()
    }

    pub fn get_detailed_x(&self) -> u32 {
        self.detailed_x.clone()
    }

    pub fn set_detailed_x(&mut self, detailed_x : u32) {
        self.detailed_x = detailed_x.clone()
    }

    pub fn get_detailed_y(&self) -> u32 {
        self.detailed_y.clone()
    }

    pub fn set_detailed_y(&mut self, detailed_y : u32) {
        self.detailed_y = detailed_y.clone()
    }

    pub fn get_detailed_width(&self) -> u32 {
        self.detailed_width.clone()
    }

    pub fn set_detailed_width(&mut self, detailed_width : u32) {
        self.detailed_width = detailed_width.clone()
    }

    pub fn get_detailed_height(&self) -> u32 {
        self.detailed_height.clone()
    }

    pub fn set_detaied_height(&mut self, detailed_height : u32) {
        self.detailed_height = detailed_height.clone()
    }

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer : String) {
        self.buffer = buffer.clone()
    }

    pub fn get_pid_counter(&self) -> HashMap<u32, u32> {
        self.pid_counter.clone()
    }

    pub fn set_pid_counter(&mut self, pid_counter : HashMap<u32, u32>) {
        self.pid_counter = pid_counter.clone()
    }

    pub fn get_pid_counter_index(&self, index : u32) -> Option<u32> {
        match self.get_pid_counter().get(&index.clone()) {
            Some(u) => Some(u.to_owned()),
            None => None,
        }
    }

    pub fn set_pid_counter_index(&mut self, index : u32, element : u32) {
        self.pid_counter.insert(index.clone(), element.clone());
    }

    pub fn get_redraw(&self) -> bool {
        self.redraw.clone()
    }

    pub fn set_redraw(&mut self, redraw : bool) {
        self.redraw = redraw.clone()
    }

}

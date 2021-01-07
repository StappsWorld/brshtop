use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox},
        collector::{Collector, Collectors},
        config::{Config, ViewMode},
        create_box,
        key::Key,
        proccollector::ProcCollector,
        term::Term,
        theme::Theme,
    },
    std::{
        path::*,
        collections::HashMap,
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
    resized: bool,
    redraw: bool,
    buffer: String,
    pid_counter: HashMap<i32, i32>,
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
            resized: true,
            redraw: true,
            buffer: "proc".to_owned(),
            pid_counter: HashMap::<i32, i32>::new(),
        };
        procbox.parent.x = 1;
        procbox.parent.y = 1;
        procbox.parent.height_p = 68;
        procbox.parent.width_p = 55;
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
        self.resized = true;
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
    pub fn selector<P: AsRef<Path>>(
        &mut self,
        key: String,
        mouse_pos: (i32, i32),
        proc_collector: &mut ProcCollector,
        key_class: &mut Key,
        collector: &mut Collector,
        CONFIG : &mut Config,
        CONFIG_DIR : P,
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
        if self.selected as u32 > proc_collector.num_procs && proc_collector.num_procs < self.select_max as u32 {
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
                CONFIG_DIR,
                true,
                false,
                true,
                true,
                true,
            );
        }
    }

    pub fn draw_fg(&mut self) {
        if self.parent.stat_mode {
            return;
        }
        // TODO : Fix ProcCollector initialization
        let mut proc = ProcCollector::new(self.buffer.clone());

        if proc.parent.proc_interrupt {
            return;
        }

    }


}

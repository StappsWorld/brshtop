use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox, SubBoxes},
        config::{Config, ViewMode},
        cpucollector::CpuCollector,
        create_box, error, fx,
        graph::{Graph, Graphs},
        key::Key,
        menu::Menu,
        mv, readfile,
        subbox::SubBox,
        symbol,
        term::Term,
        theme::{Color, Theme},
    },
    battery::{
        units::{ratio::percent, time::second},
        *,
    },
    math::round::ceil,
    std::{collections::HashMap, fs::File, path::Path},
};

pub struct CpuBox {
    pub parent: BrshtopBox,
    pub sub: SubBox,
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub height_p: u32,
    pub width_p: u32,
    pub resized: bool,
    pub redraw: bool,
    pub buffer: String,
    pub battery_percent: f32,
    pub battery_secs: f32,
    pub battery_status: String,
    pub old_battery_pos: u32,
    pub old_battery_len: usize,
    pub battery_path: Option<String>,
    pub battery_clear: bool,
    pub battery_symbols: HashMap<String, String>,
    pub clock_block: bool,
}
impl CpuBox {
    pub fn new(brshtop_box: &mut BrshtopBox, config: &mut Config, ARG_MODE: ViewMode) -> Self {
        let mut bsm: HashMap<String, String> = HashMap::<String, String>::new();
        bsm.insert("Charging".to_owned(), "▲".to_owned());
        bsm.insert("Discharging".to_owned(), "▼".to_owned());
        bsm.insert("Full".to_owned(), "■".to_owned());
        bsm.insert("Not charging".to_owned(), "■".to_owned());

        let buffer_mut: String = "cpu".to_owned();

        brshtop_box.buffers.push(buffer_mut.clone());

        CpuBox {
            parent: BrshtopBox::new(config, ARG_MODE),
            sub: SubBox::new(),
            name: "cpu".to_owned(),
            x: 1,
            y: 1,
            height_p: 32,
            width_p: 100,
            resized: true,
            redraw: false,
            buffer: buffer_mut.clone(),
            battery_percent: 1000.0,
            battery_secs: 0.0,
            battery_status: "Unknown".to_owned(),
            old_battery_pos: 0,
            old_battery_len: 0,
            battery_path: Some("".to_owned()),
            battery_clear: false,
            battery_symbols: bsm.clone(),
            clock_block: true,
        }
    }

    pub fn calc_size(&mut self, THREADS: u64, term: &mut Term, brshtop_box: &mut BrshtopBox) {
        let cpu: CpuCollector = CpuCollector::new(THREADS);
        let mut height_p: u32 = 0;
        height_p = if self.parent.proc_mode {
            20
        } else {
            self.height_p
        };

        self.parent.width = (term.width as u32 * self.width_p / 100) as u32;
        self.parent.height = (term.height as u32 * self.height_p / 100) as u32;

        if self.parent.height < 8 {
            self.parent.height = 8;
        }

        brshtop_box._b_cpu_h = self.parent.height as i32;

        self.sub.box_columns =
            ceil(((THREADS + 1) / (self.parent.height - 5) as u64) as f64, 0) as u32;

        if self.sub.box_columns * (20 + if cpu.got_sensors { 13 } else { 21 })
            < self.parent.width - (self.parent.width / 3) as u32
        {
            self.sub.column_size = 2;
            self.sub.box_width = 20 + if cpu.got_sensors { 13 } else { 21 };
        } else if self.sub.box_columns * (15 + if cpu.got_sensors { 6 } else { 15 })
            < self.parent.width - (self.parent.width / 3) as u32
        {
            self.sub.column_size = 1;
            self.sub.box_width = 15 + if cpu.got_sensors { 6 } else { 15 };
        } else if self.sub.box_columns * (8 + if cpu.got_sensors { 6 } else { 8 })
            < self.parent.width - (self.parent.width / 3) as u32
        {
            self.sub.column_size = 0;
        } else {
            self.sub.box_columns = (self.parent.width - (self.parent.width / 3) as u32)
                / (8 + if cpu.got_sensors { 6 } else { 8 });
            self.sub.column_size = 0;
        }

        if self.sub.column_size == 0 {
            self.sub.box_width = 8 + if cpu.got_sensors { 6 } else { 8 } * self.sub.box_columns + 1;
        }

        self.sub.box_height = ceil((THREADS / self.sub.box_columns as u64) as f64, 0) as u32 + 4;

        if self.sub.box_height > self.parent.height - 2 {
            self.sub.box_height = self.parent.height - 2;
        }

        self.sub.box_x = (self.parent.width - 1) - self.sub.box_width;
        self.sub.box_y = self.y
            + (ceil(((self.parent.height - 2) / 2) as f64, 0)
                - ceil((self.sub.box_height / 2) as f64, 0)
                + 1.0) as u32;
    }

    pub fn draw_bg(
        &mut self,
        key: &mut Key,
        theme: &mut Theme,
        term: &mut Term,
        config: &mut Config,
        CPU_NAME: String,
    ) -> String {
        if !key.mouse.contains("M".to_owned()) {
            let mut top: Vec<Vec<u32>> = Vec::<Vec<u32>>::new();
            for i in 0..6 {
                let mut pusher: Vec<u32> = Vec::<u32>::new();
                pusher.push(self.x + 10 + i);
                pusher.push(self.y);
                top.push(pusher);
            }
            key.insert("M".to_owned(), top);
        }

        return format!(
            "{}{}{}{}{}{}{}{}{}",
            create_box(
                0,
                0,
                0,
                0,
                Some(String::default()),
                Some(String::default()),
                Some(theme.colors.cpu_box),
                None,
                true,
                Some(Boxes::CpuBox(self))
            ),
            mv::to(self.y, self.x + 10),
            theme.colors.cpu_box.call(symbol::title_left.to_owned(), term),
            fx::b,
            theme.colors.hi_fg.call("M".to_owned(), term),
            theme.colors.title.call("enu".to_owned(), term),
            fx::ub,
            theme.colors.cpu_box.call(symbol::title_right.to_owned(), term),
            create_box(
                self.sub.box_x as i32,
                self.sub.box_y as i32,
                self.sub.box_width as i32,
                self.sub.box_height as i32,
                Some(if config.custom_cpu_name != String::default() {
                    CPU_NAME[..self.sub.box_width as usize - 14].to_owned()
                } else {
                    config.custom_cpu_name[..self.sub.box_width as usize - 14].to_owned()
                }),
                None,
                Some(theme.colors.div_line),
                None,
                true,
                Some(Boxes::CpuBox(self))
            )
        );
    }

    pub fn battery_activity<P: AsRef<Path>>(&mut self, config_dir: P, menu: &mut Menu) -> bool {
        let battery_manager = match Manager::new() {
            Ok(m) => m,
            Err(_) => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            }
        };

        let batteries = match battery_manager.batteries() {
            Ok(b) => b,
            Err(_) => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            }
        };

        let currentBattery = match batteries.next() {
            None => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            }
            Some(r) => r.unwrap(),
        };

        match self.battery_path {
            Some(_) => {
                self.battery_path = None;
                let checker = Path::new("/sys/class/power_supply");
                if checker.exists() {
                    match checker.read_dir() {
                        Ok(i) => {
                            for directory in i {
                                match directory {
                                    Ok(entry) => {
                                        let filename = match entry.file_name().into_string() {
                                            Ok(f) => f,
                                            Err(e) => {
                                                error::errlog(
                                                    config_dir,
                                                    format!("Unable to read a filename ({:#?})", e),
                                                );
                                                continue;
                                            }
                                        };
                                        if filename.starts_with("BAT")
                                            || filename.to_lowercase().contains("battery")
                                        {
                                            self.battery_path = Some(format!(
                                                "/sys/class/power_supply/{}",
                                                filename
                                            ));
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                        Err(e) => (),
                    }
                }
            }
            None => (),
        };

        let mut return_true: bool = false;
        let percentage: f32 =
            ceil(currentBattery.state_of_charge().get::<percent>() as f64, 0) as f32;

        if percentage != self.battery_percent {
            self.battery_percent = percentage;
            return_true = true;
        }

        let seconds: f32 = currentBattery.time_to_empty().unwrap().get::<second>();

        if seconds != self.battery_secs as f32 {
            self.battery_secs = seconds;
            return_true = true;
        }

        let status: String = "not_set".to_owned();

        if self.battery_path != None {
            status = match File::open(self.battery_path.unwrap() + "status") {
                Ok(f) => match readfile(f) {
                    Some(s) => s,
                    None => "not_set".to_owned(),
                },
                Err(e) => "not_set".to_owned(),
            };
        }
        if status == "not_set".to_owned() {
            status = match currentBattery.state() {
                State::Charging => "Charging".to_owned(),
                State::Discharging => "Discharging".to_owned(),
                State::Full => "Full".to_owned(),
                State::Unknown => "Unknown".to_owned(),
                State::Empty => "Empty".to_owned(),
            };
        }
        if status != self.battery_status {
            self.battery_status = status;
            return_true = true;
        }

        return return_true || self.resized || self.redraw || menu.active;
    }

    pub fn draw_fg(&mut self, cpu: &mut CpuCollector, config: &mut Config, key: &mut Key, theme : &mut Theme, term : &mut Term, ARG_MODE : ViewMode, graphs : Graphs) {
        if cpu.parent.redraw {
            self.redraw = true;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut lavg: String = String::default();

        let mut x = self.x + 1;
        let mut y = self.y + 1;
        let mut w = self.parent.width - 2;
        let mut h = self.parent.height - 2;
        let mut bx = self.sub.box_x + 1;
        let mut by = self.sub.box_y + 1;
        let mut bw = self.sub.box_width - 2;
        let mut bh = self.sub.box_height - 2;
        let mut hh = ceil((h / 2) as f64, 0);
        let mut hide_cores: bool = (cpu.cpu_temp_only || !config.show_coretemp) && cpu.got_sensors;
        let mut ct_width: u32 = if hide_cores {
            if 6 * self.sub.column_size > 6 {
                6 * self.sub.column_size
            } else {
                6
            }
        } else {
            0
        };

        if self.resized || self.redraw {
            if !key.mouse.contains("m".to_owned()) {
                let mut parent = Vec::<Vec<u32>>::new();
                for i in 0..12 {
                    let mut adder = Vec::<u32>::new();
                    adder.push(self.x + 16 + i);
                    adder.push(self.y);
                    parent.push(adder);
                }
                key.mouse.set("m".to_owned(), parent);
            }
            out_misc += format!("{}{}{}{}{}ode:{}{}{}", 
                mv::to(self.y, self.x + 16),
                theme.colors.cpu_box.call(symbol::title_left.to_owned(), term),
                fx::b,
                theme.colors.title,
                ARG_MODE != ViewMode::None || config.view_mode != ViewMode::None,
                fx::ub,
                theme.colors.cpu_box.call(symbol::title_right.to_owned(), term)
            );
            graphs.cpu.insert("up".to_owned(), Graph::new((w - bw - 3) as usize, hh as usize, Some(theme.gradient.get(&"cpu".to_owned()).unwrap()), cpu.cpu_usage[0], term));
        }
    }
}

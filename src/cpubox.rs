use {
    battery::*,
    crate::{
        brshtop_box::{BrshtopBox, Boxes, SubBoxes},
        cpucollector::CpuCollector,
        config::{Config, ViewMode},
        create_box,
        fx,
        key::Key,
        mv,
        subbox::SubBox,
        symbol,
        term::Term,
        theme::{Color, Theme},
    },
    error,
    std::{collections::HashMap, path::Path},
    math::round::ceil,
};

pub struct CpuBox {
    pub parent : BrshtopBox,
    pub sub : SubBox,
    pub name : String,
    pub x : u32,
    pub y : u32,
    pub height_p : u32,
    pub width_p : u32,
    pub resized : bool,
    pub redraw : bool,
    pub buffer : String,
    pub battery_percent : f32,
    pub battery_secs : i32,
    pub battery_status : String,
    pub old_battery_pos : u32,
    pub old_battery_len : usize,
    pub battery_path : Option<String>,
    pub battery_clear : bool,
    pub battery_symbols : HashMap<String, String>,
    pub clock_block : bool,
} impl CpuBox {
    pub fn new(brshtop_box : BrshtopBox, config : Config, ARG_MODE : ViewMode) -> Self {
        let mut bsm : HashMap::<String, String> = HashMap::<String, String>::new();
        bsm.insert("Charging".to_owned(), "▲".to_owned());
        bsm.insert("Discharging".to_owned(), "▼".to_owned());
        bsm.insert("Full".to_owned(), "■".to_owned());
        bsm.insert("Not charging".to_owned(), "■".to_owned());

        let buffer_mut : String = "cpu".to_owned();

        brshtop_box.buffers.push(buffer_mut.clone());

        CpuBox {
            parent : BrshtopBox::new(config, ARG_MODE),
            sub : SubBox::new(),
            name : "cpu".to_owned(),
            x : 1,
            y : 1,
            height_p : 32,
            width_p : 100,
            resized : true,
            redraw : false,
            buffer : buffer_mut.clone(),
            battery_percent : 1000.0,
            battery_secs : 0,
            battery_status : "Unknown".to_owned(),
            old_battery_pos : 0,
            old_battery_len : 0,
            battery_path : Some("".to_owned()),
            battery_clear : false,
            battery_symbols : bsm.clone(),
            clock_block : true,
        }
    }

    pub fn calc_size(&mut self, THREADS : u64, term : Term, brshtop_box : BrshtopBox) {
        let cpu : CpuCollector = CpuCollector::new(THREADS);
        let mut height_p : u32 = 0;
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

        self.sub.box_columns = ceil(((THREADS + 1) / (self.parent.height - 5) as u64) as f64, 0) as u32;

        if self.sub.box_columns * (20 + if cpu.got_sensors {13} else {21}) < self.parent.width - (self.parent.width / 3) as u32 {
            self.sub.column_size = 2;
            self.sub.box_width = 20 + if cpu.got_sensors {13} else {21};    
        } else if self.sub.box_columns * (15 + if cpu.got_sensors {6} else {15}) < self.parent.width - (self.parent.width / 3) as u32 {
            self.sub.column_size = 1;
            self.sub.box_width = 15 + if cpu.got_sensors {6} else {15};
        } else if self.sub.box_columns * (8 + if cpu.got_sensors {6} else {8}) < self.parent.width - (self.parent.width / 3) as u32 {
            self.sub.column_size = 0;
        } else {
            self.sub.box_columns = (self.parent.width - (self.parent.width / 3) as u32) / (8 + if cpu.got_sensors {6} else {8});
            self.sub.column_size = 0;
        }

        if self.sub.column_size == 0 {
            self.sub.box_width = 8 + if cpu.got_sensors {6} else {8} * self.sub.box_columns + 1;
        }

        self.sub.box_height = ceil((THREADS / self.sub.box_columns as u64) as f64, 0) as u32 + 4;

        if self.sub.box_height > self.parent.height - 2 {
            self.sub.box_height = self.parent.height - 2;
        }

        self.sub.box_x = (self.parent.width - 1) - self.sub.box_width;
        self.sub.box_y = self.y + (ceil(((self.parent.height - 2) / 2) as f64, 0) - ceil((self.sub.box_height / 2) as f64, 0) + 1.0) as u32;
    }

    pub fn draw_bg(&mut self, key : Key, theme : Theme, term : Term, config : Config, CPU_NAME : String) -> String {
        if !key.mouse.contains("M".to_owned()) {
            let mut top : Vec<Vec<String>> = Vec::<Vec<String>>::new();
            for i in 0..6 {
                let mut pusher : Vec<String> = Vec::<String>::new();
                pusher.push(format!("{}", self.x + 10 + i));
                pusher.push(self.y.to_string());
                top.push(pusher);
            }
            key.insert("M".to_owned(), top);
        }

        return format!("{}{}{}{}{}{}{}{}{}",
            create_box(0, 0, 0, 0, Some(String::default()), Some(String::default()), Some(theme.cpu_box), None, true, Boxes::CpuBox(self)),
            mv::to(self.y, self.x + 10),
            theme.cpu_box.call(symbol::title_left.to_owned(), term),
            fx::b,
            theme.hi_fg.call("M".to_owned(), term),
            theme.title.call("enu".to_owned(), term),
            fx::ub,
            theme.cpu_box.call(symbol::title_right.to_owned(), term),
            create_box(self.sub.box_x as i32, 
                self.sub.box_y as i32, 
                self.sub.box_width as i32, 
                self.sub.box_height as i32, 
                Some(
                    if config.custom_cpu_name != String::default(){
                        CPU_NAME[..self.sub.box_width as usize - 14].to_owned()
                    } else {
                        config.custom_cpu_name[..self.sub.box_width as usize - 14].to_owned()
                }), 
                None, 
                Some(theme.div_line), 
                None, 
                true, 
                Boxes::CpuBox(self))
        );
    }

    pub fn battery_activity<P: AsRef<Path>>(&mut self, config_dir : P) -> bool {
        let battery_manager = match Manager::new() {
            Ok(m) => m,
            Err(_) => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            },
        };

        let batteries = match battery_manager.batteries() {
            Ok(b) => b,
            Err(_) => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            },
        };

        let currentBattery = match batteries.next() {
            None => {
                if self.battery_percent != 1000.0 {
                    self.battery_clear = true;
                }
                return false;
            },
            Some(r) => match r {
                battery::Battery(b) => b,
                battery::errors::Error(e) =>{
                    error::errlog(config_dir, format!("Unable to read current battery charge ({:#?})", e));
                    return false;
                },
            },
        };

        match self.battery_path {
            Some(_) => {
                self.battery_path = None;
                let checker = Path::new("/sys/class/power_supply");
                if checker.exists() {
                    match checker.read_dir() {
                        Ok(i) => for directory in i {
                            match directory {
                                Ok(entry) => {
                                    let filename = match entry.file_name().into_string() {
                                        Ok(f) => f,
                                        Err(e) => {
                                            error::errlog(config_dir, format!("Unable to read a filename ({:#?})", e));
                                            continue;
                                        },
                                    };
                                    if filename.starts_with("BAT") || filename.to_lowercase().contains("battery") {
                                        self.battery_path = format!("/sys/class/power_supply/{}", filename);
                                        break;
                                    }
                                },
                                Err(_) => break,
                            }
                        },
                        Err(e) => (),
                    }
                }
            },
            None => (),
        };

        let mut return_true : bool = false;
        let current_charge = match currentBattery.state_of_charge() {
            Battery(p) => p,
            Error(e) => {
                error::errlog(config_dir, format!("Unable to read current battery charge ({:#?})", e));
                return false;
            },
        };
        let mut percent : u32 = ceil(current_charge, 0) as u32;

        return false;
        
    }
}
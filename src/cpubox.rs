use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox, SubBoxes},
        config::{Config, ViewMode},
        cpucollector::CpuCollector,
        create_box, error, fx, min_max,
        draw::Draw,
        graph::{Graph, Graphs},
        key::Key,
        menu::Menu,
        meter::{Meter, Meters},
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
        if !key.mouse.contains_key(&"M".to_owned()) {
            let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
            for i in 0..6 {
                let mut pusher: Vec<i32> = Vec::<i32>::new();
                pusher.push((self.x + 10 + i) as i32);
                pusher.push(self.y as i32);
                top.push(pusher);
            }
            key.mouse.insert("M".to_owned(), top);
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
            theme
                .colors
                .cpu_box
                .call(symbol::title_left.to_owned(), term),
            fx::b,
            theme.colors.hi_fg.call("M".to_owned(), term),
            theme.colors.title.call("enu".to_owned(), term),
            fx::ub,
            theme
                .colors
                .cpu_box
                .call(symbol::title_right.to_owned(), term),
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
                State::__Nonexhaustive => "Nonexhaustive".to_owned(),
            };
        }
        if status != self.battery_status {
            self.battery_status = status;
            return_true = true;
        }

        return return_true || self.resized || self.redraw || menu.active;
    }

    pub fn draw_fg<P: AsRef<Path>>(
        &mut self,
        cpu: &mut CpuCollector,
        config: &mut Config,
        key: &mut Key,
        theme: &mut Theme,
        term: &mut Term,
        draw : &mut Draw,
        ARG_MODE: ViewMode,
        graphs: &mut Graphs,
        meters : &mut Meters,
        THREADS : u64,
        menu : &mut Menu,
        config_dir : P,
        THEME : &mut Theme,
    ) {
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
        let mut hh = ceil((h / 2) as f64, 0) as i32;
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
            if !key.mouse.contains_key(&"m".to_owned()) {
                let mut parent = Vec::<Vec<i32>>::new();
                for i in 0..12 {
                    let mut adder = Vec::<i32>::new();
                    adder.push((self.x + 16 + i) as i32);
                    adder.push(self.y as i32);
                    parent.push(adder);
                }
                key.mouse.insert("m".to_owned(), parent);
            }
            out_misc += format!(
                "{}{}{}{}{}ode:{}{}{}",
                mv::to(self.y, self.x + 16),
                theme
                    .colors
                    .cpu_box
                    .call(symbol::title_left.to_owned(), term),
                fx::b,
                theme.colors.hi_fg.call("m".to_owned(), term),
                theme.colors.title,
                ARG_MODE != ViewMode::None || config.view_mode != ViewMode::None,
                fx::ub,
                theme
                    .colors
                    .cpu_box
                    .call(symbol::title_right.to_owned(), term)
            ).as_str();
            graphs.cpu.insert(
                "up".to_owned(),
                Graph::new_with_vec::<Color>(
                    (w - bw - 3) as u32,
                    hh as u32,
                    theme.gradient.get(&"cpu".to_owned()).unwrap().clone(),
                    cpu.cpu_usage[0].iter().map(|u| *u as i32).collect(),
                    term,
                    false,
                    0,
                    0,
                    None,
                ),
            );
            graphs.cpu.insert(
                "down".to_owned(),
                Graph::new_with_vec::<Color>(
                    (w - bw - 3) as u32,
                    hh as u32,
                    theme.gradient.get(&"cpu".to_owned()).unwrap().clone(),
                    cpu.cpu_usage[0].iter().map(|u| *u as i32).collect(),
                    term,
                    true,
                    0,
                    0,
                    None,
                ),
            );
            meters.cpu = Meter::new(
                cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32,
                bw - (if cpu.got_sensors {21} else {9}),
                "cpu".to_owned(),
                false,
                THEME,
                term,
            );

            if self.sub.column_size > 0 || ct_width > 0 {
                for n in 0..THREADS as usize {
                    graphs.cores[n] = Graph::new::<Color>(
                        5, 
                        1, 
                        None, 
                        cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(), 
                        term, 
                        false, 
                        cpu.cpu_temp_crit, 
                        -23, 
                        None
                    );
                }
            }
            if cpu.got_sensors {
                graphs.temps[0] = Graph::new::<Color>(
                    5, 
                    1, 
                    None, 
                    cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(), 
                    term, 
                    false, 
                    cpu.cpu_temp_crit, 
                    -23, 
                    None
                );
                if self.sub.column_size > 1 {
                    for n in 1..(THREADS + 1) as usize {
                        if cpu.cpu_temp[n].len() == 0 {
                            continue;
                        }
                        graphs.temps[n] = Graph::new::<Color>(
                            5, 
                            1, 
                            None, 
                            cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(), 
                            term, 
                            false, 
                            cpu.cpu_temp_crit, 
                            -23, 
                            None
                        );
                    }
                }
            }

            draw.buffer("cpu_misc".to_owned(), vec![out_misc.clone()], false, false, 100, true, false, false, key);
        }

        if config.show_battery && self.battery_activity(config_dir, menu) {
            let mut bat_out : String = String::default();
            let mut battery_time : String = String::default();
            if self.battery_secs > 0.0 {
                battery_time = format!("{:02}:{:02}", (self.battery_secs / 3600.0) as i32, ((self.battery_secs % 3600.0) / 60.0) as i32);
            }

            if self.resized {
                // TODO : Fix meter initialization, invert=true
                meters.battery = Meter::new(
                    self.battery_percent as i32, 
                    10, 
                    "cpu".to_owned(),
                    true,
                    THEME,
                    term
                );
            }
            
            let mut battery_symbol : String = self.battery_symbols.get(&self.battery_status).unwrap().clone();
            let battery_len : u32 = (format!("{}", config.update_ms).len() + 
                if self.parent.width >= 100 {11} else {0} + 
                battery_time.len() + 
                format!("{}", self.battery_percent).len()) as u32;
            let battery_pos : u32 = self.parent.width - battery_len - 17;

            if (battery_pos != self.old_battery_pos || battery_len != self.old_battery_len as u32) && self.old_battery_pos > 0 && !self.resized {
                bat_out.push_str(format!("{}{}", mv::to(y-1, self.old_battery_pos), theme.colors.cpu_box.call(symbol::h_line.repeat(self.old_battery_len + 4), term)).as_str());
            }

            self.old_battery_pos = battery_pos;
            self.old_battery_len = battery_len as usize;

            bat_out.push_str(format!("{}{}{}{}BAT{} {}%{}{}{}{}{}",
                    mv::to(y-1, battery_pos),
                    theme.colors.cpu_box.call(symbol::title_left.to_owned(), term),
                    fx::b,
                    theme.colors.title,
                    battery_symbol,
                    self.battery_percent,
                    if self.parent.width < 100 {
                        String::default()
                    } else {
                        format!(" {}{}{}",
                            fx::ub,
                            meters.battery.call(Some(self.battery_percent as i32), term),
                            fx::b,
                        )
                    },
                    theme.colors.title,
                    battery_time,
                    fx::ub,
                    theme.colors.cpu_box.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );

            draw.buffer(
                "battery".to_owned(),
                vec![format!("{}{}", 
                    bat_out,
                    term.fg,
                )],
                false,
                false,
                100,
                menu.active,
                false,
                false,
                key,
            );
        } else if self.battery_clear {
            out.push_str(format!("{}{}",
                    mv::to(y-1, self.old_battery_pos),
                    theme.colors.cpu_box.call(symbol::h_line.repeat(self.old_battery_len + 4), term),
                )
                .as_str()
            );
            self.battery_clear = false;
            self.battery_percent = 1000.0;
            self.battery_secs = 0.0;
            self.battery_status = "Unkown".to_owned();
            self.old_battery_pos = 0;
            self.old_battery_len = 0;
            self.battery_path = None;
            
            draw.clear(vec!["battery".to_owned()], true);
        }

        let mut cx : u32 = 0;
        let mut cy : u32 = 0;
        let mut cc : u32 = 0;
        let mut ccw : u32 = ((bw + 1) / self.sub.box_columns) as u32;

        if cpu.cpu_freq != 0.0 {
            let mut freq : String = if cpu.cpu_freq < 1000.0 {
                format!("{:.0} Mhz", cpu.cpu_freq)
            } else {
                format!("{:.1}", cpu.cpu_freq / 1000.0)
            };

            out.push_str(format!("{}{}{}{}{}{}",
                    mv::to(by -1, bx + bw - 9),
                    theme.colors.div_line.call(symbol::title_left.to_owned(), term),
                    fx::b,
                    theme.colors.title.call(freq, term),
                    fx::ub,
                    theme.colors.div_line.call(symbol::title_right.to_owned(), term),
                )
                .as_str()
            );
        }

        out.push_str(format!("{}{}{}{}{}{}{}{}{}{}{}{:>4}{}%",
                mv::to(y, x),
                graphs.cpu[&"up".to_owned()].call(
                    if self.resized {
                        None
                    } else {
                        Some(cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32)
                    }, 
                    term
                ),
                mv::to(y + hh as u32, x),
                graphs.cpu[&"up".to_owned()].call(
                    if self.resized {
                        None
                    } else {
                        Some(cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32)
                    }, 
                    term
                ),
                theme.colors.main_fg,
                mv::to(by + cy, bx + cx),
                fx::b,
                "CPU ",
                fx::ub,
                meters.cpu.call(Some(cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32), term),
                theme.gradient[&"cpu".to_owned()][cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as usize],
                cpu.cpu_usage[0][cpu.cpu_usage.len() - 2],
                theme.colors.main_fg
            )
            .as_str()
        );

        if cpu.got_sensors {
            out.push_str(format!("{} . . . . . {}{}{}{:>4}{}°C",
                    theme.colors.inactive_fg,
                    mv::left(5),
                    theme.gradient[&"temp".to_owned()][(min_max(cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2] as i32, 0, cpu.cpu_temp_crit) * (100 / cpu.cpu_temp_crit) as i32) as usize],
                    graphs.temps[0].call(if self.resized {
                            None
                        } else {
                            Some(cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2] as i32)
                        },
                        term
                    ),
                    cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2],
                    theme.colors.main_fg,
                )
                .as_str()
            );
        }

        cy += 1;
        for n in 1..(THREADS + 1) as usize {
            out.push_str(format!("{}{}{}{:<width$}",
                theme.colors.main_fg,
                mv::to(by + cy, bx + cx),
                fx::b.to_owned() + "C" + if THREADS < 100 {
                    fx::ub
                } else {
                    ""
                },
                if self.sub.column_size == 0 {
                    2
                } else {
                    3
                },
                width = n,
            )
            .as_str()
            );

            if self.sub.column_size > 0 || ct_width > 0 {
                out.push_str(format!("{}{}{}{}{}",
                    theme.colors.inactive_fg,
                    ".".repeat((5 * self.sub.column_size + ct_width) as usize),
                    mv::left((5 * self.sub.column_size + ct_width) as u32),
                    theme.gradient[&"cpu".to_owned()][(cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2]) as usize],
                    graphs.cores[n - 1].call(if self.resized {
                            None
                        } else {
                            Some(cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] as i32)
                        }, 
                        term
                    ),
                )
                .as_str()
                );
            } else {
                out.push_str(format!("{}", theme.gradient[&"cpu".to_owned()][(cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2]) as usize]).as_str());
            }

            out.push_str(format!("{:width$}{}°C",
                    cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2],
                    theme.colors.main_fg,
                    width = if self.sub.column_size < 2 {3} else {4},
                )
                .as_str()
            );

            if cpu.got_sensors && cpu.cpu_temp[n].len() != 0 && !hide_cores {
                if self.sub.column_size > 1 {
                    out.push_str(format!("{} . . . . . {}{}{}",
                            theme.colors.inactive_fg,
                            mv::left(5),
                            theme.gradient[&"temp".to_owned()][(if cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] >= cpu.cpu_temp_crit as u32 {100} else {cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] * (100 / cpu.cpu_temp_crit) as u32}) as usize],
                            graphs.temps[n].call(if self.resized {None} else {Some(cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] as i32)}, term)
                        )
                        .as_str()
                    );
                } else {
                    out.push_str(format!("{}",
                            theme.gradient[&"temp".to_owned()][if cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] >= cpu.cpu_temp_crit as u32 {
                                100
                            } else {
                                (cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] * (100 / cpu.cpu_temp_crit) as u32) as usize
                            }]
                        )
                        .as_str()
                    );
                }

            } else if cpu.got_sensors && !hide_cores {
                out.push_str(format!("{}", mv::right(if self.sub.column_size * 6 > 6 {self.sub.column_size * 6} else {6})).as_str());
            }

            out.push_str(theme.colors.div_line.call(symbol::v_line.to_owned(), term).to_string().as_str());
            cy += 1;

            if cy > ceil((THREADS / self.sub.box_columns as u64) as f64, 0) as u32 && n != THREADS as usize {
                cc += 1;
                cy = 1;
                cx = ccw * cc;
                if cc == self.sub.box_columns {
                    break;
                }
            }
        }

        if cy < bh - 1 {
            cy = bh - 1;
        }

        if cy < bh && cc < self.sub.box_columns {
            if self.sub.column_size == 2 && cpu.got_sensors {
                let mut adder : String = "   ".to_owned();
                cpu.load_avg.iter().map(|l| adder.push_str(l.to_string().as_str()));
                lavg = format!(" Load AVG:  {:^19.19}", adder);
            } else if self.sub.column_size == 2 || (self.sub.column_size == 1 && cpu.got_sensors) {
                let mut adder : String = " ".to_owned();
                cpu.load_avg.iter().map(|l| adder.push_str(l.to_string().as_str()));
                lavg = format!("LAV: {:^14.14}", adder);
            } else if self.sub.column_size == 1 || (self.sub.column_size == 0 && cpu.got_sensors) {
                let mut adder : String = "   ".to_owned();
                cpu.load_avg.iter().map(|l| adder.push_str(ceil(*l, 1).to_string().as_str()));
                lavg = format!("L {:^11.11}", adder);
            } else {
                let mut adder : String = "   ".to_owned();
                cpu.load_avg.iter().map(|l| adder.push_str(ceil(*l, 1).to_string().as_str()));
                lavg = format!("{:^7.7}", adder);
            }
            out.push_str(format!("{}{}{}{}", 
                    mv::to(by + cy, bx + cx),
                    theme.colors.main_fg,
                    lavg,
                    theme.colors.div_line.call(symbol::v_line.to_owned(), term)
                )
                .as_str()
            );
        }

        out.push_str(format!("{}{}up {}",
                mv::to(y + h - 1, x + 1),
                theme.colors.graph_text,
                cpu.uptime
            )
            .as_str()
        );

        // TODO : Fix buffer call, only_save = menu.active
        draw.buffer(
            self.buffer, 
            vec![format!("{}{}{}", 
                out_misc, 
                out, 
                term.fg
            )],
            false,
            false,
            100,
            menu.active,
            false,
            false,
            key
        );

        self.resized = false;
        self.redraw = false;
        self.clock_block = false;
    }

    
}

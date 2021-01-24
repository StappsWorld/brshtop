use {
    crate::{
        brshtop_box::{Boxes, BrshtopBox, SubBoxes},
        config::{Config, ViewMode},
        cpucollector::CpuCollector,
        create_box,
        draw::Draw,
        error, fx,
        graph::{Graph, Graphs},
        key::Key,
        menu::Menu,
        meter::{Meter, Meters},
        min_max, mv, readfile,
        subbox::SubBox,
        symbol,
        term::Term,
        theme::{Color, Theme},
        CPU_NAME, THREADS,
    },
    battery::{
        units::{ratio::percent, time::second},
        *,
    },
    math::round::ceil,
    std::{collections::HashMap, convert::TryFrom, fs::File, path::Path},
};

pub struct CpuBox {
    parent: BrshtopBox,
    sub: SubBox,
    redraw: bool,
    buffer: String,
    battery_percent: f32,
    battery_secs: f32,
    battery_status: String,
    old_battery_pos: u32,
    old_battery_len: usize,
    battery_path: Option<String>,
    battery_clear: bool,
    battery_symbols: HashMap<String, String>,
    clock_block: bool,
}
impl CpuBox {
    pub fn new(brshtop_box: &BrshtopBox, config: &Config, ARG_MODE: ViewMode) -> Self {
        let mut bsm: HashMap<String, String> = HashMap::<String, String>::new();
        bsm.insert("Charging".to_owned(), "▲".to_owned());
        bsm.insert("Discharging".to_owned(), "▼".to_owned());
        bsm.insert("Full".to_owned(), "■".to_owned());
        bsm.insert("Not charging".to_owned(), "■".to_owned());

        let buffer_mut: String = "cpu".to_owned();

        brshtop_box.push_buffers(buffer_mut.clone());

        let cpu_box = CpuBox {
            parent: BrshtopBox::new(config, ARG_MODE),
            sub: SubBox::new(),
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
        };
        cpu_box.set_parent_name("cpu".to_owned());
        cpu_box.set_parent_y(1);
        cpu_box.set_parent_x(1);
        cpu_box.set_parent_height_p(32);
        cpu_box.set_parent_width_p(100);
        cpu_box.set_parent_resized(true);
        cpu_box
    }

    pub fn calc_size(
        &self,
        term: &Term,
        brshtop_box: &BrshtopBox,
        cpu: &CpuCollector,
    ) {
        let mut height_p: u32 = if self.get_parent().get_proc_mode() {
            20
        } else {
            self.get_parent().get_height_p()
        };

        self.set_parent_width((term.width as u32 * self.get_parent().get_width_p() / 100) as u32);
        self.set_parent_height(
            (term.height as u32 * self.get_parent().get_height_p() / 100) as u32,
        );

        if self.get_parent().get_height() < 8 {
            self.set_parent_height(8);
        }

        brshtop_box.set_b_cpu_h(self.get_parent().get_height() as i32);

        self.set_sub_box_columns(ceil(
            ((THREADS.to_owned() + 1) / (self.get_parent().get_height() - 5) as u64) as f64,
            0,
        ) as u32);

        if self.get_sub().get_box_columns() * (20 + if cpu.got_sensors { 13 } else { 21 })
            < self.get_parent().get_width() - (self.get_parent().get_width() / 3) as u32
        {
            self.set_sub_column_size(2);
            self.set_sub_box_width(20 + if cpu.got_sensors { 13 } else { 21 });
        } else if self.get_sub().get_box_columns() * (15 + if cpu.got_sensors { 6 } else { 15 })
            < self.get_parent().get_width() - (self.get_parent().get_width() / 3) as u32
        {
            self.set_sub_column_size(1);
            self.set_sub_box_width(15 + if cpu.got_sensors { 6 } else { 15 });
        } else if self.get_sub().get_box_columns() * (8 + if cpu.got_sensors { 6 } else { 8 })
            < self.get_parent().get_width() - (self.get_parent().get_width() / 3) as u32
        {
            self.set_sub_column_size(0);
        } else {
            self.set_sub_box_columns(
                (self.get_parent().get_width() - (self.get_parent().get_width() / 3) as u32)
                    / (8 + if cpu.got_sensors { 6 } else { 8 }),
            );
            self.set_sub_column_size(0);
        }

        if self.get_sub().get_column_size() == 0 {
            self.set_sub_box_width(
                8 + if cpu.got_sensors { 6 } else { 8 } * self.get_sub().get_box_columns() + 1,
            );
        }

        self.set_sub_box_height(
            ceil(
                (THREADS.to_owned() / self.get_sub().get_box_columns() as u64) as f64,
                0,
            ) as u32
                + 4,
        );

        if self.get_sub().get_box_height() > self.get_parent().get_height() - 2 {
            self.set_sub_box_height(
                u32::try_from(self.get_parent().get_height() as i32 - 2).unwrap_or(0),
            );
        }

        self.set_sub_box_x(
            u32::try_from(
                (self.get_parent().get_width() as i32 - 1) - self.get_sub().get_box_width() as i32,
            )
            .unwrap_or(0),
        );
        self.set_sub_box_y(
            self.get_parent().get_y() + {
                let total: f64 = ceil(((self.get_parent().get_height() as i32 - 2) / 2) as f64, 0)
                    - ceil(
                        u32::try_from(self.get_sub().get_box_height() as i32 / 2).unwrap_or(0)
                            as f64,
                        0,
                    )
                    + 1.0;
                if total < 0.0 {
                    0
                } else {
                    total as u32
                }
            },
        );
    }

    pub fn draw_bg(
        &mut self,
        key: &Key,
        theme: &Theme,
        term: &Term,
        config: &Config,
    ) -> String {
        if !key.mouse.contains_key(&"M".to_owned()) {
            let mut top: Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
            for i in 0..6 {
                let mut pusher: Vec<i32> = Vec::<i32>::new();
                pusher.push((self.get_parent().get_x() + 10 + i) as i32);
                pusher.push(self.get_parent().get_y() as i32);
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
                Some(Boxes::CpuBox(self)),
                term,
                theme,
            ),
            mv::to(self.get_parent().get_y(), self.get_parent().get_x() + 10),
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
                self.get_sub().get_box_x(),
                self.get_sub().get_box_y(),
                self.get_sub().get_box_width(),
                self.get_sub().get_box_height(),
                Some(if config.custom_cpu_name != String::default() {
                    CPU_NAME.to_owned()
                        [..usize::try_from(self.get_sub().get_box_width() as i32 - 14).unwrap_or(0)]
                        .to_owned()
                } else {
                    config.custom_cpu_name
                        [..usize::try_from(self.get_sub().get_box_width() as i32 - 14).unwrap_or(0)]
                        .to_owned()
                }),
                None,
                Some(theme.colors.div_line),
                None,
                true,
                Some(Boxes::CpuBox(self)),
                term,
                theme,
            )
        );
    }

    pub fn battery_activity(&mut self, menu: &Menu) -> bool {
        let battery_manager = match Manager::new() {
            Ok(m) => m,
            Err(_) => {
                if self.get_battery_percent() != 1000.0 {
                    self.set_battery_clear(true);
                }
                return false;
            }
        };

        let batteries = match battery_manager.batteries() {
            Ok(b) => b,
            Err(_) => {
                if self.get_battery_percent() != 1000.0 {
                    self.set_battery_clear(true);
                }
                return false;
            }
        };

        let currentBattery: Battery = match batteries.next() {
            None => {
                if self.get_battery_percent() != 1000.0 {
                    self.set_battery_clear(true);
                }
                return false;
            }
            Some(r) => match r {
                Ok(b) => b,
                Err(_) => {
                    if self.get_battery_percent() != 1000.0 {
                        self.set_battery_clear(true);
                    }
                    return false;
                }
            },
        };

        match self.get_battery_path() {
            Some(_) => {
                self.set_battery_path(None);
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
                                                error::errlog(format!(
                                                    "Unable to read a filename ({:#?})",
                                                    e
                                                ));
                                                continue;
                                            }
                                        };
                                        if filename.starts_with("BAT")
                                            || filename.to_lowercase().contains("battery")
                                        {
                                            self.set_battery_path(Some(format!(
                                                "/sys/class/power_supply/{}",
                                                filename
                                            )));
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

        if percentage != self.get_battery_percent() {
            self.set_battery_percent(percentage);
            return_true = true;
        }

        let seconds: f32 = currentBattery.time_to_empty().unwrap().get::<second>();

        if seconds != self.get_battery_secs() {
            self.set_battery_secs(seconds);
            return_true = true;
        }

        let status: String = "not_set".to_owned();

        if self.get_battery_path().is_some() {
            status = match File::open(self.get_battery_path().unwrap() + "status") {
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
        if status != self.get_battery_status() {
            self.set_battery_status(status.clone());
            return_true = true;
        }

        return return_true || self.get_parent().get_resized() || self.get_redraw() || menu.active;
    }

    pub fn draw_fg(
        &mut self,
        cpu: &CpuCollector,
        config: &Config,
        key: &Key,
        theme: &Theme,
        term: &Term,
        draw: &Draw,
        ARG_MODE: ViewMode,
        graphs: &Graphs,
        meters: &Meters,
        menu: &Menu,
        THEME: &Theme,
    ) {
        if cpu.parent.get_redraw() {
            self.redraw = true;
        }

        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut lavg: String = String::default();

        let parent_box: BrshtopBox = self.get_parent();
        let mut x: u32 = parent_box.get_x() + 1;
        let mut y: u32 = parent_box.get_y() + 1;
        let mut w: u32 = parent_box.get_width() - 2;
        let mut h: u32 = parent_box.get_height() - 2;

        let sub: SubBox = self.get_sub();
        let mut bx: u32 = sub.get_box_x() + 1;
        let mut by: u32 = sub.get_box_y() + 1;
        let mut bw: u32 = sub.get_box_width() - 2;
        let mut bh: u32 = sub.get_box_height() - 2;
        let mut hh: u32 = ceil((h / 2) as f64, 0) as u32;
        let mut hide_cores: bool = (cpu.cpu_temp_only || !config.show_coretemp) && cpu.got_sensors;
        let mut ct_width: u32 = if hide_cores {
            if 6 * sub.get_column_size() > 6 {
                6 * sub.get_column_size()
            } else {
                6
            }
        } else {
            0
        };

        if parent_box.get_resized() || self.get_redraw() {
            if !key.mouse.contains_key(&"m".to_owned()) {
                let mut parent = Vec::<Vec<i32>>::new();
                for i in 0..12 {
                    let mut adder = Vec::<i32>::new();
                    adder.push((self.get_parent().get_x() + 16 + i) as i32);
                    adder.push(self.get_parent().get_y() as i32);
                    parent.push(adder);
                }
                key.mouse.insert("m".to_owned(), parent);
            }
            out_misc += format!(
                "{}{}{}{}{}ode:{}{}{}",
                mv::to(self.get_parent().get_y(), self.get_parent().get_x() + 16),
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
            )
            .as_str();
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
            meters.set_cpu(Meter::new(
                cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32,
                bw - (if cpu.got_sensors { 21 } else { 9 }),
                "cpu".to_owned(),
                false,
                THEME,
                term,
            ));

            if sub.get_column_size() > 0 || ct_width > 0 {
                for n in 0..THREADS.to_owned() as usize {
                    graphs.cores[n] = Graph::new(
                        5,
                        1,
                        None,
                        cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(),
                        term,
                        false,
                        cpu.cpu_temp_crit,
                        -23,
                        None,
                    );
                }
            }
            if cpu.got_sensors {
                graphs.temps[0] = Graph::new(
                    5,
                    1,
                    None,
                    cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(),
                    term,
                    false,
                    cpu.cpu_temp_crit,
                    -23,
                    None,
                );
                if sub.get_column_size() > 1 {
                    for n in 1..(THREADS.to_owned() + 1) as usize {
                        if cpu.cpu_temp[n].len() == 0 {
                            continue;
                        }
                        graphs.temps[n] = Graph::new(
                            5,
                            1,
                            None,
                            cpu.cpu_temp[0].iter().map(|u| *u as i32).collect(),
                            term,
                            false,
                            cpu.cpu_temp_crit,
                            -23,
                            None,
                        );
                    }
                }
            }

            draw.buffer(
                "cpu_misc".to_owned(),
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

        if config.show_battery && self.battery_activity(menu) {
            let mut bat_out: String = String::default();
            let mut battery_time: String = String::default();
            let battery_secs: f32 = self.get_battery_secs();
            if battery_secs > 0.0 {
                battery_time = format!(
                    "{:02}:{:02}",
                    (battery_secs / 3600.0) as i32,
                    ((battery_secs % 3600.0) / 60.0) as i32
                );
            }

            if self.get_parent().get_resized() {
                meters.set_battery(Meter::new(
                    self.battery_percent as i32,
                    10,
                    "cpu".to_owned(),
                    true,
                    THEME,
                    term,
                ));
            }

            let mut battery_symbol: String = self
                .get_battery_symbols()
                .get(&self.get_battery_status())
                .unwrap()
                .clone();
            let battery_len: u32 = (format!("{}", config.update_ms).len()
                + if self.get_parent().get_width() >= 100 {
                    11
                } else {
                    0
                }
                + battery_time.len()
                + format!("{}", self.battery_percent).len())
                as u32;
            let battery_pos: u32 = self.get_parent().get_width() - battery_len - 17;

            if (battery_pos != self.get_old_battery_pos()
                || battery_len != self.get_old_battery_len() as u32)
                && self.get_old_battery_pos() > 0
                && !self.get_parent().get_resized()
            {
                bat_out.push_str(
                    format!(
                        "{}{}",
                        mv::to(y - 1, self.old_battery_pos),
                        theme
                            .colors
                            .cpu_box
                            .call(symbol::h_line.repeat(self.old_battery_len + 4), term)
                    )
                    .as_str(),
                );
            }

            self.set_old_battery_pos(battery_pos);
            self.set_old_battery_len(battery_len as usize);

            bat_out.push_str(
                format!(
                    "{}{}{}{}BAT{} {}%{}{}{}{}{}",
                    mv::to(y - 1, battery_pos),
                    theme
                        .colors
                        .cpu_box
                        .call(symbol::title_left.to_owned(), term),
                    fx::b,
                    theme.colors.title,
                    battery_symbol,
                    self.battery_percent,
                    if self.get_parent().get_width() < 100 {
                        String::default()
                    } else {
                        format!(
                            " {}{}{}",
                            fx::ub,
                            meters
                                .get_battery()
                                .call(Some(self.get_battery_percent() as i32), term),
                            fx::b,
                        )
                    },
                    theme.colors.title,
                    battery_time,
                    fx::ub,
                    theme
                        .colors
                        .cpu_box
                        .call(symbol::title_right.to_owned(), term),
                )
                .as_str(),
            );

            draw.buffer(
                "battery".to_owned(),
                vec![format!("{}{}", bat_out, term.fg,)],
                false,
                false,
                100,
                menu.active,
                false,
                false,
                key,
            );
        } else if self.get_battery_clear() {
            out.push_str(
                format!(
                    "{}{}",
                    mv::to(y - 1, self.old_battery_pos),
                    theme
                        .colors
                        .cpu_box
                        .call(symbol::h_line.repeat(self.old_battery_len + 4), term),
                )
                .as_str(),
            );
            self.set_battery_clear(false);
            self.set_battery_percent(1000.0);
            self.set_battery_secs(0.0);
            self.set_battery_status("Unknown".to_owned());
            self.set_old_battery_pos(0);
            self.set_old_battery_len(0);
            self.set_battery_path(None);

            draw.clear(vec!["battery".to_owned()], true);
        }

        let mut cx: u32 = 0;
        let mut cy: u32 = 0;
        let mut cc: u32 = 0;
        let mut ccw: u32 = ((bw + 1) / sub.get_box_columns()) as u32;

        if cpu.cpu_freq != 0.0 {
            let mut freq: String = if cpu.cpu_freq < 1000.0 {
                format!("{:.0} Mhz", cpu.cpu_freq)
            } else {
                format!("{:.1}", cpu.cpu_freq / 1000.0)
            };

            out.push_str(
                format!(
                    "{}{}{}{}{}{}",
                    mv::to(by - 1, bx + bw - 9),
                    theme
                        .colors
                        .div_line
                        .call(symbol::title_left.to_owned(), term),
                    fx::b,
                    theme.colors.title.call(freq, term),
                    fx::ub,
                    theme
                        .colors
                        .div_line
                        .call(symbol::title_right.to_owned(), term),
                )
                .as_str(),
            );
        }

        out.push_str(
            format!(
                "{}{}{}{}{}{}{}{}{}{}{}{:>4}{}%",
                mv::to(y, x),
                graphs.cpu[&"up".to_owned()].call(
                    if self.get_parent().get_resized() {
                        None
                    } else {
                        Some(cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32)
                    },
                    term
                ),
                mv::to(y + hh as u32, x),
                graphs.cpu[&"up".to_owned()].call(
                    if self.get_parent().get_resized() {
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
                meters.get_cpu().call(
                    Some(cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as i32),
                    term
                ),
                theme.gradient[&"cpu".to_owned()]
                    [cpu.cpu_usage[0][cpu.cpu_usage[0].len() - 2] as usize],
                cpu.cpu_usage[0][cpu.cpu_usage.len() - 2],
                theme.colors.main_fg
            )
            .as_str(),
        );

        if cpu.got_sensors {
            out.push_str(
                format!(
                    "{} . . . . . {}{}{}{:>4}{}°C",
                    theme.colors.inactive_fg,
                    mv::left(5),
                    theme.gradient[&"temp".to_owned()][(min_max(
                        cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2] as i32,
                        0,
                        cpu.cpu_temp_crit
                    ) * (100 / cpu.cpu_temp_crit) as i32)
                        as usize],
                    graphs.temps[0].call(
                        if self.get_parent().get_resized() {
                            None
                        } else {
                            Some(cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2] as i32)
                        },
                        term
                    ),
                    cpu.cpu_temp[0][cpu.cpu_temp[0].len() - 2],
                    theme.colors.main_fg,
                )
                .as_str(),
            );
        }

        cy += 1;
        for n in 1..(THREADS.to_owned() + 1) as usize {
            out.push_str(
                format!(
                    "{}{}{}{:<width$}",
                    theme.colors.main_fg,
                    mv::to(by + cy, bx + cx),
                    fx::b.to_owned() + "C" + if THREADS.to_owned() < 100 { fx::ub } else { "" },
                    if self.get_sub().get_column_size() == 0 {
                        2
                    } else {
                        3
                    },
                    width = n,
                )
                .as_str(),
            );

            if self.get_sub().get_column_size() > 0 || ct_width > 0 {
                out.push_str(
                    format!(
                        "{}{}{}{}{}",
                        theme.colors.inactive_fg,
                        ".".repeat((5 * self.get_sub().get_column_size() + ct_width) as usize),
                        mv::left((5 * self.get_sub().get_column_size() + ct_width) as u32),
                        theme.gradient[&"cpu".to_owned()]
                            [(cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2]) as usize],
                        graphs.cores[n - 1].call(
                            if self.get_parent().get_resized() {
                                None
                            } else {
                                Some(cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] as i32)
                            },
                            term
                        ),
                    )
                    .as_str(),
                );
            } else {
                out.push_str(
                    format!(
                        "{}",
                        theme.gradient[&"cpu".to_owned()]
                            [(cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2]) as usize]
                    )
                    .as_str(),
                );
            }

            out.push_str(
                format!(
                    "{:width$}{}°C",
                    cpu.cpu_usage[n][cpu.cpu_usage[n].len() - 2],
                    theme.colors.main_fg,
                    width = if self.get_sub().get_column_size() < 2 {
                        3
                    } else {
                        4
                    },
                )
                .as_str(),
            );

            if cpu.got_sensors && cpu.cpu_temp[n].len() != 0 && !hide_cores {
                if self.get_sub().get_column_size() > 1 {
                    out.push_str(
                        format!(
                            "{} . . . . . {}{}{}",
                            theme.colors.inactive_fg,
                            mv::left(5),
                            theme.gradient[&"temp".to_owned()][(if cpu.cpu_temp[n]
                                [cpu.cpu_temp[n].len() - 2]
                                >= cpu.cpu_temp_crit as u32
                            {
                                100
                            } else {
                                cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2]
                                    * (100 / cpu.cpu_temp_crit) as u32
                            })
                                as usize],
                            graphs.temps[n].call(
                                if self.get_parent().get_resized() {
                                    None
                                } else {
                                    Some(cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2] as i32)
                                },
                                term
                            )
                        )
                        .as_str(),
                    );
                } else {
                    out.push_str(
                        format!(
                            "{}",
                            theme.gradient[&"temp".to_owned()][if cpu.cpu_temp[n]
                                [cpu.cpu_temp[n].len() - 2]
                                >= cpu.cpu_temp_crit as u32
                            {
                                100
                            } else {
                                (cpu.cpu_temp[n][cpu.cpu_temp[n].len() - 2]
                                    * (100 / cpu.cpu_temp_crit) as u32)
                                    as usize
                            }]
                        )
                        .as_str(),
                    );
                }
            } else if cpu.got_sensors && !hide_cores {
                out.push_str(
                    format!(
                        "{}",
                        mv::right(if self.get_sub().get_box_columns() * 6 > 6 {
                            self.get_sub().get_box_columns() * 6
                        } else {
                            6
                        })
                    )
                    .as_str(),
                );
            }

            out.push_str(
                theme
                    .colors
                    .div_line
                    .call(symbol::v_line.to_owned(), term)
                    .to_string()
                    .as_str(),
            );
            cy += 1;

            if cy
                > ceil(
                    (THREADS.to_owned() / self.get_sub().get_box_columns() as u64) as f64,
                    0,
                ) as u32
                && n != THREADS.to_owned() as usize
            {
                cc += 1;
                cy = 1;
                cx = ccw * cc;
                if cc == self.get_sub().get_box_columns() {
                    break;
                }
            }
        }

        if cy < bh - 1 {
            cy = bh - 1;
        }

        if cy < bh && cc < self.get_sub().get_box_columns() {
            if self.get_sub().get_box_columns() == 2 && cpu.got_sensors {
                let mut adder: String = "   ".to_owned();
                cpu.load_avg
                    .iter()
                    .map(|l| adder.push_str(l.to_string().as_str()));
                lavg = format!(" Load AVG:  {:^19.19}", adder);
            } else if self.get_sub().get_box_columns() == 2
                || (self.get_sub().get_box_columns() == 1 && cpu.got_sensors)
            {
                let mut adder: String = " ".to_owned();
                cpu.load_avg
                    .iter()
                    .map(|l| adder.push_str(l.to_string().as_str()));
                lavg = format!("LAV: {:^14.14}", adder);
            } else if self.get_sub().get_box_columns() == 1
                || (self.get_sub().get_box_columns() == 0 && cpu.got_sensors)
            {
                let mut adder: String = "   ".to_owned();
                cpu.load_avg
                    .iter()
                    .map(|l| adder.push_str(ceil(*l, 1).to_string().as_str()));
                lavg = format!("L {:^11.11}", adder);
            } else {
                let mut adder: String = "   ".to_owned();
                cpu.load_avg
                    .iter()
                    .map(|l| adder.push_str(ceil(*l, 1).to_string().as_str()));
                lavg = format!("{:^7.7}", adder);
            }
            out.push_str(
                format!(
                    "{}{}{}{}",
                    mv::to(by + cy, bx + cx),
                    theme.colors.main_fg,
                    lavg,
                    theme.colors.div_line.call(symbol::v_line.to_owned(), term)
                )
                .as_str(),
            );
        }

        out.push_str(
            format!(
                "{}{}up {}",
                mv::to(y + h - 1, x + 1),
                theme.colors.graph_text,
                cpu.uptime
            )
            .as_str(),
        );

        draw.buffer(
            self.get_buffer(),
            vec![format!("{}{}{}", out_misc, out, term.fg)],
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
        self.set_clock_block(false);
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

    pub fn set_parent_x(&mut self, x: u32) {
        self.parent.set_x(x.clone())
    }

    pub fn set_parent_y(&mut self, y: u32) {
        self.parent.set_y(y.clone())
    }

    pub fn set_parent_height_p(&mut self, height_p: u32) {
        self.parent.set_height_p(height_p.clone())
    }

    pub fn set_parent_width_p(&mut self, width_p: u32) {
        self.parent.set_width_p(width_p.clone())
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

    pub fn get_sub(&self) -> SubBox {
        self.sub.clone()
    }

    pub fn set_sub(&mut self, sub: SubBox) {
        self.sub = sub.clone()
    }

    pub fn set_sub_box_columns(&mut self, box_columns: u32) {
        self.sub.set_box_columns(box_columns.clone())
    }

    pub fn set_sub_column_size(&mut self, column_size: u32) {
        self.sub.set_column_size(column_size.clone())
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

    pub fn get_buffer(&self) -> String {
        self.buffer.clone()
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer.clone()
    }

    pub fn get_battery_percent(&self) -> f32 {
        self.battery_percent.clone()
    }

    pub fn set_battery_percent(&mut self, battery_percent: f32) {
        self.battery_percent = battery_percent.clone()
    }

    pub fn get_battery_secs(&self) -> f32 {
        self.battery_secs.clone()
    }

    pub fn set_battery_secs(&mut self, battery_secs: f32) {
        self.battery_secs = battery_secs.clone()
    }

    pub fn get_battery_status(&self) -> String {
        self.battery_status.clone()
    }

    pub fn set_battery_status(&mut self, battery_status: String) {
        self.battery_status = battery_status.clone()
    }

    pub fn get_old_battery_pos(&self) -> u32 {
        self.old_battery_pos.clone()
    }

    pub fn set_old_battery_pos(&mut self, old_battery_pos: u32) {
        self.old_battery_pos = old_battery_pos.clone()
    }

    pub fn get_old_battery_len(&self) -> usize {
        self.old_battery_len.clone()
    }

    pub fn set_old_battery_len(&mut self, old_battery_len: usize) {
        self.old_battery_len = old_battery_len.clone()
    }

    pub fn get_battery_path(&self) -> Option<String> {
        match self.battery_path {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub fn set_battery_path(&mut self, battery_path: Option<String>) {
        self.battery_path = match battery_path {
            Some(s) => Some(s.clone()),
            None => None,
        }
    }

    pub fn get_battery_clear(&self) -> bool {
        self.battery_clear.clone()
    }

    pub fn set_battery_clear(&mut self, battery_clear: bool) {
        self.battery_clear = battery_clear.clone()
    }

    pub fn get_battery_symbols(&self) -> HashMap<String, String> {
        self.battery_symbols.clone()
    }

    pub fn set_battery_symbols(&mut self, battery_symbols: HashMap<String, String>) {
        self.battery_symbols = battery_symbols.clone()
    }

    pub fn get_clock_block(&self) -> bool {
        self.clock_block.clone()
    }

    pub fn set_clock_block(&mut self, clock_block: bool) {
        self.clock_block = clock_block.clone()
    }
}

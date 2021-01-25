use crate::{brshtop_box, consts::THEME_DIRS, cpucollector};

use {
    crate::{
        banner,
        brshtop_box::{Boxes, BrshtopBox},
        clean_quit,
        collector::{Collector, Collectors},
        config::{Config, ConfigAttr, LogLevel, ViewMode},
        cpubox::CpuBox,
        cpucollector::CpuCollector,
        create_box,
        draw::Draw,
        error,
        event::Event,
        first_letter_to_upper_case, fx,
        init::Init,
        key::Key,
        membox::MemBox,
        mv,
        netbox::NetBox,
        netcollector::NetCollector,
        procbox::ProcBox,
        proccollector::ProcCollector,
        symbol,
        term::Term,
        theme::{Color, Colors, Theme},
        timer::Timer,
        updatechecker::UpdateChecker,
        THEME_DIR, THREADS, VERSION,
    },
    math::round::ceil,
    std::{collections::HashMap, iter::FromIterator, mem, path::Path},
};

pub struct Menu {
    pub active: bool,
    pub close: bool,
    pub resized: bool,
    pub menus: HashMap<String, HashMap<String, String>>,
    pub menu_length: HashMap<String, i32>,
    pub background: String,
}
impl Menu {
    pub fn new(
        MENUS: HashMap<String, HashMap<String, (String, String, String)>>,
        MENU_COLORS: HashMap<String, Vec<String>>,
    ) -> Self {
        let mut menu_length_mut: HashMap<String, i32> = HashMap::<String, i32>::new();
        let mut menus_mut: HashMap<String, HashMap<String, String>> =
            HashMap::<String, HashMap<String, String>>::new();

        for (name, menu) in MENUS {
            menu_length_mut[&name] = menu[&"normal".to_owned()].0.len() as i32;
            menus_mut.insert(name, HashMap::<String, String>::new());
            for sel in vec!["normal".to_owned(), "selected".to_owned()] {
                menus_mut[&name][&sel] = String::default();
                let menu_string: String =
                    (menu[&sel].0 + menu[&sel].1.as_str() + menu[&sel].2.as_str()).to_owned();
                let iterable: Vec<String> = vec![menu[&sel].0, menu[&sel].1, menu[&sel].2];
                for i in 0..menu_string.len() {
                    menus_mut[&name][&sel].push_str(
                        fx::Fx::trans(format!(
                            "{}{}",
                            Color::fg(MENU_COLORS[&sel][i]).unwrap(),
                            iterable[i]
                        ))
                        .as_str(),
                    );

                    if i < 2 {
                        menus_mut[&name][&sel] += format!(
                            "{}{}",
                            mv::down(1),
                            mv::left(iterable[i].to_string().len() as u32)
                        )
                        .as_str();
                    }
                }
            }
        }

        Menu {
            active: false,
            close: false,
            resized: true,
            menus: menus_mut,
            menu_length: menu_length_mut,
            background: String::default(),
        }
    }

    pub fn main(
        &mut self,
        theme: &Theme,
        draw: &Draw,
        term: &Term,
        update_checker: &UpdateChecker,
        THEME: &Theme,
        key_class: &Key,
        timer: &Timer,
        collector: &Collector,
        collectors: Vec<Collectors>,
        CONFIG: &Config,
        ARG_MODE: ViewMode,
        netcollector: &NetCollector,
        brshtop_box: &BrshtopBox,
        init: &Init,
        cpubox: &CpuBox,
        cpucollector: &CpuCollector,
        boxes: Vec<Boxes>,
        netbox: &NetBox,
        proccollector: &ProcCollector,
        membox: &MemBox,
        procbox: &ProcBox,
    ) {
        let mut out: String = String::default();
        let mut banner_mut: String = String::default();
        let mut redraw: bool = true;
        let mut key: String = String::default();
        let mut mx: i32 = 0;
        let mut my: i32 = 0;
        let mut skip: bool = false;
        let mut mouse_over: bool = false;
        let mut mouse_items: HashMap<String, HashMap<String, i32>> =
            HashMap::<String, HashMap<String, i32>>::new();
        self.active = true;
        self.resized = true;
        let mut menu_names: Vec<String> = self.menus.keys().map(|s| s.clone()).collect();
        let mut menu_index: usize = 0;
        let mut menu_current: String = menu_names[0];
        self.background = format!(
            "{}{}{}",
            theme.colors.inactive_fg,
            fx::Fx::uncolor(draw.saved_buffer()),
            term.fg
        );

        while !self.close {
            key = String::default();
            if self.resized {
                banner_mut = format!(
                    "{}{}{}{}{}{} ← esc{}{}Version: {}{}{}{}{}",
                    banner::draw_banner(
                        (term.height / 2) as u32 - 10,
                        0,
                        true,
                        false,
                        term,
                        draw,
                        key_class
                    ),
                    mv::down(1),
                    mv::left(46),
                    Color::BlackBg(),
                    Color::default(),
                    fx::b,
                    mv::right(30),
                    fx::i,
                    VERSION.to_owned(),
                    fx::ui,
                    fx::ub,
                    term.bg,
                    term.fg,
                );

                if update_checker.version != VERSION.to_owned() {
                    banner_mut.push_str(format!("{}{}{}New release {} availabel at https://github.com/aristocratos/bpytop{}{}",
                            mv::to(term.height as u32, 1),
                            fx::b,
                            THEME.colors.title,
                            update_checker.version,
                            fx::ub,
                            term.fg,
                        )
                        .as_str()
                    );
                }
                let mut cy: u32 = 0;
                for (name, menu) in self.menus {
                    let ypos: u32 = (term.height / 2) as u32 - 2 + cy;
                    let xpos: u32 = (term.width / 2) as u32 - (self.menu_length[&name] / 2) as u32;
                    mouse_items[&name] = [
                        ("x1", xpos),
                        ("x2", xpos + self.menu_length[&name] as u32 - 1),
                        ("y1", ypos),
                        ("y2", ypos + 2),
                    ]
                    .iter()
                    .cloned()
                    .map(|(i, j)| (i.to_owned(), j as i32))
                    .collect::<HashMap<String, i32>>();
                    cy += 3;
                }
                redraw = true;
                self.resized = false;
            }

            if redraw {
                out = String::default();
                for (name, menu) in self.menus {
                    out.push_str(
                        format!(
                            "{}{}",
                            mv::to(
                                mouse_items[&name][&"y1".to_owned()] as u32,
                                mouse_items[&name][&"x1".to_owned()] as u32
                            ),
                            menu[&(if name == menu_current {
                                "selected".to_owned()
                            } else {
                                "normal".to_owned()
                            })],
                        )
                        .as_str(),
                    );
                }
            }

            if skip && redraw {
                draw.now(vec![out], key_class);
            } else if !skip {
                draw.now(
                    vec![format!("{}{}{}", self.background, banner_mut, out)],
                    key_class,
                );
            }
            skip = false;
            redraw = false;

            if key_class.input_wait(timer.left(CONFIG).as_secs_f64(), true, draw, term) {
                if key_class.mouse_moved() {
                    let (mx_set, my_set) = key_class.get_mouse();
                    mx = mx_set;
                    my = my_set;

                    let mut broke: bool = false;
                    for (name, pos) in mouse_items {
                        if pos[&"x1".to_owned()] <= mx
                            && mx <= pos[&"x2".to_owned()]
                            && pos[&"y1".to_owned()] <= my
                            && my <= pos[&"y2".to_owned()]
                        {
                            mouse_over = true;
                            if name != menu_current {
                                menu_current = name;
                                menu_index =
                                    menu_names.iter().position(|&r| r == name).unwrap() as usize;
                                redraw = true;
                            }
                            broke = true;
                            break;
                        }
                    }
                    if !broke {
                        mouse_over = false;
                    }
                } else {
                    key = match key_class.get() {
                        Some(k) => k,
                        None => String::default(),
                    };
                }

                if key == "mouse_click".to_owned() && !mouse_over {
                    key = "M".to_owned();
                }

                if key == "q".to_owned() {
                    clean_quit(None, None, key_class, collector, draw, term, CONFIG, None);
                } else if vec!["up", "mouse_scroll_up", "shift_tab"]
                    .iter()
                    .map(|s| s.clone().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                {
                    menu_index -= 1;
                    if menu_index < 0 {
                        menu_index = menu_names.len() - 1;
                    }
                    menu_current = menu_names[menu_index];
                    redraw = true;
                } else if vec!["down", "mouse_scroll_down", "tab"]
                    .iter()
                    .map(|s| s.clone().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                {
                    menu_index += 1;
                    if menu_index > menu_names.len() - 1 {
                        menu_index = 0;
                    }
                    menu_current = menu_names[menu_index];
                    redraw = true;
                } else if key == "enter".to_owned()
                    || (key == "mouse_click".to_owned() && mouse_over)
                {
                    if menu_current == "quit".to_owned() {
                        clean_quit(None, None, key_class, collector, draw, term, CONFIG, None);
                    } else if menu_current == "options".to_owned() {
                        self.options(
                            ARG_MODE,
                            THEME,
                            theme,
                            draw,
                            term,
                            CONFIG,
                            key_class,
                            timer,
                            netcollector,
                            brshtop_box,
                            boxes,
                            collector,
                            init,
                            cpubox,
                            cpucollector,
                            netbox,
                            proccollector,
                            collectors,
                            procbox,
                            membox,
                        );
                        self.resized = true;
                    } else if menu_current == "help".to_owned() {
                        self.help(
                            THEME, draw, term, key_class, collector, collectors, CONFIG, timer,
                        );
                        self.resized = true;
                    }
                }
            }

            if timer.not_zero(CONFIG) && !self.resized {
                skip = true;
            } else {
                collector.collect(collectors, CONFIG, true, false, false, false, false);
                collector.set_collect_done(Event::Wait);
                collector.get_collect_done_reference().wait(2.0);
                collector.set_collect_done(Event::Flag(false));

                if CONFIG.background_update {
                    self.background = format!(
                        "{}{}{}",
                        THEME.colors.inactive_fg,
                        fx::Fx::uncolor(draw.saved_buffer()),
                        term.fg,
                    );
                }
                timer.stamp();
            }
        }

        draw.now(vec![format!("{}", draw.saved_buffer())], key_class);
        self.background = String::default();
        self.active = false;
        self.close = false;
    }

    pub fn help(
        &mut self,
        theme: &Theme,
        draw: &Draw,
        term: &Term,
        key_class: &Key,
        collector: &Collector,
        collectors: Vec<Collectors>,
        CONFIG: &Config,
        timer: &Timer,
    ) {
        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut redraw: bool = true;
        let mut key: String = String::default();
        let mut skip: bool = false;
        let mut main_active: bool = self.active;

        self.active = true;
        self.resized = true;
        if self.background == String::default() {
            self.background = format!(
                "{}{}{}",
                theme.colors.inactive_fg,
                fx::Fx::uncolor(draw.saved_buffer()),
                term.fg
            );
        }
        let mut help_items: HashMap<String, String> = [
            ("(Mouse 1)", "Clicks buttons and selects in process list."),
            (
                "Selected (Mouse 1)",
                "Show detailed information for selected process.",
            ),
            (
                "(Mouse scroll)",
                "Scrolls any scrollable list/text under cursor.",
            ),
            ("(Esc, shift+m)", "Toggles main menu."),
            ("(m)", "Change current view mode, order full->proc->stat."),
            ("(F2, o)", "Shows options."),
            ("(F1, h)", "Shows this window."),
            ("(ctrl+z)", "Sleep program and put in background."),
            ("(ctrl+c, q)", "Quits program."),
            ("(+) / (-)", "Add/Subtract 100ms to/from update timer."),
            ("(Up) (Down)", "Select in process list."),
            ("(Enter)", "Show detailed information for selected process."),
            (
                "(Spacebar)",
                "Expand/collapse the selected process in tree view.",
            ),
            ("(Pg Up) (Pg Down)", "Jump 1 page in process list."),
            (
                "(Home) (End)",
                "Jump to first or last page in process list.",
            ),
            ("(Left) (Right)", "Select previous/next sorting column."),
            ("(b) (n)", "Select previous/next network device."),
            ("(z)", "Toggle totals reset for current network device"),
            ("(a)", "Toggle auto scaling for the network graphs."),
            ("(y)", "Toggle synced scaling mode for network graphs."),
            ("(f)", "Input a string to filter processes with."),
            ("(c)", "Toggle per-core cpu usage of processes."),
            ("(r)", "Reverse sorting order in processes box."),
            ("(e)", "Toggle processes tree view."),
            ("(delete)", "Clear any entered filter."),
            (
                "Selected (T, t)",
                "Terminate selected process with SIGTERM - 15.",
            ),
            ("Selected (K, k)", "Kill selected process with SIGKILL - 9."),
            (
                "Selected (I, i)",
                "Interrupt selected process with SIGINT - 2.",
            ),
            ("_1", " "),
            ("_2", "For bug reporting and project updates, visit,"),
            ("_3", "https,//github.com/aristocratos/bpytop"),
        ]
        .iter()
        .map(|(s1, s2)| (s1.clone().to_owned(), s2.clone().to_owned()))
        .collect();

        while !self.close {
            key = String::default();
            if self.resized {
                let mut y: u32 = if term.height < (help_items.len() + 10) as u16 {
                    8
                } else {
                    ((term.height / 2) as i32 - (help_items.len() / 2) as i32 + 4) as u32
                };
                out_misc = format!(
                    "{}{}{}{}{}{}← esc{}{}Version: {}{}{}{}{}",
                    banner::draw_banner(y - 7, 0, true, false, term, draw, key_class),
                    mv::down(1),
                    mv::left(46),
                    Color::BlackBg(),
                    Color::default(),
                    fx::b,
                    mv::right(30),
                    fx::i,
                    VERSION.to_owned(),
                    fx::ui,
                    fx::ub,
                    term.bg,
                    term.fg
                );
                let mut x: u32 = (term.width / 2) as u32 - 36;
                let mut h: u32 = term.height as u32 - 2 - y;
                let mut w: u32 = 72;

                let mut pages: i32 = 0;
                if help_items.len() > h as usize {
                    pages = ceil((help_items.len() as u32 / h) as f64, 0) as i32;
                } else {
                    h = help_items.len() as u32;
                    pages = 0;
                }
                let mut page: i32 = 1;
                out_misc.push_str(
                    create_box(
                        x,
                        y,
                        w,
                        h + 3,
                        Some("help".to_owned()),
                        None,
                        Some(theme.colors.div_line),
                        None,
                        true,
                        None,
                        term,
                        theme,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .as_str(),
                );
                redraw = true;
                self.resized = false;

                if redraw {
                    out = String::default();
                    let mut cy = 0;
                    if pages != 0 {
                        out.push_str(
                            format!(
                                "{}{}{}{}{}{} {}{}{}/{} pg{}{}{}",
                                mv::to(y, x + 56),
                                theme
                                    .colors
                                    .div_line
                                    .call(symbol::title_left.to_owned(), term),
                                fx::b,
                                theme.colors.title.call("pg".to_owned(), term),
                                fx::ub,
                                theme.colors.main_fg.call(symbol::up.to_owned(), term),
                                fx::b,
                                theme.colors.title,
                                page,
                                pages,
                                fx::ub,
                                theme.colors.main_fg.call(symbol::down.to_owned(), term),
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
                            "{}{}{}{:^20}Description:{}",
                            mv::to(y + 1, x + 1),
                            theme.colors.title,
                            fx::b,
                            "Keys",
                            theme.colors.main_fg
                        )
                        .as_str(),
                    );

                    let mut n: usize = 0;
                    for (keys, desc) in help_items {
                        if pages != 0 && n < ((page - 1) * h as i32) as usize {
                            n += 1;
                            continue;
                        }
                        out.push_str(
                            format!(
                                "{}{}{:^20.20}{}{:50.50}",
                                mv::to(y + 2 + cy, x + 1),
                                fx::b,
                                if keys.starts_with("_") {
                                    "".to_owned()
                                } else {
                                    keys
                                },
                                fx::ub,
                                desc
                            )
                            .as_str(),
                        );
                        cy += 1;
                        if cy == h {
                            break;
                        }
                        n += 1;
                    }
                    if cy < h {
                        for i in 0..h - cy {
                            out.push_str(
                                format!(
                                    "{}{}",
                                    mv::to(y + 2 + cy + i, x + 1),
                                    " ".repeat((w - 2) as usize),
                                )
                                .as_str(),
                            );
                        }
                    }
                }

                if skip && redraw {
                    draw.now(vec![out], key_class);
                } else if !skip {
                    draw.now(
                        vec![format!("{}{}{}", self.background, out_misc, out)],
                        key_class,
                    );
                }
                skip = false;
                redraw = false;

                if key_class.input_wait(timer.left(CONFIG).as_secs_f64(), false, draw, term) {
                    key = match key_class.get() {
                        Some(k) => k,
                        None => break,
                    };

                    if key == "mouse_click".to_owned() {
                        let (mx, my) = key_class.get_mouse();

                        if x <= mx as u32
                            && mx <= (x + w) as i32
                            && y <= my as u32
                            && my <= (y + h + 3) as i32
                        {
                            if pages != 0
                                && my == y as i32
                                && x + 56 < mx as u32
                                && mx < (x + 61) as i32
                            {
                                key = "up".to_owned();
                            } else if pages != 0
                                && my == y as i32
                                && x + 63 < mx as u32
                                && mx < (x + 68) as i32
                            {
                                key = "down".to_owned();
                            }
                        } else {
                            key = "escape".to_owned();
                        }
                    }

                    if key == "q".to_owned() {
                        clean_quit(None, None, key_class, collector, draw, term, CONFIG, None);
                    } else if vec!["escape", "M", "enter", "backspace", "h", "f1"]
                        .contains(&key.as_str())
                    {
                        self.close = true;
                        break;
                    } else if vec!["up", "mouse_scroll_up", "page_up"].contains(&key.as_str())
                        && pages != 0
                    {
                        page -= 1;
                        if page < 1 {
                            page = pages;
                        }
                        redraw = true;
                    } else if vec!["down", "mouse_scroll_down", "page_down"].contains(&key.as_str())
                        && pages != 0
                    {
                        page == 1;
                        if page > pages {
                            page = 1;
                        }
                        redraw = true;
                    }
                }

                if timer.not_zero(CONFIG) && !self.resized {
                    skip = true;
                } else {
                    collector.collect(collectors, CONFIG, true, false, false, false, false);
                    collector.set_collect_done(Event::Wait);
                    collector.get_collect_done_reference().wait(2.0);
                    collector.set_collect_done(Event::Flag(false));
                    if CONFIG.background_update {
                        self.background = format!(
                            "{}{}{}",
                            theme.colors.inactive_fg,
                            fx::Fx::uncolor(draw.saved_buffer()),
                            term.fg,
                        );
                    }
                    timer.stamp();
                }
            }

            if main_active {
                self.close = false;
                return;
            }
            draw.now(vec![draw.saved_buffer()], key_class);
            self.active = false;
            self.close = false;
        }
    }

    pub fn options(
        &mut self,
        ARG_MODE: ViewMode,
        THEME: &Theme,
        theme: &Theme,
        draw: &Draw,
        term: &Term,
        CONFIG: &Config,
        key_class: &Key,
        timer: &Timer,
        netcollector: &NetCollector,
        brshtop_box: &BrshtopBox,
        boxes: Vec<Boxes>,
        collector: &Collector,
        init: &Init,
        cpubox: &CpuBox,
        cpucollector: &CpuCollector,
        netbox: &NetBox,
        proc_collector: &ProcCollector,
        collectors: Vec<Collectors>,
        procbox: &ProcBox,
        membox: &MemBox,
    ) {
        let mut out: String = String::default();
        let mut out_misc: String = String::default();
        let mut redraw: bool = true;
        let mut key: String = String::default();
        let mut skip: bool = false;
        let mut main_active: bool = self.active;
        self.active = true;
        self.resized = true;
        let mut d_quote: String = String::default();
        let mut inputting: bool = false;
        let mut input_val: String = String::default();
        THEME.refresh();
        if self.background == String::default() {
            self.background = format!(
                "{}{}{}",
                THEME.colors.inactive_fg,
                fx::Fx::uncolor(draw.saved_buffer()),
                term.fg,
            );
        }

        let mut option_items: HashMap<String, Vec<String>> = vec![
            (
                "color_theme",
                vec![
                    "Set color theme.",
                    "",
                    "Choose from all theme files in",
                    "\"/usr/[local/]share/bpytop/themes\" and",
                    "\"~/.config/bpytop/themes\".",
                    "",
                    "\"Default\" for builtin default theme.",
                    "User themes are prefixed by a plus sign \"+\".",
                    "",
                    "For theme updates see:",
                    "https://github.com/aristocratos/bpytop",
                ],
            ),
            (
                "theme_background",
                vec![
                    "If the theme set background should be shown.",
                    "",
                    "Set to False if you want terminal background",
                    "transparency.",
                ],
            ),
            (
                "view_mode",
                vec![
                    "Set bpytop view mode.",
                    "",
                    "\"full\" for everything shown.",
                    "\"proc\" for cpu stats and processes.",
                    "\"stat\" for cpu, mem, disks and net stats shown.",
                ],
            ),
            (
                "update_ms",
                vec![
                    "Update time in milliseconds.",
                    "",
                    "Recommended 2000 ms or above for better sample",
                    "times for graphs.",
                    "",
                    "Min value: 100 ms",
                    "Max value: 86400000 ms = 24 hours.",
                ],
            ),
            (
                "proc_sorting",
                vec![
                    "Processes sorting option.",
                    "",
                    "Possible values: \"pid\", \"program\", \"arguments\",",
                    "\"threads\", \"user\", \"memory\", \"cpu lazy\" and",
                    "\"cpu responsive\".",
                    "",
                    "\"cpu lazy\" updates top process over time,",
                    "\"cpu responsive\" updates top process directly.",
                ],
            ),
            (
                "proc_reversed",
                vec!["Reverse processes sorting order.", "", "True or False."],
            ),
            (
                "proc_tree",
                vec![
                    "Processes tree view.",
                    "",
                    "Set true to show processes grouped by parents,",
                    "with lines drawn between parent and child",
                    "process.",
                ],
            ),
            (
                "tree_depth",
                vec![
                    "Process tree auto collapse depth.",
                    "",
                    "Sets the depth were the tree view will auto",
                    "collapse processes at.",
                ],
            ),
            (
                "proc_colors",
                vec![
                    "Enable colors in process view.",
                    "",
                    "Uses the cpu graph gradient colors.",
                ],
            ),
            (
                "proc_gradient",
                vec![
                    "Enable process view gradient fade.",
                    "",
                    "Fades from top or current selection.",
                    "Max fade value is equal to current themes",
                    "\"inactive_fg\" color value.",
                ],
            ),
            (
                "proc_per_core",
                vec![
                    "Process usage per core.",
                    "",
                    "If process cpu usage should be of the core",
                    "it\'s running on or usage of the total",
                    "available cpu power.",
                    "",
                    "If true and process is multithreaded",
                    "cpu usage can reach over 100%.",
                ],
            ),
            (
                "proc_mem_bytes",
                vec![
                    "Show memory as bytes in process list.",
                    " ",
                    "True or False.",
                ],
            ),
            (
                "check_temp",
                vec!["Enable cpu temperature reporting.", "", "True or False."],
            ),
            (
                "cpu_sensor",
                vec![
                    "Cpu temperature sensor",
                    "",
                    "Select the sensor that corresponds to",
                    "your cpu temperature.",
                    "Set to \"Auto\" for auto detection.",
                ],
            ),
            (
                "show_coretemp",
                vec![
                    "Show temperatures for cpu cores.",
                    "",
                    "Only works if check_temp is True and",
                    "the system is reporting core temps.",
                ],
            ),
            (
                "draw_clock",
                vec![
                    "Draw a clock at top of screen.",
                    "",
                    "Formatting according to strftime, empty",
                    "string to disable.",
                    "",
                    "Custom formatting options:",
                    "\"/host\" = hostname",
                    "\"/user\" = username",
                    "",
                    "Examples of strftime formats:",
                    "\"%X\" = locale HH:MM:SS",
                    "\"%H\" = 24h hour, \"%I\" = 12h hour",
                    "\"%M\" = minute, \"%S\" = second",
                    "\"%d\" = day, \"%m\" = month, \"%y\" = year",
                ],
            ),
            (
                "background_update",
                vec![
                    "Update main ui when menus are showing.",
                    "",
                    "True or False.",
                    "",
                    "Set this to false if the menus is flickering",
                    "too much for a comfortable experience.",
                ],
            ),
            (
                "custom_cpu_name",
                vec![
                    "Custom cpu model name in cpu percentage box.",
                    "",
                    "Empty string to disable.",
                ],
            ),
            (
                "disks_filter",
                vec![
                    "Optional filter for shown disks.",
                    "",
                    "Should be last folder in path of a mountpoint,",
                    "\"root\" replaces \"/\", separate multiple values",
                    "with a comma.",
                    "Begin line with \"exclude=\" to change to exclude",
                    "filter.",
                    "Oterwise defaults to \"most include\" filter.",
                    "",
                    "Example: disks_filter=\"exclude=boot, home\"",
                ],
            ),
            (
                "mem_graphs",
                vec!["Show graphs for memory values.", "", "True or False."],
            ),
            (
                "show_swap",
                vec![
                    "If swap memory should be shown in memory box.",
                    "",
                    "True or False.",
                ],
            ),
            (
                "swap_disk",
                vec![
                    "Show swap as a disk.",
                    "",
                    "Ignores show_swap value above.",
                    "Inserts itself after first disk.",
                ],
            ),
            (
                "show_disks",
                vec!["Split memory box to also show disks.", "", "True or False."],
            ),
            (
                "net_download",
                vec![
                    "Fixed network graph download value.",
                    "",
                    "Default \"10M\" = 10 MibiBytes.",
                    "Possible units:",
                    "\"K\" (KiB), \"M\" (MiB), \"G\" (GiB).",
                    "",
                    "Append \"bit\" for bits instead of bytes,",
                    "i.e \"100Mbit\"",
                    "",
                    "Can be toggled with auto button.",
                ],
            ),
            (
                "net_upload",
                vec![
                    "Fixed network graph upload value.",
                    "",
                    "Default \"10M\" = 10 MibiBytes.",
                    "Possible units:",
                    "\"K\" (KiB), \"M\" (MiB), \"G\" (GiB).",
                    "",
                    "Append \"bit\" for bits instead of bytes,",
                    "i.e \"100Mbit\"",
                    "",
                    "Can be toggled with auto button.",
                ],
            ),
            (
                "net_auto",
                vec![
                    "Start in network graphs auto rescaling mode.",
                    "",
                    "Ignores any values set above at start and",
                    "rescales down to 10KibiBytes at the lowest.",
                    "",
                    "True or False.",
                ],
            ),
            (
                "net_sync",
                vec![
                    "Network scale sync.",
                    "",
                    "Syncs the scaling for download and upload to",
                    "whichever currently has the highest scale.",
                    "",
                    "True or False.",
                ],
            ),
            (
                "net_color_fixed",
                vec![
                    "Set network graphs color gradient to fixed.",
                    "",
                    "If True the network graphs color is based",
                    "on the total bandwidth usage instead of",
                    "the current autoscaling.",
                    "",
                    "The bandwidth usage is based on the",
                    "\"net_download\" and \"net_upload\" values set",
                    "above.",
                ],
            ),
            (
                "show_battery",
                vec![
                    "Show battery stats.",
                    "",
                    "Show battery stats in the top right corner",
                    "if a battery is present.",
                ],
            ),
            (
                "show_init",
                vec![
                    "Show init screen at startup.",
                    "",
                    "The init screen is purely cosmetical and",
                    "slows down start to show status messages.",
                ],
            ),
            (
                "update_check",
                vec![
                    "Check for updates at start.",
                    "",
                    "Checks for latest version from:",
                    "https://github.com/aristocratos/bpytop",
                ],
            ),
            (
                "log_level",
                vec![
                    "Set loglevel for error.log",
                    "",
                    "Levels are: \"ERROR\" \"WARNING\" \"INFO\" \"DEBUG\".",
                    "The level set includes all lower levels,",
                    "i.e. \"DEBUG\" will show all logging info.",
                ],
            ),
        ]
        .iter()
        .map(|(key, val)| {
            (
                key.clone().to_owned(),
                val.iter().map(|s| s.clone().to_owned()).collect(),
            )
        })
        .collect();

        let option_len: usize = option_items.len() * 2;
        let sorting_i: usize = CONFIG
            .sorting_options
            .iter()
            .position(|SO| *SO == CONFIG.proc_sorting)
            .unwrap();
        let loglevel_i: usize = CONFIG
            .log_levels
            .iter()
            .position(|LL| *LL == CONFIG.log_level)
            .unwrap();
        let view_mode_i: usize = CONFIG
            .view_modes
            .iter()
            .position(|VM| *VM == CONFIG.view_mode)
            .unwrap();
        let cpu_sensor_i: usize = CONFIG
            .cpu_sensors
            .iter()
            .position(|s| s.clone() == CONFIG.cpu_sensor)
            .unwrap();
        let mut color_i: usize = 0;

        while !self.close {
            key = String::default();
            let selected_int: usize = 0;
            let mut pages: u32 = 0;
            let mut page: u32 = 1;
            let y: u32 = if (term.height as u32) < (option_len as u32 + 10) {
                9
            } else {
                (term.height / 2) as u32 - (option_len / 2) as u32 + 4
            };
            let x: u32 = (term.width / 2) as u32 - 38;
            let h: u32 = term.height as u32 - 2 - y;
            let w: u32 = 26;
            let x2: u32 = x + 27;
            let w2: u32 = 50;

            if self.resized {
                out_misc = format!(
                    "{}{}{}{}{}{}← esc{}{}Version: {}{}{}{}{}",
                    banner::draw_banner(y - 7, 0, true, false, term, draw, key_class),
                    mv::down(1),
                    mv::left(46),
                    Color::BlackBg(),
                    Color::default(),
                    fx::b,
                    mv::right(30),
                    fx::i,
                    VERSION.to_owned(),
                    fx::ui,
                    fx::ub,
                    term.bg,
                    term.fg
                );
                h -= h % 2;
                color_i = THEME
                    .themes
                    .iter()
                    .position(|(s1, s2)| s1.clone() == THEME.current)
                    .unwrap();
                if option_len > h as usize {
                    pages = ceil((option_len / h as usize) as f64, 0) as u32;
                } else {
                    h = option_len as u32;
                    pages = 0;
                }
                let mut selected_int: usize = 0;
                out_misc.push_str(
                    create_box(
                        x,
                        y,
                        w,
                        h + 2,
                        Some("options".to_owned()),
                        None,
                        None,
                        None,
                        false,
                        None,
                        term,
                        theme,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                    .as_str(),
                );
                redraw = true;
                self.resized = false;
            }

            let selected: String = option_items
                .iter()
                .map(|(key, val)| key.clone())
                .collect::<Vec<String>>()[selected_int];

            if redraw {
                out = String::default();
                let cy: u32 = 0;

                if pages > 0 {
                    out.push_str(
                        format!(
                            "{}{}{}{}{}{} {}{}{}/{}pg{}{}{}",
                            mv::to(y + h + 1, x + 11),
                            THEME
                                .colors
                                .main_fg
                                .call(symbol::title_left.to_owned(), term),
                            fx::b,
                            THEME.colors.title.call("pg".to_owned(), term),
                            fx::ub,
                            THEME.colors.main_fg.call(symbol::up.to_owned(), term),
                            fx::b,
                            THEME.colors.title,
                            page,
                            pages,
                            fx::ub,
                            THEME.colors.main_fg.call(symbol::down.to_owned(), term),
                            THEME
                                .colors
                                .div_line
                                .call(symbol::title_right.to_owned(), term),
                        )
                        .as_str(),
                    );
                }

                let mut n: usize = 0;
                for (opt, _) in option_items {
                    if pages != 0 && (n as u32) < (pages - 1) * ceil((h / 2) as f64, 0) as u32 {
                        continue;
                    }

                    let value: ConfigAttr = CONFIG.getattr(opt);
                    let attr: String = match value {
                        ConfigAttr::Bool(_) => "bool".to_owned(),
                        ConfigAttr::Int64(_) => "i64".to_owned(),
                        ConfigAttr::LogLevel(l) => {
                            value = ConfigAttr::String(l.to_string());
                            "String".to_owned()
                        }
                        ConfigAttr::SortingOption(s) => {
                            value = ConfigAttr::String(s.to_string());
                            "String".to_owned()
                        }
                        ConfigAttr::String(_) => "String".to_owned(),
                        ConfigAttr::ViewMode(v) => {
                            value = ConfigAttr::String(v.to_string());
                            "String".to_owned()
                        }
                    };

                    let t_color: String = format!(
                        "{}{}",
                        THEME.colors.selected_bg,
                        if opt == selected {
                            THEME.colors.selected_fg
                        } else {
                            THEME.colors.title
                        },
                    );
                    let v_color: String = if opt == selected {
                        "".to_owned()
                    } else {
                        THEME.colors.title.to_string()
                    };
                    let d_quote = match value {
                        ConfigAttr::String(_) => "\"".to_owned(),
                        _ => "".to_owned(),
                    };
                    let mut counter: String = String::default();
                    if opt == "color_theme".to_owned() {
                        counter = format!(" {}/{}", color_i + 1, THEME.themes.len());
                    } else if opt == "proc_sorting".to_owned() {
                        counter = format!(" {}/{}", sorting_i + 1, CONFIG.sorting_options.len());
                    } else if opt == "log_level".to_owned() {
                        counter = format!(" {}/{}", loglevel_i + 1, CONFIG.log_levels.len());
                    } else if opt == "view_mode".to_owned() {
                        counter = format!(" {}/{}", view_mode_i + 1, CONFIG.view_modes.len());
                    } else if opt == "cpu_sensor".to_owned() {
                        counter = format!(" {}/{}", cpu_sensor_i + 1, CONFIG.cpu_sensors.len());
                    } else {
                        counter = String::default();
                    }

                    out.push_str(
                        format!(
                            "{}{}{}{:^24.24}{}{}{}",
                            mv::to(y + 1 + cy, x + 1),
                            t_color,
                            fx::b,
                            first_letter_to_upper_case(opt.replace("_", " ").to_owned())
                                + counter.as_str(),
                            fx::ub,
                            mv::to(y + 2 + cy, x + 1),
                            v_color,
                        )
                        .as_str(),
                    );

                    if opt == selected {
                        if attr == "bool".to_owned()
                            || vec![
                                "color_theme",
                                "proc_sorting",
                                "log_level",
                                "view_mode",
                                "cpu_sensor",
                            ]
                            .iter()
                            .map(|s| s.clone().to_owned())
                            .collect::<Vec<String>>()
                            .contains(&opt)
                        {
                            out.push_str(
                                format!(
                                    "{} {}{}{:^20.20}{}{}",
                                    t_color,
                                    symbol::left,
                                    v_color,
                                    d_quote
                                        + match value {
                                            ConfigAttr::Bool(b) => &b.to_string(),
                                            _ => "",
                                        }
                                        + d_quote.as_str(),
                                    t_color,
                                    symbol::right
                                )
                                .as_str(),
                            );
                        } else if inputting {
                            out.push_str(
                                format!(
                                    "{:^33.33}",
                                    input_val[input_val.len() - 18..].to_owned()
                                        + fx::bl
                                        + "█"
                                        + fx::ubl
                                        + ""
                                        + symbol::enter,
                                )
                                .as_str(),
                            );
                        } else {
                            out.push_str(
                                format!(
                                    "{}{:^20.20}{}",
                                    if attr == "i64".to_owned() {
                                        format!("{} {}{}", t_color, symbol::left, v_color,)
                                    } else {
                                        "  ".to_owned()
                                    },
                                    match value {
                                        ConfigAttr::Bool(b) => &b.to_string(),
                                        ConfigAttr::Int64(i) => &i.to_string(),
                                        ConfigAttr::String(s) => &s.clone(),
                                        ConfigAttr::LogLevel(l) => &l.clone().to_string(),
                                        ConfigAttr::SortingOption(s) => &s.clone().to_string(),
                                        ConfigAttr::ViewMode(v) => &v.clone().to_string(),
                                    }
                                    .clone()
                                        + " "
                                        + symbol::enter,
                                    if attr == "i64".to_owned() {
                                        format!("{}{} ", t_color, symbol::right)
                                    } else {
                                        "  ".to_owned()
                                    }
                                )
                                .as_str(),
                            );
                        }
                    } else {
                        out.push_str(
                            format!(
                                "{:^24.24}",
                                d_quote
                                    + match value {
                                        ConfigAttr::Bool(b) => &b.to_string(),
                                        ConfigAttr::Int64(i) => &i.to_string(),
                                        ConfigAttr::String(s) => &s.clone(),
                                        ConfigAttr::LogLevel(l) => &l.clone().to_string(),
                                        ConfigAttr::SortingOption(s) => &s.clone().to_string(),
                                        ConfigAttr::ViewMode(v) => &v.clone().to_string(),
                                    }
                                    .clone()
                                    .as_str()
                                    + d_quote.as_str()
                            )
                            .as_str(),
                        )
                    }
                    out.push_str(term.bg.to_string().as_str());
                    if opt == selected {
                        let h2: u32 = (option_items[&opt].len() + 2) as u32;
                        let mut y2: u32 = (y + (selected_int as u32 * 2) - ((page - 1) * h)) as u32;
                        if y2 + h2 > term.height as u32 {
                            y2 = term.height as u32 - h2;
                        }
                        out.push_str(
                            create_box(
                                x2,
                                y2,
                                w2,
                                h2,
                                Some("description".to_owned()),
                                None,
                                Some(THEME.colors.div_line),
                                None,
                                true,
                                None,
                                term,
                                theme,
                                None,
                                None,
                                None,
                                None,
                                None,
                            )
                            .as_str(),
                        );
                        let mut n2: usize = 0;
                        for desc in option_items[&opt] {
                            out.push_str(
                                format!("{}{:.48}", mv::to(y2 + 1 + n2 as u32, x2 + 2), desc)
                                    .as_str(),
                            );
                            n2 += 1;
                        }
                    }
                    cy += 2;
                    if cy < h {
                        break;
                    }
                    n += 1;
                }
                if cy < h {
                    for i in 0..h - cy {
                        out.push_str(
                            format!(
                                "{}{}",
                                mv::to(y + 1 + cy + i, x + 1),
                                " ".repeat((w - 2) as usize)
                            )
                            .as_str(),
                        );
                    }
                }
            }

            if !skip || redraw {
                draw.now(
                    vec![format!("{}{}{}", self.background, out_misc, out)],
                    key_class,
                );
            }
            skip = false;
            redraw = false;

            if key_class.input_wait(timer.left(CONFIG).as_secs_f64(), false, draw, term) {
                key = match key_class.get() {
                    Some(k) => k,
                    None => "".to_owned(),
                };
                redraw = true;
                let mut has_sel = false;
                if key == "mouse_click".to_owned() && !inputting {
                    let (mx, my) = key_class.get_mouse();
                    if x < mx as u32
                        && mx < (x + w) as i32
                        && y < my as u32
                        && my < (y + h + w) as i32
                    {
                        let mouse_sel: u32 = ceil(((my - y as i32) / 2) as f64, 0) as u32
                            + ceil(((page - 1) * (h / 2)) as f64, 0) as u32;
                        if pages != 0
                            && my == (y + h + 1) as i32
                            && (x as i32 + 11) < mx
                            && mx < (x as i32 + 16)
                        {
                            key = "page_up".to_owned();
                        } else if pages != 0
                            && my == (y + h + 1) as i32
                            && (x as i32 + 19) < mx
                            && mx < (x as i32 + 24)
                        {
                            key = "page_down".to_owned();
                        } else if my == (y + h + 1) as i32 {
                            //pass
                        } else if mouse_sel == selected_int as u32 {
                            if mx < (x + 6) as i32 {
                                key = "left".to_owned();
                            } else if mx > (x + 19) as i32 {
                                key = "right".to_owned();
                            } else {
                                key = "enter".to_owned();
                            }
                        } else if mouse_sel < option_items.len() as u32 {
                            selected_int = mouse_sel as usize;
                            has_sel = true;
                        }
                    } else {
                        key = "escape".to_owned();
                    }
                }

                if inputting {
                    if vec!["escape", "mouse_click"]
                        .iter()
                        .map(|s| s.to_owned().to_owned())
                        .collect::<Vec<String>>()
                        .contains(&key)
                    {
                        inputting = false;
                    } else if key == "enter".to_owned() {
                        inputting = false;
                        match CONFIG.getattr(selected) {
                            ConfigAttr::String(s) => {
                                if s != input_val {
                                    let parsed: i64 = input_val.parse::<i64>().unwrap();
                                    if selected == "update_ms".to_owned() {
                                        if input_val == String::default() || parsed < 100 {
                                            CONFIG.update_ms = 100;
                                        } else if parsed > 86399900 {
                                            CONFIG.update_ms = 86399900;
                                        } else {
                                            CONFIG.update_ms = parsed;
                                        }
                                    } else if selected == "tree_depth".to_owned() {
                                        if input_val == String::default() || parsed < 0 {
                                            CONFIG.tree_depth = 0;
                                        } else {
                                            CONFIG.tree_depth = parsed as i32;
                                        }
                                    } else {
                                        match CONFIG.getattr(selected) {
                                            ConfigAttr::String(s) => {
                                                CONFIG.setattr_configattr(
                                                    selected,
                                                    ConfigAttr::String(input_val),
                                                );
                                                if selected.starts_with("net_") {
                                                    netcollector.net_min = [
                                                        ("download".to_owned(), -1),
                                                        ("upload".to_owned(), -1),
                                                    ]
                                                    .iter()
                                                    .map(|(s, i)| (s.clone(), *i as i32))
                                                    .collect::<HashMap<String, i32>>();
                                                } else if selected == "draw_clock".to_owned() {
                                                    brshtop_box
                                                        .set_clock_on(CONFIG.draw_clock.len() > 0);
                                                    if !brshtop_box.get_clock_on() {
                                                        draw.clear(vec!["clock".to_owned()], true);
                                                    }
                                                }
                                            }
                                            _ => (),
                                        }
                                        term.refresh(
                                            vec![],
                                            boxes,
                                            collector,
                                            init,
                                            cpubox,
                                            draw,
                                            true,
                                            key_class,
                                            self,
                                            brshtop_box,
                                            timer,
                                            CONFIG,
                                            THEME,
                                            cpucollector,
                                            membox,
                                            netbox,
                                            procbox,
                                        );
                                        self.resized = false;
                                    }
                                }
                            }
                            _ => (),
                        };
                    } else if key == "backspace".to_owned() && input_val.len() > 0 {
                        input_val = input_val[..input_val.len() - 2].to_owned();
                    } else if key == "delete".to_owned() {
                        input_val = String::default();
                    } else if match CONFIG.getattr(selected) {
                        ConfigAttr::String(_) => true,
                        _ => false,
                    } && key.len() == 1
                    {
                        input_val.push_str(key.as_str());
                    } else if match CONFIG.getattr(selected) {
                        ConfigAttr::Int64(_) => true,
                        _ => false,
                    } && match key.trim().parse::<f64>() {
                        Ok(_) => true,
                        _ => false,
                    } {
                        input_val.push_str(key.as_str());
                    }
                } else if key == "q".to_owned() {
                    clean_quit(None, None, key_class, collector, draw, term, CONFIG, None);
                } else if ["escape", "o", "M", "f2"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                {
                    self.close = true;
                    break;
                } else if key == "enter".to_owned()
                    && [
                        "update_ms",
                        "disks_filter",
                        "custom_cpu_name",
                        "net_download",
                        "net_upload",
                        "draw_clock",
                        "tree_depth",
                    ]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&selected)
                {
                    inputting = true;
                    input_val = CONFIG.getattr(selected).to_string();
                } else if key == "left".to_owned()
                    && selected == "update_ms".to_owned()
                    && CONFIG.update_ms - 100 >= 100
                {
                    CONFIG.update_ms -= 100;
                    brshtop_box
                        .draw_update_ms(true, CONFIG, cpubox, key_class, draw, self, THEME, term);
                } else if key == "right".to_owned()
                    && selected == "update_ms".to_owned()
                    && CONFIG.update_ms + 100 <= 86399900
                {
                    CONFIG.update_ms += 100;
                    brshtop_box
                        .draw_update_ms(true, CONFIG, cpubox, key_class, draw, self, THEME, term);
                } else if key == "left".to_owned()
                    && selected == "tree_depth".to_owned()
                    && CONFIG.tree_depth > 0
                {
                    CONFIG.tree_depth -= 1;
                    proc_collector.collapsed = HashMap::<u32, bool>::new();
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && match CONFIG.getattr(selected) {
                        ConfigAttr::Bool(b) => true,
                        _ => false,
                    }
                {
                    CONFIG.setattr_configattr(
                        selected,
                        ConfigAttr::Bool(match CONFIG.getattr(selected) {
                            ConfigAttr::Bool(b) => !b,
                            _ => false,
                        }),
                    );
                    if selected == "check_temp".to_owned() {
                        if CONFIG.check_temp {
                            cpucollector.get_sensors(CONFIG);
                        } else {
                            cpucollector.sensor_method = String::default();
                            cpucollector.got_sensors = false;
                        }
                    }
                    if ["net_auto", "net_color_fixed", "net_sync"]
                        .iter()
                        .map(|s| s.to_owned().to_owned())
                        .collect::<Vec<String>>()
                        .contains(&selected)
                    {
                        if selected == "net_auto".to_owned() {
                            netcollector.auto_min = CONFIG.net_auto;
                        }
                        netbox.set_redraw(true);
                    } else if selected == "theme_background".to_owned() {
                        term.bg = if CONFIG.theme_background {
                            theme.colors.main_bg
                        } else {
                            Color::from("\033[49m".to_owned())
                        };
                        draw.now(vec![term.bg.to_string()], key_class);
                    } else if selected == "show_battery".to_owned() {
                        draw.clear(vec!["battery".to_owned()], true);
                    }
                    term.refresh(
                        vec![],
                        boxes,
                        collector,
                        init,
                        cpubox,
                        draw,
                        true,
                        key_class,
                        self,
                        brshtop_box,
                        timer,
                        CONFIG,
                        THEME,
                        cpucollector,
                        membox,
                        netbox,
                        procbox,
                    );
                    self.resized = true;
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && selected == "color_theme".to_owned()
                    && theme.themes.len() > 1
                {
                    if key == "left".to_owned() {
                        color_i -= 1;
                        if color_i < 0 {
                            color_i = theme.themes.len() - 1;
                        }
                    } else if key == "right".to_owned() {
                        color_i += 1;
                        if color_i > theme.themes.len() - 1 {
                            color_i = 0;
                        }
                    }
                    collector.set_collect_idle(Event::Wait);
                    collector.get_collect_idle_reference().wait(-1.0);
                    CONFIG.color_theme =
                        theme.themes.keys().cloned().collect::<Vec<String>>()[color_i];
                    THEME.replace_self(
                        Theme::from_str(CONFIG.color_theme).unwrap_or(Theme::default()),
                    );
                    term.refresh(
                        vec![],
                        boxes,
                        collector,
                        init,
                        cpubox,
                        draw,
                        true,
                        key_class,
                        self,
                        brshtop_box,
                        timer,
                        CONFIG,
                        THEME,
                        cpucollector,
                        membox,
                        netbox,
                        procbox,
                    );
                    timer.finish(key_class, CONFIG);
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && selected == "proc_sorting".to_owned()
                {
                    // TODO : Need to figure out how to do this...
                    // proc_collector.sorting(key);
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && selected == "log_level".to_owned()
                {
                    if key == "left".to_owned() {
                        loglevel_i -= 1;
                        if loglevel_i < 0 {
                            loglevel_i = CONFIG.log_levels.len() - 1;
                        }
                    } else if key == "right".to_owned() {
                        loglevel_i == 1;
                        if loglevel_i > CONFIG.log_levels.len() - 1 {
                            loglevel_i = 0;
                        }
                    }
                    CONFIG.log_level = CONFIG.log_levels[loglevel_i];
                    // TODO : Implement error logging level
                    error::errlog(
                        ("LogLevel set to ".to_owned() + CONFIG.log_level.to_string().as_str())
                            .to_owned(),
                    );
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && selected == "cpu_sensor".to_owned()
                    && CONFIG.cpu_sensors.len() > 1
                {
                    if key == "left".to_owned() {
                        cpu_sensor_i -= 1;
                        if cpu_sensor_i < 0 {
                            cpu_sensor_i = CONFIG.cpu_sensors.len() - 1;
                        }
                    } else if key == "right".to_owned() {
                        cpu_sensor_i += 1;
                        if cpu_sensor_i < CONFIG.cpu_sensors.len() - 1 {
                            cpu_sensor_i = 0;
                        }
                    }
                    collector.set_collect_idle(Event::Wait);
                    collector.get_collect_idle_reference().wait(-1.0);
                    cpucollector.sensor_swap = true;
                    CONFIG.cpu_sensor = CONFIG.cpu_sensors[cpu_sensor_i];
                    if CONFIG.check_temp
                        && (cpucollector.sensor_method != "psutil".to_owned()
                            || CONFIG.cpu_sensor == "Auto".to_owned())
                    {
                        cpucollector.get_sensors(CONFIG);
                        term.refresh(
                            vec![],
                            boxes,
                            collector,
                            init,
                            cpubox,
                            draw,
                            true,
                            key_class,
                            self,
                            brshtop_box,
                            timer,
                            CONFIG,
                            THEME,
                            cpucollector,
                            membox,
                            netbox,
                            procbox,
                        );
                        self.resized = false;
                    }
                } else if ["left", "right"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && selected == "view_mode".to_owned()
                {
                    if key == "left".to_owned() {
                        view_mode_i -= 1;
                        if view_mode_i < 0 {
                            view_mode_i = CONFIG.view_modes.len() - 1;
                        }
                    } else if key == "right".to_owned() {
                        view_mode_i += 1;
                        if view_mode_i > CONFIG.view_modes.len() - 1 {
                            view_mode_i = 0;
                        }
                    }
                    CONFIG.view_mode = CONFIG.view_modes[view_mode_i];
                    brshtop_box.set_proc_mode(CONFIG.view_mode == ViewMode::Proc);
                    brshtop_box.set_stat_mode(CONFIG.view_mode == ViewMode::Stat);
                    if ARG_MODE != ViewMode::None {
                        ARG_MODE.replace_self(ViewMode::None);
                    }
                    draw.clear(vec![], true);
                    term.refresh(
                        vec![],
                        boxes,
                        collector,
                        init,
                        cpubox,
                        draw,
                        true,
                        key_class,
                        self,
                        brshtop_box,
                        timer,
                        CONFIG,
                        THEME,
                        cpucollector,
                        membox,
                        netbox,
                        procbox,
                    );
                    self.resized = false;
                } else if key == "up".to_owned() {
                    selected_int -= 1;
                    if selected_int < 0 {
                        selected_int = option_items.len() - 1;
                    }
                    page = (selected_int as u32 * 2 / h) as u32 - 1;
                } else if ["mouse_scroll_up", "page_up"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && pages > 0
                {
                    page -= 1;
                    if page < 1 {
                        page = pages;
                    }
                    selected_int = (page - 1) as usize * ceil((h as f64 / 2.0), 0) as usize;
                } else if ["mouse_scroll_down", "page_down"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect::<Vec<String>>()
                    .contains(&key)
                    && pages > 0
                {
                    page += 1;
                    if page > pages {
                        page = pages;
                    }
                    selected_int = (page - 1) as usize * ceil((h as f64 / 2.0), 0) as usize;
                } else if has_sel {
                    // pass
                } else {
                    redraw = false;
                }
            }

            if timer.not_zero(CONFIG) && !self.resized {
                skip = true;
            } else {
                collector.collect(collectors, CONFIG, true, false, false, false, false);
                collector.set_collect_done(Event::Wait);
                collector.get_collect_done_reference().wait(2.0);
                if CONFIG.background_update {
                    self.background = format!(
                        "{}{}{}",
                        THEME.colors.inactive_fg,
                        fx::Fx::uncolor(draw.saved_buffer()),
                        term.fg,
                    );
                }
                timer.stamp();
            }
        }

        if main_active {
            self.close = false;
            return;
        }
        draw.now(vec![draw.saved_buffer()], key_class);
        self.background = String::default();
        self.active = false;
        self.close = false;
    }
}

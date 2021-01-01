use {
    crate::{
        banner,
        clean_quit,
        collector::Collector,
        draw::Draw,
        event::Event,
        fx, mv,
        key::Key,
        term::Term,
        theme::{Color, Colors, Theme},
        timer::Timer,
        updatechecker::UpdateChecker,
    },
    std::{collections::HashMap, iter::FromIterator},
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
                        );
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
        theme: &mut Theme,
        draw: &mut Draw,
        term: &mut Term,
        VERSION: String,
        update_checker: &mut UpdateChecker,
        THEME: &mut Theme,
        key_class : &mut Key,
        timer : &mut Timer,
        collector : &mut Collector,
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
                    banner::draw_banner((term.height / 2) as u32 - 10, 0, true, false, term),
                    mv::down(1),
                    mv::left(46),
                    Color::BlackBg(),
                    Color::default(),
                    fx::b,
                    mv::right(30),
                    fx::i,
                    VERSION,
                    fx::ui,
                    fx::ub,
                    term.bg,
                    term.fg,
                );

                if update_checker.version != VERSION {
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
                    out.push_str(format!(
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
                    ));
                }
            }

            if skip && redraw {
                draw.now(vec![out], key_class);
            } else if !skip {
                draw.now(vec![format!("{}{}{}", self.background, banner_mut, out)], key_class);
            }
            skip = false;
            redraw = false;

            if key_class.input_wait(timer.left(), true, draw, term) {
                if key_class.mouse_moved() {
                    let (mx_set, my_set) = key_class.get_mouse();
                    mx = mx_set;
                    my = my_set;

                    let mut broke : bool = false;
                    for (name, pos) in mouse_items {
                        if pos[&"x1".to_owned()] <= mx && mx <= pos[&"x2".to_owned()] && pos[&"y1".to_owned()] <= my && my <= pos[&"y2".to_owned()] {
                            mouse_over = true;
                            if name != menu_current {
                                menu_current = name;
                                menu_index = menu_names.iter().position(|&r| r == name).unwrap() as usize;
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
                    clean_quit();
                } else if vec!["up", "mouse_scroll_up", "shift_tab"].iter().map(|s| s.clone().to_owned()).collect().contains(key) {
                    menu_index -= 1;
                    if menu_index < 0 {
                        menu_index = menu_names.len() - 1;
                    }
                    menu_current = menu_names[menu_index];
                    redraw = true;
                } else if vec!["down", "mouse_scroll_down", "tab"].iter().map(|s| s.clone().to_owned()).collect().contains(key) {
                    menu_index += 1;
                    if menu_index > menu_names.len() - 1 {
                        menu_index = 0;
                    }
                    menu_current = menu_names[menu_index];
                    redraw = true;
                } else if key == "enter".to_owned() || (key == "mouse_click".to_owned() && mouse_over) {
                    if menu_current == "quit".to_owned() {
                        clean_quit()
                    } else if menu_current == "options".to_owned() {
                        self.options();
                        self.resized = true;
                    } else if menu_current == "help".to_owned() {
                        self.help();
                        self.resized = true;
                    }
                }
            }

            if timer.not_zero() && !self.resized {
                skip = true;
            } else {
                collector.collect();
                collector.collect_done = Event::Wait;
                collector.collect_done.wait(2)
            }
        }
    }
}

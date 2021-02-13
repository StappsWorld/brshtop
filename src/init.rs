use crate::clean_quit;

use {
    crate::{
        banner::draw_banner,
        collector::Collector,
        config::Config,
        draw::Draw,
        error::errlog,
        fx,
        fx::Fx,
        graph::{ColorSwitch, Graph},
        key::Key,
        mv, symbol,
        term::Term,
        theme::Color,
        CONFIG_DIR, VERSION,
    },
    once_cell::sync::OnceCell,
    rand::Rng,
    std::{sync::Mutex, thread, time},
};

pub struct Init {
    pub running: bool,
    pub initbg_colors: Vec<Color>,
    pub initbg_data: Vec<i32>,
    pub initbg_up: Graph,
    pub initbg_down: Graph,
    pub resized: bool,
}
impl Init {
    pub fn new() -> Self {
        Init {
            running: true,
            initbg_colors: Vec::<Color>::new(),
            initbg_data: Vec::<i32>::new(),
            initbg_up: Graph::default(),
            initbg_down: Graph::default(),
            resized: false,
        }
    }

    pub fn start(&mut self, draw: &Draw, key: &mut Key, term: &Term) {
        draw.buffer(
            "init".to_owned(),
            vec![],
            false,
            false,
            0,
            false,
            false,
            false,
            key,
        );
        draw.buffer(
            "initbg".to_owned(),
            vec![],
            false,
            false,
            10,
            false,
            false,
            false,
            key,
        );
        for i in 0..51 {
            for _ in 0..2 {
                self.initbg_colors
                    .push(Color::fg(format!("{} {} {}", i, i, i)).unwrap_or(Color::default()));
            }
        }

        let insert_height = term.get_height();
        drop(draw);
        drop(term);
        let inserter = draw_banner(
            ((insert_height / 2) - 10) as u32,
            0,
            true,
            false,
            term,
            draw,
            key,
        );
        draw.buffer(
            "+banner".to_owned(),
            vec![format!(
                "{}{}{}{}{}{}{}Version: {}{}{}{}{}{}",
                inserter,
                mv::down(1),
                mv::left(11),
                Color::BlackBg(),
                Color::default(),
                fx::b,
                fx::i,
                VERSION.to_owned(),
                fx::ui,
                fx::ub,
                term.get_bg(),
                term.get_fg(),
                Color::fg("#50").unwrap_or(Color::default())
            )],
            false,
            false,
            2,
            false,
            false,
            false,
            key,
        );
        for i in 0..7 {
            let perc = format!("{:>5}", ((i + 1) * 14 + 2).to_string() + "%");
            draw.buffer(
                "+banner".to_owned(),
                vec![format!(
                    "{}{}{}",
                    mv::to(
                        (term.get_height() / 2) as u32 - 2 + i,
                        (term.get_width() as u32 / 2) - 28
                    ),
                    Fx::trans(perc),
                    symbol::v_line,
                )],
                false,
                false,
                100,
                false,
                false,
                false,
                key,
            );
        }
        draw.out(vec!["banner".to_owned()], false, key);
        draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}{}{}",
                Color::fg("#cc").unwrap_or(Color::default()),
                fx::b,
                mv::to(
                    (term.get_height() as u32 / 2) - 2,
                    (term.get_width() as u32 / 2) - 21
                ),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            key,
        );

        let mut rand: Vec<i32> = Vec::<i32>::new();
        let mut rng = rand::thread_rng();
        for _ in 0..term.get_width() * 2 {
            rand.push(rng.gen_range(0..100));
        }
        self.initbg_data = rand.clone();

        let insert_width = term.get_width();
        let insert_height = term.get_height();

        self.initbg_up = Graph::new(
            insert_width as i32,
            insert_height as i32 / 2,
            Some(ColorSwitch::VecColor(self.initbg_colors.clone())),
            self.initbg_data.clone(),
            term,
            true,
            0,
            0,
            None,
        );
        self.initbg_down = Graph::new(
            insert_width as i32,
            insert_height as i32 / 2,
            Some(ColorSwitch::VecColor(self.initbg_colors.clone())),
            self.initbg_data.clone(),
            term,
            false,
            0,
            0,
            None,
        );
    }

    pub fn success(
        &mut self,
        CONFIG_p: &OnceCell<Mutex<Config>>,
        draw: &OnceCell<Mutex<Draw>>,
        term: &OnceCell<Mutex<Term>>,
        key: &OnceCell<Mutex<Key>>,
    ) {
        let mut CONFIG = CONFIG_p.get().unwrap().try_lock().unwrap();
        if !CONFIG.show_init || self.resized {
            return;
        }
        let temp_term = term.get().unwrap().try_lock().unwrap();
        let inserter = temp_term.get_width();
        drop(temp_term);
        self.draw_bg(5, draw, term, key);

        let mut draw = draw.get().unwrap().try_lock().unwrap();
        draw.buffer(
            "+init!".to_owned(),
            vec![format!(
                "{}{}\n{}{}",
                mv::restore,
                symbol::ok(),
                mv::right((inserter as u32 / 2) - 22),
                mv::save
            )],
            false,
            false,
            100,
            false,
            false,
            false,
            key,
        );
    }

    pub fn fail(
        err: String,
        CONFIG_p: &OnceCell<Mutex<Config>>,
        draw: &OnceCell<Mutex<Draw>>,
        collector: &OnceCell<Mutex<Collector>>,
        key: &OnceCell<Mutex<Key>>,
        term: &OnceCell<Mutex<Term>>,
    ) {
        let CONFIG = CONFIG_p.get().unwrap().try_lock().unwrap();
        let mut draw = draw.get().unwrap().try_lock().unwrap();

        if CONFIG.show_init {
            draw.buffer(
                "+init!".to_owned(),
                vec![format!("{}{}", mv::restore, symbol::fail())],
                false,
                false,
                100,
                false,
                false,
                false,
                key,
            );
            thread::sleep(time::Duration::from_secs(2));
        }
        errlog(err);

        drop(CONFIG);
        drop(draw);
        clean_quit(
            Some(1),
            Some(format!(
                "Error during init! See {}/error.log for more information.",
                CONFIG_DIR.to_owned().to_str().unwrap()
            )),
            key,
            collector,
            draw,
            term,
            CONFIG_p,
        );
    }

    /// Defaults times : 5
    pub fn draw_bg(
        &mut self,
        times: u32,
        draw: &OnceCell<Mutex<Draw>>,
        term: &OnceCell<Mutex<Term>>,
        key: &OnceCell<Mutex<Key>>,
    ) {
        let mut draw = draw.get().unwrap().try_lock().unwrap();
        for _ in 0..times {
            thread::sleep(time::Duration::from_secs_f32(0.05));
            let mut rng = rand::thread_rng();
            let x: u32 = rng.gen_range(0..100);
            let term_temp = term.get().unwrap().try_lock().unwrap();
            let inserter = term_temp.get_height();
            drop(term_temp);
            draw.buffer(
                "initbg".to_owned(),
                vec![format!(
                    "{}{}{}{}{}",
                    fx::ub,
                    mv::to(0, 0),
                    self.initbg_up.call(Some(x as i32), term),
                    mv::to(inserter as u32 / 2, 0),
                    self.initbg_down.call(Some(x as i32), term)
                )],
                false,
                false,
                100,
                false,
                false,
                false,
                key,
            );
            let mut pass_key = key.get().unwrap().try_lock().unwrap();
            draw.out(
                vec!["initbg", "banner", "init"]
                    .iter()
                    .map(|s| s.to_owned().to_owned())
                    .collect(),
                false,
                &mut pass_key,
            );
            drop(pass_key);
        }
    }

    pub fn done(
        &mut self,
        CONFIG_p: &OnceCell<Mutex<Config>>,
        draw: &OnceCell<Mutex<Draw>>,
        term: &OnceCell<Mutex<Term>>,
        key: &OnceCell<Mutex<Key>>,
    ) {
        let mut CONFIG = CONFIG_p.get().unwrap().try_lock().unwrap();
        let mut draw = draw.get().unwrap().try_lock().unwrap();
        let mut term = term.get().unwrap().try_lock().unwrap();

        self.running = false;
        if !CONFIG.show_init {
            return;
        }
        if self.resized {
            let mut pass_key = key.get().unwrap().try_lock().unwrap();
            draw.now(vec![term.get_clear()], &mut pass_key);
            drop(pass_key);
        } else {
            drop(draw);
            drop(term);
            self.draw_bg(10, draw, term, key);
            draw = draw.get().unwrap().try_lock().unwrap();
            term = term.get().unwrap().try_lock().unwrap();
        }
        draw.clear(
            vec!["initbg", "banner", "init"]
                .iter()
                .map(|s| s.to_owned().to_owned())
                .collect(),
            true,
        );
        if self.resized {
            return;
        }
        self.initbg_up = Graph::default();
        self.initbg_down = Graph::default();
        self.initbg_data = vec![];
        self.initbg_colors = vec![];
    }
}

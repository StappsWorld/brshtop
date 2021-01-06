use {
    crate::{
        create_box, 
        error::throw_error,
        brshtop_box::{
            BrshtopBox,
            Boxes,
        },
        config::{
            Config, 
            ViewMode,
        },
        fx,
        key::Key,
        mv,
        netcollector::{
            NetCollector,
            NetCollectorStat,
        },
        subbox::SubBox,
        symbol,
        term::Term,
        theme::Theme,
    },
    std::{
        convert::TryFrom,
        collections::HashMap,
    },
};

pub struct NetBox {
    pub parent : BrshtopBox,
    pub sub : SubBox,
    pub name : String,
    pub height_p : u32,
    pub width_p : u32,
    pub x : i32,
    pub y : i32,
    pub resized : bool,
    pub redraw : bool,
    pub graph_height : HashMap<String, u32>,
    pub symbols : HashMap<String, String>,
    pub buffer : String,
} impl NetBox {

    pub fn new(CONFIG : &mut Config, ARG_MODE: ViewMode, brshtop_box : &mut BrshtopBox) -> Self {
        let net = NetBox {
            parent : BrshtopBox::new(CONFIG, ARG_MODE),
            sub : SubBox::new(),
            name : "net".to_owned(),
            height_p : 30,
            width_p : 45,
            x : 1,
            y : 1,
            resized : true,
            redraw : true,
            graph_height : HashMap::<String, u32>::new(),
            symbols : [("download", "▼"), ("upload", "▲")].iter().map(|(s1, s2)| (s1.to_owned(), s2.to_owned())).collect(),
            buffer : "net".to_owned(),
        };
        
        brshtop_box.buffers.push(net.buffer);
        
        net
    }

    pub fn calc_size(&mut self, term : &mut Term, brshtop_box : &mut BrshtopBox) {
        let mut width_p : u32 = 0;

        if self.parent.stat_mode {
            width_p = 100;
        } else {
            width_p = self.width_p;
        }
        self.parent.width = (term.width * width_p / 100) as u32;
        self.parent.height = u32::try_from(term.height as i32 - brshtop_box._b_cpu_h - brshtop_box._b_mem_h).unwrap_or(0);
        self.y = (term.height - self.parent.height + 1) as i32;
        self.sub.box_width = if self.parent.width > 45 {
            27
        } else {
            19
        };
        self.sub.box_height = if self.parent.height > 10 {
            9
        } else {
            self.parent.height - 2
        };
        self.parent.box_x = self.parent.width - self.box_width - 1;
        self.parent.box_y = self.y + ((self.parent.height - 2) / 2) as u32 - (self.sub.box_height / 2) as u32 + 1;
        self.graph_height.insert("download".to_owned(), ((self.height - 2) as f64 / 2.0).round() as u32);
        self.graph_height.insert("upload".to_owned(), self.height - 2 - self.graph_height["download".to_owned()]);
        self.redraw = true;
    }

    pub fn draw_bg(&mut self, theme : &mut Theme) -> String {
        if self.parent.proc_mode {
            return String::default();
        }
        format!("{}{}",
            create_box(
                0,
                0, 
                0, 
                0, 
                None, 
                None, 
                Some(theme.colors.net_box), 
                None, 
                true, 
                Some(Boxes::NetBox(self))
            ),
            create_box(
                self.sub.box_x, 
                self.sub.box_y, 
                self.sub.box_width, 
                self.sub.box_height, 
                Some("Download".to_owned()), 
                Some("Upload".to_owned()),
                Some(theme.colors.div_line), 
                None, 
                false, 
                None
            )
        )
    }

    pub fn draw_fg(&mut self, theme : &mut Theme, key : &mut Key, term : &mut Term) {
        if self.parent.proc_mode {
            return
        }

        let mut net : NetCollector = NetCollector::new();
        if net.parent.redraw {
            self.redraw = true;
        }
        if net.nic.len() == 0 {
            return
        }

        let mut out : String = String::default();
        let mut out_misc : String = String::default();
        let x = self.x + 1;
        let y = self.y + 1;
        let w = self.parent.width - 2;
        let h = self.parent.height - 2;
        let bx = self.sub.box_x + 1;
        let by = self.sub.box_y + 1;
        let bw = self.sub.box_width - 2;
        let bh = self.sub.box_height - 2;
        let reset : bool = match net.stats[net.nic]["download".to_owned()]["offset".to_owned()] {
            NetCollectorStat::Bool(b) => b,
            NetCollectorStat::Int(i) => i > 0,
            NetCollectorStat::Vec(v) => v.len() > 0,
        };
        
        if self.resized || self.redraw {
            out_misc.push_str(self.draw_bg(theme).as_str());
            if key.mouse.contains_key("b".to_owned()) {
                let mut b_vec_top : Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut b_insert : Vec<i32> = Vec::<i32>::new();
                    b_insert.push(x + w as i32 - net.nic[..10].len() as i32 - 9 + i);
                    b_insert.push(y - 1);
                    b_vec_top.push(b_insert);
                }

                key.mouse.insert("b".to_owned(), b_vec_top);

                let mut n_vec_top : Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut n_insert : Vec<i32> = Vec::<i32>::new();
                    n_insert.push(x + w as i32 -5 + i);
                    n_insert.push(y - 1);
                    n_vec_top.push(n_insert);
                }

                key.mouse.insert("n".to_owned(), n_vec_top);

                let mut z_vec_top : Vec<Vec<i32>> = Vec::<Vec<i32>>::new();

                for i in 0..4 {
                    let mut z_insert : Vec<i32> = Vec::<i32>::new();
                    z_insert.push(x + w as i32 - net.nix[..10].len() as i32 - 14 + i);
                    z_insert.push(y - 1);
                    z_vec_top.push(z_insert);
                }

                key.mouse.insert("z".to_owned(), z_vec_top);
            }
            out_misc.push_str(format!("{}{}{}{}{}{}{}{}{}{}{}{}{}{} {} {}{}{}{}",
                    mv::to(y as u32 - 1, x as u32 + w - 25),
                    theme.colors.net_box,
                    symbol::h_line.repeat(
                        10 - net.nic[..10].len()
                    ),
                    symbol::title_left,
                    if reset {
                        fx::bold
                    } else {
                        ""
                    },
                    theme.colors.hi_fg.call("z".to_owned(), term),
                    theme.colors.title.call("ero".to_owned(), term),
                    fx::ub,
                    theme.colors.net_box.call(symbol::title_right,term),
                    term.fg,
                    theme.colors.net_box,
                    symbol::title_left,
                    fx::b,
                    theme.colors.hi_fg.call("<b".to_owned(), term),
                    theme.colors.title.call(net.nix[..10], term),
                    theme.colors.hi_fg.call("n>", term),
                    fx::ub,
                    theme.colors.net_box.call(symbol::title_right, term),
                    term.fg,
                )
                .as_str()
            );

            if (w as usize) - net.nix[..10].len() - 20 > 6 {
                if !key.mouse.contains_key("a".to_owned()) {
                    let mut inserter_top : Vec<Vec<i32>> = Vec::<Vec<i32>>::new();
                    for i in 0..4 {
                        let mut inserter : Vec<i32> = Vec::<i32>::new();

                        inserter.push(x + w - 20 - net.nic[..10].len() + i);
                        inserter.push(y - 1);
                        inserter_top.push(inserter);
                    }
                    key.mouse.insert("a".to_owned(), inserter_top);
                }
                out_misc.push_str(format!("{}{}{}{}{}{}{}{}",
                        mv::to((y as u32) - 1, (x as u32) + w - 21 - net.nix[..10].len() as u32),
                        theme.colors.net_box.call(symbol::title_left),
                        if net.auto_min {
                            fx::b
                        } else {
                            ""
                        },
                        theme.colors.hi_fg.call("a".to_owned(), term),
                        theme.colors.title.call("uto".to_owned(), term),
                        fx::ub,
                        theme.colors.net_box.call(symbol::title_right, term),
                        term.fg,
                    )
                    .as_str()
                );
            }

        }

    }

}
use crate::config::ViewMode;

use {
    crate::{
        brshtop_box::BrshtopBox,
        config::{Config, ViewMode},
        subbox::SubBox,
        term::Term,
    },
    std::{
        convert::TryFrom,
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

    pub fn calc_size(term : &mut Term, brshtop_box : &mut BrshtopBox) {
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
    }

}
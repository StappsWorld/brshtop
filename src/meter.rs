use {
    crate::{
        graph::Graph,
        theme::{Color, Theme},
        symbol,
        term::Term,
    },
    std::{
        collections::HashMap,
        string::ToString,
    },
};

pub enum MeterUnion {
    Meter(Meter),
    Graph(Graph),
}

pub struct Meters {
    pub cpu : Meter,
    pub battery : Meter,
    pub mem : HashMap<String, MeterUnion>,
    pub swap : HashMap<String, MeterUnion>,
    pub disk_used : HashMap<String, Meter>,
    pub disk_free : HashMap<String, Meter>,
}

pub struct Meter {
    pub out : String,
    pub color_gradient : Vec<String>,
    pub color_inactive : Color,
    pub gradient_name : String,
    pub width : u32,
    pub invert : bool,
    pub saved : HashMap<i32, String>,
} impl Meter {

    /// Defaults invert : bool = false
    pub fn new(value : i32, width : u32, gradient_name : String, invert : bool, THEME : &mut Theme, term : &mut Term) -> Self {
        let meter = Meter{
            out : gradient_name,
            color_gradient : THEME.gradient[&gradient_name],
            color_inactive : THEME.colors.meter_bg,
            gradient_name : String::default(),
            width : width,
            invert : invert,
            saved : HashMap::<i32, String>::new(),
        };

        meter.out = meter._create(value, term);
        meter
    }

    pub fn call(&mut self, value : Option<i32>, term : &mut Term) -> String {
        match value {
            Some(i) => {
                let mut new_val : i32 = 0;
                if i > 100 {
                    new_val = 100;
                } else if i < 0 {
                    new_val = 100;
                }
                if self.saved.contains_key(&new_val) {
                    self.out = self.saved[&new_val];
                } else {
                    self.out = self._create(new_val, term);
                }
                self.out
            }
            None => self.out,
        }
    }

    pub fn _create(&mut self, value : i32, term : &mut Term) -> String {
        let mut new_value : i32 = 0;
        if value > 100 {
            new_value = 100;
        } else if value < 0 {
            new_value = 100;
        }
        let mut out : String = String::default();
        let mut broke : bool = false;
        for i in 1..self.width + 1 {
            if value >= (i * 100 / self.width) as i32 {
                out.push_str(format!("{}{}",
                        self.color_gradient[
                            if !self.invert {
                                (i * 100 / self.width) as usize
                            } else {
                                100 - (i * 100 / self.width) as usize
                            }
                        ],
                        symbol::meter,
                    )
                    .as_str()
                );
            } else {
                out.push_str(
                    self.color_inactive.call(
                        symbol::meter
                                .repeat(
                                    (self.width + 1 - i) as usize), 
                                    term)
                                .to_string()
                                .as_str()
                );

                broke = true;
                break;
            }
        }
        if !broke {
            out.push_str(term.fg.to_string().as_str());
        }

        if self.saved.contains_key(&new_value) {
            self.saved[&new_value] = out;
        }

        out
    }



} impl ToString for Meter {
    fn to_string(&self) -> String {
        self.out
    }
}
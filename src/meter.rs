use {
    crate::{
        graph::Graph,
        theme::{Color, Theme},
        symbol,
        term::Term,
    },
    once_cell::sync::OnceCell,
    std::{
        collections::HashMap,
        fmt,
        sync::Mutex,
    },
};

#[derive(Clone)]
pub enum MeterUnion {
    Meter(Meter),
    Graph(Graph),
}

#[derive(Default)]
pub struct Meters {
    cpu : Meter,
    battery : Meter,
    mem : HashMap<String, MeterUnion>,
    swap : HashMap<String, MeterUnion>,
    disks_used : HashMap<String, Meter>,
    disks_free : HashMap<String, Meter>,
} impl Meters {
    pub fn get_cpu(&self) -> Meter {
        self.cpu.clone()
    }

    pub fn set_cpu(&mut self, cpu : Meter) {
        self.cpu = cpu.clone()
    }

    pub fn get_battery(&self) -> Meter {
        self.battery.clone()
    }

    pub fn set_battery(&mut self, battery : Meter) {
        self.battery = battery.clone()
    }

    pub fn get_mem(&self) -> HashMap<String, MeterUnion> {
        self.mem.clone()
    }

    pub fn set_mem(&mut self, mem : HashMap<String, MeterUnion>) {
        self.mem = mem.clone()
    }

    pub fn get_mem_index(&self, index : String) -> Option<MeterUnion> {
        match self.get_mem().get(&index.clone()) {
            Some(m) => Some(m.clone()),
            None => None,
        }
    }

    pub fn set_mem_index(&mut self, index : String, element : MeterUnion) {
        self.mem.insert(index.clone(), element.clone());
    }

    pub fn get_swap(&self) -> HashMap<String, MeterUnion> {
        self.swap.clone()
    }

    pub fn set_swap(&mut self, swap : HashMap<String, MeterUnion>) {
        self.swap = swap.clone()
    }

    pub fn get_swap_index(&self, index : String) -> Option<MeterUnion> {
        match self.get_swap().get(&index.clone()) {
            Some(m) => Some(m.clone()),
            None => None,
        }
    }

    pub fn set_swap_index(&mut self, index : String, element : MeterUnion) {
        self.swap.insert(index.clone(), element.clone());
    }

    pub fn get_disks_used(&self) -> HashMap<String, Meter> {
        self.disks_used.clone()
    }

    pub fn set_disks_used(&mut self, disks_used : HashMap<String, Meter>) {
        self.disks_used = disks_used.clone()
    }

    pub fn get_disks_used_index(&self, index : String) -> Option<Meter> {
        match self.get_disks_used().get(&index.clone()) {
            Some(m) => Some(m.clone()),
            None => None,
        }
    }

    pub fn set_disks_used_index(&mut self, index : String, element : Meter) {
        self.disks_used.insert(index.clone(), element.clone());
    }

    pub fn get_disks_free(&self) -> HashMap<String, Meter> {
        self.disks_free.clone()
    }

    pub fn set_disks_free(&mut self, disks_free : HashMap<String, Meter>) {
        self.disks_free = disks_free.clone()
    }

    pub fn get_disks_free_index(&self, index : String) -> Option<Meter> {
        match self.get_disks_free().get(&index.clone()) {
            Some(m) => Some(m.clone()),
            None => None,
        }
    }

    pub fn set_disks_free_index(&mut self, index : String, element : Meter) {
        self.disks_free.insert(index.clone(), element.clone());
    }

}

#[derive(Default, Clone)]
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
    pub fn new(value : i32, width : u32, gradient_name : String, invert : bool, THEME : &Theme, term : &OnceCell<Mutex<Term>>) -> Self {
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

    pub fn call(&mut self, value : Option<i32>, term : &OnceCell<Mutex<Term>>) -> String {
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

    pub fn _create(&mut self, value : i32, term : &OnceCell<Mutex<Term>>) -> String {
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
            out.push_str(term.get().unwrap().lock().unwrap().get_fg().to_string().as_str());
        }

        if self.saved.contains_key(&new_value) {
            self.saved[&new_value] = out;
        }

        out
    }



} impl fmt::Display for Meter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.out.clone())
    }
}
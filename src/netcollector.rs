use {
    crate::{
        collector::Collector,
        netbox::NetBox,
    },
    std::collections::HashMap,
};




pub enum NetCollectorStat {
    Int(i32),
    Vec(Vec<i32>),
    Bool(bool),
}

pub struct NetCollector {
    parent : Collector,
    buffer: String,
    nics: Vec<String>,
    nic_i: usize,
    nic: String,
    new_nic: String,
    nic_error: bool,
    reset: bool,
    graph_raise: HashMap<String, i32>,
    graph_lower: HashMap<String, i32>,
    stats: HashMap<String, HashMap<String, HashMap<String, NetCollectorStat>>>,
    strings: HashMap<String, HashMap<String, HashMap<String, String>>>,
    switched: bool,
    timestamp: f64,
    net_min: HashMap<String, i32>,
    auto_min: bool,
    sync_top: i32,
    sync_string: String,
} impl NetCollector {

    pub fn new() -> Self {
        NetCollector
    }

    pub fn get_nics(&mut self) {

    }

    pub fn switch(&mut self) {

    }

    pub fn collect(&mut self) {

    }

    pub fn draw(&mut self, netbox : &mut NetBox) {
        netbox.draw_fg()
    }
}
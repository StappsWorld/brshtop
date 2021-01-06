use {
    crate::{
        collector::Collector,
        netbox::NetBox,
    },
    std::collections::HashMap,
};



#[derive(Clone, Debug, PartialEq)]
pub enum NetCollectorStat {
    Int(i32),
    Vec(Vec<i32>),
    Bool(bool),
}

#[derive(Clone)]
pub struct NetCollector {
    pub parent : Collector,
    pub buffer: String,
    pub nics: Vec<String>,
    pub nic_i: usize,
    pub nic: String,
    pub new_nic: String,
    pub nic_error: bool,
    pub reset: bool,
    pub graph_raise: HashMap<String, i32>,
    pub graph_lower: HashMap<String, i32>,
    pub stats: HashMap<String, HashMap<String, HashMap<String, NetCollectorStat>>>,
    pub strings: HashMap<String, HashMap<String, HashMap<String, String>>>,
    pub switched: bool,
    pub timestamp: f64,
    pub net_min: HashMap<String, i32>,
    pub auto_min: bool,
    pub sync_top: i32,
    pub sync_string: String,
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

    /// JUST CALL NETBOX.draw_fg()
    pub fn draw(&mut self) {}
}
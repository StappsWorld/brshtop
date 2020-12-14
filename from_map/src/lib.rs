use std::collections::HashMap;

pub trait FromMap {
    type Value: Default + Copy;
    fn from_map(map: HashMap<String, Self::Value>) -> Self;
}

pub trait FromMapDefault: Default {
    type Value: Default + Copy + From<String>;
    fn from_map_default(map: HashMap<String, Self::Value>) -> Self;
    fn default_map() -> HashMap<String, Self::Value>;
}

pub use from_map_derive::*;

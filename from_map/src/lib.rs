use std::collections::HashMap;

pub trait FromMap {
    type Value: Default + Copy;
    fn from_map(map: HashMap<String, Self::Value>) -> Self;
}

pub use from_map_derive::*;

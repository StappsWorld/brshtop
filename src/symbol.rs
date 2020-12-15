use crate::theme::Color;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref graph_up: HashMap<i32, &'static str> = {
        let mut map = HashMap::new();
        map.insert(0, " ");
        map.insert(1, "⢀");
        map.insert(2, "⢠");
        map.insert(3, "⢰");
        map.insert(4, "⢸");
        map.insert(10, "⡀");
        map.insert(11, "⣀");
        map.insert(12, "⣠");
        map.insert(13, "⣰");
        map.insert(14, "⣸");
        map.insert(20, "⡄");
        map.insert(21, "⣄");
        map.insert(22, "⣤");
        map.insert(23, "⣴");
        map.insert(24, "⣼");
        map.insert(30, "⡆");
        map.insert(31, "⣆");
        map.insert(32, "⣦");
        map.insert(33, "⣶");
        map.insert(34, "⣾");
        map.insert(40, "⡇");
        map.insert(41, "⣇");
        map.insert(42, "⣧");
        map.insert(43, "⣷");
        map.insert(44, "⣿");
        map
    };
    pub static ref graph_up_small: HashMap<i32, &'static str> = {
        let mut map = graph_up.clone();
        map.insert(0, "\033[1C");
        map
    };
    pub static ref graph_down: HashMap<i32, &'static str> = {
        let mut map = HashMap::new();
        map.insert(00, " ");
        map.insert(01, "⠈");
        map.insert(02, "⠘");
        map.insert(03, "⠸");
        map.insert(04, "⢸");
        map.insert(10, "⠁");
        map.insert(11, "⠉");
        map.insert(12, "⠙");
        map.insert(13, "⠹");
        map.insert(14, "⢹");
        map.insert(20, "⠃");
        map.insert(21, "⠋");
        map.insert(22, "⠛");
        map.insert(23, "⠻");
        map.insert(24, "⢻");
        map.insert(30, "⠇");
        map.insert(31, "⠏");
        map.insert(32, "⠟");
        map.insert(33, "⠿");
        map.insert(34, "⢿");
        map.insert(40, "⡇");
        map.insert(41, "⡏");
        map.insert(42, "⡟");
        map.insert(43, "⡿");
        map.insert(44, "⣿");
        map
    };
    pub static ref graph_down_small: HashMap<i32, &'static str> = {
        let mut map = graph_down.clone();
        map.insert(0, "\033[1C");
        map
    };
    pub static ref ok: String = format!(
        "{}√{}",
        Color::fg("#30ff50").unwrap(),
        Color::fg("#cc").unwrap()
    );
    pub static ref fail: String = format!(
        "{}!{}",
        Color::fg("#ff3050").unwrap(),
        Color::fg("#cc").unwrap()
    );
}
pub const meter: &'static str = "■";
pub const up: &'static str = "↑";
pub const down: &'static str = "↓";
pub const left: &'static str = "←";
pub const right: &'static str = "→";
pub const enter: &'static str = "↲";
pub const h_line: &'static str = "─";
pub const v_line: &'static str = "│";
pub const left_up: &'static str = "┌";
pub const right_up: &'static str = "┐";
pub const left_down: &'static str = "└";
pub const right_down: &'static str = "┘";
pub const title_left: &'static str = "┤";
pub const title_right: &'static str = "├";
pub const div_up: &'static str = "┬";
pub const div_down: &'static str = "┴";

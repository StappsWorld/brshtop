pub fn right(n: u32) -> String {
    format!("\033[{}C", n)
}

pub fn left(n: u32) -> String {
    format!("\033[{}D", n)
}

pub fn up(n: u32) -> String {
    format!("\033[{}A", n)
}

pub fn down(n: u32) -> String {
    format!("\033[{}B", n)
}

pub fn to(line: u32, col: u32) -> String {
    format!("\033[{};{}f", line, col)
}

pub const save : &'static str = "\033[s";
pub const restore : &'static str = "\033[u";
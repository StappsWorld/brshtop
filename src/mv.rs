pub fn right(n: u32) -> String {
    format!("\x1b[{}C", n)
}

pub fn left(n: u32) -> String {
    format!("\x1b[{}D", n)
}

pub fn up(n: u32) -> String {
    format!("\x1b[{}A", n)
}

pub fn down(n: u32) -> String {
    format!("\x1b[{}B", n)
}

pub fn to(line: u32, col: u32) -> String {
    format!("\x1b[{};{}f", line, col)
}

pub const save : &'static str = "\x1b[s";
pub const restore : &'static str = "\x1b[u";
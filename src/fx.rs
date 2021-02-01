use regex::Regex;


pub const start : &'static str = "\x1b[";  // Escape sequence start
pub const sep : &'static str = ";";  // Escape sequence separator
pub const end : &'static str = "m";  // Escape sequence end

// Reset foreground/background color and text effects
pub const reset : &'static str = "\x1b[0m";
pub const rs : &'static str = "\x1b[0m";
pub const bold : &'static str = "\x1b[1m";
pub const b : &'static str = "\x1b[1m";  // Bold on
pub const unbold : &'static str = "\x1b[22m";
pub const ub : &'static str = "\x1b[22m"; // Bold off
pub const dark : &'static str = "\x1b[2m";
pub const d : &'static str = "\x1b[2m"; // Dark on
pub const undark : &'static str = "\x1b[22m";
pub const ud : &'static str = "\x1b[22m"; // Dark off
pub const italic : &'static str = "\x1b[3m";
pub const i : &'static str = "\x1b[3m"; // Italic on
pub const unitalic : &'static str = "\x1b[23m";
pub const ui : &'static str = "\x1b[23m";  // Italic off
pub const underline : &'static str = "\x1b[4m";
pub const u : &'static str = "\x1b[4m"; // Underline on
pub const ununderline : &'static str = "\x1b[24m";
pub const uu : &'static str = "\x1b[24m"; // Underline off
pub const blink : &'static str = "\x1b[5m";
pub const bl : &'static str = "\x1b[5m"; // Blink on
pub const unblink : &'static str = "\x1b[25m";
pub const ubl : &'static str = "\x1b[25m";  // Blink off
pub const strike : &'static str = "\x1b[9m";
pub const s : &'static str = "\x1b[9m";// Strike / crossed-out on
pub const unstrike : &'static str = "\x1b[29m";
pub const us : &'static str = "\x1b[29m"; // Strike / crossed-out off
 
// Precompiled regex for finding a 24-bit color escape sequence in a string
// let color_re : Regex = Regex::new(r"\x1b\[\d+;\d?;?\d*;?\d*;?\d*m").unwrap();

pub struct Fx {}
impl Fx {
    
    /// Regex for finding a 24-bit color escape sequence in a string
    pub fn color_re() -> Regex {
        let re : Regex = Regex::new(r"\x1b\[\d+;\d?;?\d*;?\d*;?\d*m").unwrap();
        re
    }

    pub fn trans(string : String) -> String {
        return string.replace(" ", "\x1b[1C").clone();
    }

    pub fn uncolor(string : String) -> String {
        format!("{}", Fx::color_re().replace_all("", string.as_str()))
    }
}
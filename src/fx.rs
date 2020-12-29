use regex::Regex;


pub const start : &'static str = "\033[";  // Escape sequence start
pub const sep : &'static str = ";";  // Escape sequence separator
pub const end : &'static str = "m";  // Escape sequence end

// Reset foreground/background color and text effects
pub const reset : &'static str = "\033[0m";
pub const rs : &'static str = "\033[0m";
pub const bold : &'static str = "\033[1m";
pub const b : &'static str = "\033[1m";  // Bold on
pub const unbold : &'static str = "\033[22m";
pub const ub : &'static str = "\033[22m"; // Bold off
pub const dark : &'static str = "\033[2m";
pub const d : &'static str = "\033[2m"; // Dark on
pub const undark : &'static str = "\033[22m";
pub const ud : &'static str = "\033[22m"; // Dark off
pub const italic : &'static str = "\033[3m";
pub const i : &'static str = "\033[3m"; // Italic on
pub const unitalic : &'static str = "\033[23m";
pub const ui : &'static str = "\033[23m";  // Italic off
pub const underline : &'static str = "\033[4m";
pub const u : &'static str = "\033[4m"; // Underline on
pub const ununderline : &'static str = "\033[24m";
pub const uu : &'static str = "\033[24m"; // Underline off
pub const blink : &'static str = "\033[5m";
pub const bl : &'static str = "\033[5m"; // Blink on
pub const unblink : &'static str = "\033[25m";
pub const ubl : &'static str = "\033[25m";  // Blink off
pub const strike : &'static str = "\033[9m";
pub const s : &'static str = "\033[9m";// Strike / crossed-out on
pub const unstrike : &'static str = "\033[29m";
pub const us : &'static str = "\033[29m"; // Strike / crossed-out off
 
// Precompiled regex for finding a 24-bit color escape sequence in a string
// let color_re : Regex = Regex::new(r"\033\[\d+;\d?;?\d*;?\d*;?\d*m").unwrap();

pub struct Fx {}
impl Fx {
    
    /// Regex for finding a 24-bit color escape sequence in a string
    pub fn color_re() -> Regex {
        let re : Regex = Regex::new(r"\033\[\d+;\d?;?\d*;?\d*;?\d*m").unwrap();
        re
    }

    pub fn trans(string : String) -> String {
        return string.replace(" ", "\033[1C").clone();
    }

    pub fn uncolor(&mut self, string : String) -> String {
        format!("{}", Fx::color_re().replace_all("", string.as_str()))
    }
}
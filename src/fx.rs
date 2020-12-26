use regex::Regex;


const start : &'static str = "\033[";  // Escape sequence start
const sep : &'static str = ";";  // Escape sequence separator
const end : &'static str = "m";  // Escape sequence end

// Reset foreground/background color and text effects
const reset : &'static str = "\033[0m";
const rs : &'static str = "\033[0m";
const bold : &'static str = "\033[1m";
const b : &'static str = "\033[1m";  // Bold on
const unbold : &'static str = "\033[22m";
const ub : &'static str = "\033[22m"; // Bold off
const dark : &'static str = "\033[2m";
const d : &'static str = "\033[2m"; // Dark on
const undark : &'static str = "\033[22m";
const ud : &'static str = "\033[22m"; // Dark off
const italic : &'static str = "\033[3m";
const i : &'static str = "\033[3m"; // Italic on
const unitalic : &'static str = "\033[23m";
const ui : &'static str = "\033[23m";  // Italic off
const underline : &'static str = "\033[4m";
const u : &'static str = "\033[4m"; // Underline on
const ununderline : &'static str = "\033[24m";
const uu : &'static str = "\033[24m"; // Underline off
const blink : &'static str = "\033[5m";
const bl : &'static str = "\033[5m"; // Blink on
const unblink : &'static str = "\033[25m";
const ubl : &'static str = "\033[25m";  // Blink off
const strike : &'static str = "\033[9m";
const s : &'static str = "\033[9m";// Strike / crossed-out on
const unstrike : &'static str = "\033[29m";
const us : &'static str = "\033[29m"; // Strike / crossed-out off
 
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
use regex::Regex;


const start : &str = "\033[";  // Escape sequence start
const sep : &str = ";";  // Escape sequence separator
const end : &str = "m";  // Escape sequence end

// Reset foreground/background color and text effects
const reset : &str = "\033[0m";
const rs : &str = "\033[0m";
const bold : &str = "\033[1m";
const b : &str = "\033[1m";  // Bold on
const unbold : &str = "\033[22m";
const ub : &str = "\033[22m"; // Bold off
const dark : &str = "\033[2m";
const d : &str = "\033[2m"; // Dark on
const undark : &str = "\033[22m";
const ud : &str = "\033[22m"; // Dark off
const italic : &str = "\033[3m";
const i : &str = "\033[3m"; // Italic on
const unitalic : &str = "\033[23m";
const ui : &str = "\033[23m";  // Italic off
const underline : &str = "\033[4m";
const u : &str = "\033[4m"; // Underline on
const ununderline : &str = "\033[24m";
const uu : &str = "\033[24m"; // Underline off
const blink : &str = "\033[5m";
const bl : &str = "\033[5m"; // Blink on
const unblink : &str = "\033[25m";
const ubl : &str = "\033[25m";  // Blink off
const strike : &str = "\033[9m";
const s : &str = "\033[9m";// Strike / crossed-out on
const unstrike : &str = "\033[29m";
const us : &str = "\033[29m"; // Strike / crossed-out off
 
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
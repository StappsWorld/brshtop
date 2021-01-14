use {
    crate::{draw::Draw, mv, theme::Color, BANNER_SRC, term::Term, key::Key,},
    lazy_static::lazy_static,
};
lazy_static! {
    static ref BANNER_META: BannerMeta = BannerMeta::new();
}
struct BannerMeta {
    length: usize,
    c_color: Color,
    out: Vec<String>,
}
impl BannerMeta {
    fn new() -> Self {
        let length = crate::BANNER_SRC
            .iter()
            .max_by(|(_, _, a), (_, _, b)| a.len().cmp(&b.len()))
            .map(|(_, _, l)| l.len())
            .unwrap_or(0);

        let mut c_color = Color::default();
        let mut out = vec![];

        for (line_num, (col1, col2, line)) in BANNER_SRC.iter().enumerate() {
            let mut out_str = String::new();
            let line_color = Color::fg(col1).unwrap();
            let line_color2 = Color::fg(col2).unwrap();
            let line_dark = Color::fg(format!("#{}", 80 - line_num * 6)).unwrap();
            for (char_num, c) in line.chars().enumerate() {
                match c {
                    '█' if c_color != line_color => {
                        c_color = if char_num > 5 && char_num < 25 {
                            line_color2
                        } else {
                            line_color
                        };
                        out_str.push_str(&c_color.to_string())
                    }
                    ' ' => {
                        out_str.push_str(&mv::right(1));
                        c_color = Color::default();
                    }
                    _ if c_color != line_dark => {
                        c_color = line_dark;
                        out_str.push_str(&line_dark.to_string())
                    }
                    _ => out_str.push(c),
                }
            }
            out.push(out_str)
        }
        Self {
            length,
            c_color,
            out,
        }
    }
}

/// Defaults col: int = 0, center: bool = False, now: bool = False
pub fn draw_banner(
    line: u32,    /* TODO: line number type*/
    mut col: u32, /*TODO: Same*/
    center: bool,
    now: bool,
    term : &mut Term,
    draw : &mut Draw,
    key : &mut Key,
) -> String {
    let mut out = String::new();
    if center {
        col = term.width as u32 / 2 - BANNER_META.length as u32 / 2;
    }

    for (n, o) in BANNER_META.out.iter().enumerate() {
        out.push_str(&format!("{}{}", mv::to(line + n as u32, col), o))
    }

    out.push_str(&term.fg.to_string());

    if now {
        draw.out(vec![out], false, key);
    } else {
        return out;
    }

    out
}

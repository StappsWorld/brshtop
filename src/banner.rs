use {
    crate::{draw::Draw, error::errlog, key::Key, mv, term::Term, theme::Color, BANNER_SRC},
    lazy_static::lazy_static,
    std::convert::TryFrom,
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
        // let mut length = crate::BANNER_SRC
        //     .iter()
        //     .max_by(|(_, _, a), (_, _, b)| a.len().cmp(&b.len()))
        //     .map(|(_, _, l)| l.len())
        //     .unwrap_or(0);
        let mut length: usize = 0;

        let mut c_color = Color::default();
        let mut out = vec![];

        for (line_num, (col1, col2, line_unowned)) in BANNER_SRC.iter().enumerate() {
            let line : String = line_unowned.to_owned().to_owned();
            let mut length_of_current_line : usize = 0;
            for _ in line.chars() {
                length_of_current_line += 1;
            }
            if length_of_current_line > length {
                length = length_of_current_line;
            }
            let mut out_str = String::new();
            let line_color = Color::fg(col1).unwrap();
            let line_color2 = Color::fg(col2).unwrap();
            let line_dark = Color::fg(format!("#{}", 80 - line_num * 6)).unwrap();
            for (char_num, c) in line.chars().enumerate() {
                let mut to_push: String = c.into();
                if c == '█' && c_color != line_color {
                    c_color = if 5 < char_num && char_num < 25 {
                        line_color2
                    } else {
                        line_color
                    };
                    out_str.push_str(&c_color.to_string())
                } else if c == ' ' {
                    to_push = mv::right(1);
                    c_color = Color::default();
                } else if c != '█' && c_color != line_dark {
                    c_color = line_dark;
                    out_str.push_str(line_dark.to_string().as_str());
                }
                // match c {
                //     '█' if c_color != line_color => {
                //         c_color = if char_num > 5 && char_num < 25 {
                //             line_color2
                //         } else {
                //             line_color
                //         };
                //         out_str.push_str(&c_color.to_string())
                //     }
                //     ' ' => {
                //         out_str.push_str(&mv::right(1));
                //         c_color = Color::default();
                //     }
                //     _ if c_color != line_dark => {
                //         c_color = line_dark;
                //         out_str.push_str(&line_dark.to_string())
                //     }
                //     _ => (),
                // }
                out_str.push_str(to_push.as_str());
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
    term: &Term,
    draw: &mut Draw,
    key: &mut Key,
) -> String {
    let mut out = String::new();
    if center {
        col = u32::try_from((term.get_width() as i32 / 2) - (BANNER_META.length as i32 / 2))
            .unwrap_or(0);
        errlog(format!(
            "Col is {}, width is {} and banner length is {}",
            col,
            term.get_width(),
            BANNER_META.length
        ));
    }

    for (n, o) in BANNER_META.out.iter().enumerate() {
        out.push_str(&format!("{}{}", mv::to(line + n as u32, col), o).as_str())
    }

    out.push_str(&term.get_fg().to_string().as_str());

    if now {
        draw.out(vec![out.clone()], false, key);
    }

    out
}

use {
    from_map::{FromMap, FromMapDefault},
    lazy_static::lazy_static,
    regex::Regex,
    std::{
        collections::HashMap,
        fs::File,
        io::{self, Read},
        iter::FromIterator,
        path::Path,
    },
};

lazy_static! {
    static ref SIX_DIGIT_HEX: Regex = Regex::new("^#([0-9a-fA-F]{6})$").unwrap();
    static ref TWO_DIGIT_HEX: Regex = Regex::new("^#([0-9a-fA-F]{2})$").unwrap();
    static ref DECIMAL: Regex = Regex::new(r"^(\d{1,3}) (\d{1,3}) (\d{1,3})$").unwrap();
    static ref THEME_SELECTOR: Regex = Regex::new(r#"^theme\[(.+)\] *= *['"](.+)['"]$"#).unwrap();
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum LayerDepth {
    Fg,
    Bg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    depth: LayerDepth,
}
impl Color {
    pub fn escape(&self) -> String {
        format!(
            "\033[{};2;{};{};{}m",
            if self.depth == LayerDepth::Fg { 38 } else { 48 },
            self.r,
            self.g,
            self.b
        )
    }

    pub fn bg<S: ToString>(s: S) -> Result<Self, String> {
        Self::new(s).map(|mut col| {
            col.depth = LayerDepth::Bg;
            col
        })
    }

    pub fn fg<S: ToString>(s: S) -> Result<Self, String> {
        Self::new(s)
    }

    pub fn new<S: ToString>(s: S) -> Result<Self, String> {
        let s = s.to_string();
        let (r, g, b);
        if let Some(captures) = SIX_DIGIT_HEX.captures(&s) {
            let hex = captures.get(1).unwrap().as_str(); // Unwrap is safe, only one possible capture if we got any

            r = u8::from_str_radix(&hex.get(0..2).unwrap(), 16).unwrap(); // These unwraps are unreachable because of the regex
            g = u8::from_str_radix(&hex.get(2..4).unwrap(), 16).unwrap(); // These unwraps are unreachable because of the regex
            b = u8::from_str_radix(&hex.get(4..6).unwrap(), 16).unwrap(); // These unwraps are unreachable because of the regex
        } else if let Some(captures) = TWO_DIGIT_HEX.captures(&s) {
            let hex = captures.get(1).unwrap().as_str(); // Unwrap is safe, only one possible capture if we got any

            let byte = u8::from_str_radix(hex, 16).unwrap(); // Unwrap is safe, regex will not match invalid hex

            r = byte;
            g = byte;
            b = byte;
        } else if let Some(captures) = DECIMAL.captures(&s) {
            let mut parts = captures
                .iter()
                .take(3)
                .map(|capture| capture.unwrap().as_str()); // Unwrap is safe, regex will only match if 3 decimal values exist
            r = u8::from_str_radix(&parts.next().unwrap(), 10).unwrap(); // These unwraps are unreachable because of the regex
            g = u8::from_str_radix(&parts.next().unwrap(), 10).unwrap(); // These unwraps are unreachable because of the regex
            b = u8::from_str_radix(&parts.next().unwrap(), 10).unwrap(); // These unwraps are unreachable because of the regex
        } else {
            return Err(format!("Unable to parse color from {:?}", s));
        }

        Ok(Self {
            r,
            g,
            b,
            depth: LayerDepth::Fg,
        })
    }

    pub fn Default() -> Self {
        Self::new("#cc").unwrap()
    }
    pub fn White() -> Self {
        Color::new("#ff").unwrap()
    }
    pub fn Red() -> Self {
        Color::new("#bf3636").unwrap()
    }
    pub fn Green() -> Self {
        Color::new("#68bf36").unwrap()
    }
    pub fn Blue() -> Self {
        Color::new("#0fd7ff").unwrap()
    }
    pub fn Yellow() -> Self {
        Color::new("#db8b00").unwrap()
    }
    pub fn BlackBg() -> Self {
        Color::bg("#00").unwrap()
    }
    pub fn Null() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            depth: LayerDepth::Fg,
        }
    }
}
impl std::default::Default for Color {
    fn default() -> Self {
        Self::Default()
    }
}
impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.escape())
    }
}
impl std::fmt::UpperHex for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#{:X}{:X}{:X}", self.r, self.g, self.b)
    }
}
impl From<String> for Color {
    // This is unsafe lol
    fn from(s: String) -> Self {
        Self::new(s).unwrap()
    }
}

#[derive(FromMapDefault, FromMap, Debug)]
#[value_type = "Color"]
pub struct Theme {
    pub main_bg: Color,
    #[default("#cc")]
    pub main_fg: Color,
    #[default("#ee")]
    pub title: Color,
    #[default("#969696")]
    pub hi_fg: Color,
    #[default("#7e2626")]
    pub selected_bg: Color,
    #[default("#ee")]
    pub selected_fg: Color,
    #[default("#40")]
    pub inactive_fg: Color,
    #[default("#60")]
    pub proc_misc: Color,
    #[default("#40")]
    pub cpu_box: Color,
    #[default("#0de756")]
    pub mem_box: Color,
    #[default("#3d7b46")]
    pub net_box: Color,
    #[default("#8a882e")]
    pub proc_box: Color,
    #[default("#423ba5")]
    pub div_line: Color,
    #[default("#923535")]
    pub temp_start: Color,
    #[default("#30")]
    pub temp_mid: Color,
    #[default("#4897d4")]
    pub temp_end: Color,
    #[default("#5474e8")]
    pub cpu_start: Color,
    #[default("#ff40b6")]
    pub cpu_mid: Color,
    #[default("#50f095")]
    pub cpu_end: Color,
    #[default("#f2e266")]
    pub free_start: Color,
    #[default("#fa1e1e")]
    pub free_mid: Color,
    #[default("#223014")]
    pub free_end: Color,
    #[default("#b5e685")]
    pub cached_start: Color,
    #[default("#dcff85")]
    pub cached_mid: Color,
    #[default("#0b1a29")]
    pub cached_end: Color,
    #[default("#74e6fc")]
    pub available_start: Color,
    #[default("#26c5ff")]
    pub available_mid: Color,
    #[default("#292107")]
    pub available_end: Color,
    #[default("#ffd77a")]
    pub used_start: Color,
    #[default("#ffb814")]
    pub used_mid: Color,
    #[default("#3b1f1c")]
    pub used_end: Color,
    #[default("#d9626d")]
    pub download_start: Color,
    #[default("#ff4769")]
    pub download_mid: Color,
    #[default("#231a63")]
    pub download_end: Color,
    #[default("#4f43a3")]
    pub upload_start: Color,
    #[default("#b0a9de")]
    pub upload_mid: Color,
    #[default("#510554")]
    pub upload_end: Color,
    #[default("#7d4180")]
    pub graph_text: Color,
    #[default("#dcafde")]
    pub meter_bg: Color,
    #[default("#80d0a3")]
    pub process_start: Color,
    #[default("#dcd179")]
    pub process_mid: Color,
    #[default("#d45454")]
    pub process_end: Color,
}
impl Theme {
    fn from_str<S: ToString>(s: S) -> Result<Self, String> {
        let s = s.to_string();
        let map: HashMap<String, Color> = HashMap::from_iter(
            s.split('\n')
                .filter(|line| !line.starts_with("#") && THEME_SELECTOR.is_match(line))
                .map(|line: &str| -> Result<(String, Color), String> {
                    let captures = match THEME_SELECTOR.captures(line) {
                        Some(caps) => caps,
                        None => unreachable!(),
                    };
                    Ok((
                        captures.get(1).unwrap().as_str().into(),
                        Color::new(captures.get(2).unwrap().as_str())?,
                    ))
                })
                .filter(|result| {
                    if let Err(msg) = result {
                        // errlog(config_dir: &Path, message: String)
                        false
                    } else {
                        true
                    }
                })
                .map(|res| res.unwrap()),
        );
        Ok(Self::from_map_default(map))
    }

    pub fn new<R>(mut reader: R) -> Result<Self, String>
    where
        R: Read,
    {
        let mut buffer = String::new();
        match reader.read_to_string(&mut buffer) {
            Err(e) => {
                return Err(format!(
                    "failed to read from the given reader: {}",
                    e.to_string()
                ))
            }
            _ => {}
        };
        Self::from_str(&buffer)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Result<Self, String>, io::Error> {
        Ok(Self::new(File::open(path)?))
    }
}
/*

class Theme:
    '''__init__ accepts a dict containing { "color_element" : "color" }'''

    themes: Dict[str, str] = {}
    cached: Dict[str, Dict[str, str]] = { "Default" : DEFAULT_THEME }
    current: str = ""

    main_bg = main_fg = title = hi_fg = selected_bg = selected_fg = inactive_fg = proc_misc = cpu_box = mem_box = net_box = proc_box = div_line = temp_start = temp_mid = temp_end = cpu_start = cpu_mid = cpu_end = free_start = free_mid = free_end = cached_start = cached_mid = cached_end = available_start = available_mid = available_end = used_start = used_mid = used_end = download_start = download_mid = download_end = upload_start = upload_mid = upload_end = graph_text = meter_bg = process_start = process_mid = process_end = Colors.default

    gradient: Dict[str, List[str]] = {
        "temp" : [],
        "cpu" : [],
        "free" : [],
        "cached" : [],
        "available" : [],
        "used" : [],
        "download" : [],
        "upload" : [],
        "proc" : [],
        "proc_color" : [],
        "process" : [],
    }
    def __init__(self, theme: str):
        self.refresh()
        self._load_theme(theme)

    def __call__(self, theme: str):
        for k in self.gradient.keys(): self.gradient[k] = []
        self._load_theme(theme)

    def _load_theme(self, theme: str):
        tdict: Dict[str, str]
        if theme in self.cached:
            tdict = self.cached[theme]
        elif theme in self.themes:
            tdict = self._load_file(self.themes[theme])
            self.cached[theme] = tdict
        else:
            errlog.warning(f'No theme named "{theme}" found!')
            theme = "Default"
            CONFIG.color_theme = theme
            tdict = DEFAULT_THEME
        self.current = theme
        #if CONFIG.color_theme != theme: CONFIG.color_theme = theme
        if not "graph_text" in tdict and "inactive_fg" in tdict:
            tdict["graph_text"] = tdict["inactive_fg"]
        if not "meter_bg" in tdict and "inactive_fg" in tdict:
            tdict["meter_bg"] = tdict["inactive_fg"]
        if not "process_start" in tdict and "cpu_start" in tdict:
            tdict["process_start"] = tdict["cpu_start"]
            tdict["process_mid"] = tdict.get("cpu_mid", "")
            tdict["process_end"] = tdict.get("cpu_end", "")


        #* Get key names from DEFAULT_THEME dict to not leave any color unset if missing from theme dict
        for item, value in DEFAULT_THEME.items():
            default = item in ["main_fg", "main_bg"]
            depth = "bg" if item in ["main_bg", "selected_bg"] else "fg"
            if item in tdict:
                setattr(self, item, Color(tdict[item], depth=depth, default=default))
            else:
                setattr(self, item, Color(value, depth=depth, default=default))

        #* Create color gradients from one, two or three colors, 101 values indexed 0-100
        self.proc_start, self.proc_mid, self.proc_end = self.main_fg, Colors.null, self.inactive_fg
        self.proc_color_start, self.proc_color_mid, self.proc_color_end = self.inactive_fg, Colors.null, self.process_start

        rgb: Dict[str, Tuple[int, int, int]]
        colors: List[List[int]] = []
        for name in self.gradient:
            rgb = { "start" : getattr(self, f'{name}_start').dec, "mid" : getattr(self, f'{name}_mid').dec, "end" : getattr(self, f'{name}_end').dec }
            colors = [ list(getattr(self, f'{name}_start')) ]
            if rgb["end"][0] >= 0:
                r = 50 if rgb["mid"][0] >= 0 else 100
                for first, second in ["start", "mid" if r == 50 else "end"], ["mid", "end"]:
                    for i in range(r):
                        colors += [[rgb[first][n] + i * (rgb[second][n] - rgb[first][n]) // r for n in range(3)]]
                    if r == 100:
                        break
                self.gradient[name] += [ Color.fg(*color) for color in colors ]

            else:
                c = Color.fg(*rgb["start"])
                self.gradient[name] += [c] * 101
        #* Set terminal colors
        Term.fg = f'{self.main_fg}'
        Term.bg = f'{self.main_bg}' if CONFIG.theme_background else "\033[49m"
        Draw.now(self.main_fg, self.main_bg)

    @classmethod
    def refresh(cls):
        '''Sets themes dict with names and paths to all found themes'''
        cls.themes = { "Default" : "Default" }
        try:
            for d in (THEME_DIR, USER_THEME_DIR):
                if not d: continue
                for f in os.listdir(d):
                    if f.endswith(".theme"):
                        cls.themes[f'{"" if d == THEME_DIR else "+"}{f[:-6]}'] = f'{d}/{f}'
        except Exception as e:
            errlog.exception(str(e))

    @staticmethod
    def _load_file(path: str) -> Dict[str, str]:
        '''Load a bashtop formatted theme file and return a dict'''
        new_theme: Dict[str, str] = {}
        try:
            with open(path, "r") as f:
                for line in f:
                    if not line.startswith("theme["): continue
                    key = line[6:line.find("]")]
                    s = line.find('"')
                    value = line[s + 1:line.find('"', s + 1)]
                    new_theme[key] = value
        except Exception as e:
            errlog.exception(str(e))

        return new_theme
 */

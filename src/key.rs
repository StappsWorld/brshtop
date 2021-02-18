use {
    crate::{
        draw::Draw,
        error::throw_error,
        event::{Event, EventEnum},
        menu::Menu,
        nonblocking::Nonblocking,
        raw::Raw,
        term::Term,
    },
    nix::sys::{
        select::select,
        time::{TimeVal, TimeValLike},
    },
    std::{
        collections::HashMap,
        io::Read,
        sync::{Arc, Mutex},
        thread,
        time::Duration,
    },
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum KeyUnion {
    String(String),
    Tuple((String, String)),
}

pub struct Key {
    pub list: Vec<String>,
    pub mouse: HashMap<String, Vec<Vec<i32>>>,
    pub mouse_pos: (i32, i32),
    pub escape: HashMap<KeyUnion, String>,
    pub new: Event,
    pub idle: Event,
    pub mouse_move: Event,
    pub mouse_report: bool,
    pub stopping: bool,
    pub started: bool,
}
impl Key {
    pub fn new() -> Self {
        let mut escape_mut: HashMap<KeyUnion, String> = HashMap::<KeyUnion, String>::new();
        escape_mut.insert(KeyUnion::String("\n".to_owned()), "enter".to_owned());
        escape_mut.insert(
            KeyUnion::Tuple(("\x7f".to_owned(), "\x08".to_owned())),
            "backspace".to_owned(),
        );
        escape_mut.insert(
            KeyUnion::Tuple(("[A".to_owned(), "OA".to_owned())),
            "up".to_owned(),
        );
        escape_mut.insert(
            KeyUnion::Tuple(("[B".to_owned(), "OB".to_owned())),
            "down".to_owned(),
        );
        escape_mut.insert(
            KeyUnion::Tuple(("[D".to_owned(), "OD".to_owned())),
            "left".to_owned(),
        );
        escape_mut.insert(
            KeyUnion::Tuple(("[C".to_owned(), "OC".to_owned())),
            "right".to_owned(),
        );
        escape_mut.insert(KeyUnion::String("[2~".to_owned()), "insert".to_owned());
        escape_mut.insert(KeyUnion::String("[3~".to_owned()), "delete".to_owned());
        escape_mut.insert(KeyUnion::String("[H".to_owned()), "home".to_owned());
        escape_mut.insert(KeyUnion::String("[F".to_owned()), "end".to_owned());
        escape_mut.insert(KeyUnion::String("[5~".to_owned()), "page_up".to_owned());
        escape_mut.insert(KeyUnion::String("[6~".to_owned()), "page_down".to_owned());
        escape_mut.insert(KeyUnion::String("\t".to_owned()), "tab".to_owned());
        escape_mut.insert(KeyUnion::String("[Z".to_owned()), "shift_tab".to_owned());
        escape_mut.insert(KeyUnion::String("OP".to_owned()), "f1".to_owned());
        escape_mut.insert(KeyUnion::String("OQ".to_owned()), "f2".to_owned());
        escape_mut.insert(KeyUnion::String("OR".to_owned()), "f3".to_owned());
        escape_mut.insert(KeyUnion::String("OS".to_owned()), "f4".to_owned());
        escape_mut.insert(KeyUnion::String("[15".to_owned()), "f5".to_owned());
        escape_mut.insert(KeyUnion::String("[17".to_owned()), "f6".to_owned());
        escape_mut.insert(KeyUnion::String("[18".to_owned()), "f7".to_owned());
        escape_mut.insert(KeyUnion::String("[19".to_owned()), "f8".to_owned());
        escape_mut.insert(KeyUnion::String("[20".to_owned()), "f9".to_owned());
        escape_mut.insert(KeyUnion::String("[21".to_owned()), "f10".to_owned());
        escape_mut.insert(KeyUnion::String("[23".to_owned()), "f11".to_owned());
        escape_mut.insert(KeyUnion::String("[24".to_owned()), "f12".to_owned());

        Key {
            list: Vec::<String>::new(),
            mouse: HashMap::<String, Vec<Vec<i32>>>::new(),
            mouse_pos: (0, 0),
            escape: escape_mut.clone(),
            new: Event {
                t: EventEnum::Flag(false),
            },
            idle: Event {
                t: EventEnum::Flag(false),
            },
            mouse_move: Event {
                t: EventEnum::Flag(false),
            },
            mouse_report: false,
            stopping: false,
            started: false,
        }
    }

    pub fn start(_self : Arc<Mutex<Key>>, draw: Arc<Mutex<Draw>>, menu: Arc<Mutex<Menu>>) {
        let mut initial_changes = _self.lock().unwrap();
        initial_changes.stopping = false;
        drop(initial_changes);

        let mut self_clone = _self.clone();
        thread::spawn(move || {
            Key::get_key(self_clone, draw, menu);
        });

        let mut after_changes = _self.lock().unwrap();
        after_changes.started = true;
    }

    pub fn stop(&mut self) {
        self.stopping = true;
    }

    pub fn last(&mut self) -> Option<String> {
        if self.list.len() > 0 {
            self.list.pop()
        } else {
            None
        }
    }

    pub fn get(&mut self) -> Option<String> {
        if self.list.len() > 0 {
            let returnable = match self.list.get(0) {
                Some(s) => Some(s.clone()),
                None => None,
            };
            self.list.remove(0);
            returnable
        } else {
            None
        }
    }

    pub fn get_mouse(&mut self) -> (i32, i32) {
        if self.new.is_set() {
            self.new.replace_self(EventEnum::Flag(false));
        }
        self.mouse_pos
    }

    pub fn mouse_moved(&mut self) -> bool {
        if self.mouse_move.is_set() {
            self.mouse_move.replace_self(EventEnum::Flag(false));
            true
        } else {
            false
        }
    }

    pub fn has_key(&mut self) -> bool {
        self.list.len() > 0
    }

    pub fn clear(&mut self) {
        self.list = Vec::<String>::new();
    }

    /// Returns true if key is detected else waits out timer and returns false, defaults sec: float = 0.0, mouse: bool = False
    pub fn input_wait(
        &mut self,
        sec: f64,
        mouse: bool,
        draw: &mut Draw,
        term: &Term,
    ) -> bool {

        if self.list.len() > 0 {
            return true;
        }
        if mouse {
            draw.now(vec![term.get_mouse_direct_on()], self);
        }
        self.new.replace_self(EventEnum::Flag(false));
        self.new.wait(if sec > 0.0 { sec } else { 0.0 });
        self.new.replace_self(EventEnum::Flag(false));
        if mouse {
            draw.now(vec![term.get_mouse_direct_off(), term.get_mouse_on()], self);
        }

        if self.new.is_set() {
            self.new.replace_self(EventEnum::Flag(false));

            true
        } else {
            false
        }
    }

    pub fn break_wait(&mut self) {
        self.list.push("_null".to_owned());
        self.new.replace_self(EventEnum::Flag(false));
        thread::sleep(Duration::from_secs_f64(0.01));
        self.new.replace_self(EventEnum::Flag(false));
    }

    /// Get a key or escape sequence from stdin, convert to readable format and save to keys list. Meant to be run in it's own thread
    pub fn get_key(_self : Arc<Mutex<Key>>, draw_mutex: Arc<Mutex<Draw>>, menu_mutex: Arc<Mutex<Menu>>) {


        let mut input_key: String = String::default();
        let mut clean_key: String = String::default();

        let mut initial_self = _self.lock().unwrap();
        let mut stopping = initial_self.stopping.clone();
        drop(initial_self);

        while !stopping {

            let mut self_key = _self.lock().unwrap();
            let mut draw = draw_mutex.lock().unwrap();
            let mut menu = menu_mutex.lock().unwrap();

            let mut raw = Raw::new();
            raw.enter();

            match select(
                libc::STDIN_FILENO,
                None,
                None,
                None,
                &mut TimeVal::milliseconds(100),
            ) {
                Ok(s) => {
                    if s > 0 {
                        let mut buffer = [0; 1];
                        match raw.stream.read_to_string(&mut input_key) {
                            Ok(_) => {
                                if input_key == String::from("\x1b") {
                                    self_key.idle.replace_self(EventEnum::Flag(false));
                                    draw.idle.replace_self(EventEnum::Wait);
                                    draw.idle.wait(1.0);

                                    let mut nonblocking = Nonblocking::new();
                                    nonblocking.enter();

                                    match raw.stream.read_to_string(&mut input_key) {
                                        Ok(_) => {
                                            if input_key.starts_with("\x1b[<") {
                                                raw.stream.read_to_end(&mut Vec::new());
                                            }
                                        }
                                        Err(_) => (),
                                    }
                                    nonblocking.exit();
                                    self_key.idle.replace_self(EventEnum::Flag(true));
                                }

                                if input_key == String::from("\x1b") {
                                    clean_key = String::from("escape");
                                } else if input_key.starts_with("\x1b[<0;")
                                    || input_key.starts_with("\x1b[<35;")
                                    || input_key.starts_with("\x1b[<64;")
                                    || input_key.starts_with("\x1b[<65;")
                                {
                                    let mut input_vec =
                                        input_key.as_str().split(';').collect::<Vec<&str>>();
                                    self_key.mouse_pos = (
                                        input_vec[1].parse::<i32>().unwrap(),
                                        input_vec[2]
                                            .to_owned()
                                            .trim_end_matches("mM")
                                            .trim_start_matches("mM")
                                            .parse::<i32>()
                                            .unwrap(),
                                    );

                                    if input_key.starts_with("\x1b[<35;") {
                                        self_key.mouse_move.replace_self(EventEnum::Flag(true));
                                        self_key.new.replace_self(EventEnum::Flag(true));
                                    } else if input_key.starts_with("\x1b[<64;") {
                                        clean_key = "mouse_scroll_up".to_owned();
                                    } else if input_key.starts_with("\x1b[<65;") {
                                        clean_key = "mouse_scroll_down".to_owned();
                                    } else if input_key.starts_with("\x1b[<0;")
                                        && input_key.ends_with("m")
                                    {
                                        if menu.active {
                                            clean_key = "mouse_click".to_owned();
                                        } else {
                                            let mut broke: bool = false;
                                            for (key_name, positions) in self_key.mouse.clone() {
                                                let check_inside: Vec<i32> =
                                                    vec![self_key.mouse_pos.0, self_key.mouse_pos.1];
                                                if positions.contains(&check_inside) {
                                                    clean_key = key_name;
                                                    broke = true;
                                                    break;
                                                }
                                            }
                                            if !broke {
                                                clean_key = "mouse_click".to_owned();
                                            }
                                        }
                                    }
                                } else if input_key == String::from("\\") {
                                    clean_key = "\\".to_owned();
                                } else {
                                    let mut broke: bool = false;
                                    for code in self_key.escape.keys() {
                                        if input_key.strip_prefix("\x1b").unwrap().starts_with(
                                            match code {
                                                KeyUnion::String(s) => s.to_owned(),
                                                KeyUnion::Tuple((s1, s2)) => {
                                                    let first = s1.clone();
                                                    let second = s2.clone();
                                                    let together = first + second.as_str();
                                                    together.to_owned()
                                                }
                                            }
                                            .as_str(),
                                        ) {
                                            clean_key =
                                                self_key.escape.get(&code.clone()).unwrap().clone();
                                            broke = true;
                                            break;
                                        }
                                    }
                                    if !broke {
                                        if input_key.len() == 1 {
                                            clean_key = input_key;
                                        }
                                    }
                                }

                                if clean_key != String::default() {
                                    self_key.list.push(clean_key);
                                    if self_key.list.len() > 10 {
                                        self_key.list.remove(0);
                                    }
                                    clean_key = String::default();
                                    self_key.new.replace_self(EventEnum::Flag(true));
                                }
                                input_key = String::default();
                            }
                            Err(_) => {
                                throw_error("Unable to get input from stdin");
                                return;
                            }
                        };
                    }
                }
                Err(_) => self_key.stop(),
            };

            raw.exit();
        }
    }
}

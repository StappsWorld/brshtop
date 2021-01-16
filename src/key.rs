use {
    crate::{
        draw::Draw,
        error::throw_error,
        event::Event,
        menu::Menu,
        nonblocking::Nonblocking,
        raw::Raw,
        term::Term,
    },
    std::{
        collections::{
            HashMap,
        },
        io::{
            self,
            stdin,
            Stdin,
            Read,
        },
        thread,
        time::Duration,
        path::Path,
    },
    nix::sys::{
        select::{select, FdSet},
        time::{
            TimeVal,
            TimeValLike,
        },
    },
};

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum KeyUnion {
    String(String),
    Tuple((String,String)),
}

pub struct Key {
    pub list : Vec<String>,
    pub mouse : HashMap<String, Vec<Vec<i32>>>,
    pub mouse_pos : (i32, i32),
    pub escape : HashMap<KeyUnion, String>,
    pub new : Event,
    pub idle : Event,
    pub mouse_move : Event,
    pub mouse_report : bool,
    pub stopping : bool,
    pub started : bool,
    pub reader : Option<thread::JoinHandle<()>>,
}
impl Key {

    pub fn new() -> Self {
        let escape_mut : HashMap<KeyUnion, String> = HashMap::<KeyUnion, String>::new();
            escape_mut.insert(KeyUnion::String("\n".to_owned()),"enter".to_owned());
            escape_mut.insert(KeyUnion::Tuple(("\x7f".to_owned(), "\x08".to_owned())),"backspace".to_owned());
            escape_mut.insert(KeyUnion::Tuple(("[A".to_owned(), "OA".to_owned())),"up".to_owned());
            escape_mut.insert(KeyUnion::Tuple(("[B".to_owned(), "OB".to_owned())),"down".to_owned());
            escape_mut.insert(KeyUnion::Tuple(("[D".to_owned(), "OD".to_owned())),"left".to_owned());
            escape_mut.insert(KeyUnion::Tuple(("[C".to_owned(), "OC".to_owned())),"right".to_owned());
            escape_mut.insert(KeyUnion::String("[2~".to_owned()),"insert".to_owned());
            escape_mut.insert(KeyUnion::String("[3~".to_owned()),"delete".to_owned());
            escape_mut.insert(KeyUnion::String("[H".to_owned()),"home".to_owned());
            escape_mut.insert(KeyUnion::String("[F".to_owned()),"end".to_owned());
            escape_mut.insert(KeyUnion::String("[5~".to_owned()),"page_up".to_owned());
            escape_mut.insert(KeyUnion::String("[6~".to_owned()),"page_down".to_owned());
            escape_mut.insert(KeyUnion::String("\t".to_owned()),"tab".to_owned());
            escape_mut.insert(KeyUnion::String("[Z".to_owned()),"shift_tab".to_owned());
            escape_mut.insert(KeyUnion::String("OP".to_owned()),"f1".to_owned());
            escape_mut.insert(KeyUnion::String("OQ".to_owned()),"f2".to_owned());
            escape_mut.insert(KeyUnion::String("OR".to_owned()),"f3".to_owned());
            escape_mut.insert(KeyUnion::String("OS".to_owned()),"f4".to_owned());
            escape_mut.insert(KeyUnion::String("[15".to_owned()),"f5".to_owned());
            escape_mut.insert(KeyUnion::String("[17".to_owned()),"f6".to_owned());
            escape_mut.insert(KeyUnion::String("[18".to_owned()),"f7".to_owned());
            escape_mut.insert(KeyUnion::String("[19".to_owned()),"f8".to_owned());
            escape_mut.insert(KeyUnion::String("[20".to_owned()),"f9".to_owned());
            escape_mut.insert(KeyUnion::String("[21".to_owned()),"f10".to_owned());
            escape_mut.insert(KeyUnion::String("[23".to_owned()),"f11".to_owned());
            escape_mut.insert(KeyUnion::String("[24".to_owned()),"f12".to_owned());
        

        Key {
            list : Vec::<String>::new(),
            mouse : HashMap::<String, Vec<Vec<i32>>>::new(),
            mouse_pos : (0,0),
            escape : escape_mut.clone(),
            new : Event::Flag(false),
            idle : Event::Flag(false),
            mouse_move : Event::Flag(false),
            mouse_report : false,
            stopping : false,
            started : false,
            reader : None,
        }
    }

    pub fn start(&'static mut self, draw : &'static mut Draw, menu : &'static mut Menu) {
        self.stopping = false;
        self.reader = Some(thread::spawn(|| self.get_key(draw, menu)));
        self.started = true;
    }

    pub fn stop(&mut self) -> Option<bool> {
        if self.started && match self.reader.unwrap().join() {
            Ok(_) => true,
            Err(_) => return None,
         } {
            self.stopping = true;
            return Some(true);
         }
         None
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
            let returnable = match self.list.get(0){
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
            self.new = Event::Flag(false);
        }
        self.mouse_pos
    }

    pub fn mouse_moved(&mut self) -> bool {
        if self.mouse_move.is_set() {
            self.mouse_move = Event::Flag(false);
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
    pub fn input_wait(&mut self, sec : f64, mouse : bool, draw : &mut Draw, term : &mut Term) -> bool {
        if self.list.len() > 0 {
            return true;
        }
        if mouse {
            draw.now(vec![term.mouse_direct_on], self);
        }
        self.new = Event::Wait;
        self.new.wait(if sec > 0.0 {sec} else {0.0});
        self.new = Event::Flag(false);
        if mouse {
            draw.now(vec![term.mouse_direct_off, term.mouse_on], self);
        }

        if self.new.is_set() {
            self.new = Event::Flag(false);

            true
        } else {
            false
        }
    }

    pub fn break_wait(&mut self) {
        self.list.push("_null".to_owned());
        self.new = Event::Flag(true);
        thread::sleep(Duration::from_secs_f64(0.01));
        self.new = Event::Flag(false);
    }

    /// Get a key or escape sequence from stdin, convert to readable format and save to keys list. Meant to be run in it's own thread
    pub fn get_key(&mut self, draw : &mut Draw, menu : &mut Menu) {
        let mut input_key : String = String::default();
        let mut clean_key : String = String::default();

        while !self.stopping {
            let mut current_stdin : Stdin = stdin();
            let mut raw = Raw::new(&mut current_stdin);
            raw.enter();

            
            match select(libc::STDIN_FILENO, None, None, None, &mut TimeVal::milliseconds(100)) {
                Ok(s) => if s > 0 {
                    let mut buffer = [0; 1];
                    match current_stdin.read_to_string(&mut input_key) {
                        Ok(_) => {
                            if input_key == String::from("\033") {
                                self.idle = Event::Flag(false);
                                draw.idle = Event::Wait;
                                draw.idle.wait(-1.0);

                                let mut nonblocking = Nonblocking::new(&mut current_stdin);
                                nonblocking.enter();

                                match current_stdin.read_to_string(&mut input_key) {
                                    Ok(_) => {
                                        if input_key.starts_with("\033[<"){
                                            current_stdin.read_to_end(&mut Vec::new());
                                        }
                                    },
                                    Err(_) => ()
                                }
                                nonblocking.exit();
                                self.idle = Event::Flag(true);
                            }

                            if input_key == String::from("\033") {
                                clean_key = String::from("escape");
                            } else if input_key.starts_with("\033[<0;") || 
                                input_key.starts_with("\033[<35;") ||
                                input_key.starts_with("\033[<64;") ||
                                input_key.starts_with("\033[<65;")
                            {
                                let mut input_vec = input_key.as_str().split(';').collect::<Vec<&str>>();
                                self.mouse_pos = (input_vec[1].parse::<i32>().unwrap(), 
                                    input_vec[2]
                                        .to_owned()
                                        .trim_end_matches("mM")
                                        .trim_start_matches("mM")
                                        .parse::<i32>()
                                        .unwrap());
                                
                                if input_key.starts_with("\033[<35;") {
                                    self.mouse_move = Event::Flag(true);
                                    self.new = Event::Flag(true);
                                } else if input_key.starts_with("\033[<64;") {
                                    clean_key = "mouse_scroll_up".to_owned();
                                } else if input_key.starts_with("\033[<65;") {
                                    clean_key = "mouse_scroll_down".to_owned();   
                                } else if input_key.starts_with("\033[<0;") && input_key.ends_with("m") {
                                    if menu.active {
                                        clean_key = "mouse_click".to_owned();
                                    } else {
                                        let mut broke : bool = false;
                                        for (key_name, positions) in self.mouse {
                                            let check_inside : Vec<i32> = vec![self.mouse_pos.0, self.mouse_pos.1];
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
                            } else if input_key == String::from("\\"){
                                clean_key = "\\".to_owned();
                            } else {
                                let mut broke : bool = false;
                                for code in self.escape.keys() {
                                    if input_key.strip_prefix("\033").unwrap().starts_with(match code {
                                        KeyUnion::String(s) => s,
                                        KeyUnion::Tuple((s1, s2)) => &(s1.clone() + s2.as_str()),
                                    }) {
                                        clean_key = self.escape[code];
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
                                self.list.push(clean_key);
                                if self.list.len() > 10 {
                                    self.list.remove(0);
                                }
                                clean_key = String::default();
                                self.new = Event::Flag(true);
                            }
                            input_key = String::default();
                        },
                        Err(_) => {
                            throw_error("Unable to get input from stdin");
                            return;
                        },
                    };

                },
                Err(_) => match self.stop() {
                    Some(s) => (),
                    None => throw_error("Unable to get input from stdin"),
                },
            };


            raw.exit();
        }

    }

}
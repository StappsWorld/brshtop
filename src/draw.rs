use {
    crate::{
        event::{Event, EventEnum},
        key::Key,
    },
    once_cell::sync::OnceCell,
    std::{
        collections::HashMap,
        io::{self, Write},
        sync::Mutex,
    },
};

pub struct Draw {
    pub strings: HashMap<String, String>,
    pub z_order: HashMap<String, i32>,
    pub saved: HashMap<String, String>,
    pub save: HashMap<String, bool>,
    pub once: HashMap<String, bool>,
    pub idle: Event,
}
impl Draw {
    pub fn new() -> Self {
        Draw {
            strings: HashMap::<String, String>::new(),
            z_order: HashMap::<String, i32>::new(),
            saved: HashMap::<String, String>::new(),
            save: HashMap::<String, bool>::new(),
            once: HashMap::<String, bool>::new(),
            idle: Event {
                t: EventEnum::Flag(true),
            },
        }
    }

    /// Wait for input reader and self to be idle then print to screen
    pub fn now(&mut self, args: Vec<String>, key_p: &OnceCell<Mutex<Key>>) {
        
        let mut key = key_p.get().unwrap().lock().unwrap();

        key.idle.replace_self(EventEnum::Wait);
        //key.get().unwrap().lock().unwrap().idle.wait(-1.0);
        drop(key);


        self.idle.replace_self(EventEnum::Wait);

        //self.idle.wait(-1.0);

        self.idle.replace_self(EventEnum::Flag(false));


        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        for s in args.clone() {
            print!("{}", s);
            io::stdout().flush().unwrap();
        }

        self.idle.replace_self(EventEnum::Flag(true));
    }

    /// Defaults append: bool = False, now: bool = False, z: int = 100, only_save: bool = False, no_save: bool = False, once: bool = False
    pub fn buffer(
        &mut self,
        name: String,
        args: Vec<String>,
        append: bool,
        now: bool,
        z: i32,
        only_save: bool,
        no_save: bool,
        once: bool,
        key: &OnceCell<Mutex<Key>>,
    ) {
        let mut string: String = String::default();
        let mut append_mut: bool = append.clone();
        let mut mutable_name = name.clone();
        if mutable_name.starts_with("+") {
            mutable_name = mutable_name.strip_prefix("+").unwrap().to_owned();
            append_mut = true;
        }
        let mut now_mut: bool = now.clone();
        if mutable_name.ends_with("!") {
            mutable_name = mutable_name.strip_suffix("!").unwrap().to_owned();
            now_mut = true;
        }
        self.save.insert(mutable_name.clone(), !no_save);
        self.once.insert(mutable_name.clone(), once);

        if !self.z_order.contains_key(&mutable_name.clone().clone()) || z != 100 {
            self.z_order.insert(mutable_name.clone(), z);
        }
        if args.len() > 0 {
            args.iter().map(|s| string.push_str(s));
        }
        if only_save {
            if !self.saved.contains_key(&mutable_name.clone()) || !append_mut {
                self.saved.insert(mutable_name.clone(), String::default());
            }
            self.saved
                .get_mut(&mutable_name.clone())
                .unwrap()
                .push_str(string.as_str());
        } else {
            if !self.strings.contains_key(&mutable_name.clone()) || !append_mut {
                self.strings.insert(mutable_name.clone(), String::default());
            }
            self.strings
                .get_mut(&mutable_name.clone())
                .unwrap()
                .push_str(string.as_str());
            if now_mut {
                self.out(vec![mutable_name.clone()], false, key);
            }
        }
    }

    /// Defaults clear = false
    pub fn out(&mut self, names: Vec<String>, clear: bool, key: &OnceCell<Mutex<Key>>) {
        let mut out: String = String::default();
        if self.strings.len() == 0 {
            return;
        }
        if names.len() > 0 {
            let z_order_save = self.z_order.clone();
            let mut z_order_sort: Vec<(&String, &i32)> = z_order_save.iter().collect();
            z_order_sort.sort_by(|a, b| b.1.cmp(a.1));
            for (name, value) in z_order_sort {
                if names.contains(name) && self.strings.contains_key(name) {
                    out.push_str(self.strings[name].as_str());
                    if self.save[name] {
                        self.saved.insert(
                            name.clone(),
                            self.strings.get(&name.clone()).unwrap().clone(),
                        );
                    }
                    if clear || self.once[name] {
                        self.clear(vec![name.clone()], false);
                    }
                }
            }

            if clear {
                self.clear(vec![], false);
            }
            self.now(vec![out], key);
        }
    }

    pub fn saved_buffer(&mut self) -> String {
        let mut out: String = String::default();

        let mut z_order_sort: Vec<(&String, &i32)> = self.z_order.iter().collect();
        z_order_sort.sort_by(|a, b| b.1.cmp(a.1));
        for (name, value) in z_order_sort {
            if self.saved.contains_key(name) {
                out.push_str(self.saved[name].as_str());
            }
        }
        out
    }

    /// Defaults saved = false
    pub fn clear(&mut self, names: Vec<String>, saved: bool) {
        if names.len() > 0 {
            for name in names {
                if self.strings.contains_key(&name) {
                    self.strings.remove(&name);
                }
                if self.save.contains_key(&name) {
                    self.save.remove(&name);
                }
                if self.once.contains_key(&name) {
                    self.once.remove(&name);
                }
                if saved {
                    if self.saved.contains_key(&name) {
                        self.saved.remove(&name);
                    }
                    if self.z_order.contains_key(&name) {
                        self.z_order.remove(&name);
                    }
                }
            }
        } else {
            self.strings = HashMap::<String, String>::new();
            self.save = HashMap::<String, bool>::new();
            self.once = HashMap::<String, bool>::new();
            if saved {
                self.saved = HashMap::<String, String>::new();
                self.z_order = HashMap::<String, i32>::new();
            }
        }
    }
}

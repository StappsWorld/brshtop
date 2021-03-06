use crate::theme::Theme;
use std::{collections::HashMap, fs::File};
pub struct Brshtop {
    theme_cache: HashMap<String, Theme>,
}
impl Brshtop {
    pub fn new() -> Self {
        Self {
            theme_cache: HashMap::new(),
        }
    }
    pub fn _init(&mut self) -> Vec<crate::error::Error> {
        self._load_themes()
    }

    fn _load_themes(&mut self) -> Vec<crate::error::Error> {
        let mut errors = vec![];
        let mut files = vec![];
        for path in crate::THEME_DIRS.to_owned().iter() {
            match std::fs::metadata(path) {
                Err(e) => errors.push(e.into()),
                Ok(meta) if meta.is_dir() => {
                    let reader = match std::fs::read_dir(path) {
                        Ok(reader) => reader,
                        Err(e) => {
                            errors.push(e.into());
                            continue;
                        }
                    };
                    for entry in reader.filter_map(Result::ok) {
                        match File::open(entry.path()) {
                            Ok(file) => files.push((entry.path(), file)),
                            Err(e) => errors.push(e.into()),
                        }
                    }
                }
                Ok(meta) if meta.is_file() => match File::open(path) {
                    Ok(file) => files.push((path.into(), file)),
                    Err(e) => errors.push(e.into()),
                },
                _ => {
                    unreachable!()
                }
            }
        }

        for (path, file) in files {
            match Theme::new(file) {
                Ok(theme) => {
                    self.theme_cache
                        .insert(path.to_str().unwrap().to_string(), theme);
                }
                Err(e) => errors.push(crate::error::Error::Theme(e)),
            }
        }

        //println!("{:#?}\n\n{:#?}", self.theme_cache, errors);
        errors
    }
}

use crate::{mv, symbol, theme::Color, term::Term};
use maplit::hashmap;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use math::round::ceil;

pub struct Graphs {
    pub cpu: HashMap<String, Graph>,
    pub cores: Vec<Graph>,
    pub temps: Vec<Graph>,
    pub net: HashMap<String, Graph>,
    pub detailed_cpu: Graph,
    pub detailed_mem: Graph,
    pub pid_cpu: HashMap<u32, Graph>, // TODO: PID type
}

// FIXME
// REMINDER: i store Colors, py stores Strings, convert them to escapes when used :)

#[derive(Debug)]
pub struct Graph {
    out: String,
    width: u32,
    height: u32,
    graphs: HashMap<bool, Vec<String>>,
    colors: Vec<Color>,
    invert: bool,
    max_value: i32,
    color_max_value: i32,
    offset: i32,
    current: bool,
    last: i32,
    symbol: HashMap<u32, &'static str>,
    _data: Vec<i32>, // TODO: Data type
}
impl Graph {

    /// Defaults invert: bool = False, max_value: int = 0, offset: int = 0, color_max_value: Union[int, None] = None
    pub fn new<C>(
        width: i32,
        height: i32,
        color: Option<C>,
        data: Vec<i32>,
        term : &mut Term,
        invert : bool,
        max_value : i32,
        offset : i32,
        color_max_value : Option<i32>,
    ) -> Self
    where
        C: Into<Color>,
    {
        let graphs = hashmap! {
            true => Vec::new(),
            false => Vec::new(),
        };

        let mut real_data = data.clone();
        if data.len() == 0 {
            real_data = vec![0];
        }

        let mut color_scale : i32 = 100;
        if max_value != 0 {
            let mut to_set : Vec<i32> = Vec::<i32>::new();

            for v in real_data {
                to_set.push(if (v + offset) * (100 / (max_value + offset)) as i32 > 100 {100} else {(v + offset) * (100 / (max_value + offset)) as i32});
            }

            real_data = to_set;

            if color_max_value != None {
                color_scale = 100 * (max_value / match color_max_value {
                    Some(val) => val,
                    None => max_value,
                } as i32);
            }
        }

        

        let colors = if let Some(color) = color.map(<_ as Into<Color>>::into) {
            if height > 1 {
                (0..height).map(|_| color).collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        

        let mut graph = Self {
            out: String::new(),
            width : width as u32,
            height : height as u32,
            invert: false,
            offset: offset,
            colors,
            symbol: if height == 1 {
                if invert {
                    symbol::graph_down_small()
                } else {
                    symbol::graph_up_small()
                }
            } else {
                if invert {
                    symbol::graph_down()
                } else {
                    symbol::graph_up()
                }
            },
            max_value: max_value,
            color_max_value: match color_max_value {
                Some(c) => c,
                None => max_value,
            },
            _data: real_data,
            graphs,
            current: false,
            last: 0,
        };

        graph._refresh_data(term);

        let mut value_width : i32 = ceil(data.len() as f64 / 2.0, 0) as i32;
        let mut filler : String = String::default();

        if value_width > width {
            real_data = data[(width as usize * 2)..].to_vec();
        } else if value_width < width {
            filler = graph.symbol[&(0 as u32)].repeat((width - value_width) as usize);
        }

        if real_data.len() % 2 != 0 {
            real_data.insert(0,0);
        }

        for _ in 0..height {
            for b in vec![true, false] {
                graph.graphs[&b].push(filler);
            }
        }

        graph._create(true, term);

        graph
    }

    /// Defaults invert: bool = False, max_value: int = 0, offset: int = 0, color_max_value: Union[int, None] = None
    pub fn new_with_vec<C>(
        width: u32,
        height: u32,
        color: Vec<String>,
        data: Vec<i32>, // TODO: Data type
        term : &mut Term,
        invert : bool,
        max_value : i32,
        offset : i32,
        color_max_value : Option<i32>,
    ) -> Self
    {

        let graphs = hashmap! {
            true => Vec::new(),
            false => Vec::new(),
        };

        let mut real_data = data.clone();
        if data.len() == 0 {
            real_data = vec![0];
        }

        let mut color_scale : u32 = 100;
        if max_value != 0 {
            let mut to_set : Vec<i32> = Vec::<i32>::new();

            for v in real_data {
                to_set.push(if (v + offset) * (100 / (max_value + offset)) as i32 > 100 {100} else {(v + offset) * (100 / (max_value + offset)) as i32});
            }

            real_data = to_set;

            if color_max_value != None {
                color_scale = 100 * (max_value / match color_max_value {
                    Some(val) => val,
                    None => max_value,
                }) as u32;
            }
        }

        

        let colors : Vec<Color> = Vec::<Color>::new();
        if height > 1 {
            for i in 1..height + 1 {
                colors.insert(0, Color::new(color.get(if i * (color_scale / height) < 100 {(i * (color_scale / height)) as usize} else {100 as usize}).unwrap()).unwrap());

            }
        }

        

        let mut graph = Self {
            out: String::new(),
            width,
            height,
            invert: false,
            offset: offset,
            colors,
            symbol: if height == 1 {
                if invert {
                    symbol::graph_down_small()
                } else {
                    symbol::graph_up_small()
                }
            } else {
                if invert {
                    symbol::graph_down()
                } else {
                    symbol::graph_up()
                }
            },
            max_value: max_value,
            color_max_value: match color_max_value {
                Some(c) => c,
                None => max_value,
            },
            _data: real_data,
            graphs,
            current: false,
            last: 0,
        };

        graph._refresh_data(term);


        graph
    }

    pub fn invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self.symbol = match self.height {
            1 if invert => symbol::graph_down_small(),
            1 => symbol::graph_up_small(),
            _ if invert => symbol::graph_down(),
            _ => symbol::graph_up(),
        };
        self
    }
    pub fn max_value(mut self, max_value: i32, term : &mut Term) -> Self {
        self.max_value = max_value;
        self._refresh_data(term);
        self
    }
    pub fn offset(mut self, offset: i32) -> Self {
        self.offset = offset;
        self
    }
    pub fn color_max_value(mut self, color_max_value: i32) -> Self {
        self.color_max_value = color_max_value;
        self
    }

    fn _refresh_data(&mut self, term : &mut Term) {
        let value_width = (self._data.len() as f32 / 2.).ceil() as i32;

        self._data = if self._data.is_empty() {
            vec![]
        } else {
            self._data
                .iter()
                .map(|v| (v + self.offset) * (100 / (self.max_value + self.offset)))
                .skip(if value_width < self.width as i32 {
                    self._data.len() - self.width as usize * 2
                } else {
                    0
                })
                .collect()
        };

        let filler: String = if value_width < self.width as i32 {
            (0..self.width - value_width as u32)
                .map(|_| self.symbol[&0].to_string())
                .collect()
        } else {
            "".into()
        };

        for _ in 0..self.height {
            // TODO, try to remove clones, at least remove the to_string above :)
            self.graphs.get_mut(&true).unwrap().push(filler.clone());
            self.graphs.get_mut(&false).unwrap().push(filler.clone());
        }

        self._create(true, term);
    }

    fn _create(&mut self, new: bool, term : &mut Term) {
        let mut value = hashmap! {
            "left" => 0,
            "right" => 0,
        };
        for h in 0..self.height {
            let h_high = if self.height > 1 {
                (100. * (self.height - h) as f32 / self.height as f32).round() as i32
            } else {
                100
            };

            let h_low = if self.height > 1 {
                (100. * (self.height - (h + 1)) as f32 / self.height as f32).round() as i32
            } else {
                0
            };

            for (v, item) in self._data.iter().enumerate() {
                if new {
                    self.current = v % 2 == 0;
                    if v == 0 {
                        self.last = 0
                    }
                }

                for (val, side) in [self.last, *item].iter().zip(["left", "right"].iter()) {
                    value.insert(
                        side,
                        if val >= &h_high {
                            4
                        } else if val <= &h_low {
                            0
                        } else {
                            if self.height == 1 {
                                ((val * 4) as f32 * 100.5).round() as usize
                            } else {
                                (((val - h_low) * 4) as f32 / (h_high - h_low) as f32 + 0.1).round()
                                    as usize
                            }
                        },
                    );
                }

                if new {
                    self.last = *item
                }

                // Unwrap is safe, self.current will only ever be true or false, self.graphs is preloaded with true/false values
                let graph = self.graphs.get_mut(&self.current).unwrap();
                if h < graph.len() as u32 {
                    // TODO: Determine if this unwrap is safe (value[left/right] can only be 0-4)
                    graph[h as usize].push_str(
                        self.symbol
                            .get(&((value["left"] * 10 + value["right"]) as u32))
                            .unwrap(),
                    );
                } else {
                    // TODO: What do here lol
                }
            }
        }

        if !self._data.is_empty() {
            // unwrap is safe
            self.last = *self._data.last().unwrap();
        }

        self.out = String::new();

        match self.height {
            1 => self.out.push_str(&format!(
                "{}{}",
                if self.colors.is_empty() {
                    "".into()
                } else {
                    self.colors
                        .get(self.last as usize)
                        .map(Color::to_string)
                        .unwrap_or_default()
                },
                self.graphs
                    .get(&self.current)
                    .map(|graph| graph.get(0))
                    .flatten()
                    .cloned()
                    .unwrap_or_default()
            )),
            _ => {
                for h in 0..self.height {
                    if h > 0 {
                        self.out.push_str(&format!(
                            "{}{}",
                            mv::down(1),
                            mv::left(self.width as u32)
                        ))
                    }

                    self.out.push_str(&format!(
                        "{}{}",
                        if self.colors.is_empty() {
                            "".into()
                        } else {
                            self.colors.get(h as usize).map(Color::to_string).unwrap_or_default()
                        },
                        self.graphs
                            .get(&self.current)
                            .map(|graph| {
                                graph.get(if self.invert { self.height - 1 - h } else { h } as usize)
                            })
                            .flatten()
                            .cloned()
                            .unwrap_or_default()
                    ))
                }
            }
        }

        if !self.colors.is_empty() {
            self.out.push_str(&term.fg.to_string())
        }
    }

    fn _call(&mut self, value: Option<i32>, term : &mut Term) -> String {
        if let Some(value) = value {
            self.current = !self.current;

            // TODO: This is disgusting
            if self.height == 1 {
                if let Some(true) = self
                    .graphs
                    .get(&self.current)
                    .map(|graph| graph.first())
                    .flatten()
                    .map(|s| s.starts_with(self.symbol.get(&0).unwrap()))
                {
                    let graph = self
                        .graphs
                        .get_mut(&self.current)
                        .expect("Graph not available");
                    graph[0] = graph[0].replacen(self.symbol.get(&0).unwrap(), "", 1);
                } else {
                    let graph = self
                        .graphs
                        .get_mut(&self.current)
                        .expect("Graph not available");
                    graph[0] = graph[0].chars().skip(1).collect();
                }
            } else {
                for n in 0..self.max_value {
                    let graph = self
                        .graphs
                        .get_mut(&self.current)
                        .expect("Graph not available");
                    graph[n as usize] = graph[n as usize].chars().skip(1).collect();
                }
            }

            if self.max_value != 0 {
                self._data = vec![if value < self.max_value {
                    ((value + self.offset) * 100) / (self.max_value + self.offset)
                } else {
                    100
                } as i32];
                self._refresh_data(term);
            }

            self._create(false, term);
        }

        self.out.clone()
    }

    pub fn add(&mut self, value: Option<i32>, term : &mut Term) -> String {
        self._call(value, term)
    }
}
impl Display for Graph {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.out)
    }
}

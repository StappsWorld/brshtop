use crate::{mv, symbol, theme::Color, term::Term};
use maplit::hashmap;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

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
    width: usize,
    height: usize,
    graphs: HashMap<bool, Vec<String>>,
    colors: Vec<Color>,
    invert: bool,
    max_value: usize,
    color_max_value: usize,
    offset: usize,
    current: bool,
    last: usize,
    symbol: HashMap<u32, &'static str>,
    _data: Vec<usize>, // TODO: Data type
}
impl Graph {

    /// Defaults invert: bool = False, max_value: int = 0, offset: int = 0, color_max_value: Union[int, None] = None
    pub fn new<C>(
        width: usize,
        height: usize,
        color: Option<C>,
        data: Vec<usize>, // TODO: Data type
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
            width,
            height,
            invert: false,
            offset: 0,
            color_max_value: 0,
            colors,
            symbol: if height == 1 {
                symbol::graph_up_small()
            } else {
                symbol::graph_up()
            },
            max_value: 0,
            _data: data,
            graphs,
            current: false,
            last: 0,
        };

        graph._refresh_data(term);

        graph
    }

    /// Defaults invert: bool = False, max_value: int = 0, offset: int = 0, color_max_value: Union[int, None] = None
    pub fn new_with_vec<C>(
        width: usize,
        height: usize,
        color: Vec<String>,
        data: Vec<usize>, // TODO: Data type
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

        let colors : Vec<Color> = Vec::<Color>::new();
        if height > 1 {
            for i in 1..height + 1 {
                colors.insert(0, Color::new(color.get(if i * (color_scale / height) as i32 < 100 {i * (color_scale / height) as i32} else {100}).unwrap()).unwrap());

            }
        }

        let mut graph = Self {
            out: String::new(),
            width,
            height,
            invert: false,
            offset: 0,
            color_max_value: 0,
            colors,
            symbol: if height == 1 {
                symbol::graph_up_small()
            } else {
                symbol::graph_up()
            },
            max_value: 0,
            _data: data,
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
    pub fn max_value(mut self, max_value: usize, term : &mut Term) -> Self {
        self.max_value = max_value;
        self._refresh_data(term);
        self
    }
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }
    pub fn color_max_value(mut self, color_max_value: usize) -> Self {
        self.color_max_value = color_max_value;
        self
    }

    fn _refresh_data(&mut self, term : &mut Term) {
        let value_width = (self._data.len() as f32 / 2.).ceil() as usize;

        self._data = if self._data.is_empty() {
            vec![]
        } else {
            self._data
                .iter()
                .map(|v| (v + self.offset) * (100 / (self.max_value + self.offset)))
                .skip(if value_width < self.width {
                    self._data.len() - self.width as usize * 2
                } else {
                    0
                })
                .collect()
        };

        let filler: String = if value_width < self.width {
            (0..self.width - value_width)
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
                (100. * (self.height - h) as f32 / self.height as f32).round() as usize
            } else {
                100
            };

            let h_low = if self.height > 1 {
                (100. * (self.height - (h + 1)) as f32 / self.height as f32).round() as usize
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
                if h < graph.len() {
                    // TODO: Determine if this unwrap is safe (value[left/right] can only be 0-4)
                    graph[h].push_str(
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
                        .get(self.last)
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
                            self.colors.get(h).map(Color::to_string).unwrap_or_default()
                        },
                        self.graphs
                            .get(&self.current)
                            .map(|graph| {
                                graph.get(if self.invert { self.height - 1 - h } else { h })
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

    fn _call(&mut self, value: Option<usize>, term : &mut Term) -> String {
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
                    graph[n] = graph[n].chars().skip(1).collect();
                }
            }

            if self.max_value != 0 {
                self._data = vec![if value < self.max_value {
                    ((value + self.offset) * 100) / (self.max_value + self.offset)
                } else {
                    100
                }];
                self._refresh_data(term);
            }

            self._create(false, term);
        }

        self.out.clone()
    }

    pub fn add(&mut self, value: Option<usize>, term : &mut Term) -> String {
        self._call(value, term)
    }
}
impl Display for Graph {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.out)
    }
}

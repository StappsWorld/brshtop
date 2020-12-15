use crate::Collector::*;
use which::which;
use psutil::sensors::*;



pub struct CpuCollector {
    pub stopping: bool,
    pub started: bool,
    pub draw_now: bool,
    pub redraw: bool,
    pub only_draw: bool,
    pub tx: Sender<Event>,
    pub rx: Reciever<Event>,
    pub thread: Option<thread::JoinHandle<()>>,
    pub flag: Flag,
    pub control: Control,
    pub collect_run : Event,
    pub collect_idle: Event,
    pub collect_done: Event,
    pub collect_queue: Vec,
    pub collect_interrupt: bool,
    pub proc_interrupt: bool,
    pub use_draw_list: bool,
    pub cpu_usage: Vec<Vec<u32>>,
    pub cpu_temp: Vec<Vec<u32>>,
    pub cpu_temp_high: i32,
    pub cpu_temp_crit: i32,
    pub freq_error: bool,
    pub cpu_freq: i32,
    pub load_avg: Vec<f32>,
    pub uptime: String,
    pub buffer: String,
    pub sensor_method: String,
    pub got_sensors: bool,
    pub sensor_swap: bool,
    pub cpu_temp_only: bool,
} impl CpuCollector for CollTrait {

    fn init(THREADS : u64) {

        let mut cpu_usage_mut = Vec::<Vec<u32>>::new();
        let mut cpu_temp_mut = Vec::<Vec<u32>>::new();
        for _ in 0..THREADS + 1 {
            cpu_usage_mut.push(Vec::new());
            cpu_temp_mut.push(Vec::new());
        }

        let mut CpuCollector_initialize = CpuCollector {
            stopping = false,
            started = false,
            draw_now = false,
            redraw = false,
            only_draw = false,
            tx = tx_build,
            rx = rx_build,
            flag = flag_build,
            control = control_build,
            thread = None,
            collect_run = Event::Flag(false),
            collect_done = Event::Flag(false),
            collect_idle = Event::Flag(true),
            collect_done = Event::Flag(false),
            collect_queue = Vec::<CollTrait>::new(),
            collect_interrupt = false,
            proc_interrupt = false,
            use_draw_list = false,
            cpu_usage: cpu_usage_mut,
            cpu_temp: cpu_temp_mut,
            cpu_temp_high: 0,
            cpu_temp_crit: 0,
            freq_error: false,
            cpu_freq: 0,
            load_avg: Vec::<f32>::new(),
            uptime: String.from(""),
            buffer: String.from(""),
            sensor_method: String.from(""),
            got_sensors: false,
            sensor_swap: false,
            cpu_temp_only: false,
        };
    }

    fn get_sensors(&mut self, CONFIG : Config, SYSTEM : String) {
        self.sensor_method = String::from("");

        if SYSTEM == "MacOS" {
            match which("coretemp") {
                Ok() => {
                    let output = Exec::shell("coretemp -p").capture()?.stdout_str().to_owned();
                    match output.trim().replace("-", "").parse::<f64>() {
                        Some(n) => self.sensor_method = "coretemp",
                        None => match which("osx-cpu-temp") {
                            Ok() => {
                                let output = Exec::shell("osx-cpu-temp").capture()?.stdout_str().to_owned();
                                match output.trim_end() {
                                    Some(s) => if s.ends_with("°C") {
                                        self.sensor_method = "osx-cpu-temp";
                                    },
                                    None => (),
                                };
                            },
                            Err => (),
                        },
                    }
                }
                Err => (),
            }
        } else if CONFIG.cpu_sensor != "Auto" && CONFIG.cpu_sensor.contains(CONFIG.cpu_sensor) {
            self.sensor_method = "psutil";
        } else {
            for res in temperatures() {
                match res {
                    Ok(temp) => {
                        if temp.unit().to_lowercase().starts_with("cpu"){
                            self.sensor_method = "psutil";
                            break;
                        }
                        match temp.label {
                            Some(label) => {
                                let arr: &str = vec!["Package", "Core 0", "Tdie", "CPU"];

                                for test in arr {
                                    if label.starts_with(test) {
                                        self.sensor_method = "psutil";
                                        break;
                                    }
                                }
                            },
                            None => (),
                        };
                    },
                    Err(e) => (),
                };
            }
        }

        if self.sensor_method == "" && SYSTEM == "Linux" {
            let output : Option<String> = match which("vcgencmd") {
                Some(s) => Some(Exec::shell("vcgencmd measure_temp").capture()?.stdout_str().to_owned()),
                None => None,
            };

            match output {
                Some(s) => if s.trim().endswith("'C"){
                    self.sensor_method = "vcgencmd";
                },
                None => (),
            };

            self.got_sensors = self.sensor_method.chars.count() > 0;
        }
    }

    fn collect(&mut self, collecters : Vec<CollTrait>, draw_now : bool, interrupt : bool, proc_interrupt : bool, redraw : bool, only_draw : bool, t : Term) {
        
        match psutil::cpu::CpuPercentCollector::cpu_percent() {
            Some(p) =>self.cpu_usage[0].push(format!("{:.2}", p)),
            None => (),
        }

        if self.cpu_usage[0] > t.width * 4 {
            self.cpu_usage[0].remove(0);
        }
        
        
    }
}
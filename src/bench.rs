use crate::collector;
use ascii_table::AsciiTable;
use heim::units::{
    information::{gigabyte, megabyte},
    time::{millisecond, second},
};
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::IteratorRandom;
use sysinfo::{System, SystemExt};
use tokio_stream::StreamExt;

#[macro_export]
macro_rules! build_column {
    ($header:expr) => {{
        let mut column = ascii_table::Column::default();
        column.header = $header.into();
        column.align = ascii_table::Align::Right;
        column
    }};
}

#[macro_export]
macro_rules! timeit {
    ($name:literal, $body:expr) => {{
        timeit!($name, $body, NUMRUNS)
    }};
    ($name:literal, $body:expr, $numruns:expr) => {{
        use crate::bench::fmt_time;
        use ascii_table::{Align, AsciiTable, Column};
        use indicatif::{ProgressBar, ProgressStyle};
        let bar = ProgressBar::new($numruns);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "{:<8} : [{{msg:^9}}] |{{bar:40.cyan/blue}}| {{pos:^5}}/{{len:^5}}",
                    $name
                ))
                .progress_chars("##-"),
        );
        let bar_time = std::time::Instant::now();

        let mut times_nanos = vec![];
        for _ in (0..$numruns) {
            bar.inc(1);
            bar.set_message(fmt_time(bar_time.elapsed().as_nanos()));

            let start = std::time::Instant::now();
            $body;
            times_nanos.push(start.elapsed().as_nanos());
        }

        // Calculate the total time it took for all of the bodies to execute, ignore the bar time :)
        let total = times_nanos.iter().sum::<u128>();
        let min = times_nanos.iter().min().unwrap();
        let avg = total / $numruns as u128;
        bar.finish_with_message(format!("avg {:>5}", fmt_time(avg)));
        let max = times_nanos.iter().max().unwrap();
        vec![
            $name.into(),
            $numruns.to_string(),
            fmt_time(total),
            fmt_time(*min),
            fmt_time(avg),
            fmt_time(*max),
        ]
    }};
}

pub fn fmt_time(num_nanos: u128) -> String {
    let power = ((num_nanos as f64).log10().floor() as u128 / 3) + 1;

    let abbr = match power {
        1 => "ns",
        2 => "µs",
        3 => "ms",
        x if x >= 4 => "s ",
        _ => unreachable!(),
    };

    let value = num_nanos / 1000u128.pow(power as u32 - 1);

    format!("{}{}", value, abbr)
}

pub async fn bench() -> heim::Result<()> {
    use ascii_table::{Align, AsciiTable, Column};
    let mut table = AsciiTable::default();
    let mut table_data: Vec<Vec<String>> = Vec::new();
    const NUMRUNS: u64 = 150;

    // Insert column headers
    [
        "name",
        "numruns",
        "total_time",
        "min_time",
        "avg_time",
        "max_time",
    ]
    .iter()
    .enumerate()
    .for_each(|(i, n)| {
        table.columns.insert(i, build_column!(n.to_string()));
    });

    let mut system = System::new_all();
    table_data.push(timeit!("mem", collector::memory::collect().await.unwrap()));
    table_data.push(timeit!("disk", collector::disk::collect().await.unwrap()));
    table_data.push(timeit!("cpu", collector::cpu::collect().await.unwrap()));
    table_data.push(timeit!("cpu(s)", collector::cpu::collect_sync(&system)));
    table_data.push(timeit!("cpu(s/u)", {
        system.refresh_cpu();
        collector::cpu::collect_sync(&system)
    }));
    table_data.push(timeit!("upd", system.refresh_all()));
    table_data.push(timeit!("sys", System::new_all()));
    table_data.push(timeit!("net", collector::network::collect(&system)));
    table_data.push(timeit!("proc", collector::process::collect(&system)));
    table_data.push(timeit!("proc(u)", {
        system.refresh_processes();
        collector::process::collect(&system)
    }));

    let table = table.format(table_data);
    let mut lines = table.lines();
    while let Some(line) = lines.next() {
        let box_width = line.chars().count();
        let padding_f = ((73 - box_width) as f32) / 2.;
        let padding_l = padding_f.ceil() as usize;
        let padding_r = padding_f.floor() as usize;
        println!("{:^73}", line,);
    }
    Ok(())
}

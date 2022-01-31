use crate::collector;
use sysinfo::{System, SystemExt};

#[macro_export]
macro_rules! timeit {
    ($name:literal, $body:expr) => {{
        timeit!($name, $body, NUMRUNS)
    }};
    ($name:literal, $body:expr, $numruns:expr) => {{
        #![allow(unused_imports)]
        use crate::bench::fmt_time;
        use ascii_table::{Align, AsciiTable, Column};
        use indicatif::{ProgressBar, ProgressStyle};
        let bar = ProgressBar::new($numruns);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "{:<8} : [{{msg:^9}}] |{{bar:40.white/gray}}| {{pos:^5}}/{{len:^5}}",
                    $name
                ))
                .progress_chars("▓▒░"),
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

macro_rules! run_tests {
    {$($test_name:literal : $body:expr,)+} => {{
        let mut table_data: Vec<Vec<String>> = Vec::new();

        $(
            table_data.push(timeit!($test_name, $body));
        )+

        table_data
    }};
}

#[inline(always)]
pub fn fmt_time(num_nanos: u128) -> String {
    let power = ((num_nanos as f64).log10().floor() as u128 / 3) + 1;
    let value = num_nanos / 1000u128.pow(power as u32 - 1);

    match power {
        1 => format!("{}ns", value),
        2 => format!("{}µs", value),
        3 => format!("{}ms", value),
        x if x >= 4 => format!("{}s ", value),
        _ => unreachable!(),
    }
}

pub async fn bench() -> heim::Result<()> {
    let mut table = ascii_table::AsciiTable::default();
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
        table.column(i).set_header(n.to_string()).set_align(ascii_table::Align::Right);
    });

    let mut system = System::new_all();
    let mut col = collector::Collector::new().await?;

    let table_data: Vec<Vec<String>> = run_tests! {
        "mem": collector::memory::collect().await.unwrap(),
        "disk": collector::disk::collect().await.unwrap(),
        "cpu": collector::cpu::collect().await.unwrap(),
        "cpu(s)": collector::cpu::collect_sync(&system),
        "net": collector::network::collect(&system),
        "proc": collector::process::collect(&system),
        "upd": system.refresh_all(),
        "sys": System::new_all(),
        "col": col.update().await.unwrap(),
    };

    let table = table.format(table_data);
    let mut lines = table.lines();
    for line in &mut lines {
        println!("{:^73}", line,);
    }
    Ok(())
}

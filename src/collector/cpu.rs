use std::collections::HashMap;

use heim::{
    cpu::{logical_count, physical_count, CpuFrequency, CpuTime},
    units::{frequency::megahertz, time::second},
};
use sysinfo::{ProcessorExt, System, SystemExt};

#[derive(Debug)]
pub struct CpuData {
    // Frequency in MHz
    current_freq: u64,
    min_freq: Option<u64>,
    max_freq: Option<u64>,

    // Time in Seconds
    user_time: f64,
    system_time: f64,
    idle_time: f64,

    logical_count: u64,
    physical_count: Option<u64>,
}
async fn get_time() -> heim::Result<CpuTime> {
    heim::cpu::time().await
}
fn get_user_time(time: &CpuTime) -> f64 {
    time.user().get::<second>()
}
fn get_system_time(time: &CpuTime) -> f64 {
    time.system().get::<second>()
}
fn get_idle_time(time: &CpuTime) -> f64 {
    time.idle().get::<second>()
}

async fn get_frequency() -> heim::Result<CpuFrequency> {
    heim::cpu::frequency().await
}
fn get_current_frequency(frequency: &CpuFrequency) -> u64 {
    frequency.current().get::<megahertz>()
}
fn get_min_frequency(frequency: &CpuFrequency) -> Option<u64> {
    frequency.min().map(|freq| freq.get::<megahertz>())
}
fn get_max_frequency(frequency: &CpuFrequency) -> Option<u64> {
    frequency.max().map(|freq| freq.get::<megahertz>())
}

pub async fn collect() -> heim::Result<CpuData> {
    let (freq, cpu_time, logical_count, physical_count) = tokio::try_join!(
        get_frequency(),
        get_time(),
        logical_count(),
        physical_count(),
    )?;

    Ok(CpuData {
        current_freq: get_current_frequency(&freq),
        min_freq: get_min_frequency(&freq),
        max_freq: get_max_frequency(&freq),
        user_time: get_user_time(&cpu_time),
        system_time: get_system_time(&cpu_time),
        idle_time: get_idle_time(&cpu_time),
        logical_count,
        physical_count,
    })
}

pub struct CpuDataSync {
    freq_mhz: u64,
    // Percent
    usage: f32,
    vendor: String,
    brand: String,
}

pub fn collect_sync(system: &System) -> HashMap<String, CpuDataSync> {
    system
        .processors()
        .into_iter()
        .map(|processor| {
            (
                processor.name().to_string(),
                CpuDataSync {
                    freq_mhz: processor.frequency(),
                    usage: processor.cpu_usage(),
                    vendor: processor.vendor_id().into(),
                    brand: processor.brand().into(),
                },
            )
        })
        .collect()
}

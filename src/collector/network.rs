use sysinfo::{NetworkExt, System, SystemExt};

#[derive(Debug)]
pub struct Network {
    device_name: String,
    total_bytes_received: u64,
    bytes_received: u64,
    total_bytes_transmitted: u64,
    bytes_transmitted: u64,
}

pub fn collect(system: &System) -> Vec<Network> {
    let raw = system.networks();

    raw.into_iter()
        .map(|(name, device)| Network {
            device_name: name.to_string(),
            total_bytes_received: device.total_received(),
            bytes_received: device.received(),
            total_bytes_transmitted: device.total_transmitted(),
            bytes_transmitted: device.transmitted(),
        })
        .collect()
}

use std::path::Path;
use std::str::FromStr;

use heim_common::prelude::*;
use heim_common::units::{information, Information};
use heim_common::utils::iter::*;
use heim_common::Pid;
use heim_runtime as rt;

#[derive(Debug)]
pub struct IoCounters {
    interface: String,
    rx_bytes: Information,
    rx_packets: u64,
    rx_errs: u64,
    rx_drop: u64,
    rx_fifo: u64,
    rx_frame: u64,
    rx_compressed: u64,
    rx_multicast: u64,
    tx_bytes: Information,
    tx_packets: u64,
    tx_errs: u64,
    tx_drop: u64,
    tx_fifo: u64,
    tx_frame: u64,
    tx_compressed: u64,
    tx_multicast: u64,
}

impl IoCounters {
    pub fn interface(&self) -> &str {
        self.interface.as_str()
    }

    pub fn bytes_sent(&self) -> Information {
        self.tx_bytes
    }

    pub fn bytes_recv(&self) -> Information {
        self.rx_bytes
    }

    pub fn packets_sent(&self) -> u64 {
        self.tx_packets
    }

    pub fn packets_recv(&self) -> u64 {
        self.rx_packets
    }

    pub fn errors_sent(&self) -> u64 {
        self.tx_errs
    }

    pub fn errors_recv(&self) -> u64 {
        self.rx_errs
    }

    pub fn drop_recv(&self) -> u64 {
        self.rx_drop
    }

    pub fn drop_sent(&self) -> u64 {
        self.tx_drop
    }
}

impl FromStr for IoCounters {
    type Err = Error;

    // Example:
    // wlp3s0: 550608563  390526    0    0    0 61962          0         0 14822919  103337    0    0    0     0       0
    // 0
    #[allow(clippy::redundant_closure)]
    fn from_str(s: &str) -> Result<IoCounters> {
        let mut parts = s.split_whitespace();
        let interface = match parts.next() {
            Some(str) => str.trim_end_matches(':').to_string(),
            None => {
                return Err(Error::missing_key(
                    "Interface",
                    format!("{}/net/dev", rt::linux::procfs_root().display()),
                ))
            }
        };

        Ok(IoCounters {
            interface,
            rx_bytes: parts
                .try_parse_next()
                .map(|bytes: u64| Information::new::<information::byte>(bytes))?,
            rx_packets: parts.try_parse_next()?,
            rx_errs: parts.try_parse_next()?,
            rx_drop: parts.try_parse_next()?,
            rx_fifo: parts.try_parse_next()?,
            rx_frame: parts.try_parse_next()?,
            rx_compressed: parts.try_parse_next()?,
            rx_multicast: parts.try_parse_next()?,
            tx_bytes: parts
                .try_parse_next()
                .map(|bytes: u64| Information::new::<information::byte>(bytes))?,
            tx_packets: parts.try_parse_next()?,
            tx_errs: parts.try_parse_next()?,
            tx_drop: parts.try_parse_next()?,
            tx_fifo: parts.try_parse_next()?,
            tx_frame: parts.try_parse_next()?,
            tx_compressed: parts.try_parse_next()?,
            tx_multicast: parts.try_parse_next()?,
        })
    }
}

async fn inner<T: AsRef<Path> + Send + 'static>(
    path: T,
) -> Result<impl Stream<Item = Result<IoCounters>>> {
    let lines = rt::fs::read_lines(path.as_ref().to_path_buf())
        .await
        .map_err(|e| Error::from(e).with_file(path.as_ref()))?;

    let stream = lines
        .skip(2)
        .map_err(Error::from)
        .and_then(|line| async move { IoCounters::from_str(&line) });

    Ok(stream)
}

pub async fn io_counters() -> Result<impl Stream<Item = Result<IoCounters>>> {
    inner(rt::linux::procfs_root().join("net/dev")).await
}

pub async fn io_counters_for_pid(pid: Pid) -> Result<impl Stream<Item = Result<IoCounters>>> {
    let path = rt::linux::procfs_root()
        .join(pid.to_string())
        .join("net/dev");

    inner(path).await
}

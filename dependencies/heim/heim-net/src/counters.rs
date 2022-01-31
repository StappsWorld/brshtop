use std::fmt;

use heim_common::prelude::*;
use heim_common::units::Information;

use crate::sys;

/// Network device I/O counters.
pub struct IoCounters(sys::IoCounters);

wrap!(IoCounters, sys::IoCounters);

impl IoCounters {
    /// Returns network interface name.
    pub fn interface(&self) -> &str {
        self.as_ref().interface()
    }

    /// Returns information amount which was sent via this interface.
    // TODO: Method returns `Information`, not the "bytes". Should it be renamed?
    pub fn bytes_sent(&self) -> Information {
        self.as_ref().bytes_sent()
    }

    /// Returns information amount which was received via this interface.
    // TODO: Method returns `Information`, not the "bytes". Should it be renamed?
    pub fn bytes_recv(&self) -> Information {
        self.as_ref().bytes_recv()
    }

    /// Returns packets amount which was sent via this interface.
    pub fn packets_sent(&self) -> u64 {
        self.as_ref().packets_sent()
    }

    /// Returns packets amount which was sent via this interface.
    pub fn packets_recv(&self) -> u64 {
        self.as_ref().packets_recv()
    }

    // TODO: Not sure about methods names below:

    /// Returns errors amount which had occurred while sending data
    /// via this interface.
    pub fn errors_sent(&self) -> u64 {
        self.as_ref().errors_sent()
    }

    /// Returns errors amount which had occurred while receiving data
    /// via this interface.
    pub fn errors_recv(&self) -> u64 {
        self.as_ref().errors_recv()
    }

    /// Returns packets amount which were dropped while receiving them.
    pub fn drop_recv(&self) -> u64 {
        self.as_ref().drop_recv()
    }
}

impl fmt::Debug for IoCounters {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IoCounters")
            .field("interface", &self.interface())
            .field("bytes_sent", &self.bytes_sent())
            .field("bytes_recv", &self.bytes_recv())
            .field("packets_sent", &self.packets_sent())
            .field("packets_recv", &self.packets_recv())
            .field("errors_sent", &self.errors_sent())
            .field("errors_recv", &self.errors_recv())
            .field("drop_recv", &self.drop_recv())
            .finish()
    }
}

/// Returns a stream over the [IO counters] for each network interface.
///
/// ## Compatibility
///
/// Windows implementation is missing, see [related issue](https://github.com/heim-rs/heim/issues/26)
///
/// [IO counters]: struct.IoCounters.html
pub async fn io_counters() -> Result<impl Stream<Item = Result<IoCounters>>> {
    let inner = sys::io_counters().await?;

    Ok(inner.map_ok(Into::into))
}

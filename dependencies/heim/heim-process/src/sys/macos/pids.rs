use heim_common::prelude::{stream, Result, Stream};

use super::bindings;
use crate::sys::unix;
use crate::{Pid, ProcessResult};

pub async fn pids() -> Result<impl Stream<Item = Result<Pid>>> {
    // `kinfo_proc` is not `Send`-able, so it would not be possible
    // later to send it between threads (it's full of raw pointers),
    // so for MVP we are just going to collect all the pids in-place.
    let pids = bindings::processes()?
        .into_iter()
        .map(|proc| Ok(proc.kp_proc.p_pid))
        .collect::<Vec<_>>();

    Ok(stream::iter(pids))
}

pub async fn pid_exists(pid: Pid) -> ProcessResult<bool> {
    Ok(unix::pid_exists(pid))
}

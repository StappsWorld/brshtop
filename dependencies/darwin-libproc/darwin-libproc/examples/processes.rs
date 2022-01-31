use std::io;

// NOTE: It's better to run this example with `sudo` or at least with a SUID bit,
// because some functions will return `Operation not permitted` for any pid given.

fn main() -> io::Result<()> {
    let pids = darwin_libproc::all_pids()?;
    for pid in pids {
        println!("PID: #{}", pid);
        println!("\tName: {:?}", darwin_libproc::name(pid)?);
        println!("\tPath: {:?}", darwin_libproc::pid_path(pid)?);
    }

    Ok(())
}

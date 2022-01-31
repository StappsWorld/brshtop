use std::io;

fn main() -> io::Result<()> {
    let (major, minor) = darwin_libproc::version()?;

    println!("libproc version: {}.{}", major, minor);

    Ok(())
}

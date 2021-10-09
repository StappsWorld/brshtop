use heim::net::Nic;
use sysinfo::{NetworkExt, System, SystemExt};
use tokio_stream::StreamExt;

mod collector;

mod bench;

#[tokio::main]
async fn main() -> heim::Result<()> {
    // let system = System::new_all();
    // loop {
    //     println!("{:#?}", collector::network::collect(&system));
    // }
    bench::bench().await?;
    Ok(())
}

#![allow(dead_code, unused_attributes)]

mod bench;
mod collector;

#[tokio::main]
async fn main() -> heim::Result<()> {
    // let mut collector = Collector::new().await?;
    // println!("Running");
    // loop {
    //     let start = std::time::Instant::now();
    //     collector.update();
    //     tokio::time::sleep_until((start + std::time::Duration::from_millis(10)).into()).await;
    // }
    bench::bench().await?;
    Ok(())
}

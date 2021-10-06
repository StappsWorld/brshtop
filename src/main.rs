mod collector;

mod bench;

#[tokio::main]
async fn main() -> heim::Result<()> {
    // collector::process::collect();
    bench::bench().await?;
    Ok(())
}

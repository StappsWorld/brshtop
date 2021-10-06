use heim::memory::{Memory, Swap};

#[derive(Debug)]
pub struct MemoryData {
    // TODO(Charlie): Visibility
    pub memory: Memory,
    pub swap: Swap,
}

pub async fn collect() -> heim::Result<MemoryData> {
    let (memory, swap) = tokio::try_join!(heim::memory::memory(), heim::memory::swap())?;

    Ok(MemoryData { memory, swap })
}

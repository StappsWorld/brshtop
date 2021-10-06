use std::path::Path;

use heim::disk::{FileSystem, Usage};
use tokio_stream::StreamExt;

#[derive(Debug)]
pub struct DiskData {
    partitions: Vec<Partition>,
    // TODO(Charlie): Figure out how to get partition names on windows :)
}

#[derive(Debug)]
pub struct Partition {
    mount_point: String,
    file_system: FileSystem,
    usage: Usage,
}

fn get_mount_point(partition: &heim::disk::Partition) -> String {
    // TODO(Charlie): Try to find a less shit way to store the mount point,
    // strings are heavy and this clones like 3 times lol
    partition.mount_point().to_string_lossy().into_owned()
}
async fn get_usage<P: AsRef<Path>>(path: P) -> heim::Result<Usage> {
    heim::disk::usage(path).await
}

pub async fn collect() -> heim::Result<DiskData> {
    let mut partitions_stream = heim::disk::partitions();

    let mut partitions = vec![];
    while let Some(partition) = partitions_stream.try_next().await? {
        let mount_point = get_mount_point(&partition);
        let usage = get_usage(&mount_point).await?;

        partitions.push(Partition {
            mount_point,
            usage,
            file_system: partition.file_system().clone(),
        })
    }

    Ok(DiskData { partitions })
}

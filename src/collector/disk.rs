use std::{path::Path, sync::Arc};

use futures::stream::StreamExt;
use heim::disk::{FileSystem, Usage};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct DiskData {
    partitions: Vec<Partition>,
    // TODO(Charlie): Figure out how to get partition names on windows :)
}

#[derive(Debug, Clone)]
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
    let mut partitions_stream = heim::disk::partitions().await?;

    let mut partitions_arc = Arc::new(Mutex::new(vec![]));

    partitions_stream
        .for_each(|r| {
            let partitions = partitions_arc.clone();
            async move {
                match r {
                    Ok(partition) => {
                        let mount_point = get_mount_point(&partition);
                        let usage = match get_usage(&mount_point).await {
                            Ok(u) => u,
                            Err(_) => return,
                        };

                        let mut partitions = partitions.lock().await;
                        partitions.push(Partition {
                            mount_point,
                            usage,
                            file_system: partition.file_system().clone(),
                        })
                    }
                    Err(_) => ()
                }
            }
        })
        .await;

    let partitions: Vec<Partition> = partitions_arc.lock_owned().await.to_vec();
    Ok(DiskData { partitions })
}

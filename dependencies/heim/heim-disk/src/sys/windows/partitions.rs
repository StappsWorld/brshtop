use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use heim_common::prelude::*;

use super::bindings;
use crate::os::windows::{DriveType, Flags};
use crate::FileSystem;

#[derive(Debug)]
pub struct Partition {
    // Might be missing for a remote FS, such as SMB
    volume: Option<OsString>,
    mount_point: PathBuf,
    drive_type: Option<DriveType>,
    flags: Flags,
    file_system: FileSystem,
}

impl Partition {
    #[allow(clippy::option_as_ref_deref)] // >= 1.40.0
    pub fn device(&self) -> Option<&OsStr> {
        self.volume.as_ref().map(OsString::as_os_str)
    }

    pub fn mount_point(&self) -> &Path {
        self.mount_point.as_path()
    }

    pub fn file_system(&self) -> &FileSystem {
        &self.file_system
    }

    pub fn flags(&self) -> Flags {
        self.flags
    }

    pub fn drive_type(&self) -> Option<DriveType> {
        self.drive_type
    }
}

pub async fn partitions() -> Result<impl Stream<Item = Result<Partition>>> {
    let drives = bindings::Drives::new()?;

    let iter = drives.filter_map(|drive| match drive.information() {
        Ok(Some((drive_type, flags, file_system))) => Some(Ok(Partition {
            volume: drive.volume_name().ok(),
            mount_point: drive.to_path_buf(),
            file_system,
            drive_type,
            flags,
        })),
        Ok(None) => None,
        Err(e) => Some(Err(e)),
    });

    Ok(stream::iter(iter))
}

pub async fn partitions_physical() -> Result<impl Stream<Item = Result<Partition>>> {
    let stream = partitions().await?;
    let stream = stream.try_filter(|drive| {
        let result = match drive.drive_type {
            Some(DriveType::NoRootDir) => false,
            Some(DriveType::Remote) => false,
            Some(DriveType::RamDisk) => false,
            None => false,
            _ => true,
        };

        future::ready(result)
    });

    Ok(stream)
}

use std::path::Path;

use crate::device::DeviceInfo;
use crate::error::Result;

/// Normalized track metadata read from or written to an iPod database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackMetadata {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub duration_ms: u32,
    pub has_artwork: bool,
}

/// Abstract backend interface for interacting with iPod databases.
///
/// Decoupling the UI and core workflows behind this trait allows supporting
/// `libgpod` initially while leaving room for a pure-Rust or newer backend later.
pub trait IpodBackend {
    fn inspect_device(&self, mount_path: &Path) -> Result<DeviceInfo>;
    fn list_tracks(&self, mount_path: &Path) -> Result<Vec<TrackMetadata>>;
}

use std::path::PathBuf;

/// Supported iPod models in the initial v0.1.x scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeviceModel {
    Classic1G,
    Classic2G,
    Classic3G,
    Classic4G,
    Classic5G,
    Classic6G,
    Classic7G,
    Mini1G,
    Mini2G,
    Nano1G,
    Nano2G,
    Nano3G,
    Nano4G,
    Nano5G,
}

impl DeviceModel {
    /// Returns a human-readable model name.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Classic1G => "iPod Classic (1st Gen)",
            Self::Classic2G => "iPod Classic (2nd Gen)",
            Self::Classic3G => "iPod Classic (3rd Gen)",
            Self::Classic4G => "iPod Classic (4th Gen)",
            Self::Classic5G => "iPod Classic (5th Gen / Video)",
            Self::Classic6G => "iPod Classic (6th Gen)",
            Self::Classic7G => "iPod Classic (7th Gen)",
            Self::Mini1G => "iPod Mini (1st Gen)",
            Self::Mini2G => "iPod Mini (2nd Gen)",
            Self::Nano1G => "iPod Nano (1st Gen)",
            Self::Nano2G => "iPod Nano (2nd Gen)",
            Self::Nano3G => "iPod Nano (3rd Gen)",
            Self::Nano4G => "iPod Nano (4th Gen)",
            Self::Nano5G => "iPod Nano (5th Gen)",
        }
    }

    /// Returns whether this model supports album artwork (`ArtworkDB`).
    pub const fn supports_artwork(self) -> bool {
        !matches!(
            self,
            Self::Classic1G | Self::Classic2G | Self::Classic3G | Self::Mini1G | Self::Mini2G
        )
    }
}

/// Filesystem format detected on the mounted iPod.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemType {
    Fat32,
    HfsPlus,
    Unknown,
}

/// Basic metadata about a connected iPod device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub mount_path: PathBuf,
    pub model: DeviceModel,
    pub filesystem: FilesystemType,
    pub has_sysinfo_extended: bool,
    pub firewire_guid: Option<String>,
}

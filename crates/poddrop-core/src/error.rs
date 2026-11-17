use std::fmt;

/// Unified error type for PodDrop core operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodDropError {
    DeviceNotFound(String),
    UnsupportedModel(String),
    ReadOnlyFilesystem(String),
    Io(String),
    Backend(String),
}

impl fmt::Display for PodDropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceNotFound(path) => write!(f, "iPod device not found at: {path}"),
            Self::UnsupportedModel(model) => write!(f, "Unsupported iPod model: {model}"),
            Self::ReadOnlyFilesystem(fs) => write!(f, "Read-only filesystem detected: {fs}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Backend(msg) => write!(f, "Backend error: {msg}"),
        }
    }
}

impl std::error::Error for PodDropError {}

pub type Result<T> = std::result::Result<T, PodDropError>;

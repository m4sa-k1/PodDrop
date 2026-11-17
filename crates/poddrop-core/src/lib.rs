//! Core domain types, backend abstractions, and localization for PodDrop.

pub mod backend;
pub mod backup;
pub mod device;
pub mod error;
pub mod gpod;
pub mod i18n;
pub mod media;

pub const APP_NAME: &str = "PodDrop";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

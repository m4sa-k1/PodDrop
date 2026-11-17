use std::process::Command;

use crate::backup::default_backup_root;
use crate::{APP_NAME, APP_VERSION};

/// Runtime system and dependency diagnostics for PodDrop.
#[derive(Debug, Clone)]
pub struct SystemDiagnostics {
    pub app_name: String,
    pub app_version: String,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
    pub udisksctl_status: String,
    pub backup_dir: String,
}

/// Collects current application and external tool status.
pub fn collect_system_diagnostics() -> SystemDiagnostics {
    SystemDiagnostics {
        app_name: APP_NAME.to_string(),
        app_version: APP_VERSION.to_string(),
        ffmpeg_version: tool_first_line("ffmpeg", &["-version"]),
        ffprobe_version: tool_first_line("ffprobe", &["-version"]),
        udisksctl_status: tool_first_line("udisksctl", &["--version"]),
        backup_dir: default_backup_root().display().to_string(),
    }
}

fn tool_first_line(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let line = stdout.lines().next().unwrap_or("Available").trim();
            if line.is_empty() {
                "Available".to_string()
            } else {
                line.to_string()
            }
        }
        Ok(_) => "Error / Unavailable".to_string(),
        Err(_) => "Not found".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_non_empty_diagnostics() {
        let diag = collect_system_diagnostics();
        assert_eq!(diag.app_name, "PodDrop");
        assert!(!diag.app_version.is_empty());
        assert!(!diag.backup_dir.is_empty());
    }
}

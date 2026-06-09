use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Local;
use serde::Serialize;

pub struct NdjsonWriter {
    output_dir: PathBuf,
}

impl NdjsonWriter {
    pub fn new(output_dir: &str) -> Self {
        let expanded = if output_dir.starts_with("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(&output_dir[2..])
        } else {
            PathBuf::from(output_dir)
        };
        Self {
            output_dir: expanded,
        }
    }

    /// Creates a timestamped snapshot directory: `{output_dir}/YYYYMMDD_HHMM/`
    /// with `comments/` and `danmaku/` subdirectories.
    /// Returns the snapshot directory path.
    pub fn create_snapshot_dir(&self) -> crate::error::Result<PathBuf> {
        let now = Local::now();
        let dir_name = now.format("%Y%m%d_%H%M").to_string();
        let dir_path = self.output_dir.join(&dir_name);
        fs::create_dir_all(&dir_path)?;
        fs::create_dir_all(dir_path.join("comments"))?;
        fs::create_dir_all(dir_path.join("danmaku"))?;
        Ok(dir_path)
    }

    /// Creates (or truncates) a file at `path` and writes each element of `data`
    /// as a single JSON line.
    pub fn write_ndjson<T: Serialize>(&self, path: &Path, data: &[T]) -> crate::error::Result<()> {
        let mut file = File::create(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }

    /// Opens (or creates) a file at `path` in append mode and writes each element
    /// of `data` as a single JSON line.
    pub fn append_ndjson<T: Serialize>(&self, path: &Path, data: &[T]) -> crate::error::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }
}

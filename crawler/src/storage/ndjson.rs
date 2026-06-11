use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Local;
use serde::Serialize;

/// Thread-safe NDJSON writer.
///
/// All write operations are serialized through a `Mutex` so multiple tokio
/// tasks can safely append to the same files.
pub struct NdjsonWriter {
    output_dir: PathBuf,
    lock: Mutex<()>,
}

impl NdjsonWriter {
    pub fn new(output_dir: &str) -> Self {
        let expanded = if let Some(stripped) = output_dir.strip_prefix("~/") {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(stripped)
        } else {
            PathBuf::from(output_dir)
        };
        Self {
            output_dir: expanded,
            lock: Mutex::new(()),
        }
    }

    /// Creates a daily snapshot directory: `{output_dir}/YYYYMMDD/`
    /// with `comments/` and `danmaku/` subdirectories.
    /// Reuses the existing directory if it already exists (idempotent).
    pub fn create_snapshot_dir(&self) -> crate::error::Result<PathBuf> {
        let now = Local::now();
        let dir_name = now.format("%Y%m%d").to_string();
        let dir_path = self.output_dir.join(&dir_name);
        fs::create_dir_all(&dir_path)?;
        fs::create_dir_all(dir_path.join("video_detail"))?;
        fs::create_dir_all(dir_path.join("up_info"))?;
        fs::create_dir_all(dir_path.join("comments"))?;
        fs::create_dir_all(dir_path.join("danmaku"))?;
        Ok(dir_path)
    }

    /// Creates (or truncates) a file and writes each element as a JSON line.
    pub fn write_ndjson<T: Serialize>(&self, path: &Path, data: &[T]) -> crate::error::Result<()> {
        let _guard = self.lock.lock().unwrap();
        let mut file = File::create(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }

    /// Opens (or creates) a file in append mode and writes each element as a JSON line.
    /// Thread-safe via internal mutex.
    pub fn append_ndjson<T: Serialize>(&self, path: &Path, data: &[T]) -> crate::error::Result<()> {
        let _guard = self.lock.lock().unwrap();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }
}

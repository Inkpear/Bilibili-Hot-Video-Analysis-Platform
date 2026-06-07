use std::fs::{self, File, OpenOptions};
use std::io::Write;
use serde::Serialize;
use anyhow::Result;
use chrono::Local;

pub struct NdjsonWriter {
    output_dir: String,
}

impl NdjsonWriter {
    pub fn new(output_dir: &str) -> Self {
        Self {
            output_dir: output_dir.to_string(),
        }
    }

    pub fn create_snapshot_dir(&self) -> Result<String> {
        let now = Local::now();
        let dir_name = now.format("%Y%m%d_%H%M").to_string();
        let dir_path = format!("{}/{}", self.output_dir, dir_name);
        fs::create_dir_all(&dir_path)?;
        fs::create_dir_all(format!("{}/comments", dir_path))?;
        fs::create_dir_all(format!("{}/danmaku", dir_path))?;
        Ok(dir_path)
    }

    pub fn write_ndjson<T: Serialize>(&self, path: &str, data: &[T]) -> Result<()> {
        let mut file = File::create(path)?;
        for item in data {
            let json = serde_json::to_string(item)?;
            writeln!(file, "{}", json)?;
        }
        Ok(())
    }

    pub fn append_ndjson<T: Serialize>(&self, path: &str, data: &[T]) -> Result<()> {
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

use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct Logger {
    log_file: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self, std::io::Error> {
        let log_dir = dirs::config_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found"))?
            .join("giterdone")
            .join("logs");
        fs::create_dir_all(&log_dir)?;
        let log_file = log_dir.join("giterdone.log");
        Ok(Logger { log_file })
    }

    pub fn log(&self, message: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.log_file)?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        writeln!(file, "[{}]: {}", timestamp, message)
    }
}

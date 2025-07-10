use std::process::Command;
use crate::logger::Logger;

pub fn setup_cron_job(schedule: &str, logger: &Logger) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;
    let command_to_run = format!("{} --run-now", current_exe.to_str().unwrap());

    // Get current crontab
    let current_crontab = Command::new("crontab")
        .arg("-l")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "".to_string());

    // Remove old giterdone jobs
    let new_crontab: String = current_crontab
        .lines()
        .filter(|line| !line.contains("giterdone"))
        .collect::<Vec<_>>()
        .join("\n");

    // Add new job
    let new_job = format!("{} {}", schedule, command_to_run);
    let final_crontab = format!("{}\n{}\n", new_crontab, new_job);

    // Install new crontab
    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn crontab command: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        std::io::Write::write_all(&mut stdin, final_crontab.as_bytes())
            .map_err(|e| format!("Failed to write to crontab stdin: {}", e))?;
    } else {
        return Err("Failed to get crontab stdin".to_string());
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for crontab command: {}", e))?;
    if !status.success() {
        return Err(format!("crontab command failed with status: {}", status));
    }

    logger.log(&format!("Cron job set up with schedule: {}", schedule)).unwrap();
    Ok(())
}

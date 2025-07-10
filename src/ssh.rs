use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use crate::logger::Logger;

pub fn setup_ssh_key(key_content: &str, logger: &Logger) -> Result<PathBuf, String> {
    let ssh_dir = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?
        .join(".ssh");

    fs::create_dir_all(&ssh_dir)
        .map_err(|e| format!("Failed to create ~/.ssh directory: {}", e))?;

    let key_path = ssh_dir.join("id_rsa"); // Default key path

    if key_path.exists() {
        logger.log(&format!("Warning: SSH key already exists at {:?}. Overwriting.", key_path)).unwrap();
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true) // Overwrite existing content
        .open(&key_path)
        .map_err(|e| format!("Failed to open SSH key file {:?}: {}", key_path, e))?;

    file.write_all(key_content.as_bytes())
        .map_err(|e| format!("Failed to write SSH key to {:?}: {}", key_path, e))?;

    // Set permissions to 0o600 (read/write for owner only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&key_path).map_err(|e| format!("Failed to get SSH key file metadata: {}", e))?.permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&key_path, permissions).map_err(|e| format!("Failed to set SSH key file permissions: {}", e))?;
    }

    logger.log(&format!("SSH key saved to {:?} with permissions 0o600.", key_path)).unwrap();
    Ok(key_path)
}

pub fn add_github_to_known_hosts(logger: &Logger) -> Result<(), String> {
    let known_hosts_path = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?
        .join(".ssh").join("known_hosts");

    let output = Command::new("ssh-keyscan")
        .arg("github.com")
        .output()
        .map_err(|e| format!("Failed to run ssh-keyscan: {}. Make sure it is installed and in your PATH.", e))?;

    if !output.status.success() {
        return Err(format!("ssh-keyscan failed: {}\n{}",
                           String::from_utf8_lossy(&output.stderr),
                           String::from_utf8_lossy(&output.stdout)));
    }

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true) // Append to existing content
        .open(&known_hosts_path)
        .map_err(|e| format!("Failed to open known_hosts file {:?}: {}", known_hosts_path, e))?;

    file.write_all(&output.stdout)
        .map_err(|e| format!("Failed to write to known_hosts file {:?}: {}", known_hosts_path, e))?;

    logger.log(&format!("github.com added to {:?}.", known_hosts_path)).unwrap();
    Ok(())
}
use std::process::{Command, Stdio};
use std::path::{Path, PathBuf};
use crate::config::{Config, AuthMethod};
use crate::logger::Logger;

pub fn ensure_repo(config: &Config, logger: &Logger) -> Result<(), String> {
    let repo_path = get_repo_path(&config.repo_url);
    if !repo_path.exists() {
        logger.log("Local repository not found, cloning...").unwrap();
        clone_repo(config, &repo_path, logger)?;
    } else {
        logger.log("Local repository found, checking remote...").unwrap();
        validate_remote(config, &repo_path, logger)?;
    }
    Ok(())
}

pub fn add_commit_push(config: &Config, message: &str, dry_run: bool, logger: &Logger) -> Result<(), String> {
    let repo_path = get_repo_path(&config.repo_url);
    add(&repo_path, logger)?;
    commit(message, &repo_path, dry_run, logger)?;
    if !dry_run {
        push(config, &repo_path, logger)?;
    }
    Ok(())
}

fn get_repo_path(repo_url: &str) -> std::path::PathBuf {
    // Heuristic to get a good local repo path from the URL
    let repo_name = repo_url.split('/').last().unwrap_or("giterdone-backup");
    let repo_name = repo_name.trim_end_matches(".git");
    dirs::config_dir().unwrap().join("giterdone").join(repo_name)
}

fn clone_repo(config: &Config, path: &Path, logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.arg("clone");

    let clone_url = match config.auth {
        AuthMethod::Ssh => convert_https_to_ssh(&config.repo_url),
    };
    command.arg(clone_url).arg(path);
    
    // Set GIT_SSH_COMMAND if a custom key path was provided (e.g., id_rsa)
    if let Some(ssh_key_path) = get_ssh_key_path() {
        command.env("GIT_SSH_COMMAND", format!("ssh -i {}", ssh_key_path.display()));
    }

    execute_git_command(command, "clone", logger)
}

fn validate_remote(config: &Config, path: &Path, _logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.current_dir(path).arg("remote").arg("-v");
    let output = command.output().map_err(|e| format!("Failed to execute git remote: {}", e))?;
    let remote_output = String::from_utf8_lossy(&output.stdout);

    let expected_url = match config.auth {
        AuthMethod::Ssh => convert_https_to_ssh(&config.repo_url),
    };

    if !remote_output.contains(&expected_url) {
        return Err(format!("Remote URL mismatch. Expected: {}, Found: {}", expected_url, remote_output));
    }
    Ok(())
}

fn add(path: &Path, logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.current_dir(path).arg("add").arg(".");
    execute_git_command(command, "add", logger)
}

fn commit(message: &str, path: &Path, dry_run: bool, logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.current_dir(path).arg("commit").arg("-m").arg(message);
    if dry_run {
        command.arg("--dry-run");
    }
    execute_git_command(command, "commit", logger)
}

fn push(config: &Config, path: &Path, logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.current_dir(path);
    command.arg("push");

    // Set GIT_SSH_COMMAND if a custom key path was provided (e.g., id_rsa)
    if let Some(ssh_key_path) = get_ssh_key_path() {
        command.env("GIT_SSH_COMMAND", format!("ssh -i {}", ssh_key_path.display()));
    }

    execute_git_command(command, "push", logger)
}

fn execute_git_command(mut command: Command, operation: &str, logger: &Logger) -> Result<(), String> {
    logger.log(&format!("Executing git {}", operation)).unwrap();
    let status = command.stdout(Stdio::piped()).stderr(Stdio::piped()).status()
        .map_err(|e| format!("Failed to execute git {}: {}", operation, e))?;

    if !status.success() {
        let output = command.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let error_message = format!("git {} failed: {}\n{}", operation, status, stderr);
        logger.log(&error_message).unwrap();
        return Err(error_message);
    }
    logger.log(&format!("git {} successful", operation)).unwrap();
    Ok(())
}

fn convert_https_to_ssh(https_url: &str) -> String {
    https_url
        .replace("https://github.com/", "git@github.com:")
        .replace(".git", "") // Remove .git if present, as SSH URLs often omit it
}

// Helper to get the default SSH key path
fn get_ssh_key_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".ssh").join("id_rsa"))
}

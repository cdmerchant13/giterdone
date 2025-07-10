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
        logger.log("Local repository found, synchronizing with remote...").unwrap();
        
        let current_dir_command = |cmd: &mut Command| { cmd.current_dir(&repo_path); };

        // Fetch latest changes from remote
        let mut fetch_cmd = Command::new("git");
        fetch_cmd.arg("fetch").arg("origin");
        execute_git_command_with_dir(fetch_cmd, current_dir_command, "fetch", logger)?;

        // Check if remote main branch exists
        let remote_main_exists = Command::new("git")
            .current_dir(&repo_path)
            .arg("branch")
            .arg("-r")
            .output()
            .map_err(|e| format!("Failed to check remote branches: {}", e))?;
        
        let remote_main_exists = String::from_utf8_lossy(&remote_main_exists.stdout).contains("origin/main");

        if remote_main_exists {
            logger.log("Remote 'main' branch found. Resetting local to remote...").unwrap();
            let mut reset_cmd = Command::new("git");
            reset_cmd.arg("reset").arg("--hard").arg("origin/main");
            execute_git_command_with_dir(reset_cmd, current_dir_command, "reset --hard", logger)?;
        } else {
            logger.log("Remote 'main' branch not found. Ensuring local branch is pushed...").unwrap();
            // Ensure local 'main' branch exists and is pushed as new upstream
            let mut checkout_cmd = Command::new("git");
            checkout_cmd.arg("checkout").arg("-b").arg("main");
            execute_git_command_with_dir(checkout_cmd, current_dir_command, "checkout -b main", logger).ok(); // Create if not exists
            
            let mut push_u_cmd = Command::new("git");
            push_u_cmd.arg("push").arg("-u").arg("origin").arg("main");
            execute_git_command_with_dir(push_u_cmd, current_dir_command, "push -u origin main", logger)?;
        }

        // Ensure 'alt' branch exists locally, but don't reset it automatically
        let mut checkout_alt_cmd = Command::new("git");
        checkout_alt_cmd.arg("checkout").arg("-b").arg("alt");
        execute_git_command_with_dir(checkout_alt_cmd, current_dir_command, "checkout -b alt", logger).ok(); // Create if not exists

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
    let result = execute_git_command(command, "commit", logger);

    if let Err(e) = &result {
        if e.contains("nothing to commit") || e.contains("no changes added to commit") {
            logger.log("No changes to commit. Treating as successful.").unwrap();
            return Ok(());
        }
    }
    result
}

fn push(_config: &Config, path: &Path, logger: &Logger) -> Result<(), String> {
    let mut command = Command::new("git");
    command.current_dir(path);
    command.arg("push").arg("origin").arg("main");

    // Set GIT_SSH_COMMAND if a custom key path was provided (e.g., id_rsa)
    if let Some(ssh_key_path) = get_ssh_key_path() {
        command.env("GIT_SSH_COMMAND", format!("ssh -i {}", ssh_key_path.display()));
    }

    let result = execute_git_command(command, "push to main", logger);

    if let Err(e) = &result {
        if e.contains("rejected") || e.contains("fetch first") {
            logger.log("Push to 'main' rejected due to divergent history. Attempting push to 'alt'...").unwrap();
            let mut alt_command = Command::new("git");
            alt_command.current_dir(path);
            alt_command.arg("push").arg("origin").arg("alt");
            if let Some(ssh_key_path) = get_ssh_key_path() {
                alt_command.env("GIT_SSH_COMMAND", format!("ssh -i {}", ssh_key_path.display()));
            }
            let alt_result = execute_git_command(alt_command, "push to alt", logger);

            if let Err(e_alt) = &alt_result {
                if e_alt.contains("rejected") || e_alt.contains("fetch first") {
                    logger.log("Push to 'alt' also rejected. Attempting force push to 'alt'...").unwrap();
                    let mut force_alt_command = Command::new("git");
                    force_alt_command.current_dir(path);
                    force_alt_command.arg("push").arg("--force").arg("origin").arg("alt");
                    if let Some(ssh_key_path) = get_ssh_key_path() {
                        force_alt_command.env("GIT_SSH_COMMAND", format!("ssh -i {}", ssh_key_path.display()));
                    }
                    return execute_git_command(force_alt_command, "force push to alt", logger);
                }
            }
            return alt_result;
        }
    }
    result
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

// Helper function to execute git commands with a custom directory closure
fn execute_git_command_with_dir<F>(command: Command, dir_setter: F, operation: &str, logger: &Logger) -> Result<(), String>
where
    F: FnOnce(&mut Command),
{
    let mut cmd = command;
    dir_setter(&mut cmd);
    execute_git_command(cmd, operation, logger)
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
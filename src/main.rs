mod cli;
mod config;
mod git;
mod logger;
mod scanner;
mod scheduler;
mod ssh;

use crate::logger::Logger;
use chrono::Local;
use clap::Parser;
use cli::{Cli, Commands};
use config::{AuthMethod, Config};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::fs;

fn main() {
    let cli = Cli::parse();
    let logger = Logger::new().expect("Failed to initialize logger");

    match &cli.command {
        Some(Commands::Init) => {
            println!("Running setup wizard...");
            if let Err(e) = setup_wizard(&logger) {
                eprintln!("Setup failed: {}", e);
                logger.log(&format!("Setup failed: {}", e)).unwrap();
            }
        }
        Some(Commands::RunNow) => {
            println!("Running immediate backup...");
            run_backup(false, &logger);
        }
        Some(Commands::DryRun) => {
            println!("Performing a dry run...");
            run_backup(true, &logger);
        }
        Some(Commands::Status) => {
            match Config::load() {
                Ok(config) => println!("Current configuration:
{:#?}", config),
                Err(_) => println!("Configuration file not found. Run 'giterdone init' to set up."),
            }
        }
        None => {
            // Default action: run backup if config exists
            if Config::load().is_ok() {
                run_backup(false, &logger);
            } else {
                println!("Configuration not found. Running setup wizard...");
                if let Err(e) = setup_wizard(&logger) {
                    eprintln!("Setup failed: {}", e);
                    logger.log(&format!("Setup failed: {}", e)).unwrap();
                } else {
                    // Run a backup immediately after setup
                    run_backup(false, &logger);
                }
            }
        }
    }
}

fn run_backup(dry_run: bool, logger: &Logger) {
    logger.log("Starting backup process...").unwrap();
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            let msg = format!("Failed to load config: {}. Run 'giterdone init'.", e);
            eprintln!("{}", msg);
            logger.log(&msg).unwrap();
            return;
        }
    };

    // Ensure SSH setup is complete before proceeding with Git operations
    if let Err(e) = ensure_ssh_setup(logger) {
        let msg = format!("SSH setup failed: {}", e);
        eprintln!("{}", msg);
        logger.log(&msg).unwrap();
        return;
    }

    if let Err(e) = git::ensure_repo(&config, logger) {
        let msg = format!("Git repository validation failed: {}", e);
        eprintln!("{}", msg);
        logger.log(&msg).unwrap();
        return;
    }

    // The local path where the git repo is cloned
    let repo_base_path = get_repo_local_path(&config.repo_url);

    // 1. Scan for files and generate .gitignore content
    let (files_to_backup, gitignore_content) = scanner::scan(&config.files_to_backup);
    
    // 2. Write the .gitignore file
    let gitignore_path = repo_base_path.join(".gitignore");
    if let Err(e) = fs::write(&gitignore_path, gitignore_content) {
        let msg = format!("Failed to write .gitignore: {}", e);
        eprintln!("{}", msg);
        logger.log(&msg).unwrap();
        return;
    }
    logger.log(&format!(".gitignore file written to {:?}", gitignore_path)).unwrap();

    // 3. Copy the discovered files to the local git repo, preserving structure
    for (source_path, relative_dest_path) in &files_to_backup {
        if let Err(e) = copy_file_to_repo(source_path, &repo_base_path, relative_dest_path) {
            let msg = format!("Failed to copy file {:?}: {}", source_path, e);
            eprintln!("{}", msg);
            logger.log(&msg).unwrap();
        }
    }
    logger.log("All files copied to local repository.").unwrap();

    // 4. Add, Commit, and Push
    let timestamp = Local::now().format(&config.commit_message_template).to_string();
    match git::add_commit_push(&config, &timestamp, dry_run, logger) {
        Ok(_) => {
            let msg = if dry_run { "Dry run successful." } else { "Backup successful." };
            println!("{}", msg);
            logger.log(msg).unwrap();
        }
        Err(e) => {
            let msg = format!("Backup process failed: {}", e);
            eprintln!("{}", msg);
            logger.log(&msg).unwrap();
        }
    }
}

fn setup_wizard(logger: &Logger) -> Result<(), String> {
    println!("Welcome to giterdone setup!");

    let repo_url = prompt("Enter the remote GitHub repository URL (e.g., https://github.com/user/repo.git):")?;
    let auth = AuthMethod::Ssh;

    // Ensure SSH setup is complete during initial wizard as well
    ensure_ssh_setup(logger)?;

    let files_str = prompt("Enter files or directories to back up (comma-separated absolute paths):")?;
    let files_to_backup: Vec<PathBuf> = files_str.split(',').map(|s| PathBuf::from(s.trim())).collect();

    let backup_schedule = prompt("Enter backup schedule (e.g., '0 * * * *' for hourly, '@daily', etc.):")?;
    let commit_message_template = prompt("Enter commit message template (e.g., 'Backup on %Y-%m-%d %H:%M:%S'):")?;

    let log_dir = dirs::config_dir().unwrap().join("giterdone/logs");
    fs::create_dir_all(&log_dir).map_err(|e| format!("Failed to create log dir: {}", e))?;
    let log_file = log_dir.join("giterdone.log");

    let config = Config {
        repo_url,
        auth,
        files_to_backup,
        backup_schedule: backup_schedule.clone(),
        commit_message_template,
        log_file,
    };

    // Save config
    config.save().map_err(|e| format!("Failed to save config: {}", e))?;
    println!("Configuration saved successfully.");
    logger.log("Configuration saved.").unwrap();

    // Setup cron job
    scheduler::setup_cron_job(&config.backup_schedule, logger)?;
    println!("Cron job scheduled successfully.");

    // Initial clone
    git::ensure_repo(&config, logger)?;
    println!("Repository cloned and validated.");

    Ok(())
}

fn ensure_ssh_setup(logger: &Logger) -> Result<(), String> {
    let ssh_key_path = dirs::home_dir().map(|home| home.join(".ssh").join("id_rsa"));
    let known_hosts_path = dirs::home_dir().map(|home| home.join(".ssh").join("known_hosts"));

    let mut key_exists = false;
    if let Some(path) = &ssh_key_path {
        if path.exists() {
            key_exists = true;
        }
    }

    let mut github_known = false;
    if let Some(path) = &known_hosts_path {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if content.contains("github.com") {
                    github_known = true;
                }
            }
        }
    }

    if !key_exists {
        println!("\nSSH private key (~/.ssh/id_rsa) not found.");
        let setup_ssh = prompt_bool("Do you want to provide your SSH private key now? (y/n)")?;
        if setup_ssh {
            println!("Paste your SSH private key (e.g., content of ~/.ssh/id_rsa). Press Enter twice when done:");
            let key_content = read_multiline_input()?;
            ssh::setup_ssh_key(&key_content, logger)?;
        } else {
            return Err("SSH key not provided. Cannot proceed with Git operations.".to_string());
        }
    }

    if !github_known {
        println!("\nGitHub's host key not found in ~/.ssh/known_hosts.");
        let add_host = prompt_bool("Do you want to add github.com to known_hosts now? (y/n)")?;
        if add_host {
            ssh::add_github_to_known_hosts(logger)?;
        } else {
            return Err("GitHub's host key not added to known_hosts. Cannot proceed with Git operations.".to_string());
        }
    }

    Ok(())
}

fn prompt(message: &str) -> Result<String, String> {
    print!("{} ", message);
    io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;
    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| format!("Failed to read input: {}", e))?;
    Ok(input.trim().to_string())
}

fn prompt_bool(message: &str) -> Result<bool, String> {
    loop {
        let input = prompt(message)?.to_lowercase();
        match input.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("Invalid input. Please enter 'y' or 'n'."),
        }
    }
}

fn read_multiline_input() -> Result<String, String> {
    let mut input = String::new();
    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line).map_err(|e| format!("Failed to read input: {}", e))?;
        if line.trim().is_empty() {
            break;
        }
        input.push_str(&line);
    }
    Ok(input.trim().to_string())
}

fn get_repo_local_path(repo_url: &str) -> PathBuf {
    let repo_name = repo_url.split('/').last().unwrap_or("giterdone-backup").trim_end_matches(".git");
    dirs::config_dir().unwrap().join("giterdone").join(repo_name)
}

fn copy_file_to_repo(source_path: &Path, repo_path: &Path, relative_dest_path: &Path) -> io::Result<()> {
    // Create a destination path that mirrors the absolute source path
    let dest_path = repo_path.join(relative_dest_path);
    
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::copy(source_path, &dest_path)?;
    Ok(())
}

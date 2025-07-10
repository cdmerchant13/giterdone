
**giterdone** is a command-line utility written in Go that helps you back up project configuration files to GitHub repositories on a schedule. It’s ideal for developers who want to keep dotfiles, local config, or dev environment settings versioned and safely stored.

## Features

- Interactive setup for first-time use
- Scheduled automatic backups via cron
- Smart `.gitignore` generation
- Handles GitHub auth via SSH or personal access token
- Scans for large or problematic files and excludes them
- Dry-run and verbose modes for testing and debugging
- Clean CLI with subcommands and flags
## Installation

### Requirements

- Go 1.22 or later
- Git installed and accessible from the shell

### Build from source

```bash
git clone https://github.com/cdmerchant13/giterdone.git
cd giterdone
go build -o giterdone
````

Move the binary somewhere in your $PATH:

```
sudo mv giterdone /usr/local/bin/
```

## **Usage**
### **First Run**

Simply run:

```
giterdone
```

If no config exists, you’ll be guided through:

1. Connecting to a GitHub repo (via SSH or token)
    
2. Selecting files or directories to back up
    
3. Generating a .gitignore to exclude unnecessary or large files
    
4. Setting a commit message template
    
5. Choosing a backup schedule (e.g., daily, weekly)
    
6. Installing a cron job to automate future runs
  

Your settings are saved to:

`~/.config/giterdone/config.json

### **Subsequent Runs**

Backups happen automatically via cron. You can also trigger one manually:

```
giterdone --run-now
```

### **Other Commands**

- --init: Re-run setup wizard
    
- --status: Show current config
    
- --dry-run: Simulate backup without pushing
    
- --verbose: Show detailed output
## **Logs**

Backup run logs are stored at:

```
~/.config/giterdone/logs/
```

Each entry includes timestamps and success/failure info.

## **Limitations**

- No support for multiple profiles (yet)
    
- Does not manage GitHub repository creation (use the GitHub UI or CLI first)
    
- Plaintext storage of personal access tokens (if used instead of SSH)

## **Roadmap**

- Optional encryption for token storage
    
- Multi-profile support
    
- Remote repo creation and access checks
    
- Interactive status/report viewer

## **License**
MIT- not sure if I want to expand this to a more robust project or not so reserving the right 

---

Contributions welcome. If you build something cool with it or want to suggest features, open an issue or pull request.

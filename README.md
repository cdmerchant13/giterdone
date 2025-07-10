# giterdone

`giterdone` is a CLI utility for backing up configuration files and directories to a GitHub repository on a schedule. It’s intended for developers who want to version control local system config, dotfiles, or project scaffolding with minimal friction.

This Rust version is statically compiled and built for Linux systems, including musl-based distributions like Alpine.

## Features

- Interactive setup for first-time use
- Backs up to a GitHub repository using SSH
- Automatically generates a `.gitignore` to exclude large or irrelevant files
- Schedules automatic backups via `cron`
- Supports dry-run and verbose logging modes
- Stores configuration in XDG-compliant paths (`~/.config/giterdone`)

## Installation

### Requirements

- Linux system (x86_64, musl-compatible)
- Git installed and available in `$PATH`
- SSH access to GitHub configured (e.g., `~/.ssh/id_rsa`)

### From source (Rust toolchain required)

```bash
git clone https://github.com/cdmerchant13/giterdone.git
cd giterdone
cargo build --release --target x86_64-unknown-linux-musl
```

Copy the binary to your $PATH:
```
sudo cp target/x86_64-unknown-linux-musl/release/giterdone /usr/local/bin/
chmod +x /usr/local/bin/giterdone
```
Usage

First Run

```giterdone```

If no existing config is found, the tool will guide you through setup:

	1.	Enter your GitHub SSH repo URL (e.g., git@github.com:youruser/yourrepo)
	2.	Select files or directories to include in the backup
	3.	Auto-generate and confirm a .gitignore
	4.	Define your commit message template
	5.	Choose a backup schedule (e.g., daily, weekly)
	6.	Install a cron job to automate backups

Settings are saved to:

```~/.config/giterdone/config.json```

Manual Backup

```giterdone --run-now```

Other Flags

	•	--init – Re-run the interactive setup wizard
	•	--status – Print current config
	•	--dry-run – Simulate the next backup
	•	--verbose – Enable detailed logging output

Logging

Backup run logs are saved to:

```~/.config/giterdone/logs/```

Each log includes timestamps and Git output from each run.

Limitations

	•	SSH is the only supported authentication method
	•	Does not create GitHub repositories (you must create and initialize it beforehand)
	•	Config and logs are stored in plaintext

Roadmap

	•	Support for multiple backup profiles
	•	Encrypted token storage (if future PAT support is reintroduced)
	•	GitHub repository creation via API
	•	Crontab management abstraction for portability

License

MIT- not sure if I want to expand this to a more robust project or not so reserving the right
⸻

Contributions and suggestions welcome. Fork it, build on it, or file an issue.

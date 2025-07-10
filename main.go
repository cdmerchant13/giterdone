package main

import (
	"fmt"
	"os"
	"time"

	"github.com/urfave/cli/v2"

	"giterdone/config"
	"giterdone/cron"
	"giterdone/git"
	"giterdone/scanner"
	"giterdone/utils"
)

func main() {
	app := &cli.App{
		Name:  "giterdone",
		Usage: "A Go command-line tool to backup local configuration files or project settings to GitHub repositories.",
		Flags: []cli.Flag{
			&cli.BoolFlag{
				Name:    "init",
				Aliases: []string{"i"},
				Usage:   "Force reinitialization of the configuration",
			},
			&cli.BoolFlag{
				Name:    "run-now",
				Aliases: []string{"r"},
				Usage:   "Perform an immediate backup",
			},
			&cli.BoolFlag{
				Name:    "dry-run",
				Aliases: []string{"d"},
				Usage:   "Simulate backup without writing or pushing",
			},
			&cli.BoolFlag{
				Name:    "status",
				Aliases: []string{"s"},
				Usage:   "Print current configuration status",
			},
			&cli.BoolFlag{
				Name:    "verbose",
				Aliases: []string{"v"},
				Usage:   "Enable detailed logs",
			},
		},
		Action: func(c *cli.Context) error {
			// Set verbose mode
			if c.Bool("verbose") {
				utils.SetVerbose(true)
			}

			cfg, err := config.LoadConfig()
			forceInit := c.Bool("init")

			if err != nil || forceInit {
				if os.IsNotExist(err) || forceInit {
					fmt.Println("Config file not found or --init flag used. Starting setup wizard...")
					cfg, err = config.RunSetupWizard()
					if err != nil {
						return fmt.Errorf("setup wizard failed: %w", err)
					}
					fmt.Println("Setup complete.")
				} else {
					return fmt.Errorf("error loading config: %w", err)
				}
			}

			// Initialize logger after config is loaded
			err = utils.InitLogger(cfg.LogPath)
			if err != nil {
				return fmt.Errorf("failed to initialize logger: %w", err)
			}
			defer utils.CloseLogger()

			// Handle --status flag
			if c.Bool("status") {
				fmt.Println("\n--- Current Configuration ---")
				fmt.Printf("GitHub Repo: %s\n", cfg.GitHubRepo)
				fmt.Printf("Auth Method: %s\n", cfg.AuthMethod)
				fmt.Printf("Include Paths: %v\n", cfg.IncludePaths)
				fmt.Printf("Commit Message Template: %s\n", cfg.CommitMessageTpl)
				fmt.Printf("Backup Frequency: %s\n", cfg.BackupFrequency)
				fmt.Printf("Log Path: %s\n", cfg.LogPath)
				fmt.Println("-----------------------------")
				return nil
			}

			// Get the executable path for cron job
			executablePath, err := os.Executable()
			if err != nil {
				return fmt.Errorf("failed to get executable path: %w", err)
			}

			// Handle --run-now flag or default behavior
			if c.Bool("run-now") || (!c.Bool("init") && !c.Bool("status")) {
				utils.LogMessage("Performing backup...")
				dryRun := c.Bool("dry-run")

				// Check if git repo is dirty before proceeding with operations
				if git.IsGitRepo() {
					isDirty, err := git.IsGitDirty()
					if err != nil {
						return fmt.Errorf("failed to check git dirty status: %w", err)
					}
					if isDirty {
						utils.LogMessage("Warning: Git repository is dirty. Please commit or stash your changes before running giterdone.")
						// For now, we'll proceed, but in a real scenario, you might want to exit or prompt the user.
					}
				}

				// 1. Initialize/clone repo if not exists
				if !git.IsGitRepo() {
					// If not a git repo, try to clone or init
					if cfg.GitHubRepo != "" {
						utils.LogMessage(fmt.Sprintf("Attempting to clone repository %s", cfg.GitHubRepo))
						err = git.CloneRepo(cfg.GitHubRepo, cfg)
						if err != nil {
							utils.LogMessage(fmt.Sprintf("Failed to clone repo, initializing new one: %v", err))
							err = git.InitRepo()
							if err != nil {
								return fmt.Errorf("failed to initialize git repo: %w", err)
							}
							// If we initialized, set the remote origin
							err = git.SetRemoteOrigin(cfg.GitHubRepo, cfg)
							if err != nil {
								return fmt.Errorf("failed to set remote origin: %w", err)
							}
						}
					} else {
						// No GitHub repo configured, just init a local one
						err = git.InitRepo()
						if err != nil {
							return fmt.Errorf("failed to initialize git repo: %w", err)
						}
					}
				} else {
					utils.LogMessage("Already in a Git repository.")
					// Ensure remote is set if it's not already
					if !git.HasRemoteOrigin() && cfg.GitHubRepo != "" {
						err = git.SetRemoteOrigin(cfg.GitHubRepo, cfg)
						if err != nil {
							return fmt.Errorf("failed to set remote origin: %w", err)
						}
					}
				}

				// 2. Add configured files
				filesToInclude, patternsToExclude := scanner.ScanFiles(cfg.IncludePaths)
				utils.LogMessage(fmt.Sprintf("Found %d files to include.", len(filesToInclude)))
				if len(patternsToExclude) > 0 {
					utils.LogMessage(fmt.Sprintf("Found %d patterns to exclude. Generating .gitignore...", len(patternsToExclude)))
					gitignoreContent := scanner.GenerateGitignoreContent(patternsToExclude)
					// Write .gitignore to the current directory (assuming it's the repo root)
					if !dryRun {
						err = scanner.WriteGitignoreFile(".", gitignoreContent)
						if err != nil {
							return fmt.Errorf("failed to write .gitignore: %w", err)
						}
						utils.LogMessage(".gitignore generated.")
					} else {
						utils.LogMessage("Dry run: Skipping .gitignore generation.")
					}
				}

				if !dryRun {
					git.AddFiles(filesToInclude)
				} else {
					utils.LogMessage("Dry run: Skipping adding files to git.")
				}

				// 3. Commit with templated message
				commitMsg, err := utils.GenerateCommitMessage(cfg.CommitMessageTpl, time.Now())
				if err != nil {
					return fmt.Errorf("failed to generate commit message: %w", err)
				}
				if !dryRun {
					git.Commit(commitMsg)
				} else {
					utils.LogMessage(fmt.Sprintf("Dry run: Skipping commit. Commit message would be: %s", commitMsg))
				}

				// 4. Push to remote
				if !dryRun {
					git.Push(cfg)
				} else {
					utils.LogMessage("Dry run: Skipping push to remote.")
				}

				utils.LogMessage("Backup completed.")
			}

			// 5. Install cron job
			err = cron.InstallCronJob(cfg.BackupFrequency, executablePath)
			if err != nil {
				return fmt.Errorf("failed to install cron job: %w", err)
			}

			return nil
		},
	}

	if err := app.Run(os.Args); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}
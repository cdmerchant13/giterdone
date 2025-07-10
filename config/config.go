package config

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"

	"github.com/manifoldco/promptui"
	"giterdone/scanner"
	"giterdone/utils"
)

const (
	configDir  = ".config/mybackup"
	configFile = "config.json"
)

type Config struct {
	GitHubRepo        string   `json:"github_repo"`
	AuthMethod        string   `json:"auth_method"` // "ssh" or "pat"
	PAT               string   `json:"pat,omitempty"`
	IncludePaths      []string `json:"include_paths"`
	CommitMessageTpl  string   `json:"commit_message_template"`
	BackupFrequency   string   `json:"backup_frequency"`
	LogPath           string   `json:"log_path"`
}

func GetConfigPath() (string, error) {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get user home directory: %w", err)
	}
	return filepath.Join(homeDir, configDir, configFile), nil
}

func LoadConfig() (*Config, error) {
	configPath, err := GetConfigPath()
	if err != nil {
		return nil, err
	}

	data, err := ioutil.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	var cfg Config
	if err := json.Unmarshal(data, &cfg); err != nil {
		return nil, fmt.Errorf("failed to unmarshal config: %w", err)
	}

	return &cfg, nil
}

func SaveConfig(cfg *Config) error {
	configPath, err := GetConfigPath()
	if err != nil {
		return err
	}

	configDir := filepath.Dir(configPath)
	if err := os.MkdirAll(configDir, 0700); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	data, err := json.MarshalIndent(cfg, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal config: %w", err)
	}

	if err := ioutil.WriteFile(configPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

func sshKeyExists() bool {
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return false
	}
	sshPath := filepath.Join(homeDir, ".ssh", "id_rsa")
	_, err = os.Stat(sshPath)
	return !os.IsNotExist(err)
}

func RunSetupWizard() (*Config, error) {
	cfg := &Config{}

	fmt.Println("\n--- Giterdone Setup Wizard ---")

	// 1. GitHub repository name or full remote URL
	prompt := promptui.Prompt{
		Label: "GitHub Repository (e.g., user/repo or https://github.com/user/repo.git)",
		Validate: func(input string) error {
			if len(input) == 0 {
				return fmt.Errorf("repository cannot be empty")
			}
			return nil
		},
	}
	result, err := prompt.Run()
	if err != nil {
		return nil, fmt.Errorf("prompt failed %w", err)
	}
	cfg.GitHubRepo = result

	// 2. Git authentication method
	if sshKeyExists() {
		prompt := promptui.Select{
			Label: "Choose Git authentication method",
			Items: []string{"SSH (recommended)", "Personal Access Token (PAT)"},
		}
		_, authMethod, err := prompt.Run()
		if err != nil {
			return nil, fmt.Errorf("prompt failed %w", err)
		}
		if strings.Contains(authMethod, "SSH") {
			cfg.AuthMethod = "ssh"
		} else {
			cfg.AuthMethod = "pat"
			patPrompt := promptui.Prompt{
				Label: "Enter GitHub Personal Access Token (PAT)",
				Mask:  '*',
				Validate: func(input string) error {
					if len(input) == 0 {
						return fmt.Errorf("PAT cannot be empty")
					}
					return nil
				},
			}
			pat, err := patPrompt.Run()
			if err != nil {
				return nil, fmt.Errorf("prompt failed %w", err)
			}
			cfg.PAT = pat
		}
	} else {
		fmt.Println("SSH key (~/.ssh/id_rsa) not found. Using Personal Access Token (PAT) for authentication.")
		cfg.AuthMethod = "pat"
		patPrompt := promptui.Prompt{
			Label: "Enter GitHub Personal Access Token (PAT)",
			Mask:  '*',
			Validate: func(input string) error {
				if len(input) == 0 {
					return fmt.Errorf("PAT cannot be empty")
				}
				return nil
			},
		}
		pat, err := patPrompt.Run()
		if err != nil {
			return nil, fmt.Errorf("prompt failed %w", err)
		}
		cfg.PAT = pat
	}

	// 3. Paths to include
	fmt.Println("\nEnter paths to include (one per line, press Enter on empty line to finish):")
	var includePaths []string
	for {
		prompt := promptui.Prompt{
			Label: fmt.Sprintf("Path %d", len(includePaths)+1),
		}
		path, err := prompt.Run()
		if err != nil {
			if err == promptui.ErrInterrupt { // User pressed Ctrl+C
				return nil, fmt.Errorf("setup interrupted")
			}
			// User pressed Enter on empty line
			break
		}
		if strings.TrimSpace(path) == "" {
			break
		}
		includePaths = append(includePaths, path)
	}
	if len(includePaths) == 0 {
		return nil, fmt.Errorf("at least one path must be included")
	}
	cfg.IncludePaths = includePaths

	// 4. .gitignore generation
	_, patternsToExclude := scanner.ScanFiles(cfg.IncludePaths)
	gitignoreContent := scanner.GenerateGitignoreContent(patternsToExclude)

	fmt.Println("\n--- Generated .gitignore Content Preview ---")
	fmt.Println(gitignoreContent)
	fmt.Println("--------------------------------------------")

	confirmPrompt := promptui.Prompt{
		Label:     "Do you want to use this .gitignore content? (y/N)",
		IsConfirm: true,
	}
	_, err = confirmPrompt.Run()
	if err != nil {
		fmt.Println("Using default .gitignore content or skipping .gitignore generation.")
		// User declined or interrupted, handle as needed. For now, we'll proceed without writing it.
	} else {
		// In a real scenario, you'd write this to the repo's .gitignore
		// For now, we just confirm the content.
		utils.LogMessage("User confirmed .gitignore content. (Not yet written to file)")
	}

	// 5. Backup frequency
	frequencyPrompt := promptui.Select{
		Label: "Select backup frequency",
		Items: []string{"hourly", "daily", "weekly", "monthly", "every 5 minutes", "every 15 minutes", "every 30 minutes", "custom cron"},
	}
	_, freqResult, err := frequencyPrompt.Run()
	if err != nil {
		return nil, fmt.Errorf("prompt failed %w", err)
	}

	if freqResult == "custom cron" {
		customCronPrompt := promptui.Prompt{
			Label: "Enter custom cron string (e.g., '0 0 * * *')",
			Validate: func(input string) error {
				if !utils.IsValidCronSpec(input) {
					return fmt.Errorf("invalid cron string")
				}
				return nil
			},
		}
		cronResult, err := customCronPrompt.Run()
		if err != nil {
			return nil, fmt.Errorf("prompt failed %w", err)
		}
		cfg.BackupFrequency = cronResult
	} else {
		cfg.BackupFrequency = freqResult
	}

	// 6. Commit message template
	prompt = promptui.Prompt{
		Label:   "Commit Message Template (e.g., 'Automated backup on {{.Timestamp}}')",
		Default: "Automated backup on {{.Timestamp}}",
	}
	result, err = prompt.Run()
	if err != nil {
		return nil, fmt.Errorf("prompt failed %w", err)
	}
	cfg.CommitMessageTpl = result

	// 7. Logging
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return nil, fmt.Errorf("failed to get user home directory: %w", err)
	}
	cfg.LogPath = filepath.Join(homeDir, configDir, "logs")

	// Save the configuration
	if err := SaveConfig(cfg); err != nil {
		return nil, fmt.Errorf("failed to save config: %w", err)
	}

	fmt.Println("Configuration saved successfully!")
	return cfg, nil
}

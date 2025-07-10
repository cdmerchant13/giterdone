package git

import (
	"fmt"
	"os/exec"
	"strings"

	"giterdone/config"
	"giterdone/utils"
)

// IsGitRepo checks if the current directory is a Git repository.
func IsGitRepo() bool {
	cmd := exec.Command("git", "rev-parse", "--is-inside-work-tree")
	err := cmd.Run()
	return err == nil
}

// IsGitDirty checks if there are uncommitted changes in the Git repository.
func IsGitDirty() (bool, error) {
	cmd := exec.Command("git", "status", "--porcelain")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return false, fmt.Errorf("error checking git status: %v\n%s", err, out)
	}
	return len(strings.TrimSpace(string(out))) > 0, nil
}

// InitRepo initializes a new Git repository in the current directory.
func InitRepo() error {
	if IsGitRepo() {
		utils.LogMessage("Git repository already initialized.")
		return nil
	}

	utils.LogMessage("Initializing Git repository...")
	cmd := exec.Command("git", "init")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error initializing repo: %v\n%s", err, out)
	}
	utils.LogMessage(fmt.Sprintf("Git repository initialized: %s", out))
	return nil
}

// CloneRepo clones a remote Git repository.
func CloneRepo(repoURL string, cfg *config.Config) error {
	utils.LogMessage(fmt.Sprintf("Cloning repository: %s", repoURL))

	var cmd *exec.Cmd
	if cfg.AuthMethod == "pat" && strings.HasPrefix(repoURL, "https://") {
		// Inject PAT into the URL for HTTPS cloning
		parts := strings.SplitN(repoURL, "https://", 2)
		authenticatedURL := fmt.Sprintf("https://oauth2:%s@%s", cfg.PAT, parts[1])
		cmd = exec.Command("git", "clone", authenticatedURL, ".")
	} else {
		cmd = exec.Command("git", "clone", repoURL, ".")
	}

	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error cloning repo: %v\n%s", err, out)
	}
	utils.LogMessage(fmt.Sprintf("Repository cloned: %s", out))
	return nil
}

// AddFiles adds specified files to the Git staging area.
func AddFiles(paths []string) error {
	utils.LogMessage("Adding files to Git...")
	args := []string{"add"}
	args = append(args, paths...)
	cmd := exec.Command("git", args...)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error adding files: %v\n%s", err, out)
	}
	utils.LogMessage(fmt.Sprintf("Files added: %s", out))
	return nil
}

// Commit creates a new Git commit with the given message.
func Commit(message string) error {
	utils.LogMessage("Committing changes...")
	cmd := exec.Command("git", "commit", "-m", message)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error committing: %v\n%s", err, out)
	}
	utils.LogMessage(fmt.Sprintf("Changes committed: %s", out))
	return nil
}

// Push pushes committed changes to the remote repository.
func Push(cfg *config.Config) error {
	utils.LogMessage("Pushing to remote...")

	// Ensure the remote URL is correctly set for PAT authentication
	if cfg.AuthMethod == "pat" && strings.HasPrefix(cfg.GitHubRepo, "https://") {
		currentRemoteURL, err := GetRemoteOriginURL()
		if err != nil || !strings.Contains(currentRemoteURL, cfg.PAT) {
			// If remote URL is not set or doesn't contain PAT, set it.
			err = SetRemoteOrigin(cfg.GitHubRepo, cfg)
			if err != nil {
				return fmt.Errorf("failed to set remote origin for push with PAT: %w", err)
			}
		}
	}

	cmd := exec.Command("git", "push")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error pushing: %v\n%s", err, out)
	}
	utils.LogMessage(fmt.Sprintf("Pushed to remote: %s", out))
	return nil
}

// HasRemoteOrigin checks if a remote named 'origin' exists.
func HasRemoteOrigin() bool {
	cmd := exec.Command("git", "remote", "get-url", "origin")
	err := cmd.Run()
	return err == nil
}

// GetRemoteOriginURL gets the URL of the remote named 'origin'.
func GetRemoteOriginURL() (string, error) {
	cmd := exec.Command("git", "remote", "get-url", "origin")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("error getting remote origin URL: %v\n%s", err, out)
	}
	return strings.TrimSpace(string(out)), nil
}

// SetRemoteOrigin sets the remote origin for the repository.
func SetRemoteOrigin(repoURL string, cfg *config.Config) error {
	utils.LogMessage(fmt.Sprintf("Setting remote origin to: %s", repoURL))

	var actualRepoURL string
	if cfg.AuthMethod == "pat" && strings.HasPrefix(repoURL, "https://") {
		parts := strings.SplitN(repoURL, "https://", 2)
		actualRepoURL = fmt.Sprintf("https://oauth2:%s@%s", cfg.PAT, parts[1])
	} else {
		actualRepoURL = repoURL
	}

	cmd := exec.Command("git", "remote", "add", "origin", actualRepoURL)
	out, err := cmd.CombinedOutput()
	if err != nil {
		// If origin already exists, try to set-url
		if strings.Contains(strings.ToLower(string(out)), "remote origin already exists") {
			utils.LogMessage("Remote 'origin' already exists, attempting to set URL.")
			cmd = exec.Command("git", "remote", "set-url", "origin", actualRepoURL)
			out, err = cmd.CombinedOutput()
			if err != nil {
				return fmt.Errorf("error setting remote origin URL: %v\n%s", err, out)
			}
		} else {
			return fmt.Errorf("error adding remote origin: %v\n%s", err, out)
		}
	}
	utils.LogMessage(fmt.Sprintf("Remote origin set: %s", out))
	return nil
}

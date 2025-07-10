package cron

import (
	"fmt"
	"os/exec"
	"strings"

	"giterdone/utils"
)

// InstallCronJob installs or updates the giterdone cron job.
func InstallCronJob(frequency, appPath string) error {
	utils.LogMessage(fmt.Sprintf("Attempting to install cron job for frequency: %s, app path: %s", frequency, appPath))

	cronSpec, err := frequencyToCronSpec(frequency)
	if err != nil {
		return fmt.Errorf("invalid backup frequency: %w", err)
	}

	currentCrontab, err := getCrontab()
	if err != nil {
		return fmt.Errorf("failed to read crontab: %w", err)
	}

	jobEntry := fmt.Sprintf("%s %s --run-now # Giterdone backup job\n", cronSpec, appPath)

	newCrontab := removeExistingGiterdoneJob(currentCrontab)
	newCrontab += jobEntry

	utils.LogMessage(fmt.Sprintf("Writing new crontab entry: %s", strings.TrimSpace(jobEntry)))
	return writeCrontab(newCrontab)
}

// getCrontab reads the current user's crontab.
func getCrontab() (string, error) {
	cmd := exec.Command("crontab", "-l")
	out, err := cmd.CombinedOutput()
	if err != nil {
		// If crontab is empty, it returns an error, but we can treat it as empty string
		if strings.Contains(strings.ToLower(string(out)), "no crontab for") {
			return "", nil
		}
		return "", fmt.Errorf("error reading crontab: %v\n%s", err, out)
	}
	return string(out), nil
}

// writeCrontab writes the given content to the user's crontab.
func writeCrontab(content string) error {
	cmd := exec.Command("crontab", "-")
	cmd.Stdin = strings.NewReader(content)
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("error writing crontab: %v\n%s", err, out)
	}
	return nil
}

// removeExistingGiterdoneJob removes any previously installed giterdone cron jobs.
func removeExistingGiterdoneJob(crontabContent string) string {
	var lines []string
	for _, line := range strings.Split(crontabContent, "\n") {
		if !strings.Contains(line, "# Giterdone backup job") {
			lines = append(lines, line)
		}
	}
	return strings.Join(lines, "\n")
}

// frequencyToCronSpec converts human-readable frequency to cron spec.
func frequencyToCronSpec(frequency string) (string, error) {
	switch strings.ToLower(frequency) {
	case "hourly":
		return "0 * * * *", nil
	case "daily":
		return "0 0 * * *", nil
	case "weekly":
		return "0 0 * * 0", nil // Every Sunday at midnight
	case "monthly":
		return "0 0 1 * *", nil // First day of every month at midnight
	case "every 5 minutes":
		return "*/5 * * * *", nil
	case "every 15 minutes":
		return "*/15 * * * *", nil
	case "every 30 minutes":
		return "*/30 * * * *", nil
	default:
		// Assume it's a custom cron spec if not recognized
		if utils.IsValidCronSpec(frequency) {
			return frequency, nil
		}
		return "", fmt.Errorf("unsupported frequency format: %s", frequency)
	}
}
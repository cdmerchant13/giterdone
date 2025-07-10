package utils

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"text/template"
	"time"
)

var verbose bool
var logFile *os.File

func SetVerbose(v bool) {
	verbose = v
}

func InitLogger(logPath string) error {
	if logPath == "" {
		return fmt.Errorf("log path cannot be empty")
	}

	logDir := filepath.Dir(logPath)
	if err := os.MkdirAll(logDir, 0755); err != nil {
		return fmt.Errorf("failed to create log directory: %w", err)
	}

	// Create or open the log file for appending
	file, err := os.OpenFile(filepath.Join(logPath, "giterdone.log"), os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
	if err != nil {
		return fmt.Errorf("failed to open log file: %w", err)
	}
	logFile = file

	// Set output to both console and file
	// This will make fmt.Println and fmt.Printf write to both
	// For now, we'll just write to the file explicitly in LogMessage
	return nil
}

func CloseLogger() {
	if logFile != nil {
		logFile.Close()
	}
}

func LogMessage(message string) {
	logEntry := fmt.Sprintf("[%s] %s\n", time.Now().Format("2006-01-02 15:04:05"), message)

	// Log to console if verbose mode is on
	if verbose {
		fmt.Print(logEntry)
	}

	// Log to file if logger is initialized
	if logFile != nil {
		if _, err := logFile.WriteString(logEntry); err != nil {
			fmt.Fprintf(os.Stderr, "Error writing to log file: %v\n", err)
		}
	}
}

func CheckError(err error) {
	if err != nil {
		LogMessage(fmt.Sprintf("Error: %v", err))
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func GenerateCommitMessage(tmplStr string, t time.Time) (string, error) {
	tmpl, err := template.New("commit").Parse(tmplStr)
	if err != nil {
		return "", fmt.Errorf("failed to parse commit message template: %w", err)
	}

	data := struct {
		Timestamp time.Time
	}{
		Timestamp: t,
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, data); err != nil {
		return "", fmt.Errorf("failed to execute commit message template: %w", err)
	}

	return buf.String(), nil
}

// IsValidCronSpec performs a basic validation of a cron spec string.
// This is a simplified check and might not cover all edge cases.
func IsValidCronSpec(spec string) bool {
	parts := strings.Fields(spec)
	return len(parts) == 5 || len(parts) == 6 // 5 or 6 parts for cron spec
}
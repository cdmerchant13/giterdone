package scanner

import (
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"strings"

	"giterdone/utils"
)

const ( // 100MB in bytes
	maxFileSize = 100 * 1024 * 1024
)

var ( // Common junk/system files to exclude
	excludePatterns = []string{
		".DS_Store",
		"Thumbs.db",
		"*.log",
		"*.tmp",
		"*.bak",
		"*.swp",
		"*.swo",
		"*.exe",
		"*.dll",
		"*.so",
		"*.dylib",
		"*.o",
		"*.obj",
		"*.pyc",
		"*.class",
		"*.jar",
		"*.war",
		"*.ear",
		"*.zip",
		"*.tar",
		"*.gz",
		"*.rar",
		"*.7z",
		"*.iso",
		"*.dmg",
		"*.bin",
		"*.dat",
		"*.db",
		"*.sqlite",
		"*.sql",
		"*.psd",
		"*.ai",
		"*.eps",
		"*.pdf",
		"*.doc",
		"*.docx",
		"*.xls",
		"*.xlsx",
		"*.ppt",
		"*.pptx",
		"*.odt",
		"*.ods",
		"*.odp",
		"*.mp3",
		"*.wav",
		"*.flac",
		"*.aac",
		"*.ogg",
		"*.wma",
		"*.mp4",
		"*.mkv",
		"*.avi",
		"*.mov",
		"*.wmv",
		"*.flv",
		"*.webm",
		"*.jpg",
		"*.jpeg",
		"*.png",
		"*.gif",
		"*.bmp",
		"*.tiff",
		"*.webp",
		"*.ico",
		"*.svg",
		"node_modules/",
		".git/",
		".vscode/",
		".idea/",
		"__pycache__/",
		"build/",
		"dist/",
		"bin/",
		"obj/",
		"pkg/",
		"vendor/",
		"tmp/",
		"temp/",
		"cache/",
		"logs/",
		"output/",
		"report/",
		"target/",
		"*.pid",
		"*.lock",
		"*.iml",
		"*.ipr",
		"*.iws",
	}
)

// ScanFiles scans the given paths and returns a list of files to include and a list of patterns to exclude.
func ScanFiles(includePaths []string) ([]string, []string) {
	var filesToInclude []string
	var patternsToExclude []string

	for _, p := range includePaths {
		info, err := os.Stat(p)
		if err != nil {
			utils.LogMessage(fmt.Sprintf("Warning: Path %s not found. Skipping.\n", p))
			continue
		}

		if info.IsDir() {
			filepath.Walk(p, func(path string, info os.FileInfo, err error) error {
				if err != nil {
					utils.LogMessage(fmt.Sprintf("Error walking path %s: %v\n", path, err))
					return nil // Continue walking
				}

				// Skip the root directory itself
				if path == p {
					return nil
				}

				// Check if directory matches any exclude pattern
				if info.IsDir() {
					for _, pattern := range excludePatterns {
						if strings.HasSuffix(pattern, "/") && strings.HasSuffix(path, pattern[:len(pattern)-1]) {
							patternsToExclude = append(patternsToExclude, filepath.Base(path)+"/")
							return filepath.SkipDir // Skip this directory
						}
					}
					return nil
				}

				// Check for file size
				if info.Size() > maxFileSize {
					utils.LogMessage(fmt.Sprintf("Warning: Skipping large file %s (%.2f MB)\n", path, float64(info.Size())/1024/1024))
					patternsToExclude = append(patternsToExclude, filepath.Base(path))
					return nil
				}

				// Check for binary/non-text files and known junk files
				shouldExclude := false
				for _, pattern := range excludePatterns {
					if !strings.HasSuffix(pattern, "/") && (strings.HasPrefix(filepath.Base(path), strings.TrimSuffix(pattern, "*")) || strings.HasSuffix(filepath.Base(path), strings.TrimPrefix(pattern, "*"))) {
						shouldExclude = true
						break
					}
				}

				if shouldExclude {
					utils.LogMessage(fmt.Sprintf("Info: Skipping excluded file %s\n", path))
					patternsToExclude = append(patternsToExclude, filepath.Base(path))
					return nil
				}

				filesToInclude = append(filesToInclude, path)
				return nil
			})
		} else { // It's a file
			// Check for file size
			if info.Size() > maxFileSize {
				utils.LogMessage(fmt.Sprintf("Warning: Skipping large file %s (%.2f MB)\n", p, float64(info.Size())/1024/1024))
				patternsToExclude = append(patternsToExclude, filepath.Base(p))
				continue
			}

			// Check for binary/non-text files and known junk files
			shouldExclude := false
			for _, pattern := range excludePatterns {
				if !strings.HasSuffix(pattern, "/") && (strings.HasPrefix(filepath.Base(p), strings.TrimSuffix(pattern, "*")) || strings.HasSuffix(filepath.Base(p), strings.TrimPrefix(pattern, "*"))) {
					shouldExclude = true
					break
				}
			}

			if shouldExclude {
				utils.LogMessage(fmt.Sprintf("Info: Skipping excluded file %s\n", p))
				patternsToExclude = append(patternsToExclude, filepath.Base(p))
				continue
			}
			filesToInclude = append(filesToInclude, p)
		}
	}

	return filesToInclude, patternsToExclude
}

// GenerateGitignoreContent creates the content for a .gitignore file
func GenerateGitignoreContent(excludedPatterns []string) string {
	var sb strings.Builder
	sb.WriteString("# Giterdone generated .gitignore\n")
	sb.WriteString("# Files and directories automatically excluded by Giterdone\n\n")

	for _, pattern := range excludedPatterns {
		sb.WriteString(pattern + "\n")
	}

	return sb.String()
}

// WriteGitignoreFile writes the .gitignore content to the specified path
func WriteGitignoreFile(repoPath, content string) error {
	gitignorePath := filepath.Join(repoPath, ".gitignore")
	return ioutil.WriteFile(gitignorePath, []byte(content), 0644)
}
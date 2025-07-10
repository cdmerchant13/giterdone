use ignore::{WalkBuilder, DirEntry};
use std::path::{Path, PathBuf};
use std::fs;

const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024; // 100MB

pub fn scan(paths: &[PathBuf]) -> (Vec<(PathBuf, PathBuf)>, String) {
    let mut files_to_backup = Vec::new();
    let mut gitignore_patterns = Vec::new();

    let single_root_mode = paths.len() == 1 && paths[0].is_dir();

    for base_path in paths {
        let walker = WalkBuilder::new(base_path)
            .standard_filters(true) // Respect .gitignore, .ignore, etc.
            .build();

        for result in walker {
            match result {
                Ok(entry) => {
                    if should_backup(&entry) {
                        let relative_path = if single_root_mode {
                            entry.path().strip_prefix(base_path).unwrap_or(entry.path()).to_path_buf()
                        } else {
                            entry.path().strip_prefix(base_path.parent().unwrap_or(base_path)).unwrap_or(entry.path()).to_path_buf()
                        };
                        files_to_backup.push((entry.path().to_path_buf(), relative_path));
                    } else if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        // Add to .gitignore if it's a file that should be ignored
                        if let Some(pattern) = path_to_gitignore_pattern(entry.path(), base_path) {
                            gitignore_patterns.push(pattern);
                        }
                    }
                }
                Err(err) => eprintln!("ERROR: {}", err),
            }
        }
    }

    (files_to_backup, gitignore_patterns.join("\n"))
}

fn should_backup(entry: &DirEntry) -> bool {
    if entry.file_type().map_or(true, |ft| ft.is_dir()) {
        return false; // Don't include directories themselves, only files
    }

    // Size check
    if let Ok(metadata) = entry.metadata() {
        if metadata.len() > MAX_FILE_SIZE {
            return false;
        }
    }

    // Binary file check (simple version)
    if is_binary(entry.path()) {
        return false;
    }

    // Junk file check
    if is_junk(entry.path()) {
        return false;
    }

    true
}

fn is_binary(path: &Path) -> bool {
    if let Ok(content) = fs::read(path) {
        // A simple heuristic: check for a significant number of non-UTF8 bytes.
        // This is not foolproof but good enough for a first pass.
        let text_likelihood = content.iter().filter(|&&b| b < 128).count() as f64 / content.len() as f64;
        if content.contains(&0) || text_likelihood < 0.8 {
            return true; // Likely binary
        }
    }
    false
}

fn is_junk(path: &Path) -> bool {
    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    matches!(file_name, ".DS_Store" | "Thumbs.db") || file_name.ends_with(".log")
}

fn path_to_gitignore_pattern(full_path: &Path, base_path: &Path) -> Option<String> {
    if let Ok(relative_path) = full_path.strip_prefix(base_path) {
        // Make it a relative path from the root of the backup set
        Some(format!("/{}", relative_path.display()))
    } else {
        // If it's not inside the base_path for some reason, ignore it absolutely
        Some(format!("/{}", full_path.display()))
    }
}
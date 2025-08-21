use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub full_path: String,
    pub is_directory: bool,
}

pub struct AutocompleteEngine {
    working_dir: String,
}

impl AutocompleteEngine {
    pub fn new(working_dir: String) -> Self {
        Self { working_dir }
    }

    pub fn get_file_suggestions(&self, partial_path: &str) -> Vec<FileEntry> {
        if partial_path.is_empty() {
            return self.list_directory(".");
        }

        let path = Path::new(partial_path);
        let (dir_path, file_prefix) = if partial_path.ends_with('/') {
            // Remove trailing slash to avoid double slashes in path construction
            (partial_path.trim_end_matches('/').to_string(), String::new())
        } else {
            match path.parent() {
                Some(parent) => {
                    let parent_str = parent.to_string_lossy().to_string();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    (parent_str, file_name)
                }
                None => (".".to_string(), partial_path.to_string()),
            }
        };

        let dir_path_str = if dir_path.is_empty() { "." } else { &dir_path };
        let entries = self.list_directory(dir_path_str);
        
        // Filter entries based on file prefix
        entries
            .into_iter()
            .filter(|entry| entry.name.starts_with(&file_prefix))
            .collect()
    }

    fn list_directory(&self, relative_path: &str) -> Vec<FileEntry> {
        let full_path = Path::new(&self.working_dir).join(relative_path);
        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(&full_path) {
            for entry in dir_entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    
                    // Skip hidden files unless specifically requested
                    if name.starts_with('.') && !relative_path.contains("/.") {
                        continue;
                    }

                    let entry_path = if relative_path == "." {
                        name.clone()
                    } else {
                        // Remove trailing slash from relative_path if it exists to avoid double slashes
                        let clean_relative_path = relative_path.trim_end_matches('/');
                        format!("{}/{}", clean_relative_path, name)
                    };

                    entries.push(FileEntry {
                        name,
                        full_path: entry_path,
                        is_directory: metadata.is_dir(),
                    });
                }
            }
        }

        // Sort: directories first, then files, both alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        entries
    }
}


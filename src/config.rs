use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;
use std::io::{self, BufRead, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub files_to_copy: Vec<String>,
    pub directories_to_copy: Vec<String>,
    pub claude_files: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            files_to_copy: vec![
                ".envrc".to_string(),
            ],
            directories_to_copy: vec![],
            claude_files: vec![
                "settings.json".to_string(),
                "settings.local.json".to_string(),
            ],
        }
    }
}

impl Config {
    pub fn load_from_file(repo_dir: &Path) -> io::Result<Self> {
        let mut config = Self::default();
        let workbloom_file = repo_dir.join(".workbloom");
        
        if workbloom_file.exists() {
            let file = fs::File::open(&workbloom_file)?;
            let reader = BufReader::new(file);
            
            for line in reader.lines() {
                let line = line?;
                let trimmed = line.trim();
                
                // Skip empty lines and comments
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                
                // Check if it's a directory (ends with /)
                if trimmed.ends_with('/') {
                    config.directories_to_copy.push(trimmed.trim_end_matches('/').to_string());
                } else {
                    config.files_to_copy.push(trimmed.to_string());
                }
            }
        }
        
        Ok(config)
    }
}
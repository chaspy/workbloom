use serde::{Deserialize, Serialize};

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
                ".env".to_string(),
            ],
            directories_to_copy: vec![
                "backend/service-account.json".to_string(),
            ],
            claude_files: vec![
                "settings.json".to_string(),
                "settings.local.json".to_string(),
            ],
        }
    }
}
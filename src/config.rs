use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub rules: HashMap<String, String>,
    pub ignore: IgnoreConfig,
    pub ci: CiConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IgnoreConfig {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CiConfig {
    pub fail_below: u32,
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        IgnoreConfig {
            files: vec!["convex/_generated/**".to_string()],
        }
    }
}

impl Config {
    pub fn load(project_root: &Path) -> Result<Self, String> {
        let config_path = project_root.join("convex-doctor.toml");
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config: {e}"))?;
        let config: Config =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {e}"))?;
        Ok(config)
    }

    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        !matches!(self.rules.get(rule_id), Some(v) if v == "off")
    }

    pub fn is_file_ignored(&self, file_path: &Path) -> bool {
        let path_str = file_path.to_string_lossy();
        for pattern in &self.ignore.files {
            if let Ok(glob) = glob::Pattern::new(pattern) {
                if glob.matches(&path_str) {
                    return true;
                }
            }
        }
        false
    }
}

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

    pub fn is_file_ignored(&self, project_root: &Path, file_path: &Path) -> bool {
        let absolute = file_path.to_string_lossy().replace('\\', "/");
        let relative = file_path
            .strip_prefix(project_root)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| absolute.clone());
        let relative_with_dot = format!("./{relative}");
        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

        for pattern in &self.ignore.files {
            for candidate in Self::glob_candidates(pattern) {
                if let Ok(glob) = glob::Pattern::new(&candidate) {
                    if glob.matches(&relative)
                        || glob.matches(&relative_with_dot)
                        || glob.matches(&absolute)
                    {
                        return true;
                    }
                }
            }

            let normalized = pattern.replace('\\', "/").trim().to_string();
            if !normalized.contains('/') && !file_name.is_empty() {
                if let Ok(basename_glob) = glob::Pattern::new(&normalized) {
                    if basename_glob.matches(file_name) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn glob_candidates(pattern: &str) -> Vec<String> {
        let normalized = pattern.replace('\\', "/").trim().to_string();
        if normalized.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        let mut push = |candidate: String| {
            if !candidate.is_empty() && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        };

        push(normalized.clone());

        if normalized.starts_with("./") {
            push(normalized.trim_start_matches("./").to_string());
        }

        if normalized.starts_with('/') {
            push(normalized.trim_start_matches('/').to_string());
        }

        if normalized.ends_with('/') {
            let trimmed = normalized.trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                push(trimmed.clone());
                push(format!("{trimmed}/**"));
            }
        }

        let has_glob_meta = normalized.contains('*')
            || normalized.contains('?')
            || normalized.contains('[')
            || normalized.contains(']');
        if !normalized.ends_with('/') && normalized.contains('/') && !has_glob_meta {
            push(format!("{normalized}/**"));
        }

        if !normalized.contains('/') {
            push(format!("**/{normalized}"));
        }

        candidates
    }
}

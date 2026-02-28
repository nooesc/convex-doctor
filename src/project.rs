use std::path::{Path, PathBuf};

use crate::config::Config;

#[derive(Debug)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub convex_dir: PathBuf,
    pub has_schema: bool,
    pub has_auth_config: bool,
    pub has_convex_json: bool,
    pub convex_version: Option<String>,
    pub framework: Option<String>,
}

impl ProjectInfo {
    pub fn detect(root: &Path) -> Result<Self, String> {
        let convex_dir = root.join("convex");
        if !convex_dir.is_dir() {
            return Err(format!("No convex/ directory found in {}", root.display()));
        }

        let has_schema = SCHEMA_FILENAMES
            .iter()
            .any(|file| convex_dir.join(file).exists());
        let has_auth_config = convex_dir.join("auth.config.ts").exists()
            || convex_dir.join("auth.config.js").exists();
        let has_convex_json = root.join("convex.json").exists();

        let (convex_version, framework) = Self::parse_package_json(root);

        Ok(ProjectInfo {
            root: root.to_path_buf(),
            convex_dir,
            has_schema,
            has_auth_config,
            has_convex_json,
            convex_version,
            framework,
        })
    }

    fn parse_package_json(root: &Path) -> (Option<String>, Option<String>) {
        let pkg_path = root.join("package.json");
        let contents = match std::fs::read_to_string(&pkg_path) {
            Ok(c) => c,
            Err(_) => return (None, None),
        };
        let json: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(_) => return (None, None),
        };

        let deps = json.get("dependencies").and_then(|d| d.as_object());
        let dev_deps = json.get("devDependencies").and_then(|d| d.as_object());

        let convex_version = deps
            .and_then(|d| d.get("convex"))
            .or_else(|| dev_deps.and_then(|d| d.get("convex")))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let framework = if Self::has_dep(deps, dev_deps, "next") {
            Some("nextjs".to_string())
        } else if Self::has_dep(deps, dev_deps, "vite") {
            Some("vite".to_string())
        } else if Self::has_dep(deps, dev_deps, "@remix-run/node") {
            Some("remix".to_string())
        } else {
            None
        };

        (convex_version, framework)
    }

    fn has_dep(
        deps: Option<&serde_json::Map<String, serde_json::Value>>,
        dev_deps: Option<&serde_json::Map<String, serde_json::Value>>,
        name: &str,
    ) -> bool {
        deps.is_some_and(|d| d.contains_key(name)) || dev_deps.is_some_and(|d| d.contains_key(name))
    }

    pub fn discover_files(&self, config: &Config) -> Vec<PathBuf> {
        let mut files = Vec::new();
        Self::walk_dir(&self.root, &self.convex_dir, config, &mut files);
        files.sort();
        files
    }

    fn walk_dir(project_root: &Path, dir: &Path, config: &Config, files: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "_generated") {
                    continue;
                }
                Self::walk_dir(project_root, &path, config, files);
            } else if let Some(ext) = path.extension() {
                if is_supported_source_file(ext) && !config.is_file_ignored(project_root, &path) {
                    files.push(path);
                }
            }
        }
    }
}

fn is_supported_source_file(ext: &std::ffi::OsStr) -> bool {
    matches!(
        ext.to_str(),
        Some("ts" | "tsx" | "js" | "jsx" | "mts" | "cts" | "mjs" | "cjs")
    )
}

const SCHEMA_FILENAMES: &[&str] = &[
    "schema.ts",
    "schema.js",
    "schema.mts",
    "schema.cts",
    "schema.mjs",
    "schema.cjs",
];

use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    pub variables: HashMap<String, String>,
    pub working_directory: PathBuf,
    pub home_directory: PathBuf,
    pub path_separator: String,
}

impl EnvironmentManager {
    pub fn new() -> Self {
        let mut variables = HashMap::new();
        
        // Copy current environment variables
        for (key, value) in env::vars() {
            variables.insert(key, value);
        }

        // Set terminal-specific environment variables
        variables.insert("TERM".to_string(), "xterm-256color".to_string());
        variables.insert("COLORTERM".to_string(), "truecolor".to_string());
        variables.insert("TERM_PROGRAM".to_string(), "rt".to_string());
        variables.insert("TERM_PROGRAM_VERSION".to_string(), env!("CARGO_PKG_VERSION").to_string());

        let working_directory = env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let home_directory = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        
        let path_separator = if cfg!(windows) {
            ";".to_string()
        } else {
            ":".to_string()
        };

        Self {
            variables,
            working_directory,
            home_directory,
            path_separator,
        }
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    pub fn remove_variable(&mut self, key: &str) -> Option<String> {
        self.variables.remove(key)
    }

    pub fn set_working_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), std::io::Error> {
        let path = path.as_ref().to_path_buf();
        if path.exists() && path.is_dir() {
            self.working_directory = path.canonicalize()?;
            self.variables.insert("PWD".to_string(), self.working_directory.to_string_lossy().to_string());
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Directory does not exist or is not a directory",
            ))
        }
    }

    pub fn get_working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn expand_path(&self, path: &str) -> PathBuf {
        let path = if path.starts_with("~/") {
            self.home_directory.join(&path[2..])
        } else if path == "~" {
            self.home_directory.clone()
        } else if path.starts_with("$HOME/") || path.starts_with("${HOME}/") {
            let start = if path.starts_with("$HOME/") { 6 } else { 8 };
            self.home_directory.join(&path[start..])
        } else if path.starts_with("$") {
            // Handle other environment variable expansions
            let mut expanded = String::new();
            let mut chars = path.chars().peekable();
            
            while let Some(ch) = chars.next() {
                if ch == '$' {
                    if chars.peek() == Some(&'{') {
                        chars.next(); // consume '{'
                        let mut var_name = String::new();
                        while let Some(ch) = chars.next() {
                            if ch == '}' {
                                break;
                            }
                            var_name.push(ch);
                        }
                        if let Some(value) = self.get_variable(&var_name) {
                            expanded.push_str(value);
                        }
                    } else {
                        let mut var_name = String::new();
                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphanumeric() || ch == '_' {
                                var_name.push(chars.next().unwrap());
                            } else {
                                break;
                            }
                        }
                        if let Some(value) = self.get_variable(&var_name) {
                            expanded.push_str(value);
                        }
                    }
                } else {
                    expanded.push(ch);
                }
            }
            PathBuf::from(expanded)
        } else {
            PathBuf::from(path)
        };

        if path.is_relative() {
            self.working_directory.join(path)
        } else {
            path
        }
    }

    pub fn get_path_entries(&self) -> Vec<PathBuf> {
        if let Some(path_var) = self.get_variable("PATH") {
            path_var
                .split(&self.path_separator)
                .map(|p| self.expand_path(p))
                .filter(|p| p.exists() && p.is_dir())
                .collect()
        } else {
            vec![]
        }
    }

    pub fn add_to_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();
        if let Some(current_path) = self.variables.get("PATH").cloned() {
            let new_path = format!("{}{}{}", path.to_string_lossy(), self.path_separator, current_path);
            self.variables.insert("PATH".to_string(), new_path);
        } else {
            self.variables.insert("PATH".to_string(), path.to_string_lossy().to_string());
        }
    }

    pub fn remove_from_path<P: AsRef<Path>>(&mut self, path: P) {
        let path_to_remove = path.as_ref().to_string_lossy();
        if let Some(current_path) = self.variables.get("PATH").cloned() {
            let new_paths: Vec<&str> = current_path
                .split(&self.path_separator)
                .filter(|p| *p != path_to_remove)
                .collect();
            let new_path = new_paths.join(&self.path_separator);
            self.variables.insert("PATH".to_string(), new_path);
        }
    }

    pub fn find_executable(&self, name: &str) -> Option<PathBuf> {
        if name.contains('/') || (cfg!(windows) && name.contains('\\')) {
            // Absolute or relative path
            let path = self.expand_path(name);
            if path.exists() && self.is_executable(&path) {
                return Some(path);
            }
        } else {
            // Search in PATH
            for path_dir in self.get_path_entries() {
                let executable_path = if cfg!(windows) {
                    // On Windows, try common executable extensions
                    for ext in &["", ".exe", ".bat", ".cmd", ".com"] {
                        let candidate = path_dir.join(format!("{}{}", name, ext));
                        if candidate.exists() && self.is_executable(&candidate) {
                            return Some(candidate);
                        }
                    }
                    continue;
                } else {
                    path_dir.join(name)
                };

                if executable_path.exists() && self.is_executable(&executable_path) {
                    return Some(executable_path);
                }
            }
        }
        None
    }

    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = path.metadata() {
                let permissions = metadata.permissions();
                // Check if owner has execute permission
                permissions.mode() & 0o100 != 0
            } else {
                false
            }
        }

        #[cfg(windows)]
        {
            // On Windows, if the file exists and has an executable extension, consider it executable
            if let Some(ext) = path.extension() {
                matches!(ext.to_string_lossy().to_lowercase().as_str(), "exe" | "bat" | "cmd" | "com")
            } else {
                false
            }
        }
    }

    pub fn export_for_subprocess(&self) -> HashMap<String, String> {
        self.variables.clone()
    }

    pub fn merge_from_subprocess(&mut self, env_vars: HashMap<String, String>) {
        for (key, value) in env_vars {
            self.variables.insert(key, value);
        }
    }

    pub fn get_shell(&self) -> String {
        if cfg!(windows) {
            self.get_variable("COMSPEC")
                .unwrap_or(&"cmd.exe".to_string())
                .clone()
        } else {
            self.get_variable("SHELL")
                .unwrap_or(&"/bin/bash".to_string())
                .clone()
        }
    }

    pub fn get_user(&self) -> Option<String> {
        self.get_variable("USER")
            .or_else(|| self.get_variable("USERNAME"))
            .cloned()
    }

    pub fn get_hostname(&self) -> Option<String> {
        self.get_variable("HOSTNAME")
            .or_else(|| self.get_variable("COMPUTERNAME"))
            .cloned()
    }

    pub fn list_variables(&self) -> Vec<(&String, &String)> {
        self.variables.iter().collect()
    }

    pub fn clear_variables(&mut self) {
        self.variables.clear();
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let value = line[eq_pos + 1..].trim();
                
                // Remove quotes if present
                let value = if (value.starts_with('"') && value.ends_with('"')) ||
                             (value.starts_with('\'') && value.ends_with('\'')) {
                    &value[1..value.len() - 1]
                } else {
                    value
                };
                
                self.variables.insert(key, value.to_string());
            }
        }
        Ok(())
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let mut content = String::new();
        let mut sorted_vars: Vec<_> = self.variables.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| *k);
        
        for (key, value) in sorted_vars {
            // Quote values that contain spaces or special characters
            if value.contains(' ') || value.contains('\t') || value.contains('\n') || value.contains('"') {
                content.push_str(&format!("{}=\"{}\"\n", key, value.replace('"', "\\\"")));
            } else {
                content.push_str(&format!("{}={}\n", key, value));
            }
        }
        
        std::fs::write(path, content)
    }
}

impl Default for EnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_environment_manager_creation() {
        let env_mgr = EnvironmentManager::new();
        assert!(env_mgr.get_variable("TERM").is_some());
        assert!(env_mgr.get_variable("COLORTERM").is_some());
    }

    #[test]
    fn test_set_get_variable() {
        let mut env_mgr = EnvironmentManager::new();
        env_mgr.set_variable("TEST_VAR".to_string(), "test_value".to_string());
        assert_eq!(env_mgr.get_variable("TEST_VAR"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_path_expansion() {
        let env_mgr = EnvironmentManager::new();
        let expanded = env_mgr.expand_path("~/test");
        assert!(expanded.starts_with(&env_mgr.home_directory));
    }

    #[test]
    fn test_working_directory() {
        let mut env_mgr = EnvironmentManager::new();
        let temp_dir = tempdir().unwrap();
        
        env_mgr.set_working_directory(temp_dir.path()).unwrap();
        assert_eq!(env_mgr.get_working_directory(), temp_dir.path().canonicalize().unwrap());
    }

    #[test]
    fn test_path_manipulation() {
        let mut env_mgr = EnvironmentManager::new();
        let test_path = PathBuf::from("/test/path");
        
        env_mgr.add_to_path(&test_path);
        let path_var = env_mgr.get_variable("PATH").unwrap();
        assert!(path_var.contains("/test/path"));
        
        env_mgr.remove_from_path(&test_path);
        let path_var = env_mgr.get_variable("PATH").unwrap();
        assert!(!path_var.contains("/test/path"));
    }

    #[test]
    fn test_env_file_operations() {
        let mut env_mgr = EnvironmentManager::new();
        let temp_dir = tempdir().unwrap();
        let env_file = temp_dir.path().join("test.env");
        
        env_mgr.set_variable("TEST_KEY".to_string(), "test_value".to_string());
        env_mgr.save_to_file(&env_file).unwrap();
        
        let mut new_env_mgr = EnvironmentManager::new();
        new_env_mgr.load_from_file(&env_file).unwrap();
        assert_eq!(new_env_mgr.get_variable("TEST_KEY"), Some(&"test_value".to_string()));
    }
}
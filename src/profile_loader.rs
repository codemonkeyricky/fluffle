use crate::agent_profile::AgentProfile;
use crate::error::{Error, Result};
use lazy_static::lazy_static;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

lazy_static! {
    static ref PROFILE_REGISTRY: RwLock<HashMap<String, AgentProfile>> =
        RwLock::new(HashMap::new());
}

/// Load all profiles from built-in and user directories
pub fn load_profiles() -> Result<()> {
    let mut registry = PROFILE_REGISTRY.write().map_err(|e| {
        Error::Agent(format!(
            "Failed to acquire write lock on profile registry: {}",
            e
        ))
    })?;

    // Clear existing registry (in case of reload)
    registry.clear();

    // Load built-in profiles
    let builtin_dir = builtin_profiles_dir()?;
    if builtin_dir.exists() {
        load_profiles_from_dir(&builtin_dir, &mut registry)?;
    }

    // Load user profiles (override built-in ones)
    let user_dir = user_profiles_dir()?;
    if user_dir.exists() {
        load_profiles_from_dir(&user_dir, &mut registry)?;
    }

    Ok(())
}

/// Get a profile by name, returns None if not found
pub fn get_profile(name: &str) -> Option<AgentProfile> {
    let registry = PROFILE_REGISTRY.read().ok()?;
    registry.get(name).cloned()
}

/// Check if a profile exists
pub fn has_profile(name: &str) -> bool {
    let registry = PROFILE_REGISTRY.read().unwrap();
    registry.contains_key(name)
}

/// Get all profile names
pub fn profile_names() -> Vec<String> {
    let registry = PROFILE_REGISTRY.read().unwrap();
    registry.keys().cloned().collect()
}

#[cfg(test)]
pub(crate) fn clear_profiles() {
    let mut registry = PROFILE_REGISTRY.write().unwrap();
    registry.clear();
}

/// Get built-in profiles directory (relative to source)
fn builtin_profiles_dir() -> Result<PathBuf> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src");
    path.push("agent_profiles");
    Ok(path)
}

/// Get user profiles directory (~/.config/nanocode/agents/)
fn user_profiles_dir() -> Result<PathBuf> {
    let mut path = dirs::config_dir()
        .ok_or_else(|| Error::ConfigLoad("Could not find config directory".to_string()))?;
    path.push("nanocode");
    path.push("agents");
    Ok(path)
}

/// Load all JSON files from a directory into the registry
fn load_profiles_from_dir(dir: &Path, registry: &mut HashMap<String, AgentProfile>) -> Result<()> {
    for entry in fs::read_dir(dir)
        .map_err(|e| Error::Agent(format!("Failed to read directory {}: {}", dir.display(), e)))?
    {
        let entry =
            entry.map_err(|e| Error::Agent(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.extension().map(|ext| ext == "json").unwrap_or(false) {
            match load_profile_from_file(&path) {
                Ok(profile) => {
                    let name = profile.name.clone();
                    registry.insert(name, profile);
                }
                Err(e) => {
                    tracing::warn!("Failed to load profile from {}: {}", path.display(), e);
                }
            }
        }
    }
    Ok(())
}

/// Load a single profile from a JSON file
fn load_profile_from_file(path: &Path) -> Result<AgentProfile> {
    let content = fs::read_to_string(path).map_err(|e| {
        Error::Agent(format!(
            "Failed to read profile file {}: {}",
            path.display(),
            e
        ))
    })?;
    let profile: AgentProfile = serde_json::from_str(&content).map_err(|e| {
        Error::Agent(format!(
            "Failed to parse profile JSON {}: {}",
            path.display(),
            e
        ))
    })?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_builtin_profiles() {
        clear_profiles();
        load_profiles().expect("Failed to load profiles");
        let names = profile_names();
        assert!(names.contains(&"generalist".to_string()));
        assert!(names.contains(&"explorer".to_string()));
        assert!(names.contains(&"security-auditor".to_string()));

        let generalist = get_profile("generalist").expect("generalist profile missing");
        assert_eq!(generalist.name, "generalist");
        assert!(!generalist.system_prompt.is_empty());
        assert!(!generalist.tools.is_empty());

        let explorer = get_profile("explorer").expect("explorer profile missing");
        assert_eq!(explorer.name, "explorer");
        assert!(explorer.system_prompt.contains("codebase explorer"));
        assert_eq!(explorer.config_overrides.len(), 2);

        let security = get_profile("security-auditor").expect("security-auditor profile missing");
        assert_eq!(security.name, "security-auditor");
        assert!(security.system_prompt.contains("security expert"));
        assert_eq!(security.config_overrides.len(), 1);
    }
}

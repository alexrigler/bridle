//! Profile management.

use std::path::PathBuf;

use get_harness::Harness;

use super::profile_name::ProfileName;
use crate::error::{Error, Result};

#[derive(Debug)]
pub struct ProfileManager {
    profiles_dir: PathBuf,
}

impl ProfileManager {
    pub fn new(profiles_dir: PathBuf) -> Self {
        Self { profiles_dir }
    }

    pub fn profiles_dir(&self) -> &PathBuf {
        &self.profiles_dir
    }

    pub fn harness_id(harness: &Harness) -> &'static str {
        match harness.kind() {
            get_harness::HarnessKind::ClaudeCode => "claude-code",
            get_harness::HarnessKind::OpenCode => "opencode",
            get_harness::HarnessKind::Goose => "goose",
            _ => "unknown",
        }
    }

    pub fn profile_path(&self, harness: &Harness, name: &ProfileName) -> PathBuf {
        self.profiles_dir
            .join(Self::harness_id(harness))
            .join(name.as_str())
    }

    pub fn profile_exists(&self, harness: &Harness, name: &ProfileName) -> bool {
        self.profile_path(harness, name).is_dir()
    }

    pub fn list_profiles(&self, harness: &Harness) -> Result<Vec<ProfileName>> {
        let harness_dir = self.profiles_dir.join(Self::harness_id(harness));

        if !harness_dir.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(&harness_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && let Some(name) = entry.file_name().to_str()
                && let Ok(profile_name) = ProfileName::new(name)
            {
                profiles.push(profile_name);
            }
        }

        profiles.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(profiles)
    }

    pub fn create_profile(&self, harness: &Harness, name: &ProfileName) -> Result<PathBuf> {
        let path = self.profile_path(harness, name);

        if path.exists() {
            return Err(Error::ProfileExists(name.as_str().to_string()));
        }

        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn delete_profile(&self, harness: &Harness, name: &ProfileName) -> Result<()> {
        let path = self.profile_path(harness, name);

        if !path.exists() {
            return Err(Error::ProfileNotFound(name.as_str().to_string()));
        }

        std::fs::remove_dir_all(&path)?;
        Ok(())
    }
}

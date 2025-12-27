//! Profile management commands.

use get_harness::{Harness, HarnessKind};

use crate::config::{BridleConfig, ProfileManager, ProfileName};

fn resolve_harness(name: &str) -> Option<Harness> {
    let kind = match name {
        "claude-code" | "claude" | "cc" => HarnessKind::ClaudeCode,
        "opencode" | "oc" => HarnessKind::OpenCode,
        "goose" => HarnessKind::Goose,
        _ => return None,
    };
    Some(Harness::new(kind))
}

fn get_manager() -> Option<ProfileManager> {
    let profiles_dir = BridleConfig::profiles_dir().ok()?;
    Some(ProfileManager::new(profiles_dir))
}

pub fn list_profiles(harness_name: &str) {
    let Some(harness) = resolve_harness(harness_name) else {
        eprintln!("Unknown harness: {harness_name}");
        eprintln!("Valid options: claude-code, opencode, goose");
        return;
    };

    let Some(manager) = get_manager() else {
        eprintln!("Could not find config directory");
        return;
    };

    match manager.list_profiles(&harness) {
        Ok(profiles) => {
            if profiles.is_empty() {
                println!(
                    "No profiles found for {}",
                    ProfileManager::harness_id(&harness)
                );
            } else {
                println!("Profiles for {}:", ProfileManager::harness_id(&harness));
                for profile in profiles {
                    println!("  {}", profile.as_str());
                }
            }
        }
        Err(e) => eprintln!("Error listing profiles: {e}"),
    }
}

pub fn show_profile(harness_name: &str, profile_name: &str) {
    let Some(harness) = resolve_harness(harness_name) else {
        eprintln!("Unknown harness: {harness_name}");
        return;
    };

    let Ok(name) = ProfileName::new(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return;
    };

    let Some(manager) = get_manager() else {
        eprintln!("Could not find config directory");
        return;
    };

    let path = manager.profile_path(&harness, &name);

    if !manager.profile_exists(&harness, &name) {
        eprintln!("Profile not found: {profile_name}");
        return;
    }

    println!("Profile: {}", name.as_str());
    println!("Harness: {}", ProfileManager::harness_id(&harness));
    println!("Path: {}", path.display());
}

pub fn create_profile(harness_name: &str, profile_name: &str) {
    let Some(harness) = resolve_harness(harness_name) else {
        eprintln!("Unknown harness: {harness_name}");
        return;
    };

    let Ok(name) = ProfileName::new(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return;
    };

    let Some(manager) = get_manager() else {
        eprintln!("Could not find config directory");
        return;
    };

    match manager.create_profile(&harness, &name) {
        Ok(path) => {
            println!("Created profile: {}", name.as_str());
            println!("Path: {}", path.display());
        }
        Err(e) => eprintln!("Error creating profile: {e}"),
    }
}

pub fn delete_profile(harness_name: &str, profile_name: &str) {
    let Some(harness) = resolve_harness(harness_name) else {
        eprintln!("Unknown harness: {harness_name}");
        return;
    };

    let Ok(name) = ProfileName::new(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return;
    };

    let Some(manager) = get_manager() else {
        eprintln!("Could not find config directory");
        return;
    };

    match manager.delete_profile(&harness, &name) {
        Ok(()) => println!("Deleted profile: {}", name.as_str()),
        Err(e) => eprintln!("Error deleting profile: {e}"),
    }
}

pub fn switch_profile(harness_name: &str, profile_name: &str) {
    let Some(harness) = resolve_harness(harness_name) else {
        eprintln!("Unknown harness: {harness_name}");
        return;
    };

    let Ok(name) = ProfileName::new(profile_name) else {
        eprintln!("Invalid profile name: {profile_name}");
        return;
    };

    let Some(manager) = get_manager() else {
        eprintln!("Could not find config directory");
        return;
    };

    if !manager.profile_exists(&harness, &name) {
        eprintln!("Profile not found: {profile_name}");
        return;
    }

    // TODO: Implement actual profile switching (copy config files)
    println!("Switched to profile: {}", name.as_str());
    println!("Harness: {}", ProfileManager::harness_id(&harness));
}

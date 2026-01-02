//! CLI install command implementation.

use std::io::IsTerminal;

use color_eyre::eyre::{eyre, Result};
use dialoguer::theme::ColorfulTheme;
use dialoguer::MultiSelect;

use crate::config::{BridleConfig, ProfileManager};
use crate::harness::HarnessConfig;
use crate::install::discovery::{discover_skills, DiscoveryError};
use crate::install::installer::install_skills;
use crate::install::{InstallOptions, InstallTarget};

pub fn run(source: &str, force: bool) -> Result<()> {
    if !std::io::stdin().is_terminal() {
        return Err(eyre!(
            "Interactive mode requires a terminal. Use --help for non-interactive options."
        ));
    }

    let url = normalize_source(source);

    eprintln!("Discovering skills from {}...", url);

    let discovery = discover_skills(&url).map_err(|e| match e {
        DiscoveryError::InvalidUrl(msg) => eyre!("Invalid URL: {}", msg),
        DiscoveryError::FetchError(e) => eyre!("Failed to fetch repository: {}", e),
        DiscoveryError::NoSkillsFound => eyre!("No skills found in repository"),
    })?;

    if discovery.skills.is_empty() && discovery.mcp_servers.is_empty() {
        eprintln!("No skills or MCP servers found in {}", url);
        return Ok(());
    }

    eprintln!(
        "Found {} skill(s) and {} MCP server(s) from {}/{}",
        discovery.skills.len(),
        discovery.mcp_servers.len(),
        discovery.source.owner,
        discovery.source.repo
    );

    let skill_names: Vec<&str> = discovery.skills.iter().map(|s| s.name.as_str()).collect();

    let Some(selected_indices) = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select skills to install (Esc to cancel)")
        .items(&skill_names)
        .defaults(&vec![true; skill_names.len()])
        .interact_opt()?
    else {
        eprintln!("Cancelled");
        return Ok(());
    };

    if selected_indices.is_empty() {
        eprintln!("No skills selected");
        return Ok(());
    }

    let selected_skills: Vec<_> = selected_indices
        .iter()
        .map(|&i| discovery.skills[i].clone())
        .collect();

    let targets = select_targets()?;

    if targets.is_empty() {
        eprintln!("No targets selected");
        return Ok(());
    }

    let options = InstallOptions { force };

    for target in &targets {
        eprintln!("\nInstalling to {}/{}...", target.harness, target.profile);

        let report = install_skills(&selected_skills, target, &options);

        for success in &report.installed {
            eprintln!("  + Installed: {}", success.skill);
        }

        for skip in &report.skipped {
            eprintln!("  = Skipped: {} (already exists)", skip.skill);
        }

        for error in &report.errors {
            eprintln!("  ! Error installing {}: {}", error.skill, error.error);
        }
    }

    eprintln!("\nDone!");
    Ok(())
}

fn normalize_source(source: &str) -> String {
    if source.starts_with("http://") || source.starts_with("https://") {
        source.to_string()
    } else if source.contains('/') && !source.contains(':') {
        format!("https://github.com/{}", source)
    } else {
        source.to_string()
    }
}

fn select_targets() -> Result<Vec<InstallTarget>> {
    use harness_locate::{Harness, HarnessKind};

    let config = BridleConfig::load()?;
    let profiles_dir = BridleConfig::profiles_dir()?;
    let manager = ProfileManager::new(profiles_dir);

    let harness_kinds = [
        HarnessKind::OpenCode,
        HarnessKind::ClaudeCode,
        HarnessKind::Goose,
    ];
    let mut target_options: Vec<(String, InstallTarget)> = Vec::new();

    for kind in &harness_kinds {
        let Ok(harness) = Harness::locate(*kind) else {
            continue;
        };
        let harness_id = harness.id();
        let Ok(profiles) = manager.list_profiles(&harness) else {
            continue;
        };
        for profile in profiles {
            let label = format!("{}/{}", harness_id, profile);
            target_options.push((
                label,
                InstallTarget {
                    harness: harness_id.to_string(),
                    profile,
                },
            ));
        }
    }

    if target_options.is_empty() {
        return Err(eyre!("No profiles found. Create a profile first with: bridle profile create <harness> <name>"));
    }

    let labels: Vec<&str> = target_options.iter().map(|(l, _)| l.as_str()).collect();

    let active_indices: Vec<bool> = target_options
        .iter()
        .map(|(_, t)| config.active_profile_for(&t.harness) == Some(t.profile.as_str()))
        .collect();

    let Some(selected) = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select target profiles (Esc to cancel)")
        .items(&labels)
        .defaults(&active_indices)
        .interact_opt()?
    else {
        return Ok(Vec::new());
    };

    Ok(selected
        .into_iter()
        .map(|i| target_options[i].1.clone())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_source_handles_shorthand() {
        assert_eq!(
            normalize_source("owner/repo"),
            "https://github.com/owner/repo"
        );
    }

    #[test]
    fn normalize_source_preserves_full_url() {
        let url = "https://github.com/owner/repo";
        assert_eq!(normalize_source(url), url);
    }

    #[test]
    fn normalize_source_preserves_http() {
        let url = "http://example.com/repo";
        assert_eq!(normalize_source(url), url);
    }
}

//! MCP configuration read/write helpers for all harnesses.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use harness_locate::HarnessKind;

use crate::config::jsonc::strip_jsonc_comments;

#[derive(Debug, thiserror::Error)]
pub enum McpConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("Failed to parse YAML: {0}")]
    YamlParseError(#[from] serde_yaml::Error),

    #[error("Failed to write config: {0}")]
    WriteError(String),
}

fn get_mcp_key(kind: HarnessKind) -> &'static str {
    match kind {
        HarnessKind::ClaudeCode => "mcpServers",
        HarnessKind::OpenCode => "mcp",
        HarnessKind::Goose => "extensions",
        HarnessKind::AmpCode => "amp.mcpServers",
        _ => "mcpServers",
    }
}

pub fn read_mcp_config(
    kind: HarnessKind,
    config_path: &Path,
) -> Result<HashMap<String, serde_json::Value>, McpConfigError> {
    if !config_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(config_path)?;
    if content.trim().is_empty() {
        return Ok(HashMap::new());
    }

    let parsed: serde_json::Value = match kind {
        HarnessKind::Goose => {
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
            serde_json::to_value(yaml)?
        }
        HarnessKind::OpenCode => {
            let stripped = strip_jsonc_comments(&content);
            serde_json::from_str(&stripped)?
        }
        _ => serde_json::from_str(&content)?,
    };

    let key = get_mcp_key(kind);
    let mcp_section = parsed.get(key).and_then(|v| v.as_object());

    match mcp_section {
        Some(obj) => {
            let mut result = HashMap::new();
            for (name, value) in obj {
                if kind == HarnessKind::Goose {
                    if let Some(ext_type) = value.get("type").and_then(|t| t.as_str()) {
                        if !["stdio", "sse", "http", "streamable_http"].contains(&ext_type) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                result.insert(name.clone(), value.clone());
            }
            Ok(result)
        }
        None => Ok(HashMap::new()),
    }
}

pub fn write_mcp_config(
    kind: HarnessKind,
    config_path: &Path,
    servers: &HashMap<String, serde_json::Value>,
) -> Result<(), McpConfigError> {
    let key = get_mcp_key(kind);

    let mut existing: serde_json::Value = if config_path.exists() {
        let content = fs::read_to_string(config_path)?;
        if content.trim().is_empty() {
            serde_json::json!({})
        } else {
            match kind {
                HarnessKind::Goose => {
                    let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
                    serde_json::to_value(yaml)?
                }
                HarnessKind::OpenCode => {
                    let stripped = strip_jsonc_comments(&content);
                    serde_json::from_str(&stripped)?
                }
                _ => serde_json::from_str(&content)?,
            }
        }
    } else {
        serde_json::json!({})
    };

    let mcp_section = existing
        .as_object_mut()
        .ok_or_else(|| McpConfigError::WriteError("Config root is not an object".to_string()))?
        .entry(key)
        .or_insert_with(|| serde_json::json!({}));

    let mcp_obj = mcp_section.as_object_mut().ok_or_else(|| {
        McpConfigError::WriteError(format!("{} section is not an object", key))
    })?;

    for (name, value) in servers {
        mcp_obj.insert(name.clone(), value.clone());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let output = match kind {
        HarnessKind::Goose => {
            let yaml_value: serde_yaml::Value = serde_json::from_value(existing)?;
            serde_yaml::to_string(&yaml_value)?
        }
        _ => serde_json::to_string_pretty(&existing)?,
    };

    fs::write(config_path, output)?;
    Ok(())
}

pub fn mcp_exists(
    kind: HarnessKind,
    config_path: &Path,
    name: &str,
) -> Result<bool, McpConfigError> {
    let servers = read_mcp_config(kind, config_path)?;
    Ok(servers.contains_key(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_nonexistent_file_returns_empty() {
        let result = read_mcp_config(HarnessKind::ClaudeCode, Path::new("/nonexistent/path.json"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn read_empty_file_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        fs::write(&path, "").unwrap();

        let result = read_mcp_config(HarnessKind::ClaudeCode, &path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn read_claude_mcp_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(".mcp.json");
        fs::write(
            &path,
            r#"{"mcpServers": {"test-server": {"command": "test"}}}"#,
        )
        .unwrap();

        let result = read_mcp_config(HarnessKind::ClaudeCode, &path).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("test-server"));
    }

    #[test]
    fn read_opencode_jsonc_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("opencode.jsonc");
        fs::write(
            &path,
            r#"{
                // This is a comment
                "mcp": {
                    "my-mcp": {"command": "npx", "args": ["-y", "server"]}
                }
            }"#,
        )
        .unwrap();

        let result = read_mcp_config(HarnessKind::OpenCode, &path).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("my-mcp"));
    }

    #[test]
    fn read_goose_yaml_filters_mcp_types() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.yaml");
        fs::write(
            &path,
            r#"
extensions:
  developer:
    enabled: true
    type: builtin
  my-mcp:
    type: stdio
    cmd: npx
    args: ["-y", "server"]
"#,
        )
        .unwrap();

        let result = read_mcp_config(HarnessKind::Goose, &path).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("my-mcp"));
        assert!(!result.contains_key("developer"));
    }

    #[test]
    fn read_amp_config() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("settings.json");
        fs::write(
            &path,
            r#"{"amp.mcpServers": {"amp-mcp": {"command": "test"}}}"#,
        )
        .unwrap();

        let result = read_mcp_config(HarnessKind::AmpCode, &path).unwrap();
        assert_eq!(result.len(), 1);
        assert!(result.contains_key("amp-mcp"));
    }

    #[test]
    fn write_creates_file_if_missing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new-config.json");

        let mut servers = HashMap::new();
        servers.insert(
            "new-server".to_string(),
            serde_json::json!({"command": "test"}),
        );

        write_mcp_config(HarnessKind::ClaudeCode, &path, &servers).unwrap();

        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("new-server"));
    }

    #[test]
    fn write_preserves_existing_mcps() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        fs::write(
            &path,
            r#"{"mcpServers": {"existing": {"command": "old"}}}"#,
        )
        .unwrap();

        let mut servers = HashMap::new();
        servers.insert(
            "new-server".to_string(),
            serde_json::json!({"command": "new"}),
        );

        write_mcp_config(HarnessKind::ClaudeCode, &path, &servers).unwrap();

        let result = read_mcp_config(HarnessKind::ClaudeCode, &path).unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains_key("existing"));
        assert!(result.contains_key("new-server"));
    }

    #[test]
    fn write_preserves_other_config_fields() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        fs::write(
            &path,
            r#"{"model": "claude-4", "mcpServers": {}}"#,
        )
        .unwrap();

        let mut servers = HashMap::new();
        servers.insert("mcp".to_string(), serde_json::json!({"command": "test"}));

        write_mcp_config(HarnessKind::ClaudeCode, &path, &servers).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("model"));
        assert!(content.contains("claude-4"));
    }

    #[test]
    fn mcp_exists_returns_true_for_existing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.json");
        fs::write(
            &path,
            r#"{"mcpServers": {"test-mcp": {"command": "test"}}}"#,
        )
        .unwrap();

        assert!(mcp_exists(HarnessKind::ClaudeCode, &path, "test-mcp").unwrap());
        assert!(!mcp_exists(HarnessKind::ClaudeCode, &path, "nonexistent").unwrap());
    }

    #[test]
    fn mcp_exists_returns_false_for_missing_file() {
        let result = mcp_exists(
            HarnessKind::ClaudeCode,
            Path::new("/nonexistent/path.json"),
            "any",
        );
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}

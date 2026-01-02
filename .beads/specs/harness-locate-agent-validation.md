# Agent Frontmatter Validation for harness-locate

## Problem Statement

When installing agents across different harnesses via bridle, format incompatibilities cause runtime errors. For example, an agent with Claude Code format:

```yaml
---
tools: Glob, Grep, LS, Read, WebFetch
color: red
---
```

...fails when installed to OpenCode because:
1. `tools` must be `Record<string, boolean>`, not a comma-separated string
2. `color` must be hex format (`#FF0000`), not a named color

**Error observed:**
```
Configuration is invalid at /Users/d0/.config/opencode/agent/code-reviewer.md
- Invalid input: expected record, received string tools
- Invalid hex color format color
```

## Proposed Solution

Add agent frontmatter validation to `harness-locate`, following the existing `validation.rs` pattern used for MCP servers.

## Harness-Specific Agent Schemas

### OpenCode

**Source:** [sst/opencode config.ts#L430-L454](https://github.com/sst/opencode/blob/main/packages/opencode/src/config/config.ts)

| Field | Type | Validation |
|-------|------|------------|
| `name` | string | Optional |
| `description` | string | Optional |
| `model` | string | Optional |
| `temperature` | number | Optional, 0.0-2.0 |
| `top_p` | number | Optional, 0.0-1.0 |
| `prompt` | string | Optional |
| `tools` | `Record<string, boolean>` | **Must be object, NOT string/array** |
| `disable` | boolean | Optional |
| `mode` | `"subagent" \| "primary" \| "all"` | Optional |
| `color` | string | **Hex format: `/^#[0-9a-fA-F]{6}$/`** |
| `steps` | integer | Optional, positive |
| `permission` | object | Optional, nested permission rules |
| `hidden` | boolean | Optional |

**Valid example:**
```yaml
---
description: Code review agent
mode: subagent
color: "#44BA81"
tools:
  write: false
  edit: false
  bash: false
---
```

### Claude Code

| Field | Type | Validation |
|-------|------|------------|
| `name` | string | Optional |
| `description` | string | Optional |
| `model` | string | Optional |
| `tools` | string (comma-separated) | Allowed tool names |
| `color` | string | Named colors OR hex |
| `mode` | string | Optional |

**Valid example:**
```yaml
---
name: code-reviewer
description: Reviews code
tools: Glob, Grep, LS, Read, WebFetch
color: red
---
```

### Goose

Goose does **not support agents** - return `None` for agent validation.

### AMP Code

Similar to Claude Code (needs verification).

## API Design

### New Types

```rust
// In src/agent.rs (new file)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed agent frontmatter in normalized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub mode: Option<AgentMode>,
    pub color: Option<String>,
    pub tools: Option<AgentTools>,
    // ... other fields as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMode {
    Subagent,
    Primary,
    All,
}

/// Tools configuration - varies by harness
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentTools {
    /// OpenCode style: { "bash": true, "edit": false }
    Record(HashMap<String, bool>),
    /// Claude Code style: "Glob, Grep, LS, Read"
    List(String),
}
```

### New Validation Codes

```rust
// In src/validation.rs (add to existing)

/// Agent tools field has wrong type for harness.
pub const CODE_AGENT_TOOLS_TYPE: &str = "agent.tools.type_mismatch";

/// Agent color field has invalid format for harness.
pub const CODE_AGENT_COLOR_FORMAT: &str = "agent.color.invalid_format";

/// Agent mode value not supported by harness.
pub const CODE_AGENT_MODE_UNSUPPORTED: &str = "agent.mode.unsupported";

/// Agent field not recognized by harness.
pub const CODE_AGENT_FIELD_UNKNOWN: &str = "agent.field.unknown";

/// Agent temperature out of valid range.
pub const CODE_AGENT_TEMPERATURE_RANGE: &str = "agent.temperature.out_of_range";
```

### Harness Methods

```rust
// In src/harness/mod.rs (add to Harness impl)

impl Harness {
    /// Returns the agent schema capabilities for this harness.
    ///
    /// Returns `None` if this harness does not support agents.
    #[must_use]
    pub fn agent_capabilities(&self) -> Option<AgentCapabilities> {
        AgentCapabilities::for_kind(self.kind)
    }

    /// Validates agent frontmatter content for this harness.
    ///
    /// Returns `None` if this harness does not support agents.
    /// Returns `Some(issues)` with validation results (empty = valid).
    ///
    /// # Arguments
    ///
    /// * `content` - Raw markdown content with YAML frontmatter
    ///
    /// # Example
    ///
    /// ```
    /// use harness_locate::{Harness, HarnessKind};
    ///
    /// let harness = Harness::new(HarnessKind::OpenCode);
    /// let content = r#"---
    /// description: My agent
    /// tools: Glob, Grep  # Wrong format for OpenCode!
    /// ---
    /// Agent prompt here.
    /// "#;
    ///
    /// if let Some(issues) = harness.validate_agent(content) {
    ///     for issue in issues {
    ///         eprintln!("{}: {}", issue.field, issue.message);
    ///     }
    /// }
    /// ```
    pub fn validate_agent(&self, content: &str) -> Option<Vec<ValidationIssue>> {
        match self.kind {
            HarnessKind::Goose => None, // No agent support
            _ => Some(validate_agent_for_harness(content, self.kind)),
        }
    }

    /// Parses agent frontmatter from markdown content.
    ///
    /// Returns the parsed frontmatter or an error with details.
    pub fn parse_agent_frontmatter(&self, content: &str) -> Result<AgentFrontmatter> {
        // Parse YAML frontmatter
        // Return normalized AgentFrontmatter
    }
}
```

### Capabilities Struct

```rust
// In src/agent.rs

/// Describes what agent features a harness supports.
#[derive(Debug, Clone)]
pub struct AgentCapabilities {
    /// Whether the harness supports agents at all.
    pub supported: bool,
    /// Expected format for `tools` field.
    pub tools_format: ToolsFormat,
    /// Expected format for `color` field.
    pub color_format: ColorFormat,
    /// Supported mode values.
    pub supported_modes: &'static [&'static str],
    /// Whether `permission` field is supported.
    pub permission_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsFormat {
    /// Record<string, boolean> - OpenCode style
    BooleanRecord,
    /// Comma-separated string - Claude Code style
    CommaSeparatedString,
    /// Either format accepted
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    /// Only hex colors (#RRGGBB)
    HexOnly,
    /// Named colors (red, blue, etc.) or hex
    NamedOrHex,
    /// Any string
    Any,
}

impl AgentCapabilities {
    pub fn for_kind(kind: HarnessKind) -> Option<Self> {
        match kind {
            HarnessKind::OpenCode => Some(Self {
                supported: true,
                tools_format: ToolsFormat::BooleanRecord,
                color_format: ColorFormat::HexOnly,
                supported_modes: &["subagent", "primary", "all"],
                permission_supported: true,
            }),
            HarnessKind::ClaudeCode => Some(Self {
                supported: true,
                tools_format: ToolsFormat::CommaSeparatedString,
                color_format: ColorFormat::NamedOrHex,
                supported_modes: &["subagent", "primary"],
                permission_supported: false,
            }),
            HarnessKind::AmpCode => Some(Self {
                supported: true,
                tools_format: ToolsFormat::CommaSeparatedString,
                color_format: ColorFormat::NamedOrHex,
                supported_modes: &["subagent", "primary"],
                permission_supported: false,
            }),
            HarnessKind::Goose => None, // No agent support
        }
    }
}
```

### Validation Function

```rust
// In src/validation.rs (or new src/agent_validation.rs)

/// Validates agent frontmatter for a specific harness.
pub fn validate_agent_for_harness(
    content: &str,
    kind: HarnessKind,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    
    // Parse frontmatter
    let frontmatter = match parse_yaml_frontmatter(content) {
        Ok(fm) => fm,
        Err(e) => {
            issues.push(ValidationIssue::error(
                "frontmatter",
                format!("Failed to parse YAML frontmatter: {e}"),
                Some(CODE_AGENT_FRONTMATTER_INVALID),
            ));
            return issues;
        }
    };

    let caps = match AgentCapabilities::for_kind(kind) {
        Some(c) => c,
        None => return issues, // Harness doesn't support agents
    };

    // Validate tools field
    if let Some(tools) = &frontmatter.get("tools") {
        issues.extend(validate_tools_format(tools, caps.tools_format, kind));
    }

    // Validate color field
    if let Some(color) = frontmatter.get("color").and_then(|v| v.as_str()) {
        issues.extend(validate_color_format(color, caps.color_format, kind));
    }

    // Validate mode field
    if let Some(mode) = frontmatter.get("mode").and_then(|v| v.as_str()) {
        if !caps.supported_modes.contains(&mode) {
            issues.push(ValidationIssue::error(
                "mode",
                format!(
                    "Mode '{}' not supported by {}. Valid modes: {:?}",
                    mode, kind.as_str(), caps.supported_modes
                ),
                Some(CODE_AGENT_MODE_UNSUPPORTED),
            ));
        }
    }

    // Validate temperature range
    if let Some(temp) = frontmatter.get("temperature").and_then(|v| v.as_f64()) {
        if !(0.0..=2.0).contains(&temp) {
            issues.push(ValidationIssue::warning(
                "temperature",
                format!("Temperature {} is outside typical range 0.0-2.0", temp),
                Some(CODE_AGENT_TEMPERATURE_RANGE),
            ));
        }
    }

    issues
}

fn validate_tools_format(
    tools: &serde_yaml::Value,
    expected: ToolsFormat,
    kind: HarnessKind,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    
    match expected {
        ToolsFormat::BooleanRecord => {
            if !tools.is_mapping() {
                issues.push(ValidationIssue::error(
                    "tools",
                    format!(
                        "{} requires tools as object (e.g., {{ bash: true, edit: false }}), \
                         got {}",
                        kind.as_str(),
                        yaml_type_name(tools)
                    ),
                    Some(CODE_AGENT_TOOLS_TYPE),
                ));
            }
        }
        ToolsFormat::CommaSeparatedString => {
            if !tools.is_string() {
                issues.push(ValidationIssue::error(
                    "tools",
                    format!(
                        "{} requires tools as comma-separated string, got {}",
                        kind.as_str(),
                        yaml_type_name(tools)
                    ),
                    Some(CODE_AGENT_TOOLS_TYPE),
                ));
            }
        }
        ToolsFormat::Any => {} // Accept anything
    }
    
    issues
}

fn validate_color_format(
    color: &str,
    expected: ColorFormat,
    kind: HarnessKind,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let hex_regex = regex::Regex::new(r"^#[0-9a-fA-F]{6}$").unwrap();
    
    match expected {
        ColorFormat::HexOnly => {
            if !hex_regex.is_match(color) {
                issues.push(ValidationIssue::error(
                    "color",
                    format!(
                        "{} requires hex color format (#RRGGBB), got '{}'",
                        kind.as_str(),
                        color
                    ),
                    Some(CODE_AGENT_COLOR_FORMAT),
                ));
            }
        }
        ColorFormat::NamedOrHex | ColorFormat::Any => {
            // Accept both named colors and hex
        }
    }
    
    issues
}
```

## Public API Summary

### New Exports (src/lib.rs)

```rust
// Add to existing exports
pub mod agent;

pub use agent::{
    AgentCapabilities, AgentFrontmatter, AgentMode, AgentTools,
    ColorFormat, ToolsFormat,
};
pub use validation::{
    // existing exports...
    CODE_AGENT_TOOLS_TYPE,
    CODE_AGENT_COLOR_FORMAT,
    CODE_AGENT_MODE_UNSUPPORTED,
    CODE_AGENT_FIELD_UNKNOWN,
    CODE_AGENT_TEMPERATURE_RANGE,
};
```

### Usage Example (bridle installer)

```rust
use harness_locate::{Harness, HarnessKind};

fn install_agent(
    content: &str,
    target_harness: HarnessKind,
) -> Result<(), InstallError> {
    let harness = Harness::new(target_harness);
    
    // Validate before installing
    if let Some(issues) = harness.validate_agent(content) {
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        
        if !errors.is_empty() {
            return Err(InstallError::InvalidAgentFormat {
                harness: target_harness.to_string(),
                issues: errors.iter().map(|i| i.message.clone()).collect(),
            });
        }
        
        // Log warnings but continue
        for issue in issues.iter().filter(|i| i.severity == Severity::Warning) {
            eprintln!("Warning: {}: {}", issue.field, issue.message);
        }
    }
    
    // Proceed with installation...
    Ok(())
}
```

## Test Cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_rejects_comma_string_tools() {
        let content = r#"---
tools: Glob, Grep, Read
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.iter().any(|i| i.code == Some(CODE_AGENT_TOOLS_TYPE)));
    }

    #[test]
    fn opencode_accepts_boolean_record_tools() {
        let content = r#"---
tools:
  bash: true
  edit: false
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.is_empty());
    }

    #[test]
    fn opencode_rejects_named_color() {
        let content = r#"---
color: red
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.iter().any(|i| i.code == Some(CODE_AGENT_COLOR_FORMAT)));
    }

    #[test]
    fn opencode_accepts_hex_color() {
        let content = r#"---
color: "#FF5733"
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.is_empty());
    }

    #[test]
    fn claude_code_accepts_comma_string_tools() {
        let content = r#"---
tools: Glob, Grep, Read
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::ClaudeCode);
        assert!(!issues.iter().any(|i| i.code == Some(CODE_AGENT_TOOLS_TYPE)));
    }

    #[test]
    fn claude_code_accepts_named_color() {
        let content = r#"---
color: red
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::ClaudeCode);
        assert!(!issues.iter().any(|i| i.code == Some(CODE_AGENT_COLOR_FORMAT)));
    }

    #[test]
    fn goose_returns_none_for_agents() {
        let harness = Harness::new(HarnessKind::Goose);
        assert!(harness.validate_agent("any content").is_none());
    }

    #[test]
    fn invalid_yaml_returns_parse_error() {
        let content = r#"---
tools: [unclosed bracket
---
Agent prompt"#;
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.iter().any(|i| i.field == "frontmatter"));
    }

    #[test]
    fn missing_frontmatter_is_valid() {
        // Agent with no frontmatter should be valid (all fields optional)
        let content = "Just the agent prompt, no frontmatter";
        
        let issues = validate_agent_for_harness(content, HarnessKind::OpenCode);
        assert!(issues.is_empty());
    }
}
```

## Implementation Checklist

- [ ] Add `src/agent.rs` with types (`AgentFrontmatter`, `AgentCapabilities`, etc.)
- [ ] Add validation codes to `src/validation.rs`
- [ ] Add `validate_agent_for_harness()` function
- [ ] Add `Harness::validate_agent()` method
- [ ] Add `Harness::agent_capabilities()` method  
- [ ] Export new types from `src/lib.rs`
- [ ] Add comprehensive tests
- [ ] Update README/docs

## Dependencies

May need to add:
- `serde_yaml` for YAML frontmatter parsing (if not already present)
- `regex` for hex color validation (if not already present)

## Breaking Changes

None - this is purely additive.

## Future Considerations

1. **Transformation support**: Could add `Harness::transform_agent()` to convert between formats (risky, may be lossy)
2. **Skill validation**: Same pattern could apply to skills with frontmatter
3. **Command validation**: Same pattern for command frontmatter

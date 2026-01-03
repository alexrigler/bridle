# Critical Fix: Profile Switch Data Loss

**Severity:** CRITICAL - Data Loss  
**Reported:** 2026-01-03 via Twitter (@melvynxdev)  
**Affects:** ALL harnesses (Claude, OpenCode, Goose, AMP)

## Problem

`lifecycle.rs:155-158` deletes entire harness config directory during profile switch:

```rust
if target_dir.exists() {
    std::fs::remove_dir_all(&target_dir)?;  // DESTROYS everything
}
std::fs::rename(&temp_dir, &target_dir)?;
```

This destroys ALL runtime files bridle doesn't manage.

## Fix: Merge-Based Switch

Replace delete-and-replace with selective sync. **Only touch what's in the profile - preserve everything else.**

### Changes to `src/config/manager/lifecycle.rs`

Replace `switch_profile_with_resources()` (lines ~137-177):

```rust
/// Switch to a profile by syncing its contents to target.
/// 
/// IMPORTANT: Only items IN the profile are copied/replaced.
/// Files/dirs in target that are NOT in profile are PRESERVED.
/// This prevents data loss of runtime files (history, logs, caches).
pub fn switch_profile_with_resources(
    profile_path: &Path,
    target_dir: &Path,
    _harness: &Harness,
) -> Result<(), ConfigError> {
    // Ensure target exists
    if !target_dir.exists() {
        std::fs::create_dir_all(target_dir)?;
    }

    // Sync each item from profile to target
    for entry in std::fs::read_dir(profile_path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let source = entry.path();
        let dest = target_dir.join(&file_name);

        if source.is_file() {
            // Overwrite file in target
            std::fs::copy(&source, &dest)?;
        } else if source.is_dir() {
            // Replace entire directory (managed resources like skills/, plugins/)
            if dest.exists() {
                std::fs::remove_dir_all(&dest)?;
            }
            copy_dir_recursive(&source, &dest)?;
        }
    }

    // Delete marker files (profile-specific)
    delete_marker_files(target_dir)?;

    Ok(())
}

/// Recursively copy a directory and all contents.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ConfigError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
```

### Behavior Change

| Scenario                 | Before (BROKEN) | After (FIXED) |
| ------------------------ | --------------- | ------------- |
| File in profile & target | Overwrite       | Overwrite     |
| File in profile only     | Create          | Create        |
| File in target only      | **DELETED**     | Preserved     |
| Dir in profile & target  | Replace         | Replace       |
| Dir in profile only      | Create          | Create        |
| Dir in target only       | **DELETED**     | Preserved     |

### Files to Modify

1. `src/config/manager/lifecycle.rs` - Main fix
2. `tests/cli_integration.rs` - Add preservation test

### Test Plan

```bash
# Setup - create unknown file in harness config
echo "precious data" > ~/.claude/unknown-file.txt
mkdir -p ~/.claude/unknown-dir
echo "more data" > ~/.claude/unknown-dir/nested.txt

# Create test profile
cargo run -- profile create claude test-fix --from-current

# Switch away and back
cargo run -- profile switch claude default
cargo run -- profile switch claude test-fix

# Verify preservation
cat ~/.claude/unknown-file.txt           # Should print "precious data"
cat ~/.claude/unknown-dir/nested.txt     # Should print "more data"
```

### Integration Test

Add to `tests/cli_integration.rs`:

```rust
#[test]
fn test_switch_preserves_unknown_files() {
    // Setup temp harness config dir
    let temp_dir = tempfile::tempdir().unwrap();
    let config_dir = temp_dir.path().join(".claude");
    fs::create_dir_all(&config_dir).unwrap();
    
    // Create "unknown" file (simulates history.jsonl, etc.)
    let unknown_file = config_dir.join("unknown.txt");
    fs::write(&unknown_file, "precious").unwrap();
    
    // Create profile with only settings.json
    let profile_dir = temp_dir.path().join("profile");
    fs::create_dir_all(&profile_dir).unwrap();
    fs::write(profile_dir.join("settings.json"), "{}").unwrap();
    
    // Switch
    switch_profile_with_resources(&profile_dir, &config_dir, &harness).unwrap();
    
    // Verify unknown file preserved
    assert!(unknown_file.exists());
    assert_eq!(fs::read_to_string(&unknown_file).unwrap(), "precious");
    
    // Verify profile file applied
    assert!(config_dir.join("settings.json").exists());
}
```

## Checklist

- [ ] Refactor `switch_profile_with_resources()` to merge-based approach
- [ ] Add `copy_dir_recursive()` helper function
- [ ] Add integration test for unknown file preservation
- [ ] Manual test on Claude, OpenCode, Goose
- [ ] Update CHANGELOG.md with security/data-loss note
- [ ] Consider adding user-visible message about preserved files

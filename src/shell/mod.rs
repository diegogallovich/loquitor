use anyhow::{Context, Result};
use std::path::PathBuf;

pub const HOOK_START: &str = "# --- Loquitor Shell Hook (managed by loquitor) ---";
pub const HOOK_END: &str = "# --- End Loquitor Hook ---";

/// Generate the hook content with the given lanes directory.
pub fn hook_content(lanes_dir: &str) -> String {
    format!(
        r#"{HOOK_START}
__loquitor_hook() {{
  if [[ "$1" == "claude" ]]; then
    local lane_dir="{lanes_dir}"
    local logfile="$lane_dir/$(basename "$PWD")-$(date +%s).log"
    mkdir -p "$lane_dir"
    script -q "$logfile" claude "${{@:2}}"
  else
    command "$@"
  fi
}}
alias claude='__loquitor_hook claude'
{HOOK_END}"#
    )
}

/// Path to the user's zshrc file.
fn zshrc_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".zshrc")
}

/// Check if the hook is present in the given zshrc content.
pub fn is_hook_present(zshrc_content: &str) -> bool {
    zshrc_content.contains(HOOK_START)
}

/// Pure function: add the hook block to an existing zshrc content.
/// Strips any existing hook block first (for idempotency).
pub fn insert_hook(zshrc_content: &str, lanes_dir: &str) -> String {
    let without_hook = strip_hook(zshrc_content);
    let mut result = without_hook.trim_end().to_string();
    if !result.is_empty() {
        result.push('\n');
    }
    result.push('\n');
    result.push_str(&hook_content(lanes_dir));
    result.push('\n');
    result
}

/// Pure function: remove the hook block (and its markers) from zshrc content.
/// Everything between HOOK_START and HOOK_END (inclusive) is removed.
/// Returns the content without the hook, trimmed to end with exactly one newline.
pub fn strip_hook(zshrc_content: &str) -> String {
    let mut result = String::new();
    let mut skipping = false;

    for line in zshrc_content.lines() {
        if line.trim() == HOOK_START {
            skipping = true;
            continue;
        }
        if line.trim() == HOOK_END {
            skipping = false;
            continue;
        }
        if !skipping {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Normalize trailing whitespace
    let trimmed = result.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

/// Check if the hook is installed in ~/.zshrc.
pub fn is_installed() -> bool {
    let path = zshrc_path();
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(&path)
        .map(|c| is_hook_present(&c))
        .unwrap_or(false)
}

/// Install the hook into ~/.zshrc. Creates the file if it doesn't exist.
/// Idempotent — removes any existing hook block before inserting the new one.
pub fn install(lanes_dir: &str) -> Result<()> {
    let path = zshrc_path();

    let current_content = if path.exists() {
        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?
    } else {
        String::new()
    };

    let new_content = insert_hook(&current_content, lanes_dir);

    std::fs::write(&path, new_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

/// Remove the hook from ~/.zshrc. No-op if not present.
pub fn remove() -> Result<()> {
    let path = zshrc_path();
    if !path.exists() {
        return Ok(());
    }

    let current_content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let new_content = strip_hook(&current_content);

    std::fs::write(&path, new_content)
        .with_context(|| format!("Failed to write {}", path.display()))?;

    Ok(())
}

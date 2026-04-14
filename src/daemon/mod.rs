pub mod ipc;
pub mod liaison_worker;
pub mod pipeline;

use anyhow::{Context, Result};
use std::path::Path;
use tracing::info;

/// Write the current process's PID to the given file.
pub fn write_pid_file(path: &Path) -> Result<()> {
    let pid = std::process::id();
    std::fs::write(path, pid.to_string())
        .with_context(|| format!("Failed to write PID file at {}", path.display()))?;
    Ok(())
}

/// Read a PID file, returning `Ok(None)` if the file doesn't exist.
pub fn read_pid_file(path: &Path) -> Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read PID file at {}", path.display()))?;
    let pid: u32 = content
        .trim()
        .parse()
        .with_context(|| format!("Invalid PID in {}: {content}", path.display()))?;
    Ok(Some(pid))
}

/// Check whether the daemon process referenced by the PID file is alive.
pub fn is_daemon_running(pid_path: &Path) -> bool {
    match read_pid_file(pid_path) {
        Ok(Some(pid)) => {
            // Signal 0 tests process existence without actually sending a signal
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
        }
        _ => false,
    }
}

/// Stop the daemon by sending SIGTERM.
/// No-op if the PID file doesn't exist or the process isn't running.
pub fn stop_daemon(pid_path: &Path) -> Result<()> {
    if let Some(pid) = read_pid_file(pid_path)? {
        info!(pid, "Sending SIGTERM to daemon");
        let kill_result = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(pid as i32),
            nix::sys::signal::Signal::SIGTERM,
        );
        // Ignore ESRCH (process doesn't exist) - already stopped
        if let Err(e) = kill_result {
            if e != nix::errno::Errno::ESRCH {
                return Err(e).context("Failed to send SIGTERM to daemon");
            }
        }
    }
    // Always remove stale PID file
    let _ = std::fs::remove_file(pid_path);
    Ok(())
}

//! Process supervision deliberately avoids opening or deleting index files.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const REINDEX_TIMEOUT: Duration = Duration::from_secs(60);

type Log = fn(&str, &str);

pub fn run(args: &[String], home: Option<&Path>, log: Log) -> std::io::Result<i32> {
    let mut recovery_attempted = false;
    loop {
        let status = Command::new(std::env::current_exe()?)
            .arg("--worker")
            .args(args.iter().skip(1).filter(|a| *a != "--worker"))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !crate_profile_permits_recovery() {
            return Ok(status.code().unwrap_or(1));
        }
        if crashed(status) && !recovery_attempted {
            recovery_attempted = true;
            log(
                "CRASH",
                "Worker crashed; attempting launch-project reindex and one restart",
            );
            auto_reindex_current_project(home, log);
            continue;
        }
        if crashed(status) {
            log(
                "ERROR",
                "Worker crashed again; restart limit reached, leaving indexes untouched",
            );
        }
        return Ok(status.code().unwrap_or(1));
    }
}

fn crate_profile_permits_recovery() -> bool {
    std::env::var("INFIGRAPH_MCP_PROFILE").map_or(true, |v| v == "normal")
}

fn crashed(status: ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        status.signal() == Some(11)
    }
    #[cfg(windows)]
    {
        status.code().is_some_and(|code| code < 0)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = status;
        false
    }
}

fn recovery_root(root: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let index = root.join(".infigraph");
    // A shared/symlinked index can belong to a different project. Never
    // infer a recovery target from the registry or follow that redirection.
    if !index.is_dir() || index.canonicalize().ok()? != index {
        return None;
    }
    Some(root)
}

fn auto_reindex_current_project(home: Option<&Path>, log: Log) {
    if std::env::var("INFIGRAPH_BACKEND").is_ok_and(|backend| backend != "kuzu") {
        log(
            "WARN",
            "Skipping local crash recovery for a non-local backend",
        );
        return;
    }
    let root = std::env::current_dir().ok().and_then(|p| recovery_root(&p));
    let Some(root) = root else {
        log(
            "WARN",
            "No local launch-project index; skipping automatic reindex",
        );
        return;
    };
    let Some(cli) = find_infigraph_cli_for_reindex(home) else {
        log("ERROR", "Cannot find infigraph CLI for auto-reindex");
        return;
    };
    reindex_path(&cli, &root, REINDEX_TIMEOUT, log);
}

fn reindex_path(cli: &Path, root: &Path, timeout: Duration, log: Log) {
    // Let the ordinary index path classify lock errors and perform its
    // existing per-store recovery. A signal alone does not prove corruption.
    // Keep CLI progress off the JSON-RPC stdout stream, and don't consume
    // client requests or spawn an independent watcher during recovery.
    let child = Command::new(cli)
        .arg("index")
        .current_dir(root)
        .env("INFIGRAPH_NO_WATCH", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(_) => {
            log(
                "ERROR",
                "Reindex spawn failed; indexes were not preemptively wiped",
            );
            return;
        }
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    log("INFO", "Launch-project reindex completed");
                } else {
                    log("ERROR", "Launch-project reindex failed");
                }
                return;
            }
            Ok(None) if start.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(50));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                log(
                    "ERROR",
                    "Reindex timed out or could not be monitored; stopping recovery child",
                );
                return;
            }
        }
    }
}

fn find_infigraph_cli_for_reindex(home: Option<&Path>) -> Option<PathBuf> {
    let bin_name = if cfg!(windows) {
        "infigraph.exe"
    } else {
        "infigraph"
    };
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.parent()?.join(bin_name);
        if sibling.exists() {
            return Some(sibling);
        }
    }
    if let Some(home) = home {
        let local_bin = home.join(".local").join("bin").join(bin_name);
        if local_bin.exists() {
            return Some(local_bin);
        }
    }
    None
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn quiet(_: &str, _: &str) {}

    #[test]
    fn recovery_child_timeout_is_bounded_and_reaped() {
        let dir = std::env::temp_dir().join(format!("infigraph-recovery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cli = dir.join("infigraph");
        std::fs::write(&cli, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
        std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o700)).unwrap();
        let started = Instant::now();
        reindex_path(&cli, &dir, Duration::from_millis(100), quiet);
        assert!(started.elapsed() >= Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_secs(3));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn shared_index_is_not_a_recovery_target() {
        let dir = std::env::temp_dir().join(format!("infigraph-target-{}", std::process::id()));
        let project = dir.join("project");
        let shared = dir.join("shared");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(&shared).unwrap();
        assert!(recovery_root(&project).is_none());
        symlink(&shared, project.join(".infigraph")).unwrap();
        assert!(recovery_root(&project).is_none());
        std::fs::remove_file(project.join(".infigraph")).unwrap();
        std::fs::create_dir(project.join(".infigraph")).unwrap();
        assert_eq!(
            recovery_root(&project),
            Some(project.canonicalize().unwrap())
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}

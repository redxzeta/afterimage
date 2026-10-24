//! Advisory evidence only: neither Git nor the MCP watcher registry proves
//! that graph content and cross-file resolution match the working tree.
use std::path::Path;
use std::process::Command;

use infigraph_core::multi::{git_head_commit, registry_path, Registry};

pub(crate) const NOTICE_PREFIX: &str = "\n\n⚠ Graph freshness unverified: ";

pub(crate) fn append_freshness_notice(root: &Path, output: String) -> String {
    let current = git_head_commit(root);
    // Read only the local JSON. Registry::load can connect
    // to a remote backend in feature-enabled builds; this advisory must not.
    let indexed = registry_path()
        .ok()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<Registry>(&bytes).ok())
        .map(|registry| {
            let mut heads: Vec<_> = registry
                .repos
                .values()
                .filter(|entry| entry.path.canonicalize().is_ok_and(|path| path == root))
                .filter_map(|entry| entry.last_indexed_commit.clone())
                .filter(|head| {
                    matches!(head.len(), 40 | 64) && head.bytes().all(|b| b.is_ascii_hexdigit())
                })
                .collect();
            heads.sort();
            heads.dedup();
            heads
        })
        .unwrap_or_default();

    let revision = match current.as_deref() {
        Some(head) if !indexed.is_empty() => format!(
            "registry_indexed_head={} current_head={head} ({}; registry may lag watcher updates)",
            indexed.join(","),
            if indexed.iter().all(|indexed| indexed == head) {
                "revision matches"
            } else {
                "branch/commit changed"
            }
        ),
        _ => "Git/index revision unavailable".into(),
    };
    let dirty = Command::new("git")
        .args([
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain",
            "--untracked-files=normal",
            "--ignore-submodules=all",
            "--",
            ".",
            ":(exclude).infigraph",
        ])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| !output.stdout.is_empty());
    let tree = match dirty {
        Some(true) => "uncommitted changes (may already be indexed)",
        Some(false) => "Git status clean (submodules not checked; not a content check)",
        None => "working-tree status unavailable",
    };
    let pending = super::watch::WATCHERS.try_lock().ok().and_then(|guard| {
        let mut count = 0;
        if let Some(map) = guard.as_ref() {
            for entry in map.values().filter(|entry| Path::new(&entry.path) == root) {
                count += entry.pending_reindex.try_lock().ok()?.len();
            }
        }
        Some(count)
    });
    let watcher = match pending {
        Some(count) => format!("pending cross-file reindex entries={count} in this MCP worker"),
        None => "pending cross-file reindex state unavailable".into(),
    };
    format!("{output}{NOTICE_PREFIX}{revision}; {tree}; {watcher}. Queued/in-progress file updates and content reconciliation are not verified. Results may be stale; use index_project to refresh.\n")
}

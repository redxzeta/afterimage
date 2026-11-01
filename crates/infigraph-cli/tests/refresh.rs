use serde_json::Value;
use std::process::Command;

fn run(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_infigraph"))
        .arg("--root")
        .arg(root)
        .args(args)
        .env("INFIGRAPH_BACKEND", "kuzu")
        .output()
        .unwrap()
}

#[test]
fn refresh_noop_lock_partial_retry_and_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let missing = run(root, &["freshness"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&missing.stdout).unwrap()["state"],
        "UNKNOWN"
    );
    assert!(!root.join(".infigraph").exists());
    std::fs::write(root.join("app.ts"), "export function first() {}\n").unwrap();
    let refresh = run(root, &["refresh", "--max-seconds", "60"]);
    assert!(
        refresh.status.success(),
        "{}",
        String::from_utf8_lossy(&refresh.stderr)
    );
    let pointer = root.join(".infigraph/current-snapshot.json");
    let first = std::fs::read(&pointer).unwrap();
    let generation: String = serde_json::from_slice(&first).unwrap();
    assert!(!root.join(".infigraph/graph").exists());
    let fresh = run(root, &["freshness"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&fresh.stdout).unwrap()["state"],
        "FRESH"
    );
    let noop = run(root, &["refresh"]);
    assert!(noop.status.success());
    assert!(String::from_utf8_lossy(&noop.stdout).contains("NO_OP"));
    assert_eq!(std::fs::read(&pointer).unwrap(), first);
    let lock = infigraph_core::lockfile::acquire(
        &root.join(".infigraph/refresh.lock"),
        "test",
        std::time::Duration::ZERO,
    )
    .unwrap();
    assert!(!run(root, &["refresh"]).status.success());
    drop(lock);
    std::fs::create_dir_all(root.join(".infigraph/snapshots/partial")).unwrap();
    std::fs::write(
        root.join(".infigraph/snapshots/partial/NOT_READY"),
        "crashed",
    )
    .unwrap();
    assert!(!run(root, &["refresh", "--rollback", "partial"])
        .status
        .success());
    assert_eq!(std::fs::read(&pointer).unwrap(), first);
    std::fs::write(root.join("app.ts"), "export function second() {}\n").unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&run(root, &["freshness"]).stdout).unwrap()["state"],
        "STALE"
    );
    let timeout = run(root, &["refresh", "--max-seconds", "0"]);
    assert!(!timeout.status.success());
    assert!(String::from_utf8_lossy(&timeout.stderr).contains("timed out"));
    assert_eq!(std::fs::read(&pointer).unwrap(), first);
    assert!(std::fs::read_dir(root.join(".infigraph/snapshots"))
        .unwrap()
        .filter_map(Result::ok)
        .any(|e| e.path().join("NOT_READY").exists()));
    assert!(run(root, &["refresh"]).status.success());
    assert_ne!(std::fs::read(&pointer).unwrap(), first);
    assert!(run(root, &["refresh", "--rollback", &generation])
        .status
        .success());
    assert_eq!(std::fs::read(&pointer).unwrap(), first);
    assert_eq!(
        serde_json::from_slice::<Value>(&run(root, &["freshness"]).stdout).unwrap()["state"],
        "STALE"
    );
    assert!(!root.join(".infigraph/watch.lock").exists());
}

#[test]
fn malformed_legacy_graph_is_never_opened_or_replaced_by_refresh() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".infigraph")).unwrap();
    let graph = dir.path().join(".infigraph/graph");
    std::fs::write(&graph, b"broken legacy graph").unwrap();
    std::fs::write(dir.path().join("app.ts"), "export function run() {}").unwrap();
    let freshness = run(dir.path(), &["freshness"]);
    assert!(freshness.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&freshness.stdout).unwrap()["state"],
        "UNKNOWN"
    );
    let result = run(dir.path(), &["refresh", "--max-seconds", "60"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(std::fs::read(&graph).unwrap(), b"broken legacy graph");
    let freshness = run(dir.path(), &["freshness"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&freshness.stdout).unwrap()["state"],
        "FRESH"
    );
}

use std::path::Path;
use std::process::Command;
use std::sync::{mpsc, Arc, Mutex};

use infigraph_core::{multi::Registry, Infigraph};
use infigraph_languages::bundled_registry;
use infigraph_mcp::{dispatch_tool, handle_tools_call};
use serde_json::{json, Value};

const TOOLS: [&str; 4] = [
    "trace_callers",
    "trace_callees",
    "transitive_impact",
    "find_all_references",
];

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn assert_notice(args: &Value, evidence: &str) {
    for tool in TOOLS {
        let output = dispatch_tool(tool, args).unwrap();
        assert!(
            output.contains("⚠ Graph freshness unverified:"),
            "{tool}: {output}"
        );
        assert!(output.contains(evidence), "{tool}: {output}");
        assert!(output.contains("Results may be stale"), "{tool}: {output}");
    }
}

// One process-local scenario keeps registry and watcher mutations serial. The
// registry is redirected to this test's disposable home before any indexing.
#[test]
fn advisory_covers_branch_edits_deletion_pending_and_protocol_without_claiming_fresh() {
    let home = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("INFIGRAPH_REGISTRY_HOME");
    std::env::set_var("INFIGRAPH_REGISTRY_HOME", home.path());
    struct RestoreRegistry(Option<std::ffi::OsString>);
    impl Drop for RestoreRegistry {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => std::env::set_var("INFIGRAPH_REGISTRY_HOME", value),
                None => std::env::remove_var("INFIGRAPH_REGISTRY_HOME"),
            }
        }
    }
    let _restore = RestoreRegistry(previous);
    let dir = tempfile::Builder::new()
        .prefix("afterimage-freshness-")
        .tempdir()
        .unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(
        root.join("lib.py"),
        "def helper():\n    return 1\n\ndef caller():\n    return helper()\n",
    )
    .unwrap();
    std::fs::write(root.join(".gitignore"), ".infigraph/\n").unwrap();
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "user.name", "Fixture"]);
    git(&root, &["config", "user.email", "fixture@example.invalid"]);
    git(&root, &["add", "."]);
    git(&root, &["commit", "-qm", "initial"]);
    let initial = git(&root, &["rev-parse", "HEAD"]);
    let mut prism = Infigraph::open(&root, bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism.index().unwrap();
    // Ensure both populated and empty result paths independent of resolver heuristics.
    prism.backend().unwrap().raw_query("MATCH (a:Symbol), (b:Symbol) WHERE a.id = 'lib.py::caller' AND b.id = 'lib.py::helper' CREATE (a)-[:CALLS]->(b)").unwrap();
    Registry::default()
        .register_repo("fixture", &root, &prism)
        .unwrap();
    drop(prism);
    let args = json!({"path":root, "symbol_id":"lib.py::helper"});
    let registry_file = home.path().join(".infigraph/registry.json");
    let registry_before = std::fs::read(&registry_file).unwrap();
    let status_file = root.join(".infigraph/index-status.json");
    let status_before = std::fs::read(&status_file).unwrap();
    assert_notice(&args, "revision matches");
    assert!(dispatch_tool("trace_callers", &args)
        .unwrap()
        .contains("lib.py::caller"));
    assert_notice(
        &json!({"path":root, "symbol_id":"missing"}),
        "revision matches",
    );
    assert_eq!(std::fs::read(&registry_file).unwrap(), registry_before);
    assert_eq!(std::fs::read(&status_file).unwrap(), status_before);

    std::fs::write(root.join("lib.py"), "def changed(): return 2\n").unwrap();
    assert_notice(&args, "uncommitted changes");
    git(&root, &["checkout", "--", "lib.py"]);
    std::fs::remove_file(root.join("lib.py")).unwrap();
    assert_notice(&args, "uncommitted changes");
    git(&root, &["checkout", "--", "lib.py"]);
    std::fs::create_dir_all(root.join("new/nested")).unwrap();
    std::fs::write(root.join("new/nested/new.py"), "def added(): pass\n").unwrap();
    assert_notice(&args, "uncommitted changes");
    git(&root, &["checkout", "-qb", "other"]);
    git(&root, &["add", "new"]);
    git(&root, &["commit", "-qm", "nested file"]);
    let other = git(&root, &["rev-parse", "HEAD"]);
    assert_notice(&args, "branch/commit changed");
    assert_notice(
        &json!({"path":root.join("new/nested"), "symbol_id":"lib.py::helper"}),
        "branch/commit changed",
    );
    let output = dispatch_tool("trace_callers", &args).unwrap();
    assert!(output.contains(&initial) && output.contains(&other));
    // Even a successful subset index at the new HEAD must not mint a fresh certificate.
    let mut prism = Infigraph::open(&root, bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism
        .index_files(&[root.join("new/nested/new.py")])
        .unwrap();
    drop(prism);
    assert_notice(&args, "branch/commit changed");
    git(&root, &["checkout", "-q", "main"]);
    assert_notice(&args, "revision matches");
    assert!(!root.join("new/nested/new.py").exists());
    let stale = dispatch_tool(
        "get_symbols_in_file",
        &json!({"path":root, "file":"new/nested/new.py"}),
    )
    .unwrap();
    assert!(
        stale.contains("added"),
        "fixture must demonstrate that matching HEAD alone can leave stale graph content: {stale}"
    );

    use infigraph_mcp::tools::watch::{WatcherEntry, WATCHERS};
    let (stop_tx, _stop_rx) = mpsc::channel();
    let pending = Arc::new(Mutex::new(vec!["lib.py".into()]));
    WATCHERS
        .lock()
        .unwrap()
        .get_or_insert_with(Default::default)
        .insert(
            "fixture".into(),
            WatcherEntry {
                path: root.to_string_lossy().replace('\\', "/"),
                stop_tx,
                pending_reindex: pending.clone(),
            },
        );
    assert_notice(&args, "pending cross-file reindex entries=1");
    {
        let _busy = pending.lock().unwrap();
        assert_notice(&args, "pending cross-file reindex state unavailable");
    }
    let poison = std::thread::spawn(move || {
        let _guard = pending.lock().unwrap();
        panic!("fixture poison");
    })
    .join();
    assert!(poison.is_err(), "fixture should poison the pending lock");
    assert_notice(&args, "pending cross-file reindex state unavailable");
    WATCHERS.lock().unwrap().as_mut().unwrap().remove("fixture");

    std::fs::write(&registry_file, "not json").unwrap();
    assert_notice(&args, "Git/index revision unavailable");
    std::fs::write(&registry_file, registry_before).unwrap();
    std::fs::remove_dir_all(root.join(".git")).unwrap();
    assert_notice(&args, "working-tree status unavailable");
    // Repeated protocol responses retain the warning through compression/dedup.
    for tool in TOOLS {
        for _ in 0..2 {
            let response = handle_tools_call(
                &json!(1),
                &json!({"params":{"name":tool, "arguments":args}}),
            );
            assert_ne!(response["result"]["isError"], true, "{response}");
            assert!(
                response["result"]["content"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("⚠ Graph freshness unverified:"),
                "{response}"
            );
        }
    }
}

#[test]
fn compression_and_seen_dedup_keep_notice_on_long_reference_lists() {
    use infigraph_mcp::compress::compress_pipeline_safe;
    let previous = std::env::var_os("INFIGRAPH_COMPRESSION_LEVEL");
    let previous_dedup = std::env::var_os("INFIGRAPH_DEDUP");
    std::env::set_var("INFIGRAPH_COMPRESSION_LEVEL", "summary");
    std::env::set_var("INFIGRAPH_DEDUP", "1");
    struct RestoreLevel(Option<std::ffi::OsString>, Option<std::ffi::OsString>);
    impl Drop for RestoreLevel {
        fn drop(&mut self) {
            for (name, previous) in [
                ("INFIGRAPH_COMPRESSION_LEVEL", &self.0),
                ("INFIGRAPH_DEDUP", &self.1),
            ] {
                match previous {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
    let _restore = RestoreLevel(previous, previous_dedup);
    let body = format!(
        "References to 'helper' (100 total):\n\n{}",
        (0..100)
            .map(|n| format!("  file.py:{n} — in caller{n}\n"))
            .collect::<String>()
    );
    let raw = format!("{body}\n\n⚠ Graph freshness unverified: fixture evidence\n");
    let args = json!({"symbol_id":"freshness-compression-fixture", "path":"unused"});
    for round in 0..2 {
        let output = compress_pipeline_safe(&raw, "find_all_references", &args);
        if round == 0 {
            assert!(
                output.contains("100x"),
                "body must exercise compression: {output}"
            );
        } else {
            assert!(
                output.starts_with("(seen "),
                "body must exercise dedup: {output}"
            );
        }
        assert!(
            output.ends_with("\n\n⚠ Graph freshness unverified: fixture evidence\n"),
            "{output}"
        );
    }
}

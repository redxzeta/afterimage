use std::{collections::BTreeMap, path::Path, process::Command};

use infigraph_core::{
    index_status::{self, StageOutcome, StageState},
    Infigraph,
};
use infigraph_languages::bundled_registry;
use infigraph_mcp::{
    handle_tools_call_with_profile, handle_tools_list_with_profile,
    profile::{self, McpProfile},
    tools::helpers::resolve_project_path,
};
use serde_json::{json, Value};

fn call(tool: &str, args: Value) -> Value {
    handle_tools_call_with_profile(
        &json!(1),
        &json!({"params":{"name":tool,"arguments":args}}),
        McpProfile::ReadOnly,
    )
}
fn text(response: &Value) -> &str {
    response["result"]["content"][0]["text"].as_str().unwrap()
}
fn bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn linked_child_and_sibling_never_borrow_parent_index() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = tmp.path().join("parent");
    std::fs::create_dir(&parent).unwrap();
    git(&parent, &["init", "--quiet"]);
    git(
        &parent,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    );
    let child = parent.join("child");
    let sibling = tmp.path().join("sibling");
    for (path, branch) in [(&child, "child"), (&sibling, "sibling")] {
        git(
            &parent,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                branch,
                path.to_str().unwrap(),
            ],
        );
        assert!(path.join(".git").is_file());
        std::fs::create_dir(path.join("src")).unwrap();
    }
    // Deliberately non-database sentinel: regression must resolve identity
    // before attempting any graph open or auto-repair.
    std::fs::create_dir(parent.join(".infigraph")).unwrap();
    std::fs::write(parent.join(".infigraph/graph"), b"parent-graph-sentinel").unwrap();
    for root in [&child, &sibling] {
        assert_eq!(
            resolve_project_path(root.join("src").to_str().unwrap()).unwrap(),
            root.to_str().unwrap()
        );
        let result = call("get_stats", json!({"path":root}));
        assert_eq!(result["result"]["isError"], true);
        assert!(text(&result).contains("NOT_INDEXED"), "{result}");
        assert!(text(&result).contains(root.to_str().unwrap()));
        assert!(!root.join(".infigraph").exists());
    }
    assert_eq!(
        std::fs::read(parent.join(".infigraph/graph")).unwrap(),
        b"parent-graph-sentinel"
    );
    assert!(
        resolve_project_path(parent.join("mistyped").to_str().unwrap())
            .unwrap_err()
            .to_string()
            .contains("INVALID_PATH")
    );
    assert_eq!(
        resolve_project_path(tmp.path().to_str().unwrap()).unwrap(),
        tmp.path().to_str().unwrap()
    );
}

#[test]
fn curated_discovery_and_manual_calls_enforce_the_same_boundary() {
    let list = handle_tools_list_with_profile(&json!(1), McpProfile::ReadOnly);
    let names: Vec<_> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), profile::READ_ONLY_TOOLS.len());
    assert!(names.contains(&"search_symbols"));
    let dir = tempfile::tempdir().unwrap();
    let before = bytes(dir.path());
    for tool in [
        "index_project",
        "delete_project",
        "save_session",
        "watch_project",
        "query_graph",
        "group_query",
        "scip_import",
        "compress",
    ] {
        assert!(!names.contains(&tool));
        let args = json!({"path":dir.path(),"cypher":"MATCH (s:Symbol) DELETE s"});
        assert!(text(&call(tool, args.clone())).contains("READ_ONLY_TOOL_DENIED"));
        assert!(
            infigraph_mcp::dispatch_tool_with_profile(tool, &args, McpProfile::ReadOnly)
                .unwrap_err()
                .to_string()
                .contains("READ_ONLY_TOOL_DENIED")
        );
    }
    assert_eq!(before, bytes(dir.path()));
    let normal = handle_tools_list_with_profile(&json!(1), McpProfile::Normal);
    assert_eq!(
        normal["result"]["tools"].as_array().unwrap().len(),
        infigraph_mcp::MCP_TOOL_NAMES.len()
    );
    // Explicit group scope remains valid in the normal profile.
    profile::validate(
        "group_search",
        &json!({"group_name":"team/services","query":"auth"}),
        McpProfile::Normal,
    )
    .unwrap();
}

#[test]
fn path_typos_are_errors_before_cwd_or_activity_selection() {
    for profile in [McpProfile::Normal, McpProfile::ReadOnly] {
        for key in [
            "project",
            "root",
            "cwd",
            "project_path",
            "paht",
            "patth",
            "worktree",
        ] {
            let mut args = json!({"query":"helper"});
            args[key] = json!("/tmp/explicit-checkout");
            let result = handle_tools_call_with_profile(
                &json!(1),
                &json!({"params":{"name":"search_symbols","arguments":args}}),
                profile,
            );
            assert_eq!(result["result"]["isError"], true);
            assert!(text(&result).contains("UNKNOWN_ARGUMENT"), "{result}");
        }
    }
    assert!(text(&call("get_stats", json!({"path":false}))).contains("INVALID_PATH"));
}

#[test]
fn reads_preserve_bytes_and_partial_warnings_survive_a_thin_client() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.py"),
        "def helper():\n    return 1\ndef caller():\n    return helper()\n",
    )
    .unwrap();
    let mut prism = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism.index().unwrap();
    eprintln!("fixture: AST index returned");
    let run = index_status::load(dir.path()).unwrap().run_id;
    index_status::update_current(dir.path(), &run, prism.backend(), |r| {
        r.scip.insert(
            "scip-python".into(),
            StageOutcome::new(StageState::Skipped, "NOT_REQUESTED"),
        );
        r.scip.insert(
            "fixture-failed".into(),
            StageOutcome::new(StageState::Failed, "SCIP_PROCESS_FAILED"),
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    drop(prism);
    eprintln!("fixture: writer closed");
    let before = bytes(dir.path());
    let registry_path = infigraph_core::multi::registry_path().unwrap();
    let registry_before = std::fs::read(&registry_path).ok();
    let session_before = infigraph_mcp::session_context::get_compression_stats();
    for tool in profile::READ_ONLY_TOOLS {
        eprintln!("read-only contract: {tool}");
        let args = match *tool {
            "diagnose" | "get_stats" => json!({"path":dir.path()}),
            "get_symbols_in_file" | "get_file_deps" => json!({"path":dir.path(),"file":"a.py"}),
            "search_symbols" => json!({"path":dir.path(),"query":"helper"}),
            _ => json!({"path":dir.path(),"symbol_id":"a.py::helper"}),
        };
        let mut response = call(tool, args);
        assert_ne!(response["result"]["isError"], true, "{tool}: {response}");
        if *tool != "diagnose" {
            let meta = &response["result"]["_meta"]["afterimage"];
            assert_eq!(meta["schema_version"], 1);
            assert_eq!(meta["project"]["root"], dir.path().to_str().unwrap());
            assert_eq!(meta["freshness"], "unknown");
            assert_eq!(
                meta["coverage"]["compiler"]["scip-python"]["reason"],
                "NOT_REQUESTED"
            );
            assert_eq!(
                meta["coverage"]["compiler"]["fixture-failed"]["state"],
                "failed"
            );
            response["result"].as_object_mut().unwrap().remove("_meta");
            let visible = text(&response);
            assert!(
                visible.contains(dir.path().to_str().unwrap()),
                "{tool}: {visible}"
            );
            assert!(visible.contains("Freshness: unknown"));
            assert!(visible.contains("NOT_REQUESTED") && visible.contains("SCIP_PROCESS_FAILED"));
            assert!(visible.contains("do not prove absence"));
        } else {
            let report: Value = serde_json::from_str(text(&response)).unwrap();
            assert_eq!(
                report["analysis_evidence"]["coverage"]["compiler"]["scip-python"]["reason"],
                "NOT_REQUESTED"
            );
            assert!(report["analysis_evidence"]["warnings"]
                .to_string()
                .contains("SCIP_PROCESS_FAILED"));
        }
    }
    // Exercise the actual supervisor/worker stdio path as a thin agent client,
    // including initialized startup with metrics enabled (still no writes).
    {
        use std::io::Write;
        use std::process::Stdio;
        let mut child = Command::new(env!("CARGO_BIN_EXE_infigraph-mcp"))
            .arg("--mcp")
            .current_dir(dir.path())
            .env("INFIGRAPH_MCP_PROFILE", "read-only")
            .env("INFIGRAPH_METRICS", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let requests = [
            json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"trace_callers","arguments":{"path":dir.path(),"symbol_id":"a.py::helper"}}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"diagnose","arguments":{"path":dir.path()}}}),
        ];
        for request in requests {
            writeln!(child.stdin.as_mut().unwrap(), "{request}").unwrap();
        }
        drop(child.stdin.take());
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let mut responses: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(responses.len(), 3);
        assert_eq!(responses[0]["result"]["protocolVersion"], "2024-11-05");
        for response in &mut responses {
            response["result"].as_object_mut().unwrap().remove("_meta");
        }
        assert!(
            text(&responses[1]).contains("NOT_REQUESTED")
                && text(&responses[1]).contains("SCIP_PROCESS_FAILED")
        );
        assert!(text(&responses[1]).contains(dir.path().to_str().unwrap()));
        let report: Value = serde_json::from_str(text(&responses[2])).unwrap();
        assert_eq!(
            report["analysis_evidence"]["coverage"]["compiler"]["fixture-failed"]["state"],
            "failed"
        );
    }
    let limited = call(
        "search_symbols",
        json!({"path":dir.path(),"query":"function","limit":1}),
    );
    assert!(text(&limited).contains("TRUNCATED:"), "{limited}");
    assert_eq!(
        limited["result"]["_meta"]["afterimage"]["limits"]["query_truncated"],
        true
    );
    assert_eq!(
        call(
            "search_symbols",
            json!({"path":dir.path(),"query":"helper","limit":1001})
        )["result"]["isError"],
        true
    );
    // Empty graph reads retain the same warnings in text and metadata.
    let empty = call(
        "trace_callers",
        json!({"path":dir.path(),"symbol_id":"a.py::missing"}),
    );
    assert!(text(&empty).contains("NOT_REQUESTED"));
    // Errors must be just as read-only as successful reads.
    assert_eq!(
        call(
            "get_code_snippet",
            json!({"path":dir.path(),"symbol_id":"missing"})
        )["result"]["isError"],
        true
    );
    assert_eq!(
        session_before,
        infigraph_mcp::session_context::get_compression_stats()
    );
    assert_eq!(before, bytes(dir.path()));
    assert_eq!(registry_before, std::fs::read(registry_path).ok());
    // A receipt from another graph must never supply a generation/coverage claim.
    let receipt = dir.path().join(".infigraph/index-status.json");
    let mut invalid: Value = serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
    invalid["indexed_file_fingerprint"] = json!("obsolete");
    std::fs::write(&receipt, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let stale = call("get_stats", json!({"path":dir.path()}));
    assert!(stale["result"]["_meta"]["afterimage"]["generation"].is_null());
    assert!(text(&stale).contains("coverage unknown"));
}

#[cfg(unix)]
#[test]
fn source_and_index_symlink_escapes_are_rejected() {
    use std::os::unix::fs::symlink;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("root");
    let other = tmp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::fs::write(other.join("secret.py"), "sentinel").unwrap();
    symlink(other.join("secret.py"), root.join("escape.py")).unwrap();
    assert!(
        infigraph_mcp::tools::helpers::project_file(&root, "escape.py")
            .unwrap_err()
            .to_string()
            .contains("PATH_ESCAPE")
    );
    assert!(infigraph_mcp::tools::helpers::project_file(&root, "../other/secret.py").is_err());
    std::fs::create_dir(other.join(".infigraph")).unwrap();
    std::fs::write(other.join(".infigraph/graph"), b"other-index").unwrap();
    symlink(other.join(".infigraph"), root.join(".infigraph")).unwrap();
    assert!(resolve_project_path(root.to_str().unwrap())
        .unwrap_err()
        .to_string()
        .contains("PATH_ESCAPE"));
    assert_eq!(
        std::fs::read(other.join(".infigraph/graph")).unwrap(),
        b"other-index"
    );
}

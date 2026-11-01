//! Agent-visible evidence plus MCP 2024-11-05 Result._meta. The text remains
//! useful when a thin client drops metadata; no protocol upgrade is required.
use anyhow::Result;
use serde_json::{json, Value};

use crate::{profile, tools::helpers};
const MAX_LINES: usize = 200;
const MAX_BYTES: usize = 32_768;

pub(crate) fn read_only_call(id: &Value, tool: &str, args: &Value) -> Value {
    // No activity, compression/dedup state, session focus, metrics, or error log
    // files on this path, including failures.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<Value> {
        let mut scoped = args.clone();
        let raw = args
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("INVALID_PATH: explicit path required"))?;
        let root = helpers::resolve_project_path(raw)?;
        scoped["path"] = json!(root);
        // Pin the immutable generation once for result and evidence together.
        scoped["_snapshot_storage"] = json!(infigraph_core::snapshot::selected(
            std::path::Path::new(&root)
        )?);
        let content = profile::dispatch(tool, &scoped)?;
        Ok(with_evidence(id, tool, &scoped, content, true))
    }));
    match result {
        Ok(Ok(response)) => response,
        Ok(Err(e)) => {
            let mut response = profile::error(id, &e);
            let requested = args
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let text = response["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or("")
                .to_owned();
            response["result"]["content"][0]["text"] = json!(format!("{text}\nProject requested: {requested}\nFreshness: unknown; no indexing or repair was performed."));
            response["result"]["_meta"]["afterimage"]["project"] =
                json!({"requested_path":requested});
            response
        }
        Err(_) => profile::error(
            id,
            &anyhow::anyhow!("TOOL_PANIC: {tool}; graph left untouched"),
        ),
    }
}

fn bounded(text: String) -> (String, bool) {
    let mut end = text.len().min(MAX_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    if let Some((position, _)) = text.match_indices('\n').nth(MAX_LINES - 1) {
        if position + 1 < text.len() {
            end = end.min(position + 1);
        }
    }
    let truncated = end < text.len();
    (text[..end].to_owned(), truncated)
}

fn evidence(args: &Value, scan_source: bool) -> Result<(Value, String)> {
    let raw = args
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("INVALID_PATH: explicit path required"))?;
    let root = helpers::resolve_project_path(raw)?;
    let root_path = std::path::Path::new(&root);
    let mut scoped = args.clone();
    scoped["path"] = json!(root);
    let prism = helpers::open_prism_read_only(&scoped)?;
    let backend = prism
        .backend()
        .ok_or_else(|| anyhow::anyhow!("graph unavailable"))?;
    // Fingerprint only the stored module hashes, never the source tree. Receipts
    // are observations of analysis stages, not proof of working-tree freshness.
    let report = infigraph_core::index_status::load(prism.storage_root())
        .ok()
        .filter(|r| {
            infigraph_core::index_status::file_fingerprint(backend)
                .is_ok_and(|hash| hash == r.indexed_file_fingerprint)
        });
    let notice = prism.coverage_notice();
    let mut sources = Vec::new();
    if let Some(id) = args.get("symbol_id").and_then(Value::as_str) {
        if let Some(symbol) = backend.find_symbol_by_id(id)? {
            sources.push(json!({"file":symbol.file,"start_line":symbol.start_line,"end_line":symbol.end_line}));
        }
    } else if let Some(file) = args.get("file").and_then(Value::as_str) {
        sources.push(json!({"file":file,"start_line":null,"end_line":null}));
    }
    let (generation, coverage) = match report {
        Some(r) => (
            json!(r.run_id),
            json!({
                "syntax":"indexed_files_recorded", "languages":r.languages,
                "resolution":r.resolution, "compiler":r.scip,
                "completeness":"not_established"
            }),
        ),
        None => (
            Value::Null,
            json!({"syntax":"unknown","compiler":"unknown","completeness":"unknown"}),
        ),
    };
    let worktree = root_path
        .ancestors()
        .find(|p| p.join(".git").exists())
        .map(|p| p.to_string_lossy().into_owned());
    let freshness = infigraph_core::snapshot::verify(&prism, scan_source);
    // Preserve the existing schema-v1 lowercase field; detailed state/CLI use
    // the explicit FRESH / STALE / UNKNOWN vocabulary.
    let state = serde_json::to_value(&freshness.state)?
        .as_str()
        .unwrap_or("UNKNOWN")
        .to_ascii_lowercase();
    let warnings = vec![notice.trim().to_owned(), freshness.reason.clone()];
    let meta = json!({
        "schema_version":1,
        "project":{"root":root,"worktree_root":worktree},
        "indexed_revision":null, "generation":generation,
        "coverage":coverage, "freshness":state, "freshness_evidence":freshness, "graph_root":prism.storage_root(), "warnings":warnings,
        "sources":sources, "error_code":null,
        "evidence_observation":"after_tool_read; not an atomic source snapshot"
    });
    let text = format!("\n\nProject: {root}\nFreshness: {} ({}). {notice}Empty graph results do not prove absence; verify current source before acting.", state, freshness.reason);
    Ok((meta, text))
}

pub(crate) fn with_evidence(
    id: &Value,
    tool: &str,
    args: &Value,
    content: String,
    read_only: bool,
) -> Value {
    let query_truncated = tool == "search_symbols" && content.contains("Warning: TRUNCATED:");
    let (mut text, truncated) = if read_only && tool != "diagnose" {
        bounded(content)
    } else {
        (content, false)
    };
    let mut meta = json!({"schema_version":1});
    if profile::READ_ONLY_TOOLS.contains(&tool) && tool != "diagnose" {
        match evidence(args, tool == "get_stats") {
            Ok((observed, fallback)) => {
                meta = observed;
                text.push_str(&fallback);
            }
            Err(e) => {
                let requested = args
                    .get("path")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                text.push_str(&format!("\n\nProject requested: {requested}\nWarning: evidence unavailable ({e}); coverage/freshness unknown. Empty graph results do not prove absence."));
                meta = json!({"schema_version":1,"project":{"requested_path":requested},"freshness":"unknown","coverage":"unknown","warnings":["EVIDENCE_UNAVAILABLE"],"error_code":"EVIDENCE_UNAVAILABLE"});
            }
        }
    }
    // Keep diagnostics parseable JSON. Read-only diagnostics add evidence inside
    // that JSON too: essential partial/unknown warnings cannot live only in _meta.
    if read_only && tool == "diagnose" {
        meta = evidence(args, true)
            .map(|(meta, _)| meta)
            .unwrap_or_else(|_| {
                json!({
                    "schema_version":1,"project":{"root":args["path"]},
                    "freshness":"unknown","coverage":"unknown",
                    "warnings":["Analysis coverage unknown; no valid local graph receipt."],
                    "error_code":"EVIDENCE_UNAVAILABLE"
                })
            });
        if let Ok(mut report) = serde_json::from_str::<Value>(&text) {
            report["analysis_evidence"] = meta.clone();
            text = serde_json::to_string_pretty(&report).unwrap_or(text);
        }
    }
    if tool != "diagnose" {
        meta["limits"] = json!({
            "max_output_lines":if read_only {Some(MAX_LINES)} else {None},
            "max_output_bytes":if read_only {Some(MAX_BYTES)} else {None},
            "output_truncated":if read_only {Some(truncated)} else {None}, "query_truncated":if read_only {Some(query_truncated)} else {None},
            "result_limit":args.get("limit").cloned().or_else(|| (tool == "search_symbols").then(|| json!(10)))
        });
        if truncated {
            text.push_str("\nWarning: TRUNCATED: output exceeded 200 lines or 32768 bytes; narrow the query.\n");
        }
    }
    json!({"jsonrpc":"2.0","id":id,"result":{
        "content":[{"type":"text","text":text}], "_meta":{"afterimage":meta}
    }})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_bound_preserves_utf8_and_reports_overflow() {
        let (text, cut) = bounded("é".repeat(MAX_BYTES));
        assert!(cut);
        assert_eq!(text.len(), MAX_BYTES);
        let (text, cut) = bounded("line\n".repeat(MAX_LINES + 10));
        assert!(cut);
        assert!(text.lines().count() <= MAX_LINES);
        let response = with_evidence(
            &json!(1),
            "get_stats",
            &json!({}),
            "x\n".repeat(MAX_LINES + 5),
            true,
        );
        assert_eq!(
            response["result"]["_meta"]["afterimage"]["limits"]["output_truncated"],
            true
        );
        let visible = response["result"]["content"][0]["text"].as_str().unwrap();
        assert!(visible.contains("TRUNCATED:") && visible.contains("coverage/freshness unknown"));
    }
}

//! Opt-in policy at the existing MCP dispatch boundary; no additional server.
use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpProfile {
    Normal,
    ReadOnly,
}

impl McpProfile {
    pub fn from_env() -> Result<Self> {
        match std::env::var("INFIGRAPH_MCP_PROFILE") {
            Err(std::env::VarError::NotPresent) => Ok(Self::Normal),
            Ok(value) if value == "normal" => Ok(Self::Normal),
            Ok(value) if value == "read-only" => Ok(Self::ReadOnly),
            _ => bail!("INVALID_PROFILE: INFIGRAPH_MCP_PROFILE must be normal or read-only"),
        }
    }
}

/// Fail closed for startup side effects, including a misspelled profile.
pub fn permits_side_effects() -> bool {
    matches!(McpProfile::from_env(), Ok(McpProfile::Normal))
}

pub const READ_ONLY_TOOLS: &[&str] = &[
    "diagnose",
    "search_symbols",
    "get_symbols_in_file",
    "get_code_snippet",
    "symbol_context",
    "trace_callers",
    "trace_callees",
    "transitive_impact",
    "get_file_deps",
    "get_stats",
];

pub fn validate(tool: &str, args: &Value, profile: McpProfile) -> Result<()> {
    if profile == McpProfile::ReadOnly {
        ensure!(
            READ_ONLY_TOOLS.contains(&tool),
            "READ_ONLY_TOOL_DENIED: {tool} is unavailable in the read-only profile"
        );
        ensure!(
            std::env::var("INFIGRAPH_BACKEND").map_or(true, |v| v == "kuzu"),
            "READ_ONLY_BACKEND_UNSUPPORTED: this profile supports existing local graphs only"
        );
    }
    let definition = crate::build_tools_list()
        .into_iter()
        .find(|t| t["name"] == tool)
        .with_context(|| format!("UNKNOWN_TOOL: {tool}"))?;
    let args = args
        .as_object()
        .context("INVALID_ARGUMENTS: arguments must be an object")?;
    let properties = definition["inputSchema"]["properties"].as_object();
    for (key, value) in args {
        let known = properties.is_some_and(|p| p.contains_key(key));
        if !known && key != "detail" {
            let path_like = [
                "path", "paht", "pat", "root", "repo", "project", "cwd", "dir",
            ]
            .iter()
            .any(|part| key.contains(part))
                || value
                    .as_str()
                    .is_some_and(|v| v.contains('/') || v.contains('\\') || v.starts_with('.'));
            ensure!(profile != McpProfile::ReadOnly && !path_like,
                "UNKNOWN_ARGUMENT: {key} is not accepted by {tool}; use the advertised schema (project directory is 'path')");
        }
    }
    if profile == McpProfile::ReadOnly {
        if let Some(required) = definition["inputSchema"]["required"].as_array() {
            for key in required.iter().filter_map(Value::as_str) {
                ensure!(args.contains_key(key), "INVALID_ARGUMENTS: missing '{key}'");
            }
        }
        if let Some(limit) = args.get("limit") {
            ensure!(
                limit.as_u64().is_some_and(|n| (1..=1000).contains(&n)),
                "INVALID_ARGUMENTS: limit must be an integer from 1 to 1000"
            );
        }
    }
    if let Some(path) = args.get("path") {
        ensure!(
            path.as_str().is_some_and(|s| !s.trim().is_empty()),
            "INVALID_PATH: path must be a nonempty string"
        );
    }
    Ok(())
}

/// The ten curated tools all use read-only graph opens. Symbol context must not
/// open the session store. Search uses the existing in-memory BM25 mechanism,
/// avoiding embedding/model/HNSW caches, downloads, and opportunistic watchers.
pub(crate) fn dispatch(tool: &str, args: &Value) -> Result<String> {
    match tool {
        "symbol_context" => crate::tools::graph::symbol_context(args, false),
        "search_symbols" => crate::tools::search::search_symbols_read_only(args),
        _ => crate::dispatch_tool_inner(tool, args),
    }
}

pub(crate) fn error(id: &Value, error: &anyhow::Error) -> Value {
    let message = error.to_string();
    let prefix = message.split(':').next().unwrap_or("");
    let code = if [
        "INVALID_PROFILE",
        "READ_ONLY_TOOL_DENIED",
        "READ_ONLY_BACKEND_UNSUPPORTED",
        "UNKNOWN_TOOL",
        "INVALID_ARGUMENTS",
        "UNKNOWN_ARGUMENT",
        "INVALID_PATH",
        "NOT_INDEXED",
        "PATH_ESCAPE",
        "TOOL_PANIC",
    ]
    .contains(&prefix)
    {
        prefix
    } else {
        "TOOL_FAILED"
    };
    json!({"jsonrpc":"2.0", "id":id, "result":{
        "isError":true, "content":[{"type":"text", "text":format!("Error: {error:#}")}],
        "_meta":{"afterimage":{"schema_version":1,"error_code":code,"freshness":"unknown"}}
    }})
}

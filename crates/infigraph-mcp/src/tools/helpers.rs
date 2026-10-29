use std::path::PathBuf;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use infigraph_core::Infigraph;
use infigraph_languages::bundled_registry;

pub use super::session::{session_date_id, session_epoch};

/// Resolve only within the requested checkout. Never guess a registry child or
/// walk past a Git boundary (linked worktrees have a `.git` file).
pub fn resolve_project_path(path: &str) -> Result<String> {
    anyhow::ensure!(
        !path.trim().is_empty(),
        "INVALID_PATH: path must not be empty"
    );
    let start = PathBuf::from(path)
        .canonicalize()
        .with_context(|| format!("INVALID_PATH: project path does not exist: {path}"))?;
    anyhow::ensure!(start.is_dir(), "INVALID_PATH: expected a project directory");
    for current in start.ancestors() {
        let index = current.join(".infigraph");
        if index.join("graph").exists() && !index.join("registry.json").exists() {
            validate_local_index(current)?;
            return Ok(current.to_string_lossy().into_owned());
        }
        if current.join(".git").exists() {
            return Ok(current.to_string_lossy().into_owned());
        }
    }
    Ok(start.to_string_lossy().into_owned())
}

/// A symlinked index can belong to another checkout, even inside the same repo.
fn validate_local_index(root: &std::path::Path) -> Result<()> {
    for path in [root.join(".infigraph"), root.join(".infigraph/graph")] {
        anyhow::ensure!(
            path.canonicalize()? == path,
            "PATH_ESCAPE: index must belong to the requested checkout"
        );
    }
    Ok(())
}

pub fn require_local_index(root: &std::path::Path) -> Result<()> {
    anyhow::ensure!(root.join(".infigraph/graph").exists(),
        "NOT_INDEXED: {} has no local graph. Explicitly index this checkout outside read-only mode; no parent or sibling graph was used.", root.display());
    validate_local_index(root)
}

/// Validate a source path against the selected project, including symlinks.
pub fn project_file(root: &std::path::Path, file: &str) -> Result<PathBuf> {
    let path = root
        .join(file)
        .canonicalize()
        .with_context(|| format!("INVALID_PATH: source file unavailable: {file}"))?;
    anyhow::ensure!(
        path.starts_with(root),
        "PATH_ESCAPE: source file is outside the selected project"
    );
    Ok(path)
}

pub fn open_prism(args: &Value) -> Result<Infigraph> {
    let raw_path = args
        .get("path")
        .and_then(|p| p.as_str())
        .context("missing 'path' argument")?;
    let path = resolve_project_path(raw_path)?;
    let registry = bundled_registry()?;
    let mut prism = Infigraph::open(&PathBuf::from(&path), registry)?;
    prism.init()?;
    apply_repo_filter(&mut prism, &path);
    Ok(prism)
}

pub fn open_prism_read_only(args: &Value) -> Result<Infigraph> {
    let raw_path = args
        .get("path")
        .and_then(|p| p.as_str())
        .context("missing 'path' argument")?;
    let path = resolve_project_path(raw_path)?;
    if std::env::var("INFIGRAPH_BACKEND").map_or(true, |v| v == "kuzu") {
        require_local_index(std::path::Path::new(&path))?;
    }
    let registry = bundled_registry()?;
    let mut prism = Infigraph::open(&PathBuf::from(&path), registry)?;
    prism.init_read_only()?;
    apply_repo_filter(&mut prism, &path);
    Ok(prism)
}

/// In Neo4j (remote) mode, scope read queries to the repo matching this path.
/// Read and write MUST agree on the `org/repo` key or repo-scoped queries return
/// nothing (files/symbols show 0 while global folders/contains stay populated).
///
/// The group registry is the source of truth for a repo's `org/repo` identity, so
/// resolve from it first. Only fall back to deriving from `INFIGRAPH_ORG` + directory
/// name when the path isn't registered — that fallback is guaranteed to match the
/// write key only when the env org equals the group's org, which is exactly why the
/// registry lookup is preferred.
#[cfg(feature = "remote")]
fn apply_repo_filter(prism: &mut Infigraph, raw_path: &str) {
    if std::env::var("INFIGRAPH_BACKEND").as_deref() != Ok("neo4j") {
        return;
    }
    let path = std::path::Path::new(raw_path);
    if let Ok(reg) = infigraph_core::multi::Registry::load() {
        if let Some(ns) = reg.resolve_repo_namespace(path) {
            prism.set_repo_filter(&ns);
            return;
        }
    }
    let repo_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| raw_path.to_string());
    let org = infigraph_core::multi::default_org();
    let key = if org.is_empty() {
        repo_name
    } else {
        format!("{org}/{repo_name}")
    };
    prism.set_repo_filter(&key);
}

#[cfg(not(feature = "remote"))]
fn apply_repo_filter(_prism: &mut Infigraph, _raw_path: &str) {}

pub fn find_infigraph_cli() -> Option<std::path::PathBuf> {
    let bin_name = if cfg!(windows) {
        "infigraph.exe"
    } else {
        "infigraph"
    };

    // Check same directory as this binary first, then parent (target/debug/deps/ → target/debug/)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(bin_name);
            if sibling.exists() {
                return Some(sibling);
            }
            if let Some(parent) = dir.parent() {
                let in_parent = parent.join(bin_name);
                if in_parent.exists() {
                    return Some(in_parent);
                }
            }
        }
    }
    // Fall back to PATH
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(out) = std::process::Command::new(which_cmd)
        .arg("infigraph")
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(std::path::PathBuf::from(path));
            }
        }
    }
    None
}

pub fn find_containing_symbol<'a>(
    intervals: &'a [(&str, usize, usize, &str)],
    file: &str,
    line: usize,
) -> Option<&'a str> {
    intervals.iter().find_map(|(f, start, end, id)| {
        if *f == file && *start <= line && line <= *end {
            Some(*id)
        } else {
            None
        }
    })
}

pub fn save_analysis(path: &str, tool_name: &str, content: &str) -> Result<String> {
    let root = PathBuf::from(path);
    let dir = root.join(".infigraph").join("sessions").join("analysis");
    std::fs::create_dir_all(&dir)?;

    let date = session_date_id().replace("session_", "");
    let filename = format!("{tool_name}_{date}.md");
    let filepath = dir.join(&filename);
    std::fs::write(&filepath, content)?;

    let lines = content.lines().count();
    let summary: String = content.lines().take(5).collect::<Vec<_>>().join("\n");
    Ok(format!(
        "Saved to {}\n({} lines, {} bytes)\n\n{}",
        filepath.display(),
        lines,
        content.len(),
        summary
    ))
}

pub fn log_activity(tool_name: &str, args: &Value) {
    if matches!(
        tool_name,
        "get_latest_session"
            | "save_session"
            | "search_sessions"
            | "purge_sessions"
            | "list_projects"
    ) {
        return;
    }
    let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
    if path.is_empty() {
        return;
    }
    let sessions_dir = PathBuf::from(path).join(".infigraph").join("sessions");
    if std::fs::create_dir_all(&sessions_dir).is_err() {
        return;
    }
    let date = session_date_id().replace("session_", "");
    let log_path = sessions_dir.join(format!("activity_{date}.jsonl"));
    let ts = session_epoch();
    let mut key_args = serde_json::Map::new();
    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            if k == "path" {
                continue;
            }
            if let Some(s) = v.as_str() {
                let truncated = if s.len() > 120 { &s[..120] } else { s };
                key_args.insert(k.clone(), json!(truncated));
            }
        }
    }
    let entry = json!({"ts": ts, "tool": tool_name, "args": key_args});
    if let Ok(line) = serde_json::to_string(&entry) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(f, "{line}");
        }
    }
}

pub fn glob_matches(glob: &str, path: &str) -> bool {
    // Simple glob: * matches any sequence, ? matches one char
    let gi = glob.chars().peekable();
    let pi = path.chars().peekable();
    glob_match_inner(&gi.collect::<Vec<_>>(), &pi.collect::<Vec<_>>())
}

pub fn glob_match_inner(glob: &[char], path: &[char]) -> bool {
    match (glob.first(), path.first()) {
        (None, None) => true,
        (Some('*'), _) => {
            // ** matches path separators too; * stops at /
            let greedy = glob.first() == Some(&'*') && glob.get(1) == Some(&'*');
            if greedy {
                // try consuming 0..=n chars including /
                for i in 0..=path.len() {
                    if glob_match_inner(&glob[2..], &path[i..]) {
                        return true;
                    }
                }
                false
            } else {
                for i in 0..=path.len() {
                    if path.get(i) == Some(&'/') && i > 0 {
                        break;
                    }
                    if glob_match_inner(&glob[1..], &path[i..]) {
                        return true;
                    }
                }
                false
            }
        }
        (Some('?'), Some(_)) => glob_match_inner(&glob[1..], &path[1..]),
        (Some(g), Some(p)) if g.eq_ignore_ascii_case(p) => glob_match_inner(&glob[1..], &path[1..]),
        _ => false,
    }
}

//! Bounded lookup over indexed IDs and local workspace manifests; never a Node resolver.
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::store_util::{file_stem, import_stem, resolve_import_candidate};

pub(crate) struct ModuleImports<'a> {
    files: HashSet<&'a str>,
    by_stem: HashMap<String, Vec<&'a str>>,
    packages: HashMap<String, Vec<(String, serde_json::Value)>>,
}

impl<'a> ModuleImports<'a> {
    pub(crate) fn new(files: impl IntoIterator<Item = &'a str>) -> Self {
        let files: HashSet<_> = files.into_iter().collect();
        let mut by_stem: HashMap<String, Vec<&str>> = HashMap::new();
        for file in &files {
            by_stem.entry(file_stem(file)).or_default().push(file);
        }
        Self {
            files,
            by_stem,
            packages: HashMap::new(),
        }
    }

    /// Only inspect manifests above indexed files in declared local workspaces.
    /// No recursive directory walk, node_modules lookup, code execution or network.
    pub(crate) fn with_root(mut self, root: Option<&Path>) -> Self {
        if self.files.is_empty() {
            return self;
        }
        let Some(root) = root.and_then(|r| r.canonicalize().ok()) else {
            return self;
        };
        let mut patterns: Vec<String> = read_json(&root, "package.json")
            .and_then(|v| v.get("workspaces").cloned())
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
        if let Some(path) = bounded_file(&root, "pnpm-workspace.yaml") {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(value) = serde_yaml::from_str::<serde_json::Value>(&text) {
                    patterns.extend(
                        value["packages"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v.as_str().map(str::to_owned)),
                    );
                }
            }
        }
        let exclusions: Vec<_> = patterns
            .iter()
            .filter_map(|p| p.strip_prefix('!'))
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();
        let patterns: Vec<_> = patterns
            .iter()
            .filter(|p| !p.contains("..") && !p.starts_with('/') && !p.starts_with('!'))
            .filter_map(|p| glob::Pattern::new(p).ok())
            .collect();
        let mut dirs = HashSet::new();
        for file in &self.files {
            let mut parent = Path::new(file).parent();
            while let Some(dir) = parent.filter(|p| !p.as_os_str().is_empty()) {
                if !exclusions.iter().any(|p| p.matches(&dir.to_string_lossy()))
                    && patterns.iter().any(|p| {
                        p.matches_with(
                            &dir.to_string_lossy(),
                            glob::MatchOptions {
                                require_literal_separator: true,
                                ..Default::default()
                            },
                        )
                    })
                {
                    dirs.insert(dir.to_string_lossy().into_owned());
                }
                parent = dir.parent();
            }
        }
        for dir in dirs {
            let Some(value) = read_json(&root, &format!("{dir}/package.json")) else {
                continue;
            };
            if let Some(name) = value["name"].as_str() {
                self.packages
                    .entry(name.to_owned())
                    .or_default()
                    .push((dir, value));
            }
        }
        // A symlink outside the project must never become an indexed destination.
        self.files
            .retain(|file| bounded_file(&root, file).is_some());
        self
    }

    pub(crate) fn resolve(&self, source: &str, target: &str) -> Option<&'a str> {
        if !self.files.contains(source) {
            return None;
        }
        // Already-resolved module IDs (e.g. supplied by an extractor).
        if let Some(file) = self.files.get(target) {
            return (*file != source).then_some(*file);
        }
        let raw = target
            .strip_prefix(&format!("{source}::"))
            .unwrap_or(target);
        let name = unquote(raw);
        let ext = source.rsplit('.').next().unwrap_or("");
        let resolved = if matches!(
            ext,
            "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs"
        ) {
            if !name.starts_with("./") && !name.starts_with("../") {
                return self.resolve_package(source, name);
            }
            let parent = source.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
            let path = normalize(&format!("{parent}/{name}"))?;
            self.resolve_relative(&path)
        } else {
            // Preserve the previous final-segment lookup for non-JS languages
            // (notably Rust scoped identifiers captured with `::`).
            let legacy_name = unquote(raw.rsplit("::").next().unwrap_or(raw));
            let candidates = self.by_stem.get(&import_stem(legacy_name))?;
            resolve_import_candidate(legacy_name, candidates)
        }?;
        (resolved != source).then_some(resolved)
    }

    fn resolve_package(&self, source: &str, name: &str) -> Option<&'a str> {
        let (package, subpath) = if name.starts_with('@') {
            let first = name.find('/')?;
            name[first + 1..]
                .find('/')
                .map(|i| name.split_at(first + 1 + i))
                .unwrap_or((name, ""))
        } else {
            name.find('/')
                .map(|i| name.split_at(i))
                .unwrap_or((name, ""))
        };
        let entries = self.packages.get(package)?;
        let [(dir, manifest)] = entries.as_slice() else {
            return None;
        };
        let exports = manifest.get("exports")?;
        let key = if subpath.is_empty() {
            ".".to_owned()
        } else {
            format!(".{subpath}")
        };
        let value = if exports.is_string() && key == "." {
            exports
        } else {
            exports.get(&key)?
        };
        // Conditions, arrays, wildcards, absent and duplicate names remain unresolved.
        let target = value.as_str()?.strip_prefix("./")?;
        if target
            .split('/')
            .any(|p| matches!(p, ".." | "node_modules"))
            || target.contains('*')
        {
            return None;
        }
        let path = normalize(&format!("{dir}/{target}"))?;
        self.resolve_relative(&path).filter(|f| *f != source)
    }

    fn resolve_relative(&self, path: &str) -> Option<&'a str> {
        let ext = path.rsplit('/').next()?.rsplit_once('.').map(|(_, e)| e);
        let candidates = match ext {
            Some("js") => vec![
                format!("{}.ts", &path[..path.len() - 3]),
                format!("{}.tsx", &path[..path.len() - 3]),
                format!("{}.d.ts", &path[..path.len() - 3]),
            ],
            Some("mjs") => vec![
                format!("{}.mts", &path[..path.len() - 4]),
                format!("{}.d.mts", &path[..path.len() - 4]),
            ],
            Some("cjs") => vec![
                format!("{}.cts", &path[..path.len() - 4]),
                format!("{}.d.cts", &path[..path.len() - 4]),
            ],
            Some(_) => return self.files.get(path).copied(),
            None => ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"]
                .iter()
                .map(|ext| format!("{path}.{ext}"))
                .collect(),
        };
        let matches: Vec<_> = candidates
            .iter()
            .filter_map(|p| self.files.get(p.as_str()).copied())
            .collect();
        // Ambiguous source alternatives are intentionally unresolved.
        if !matches.is_empty() {
            return (matches.len() == 1).then(|| matches[0]);
        }
        if ext.is_some() {
            return self.files.get(path).copied();
        }
        let indexes: Vec<_> = ["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs"]
            .iter()
            .filter_map(|ext| {
                self.files
                    .get(format!("{path}/index.{ext}").as_str())
                    .copied()
            })
            .collect();
        (indexes.len() == 1).then(|| indexes[0])
    }
}

fn bounded_file(root: &Path, relative: &str) -> Option<std::path::PathBuf> {
    if Path::new(relative).is_absolute()
        || relative
            .split('/')
            .any(|p| matches!(p, ".." | "node_modules"))
    {
        return None;
    }
    let path = root.join(relative).canonicalize().ok()?;
    (path.starts_with(root) && path.is_file()).then_some(path)
}

fn read_json(root: &Path, relative: &str) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(bounded_file(root, relative)?).ok()?).ok()
}

fn unquote(raw: &str) -> &str {
    for quote in ['\'', '"'] {
        if let Some(inner) = raw.strip_prefix(quote).and_then(|s| s.strip_suffix(quote)) {
            return inner;
        }
    }
    raw
}

fn normalize(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p),
        }
    }
    Some(parts.join("/"))
}

/// Read the persisted IDs on the caller's connection (also valid in a write transaction).
pub(crate) fn module_files(conn: &kuzu::Connection<'_>) -> anyhow::Result<Vec<String>> {
    Ok(conn
        .query("MATCH (m:Module) RETURN m.id")?
        .map(|row| row[0].to_string())
        .collect())
}

pub(crate) fn inbound_imports(
    conn: &kuzu::Connection<'_>,
    changed: &HashSet<&str>,
) -> anyhow::Result<Vec<(String, String)>> {
    Ok(conn
        .query("MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.id, b.id")?
        .map(|row| (row[0].to_string(), row[1].to_string()))
        .filter(|(a, b)| !changed.contains(a.as_str()) && changed.contains(b.as_str()))
        .collect())
}

/// Preserve incoming syntax call edges when unchanged consumers aren't re-extracted.
pub(crate) fn inbound_calls(
    conn: &kuzu::Connection<'_>,
    changed: &HashSet<&str>,
) -> anyhow::Result<Vec<(String, String)>> {
    Ok(conn
        .query("MATCH (a:Symbol)-[:CALLS]->(b:Symbol) RETURN a.id, b.id, a.file, b.file")?
        .filter(|row| {
            !changed.contains(row[2].to_string().as_str())
                && changed.contains(row[3].to_string().as_str())
        })
        .map(|row| (row[0].to_string(), row[1].to_string()))
        .collect())
}

pub(crate) fn merge_calls(
    conn: &kuzu::Connection<'_>,
    pairs: &[(String, String)],
) -> anyhow::Result<()> {
    for (a, b) in pairs {
        conn.query(&format!("MATCH (a:Symbol), (b:Symbol) WHERE a.id = '{}' AND b.id = '{}' AND b.kind IN ['Function', 'Method', 'Test'] AND (NOT (a.language IN ['typescript', 'tsx', 'javascript']) OR b.visibility = 'public') MERGE (a)-[:CALLS]->(b)", super::store_util::escape(a), super::store_util::escape(b)))?;
    }
    Ok(())
}

pub(crate) fn merge_imports(
    conn: &kuzu::Connection<'_>,
    pairs: &[(String, String)],
) -> anyhow::Result<()> {
    for chunk in pairs.chunks(500) {
        let rows: Vec<_> = chunk
            .iter()
            .map(|(a, b)| {
                format!(
                    "{{a: '{}', b: '{}'}}",
                    super::store_util::escape(a),
                    super::store_util::escape(b)
                )
            })
            .collect();
        conn.query(&format!("UNWIND [{}] AS p MATCH (a:Module), (b:Module) WHERE a.id = p.a AND b.id = p.b MERGE (a)-[:IMPORTS]->(b)", rows.join(", ")))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ModuleImports;

    #[test]
    fn non_javascript_imports_keep_dotted_module_resolution() {
        let modules = ModuleImports::new([
            "app/service/constants.py",
            "app/test/constants.py",
            "main.py",
        ]);
        assert_eq!(
            modules.resolve("main.py", "main.py::app.service.constants"),
            Some("app/service/constants.py")
        );
        assert_eq!(modules.resolve("main.py", "main.py::constants"), None);
    }

    #[test]
    fn non_javascript_scoped_identifiers_keep_existing_lookup() {
        let modules = ModuleImports::new(["main.rs", "src/helpers.rs"]);
        assert_eq!(
            modules.resolve("main.rs", "main.rs::crate::helpers"),
            Some("src/helpers.rs")
        );
    }

    #[test]
    fn explicit_ids_and_javascript_sources_are_supported_without_basename_guessing() {
        let modules = ModuleImports::new([
            "main.ts",
            "a/helper.ts",
            "b/helper.ts",
            "a/helper.js",
            "a/other.jsx",
        ]);
        assert_eq!(
            modules.resolve("main.ts", "a/helper.ts"),
            Some("a/helper.ts")
        );
        assert_eq!(
            modules.resolve("main.ts", "main.ts::'./a/helper.js'"),
            Some("a/helper.ts")
        );
        assert_eq!(
            modules.resolve("main.ts", "main.ts::\"./a/other.jsx\""),
            Some("a/other.jsx")
        );
        assert_eq!(modules.resolve("main.ts", "main.ts::'@pkg/helper'"), None);
        assert_eq!(
            modules.resolve("main.ts", "main.ts::'./missing/helper'"),
            None
        );
    }
}

#[cfg(test)]
mod workspace_tests {
    use super::ModuleImports;

    #[test]
    fn bounded_workspace_resolution_rejects_ambiguous_and_external_sources() {
        let root = tempfile::tempdir().unwrap();
        let write = |path: &str, content: &str| {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        };
        write("package.json", r#"{"workspaces":["packages/*"]}"#);
        write("app.ts", "");
        write("packages/one/src/index.ts", "");
        write(
            "packages/one/package.json",
            r#"{"name":"@test/shared","exports":{".":"./src/index.ts","./conditional":{"default":"./src/index.ts"},"./array":["./src/index.ts"],"./missing":"./absent.ts","./escape":"../../app.ts","./link":"./src/link.ts","./*":"./src/*.ts"}}"#,
        );
        let files = [
            "app.ts",
            "packages/one/src/index.ts",
            "packages/one/src/link.ts",
        ];
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("external.ts"), "").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            outside.path().join("external.ts"),
            root.path().join("packages/one/src/link.ts"),
        )
        .unwrap();
        let modules = ModuleImports::new(files).with_root(Some(root.path()));
        assert_eq!(modules.resolve("app.ts", "'@test/shared'"), Some(files[1]));
        for target in [
            "'@test/shared/conditional'",
            "'@test/shared/array'",
            "'@test/shared/missing'",
            "'@test/shared/escape'",
            "'@test/shared/link'",
            "'@test/shared/unknown'",
            "'./packages/one/src/link.ts'",
            "'../../app.ts'",
            "'missing'",
        ] {
            assert_eq!(modules.resolve("app.ts", target), None, "{target}");
        }
        write("packages/two/src/index.ts", "");
        write(
            "packages/two/package.json",
            r#"{"name":"@test/shared","exports":"./src/index.ts"}"#,
        );
        let modules = ModuleImports::new(files.into_iter().chain(["packages/two/src/index.ts"]))
            .with_root(Some(root.path()));
        assert_eq!(modules.resolve("app.ts", "'@test/shared'"), None);
    }
}

/// Preserve incoming JSX references when only their target module is reindexed.
pub(crate) fn inbound_jsx(
    conn: &kuzu::Connection<'_>,
    changed: &HashSet<&str>,
) -> anyhow::Result<Vec<(String, String)>> {
    Ok(conn
        .query("MATCH (a:Symbol)-[:JSX_USES]->(b:Symbol) RETURN a.id, b.id, a.file, b.file")?
        .filter(|row| {
            !changed.contains(row[2].to_string().as_str())
                && changed.contains(row[3].to_string().as_str())
        })
        .map(|row| (row[0].to_string(), row[1].to_string()))
        .collect())
}

pub(crate) fn merge_jsx(
    conn: &kuzu::Connection<'_>,
    pairs: &[(String, String)],
) -> anyhow::Result<()> {
    for (a, b) in pairs {
        conn.query(&format!("MATCH (a:Symbol), (b:Symbol) WHERE a.id = '{}' AND b.id = '{}' AND b.visibility = 'public' AND b.kind IN ['Function', 'Variable', 'Test'] MERGE (a)-[:JSX_USES]->(b)", super::store_util::escape(a), super::store_util::escape(b)))?;
    }
    Ok(())
}

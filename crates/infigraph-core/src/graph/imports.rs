//! Bounded module lookup over indexed file IDs; no filesystem or Node resolver.
use std::collections::{HashMap, HashSet};

use super::store_util::{file_stem, import_stem, resolve_import_candidate};

pub(crate) struct ModuleImports<'a> {
    files: HashSet<&'a str>,
    by_stem: HashMap<String, Vec<&'a str>>,
}

impl<'a> ModuleImports<'a> {
    pub(crate) fn new(files: impl IntoIterator<Item = &'a str>) -> Self {
        let files: HashSet<_> = files.into_iter().collect();
        let mut by_stem: HashMap<String, Vec<&str>> = HashMap::new();
        for file in &files {
            by_stem.entry(file_stem(file)).or_default().push(file);
        }
        Self { files, by_stem }
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
            // Bare packages, aliases and URLs need project/compiler metadata.
            // Never guess by basename for JS/TS imports.
            if !name.starts_with("./") && !name.starts_with("../") {
                return None;
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

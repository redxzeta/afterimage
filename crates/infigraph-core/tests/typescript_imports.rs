//! Module-edge regressions: assert stored endpoints, independently of UI projections.
use infigraph_core::{
    extract::extract_file,
    graph::{GraphBackend, KuzuBackend},
    model::{FileExtraction, RelationKind},
};
use infigraph_languages::bundled_registry;

fn extract(file: &str, source: &str) -> FileExtraction {
    let registry = bundled_registry().unwrap();
    extract_file(file, source.as_bytes(), registry.for_file(file).unwrap()).unwrap()
}

fn assert_import(backend: &KuzuBackend, from: &str, to: &str) {
    for property in ["file", "id"] {
        assert_eq!(
            backend.raw_query(&format!(
                "MATCH (a:Module)-[:IMPORTS]->(b:Module) WHERE a.{property} = '{from}' AND b.{property} = '{to}' RETURN a.{property}, b.{property}"
            )).unwrap(),
            vec![vec![from.to_string(), to.to_string()]],
            "missing or duplicated Module IMPORTS edge by {property}: {from} -> {to}"
        );
    }
}

#[test]
fn two_file_extensionless_import_has_module_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let backend = KuzuBackend::open(&dir.path().join("graph")).unwrap();
    let files = [
        extract("helpers.ts", "export function helper() { return 1; }"),
        extract(
            "main.ts",
            "import { helper } from './helpers'; export function main() { return helper(); }",
        ),
    ];
    // A captured quoted specifier proves parsing succeeded even if storage loses it.
    assert!(files[1]
        .relations
        .iter()
        .any(|r| r.kind == RelationKind::Imports));
    backend.upsert_files_bulk(&files, true).unwrap();
    assert_import(&backend, "main.ts", "helpers.ts");
}

#[test]
fn relative_imports_and_reexports_resolve_paths_not_basenames() {
    for (file, statement, target) in [
        (
            "src/main.ts",
            "import { helper } from './helpers'",
            "src/helpers.ts",
        ),
        (
            "src/main.ts",
            "import { helper } from './helpers.js'",
            "src/helpers.ts",
        ),
        (
            "src/main.ts",
            "import type { Helper } from './helpers.ts'",
            "src/helpers.ts",
        ),
        ("src/main.ts", "import './helpers.js'", "src/helpers.ts"),
        (
            "src/main.ts",
            "export { helper } from './helpers'",
            "src/helpers.ts",
        ),
        (
            "src/main.ts",
            "export * from './helpers.js'",
            "src/helpers.ts",
        ),
        (
            "src/main.ts",
            "export type { Helper } from './helpers.ts'",
            "src/helpers.ts",
        ),
        (
            "src/main.tsx",
            "export * from './helpers.js'",
            "src/helpers.ts",
        ),
        (
            "src/main.js",
            "export * from './helpers.js'",
            "src/helpers.ts",
        ),
        (
            "src/main.ts",
            "import { helper } from '../lib/./helpers.js'",
            "lib/helpers.ts",
        ),
        (
            "src/main.ts",
            "import { helper } from './nested'",
            "src/nested/index.ts",
        ),
        (
            "src/main.mts",
            "export * from './helpers.mjs'",
            "src/helpers.mts",
        ),
        (
            "src/main.cts",
            "export * from './helpers.cjs'",
            "src/helpers.cts",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let backend = KuzuBackend::open(&dir.path().join("graph")).unwrap();
        let files = [
            extract(file, statement),
            extract(
                target,
                "export const helper = 1; export type Helper = number;",
            ),
            extract("unrelated/helpers.ts", "export const helper = 2;"),
        ];
        backend.upsert_files_bulk(&files, true).unwrap();
        assert_import(&backend, file, target);
        assert_eq!(
            backend.get_file_deps(file).unwrap().imports,
            vec![target.to_string()]
        );
        // Update target only: incoming imports from unchanged files must survive.
        backend.upsert_files_bulk(&files[1..2], false).unwrap();
        assert_import(&backend, file, target);
        // Update both, importer first: resolve after all Module nodes exist.
        backend.upsert_files_bulk(&files[..2], false).unwrap();
        assert_import(&backend, file, target);
        // Single-file API also resolves against persisted modules.
        backend.upsert_file(&files[0]).unwrap();
        assert_import(&backend, file, target);
        backend.upsert_file(&files[1]).unwrap();
        assert_import(&backend, file, target);
    }
}

#[test]
fn unsupported_packages_and_missing_or_ambiguous_paths_are_not_guessed() {
    let dir = tempfile::tempdir().unwrap();
    let backend = KuzuBackend::open(&dir.path().join("graph")).unwrap();
    let files = [
        extract(
            "src/main.ts",
            r#"
            import '@workspace/helpers';
            import '@workspace/helpers/subpath';
            import 'helpers';
            import './missing/helpers.js';
            import './Helpers.js';
            import './ambiguous.js';
            import '../../helpers.js';
            import './main.ts';
        "#,
        ),
        extract("src/helpers.ts", "export const helper = 1;"),
        extract("src/ambiguous.ts", "export const helper = 1;"),
        extract("src/ambiguous.tsx", "export const helper = 1;"),
        extract("packages/helpers/src/index.ts", "export const helper = 1;"),
    ];
    backend.upsert_files_bulk(&files, true).unwrap();
    assert!(backend
        .raw_query("MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.id, b.id")
        .unwrap()
        .is_empty());
}

#[test]
fn bulk_unwind_and_large_incremental_parquet_keep_imports() {
    use infigraph_core::graph::GraphStore;
    let files = [
        extract("main.ts", "export * from './helpers.js';"),
        extract("helpers.ts", "export const helper = 1;"),
    ];
    let dir = tempfile::tempdir().unwrap();
    let store = GraphStore::open(&dir.path().join("graph")).unwrap();
    {
        let _lock = store.write_lock().unwrap();
        store
            .upsert_all_bulk(&store.connection().unwrap(), &files)
            .unwrap();
    }
    let backend = KuzuBackend::from_store(store);
    assert_import(&backend, "main.ts", "helpers.ts");
    let mut changed = vec![files[1].clone()];
    for n in 0..100 {
        changed.push(extract(&format!("filler{n}.ts"), "export const value = 1;"));
    }
    backend.upsert_files_bulk(&changed, false).unwrap();
    assert_import(&backend, "main.ts", "helpers.ts");
}

#[test]
fn syntax_only_workspace_exports_are_explicitly_unresolved() {
    use infigraph_core::Infigraph;
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("packages/helpers/src")).unwrap();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"private":true,"workspaces":["packages/*"]}"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("packages/helpers/package.json"), r#"{"name":"@workspace/helpers","exports":{".":"./src/index.ts","./subpath":"./src/subpath.ts"}}"#).unwrap();
    std::fs::write(
        dir.path().join("packages/helpers/src/index.ts"),
        "export const helper = 1;",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("packages/helpers/src/subpath.ts"),
        "export const subpath = 1;",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("main.ts"),
        "import '@workspace/helpers'; import '@workspace/helpers/subpath';",
    )
    .unwrap();
    let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    graph.init().unwrap();
    graph.index().unwrap(); // Syntax path only; no compiler, package manager or app runs.
    assert!(graph
        .backend()
        .unwrap()
        .get_file_deps("main.ts")
        .unwrap()
        .imports
        .is_empty());
}

#[test]
fn scip_enrichment_preserves_syntax_module_imports() {
    use protobuf::Message;
    use scip::types::{Document, Index, Occurrence, SymbolRole};
    let dir = tempfile::tempdir().unwrap();
    let backend = KuzuBackend::open(&dir.path().join("graph")).unwrap();
    let files = [
        extract("main.ts", "import { helper } from './helpers.js';"),
        extract("helpers.ts", "export const helper = 1;"),
    ];
    backend.upsert_files_bulk(&files, true).unwrap();
    let mut index = Index::new();
    for (file, roles) in [
        ("helpers.ts", SymbolRole::Definition as i32),
        ("main.ts", SymbolRole::Import as i32),
    ] {
        let mut doc = Document::new();
        doc.relative_path = file.into();
        let mut occ = Occurrence::new();
        occ.symbol = "scip-typescript npm fixture 0.0.0 `helpers.ts`/helper.".into();
        occ.symbol_roles = roles;
        occ.range = vec![0, 13, 19];
        doc.occurrences.push(occ);
        index.documents.push(doc);
    }
    let artifact = dir.path().join("index.scip");
    std::fs::write(&artifact, index.write_to_bytes().unwrap()).unwrap();
    for _ in 0..2 {
        let stats =
            infigraph_core::scip::import_scip_index(&artifact, backend.inner(), None).unwrap();
        assert_eq!(stats.files_processed, 2);
        assert_import(&backend, "main.ts", "helpers.ts");
    }
}

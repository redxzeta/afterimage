use infigraph_core::extract::extract_file;
use infigraph_core::model::{RelationKind, SymbolKind};
use infigraph_languages::bundled_registry;

#[test]
fn variable_bound_typescript_and_tsx_functions_are_callable() {
    let registry = bundled_registry().unwrap();
    let source = br#"
        export const refresh = (cwd: string) => fetchUpstream(cwd);
        const expression = function () { refresh("repo"); };
        const generator = function* () { refresh("repo"); };
        const ordinary = 42;
    "#;
    for file in ["status.ts", "status.tsx"] {
        let pack = registry.for_file(file).unwrap();
        let extraction = extract_file(file, source, pack).unwrap();
        for name in ["refresh", "expression", "generator"] {
            let symbols: Vec<_> = extraction
                .symbols
                .iter()
                .filter(|s| s.name == name)
                .collect();
            assert_eq!(symbols.len(), 1, "{file}: duplicate {name}");
            assert_eq!(symbols[0].kind, SymbolKind::Function, "{file}: {name}");
        }
        assert_eq!(
            extraction
                .symbols
                .iter()
                .find(|s| s.name == "ordinary")
                .unwrap()
                .kind,
            SymbolKind::Variable
        );
        let factory = extract_file(
            file,
            b"const factory = Effect.fn(function* callback() { target(); });",
            pack,
        )
        .unwrap();
        assert!(factory
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Calls)
            .all(|r| r.source_id == format!("{file}::{file}")));
    }
}

#[test]
fn nested_effect_callbacks_use_the_actual_named_callable_id() {
    let registry = bundled_registry().unwrap();
    let source = br#"
        const makeGitCore = () => Effect.gen(function* () {
            const refresh = (cwd: string) => Effect.gen(function* () {
                yield* fetchUpstream(cwd);
            });
            const status = () => Effect.gen(function* () {
                yield* refresh("repo");
            });
            return status;
        });
        class Client { run() { refresh("repo"); } }
    "#;
    let extraction =
        extract_file("status.ts", source, registry.for_file("status.ts").unwrap()).unwrap();
    for (caller, target) in [
        ("refresh", "fetchUpstream"),
        ("status", "refresh"),
        ("Client::run", "refresh"),
    ] {
        assert!(
            extraction
                .relations
                .iter()
                .any(|r| r.kind == RelationKind::Calls
                    && r.source_id == format!("status.ts::{caller}")
                    && r.target_id == format!("status.ts::{target}")),
            "missing {caller} -> {target}: {:?}",
            extraction.relations
        );
    }
}

#[test]
fn imported_arrow_helpers_have_callers_and_transitive_impact_after_reindex() {
    use infigraph_core::{
        index_status::{self, StageOutcome, StageState},
        Infigraph,
    };
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("fetch.ts"),
        "export const fetchUpstream = () => 1;",
    )
    .unwrap();
    std::fs::write(dir.path().join("refresh.ts"), "import { fetchUpstream } from './fetch'; export const refresh = () => Effect.gen(function* () { yield* fetchUpstream(); });").unwrap();
    std::fs::write(dir.path().join("status.ts"), "import { refresh } from './refresh'; export const status = () => Effect.gen(function* () { yield* refresh(); });").unwrap();
    let mut prism = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    assert_eq!(prism.index().unwrap().indexed_files, 3);
    let backend = prism.backend().unwrap();
    assert_eq!(
        backend.callers_of("fetch.ts::fetchUpstream").unwrap(),
        vec!["refresh.ts::refresh"]
    );
    let impacted = backend
        .transitive_impact("fetch.ts::fetchUpstream", 5)
        .unwrap();
    assert!(impacted.iter().any(|s| s.id == "status.ts::status"));
    assert_eq!(prism.index().unwrap().indexed_files, 0);
    // Simulate an existing pre-fix extraction fingerprint. The unchanged
    // source must be parsed again, not left permanently on its old graph.
    let pack = bundled_registry().unwrap();
    let mut old_files = Vec::new();
    for file in ["fetch.ts", "refresh.ts", "status.ts"] {
        let source = std::fs::read(dir.path().join(file)).unwrap();
        let mut old = extract_file(file, &source, pack.for_file(file).unwrap()).unwrap();
        old.content_hash = "old-extractor-fingerprint".into();
        old.symbols[0].kind = SymbolKind::Variable;
        old_files.push(old);
    }
    backend.upsert_files_bulk(&old_files, false).unwrap();
    assert_eq!(prism.index().unwrap().indexed_files, 3);
    assert_eq!(
        backend.callers_of("fetch.ts::fetchUpstream").unwrap(),
        vec!["refresh.ts::refresh"]
    );
    let report = index_status::load(dir.path()).unwrap();
    index_status::update_current(dir.path(), &report.run_id, Some(backend), |r| {
        r.scip.insert(
            "scip-typescript".into(),
            StageOutcome::new(StageState::Failed, "SCIP_PROCESS_FAILED"),
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    assert!(prism.coverage_notice().contains("SCIP_PROCESS_FAILED"));
    let before = std::fs::read(dir.path().join(".infigraph/index-status.json")).unwrap();
    assert!(prism.coverage_notice().contains("SCIP_PROCESS_FAILED"));
    assert_eq!(
        before,
        std::fs::read(dir.path().join(".infigraph/index-status.json")).unwrap()
    );
    assert_eq!(prism.index().unwrap().indexed_files, 0);
    assert!(index_status::load(dir.path()).unwrap().has_failures());
    assert!(
        index_status::update_current(dir.path(), &report.run_id, Some(backend), |_| Ok(()))
            .unwrap()
            .is_none()
    );
}

#[test]
fn test_callers_keep_test_classification_and_filtering() {
    let registry = bundled_registry().unwrap();
    for file in ["status.test.ts", "status.test.tsx"] {
        let extraction = extract_file(
            file,
            b"const helper = () => 1; const testStatus = () => (() => helper())();",
            registry.for_file(file).unwrap(),
        )
        .unwrap();
        assert_eq!(
            extraction
                .symbols
                .iter()
                .find(|s| s.name == "testStatus")
                .unwrap()
                .kind,
            SymbolKind::Test
        );
        assert!(extraction
            .relations
            .iter()
            .any(|r| r.kind == RelationKind::Calls
                && r.source_id == format!("{file}::testStatus")
                && r.target_id == format!("{file}::helper")));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(file),
            "const helper = () => 1; const testStatus = () => (() => helper())();",
        )
        .unwrap();
        let mut prism =
            infigraph_core::Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
        prism.init().unwrap();
        prism.index().unwrap();
        let backend = prism.backend().unwrap();
        assert_eq!(
            backend
                .callers_of_filtered(&format!("{file}::helper"), true)
                .unwrap(),
            vec![format!("{file}::testStatus")]
        );
        assert!(backend
            .callers_of_filtered(&format!("{file}::helper"), false)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn ambiguous_helpers_and_function_factories_remain_unresolved() {
    let dir = tempfile::tempdir().unwrap();
    for file in ["one.ts", "two.ts"] {
        std::fs::write(dir.path().join(file), "export const helper = () => 1;").unwrap();
    }
    std::fs::write(
        dir.path().join("caller.ts"),
        "const factoryResult = makeFactory(); export const caller = () => helper();",
    )
    .unwrap();
    let mut prism =
        infigraph_core::Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism.index().unwrap();
    let backend = prism.backend().unwrap();
    for file in ["one.ts", "two.ts"] {
        assert!(backend
            .callers_of(&format!("{file}::helper"))
            .unwrap()
            .is_empty());
    }
    assert_eq!(
        backend
            .symbols_in_file("caller.ts")
            .unwrap()
            .iter()
            .find(|s| s.name == "factoryResult")
            .unwrap()
            .kind,
        "Variable"
    );
}

#[test]
fn reindexing_unchanged_files_invalidates_compiler_enrichment() {
    use infigraph_core::{
        index_status::{self, StageOutcome, StageState},
        Infigraph,
    };
    use protobuf::Message;
    use scip::types::{Document, Index, Occurrence, SymbolRole};

    let dir = tempfile::tempdir().unwrap();
    // A compiler reference without a syntax CALLS edge (e.g. a value use).
    std::fs::write(
        dir.path().join("app.ts"),
        "export const helper = () => 1;\nexport const caller = () => {\n  return helper;\n};\n",
    )
    .unwrap();
    let mut prism = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism.index().unwrap();
    let backend = prism.backend().unwrap();
    let symbol = "scip-test npm test 1.0.0 `app.ts`/helper().";
    let artifact = dir.path().join("fixture.scip");
    let index = Index {
        documents: vec![Document {
            relative_path: "app.ts".into(),
            occurrences: vec![
                Occurrence {
                    range: vec![0, 13, 19],
                    symbol: symbol.into(),
                    symbol_roles: SymbolRole::Definition as i32,
                    ..Default::default()
                },
                Occurrence {
                    range: vec![2, 9, 15],
                    symbol: symbol.into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    std::fs::write(&artifact, index.write_to_bytes().unwrap()).unwrap();
    backend.import_scip_index(&artifact, None).unwrap();
    assert_eq!(
        backend.callers_of("app.ts::helper").unwrap(),
        vec!["app.ts::caller"]
    );
    let run = index_status::current_run(dir.path()).unwrap();
    index_status::update_current(dir.path(), &run.run_id, Some(backend), |report| {
        report.scip.insert(
            "scip-typescript".into(),
            StageOutcome::new(StageState::Succeeded, "SCIP_IMPORTED"),
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    // A genuine no-op may reuse its receipt and retain the imported edge.
    assert_eq!(prism.index().unwrap().indexed_files, 0);
    assert!(prism.coverage_notice().starts_with("Coverage:"));
    assert_eq!(
        backend.callers_of("app.ts::helper").unwrap(),
        vec!["app.ts::caller"]
    );

    // Watcher/batch indexing rewrites even an unchanged file and its symbols.
    assert_eq!(
        prism.index_files(&["app.ts".into()]).unwrap().indexed_files,
        1
    );
    assert!(backend.callers_of("app.ts::helper").unwrap().is_empty());
    let updated = index_status::current_run(dir.path()).unwrap();
    assert_eq!(
        updated.indexed_file_fingerprint,
        run.indexed_file_fingerprint
    );
    assert!(
        updated.scip.is_empty(),
        "rewritten symbols lost the imported edge; require enrichment again"
    );
    assert!(prism
        .coverage_notice()
        .contains("no compiler enrichment recorded"));
}

#[test]
fn fixed_gap_import_bindings_survive_duplicate_names_and_target_reindex() {
    let dir = tempfile::tempdir().unwrap();
    for (file, source) in [
        ("utils/canonical.ts", "export const canonicalJson = (v: unknown) => JSON.stringify(v);"),
        ("other/canonical.ts", "export const canonicalJson = () => 'wrong';"),
        ("cli/args.ts", "export function parseOptionalArg() { return 1; }"),
        ("other/args.ts", "export function parseOptionalArg() { return 2; }"),
        ("app.ts", "import { canonicalJson as encode } from './utils/canonical.js'; import { parseOptionalArg } from './cli/args.js'; const canonicalJson = () => 'local'; const json = (v: unknown) => `${encode(v)}\\n`; const jsonl = (v: unknown[]) => v.map(json).join(''); export function parse() { return parseOptionalArg(); }"),
    ] {
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let mut graph =
        infigraph_core::Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    graph.init().unwrap();
    graph.index().unwrap();
    for target in [
        vec![],
        vec!["utils/canonical.ts"],
        vec!["cli/args.ts"],
        vec!["app.ts", "utils/canonical.ts"],
        vec!["utils/canonical.ts", "app.ts"],
    ] {
        if !target.is_empty() {
            graph
                .index_files(
                    &target
                        .iter()
                        .map(std::path::PathBuf::from)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
        }
        let backend = graph.backend().unwrap();
        let actual: Vec<_> = [
            "utils/canonical.ts::canonicalJson",
            "cli/args.ts::parseOptionalArg",
        ]
        .into_iter()
        .map(|callee| backend.callers_of(callee).unwrap())
        .collect();
        assert_eq!(
            actual,
            vec![vec!["app.ts::json"], vec!["app.ts::parse"]],
            "fixed direct CALLS (2 expected), batch {target:?}"
        );
        for callee in [
            "other/canonical.ts::canonicalJson",
            "other/args.ts::parseOptionalArg",
            "app.ts::json",
            "app.ts::canonicalJson",
        ] {
            assert!(
                backend.callers_of(callee).unwrap().is_empty(),
                "invented call: {callee}"
            );
        }
    }
}

#[test]
fn import_is_not_a_call_binding_and_callback_values_are_not_calls() {
    let registry = bundled_registry().unwrap();
    for file in ["app.ts", "app.tsx", "app.js"] {
        let source = b"import { helper as imported } from './a/helper.js'; import type { phantom } from './b/helper.js'; const helper = () => 1; const local = () => helper(); const bound = () => imported(); const shadow = (imported) => imported(); const paramShadow = (helper) => helper(); const callback = (xs) => xs.map(helper); const unknown = () => phantom();";
        let source = if file.ends_with(".js") {
            String::from_utf8_lossy(source).replace("import type", "import")
        } else {
            String::from_utf8_lossy(source).into_owned()
        };
        let ext = extract_file(file, source.as_bytes(), registry.for_file(file).unwrap()).unwrap();
        let calls: Vec<_> = ext
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Calls)
            .collect();
        assert!(calls
            .iter()
            .any(|r| r.source_id == format!("{file}::local")
                && r.target_id == format!("{file}::helper")));
        assert!(calls.iter().any(|r| r.source_id == format!("{file}::bound")
            && r.target_id.ends_with("'./a/helper.js'::helper")));
        assert!(
            calls
                .iter()
                .any(|r| r.source_id == format!("{file}::shadow")
                    && r.target_id.ends_with("@unresolved::imported")),
            "{file}: {:?}",
            ext.relations
        );
        assert!(calls
            .iter()
            .any(|r| r.source_id == format!("{file}::paramShadow")
                && r.target_id.ends_with("@unresolved::helper")));
        assert!(!calls.iter().any(
            |r| r.source_id == format!("{file}::callback") && r.target_id.ends_with("::helper")
        ));
    }
}

#[test]
fn lexical_shadows_and_aliased_exports_never_select_private_same_name_functions() {
    let registry = bundled_registry().unwrap();
    for extension in ["ts", "tsx", "js"] {
        let dir = tempfile::tempdir().unwrap();
        let app = format!("app.{extension}");
        let dep = format!("dep.{extension}");
        let alias = format!("alias.{extension}");
        std::fs::write(dir.path().join(&dep), "export const helper = () => 1;").unwrap();
        std::fs::write(
            dir.path().join(&alias),
            "const helper = () => 0; const real = () => 1; export { real as helper };",
        )
        .unwrap();
        let source = "import { helper } from './dep.js'; import { helper as aliased } from './alias.js'; export function use(xs) { for (const helper of xs) helper(); for (const [helper] of xs) helper(); for (const { helper } of xs) helper(); } export const run = function helper() { return helper(); }; export const generator = function* helper() { helper(); }; export const badAlias = () => aliased(); export const good = () => helper();";
        let extraction =
            extract_file(&app, source.as_bytes(), registry.for_file(&app).unwrap()).unwrap();
        for owner in ["use", "run", "generator"] {
            assert!(
                extraction
                    .relations
                    .iter()
                    .filter(|r| r.kind == RelationKind::Calls
                        && r.source_id == format!("{app}::{owner}"))
                    .all(|r| !r.target_id.contains("'./dep.js'::")),
                "{app}: {owner} bound a shadow to the import"
            );
        }
        std::fs::write(dir.path().join(&app), source).unwrap();
        let mut graph =
            infigraph_core::Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
        graph.init().unwrap();
        graph.index().unwrap();
        let backend = graph.backend().unwrap();
        assert_eq!(
            backend.callers_of(&format!("{dep}::helper")).unwrap(),
            vec![format!("{app}::good")]
        );
        for name in ["helper", "real"] {
            assert!(
                backend
                    .callers_of(&format!("{alias}::{name}"))
                    .unwrap()
                    .is_empty(),
                "{extension}: unsupported aliased export guessed {name}"
            );
        }
        // Replacing a direct export with an unsupported alias must also drop
        // old incoming edges from an unchanged consumer.
        std::fs::write(
            dir.path().join(&dep),
            "const helper = () => 0; const real = () => 1; export { real as helper };",
        )
        .unwrap();
        graph.index_files(&[dep.clone().into()]).unwrap();
        assert!(graph
            .backend()
            .unwrap()
            .callers_of(&format!("{dep}::helper"))
            .unwrap()
            .is_empty());
    }
}

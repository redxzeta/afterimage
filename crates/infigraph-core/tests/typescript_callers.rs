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

    // Direct indexing is also a true no-op for the same source/grammar.
    prism.index_file(std::path::Path::new("app.ts")).unwrap();
    assert!(prism.coverage_notice().starts_with("Coverage:"));
    assert_eq!(
        backend.callers_of("app.ts::helper").unwrap(),
        vec!["app.ts::caller"]
    );
    let receipt_path = dir.path().join(".infigraph/index-status.json");
    let receipt_bytes = std::fs::read(&receipt_path).unwrap();
    assert!(prism
        .index_file(std::path::Path::new("missing.ts"))
        .is_err());
    assert_eq!(std::fs::read(&receipt_path).unwrap(), receipt_bytes);
    // A malformed artifact is rejected before any native graph write.
    let malformed = dir.path().join("malformed.scip");
    std::fs::write(&malformed, [0xff]).unwrap();
    assert!(backend.import_scip_index(&malformed, None).is_err());
    assert_eq!(
        backend.callers_of("app.ts::helper").unwrap(),
        vec!["app.ts::caller"]
    );
    assert_eq!(std::fs::read(&receipt_path).unwrap(), receipt_bytes);

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
fn direct_index_file_change_invalidates_success_and_obsolete_worker() {
    use infigraph_core::{
        index_status::{self, StageOutcome, StageState},
        Infigraph,
    };
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("app.ts");
    std::fs::write(&source, "export const helper = () => 1;").unwrap();
    let mut prism = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    prism.init().unwrap();
    prism.index().unwrap();
    let backend = prism.backend().unwrap();
    let run = index_status::current_run(dir.path()).unwrap().run_id;
    index_status::update_current(dir.path(), &run, Some(backend), |r| {
        r.scip.insert(
            "scip-typescript".into(),
            StageOutcome::new(StageState::Succeeded, "SCIP_IMPORTED"),
        );
        Ok(())
    })
    .unwrap()
    .unwrap();
    std::fs::write(&source, "export const helper = () => 2;").unwrap();
    prism.index_file(&source).unwrap();
    let report = index_status::current_run(dir.path()).unwrap();
    assert_ne!(report.run_id, run);
    assert!(report.scip.is_empty());
    assert!(
        index_status::update_current::<()>(dir.path(), &run, Some(backend), |_| {
            panic!("late import must not attach to changed graph")
        })
        .unwrap()
        .is_none()
    );
}

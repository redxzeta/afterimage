use infigraph_core::{
    extract::extract_file,
    model::{RelationKind, SymbolKind},
    Infigraph,
};
use infigraph_languages::bundled_registry;

#[test]
fn effect_factory_binding_and_shape_are_required() {
    let registry = bundled_registry().unwrap();
    let source = br#"
import * as Effect from 'effect/Effect';
import { Effect as E } from 'effect';
import * as unrelated from './other';
const readTask = Effect.fn("readTask")(function* (id) { helper(); });
const runOnce = Effect.fnUntraced(function* () { helper(); }, effect => effect);
const alias = E.fn("alias")(function* () { helper(); });
const value = runOnce();
const methodReference = Effect.fnUntraced;
const arbitrary = unrelated.fnUntraced(function* () { helper(); });
const noBody = Effect.fnUntraced(existing);
const curry = Effect.fn("curry");
const extraInvocation = Effect.fnUntraced(function* () {})();
const dynamicName = Effect.fn(name)(function* () {});
function helper() {}
"#;
    for file in ["app.ts", "app.tsx", "app.js"] {
        let ext = extract_file(file, source, registry.for_file(file).unwrap()).unwrap();
        for name in ["readTask", "runOnce", "alias"] {
            let s = ext.symbols.iter().find(|s| s.name == name).unwrap();
            assert_eq!(s.kind, SymbolKind::Function, "{file}: {name}");
            assert!(s.parameters.is_some());
            assert!(ext.relations.iter().any(|r| r.kind == RelationKind::Calls
                && r.source_id == format!("{file}::{name}")
                && r.target_id == format!("{file}::helper")));
        }
        for name in [
            "value",
            "methodReference",
            "arbitrary",
            "noBody",
            "curry",
            "extraInvocation",
            "dynamicName",
        ] {
            assert_eq!(
                ext.symbols.iter().find(|s| s.name == name).unwrap().kind,
                SymbolKind::Variable,
                "{file}: {name}"
            );
        }
    }
    let typed = b"import * as Effect from 'effect/Effect'; const readTask = Effect.fn(\"x\")(function* (id: Effect.Effect<string>) {}); function helper({Effect: renamed} = {}) {}";
    let ext = extract_file("typed.ts", typed, registry.for_file("typed.ts").unwrap()).unwrap();
    assert_eq!(
        ext.symbols
            .iter()
            .find(|s| s.name == "readTask")
            .unwrap()
            .kind,
        SymbolKind::Function
    );
    for import in [
        "import type\n* as Effect from 'effect/Effect';",
        "import { type\nEffect } from 'effect';",
        "import type /* comment */ * as Effect from 'effect/Effect';",
        "import type * as Effect from 'effect/Effect';",
        "import { type Effect } from 'effect';",
        "import Effect from 'effect/Effect';",
        "",
    ] {
        let source = format!("{import} const rejected = Effect.fnUntraced(function* () {{}});");
        let ext = extract_file(
            "app.ts",
            source.as_bytes(),
            registry.for_file("app.ts").unwrap(),
        )
        .unwrap();
        assert_eq!(
            ext.symbols
                .iter()
                .find(|s| s.name == "rejected")
                .unwrap()
                .kind,
            SymbolKind::Variable
        );
    }
}

#[test]
fn effect_callers_and_body_owners_survive_reindex() {
    for extension in ["ts", "tsx", "js"] {
        let dir = tempfile::tempdir().unwrap();
        let file = format!("app.{extension}");
        std::fs::write(dir.path().join(&file), r#"
import * as Effect from 'effect/Effect';
function helper() {}
const make = () => Effect.gen(function* () {
  const readTask = Effect.fn("readTask")(function* () { helper(); });
  const runOnce = Effect.fnUntraced(function* () { readTask(); outbox.claimNext(); executor.execute(); gitCore.statusDetailsRemote(); Math.max(1,2); Effect.exit(0); }, effect => effect);
  const waitForTask = () => readTask();
  return { runOnce: runOnce(), runRecoveryOnce: runOnce(true), waitForTask };
});
"#).unwrap();
        std::fs::write(dir.path().join("other.ts"), "export function claimNext() {} export function execute() {} export function statusDetailsRemote() {} export function max() {} export class Replay { exit() {} }").unwrap();
        let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
        graph.init().unwrap();
        graph.index().unwrap();
        for batch in [vec![], vec![file.clone().into()], vec!["other.ts".into()]] {
            if !batch.is_empty() {
                graph.index_files(&batch).unwrap();
            }
            let b = graph.backend().unwrap();
            let mut callers = b.callers_of(&format!("{file}::readTask")).unwrap();
            callers.sort();
            assert_eq!(
                callers,
                vec![format!("{file}::runOnce"), format!("{file}::waitForTask")]
            );
            let mut callers = b.callers_of(&format!("{file}::runOnce")).unwrap();
            callers.dedup();
            assert_eq!(callers, vec![format!("{file}::make")]);
            assert_eq!(
                b.callees_of(&format!("{file}::runOnce")).unwrap(),
                vec![format!("{file}::readTask")]
            );
            assert_eq!(
                b.callers_of(&format!("{file}::helper")).unwrap(),
                vec![format!("{file}::readTask")]
            );
        }
    }
}

#[test]
fn ambiguous_names_and_hoisted_receiver_bindings_abstain() {
    let registry = bundled_registry().unwrap();
    for declarations in [
        "interface readTask {} function scope() { const readTask = Effect.fn(\"x\")(function* () { helper(); }); }",
        "enum readTask { A } function scope() { const readTask = Effect.fn(\"x\")(function* () { helper(); }); }",
        "type readTask = number; function scope() { const readTask = Effect.fn(\"x\")(function* () { helper(); }); }",
        "function shadow(Effect) { const readTask = Effect.fnUntraced(function* () {}); }",
        "function block() { const Effect = {}; const readTask = Effect.fn(\"x\")(function* () {}); }",
        "function caught() { try {} catch (Effect) { const readTask = Effect.fnUntraced(function* () {}); } }",
        "const readTask = 0; function nested() { const readTask = Effect.fn(\"x\")(function* () { helper(); }); }",
        "const readTask = Effect.fn(\"x\")(function* () { helper(); }); function nested() { const readTask = 0; }",
        "function nested() { if (condition) { var Effect = fake; } const readTask = Effect.fn(\"x\")(function* () { helper(); }); }",
    ] {
        for file in ["app.ts", "app.tsx", "app.js"] {
            if file == "app.js" && ["interface", "enum", "type"].iter().any(|prefix| declarations.starts_with(prefix)) { continue; }
            let source = format!("import * as Effect from 'effect/Effect'; {declarations}");
            let ext = extract_file(file, source.as_bytes(), registry.for_file(file).unwrap()).unwrap();
            assert_ne!(ext.symbols.iter().find(|s| s.name == "readTask").unwrap().kind, SymbolKind::Function);
            assert!(!ext.relations.iter().any(|r| r.source_id == format!("{file}::readTask")));
        }
    }
}

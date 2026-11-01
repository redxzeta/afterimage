use infigraph_core::{extract::extract_file, model::RelationKind, Infigraph};
use infigraph_languages::bundled_registry;

#[test]
fn jsx_uses_exact_runtime_bindings_without_fabricating_calls() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("component.tsx"),
        r#"
export const Component = memo(function Component() { return <div />; });
export function Other() { return <span />; }
export function Phantom() { return <div />; }
const Private = memo(function Private() { return <div />; });
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("app.tsx"), r#"
import { Component as Menu, Other as lower, type Phantom } from './component';
import type { Component as Typed } from './component';
import { Private } from './component';
import { Other as Missing } from './missing';
import * as UI from './component';
export const App = memo(function App() { return <Menu><div /></Menu>; });
export function Paired() { return <Menu></Menu>; }
export function Shadow(Menu) { return <Menu />; }
export function Block() { const Menu = () => null; return <Menu />; }
export function Hoisted() { if (condition) { var Menu = fake; } return <Menu />; }
export function Catch() { try {} catch (Menu) { return <Menu />; } }
export function Negative() { return <><div /><lower /><Typed /><Phantom /><Private /><Missing /><Unknown /><UI.Component /></>; }
const Local = () => <span />;
export const LocalView = () => <Local />;
"#).unwrap();
    let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    graph.init().unwrap();
    graph.index().unwrap();
    for batch in [vec![], vec!["app.tsx".into()], vec!["component.tsx".into()]] {
        if !batch.is_empty() {
            let path = dir.path().join(&batch[0]);
            let content = std::fs::read_to_string(&path).unwrap();
            std::fs::write(&path, format!("{content}\n// incremental content change\n")).unwrap();
            graph.index_files(&batch).unwrap();
        }
        let backend = graph.backend().unwrap();
        let mut refs: Vec<_> = backend
            .find_all_references("component.tsx::Component")
            .unwrap()
            .into_iter()
            .map(|r| (r.caller_id, r.kind))
            .collect();
        refs.sort();
        assert_eq!(
            refs,
            vec![
                ("app.tsx::App".into(), "JSX_USES".into()),
                ("app.tsx::Paired".into(), "JSX_USES".into())
            ]
        );
        assert!(backend
            .callers_of("component.tsx::Component")
            .unwrap()
            .is_empty());
        for name in ["Other", "Phantom", "Private"] {
            assert!(
                backend
                    .find_all_references(&format!("component.tsx::{name}"))
                    .unwrap()
                    .is_empty(),
                "{name}"
            );
        }
        let refs = backend.find_all_references("app.tsx::Local").unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].caller_id, "app.tsx::LocalView");
        assert_eq!(refs[0].kind, "JSX_USES");
    }
    // Removing an export must not restore the old incoming JSX reference.
    std::fs::write(
        dir.path().join("component.tsx"),
        "const Component = memo(function Component() { return <div />; });",
    )
    .unwrap();
    graph.index_files(&["component.tsx".into()]).unwrap();
    assert!(graph
        .backend()
        .unwrap()
        .find_all_references("component.tsx::Component")
        .unwrap()
        .is_empty());
}

#[test]
fn jsx_extraction_ignores_intrinsic_dynamic_type_only_and_shadowed_tags() {
    let registry = bundled_registry().unwrap();
    for file in ["app.tsx", "app.jsx"] {
        let source = b"import { Component as Menu } from './component'; export const App = memo(function App() { return <Menu />; }); function Shadow(Menu) { return <Menu />; } function Unknown() { return <><div/><Missing/><UI.Menu/></>; }";
        let ext = extract_file(file, source, registry.for_file(file).unwrap()).unwrap();
        let jsx: Vec<_> = ext
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Custom("JSX_USES".into()))
            .collect();
        assert_eq!(jsx.len(), 1, "{file}");
        assert_eq!(jsx[0].source_id, format!("{file}::App"));
        assert_eq!(
            jsx[0].target_id,
            format!("{file}::'./component'::Component")
        );
    }
}

#[test]
fn legacy_graph_without_jsx_table_remains_readable() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("app.ts"),
        "function helper() {} export function run() { helper(); }",
    )
    .unwrap();
    let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    graph.init().unwrap();
    graph.index().unwrap();
    let conn = graph.store().unwrap().connection().unwrap();
    conn.query("DROP TABLE JSX_USES").unwrap();
    let query = infigraph_core::graph::GraphQuery::new(&conn);
    let refs = query.find_all_references("app.ts::helper").unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].kind, "CALLS");
}

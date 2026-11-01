use infigraph_core::Infigraph;
use infigraph_languages::bundled_registry;

#[test]
fn unknown_members_never_become_bare_name_calls() {
    for extension in ["ts", "tsx", "js"] {
        let dir = tempfile::tempdir().unwrap();
        let app = format!("app.{extension}");
        std::fs::write(dir.path().join(&app), "function max() {} function min() {} function exit() {} export function run(outbox, executor) { Math.max(1, 2); Math.min(1, 2); Effect.exit(0); outbox.claimNext(); executor.execute(); gitCore.statusDetailsRemote(); outbox['claimNext'](); outbox[method](); gitCore['statusDetailsRemote'](); }").unwrap();
        std::fs::write(dir.path().join("unrelated.ts"), "export class Replay { exit() {} } export function claimNext() {} export function execute() {} export function statusDetailsRemote() {}").unwrap();
        std::fs::write(
            dir.path().join("schema.graphql"),
            "type Query { max: Float min: Float }",
        )
        .unwrap();
        let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
        graph.init().unwrap();
        graph.index().unwrap();
        for batch in [vec![], vec![app.clone().into()]] {
            if !batch.is_empty() {
                graph.index_files(&batch).unwrap();
            }
            assert!(
                graph
                    .backend()
                    .unwrap()
                    .callees_of(&format!("{app}::run"))
                    .unwrap()
                    .is_empty(),
                "{extension}: fabricated member edge"
            );
        }
    }
}

#[test]
fn bound_namespace_named_import_and_self_calls_keep_exact_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("dep.ts"),
        "export function readTask() {} export function helper() {}",
    )
    .unwrap();
    std::fs::write(dir.path().join("app.ts"), "import * as tasks from './dep.js'; import { helper as imported } from './dep.js'; function local() {} class Client { static create() {} method() {} run() { this.method(); } nested() { const obj = { run() { this.method(); } }; const fn = function () { this.method(); }; } } export function run(client: Client) { tasks.readTask(); imported(); local(); Client.create(); } const X = class Client { run() { Client.create(); } }; export function nestedClass() { class Client {} Client.create(); } export function generators() { function* tasks() { tasks.readTask(); } function* Client() { Client.create(); } } export function classShadow(Client) { Client.create(); } export function classBlock() { const Client = {}; Client.create(); } export function shadow(tasks) { tasks.readTask(); } export function block() { const tasks = {}; tasks.readTask(); } export function callback(xs) { xs.map(tasks.readTask); }").unwrap();
    let mut graph = Infigraph::open(dir.path(), bundled_registry().unwrap()).unwrap();
    graph.init().unwrap();
    graph.index().unwrap();
    for batch in [vec![], vec!["dep.ts".into()], vec!["app.ts".into()]] {
        if !batch.is_empty() {
            graph.index_files(&batch).unwrap();
        }
        let backend = graph.backend().unwrap();
        for target in [
            "dep.ts::readTask",
            "dep.ts::helper",
            "app.ts::local",
            "app.ts::Client::create",
        ] {
            assert_eq!(
                backend.callers_of(target).unwrap(),
                vec!["app.ts::run"],
                "{target}"
            );
        }
        assert_eq!(
            backend.callers_of("app.ts::Client::method").unwrap(),
            vec!["app.ts::Client::run"]
        );
    }
}

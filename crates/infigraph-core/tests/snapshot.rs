use infigraph_core::{
    snapshot::{self, State},
    Infigraph,
};
use std::{path::Path, sync::Arc};

fn build(root: &Path, name: &str) -> Infigraph {
    let storage = root.join(".infigraph/snapshots").join(name);
    std::fs::create_dir_all(&storage).unwrap();
    let mut project = Infigraph::open_snapshot_shared(
        root,
        &storage,
        Arc::new(infigraph_languages::bundled_registry().unwrap()),
    )
    .unwrap();
    let before = project.source_fingerprints().unwrap();
    project.init_new_snapshot().unwrap();
    project.index().unwrap();
    snapshot::finish(&project, before).unwrap();
    project
}

#[test]
fn content_set_provenance_readers_and_atomic_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("app.ts"), "export function oldName() {}\n").unwrap();
    let first = build(root, "first");
    assert!(matches!(snapshot::verify(&first, true).state, State::Fresh));
    assert!(matches!(
        snapshot::verify(&first, false).state,
        State::Unknown
    ));
    drop(first);
    snapshot::promote(root, "first").unwrap();
    let first_storage = snapshot::selected(root).unwrap();
    let mut reader = Infigraph::open_snapshot_shared(
        root,
        &first_storage,
        Arc::new(infigraph_languages::bundled_registry().unwrap()),
    )
    .unwrap();
    reader.init_read_only().unwrap();
    std::fs::write(root.join("app.ts"), "export function newName() {}\n").unwrap();
    assert!(matches!(
        snapshot::verify(&reader, true).state,
        State::Stale
    ));
    let second = build(root, "second");
    drop(second);
    snapshot::promote(root, "second").unwrap();
    assert_eq!(
        snapshot::selected(root).unwrap(),
        root.join(".infigraph/snapshots/second")
    );
    assert!(reader
        .backend()
        .unwrap()
        .find_symbol_by_id("app.ts::oldName")
        .unwrap()
        .is_some());
    assert!(reader
        .backend()
        .unwrap()
        .find_symbol_by_id("app.ts::newName")
        .unwrap()
        .is_none());
    snapshot::promote(root, "first").unwrap();
    assert_eq!(snapshot::selected(root).unwrap(), first_storage);
    assert!(snapshot::promote(root, "../second").is_err());
    assert_eq!(snapshot::selected(root).unwrap(), first_storage);
    let manifest_path = first_storage.join("manifest.json");
    let original = std::fs::read(&manifest_path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    for (field, value) in [
        ("schema_version", serde_json::json!(99)),
        ("engine_build", serde_json::json!("other-build")),
        ("ready", serde_json::json!(false)),
        ("source_root", serde_json::json!("/different-checkout")),
    ] {
        let old = manifest[field].clone();
        manifest[field] = value;
        std::fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(
            matches!(snapshot::verify(&reader, true).state, State::Unknown),
            "{field}"
        );
        manifest[field] = old;
    }
    std::fs::write(&manifest_path, original).unwrap();
    std::fs::write(root.join("app.ts"), "export function oldName() {}\n").unwrap();
    std::fs::write(root.join("untracked.ts"), "export function unseen() {}\n").unwrap();
    assert!(matches!(
        snapshot::verify(&reader, true).state,
        State::Stale
    ));
    std::fs::remove_file(root.join("untracked.ts")).unwrap();
    assert!(matches!(
        snapshot::verify(&reader, true).state,
        State::Fresh
    ));
    std::fs::write(first_storage.join("NOT_READY"), "interrupted").unwrap();
    assert!(matches!(
        snapshot::verify(&reader, true).state,
        State::Unknown
    ));
    assert!(snapshot::selected(root).is_err());
}

#[test]
fn changed_during_build_and_running_receipt_never_ready() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("app.ts"), "function one() {}\n").unwrap();
    let project = build(root, "candidate");
    let before = project.source_fingerprints().unwrap();
    std::fs::write(root.join("app.ts"), "function two() {}\n").unwrap();
    assert!(snapshot::finish(&project, before).is_err());
    let receipt = project.storage_root().join(".infigraph/index-status.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
    value["resolution"]["state"] = serde_json::json!("running");
    std::fs::write(receipt, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(matches!(
        snapshot::verify(&project, true).state,
        State::Unknown
    ));
}

use infigraph_docs::store::DocStore;
use infigraph_docs::DocIndex;

#[test]
fn combined_doc_reads_preserve_an_incompatible_store_and_sidecars() {
    use infigraph_docs::combined::{combined_doc_query, combined_doc_search, combined_docs_path};
    let home = tempfile::tempdir().unwrap();
    struct RestoreHome(Option<std::ffi::OsString>);
    impl Drop for RestoreHome {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => std::env::set_var("HOME", value),
                None => std::env::remove_var("HOME"),
            }
        }
    }
    let _restore = RestoreHome(std::env::var_os("HOME"));
    std::env::set_var("HOME", home.path());
    let path = combined_docs_path("hardening-trial").unwrap();
    drop(DocStore::open(&path).unwrap());
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4..12].copy_from_slice(&(kuzu::get_storage_version() + 1).to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let sidecar = path.parent().unwrap().join("docs_embeddings.bin");
    std::fs::write(&sidecar, b"preserve-sidecar").unwrap();
    assert!(combined_doc_query("hardening-trial", "MATCH (d:Document) RETURN d.id").is_err());
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "query changed database bytes"
    );
    assert!(combined_doc_search("hardening-trial", "alpha", 1, 0.0).is_err());
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "search changed database bytes"
    );
    assert_eq!(std::fs::read(&sidecar).unwrap(), b"preserve-sidecar");
}

// A separate OS process is required: lbug permits repeated opens in one process.
#[test]
fn doc_store_holder() {
    let Some(root) = std::env::var_os("AFTERIMAGE_DOC_LOCK_FIXTURE") else {
        return;
    };
    let root = std::path::Path::new(&root);
    let _store = DocStore::open(&root.join(".infigraph/docs.kuzu")).unwrap();
    std::fs::write(root.join("ready"), []).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(15));
}

#[test]
fn init_preserves_docs_held_by_another_process() {
    struct Holder(std::process::Child);
    impl Drop for Holder {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".infigraph/docs.kuzu");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["doc_store_holder", "--nocapture"])
        .env("AFTERIMAGE_DOC_LOCK_FIXTURE", dir.path())
        .stdin(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut holder = Holder(child);
    let start = std::time::Instant::now();
    while !dir.path().join("ready").exists() {
        assert!(
            holder.0.try_wait().unwrap().is_none(),
            "holder exited before ready"
        );
        assert!(start.elapsed().as_secs() < 5, "holder never opened docs");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let bytes = std::fs::read(&path).unwrap();
    let result = DocIndex::open(dir.path()).unwrap().init();
    assert!(result.is_err(), "a live lock must not trigger a rebuild");
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "database bytes changed"
    );
}

#[test]
fn init_preserves_docs_with_an_incompatible_storage_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".infigraph/docs.kuzu");
    drop(DocStore::open(&path).unwrap());
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4..12].copy_from_slice(&(kuzu::get_storage_version() + 1).to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let mut index = DocIndex::open(dir.path()).unwrap();
    let result = index.init();
    assert!(result.is_err(), "incompatible docs must not be rebuilt");
    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("different version"), "{message}");
    assert!(message.contains("left untouched"), "{message}");
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "database bytes changed"
    );
}

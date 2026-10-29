use infigraph_core::graph::GraphStore;
use infigraph_core::lang::LanguageRegistry;
use infigraph_core::search::BM25Index;
use infigraph_core::Infigraph;

#[test]
fn wrapped_open_errors_retain_their_non_destructive_classification() {
    use infigraph_core::graph::open_failure_is_not_corruption;
    for message in [
        "Could not set lock on file",
        "Trying to read a database file with a different version",
    ] {
        let err = anyhow::anyhow!(message).context("outer caller context");
        assert!(open_failure_is_not_corruption(&err));
    }
    assert!(!open_failure_is_not_corruption(&anyhow::anyhow!(
        "Database ID mismatch"
    )));
}

// Run this binary serially. The holder owns a real native database lock.
#[test]
fn code_store_holder() {
    let Some(path) = std::env::var_os("AFTERIMAGE_CODE_LOCK_FIXTURE") else {
        return;
    };
    let path = std::path::Path::new(&path);
    let _store = GraphStore::open(path).unwrap();
    std::fs::write(path.with_extension("ready"), []).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(15));
}

#[test]
fn code_and_combined_stores_preserve_live_lock_holders() {
    use infigraph_core::multi::combined::{combined_graph_path, open_combined_graph};
    struct Holder(std::process::Child);
    impl Drop for Holder {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let dir = tempfile::tempdir().unwrap();
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
    for combined in [false, true] {
        let path = if combined {
            combined_graph_path("hardening-lock").unwrap()
        } else {
            dir.path().join(".infigraph/graph")
        };
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["code_store_holder", "--nocapture"])
            .env("AFTERIMAGE_CODE_LOCK_FIXTURE", &path)
            .spawn()
            .unwrap();
        let mut holder = Holder(child);
        let start = std::time::Instant::now();
        while !path.with_extension("ready").exists() {
            assert!(holder.0.try_wait().unwrap().is_none());
            assert!(start.elapsed().as_secs() < 5, "holder never opened graph");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let bytes = std::fs::read(&path).unwrap();
        let sentinel = path.with_extension("sentinel");
        std::fs::write(&sentinel, b"preserve").unwrap();
        let result = if combined {
            open_combined_graph("hardening-lock").map(|_| ())
        } else {
            Infigraph::open(dir.path(), LanguageRegistry::new())
                .unwrap()
                .init()
        };
        let err = result.expect_err("live locks must refuse, never rebuild");
        assert!(format!("{err:#}").contains("left untouched"));
        assert!(std::fs::read(&path).unwrap() == bytes, "database changed");
        assert_eq!(std::fs::read(sentinel).unwrap(), b"preserve");
    }
}

#[test]
fn hnsw_current_and_legacy_sidecars_preserve_search_results() {
    use infigraph_core::embed::{build_hnsw_index, invalidate_hnsw_cache, search_hnsw};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hnsw.usearch");
    let embeddings = dir.path().join("absent-embeddings.bin");
    let entries = vec![
        ("alpha".into(), vec![1.0, 0.0]),
        ("beta".into(), vec![0.0, 1.0]),
    ];
    assert_eq!(build_hnsw_index(&entries, &path, &embeddings).unwrap(), 2);
    for legacy in [false, true] {
        if legacy {
            std::fs::write(
                path.with_extension("meta"),
                br#"{"emb_mtime_secs":0,"dim":2,"ids":["alpha","beta"]}"#,
            )
            .unwrap();
            invalidate_hnsw_cache();
        }
        let hits = search_hnsw(&path, &embeddings, &[1.0, 0.0], 1)
            .unwrap()
            .unwrap();
        assert_eq!(hits[0].id, "alpha");
    }
}

#[test]
fn hnsw_rejects_an_impossible_id_count_before_loading_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hnsw.usearch");
    std::fs::write(&path, []).unwrap();
    let mut bytes = vec![1];
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&0u64.to_le_bytes());
    std::fs::write(path.with_extension("meta"), bytes).unwrap();
    let result = infigraph_core::embed::search_hnsw(
        &path,
        &dir.path().join("absent-embeddings.bin"),
        &[0.0; 8],
        1,
    );
    assert!(result.is_err());
}

#[test]
fn bm25_rejects_every_truncated_prefix_and_preserves_valid_scores() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bm25.bin");
    let index = BM25Index::build(vec![("a".into(), "alpha beta".into())]);
    index.save(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(
        BM25Index::load(&path).unwrap().search("alpha", 5),
        index.search("alpha", 5)
    );
    for end in 0..bytes.len() {
        std::fs::write(&path, &bytes[..end]).unwrap();
        assert!(BM25Index::load(&path).is_err(), "accepted prefix {end}");
    }
}

#[test]
fn bm25_rejects_impossible_counts_and_out_of_range_postings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bm25.bin");
    let mut bytes = vec![1];
    bytes.extend_from_slice(&1f32.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    assert!(BM25Index::load(&path).is_err());

    let index = BM25Index::build(vec![("a".into(), "alpha".into())]);
    index.save(&path).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    // The term count follows the one document; the posting count precedes
    // the final posting. Neither may reserve memory on a corrupt count.
    for offset in [9 + 4 + 1 + 4 + "alpha".len(), bytes.len() - 12] {
        let mut corrupt = bytes.clone();
        corrupt[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&path, corrupt).unwrap();
        assert!(BM25Index::load(&path).is_err());
    }
    // The single posting is the final (doc_idx, tf) pair.
    let offset = bytes.len() - 8;
    bytes[offset..offset + 4].copy_from_slice(&1u32.to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    assert!(
        BM25Index::load(&path).is_err(),
        "posting references absent document"
    );
}

#[test]
fn init_preserves_a_graph_with_an_incompatible_storage_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".infigraph/graph");
    drop(GraphStore::open(&path).unwrap());
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4..12].copy_from_slice(&(kuzu::get_storage_version() + 1).to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    let mut index = Infigraph::open(dir.path(), LanguageRegistry::new()).unwrap();
    let result = index.init();
    assert!(result.is_err(), "incompatible graph must not be rebuilt");
    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("different version"), "{message}");
    assert!(message.contains("left untouched"), "{message}");
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "database bytes changed"
    );
}

#[test]
fn combined_graph_preserves_an_incompatible_storage_version() {
    use infigraph_core::multi::combined::{combined_graph_path, open_combined_graph};
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
    // This test binary is run serially in an isolated environment.
    std::env::set_var("HOME", home.path());
    let path = combined_graph_path("hardening-trial").unwrap();
    drop(GraphStore::open(&path).unwrap());
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[4..12].copy_from_slice(&(kuzu::get_storage_version() + 1).to_le_bytes());
    std::fs::write(&path, &bytes).unwrap();
    assert!(open_combined_graph("hardening-trial").is_err());
    assert!(
        std::fs::read(&path).unwrap() == bytes,
        "database bytes changed"
    );
}

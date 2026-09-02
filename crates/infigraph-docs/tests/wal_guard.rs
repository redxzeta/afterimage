use infigraph_docs::store::DocStore;
use infigraph_docs::DocIndex;
use std::fs;

/// #143 (same class as the code graph's #140): a docs store written on a
/// lbug storage version this build cannot read is not corruption. `init()`
/// used to take the wipe-and-rebuild branch on *every* open error, silently
/// re-creating the store on this build's version; it must refuse, name both
/// versions, and leave the file byte-for-byte untouched.
#[test]
fn init_refuses_docs_store_from_a_different_storage_version_instead_of_wiping_it() {
    let tmp = tempfile::tempdir().unwrap();
    let ig = tmp.path().join(".infigraph");
    fs::create_dir_all(&ig).unwrap();
    let db_path = ig.join("docs.kuzu");
    drop(DocStore::open(&db_path).unwrap());

    // Re-stamp the header (`LBUG` magic, then the storage version as a
    // little-endian u64) with a version one past this build's own.
    let current = kuzu::get_storage_version();
    let foreign = current + 1;
    let mut bytes = fs::read(&db_path).unwrap();
    assert_eq!(&bytes[..4], b"LBUG", "unexpected database header layout");
    bytes[4..12].copy_from_slice(&foreign.to_le_bytes());
    fs::write(&db_path, &bytes).unwrap();

    let mut index = DocIndex::open(tmp.path()).unwrap();
    let err = index
        .init()
        .expect_err("a storage-version mismatch must be refused, not wiped and rebuilt");
    let msg = format!("{err:#}");
    assert!(
        msg.contains(&foreign.to_string()) && msg.contains(&current.to_string()),
        "the refusal must name both the file's and this build's storage version: {msg}"
    );
    assert!(
        index.store().is_none(),
        "no store may be open after a refusal"
    );
    assert_eq!(
        fs::read(&db_path).unwrap(),
        bytes,
        "the docs store must be left byte-for-byte untouched"
    );
}


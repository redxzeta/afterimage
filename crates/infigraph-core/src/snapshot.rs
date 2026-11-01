//! Explicit immutable AST snapshots. No startup work or automatic refresh.
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{index_status, Infigraph};

const MANIFEST_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u32,
    pub source_root: PathBuf,
    pub engine_version: String,
    pub engine_build: String,
    pub ready: bool,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum State {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Freshness {
    pub state: State,
    pub reason: String,
    pub checked_files: usize,
    pub scope: &'static str,
}

impl Freshness {
    pub fn unknown(reason: impl Into<String>) -> Self {
        Self { state: State::Unknown, reason: reason.into(), checked_files: 0,
            scope: "supported non-hidden files selected by index ignore rules; observed content, not an atomic filesystem snapshot" }
    }
}

pub fn load_manifest(storage: &Path) -> Result<Manifest> {
    anyhow::ensure!(!storage.join("NOT_READY").exists(), "snapshot NOT READY");
    let file = std::fs::File::open(storage.join("manifest.json"))?;
    anyhow::ensure!(
        file.metadata()?.len() <= MAX_MANIFEST_BYTES,
        "manifest too large"
    );
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_MANIFEST_BYTES,
        "manifest too large"
    );
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    anyhow::ensure!(
        manifest.schema_version == MANIFEST_VERSION,
        "unsupported manifest version"
    );
    anyhow::ensure!(manifest.ready, "snapshot NOT READY");
    Ok(manifest)
}

/// Resolve one atomic pointer read. A bad pointer fails closed; never fall back.
pub fn selected(root: &Path) -> Result<PathBuf> {
    let pointer = root.join(".infigraph/current-snapshot.json");
    if !pointer.exists() {
        return Ok(root.to_path_buf());
    }
    anyhow::ensure!(
        pointer.metadata()?.len() <= 256,
        "snapshot pointer too large"
    );
    let name: String = serde_json::from_slice(&std::fs::read(&pointer)?)?;
    validate_name(&name)?;
    let path = root.join(".infigraph/snapshots").join(name);
    anyhow::ensure!(
        path.canonicalize()? == path,
        "PATH_ESCAPE: snapshot symlink"
    );
    let manifest = load_manifest(&path)?;
    anyhow::ensure!(
        manifest.source_root == root,
        "snapshot belongs to a different checkout"
    );
    Ok(path)
}

pub fn validate_name(name: &str) -> Result<()> {
    anyhow::ensure!(
        !name.is_empty()
            && name.len() < 128
            && name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "invalid snapshot generation"
    );
    Ok(())
}

/// Atomic selection for new requests. Existing handles keep their old graph.
/// Selecting a previous ready generation is the explicit rollback operation.
pub fn promote(root: &Path, generation: &str) -> Result<()> {
    validate_name(generation)?;
    let directory = root.join(".infigraph");
    let storage = directory.join("snapshots").join(generation);
    anyhow::ensure!(
        storage.canonicalize()? == storage,
        "PATH_ESCAPE: snapshot symlink"
    );
    let manifest = load_manifest(&storage)?;
    anyhow::ensure!(
        manifest.source_root == root,
        "snapshot belongs to a different checkout"
    );
    validate_graph(&storage, &manifest)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
    serde_json::to_writer(&mut temporary, generation)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary.persist(directory.join("current-snapshot.json"))?;
    // Sync the directory on Unix so the rename survives a power loss.
    #[cfg(unix)]
    std::fs::File::open(&directory)?.sync_all()?;
    Ok(())
}

fn validate_graph(storage: &Path, manifest: &Manifest) -> Result<()> {
    use crate::graph::GraphBackend;
    let graph = storage.join(".infigraph/graph");
    anyhow::ensure!(graph.canonicalize()? == graph, "PATH_ESCAPE: graph");
    let metadata = graph.metadata()?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() >= 4096,
        "snapshot graph missing or truncated; not opened"
    );
    let store = crate::graph::GraphStore::open_read_only_with_config(
        &graph,
        kuzu::SystemConfig::default()
            .buffer_pool_size(64 * 1024 * 1024)
            .max_num_threads(2),
    )?;
    let backend = crate::graph::KuzuBackend::from_store(store);
    let hashes: BTreeMap<_, _> = backend.get_file_hashes()?.into_iter().collect();
    let report = index_status::load(storage)?;
    anyhow::ensure!(
        hashes == manifest.files
            && report.resolution.state == index_status::StageState::Succeeded
            && report.indexed_file_fingerprint == index_status::file_fingerprint(&backend)?,
        "snapshot graph/receipt invalid"
    );
    Ok(())
}

pub fn verify(project: &Infigraph, scan_source: bool) -> Freshness {
    let check = || -> Result<Freshness> {
        let manifest = load_manifest(project.storage_root())?;
        anyhow::ensure!(manifest.source_root == project.root(), "WRONG_CHECKOUT");
        anyhow::ensure!(
            manifest.engine_version == env!("CARGO_PKG_VERSION")
                && manifest.engine_build == crate::build_hash(),
            "ENGINE_VERSION_MISMATCH"
        );
        let backend = project.backend().context("graph unavailable")?;
        let receipt = index_status::load(project.storage_root())?;
        anyhow::ensure!(
            receipt.resolution.state == index_status::StageState::Succeeded,
            "index incomplete or failed"
        );
        let hashes: BTreeMap<_, _> = backend.get_file_hashes()?.into_iter().collect();
        anyhow::ensure!(
            hashes == manifest.files
                && index_status::file_fingerprint(backend)? == receipt.indexed_file_fingerprint,
            "graph/manifest/receipt mismatch"
        );
        if !scan_source {
            return Ok(Freshness::unknown("SOURCE_NOT_CHECKED: use get_stats, diagnose or CLI freshness for full verification"));
        }
        let current = project.source_fingerprints()?;
        let mut result = Freshness::unknown("");
        result.checked_files = current.len();
        if current == manifest.files {
            result.state = State::Fresh;
            result.reason = "CONTENT_AND_FILE_SET_MATCH".into();
        } else {
            result.state = State::Stale;
            result.reason = "CONTENT_OR_FILE_SET_CHANGED".into();
        }
        Ok(result)
    };
    check().unwrap_or_else(|error| Freshness::unknown(error.to_string()))
}

/// Finish only after indexing succeeded and a second source observation matches
/// both the pre-index source set and the graph's actual content fingerprints.
pub fn finish(project: &Infigraph, before: BTreeMap<String, String>) -> Result<()> {
    let after = project.source_fingerprints()?;
    anyhow::ensure!(
        before == after,
        "source changed during refresh; snapshot NOT READY"
    );
    let backend = project.backend().context("graph unavailable")?;
    let hashes: BTreeMap<_, _> = backend.get_file_hashes()?.into_iter().collect();
    anyhow::ensure!(
        hashes == after,
        "indexed source coverage mismatch; snapshot NOT READY"
    );
    let report = index_status::load(project.storage_root())?;
    anyhow::ensure!(
        report.resolution.state == index_status::StageState::Succeeded
            && report.indexed_file_fingerprint == index_status::file_fingerprint(backend)?,
        "index incomplete; snapshot NOT READY"
    );
    let manifest = Manifest {
        schema_version: MANIFEST_VERSION,
        source_root: project.root().to_path_buf(),
        engine_version: env!("CARGO_PKG_VERSION").into(),
        engine_build: crate::build_hash().into(),
        ready: true,
        files: after,
    };
    let mut file = tempfile::NamedTempFile::new_in(project.storage_root())?;
    serde_json::to_writer(&mut file, &manifest)?;
    file.as_file().sync_all()?;
    file.persist(project.storage_root().join("manifest.json"))?;
    Ok(())
}

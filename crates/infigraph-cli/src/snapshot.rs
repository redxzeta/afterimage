use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use infigraph_core::{lockfile, snapshot, Infigraph};

fn project(root: &Path, storage: &Path) -> Result<Infigraph> {
    Infigraph::open_snapshot_shared(root, storage, Arc::new(crate::full_registry(Some(root))?))
}

pub(crate) fn freshness(root: &Path) -> Result<()> {
    let result = (|| -> Result<_> {
        let root = root.canonicalize()?;
        let storage = snapshot::selected(&root)?;
        // Legacy graphs have no trustworthy snapshot provenance. Do not open
        // them merely to learn that freshness is unknown.
        snapshot::load_manifest(&storage)?;
        let mut project = project(&root, &storage)?;
        project.init_read_only()?;
        Ok(snapshot::verify(&project, true))
    })()
    .unwrap_or_else(|e| snapshot::Freshness::unknown(e.to_string()));
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

pub(crate) fn refresh(root: &Path, seconds: u64, rollback: Option<&str>) -> Result<()> {
    anyhow::ensure!(
        std::env::var("INFIGRAPH_BACKEND").map_or(true, |v| v == "kuzu"),
        "refresh supports local snapshots only"
    );
    let root = root.canonicalize()?;
    let directory = root.join(".infigraph");
    std::fs::create_dir_all(&directory)?;
    anyhow::ensure!(
        directory.canonicalize()? == directory,
        "PATH_ESCAPE: symlinked index"
    );
    let _guard = lockfile::acquire(&directory.join("refresh.lock"), "refresh", Duration::ZERO)?;
    if let Some(generation) = rollback {
        // Opening/validating a previous snapshot is read-only; stale source is
        // allowed for intentional rollback and is reported honestly afterwards.
        snapshot::validate_name(generation)?;
        let storage = directory.join("snapshots").join(generation);
        let manifest = snapshot::load_manifest(&storage)?;
        anyhow::ensure!(manifest.source_root == root, "WRONG_CHECKOUT");
        let mut previous = project(&root, &storage)?;
        previous.init_read_only()?;
        let evidence = snapshot::verify(&previous, false);
        anyhow::ensure!(
            evidence.reason.starts_with("SOURCE_NOT_CHECKED"),
            "invalid rollback snapshot: {}",
            evidence.reason
        );
        snapshot::promote(&root, generation)?;
        println!("ROLLED_BACK {generation}");
        return Ok(());
    }
    if let Ok(storage) = snapshot::selected(&root) {
        if let Ok(mut current) =
            snapshot::load_manifest(&storage).and_then(|_| project(&root, &storage))
        {
            if current.init_read_only().is_ok()
                && matches!(
                    snapshot::verify(&current, true).state,
                    snapshot::State::Fresh
                )
            {
                println!("NO_OP: verified source content and file set unchanged");
                return Ok(());
            }
        }
    }
    let generation = format!(
        "{}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
        std::process::id()
    );
    let storage = directory.join("snapshots").join(&generation);
    std::fs::create_dir_all(&storage)?;
    std::fs::write(
        storage.join("NOT_READY"),
        b"build incomplete; prior snapshot retained\n",
    )?;
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .arg("--root")
        .arg(&root)
        .arg("snapshot-build")
        .arg(&generation)
        .env("RAYON_NUM_THREADS", "2")
        .spawn()
        .context("start isolated snapshot builder")?;
    let start = Instant::now();
    let status = loop {
        if start.elapsed() >= Duration::from_secs(seconds) {
            child.kill()?;
            child.wait()?;
            anyhow::bail!(
                "refresh timed out; generation {generation} NOT READY; prior snapshot retained"
            );
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    anyhow::ensure!(
        status.success(),
        "refresh failed ({status}); generation {generation} NOT READY; prior snapshot retained"
    );
    // Reopen the closed graph before promotion to detect checkpoint/version errors.
    let mut candidate = project(&root, &storage)?;
    candidate.init_read_only()?;
    let evidence = snapshot::verify(&candidate, true);
    anyhow::ensure!(
        matches!(evidence.state, snapshot::State::Fresh),
        "candidate not fresh: {}",
        evidence.reason
    );
    snapshot::promote(&root, &generation)?;
    println!("REFRESHED {generation} (AST-only; old snapshots retained)");
    Ok(())
}

pub(crate) fn build(root: &Path, generation: &str) -> Result<()> {
    let root = root.canonicalize()?;
    anyhow::ensure!(
        !generation.is_empty() && generation.bytes().all(|c| c.is_ascii_digit() || c == b'-'),
        "invalid generation"
    );
    let storage = root.join(".infigraph/snapshots").join(generation);
    anyhow::ensure!(
        storage.canonicalize()? == storage && storage.join("NOT_READY").is_file(),
        "invalid build directory"
    );
    let _build_guard = lockfile::acquire(
        &storage.join("build.lock"),
        "snapshot-build",
        Duration::ZERO,
    )?;
    let mut project = project(&root, &storage)?;
    let before = project.source_fingerprints()?;
    project.init_new_snapshot()?;
    project.index()?;
    snapshot::finish(&project, before)?;
    drop(project);
    std::fs::remove_file(storage.join("NOT_READY"))?;
    Ok(())
}

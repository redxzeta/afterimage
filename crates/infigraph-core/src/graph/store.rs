use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use kuzu::{Connection, Database, SystemConfig};

use super::schema::{CREATE_SCHEMA, MIGRATIONS};
use super::store_util::escape;
use crate::lockfile::{self, LockFile};

/// RAII guard for exclusive write access to the graph store.
/// Holds an advisory file lock on `<db_path>.lock` with an identity
/// payload (see `crate::lockfile`).
#[derive(Debug)]
pub struct WriteLock {
    _guard: LockFile,
}

/// Role string stamped into the graph write lock's identity payload.
const GRAPH_WRITE_ROLE: &str = "graph-write";

/// Default wait budget for the graph write lock. Individual write calls
/// are short; 30s of waiting means something is wedged — surface it.
const GRAPH_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl WriteLock {
    fn acquire(lock_path: &Path) -> Result<Self> {
        Self::acquire_with_timeout(lock_path, GRAPH_WRITE_TIMEOUT)
    }

    fn acquire_with_timeout(lock_path: &Path, timeout: std::time::Duration) -> Result<Self> {
        let guard = lockfile::acquire(lock_path, GRAPH_WRITE_ROLE, timeout)?;
        Ok(Self { _guard: guard })
    }

    fn try_acquire(lock_path: &Path) -> Result<Option<Self>> {
        Ok(lockfile::try_acquire(lock_path, GRAPH_WRITE_ROLE)?.map(|guard| Self { _guard: guard }))
    }
}

// ---- Open-failure classification (R3.1.1) ----
//
// `Database::new` collapses every failure into one stringified error before
// it reaches any caller, so the only way to tell "not corruption -- leave the
// file alone" from "durably unopenable" is the text. These are the classes
// every wipe-on-open-failure path (`Infigraph::init`, `DocIndex::init`,
// `open_combined_graph`, the combined-docs generation) must recognise BEFORE
// taking a destructive branch. They live here, next to the open itself,
// precisely so that each of those paths does not grow its own partial list
// (#143 found two that had none at all).

/// Kuzu's IO-layer error for "another process holds this database's lock"
/// (see docs.ladybugdb.com/concurrency) is lock contention, not corruption.
/// This was previously indistinguishable from genuine corruption, so a
/// second `infigraph` process opening a graph while a watcher already had
/// it open would trigger `wipe_graph`, destroying the watcher's live data.
pub fn is_lock_contention_error(err: &anyhow::Error) -> bool {
    err.chain()
        .any(|cause| cause.to_string().contains("Could not set lock on file"))
}

/// lbug's `DatabaseHeader` refusal for a data file stamped with a storage
/// version this build cannot read ("Trying to read a database file with a
/// different version. Database file version: N, Current build storage
/// version: M"). `Database::new` raises it on any non-empty file (via
/// `Checkpointer::readCheckpoint`), so it arrives through the same "failed
/// to open ... kuzu db" wrapping as every other open failure. It is neither
/// transient nor corruption: the database was written by an `infigraph` on
/// a different lbug version (typically a stale binary still on the old one,
/// or a file left behind by a newer install) and this process simply cannot
/// read it. Wiping it would destroy a perfectly good database and serve an
/// empty one back as healthy (#140).
pub fn is_storage_version_mismatch_error(err: &anyhow::Error) -> bool {
    err.chain().any(|cause| {
        cause
            .to_string()
            .contains("Trying to read a database file with a different version")
    })
}

/// Every open failure that must never trigger a wipe or quarantine: the
/// file is either busy or written on a lbug
/// version this build cannot read. The single guard for callers whose only
/// choices are "refuse" or "destroy and rebuild" (`DocIndex::init`,
/// `open_combined_graph`).
pub fn open_failure_is_not_corruption(err: &anyhow::Error) -> bool {
    is_lock_contention_error(err) || is_storage_version_mismatch_error(err)
}

/// Context line for a lock-contention refusal. Afterimage has no lock probe.
pub fn lock_contention_context(_db_path: &Path) -> String {
    "graph is locked by another infigraph process (e.g. a running \
     `infigraph watch`) -- not corrupted, so it was left untouched. \
     Run `infigraph watch-status` or try again in a moment."
        .to_string()
}

/// Context line for a storage-version refusal. lbug's own message (kept as
/// the cause) already names both versions; this adds which build is
/// speaking, that nothing was touched, and the two ways out.
pub fn storage_version_mismatch_context(db_path: &Path) -> String {
    format!(
        "{} was written on a different lbug storage version than this build (v{}) can read \
         -- not corrupted, so it was left untouched. Every process touching this project must \
         run the same installed infigraph build (`infigraph doctor` identifies the build); to rebuild on this build's version instead, run `infigraph index --full`.",
        db_path.display(),
        kuzu::get_storage_version()
    )
}

/// Context line for any [`open_failure_is_not_corruption`] failure, so a
/// caller that refuses instead of wiping explains *why* the file was left
/// alone in the same words `Infigraph::init` uses.
pub fn non_corruption_open_context(err: &anyhow::Error, db_path: &Path) -> String {
    if is_lock_contention_error(err) {
        lock_contention_context(db_path)
    } else {
        storage_version_mismatch_context(db_path)
    }
}

/// Persistent graph store backed by Kuzu.
pub struct GraphStore {
    db: Database,
    lock_path: PathBuf,
}

impl GraphStore {
    /// Open or create a Kuzu database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock_path = path.with_extension("lock");
        let db = Database::new(path, SystemConfig::default())
            .map_err(|e| anyhow::anyhow!("failed to open kuzu db: {e}"))?;
        let store = Self { db, lock_path };
        store.init_schema()?;
        Ok(store)
    }

    /// Directory containing the graph database files. Used for disk-space
    /// preflight checks before a large write (see `store_util::check_disk_headroom`)
    /// -- Kuzu aborts the whole process with an uncaught C++ exception on
    /// ENOSPC mid-transaction rather than surfacing a Rust `Result`, so
    /// callers doing a large bulk write must check headroom themselves
    /// first (observed on sittir: SCIP enrichment ran the volume out of
    /// space mid-COPY and crashed the process).
    pub fn db_dir(&self) -> Option<&Path> {
        self.lock_path.parent()
    }

    /// Workspace metadata is available only for the standard project database layout.
    pub(crate) fn project_root(&self) -> Option<&Path> {
        self.db_dir()
            .filter(|p| p.file_name().is_some_and(|n| n == ".infigraph"))?
            .parent()
    }

    /// Open an existing Kuzu database in read-only mode.
    /// Safe for concurrent access while a watcher is writing.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        Self::open_read_only_with_config(path, SystemConfig::default())
    }

    /// Read-only open with caller-specified resource limits. Read-only mode is
    /// enforced here, regardless of the supplied configuration.
    pub fn open_read_only_with_config(path: &Path, config: SystemConfig) -> Result<Self> {
        let lock_path = path.with_extension("lock");
        let config = config.read_only(true).throw_on_wal_replay_failure(false);
        let db = Database::new(path, config)
            .map_err(|e| anyhow::anyhow!("failed to open kuzu db (read-only): {e}"))?;
        Ok(Self { db, lock_path })
    }

    /// Acquire exclusive write lock. Waits up to 30s, returning `Busy` if
    /// still held at expiry.
    pub fn write_lock(&self) -> Result<WriteLock> {
        WriteLock::acquire(&self.lock_path)
    }

    /// Acquire the write lock with a caller-chosen wait budget.
    pub fn write_lock_with_timeout(&self, timeout: std::time::Duration) -> Result<WriteLock> {
        WriteLock::acquire_with_timeout(&self.lock_path, timeout)
    }

    /// Try to acquire write lock without blocking. Returns None if already held.
    pub fn try_write_lock(&self) -> Result<Option<WriteLock>> {
        WriteLock::try_acquire(&self.lock_path)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.connection()?;
        for ddl in CREATE_SCHEMA {
            conn.query(ddl)
                .map_err(|e| anyhow::anyhow!("schema error: {e}\n  DDL: {ddl}"))?;
        }
        for migration in MIGRATIONS {
            let _ = conn.query(migration);
        }
        Ok(())
    }

    pub fn connection(&self) -> Result<Connection<'_>> {
        Connection::new(&self.db).map_err(|e| anyhow::anyhow!("failed to create connection: {e}"))
    }

    /// Remove all graph data for a deleted file.
    pub fn remove_file(&self, file: &str) -> Result<()> {
        let _lock = self.write_lock()?;
        let conn = self.connection()?;
        self.remove_file_conn(&conn, file)
    }

    /// Caller must hold WriteLock.
    pub fn remove_file_conn(&self, conn: &Connection<'_>, file: &str) -> Result<()> {
        let _ = conn.query(&format!(
            "MATCH (f:File)-[:DEFINES]->(s:Symbol)-[:HAS_STATEMENT]->(st:Statement) WHERE f.id = '{}' DETACH DELETE st",
            escape(file)
        ));
        let _ = conn.query(&format!(
            "MATCH (s:Symbol) WHERE s.file = '{}' DETACH DELETE s",
            escape(file)
        ));
        let _ = conn.query(&format!(
            "MATCH (m:Module) WHERE m.file = '{}' DETACH DELETE m",
            escape(file)
        ));
        let _ = conn.query(&format!(
            "MATCH (f:File) WHERE f.id = '{}' DETACH DELETE f",
            escape(file)
        ));
        Ok(())
    }

    /// Remove all files whose path starts with the given prefix (handles directory removal).
    pub fn remove_files_by_prefix(&self, prefix: &str) -> Result<usize> {
        let _lock = self.write_lock()?;
        let conn = self.connection()?;
        let escaped = escape(prefix);
        let result = conn
            .query(&format!(
                "MATCH (f:File) WHERE f.id STARTS WITH '{escaped}' RETURN f.id"
            ))
            .map_err(|e| anyhow::anyhow!("query files by prefix: {e}"))?;
        let mut files = Vec::new();
        for row in result {
            if let Some(val) = row.first() {
                files.push(val.to_string());
            }
        }
        for f in &files {
            self.remove_file_conn(&conn, f)?;
        }
        Ok(files.len())
    }

    /// Return map of file path -> content_hash for all indexed modules.
    /// Used by incremental indexing to skip unchanged files.
    pub fn get_file_hashes(&self) -> Result<HashMap<String, String>> {
        let conn = self.connection()?;
        let result = conn
            .query("MATCH (m:Module) RETURN m.file, m.content_hash")
            .map_err(|e| anyhow::anyhow!("get_file_hashes failed: {e}"))?;
        let mut map = HashMap::new();
        for row in result {
            if row.len() >= 2 {
                map.insert(row[0].to_string(), row[1].to_string());
            }
        }
        Ok(map)
    }

    /// Return all symbols as (name, id, file, kind) tuples -- used by resolve_calls.
    pub fn get_all_symbols(&self) -> Result<Vec<(String, String, String, String)>> {
        let conn = self.connection()?;
        let result = conn
            .query("MATCH (s:Symbol) RETURN s.name, s.id, s.file, s.kind")
            .map_err(|e| anyhow::anyhow!("get_all_symbols failed: {e}"))?;
        let mut symbols = Vec::new();
        for row in result {
            if row.len() >= 4 {
                symbols.push((
                    row[0].to_string(),
                    row[1].to_string(),
                    row[2].to_string(),
                    row[3].to_string(),
                ));
            }
        }
        Ok(symbols)
    }

    /// Get total counts for stats.
    pub fn derive_tested_by_edges(&self) -> Result<usize> {
        let _lock = self.write_lock()?;
        let conn = self.connection()?;
        let q = super::queries::GraphQuery::new(&conn);
        q.derive_tested_by_edges()
    }

    pub fn stats(&self) -> Result<GraphStats> {
        let conn = self.connection()?;

        let symbol_count = count_query(&conn, "MATCH (s:Symbol) RETURN count(s)")?;
        let module_count = count_query(&conn, "MATCH (m:Module) RETURN count(m)")?;
        let file_count = count_query(&conn, "MATCH (f:File) RETURN count(f)")?;
        let folder_count = count_query(&conn, "MATCH (d:Folder) RETURN count(d)")?;
        let calls_count = count_query(&conn, "MATCH ()-[r:CALLS]->() RETURN count(r)")?;
        let inherits_count = count_query(&conn, "MATCH ()-[r:INHERITS]->() RETURN count(r)")?;
        let contains_count = count_query(&conn, "MATCH ()-[r:CONTAINS]->() RETURN count(r)")?;

        Ok(GraphStats {
            symbols: symbol_count,
            modules: module_count,
            files: file_count,
            folders: folder_count,
            calls: calls_count,
            inherits: inherits_count,
            contains: contains_count,
        })
    }
}

#[derive(Debug)]
pub struct GraphStats {
    pub symbols: u64,
    pub modules: u64,
    pub files: u64,
    pub folders: u64,
    pub calls: u64,
    pub inherits: u64,
    pub contains: u64,
}

impl std::fmt::Display for GraphStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Graph Statistics:")?;
        writeln!(f, "  Symbols:      {}", self.symbols)?;
        writeln!(f, "  Modules:      {}", self.modules)?;
        writeln!(f, "  Files:        {}", self.files)?;
        writeln!(f, "  Folders:      {}", self.folders)?;
        writeln!(f, "  Calls edges:  {}", self.calls)?;
        writeln!(f, "  Inherits:     {}", self.inherits)?;
        writeln!(f, "  Contains:     {}", self.contains)
    }
}

fn count_query(conn: &Connection, query: &str) -> Result<u64> {
    let mut result = conn
        .query(query)
        .map_err(|e| anyhow::anyhow!("query failed: {e}"))?;
    if let Some(row) = result.next() {
        if let Some(val) = row.first() {
            return Ok(val.to_string().parse().unwrap_or(0));
        }
    }
    Ok(0)
}

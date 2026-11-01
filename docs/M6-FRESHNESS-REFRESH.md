# Explicit AST snapshot refresh and freshness

M6 baseline: Afterimage `e59380d3c7cff5f6e7a28c85b0164ec6451a5c62`.
Existing receipts record analysis execution and indexed module fingerprints;
previous diagnostics correctly left working-tree freshness unknown. The ordinary
index is `.infigraph/graph`, with `.infigraph/index-status.json` recording stage
outcomes. No freshness claim can be derived from HEAD equality alone.

## Commands and consumption

```sh
infigraph --root /path/to/checkout freshness
infigraph --root /path/to/checkout refresh --max-seconds 600
infigraph --root /path/to/checkout refresh --rollback GENERATION
```

`refresh` is explicit, local, AST-only, and synchronous. It does not copy source,
start a watcher, check for updates, install anything, or invoke external indexers.
It uses the existing language registry and file-selection rules. One refresh
holds the existing advisory-lock mechanism's `refresh.lock`; a competing refresh
fails immediately. A generation also has its own build lock. A verified unchanged
snapshot returns `NO_OP` without reindexing. A different HEAD with identical
selected source content remains a no-op.

A child process builds `.infigraph/snapshots/GENERATION/.infigraph/graph`, using
two Rayon workers, 32 MiB parser stacks, two database threads and a 512 MiB database
buffer pool. `--max-seconds` bounds the child build wall time; zero refuses any
build immediately. This is not a hard RSS limit: extraction and the native
library also allocate memory. Full source verification and final graph validation
are synchronous and outside the child build timeout.

Before promotion, the builder verifies that the before/after source file sets and
content fingerprints match the graph's actual module hashes and successful stage
receipt. After closing the writer, the parent reopens and validates the candidate
read-only. A small JSON pointer `.infigraph/current-snapshot.json` is atomically
replaced only after validation. New **read-only MCP requests**, CLI `freshness`,
and graph diagnostics use that selected generation. Each read-only MCP request
pins one generation for its result and evidence. Existing readers keep their old
graph; no graph file is overwritten. Root `.infigraph/graph` remains untouched.

Legacy CLI query/index commands and mutable MCP operations retain their existing
root-graph behavior. They do **not** consume this snapshot pointer. MCP graph
queries that use the read-only graph helper select the snapshot in either profile.
Use the read-only MCP profile to query a refreshed snapshot. A normal root reindex
does not refresh the selected snapshot. This opt-in separation preserves existing
writers and installed clients.

All previous generations remain available. `--rollback` validates a named graph,
receipt, build compatibility and provenance before atomically selecting it;
rollback can deliberately select stale source, which diagnostics report `STALE`.
An invalid, truncated, busy, incompatible or incomplete generation is rejected
without replacing the pointer. A crashed/timed-out build retains `NOT_READY`;
it cannot be selected even if it contains an apparently ready manifest. Retry
uses a new generation and preserves the prior working snapshot. There is no
automatic recovery, pruning, daemon, scheduler or startup refresh. Retained
snapshots consume disk space; removal of unused generations is an explicit
operator action, after readers have closed them.

## Freshness contract

CLI `freshness` emits `FRESH`, `STALE`, or `UNKNOWN` plus reason and checked count.
Read-only MCP `get_stats` and `diagnose` perform the same full content verification;
metadata schema v1 preserves lowercase `freshness`, while
`freshness_evidence.state` uses the uppercase vocabulary. Other graph tools report
`UNKNOWN / SOURCE_NOT_CHECKED` rather than scanning every source file on every
invocation or trusting a cached result that could hide edits.

* `FRESH`: selected ready manifest belongs to the exact canonical checkout;
  engine/build, graph module hashes and successful receipt agree; the current
  selected source file set and content fingerprints agree with the manifest.
* `STALE`: verified snapshot exists but selected source content or file set differs.
  Tracked staged/unstaged edits and new relevant untracked files are included.
* `UNKNOWN`: missing/legacy index or manifest, unsupported manifest/build version,
  incomplete/failed indexing, graph/receipt mismatch, wrong checkout, unreadable
  source or unsupported provenance. No parent/sibling graph is silently borrowed.

The scope is exactly the indexer's supported, non-hidden, non-ignored source file
set, using `.gitignore` and `.infigraphignore`. Hidden files, unsupported languages,
ignored files, `node_modules`, `.infigraph`, `__pycache__`, `.tox` and symlink file
entries are excluded. Directory-walk and read failures refuse a freshness claim.
An exported non-Git directory can be indexed explicitly with the same contract;
its canonical path is the identity, and its missing graph never falls back to an
outer exported directory. Git subdirectories resolve to their checkout boundary.

This is a content observation, not an atomic operating-system source snapshot:
concurrent source edits after observation can immediately make a result outdated.
No HEAD, branch name, process liveness, cache age or absence of logs establishes
freshness. `FRESH` describes selected source coverage, not call-graph completeness,
SCIP/type coverage, runtime behavior, or evidence that an absent edge cannot exist.

## Validation

Focused integration targets:

```sh
cargo test --locked -p infigraph-core --test snapshot \
  -p infigraph-cli --test refresh \
  -p infigraph-mcp --test readonly_profile -- --test-threads=1
```

They cover exact content, additions and edits, source changes during construction,
wrong checkout/version/incomplete receipts, no-op, concurrent refresh refusal,
timeout with unchanged pointer, partial retry, immutable old-reader consistency,
atomic rollback, snapshot-only MCP selection, source-edit staleness, wrong-path
isolation and mutation denial. The tests use temporary isolated fixtures; no live
T3Code graph or project configuration is modified.

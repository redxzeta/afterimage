# Provisional upstream index-freshness trial

## Source assumption and baseline

The original request omitted its source branch. This trial provisionally examines
[intuit/infigraph `fix/issue-48-index-freshness`](https://github.com/intuit/infigraph/tree/02c498dae06557a15a5f57d6ea3346725fdc5e4b),
not a branch explicitly selected by the owner. Bounded recent-context searches did
not identify a different source. This is an experimental fork change, not merge,
deployment, installation, or activation approval.

Verified identities on October 24, 2026:

- Experiment started at `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`.
- Fork integration baseline advanced independently through PR #6; this experiment
  fast-forwarded its own branch to `4d5266fe10c2c39dbaac423c7a43731329601e85`.
- Fork default/base: `afterimage-main`. Fork pristine `main` and upstream `main`:
  `cf82f5d7440d9be158e219df1ef256d6359ed1f6`.
- Candidate: `02c498dae06557a15a5f57d6ea3346725fdc5e4b`; code commit:
  `93a2cec183496608374ab94697e95ee02c75f636`.
- Candidate/upstream merge base: `6dfc8c1bd2e296f08374ef90cd8f85826cbdec34`.
  `git rev-list --left-right --count` reports upstream main 82 commits ahead,
  candidate 2 commits ahead. Candidate contribution: 15 files, +951/-28.
- Other inspected candidate `devel/watch-defect-tests`:
  `33214d9c3f0d0275e5ddb92677d91c074576df38`; broad, older work with multiple
  merge bases, 25 files and +923/-263 in the selected merge-base comparison.
  It is not an appropriate wholesale freshness import.

## Findings and disposition

| Candidate behavior | Decision | Evidence and reason |
| --- | --- | --- |
| Warn on `trace_callers`, `trace_callees`, `transitive_impact`, and `find_all_references` | Adapt | A useful gap remains in ordinary query output. Existing doctor revision checks and caller coverage notices do not establish query freshness. |
| New `.infigraph/index_meta.json` receipt and `Fresh` state | Reject | Candidate core `lib.rs:463` stamps HEAD after indexing a subset. A clean tree at that HEAD can still have other files from the previous index. Matching HEAD and an empty pending vector cannot certify content or resolution. Metadata writes also discard errors. |
| Suppress warnings for `Unknown` | Reject | Candidate `freshness.rs:111-118` silently returns results when evidence is absent, contrary to issue #48's unknown-state warning criterion. |
| Watcher-start reconciliation | Reject for this trial | Candidate `watch/mod.rs:434-461` returns when HEAD matches, missing offline uncommitted edits. It emits only `Modified` callbacks, not normal change-batch/deletion work. Its added test asserts pending status, not repaired graph contents. |
| Treat dirty Git state as a known stale graph | Adapt | Dirty files may already be indexed. Report the observation while keeping freshness unverified. |
| Read pending cross-file work | Adapt | Inspect the owning MCP registry without blocking; unavailable/busy/poisoned state remains explicit. This vector excludes the core queue, in-progress indexing, and other processes. |
| Watch-status formatting | Defer | It would overlap the separately owned watcher work. Existing status/doctor contracts remain unchanged. |
| Populated-directory watcher repair | Separate PR #7 | Fork PR #7 addresses directory registration and initial file queuing, not query warnings. No watcher code is changed here. |
| Docker harness and `.dockerignore` | Reject | Disposable local fixtures can validate this adaptation without new container/build infrastructure. |
| README/index-freshness guide | Adapt | This report documents the narrower behavior and its limits rather than importing unsupported guarantees. |

Sources: [issue #48](https://github.com/intuit/infigraph/issues/48),
[upstream PR #53](https://github.com/intuit/infigraph/pull/53),
[fork PR #7](https://github.com/redxzeta/afterimage/pull/7), and pinned candidate
source above. Findings about candidate behavior are source review, not a claim
that the old candidate's full native suite was executed.

## Implementation and alternatives

A private MCP helper appends an advisory to the four named tools, including empty
results. It reads the existing local registry JSON, compares recorded revisions
with current Git HEAD, observes working-tree dirtiness excluding `.infigraph`,
and reads this worker's pending cross-file entries. Git status disables optional
index writes and fsmonitor callbacks. Local registry reads do not initiate remote
backend connections. Errors are shown as unavailable evidence, not as a clean
state. It does not write new receipts or change graph data.

The existing coverage prefix, test filtering, interface expansion, result body,
read-only graph access, error propagation, and diagnostic JSON are retained.
Compression and seen-result deduplication operate on the existing body; the
advisory is reattached afterwards so repeated MCP responses retain it.

Rejected alternatives:

1. Cherry-pick the old commits: risks importing assumptions from a branch 82
   commits behind current upstream, conflicting with richer fork query behavior.
2. Add complete content/generation reconciliation: potentially useful future work,
   but requires coordinated indexing/resolution semantics beyond this trial.
3. Use HEAD equality as proof: incorrect for uncommitted work, missed events,
   subset indexing, ignored files, and concurrent changes.
4. Change watcher startup or add strict query waiting: overlaps #7 ownership and
   changes runtime behavior beyond a conservative advisory trial.

## Validation evidence and blocker

The benchmark coordinator explicitly required no native build/test overlap and
has not released the timing window. The request for a quiet two-job validation
window was acknowledged; clearance is still pending. No native command was
started, and no existing test was disabled or weakened. The local cached Rust
toolchain is available; this is coordination-blocked, not a missing-toolchain
failure. This draft is not ready for merge or runtime adoption.

Executed in the experiment worktree:

| Command/check | Exact result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS, exit 0; latest recorded wall time 0.715 s |
| `git diff --check` | PASS, exit 0 |
| Targeted `rustfmt --edition 2021 --check` on changed Rust modules/tests | PASS, exit 0 |
| `git rev-list --left-right --count refs/experiment/upstream-main...refs/experiment/upstream-freshness` | PASS, exit 0; `82 2` |
| Protected-path diff inspection | No edits to core, watcher, CLI, dependencies, CI, or operational configuration |

Written regression coverage: **two tests; zero executed**. The integration
scenario covers all four tools with matching/changing HEAD, uncommitted edits,
deletion, new nested paths, subdirectory root resolution, subset indexing, return
checkout with stale graph content, pending/busy/poisoned watcher state, malformed
registry, non-Git state, unchanged registry/coverage bytes, and repeated MCP
responses. A second test explicitly exercises long-reference compression and
seen-result replacement while retaining the advisory.

Required bounded validation after explicit window release (all currently
**BLOCKED / UNRUN**, with no pass/fail count):

```sh
# Run in an isolated HOME, registry, cache and target; offline; two build/Rayon jobs.
cargo test --locked --offline --no-default-features -p infigraph-mcp \
  --test freshness_tools --test coverage_reporting --test diagnose \
  --test tool_dispatch -- --test-threads=1
cargo clippy --locked --offline --no-default-features -p infigraph-mcp \
  --lib --bin infigraph-mcp --test freshness_tools -- -D warnings
cargo build --locked --offline --no-default-features \
  -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp
```

Copy the new regression test into a disposable archive of exact base
`4d5266fe10c2c39dbaac423c7a43731329601e85`, without production changes,
to establish red-on-base/green-on-candidate behavior. This has **not been done**.
Native CLI/MCP smoke, actual running-watcher branch-switch/deletion/new-directory
checks, candidate Clippy/type checking, and cross-platform/full-feature suites
remain unrun. Do not substitute prior PR #7 receipts or old native binaries for
validation of this candidate. No A/B functionality or performance was measured;
there is no speedup, query-accuracy, or watcher-convergence claim.

## Self-review findings and fixes

- **High correctness risk, rejected upstream design:** successful partial indexing
  cannot stamp a project-wide fresh HEAD. The trial creates no receipt and never
  certifies freshness.
- **Moderate correctness risk, fixed in adaptation:** compression or seen-result
  replacement could hide the warning. Preserve the existing body pipeline and
  reattach the final advisory on every response, including panic fallback.
- **Moderate reliability risk, fixed in adaptation:** locking unavailable watcher
  state could block or panic a query. Nonblocking lock inspection reports unknown
  state instead. The regression includes busy and poisoned pending locks.
- **Preservation check:** existing read-only query paths and caller coverage
  prefixes remain; no watcher/core or #7-owned code was changed.
- **Unresolved verification gap:** source review and rustfmt do not establish Rust
  type correctness or runtime behavior. Native checks are mandatory before
  recommending adoption. No independent second-agent review was requested/run.

## Operational and security limits

There is no new service, schedule, dependency, credential, database, deployment
step, permission model, or ongoing operator responsibility. Core indexing,
watchers, supervisor recovery, CLI behavior, diagnostics, CI, tags, releases,
upstream branches, and other applications are outside this change.

The advisory is deliberately conservative and can remain present after a correct
index. It adds two Git subprocesses and a local registry read per affected tool
call; large repositories may incur noticeable cost. Git status excludes submodule
inspection and is not a content-hash reconciliation. Queries and observations are
not atomic. Zero pending cross-file entries is not caught-up evidence. This trial
does not fix hidden-ancestor watcher omissions, offline edits, event loss,
branch-switch convergence, ignored-file freshness, or incomplete SCIP enrichment.
Only four named MCP tools gain the advisory; this is partial issue #48 coverage.

Recommendation: review only the narrow experimental adaptation after validation;
do not merge the full upstream branch or infer operational readiness. Confirm the
intended source branch before treating this provisional trial as the requested
upstream adoption.

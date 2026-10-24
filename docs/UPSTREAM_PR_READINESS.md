# Upstream PR readiness

Review snapshot: 2026-10-24 UTC (2026-10-23 America/Los_Angeles).
This is a follow-on source review, not a repeat of Phase A or a release decision.

## Pinned scope and live upstream status

Fork integration base: `1a59c728ae3284f0af3b5c781e14a9e2736df6e0` on
`afterimage-main`. Upstream `main` and fork mirror `origin/main` were both
`cf82f5d7440d9be158e219df1ef256d6359ed1f6` at review time. All five exact PR
heads below are ancestors of the integrated fork base. Fork integration and
merged fork PRs #1–#4 do **not** mean these upstream PRs have merged.

Live `gh api` inspection covered PR bodies, commits, diff files, submitted
reviews, inline review comments, issue comments, check runs, workflow runs and
jobs. All five PRs are **OPEN**, `merged=false`, mergeable without a reported
conflict, and `mergeable_state=blocked`. Each has four requested reviewers,
zero submitted reviews and zero inline review comments. The authenticated
contributor has upstream `push=false`, `maintain=false`, `admin=false`.
No upstream comments, approvals, branch updates or workflow reruns were sent.

| PR / exact head | Dependency and source assessment | Next action |
| --- | --- | --- |
| [#78](https://github.com/intuit/infigraph/pull/78) `6d77b0e7d8d18c959d62b98e2e60a81ddd250a57` | Default `kompress` opt-out and loopback HTTP default; two commits, older base `c54d13768b5c5ee06c8e122b60e9512351f8d8db`. Reviewer-requested output assertion is fixed. New MCP bind test still uses an invalid wildcard-rebind assertion. | Keep output fix; apply the already-integrated socket-address test correction to this existing PR. |
| [#82](https://github.com/intuit/infigraph/pull/82) `2c621c25d470cd37cb54a15d0775c03d0a08294d` | Independent one-commit embedding allocation bound; rejects impossible counts before reserving, preserves valid empty entries. Included in #81. | Small prerequisite suitable for separate review; does not validate native graph parsers. |
| [#83](https://github.com/intuit/infigraph/pull/83) `9e68b72365dde8e5f186362eb1ac433a6cfcee2a` | Independent one-commit strict Clippy cleanup, no allowances/suppressions. Included in #81. `as_chunks::<4>()` keeps little-endian decoding and complete 4-byte groups. | Review separately; note current stable toolchain, not an implied older MSRV guarantee. |
| [#81](https://github.com/intuit/infigraph/pull/81) `435723028002a9d294a01e113701136b4d4a954a` | Six commits including #82/#83 and their merge commits. Diagnostic engine shared by CLI/MCP; no observed mutation in reviewed diagnostic paths. Runtime CPU validation also benefits from #78. | Land/reconcile #82/#83 first, then refresh the branch by an owner-controlled forward merge of upstream main and inspect the resulting diff. Keep diagnostics and prerequisites distinct. |
| [#84](https://github.com/intuit/infigraph/pull/84) `4a775e7648414cbe3cf9684066fcc5856ea87856` | Three commits on `cf82f5d`; independent of #81 source, but baseline document/lint/default CPU blockers relate to #82/#83/#78. Fixes callable ownership and warns on incomplete stages. Unchanged-file batch writes can falsely reuse successful SCIP receipts; duplicate graph edges remain possible. | Apply the narrow receipt fix below; retain duplicate/missing-edge limitations and review single-file mutation coverage before claiming readiness. |

`git merge-tree --write-tree origin/main refs/review/upstream-prN` returned 0
without conflicts for N=78,82,83,81,84. This checks merge construction only; it
does not execute a merged build. Ancestry checks confirm #82/#83 are in #81 and
#78 is not.

No public-history rewrite is needed for this review. After prerequisite landing,
use ordinary new commits/forward merges on the contributor-owned branches; if
upstream squashes prerequisites, inspect the effective diff for duplicate
content. Recheck the replacement head and synthetic merge, not just the old
head. Do not blindly cherry-pick already-included prerequisites into #81.

| Exact-head hosted run | Current result |
| --- | --- |
| [#78 CI](https://github.com/intuit/infigraph/actions/runs/37091792570) | `action_required`, 0 jobs, 0 check runs |
| [#82 CI](https://github.com/intuit/infigraph/actions/runs/37177784672) | `action_required`, 0 jobs, 0 check runs |
| [#83 CI](https://github.com/intuit/infigraph/actions/runs/37177863735) | `action_required`, 0 jobs, 0 check runs |
| [#81 CI](https://github.com/intuit/infigraph/actions/runs/37177969612), [Pages](https://github.com/intuit/infigraph/actions/runs/37177969644) | Both `action_required`, 0 jobs, 0 check runs |
| [#84 CI](https://github.com/intuit/infigraph/actions/runs/37235469314) | `action_required`, 0 jobs, 0 check runs |

These are maintainer-approval gates, supported by the existing #81–#83 comments,
not failed builds. No executed hosted failure exists to reproduce from these
runs. There is no hosted acceptance claim, and no waiting for maintainer approval
in this lane. Fork hosted CI is not a blocker for this review.

## Findings and comment-ready feedback

### #78: text assertion fixed; bind-test assertion still wrong

The [reviewer's request](https://github.com/intuit/infigraph/pull/78#issuecomment-5960698985)
was to replace `stdout.contains("Test Context")` with
`stdout.contains("Test Generation Context")`. Commit
`6d77b0e7d8d18c959d62b98e2e60a81ddd250a57` does exactly that, matching
`info_commands.rs`. The subsequent [contributor response](https://github.com/intuit/infigraph/pull/78#issuecomment-5966337832)
is consistent with the actual final diff. No outstanding text bug remains.

Proposed comment:

> The requested test-context header fix is present at `6d77b0e`. One separate
> regression assertion still needs correction: binding `0.0.0.0:port` after a
> loopback listener need not succeed. Linux rejects that overlap even when the
> first listener correctly uses `127.0.0.1`. Assert the bound socket's address
> instead. The fork already contains the correction in
> `4ed6708c4a9fbb693435079288d7242b3f30f1ef`, covering both UI and MCP tests,
> without weakening loopback behavior.

Exact existing patch:
[4ed6708](https://github.com/redxzeta/afterimage/commit/4ed6708c4a9fbb693435079288d7242b3f30f1ef).
It factors the actual `Server::http` bind into `bind_http_server`, then checks
`listener.server_addr().to_ip().unwrap().ip() == Ipv4Addr::LOCALHOST`.
Apply as a new commit on #78; the fork needs no repeat of that Phase A change.

### #81: diagnostic execution avoids mutation; health is not freshness

Reviewed `diagnostics.rs`, `lockfile::observe`, read-only graph configuration,
registry loading, binary discovery, CLI early dispatch, MCP diagnostic dispatch
and entry-point probe handling. The CLI branches before updater/watcher startup.
MCP `diagnose` bypasses activity logging, focus/session accounting and compression;
it observes the existing watcher registry. The hidden graph child bypasses normal
startup, uses a read-only DB open, checks regular-file size, advisory locks and
WAL-family siblings, and has a ten-second parent timeout. Unix probes cap address
space and disable core dumps. Probe failure becomes UNKNOWN, never a repair.
The local registry loader reads without saving; selected remote backends are
reported UNKNOWN without a connection attempt.

An existing free lock is held shared through the probe. A missing lock is not
created; this remains an observation rather than atomic exclusion of every
noncooperating writer. Native DB locking/read-only mode still matter. Header
readability proves neither embedding contents nor model/HNSW validity. Probe
resource limits can reject a large valid graph and deliberately produce UNKNOWN.
These bounds are deliberate, not proof of a complete working index.

Proposed comment:

> `4357230` preserves the read-only diagnostic boundary in the reviewed paths.
> Keep the explicit UNKNOWN working-tree freshness line even with HEALTHY or a
> registry revision match. #82/#83 are already in this branch's history; land
> them separately and reconcile with a forward main merge before final review.
> Native compression/default-feature runtime evidence remains separate from
> diagnostics correctness. Agent-registration inspection remains #79.

### #84: batch writes can retain a false successful enrichment receipt

`Infigraph::index_files` deliberately upserts requested files even when their
bytes match. Upsert removes/recreates symbols and their incident edges.
`AstRun::finish` previously copied successful SCIP outcomes whenever indexed
file fingerprints matched, allowing `scip_succeeded` to skip needed enrichment
and queries to print `Coverage: recorded analysis stages succeeded` after a
compiler-only edge disappeared.

The new `reindexing_unchanged_files_invalidates_compiler_enrichment` regression
imports a tiny synthetic compiler reference, records success, proves a real
incremental no-op retains both receipt and edge, then batch-reindexes unchanged
source. It observes the edge deletion and identical file fingerprint and requires
the old SCIP receipt to be invalidated. No tool provisioning or target app runs.

Proposed comment:

> On `4a775e7`, identical file fingerprints do not imply unchanged graph edges.
> In `AstRun::finish`, require `!resolution_attempted` as well as matching
> fingerprints before cloning `previous.scip`. `index_files` has rewritten its
> symbols whenever extractions are nonempty. Keep genuine `index()` no-op reuse.
> The supplied regression fails on your exact production source and checks both
> cases. This prevents a false success notice and lets the normal CLI enrichment
> path retry; it does not itself run SCIP or restore deleted edges.

The [fork source commit](https://github.com/redxzeta/afterimage/commit/e8bb61bf297d28349a055dae4cc7930f8ccac14b)
contains only the production condition and its regression, so it can be proposed
as a new commit on the existing #84 branch. No contributor branch was changed.

Exact production patch applied on the fork:

```diff
-            if previous.indexed_file_fingerprint == self.report.indexed_file_fingerprint {
+            if !resolution_attempted
+                && previous.indexed_file_fingerprint == self.report.indexed_file_fingerprint
+            {
                 self.report.scip.clone_from(&previous.scip);
```

### #84: remaining graph and receipt limits

- CLI/MCP sorting and `dedup()` remove repeated IDs from displayed caller/callee
  results. They do not deduplicate stored CALLS edges. The importer deduplicates
  only within the current import and can append a pair already supplied by AST
  resolution or a prior import. A probe on exact #84 production source imported
  the same fixture twice with `Some(project_root)` and measured **2 raw CALLS
  edges** for one pair. Its passing assertion confirms the defect, not a fix.
  Use raw counts and repeated-import fixtures to
  assess storage, rather than interpreting clean display output as a graph fix.
- SCIP's TypeScript owners use one-based line conversion and extracted callable
  IDs. Boundary-line, equal-span, ambiguous target and missing extracted owner
  cases intentionally remain unresolved. These omissions must remain visible as
  graph limitations, even when the compiler process/import reports success.
- The shared edge COPY/fallback helper returns no `Result`; it can log/drop bad
  records while the enclosing import returns success and reports attempted
  counts. Stage completion is not proof that every intended edge was stored.
  A follow-up should propagate write outcomes before claiming complete import
  acceptance. This is inherited storage behavior; stage receipts must not be
  mistaken for stronger write validation. This review does not change shared
  storage infrastructure.
- `Infigraph::index_file` directly upserts without `AstRun`, unlike `index_files`.
  A same-content direct single-file rewrite can bypass receipt invalidation and
  its serialization lock. This source-review finding remains open: wrap that
  API in the same receipt boundary, marking resolution unverified when it has
  not run, and add a single-file preservation/invalidation regression. The fix
  here is deliberately confined to the reproduced batch path.
- File hashes and run IDs bind receipts to recorded modules, not working-tree
  freshness, dependency/compiler configuration, exhaustive relationship coverage
  or semantic-search quality. No fresh/comprehensive-index claim is justified.

## Verification

Exact-head checks use disposable `git archive` source snapshots and an independent
Cargo target seeded from existing build artifacts. Cargo re-evaluates fingerprints
for each source directory. Rust/Cargo 1.98.1, one build job, one native build job,
zero dev/test debug info, `--locked --offline`, serial tests, disposable HOME,
registry and temporary files; core dumps disabled. No production checkout/index,
installed binary, target application, package hook, provider, or persistent listener
is used. No benchmark, full expensive workspace suite, test exclusion, lint
allowance, installation or deployment is part of this review. Exact #83
all-feature Clippy and native compression SIGILL reproduction are NOT_RUN here;
#83 was reviewed as source and those historical reports remain historical.

Exact #78 tests did not execute: its older source pins `lbug 0.16.0`, while
current integrated/cache source uses `0.19.1`. The focused MCP test started a
native rebuild; it was stopped (only this review's process tree) to honor the
bounded-check scope. The exact-head test-context invocation was not started.
The assertion fix is source-confirmed, not freshly runtime-validated here.
A separate std-only Rust mechanism probe copies the faulty wildcard-rebind
assertion after opening a known loopback listener; it needs no database build.
The mechanism probe exits 101 on this Linux host (0 passed, 1 failed),
confirming the assertion is invalid even with a known loopback listener.
It is separate from the PR's native test executable. Reproduce it without native
DB dependencies by saving this as `loopback_rebind.rs`, then running
`rustc --test loopback_rebind.rs -o loopback_rebind` and
`./loopback_rebind --exact upstream_wildcard_rebind_assertion`:

```rust
use std::net::TcpListener;
#[test]
fn upstream_wildcard_rebind_assertion() {
    let loopback = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = loopback.local_addr().unwrap().port();
    assert!(TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok());
}
```

The duplicate-edge probe reuses the new regression's synthetic SCIP artifact
and graph setup on #84, calls `backend.import_scip_index(&artifact,
Some(dir.path()))` twice, then queries:

```cypher
MATCH (a:Symbol)-[r:CALLS]->(b:Symbol)
WHERE a.id = 'app.ts::caller' AND b.id = 'app.ts::helper'
RETURN count(r)
```

It returns 2 for one pair, independent of displayed-result deduplication.

The fork validation below is at source commit
`e8bb61bf297d28349a055dae4cc7930f8ccac14b`. #84 reproduction used its exact
production source with only the proposed regression appended; its five existing
caller tests passed and the new sixth test failed. The original failing result
is retained. The separate duplicate probe has exact #84 production source and
one disposable added test. No tests were skipped to obtain a passing run.

Common environment:

```sh
export CARGO_BUILD_JOBS=1 CMAKE_BUILD_PARALLEL_LEVEL=1
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_NET_OFFLINE=true INFIGRAPH_BACKEND=kuzu INFIGRAPH_NO_WATCH=1 CI=1
# Use separate disposable HOME, INFIGRAPH_REGISTRY_HOME, TMPDIR and CARGO_TARGET_DIR.
# Retain existing toolchain/Cargo caches; set the core dump limit to zero.
```

| Source / check | Exact command | Result |
| --- | --- | --- |
| #84: pr84-receipt-regression | `cargo test --locked --offline -p infigraph-core --test typescript_callers reindexing_unchanged_files_invalidates_compiler_enrichment -- --exact --test-threads=1` | FAIL; exit 101; 0 passed / 1 failed / 0 ignored; 37.57s including build |
| #84: pr84-core-callers | `cargo test --locked --offline -p infigraph-core --test typescript_callers -- --test-threads=1` | FAIL; exit 101; 5 passed / 1 failed / 0 ignored; 87.91s including build |
| #84: pr84-scip | `cargo test --locked --offline -p infigraph-core --lib scip::tests:: -- --test-threads=1` | PASS; exit 0; 12 passed / 0 failed / 0 ignored; 30.1s including build |
| #81: pr81-diagnostics | `cargo test --locked --offline -p infigraph-core --lib diagnostics::tests:: -- --test-threads=1` | PASS; exit 0; 10 passed / 0 failed / 0 ignored; 62.24s including build |
| #82: pr82-embedding | `cargo test --locked --offline -p infigraph-core --test embedding_format -- --test-threads=1` | PASS; exit 0; 2 passed / 0 failed / 0 ignored; 41.61s including build |
| #78: pr78-loopback | `cargo test --locked --offline -p infigraph-mcp --lib web::tests::test_mcp_http_server_binds_loopback_not_wildcard_by_default -- --exact --test-threads=1` | CANCELLED before test execution; uncached lbug 0.16.0 |
| #78: pr78-test-context | `cargo test --locked --offline -p infigraph-cli --test cli_parity test_context_runs_in_the_cli_process -- --exact --test-threads=1` | NOT_RUN; native prerequisite not built |
| #84 + disposable probe: pr84-duplicates-probe | `cargo test --locked --offline -p infigraph-core --test readiness_probe -- --test-threads=1 --nocapture` | PASS; exit 0; 1 passed / 0 failed / 0 ignored; 37.33s including build |
| fork `e8bb61b`: fork-callers | `cargo test --locked --offline -p infigraph-core --test typescript_callers -- --test-threads=1` | PASS; exit 0; 6 passed / 0 failed / 0 ignored; 119.25s including build |
| fork `e8bb61b`: fork-index-status | `cargo test --locked --offline -p infigraph-core --lib index_status::tests:: -- --test-threads=1` | PASS; exit 0; 2 passed / 0 failed / 0 ignored; 31.53s including build |
| fork `e8bb61b`: fork-cli-index | `cargo test --locked --offline -p infigraph-cli --bin infigraph index:: -- --test-threads=1` | PASS; exit 0; 35 passed / 0 failed / 0 ignored; 86.85s including build |
| fork `e8bb61b`: fork-cli-process | `cargo test --locked --offline -p infigraph-cli --test doctor -- --test-threads=1` | PASS; exit 0; 4 passed / 0 failed / 0 ignored; 40.81s including build |
| fork `e8bb61b`: fork-mcp-process | `cargo test --locked --offline -p infigraph-mcp --test diagnose --test coverage_reporting -- --test-threads=1` | PASS; exit 0; 1 passed / 0 failed / 0 ignored, 5 passed / 0 failed / 0 ignored; 136.27s including build |
| fork `e8bb61b`: fork-format | `cargo fmt --all -- --check` | PASS; exit 0; 1.93s including build |
| fork `e8bb61b`: fork-clippy-core | `cargo clippy --locked --offline -p infigraph-core --lib -- -D warnings` | PASS; exit 0; 13.31s including build |
| fork `e8bb61b`: fork-diff | `git diff --check` | PASS; exit 0; 0.02s including build |

Check wall times include compilation and fixture execution; they are not
repository benchmark comparisons. The std-only wildcard probe above separately
exited 101 as expected. The ephemeral `readiness_probe` source is described
above; it is not an added repository test or an acceptance gate.

The fork source fix is commit
`e8bb61bf297d28349a055dae4cc7930f8ccac14b`; the report is a separate docs commit.
Commit hooks are not invoked: the authorized focused checks are recorded here
instead of running the broad hook performance gates in the working checkout.
This is not a claim that those gates passed.

## Open issues and recommendation

- [#48](https://github.com/intuit/infigraph/issues/48): working-tree/index freshness,
  still OPEN; #84 explicitly does not complete it.
- [#79](https://github.com/intuit/infigraph/issues/79): read-only registration
  inspection API, still OPEN; #81 deliberately leaves configuration UNKNOWN.
- [#80](https://github.com/intuit/infigraph/issues/80): combined-document allocation
  abort, still OPEN; #82 supplies the bounded count fix, pending upstream landing.
- Native compression SIGILL, duplicate graph edges, unpropagated import failures,
  single-file mutation receipts and full workspace/platform coverage remain
  distinct from successful focused checks. Historical full-suite/Clippy reports
  in PR bodies were inspected, not relabeled as fresh results.

Confidence: high for the reviewed source paths and reproduced fixture findings;
medium for overall integration readiness. Recommend the narrow fork receipt fix
for review, separate upstream prerequisite review, and HOLD a blanket upstream
merge-ready/full-freshness claim. Do not force merge, release or activate runtime.

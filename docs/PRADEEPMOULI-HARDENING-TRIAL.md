# Corrected-source hardening trial (2026-10-24)

## Executive decision

Trial two narrow adaptations from the user-specified
[pradeepmouli/infigraph `feat/hardening`](https://github.com/pradeepmouli/infigraph/tree/d1b8bc0edc1e4645d11e51c3370ad27e10c0591c):
bounded cache parsing and non-destructive refusals for busy/incompatible stores.
These address reproduced process aborts, a parser panic, and database loss in
current Afterimage. They do not require the source fork's daemon architecture,
database upgrade, integration hooks, or file-format changes.

This evaluation is separate from [Afterimage PR #10](https://github.com/redxzeta/afterimage/pull/10).
That draft evaluated the different provisional source
`intuit/infigraph:fix/issue-48-index-freshness`; it is not the requested source
trial. No commits from #10 were used. Leave #10 draft and unmerged; recommend
closing it as superseded by the corrected source request unless its independent
freshness experiment is explicitly wanted later. This task does not close or
merge it.

## Exact identities and comparison method

| Role | Full commit |
| --- | --- |
| Supplied and fetched `pradeepmouli/infigraph:feat/hardening` | `d1b8bc0edc1e4645d11e51c3370ad27e10c0591c` |
| Initial `redxzeta/afterimage:afterimage-main` failure baseline | `de02e762c5581055d2adcefe7efe1a58bc0c7cab` |
| Refreshed final delivery base `afterimage-main` | `f4568cf15e004391f549bc8705112e37502bd3dc` |
| Fresh `intuit/infigraph:main` | `cf82f5d7440d9be158e219df1ef256d6359ed1f6` |
| Fresh source fork `main`, also the merge-base with both comparisons | `c54d13768b5c5ee06c8e122b60e9512351f8d8db` |

The dedicated trial branch is
`experiment/afterimage-pradeepmouli-hardening-20261024`, starting clean at the
Afterimage baseline, with only `redxzeta/afterimage` configured as `origin`.
The source and upstream were fetched by explicit branch ref through read-only
HTTPS URLs into separate review refs. No source push remote was added.

The stale source `main...feat/hardening` comparison has 1,095 commits ahead and
zero behind. The initial Afterimage comparison was 52 versus 1,095 commits relative to
their merge-base. The initial merge-base diff spans 576 files, 185,229 insertions and
9,683 deletions; a direct Afterimage/source tree comparison spans 610 files.
This is a semantic shortlist, not a wholesale audit or approval of 1,095 commits.
History subjects, the source's `docs/DESIGN-hardening.md`, and selected
implementation/commit diffs were compared against both current trees. The
design document's shipped claims were treated as leads, not proof.

## Feature decisions

Severity describes the underlying failure if reached; risk/complexity describe
porting to current Afterimage. Source references below are immutable commits in
the corrected source repository.

| Category / source evidence | Decision | Severity; port risk / complexity; expected value |
| --- | --- | --- |
| Cache counts and structural bounds: [`7908e29`](https://github.com/pradeepmouli/infigraph/commit/7908e299a3caf7796d4c809634b4db663f8cc2c8), [`106184a`](https://github.com/pradeepmouli/infigraph/commit/106184a396b517f12c09f3c1974f57cd136d80c8), core `embed/mod.rs`, `search/mod.rs` | **Adapt** the remaining BM25/HNSW gaps and overflow-safe embedding lengths. Embedding count bounds are **already present** in Afterimage's `embedding_format.rs` protection. Do not import the source's generic reader or newer embedding format. | High availability impact; low / small; malformed local caches return errors instead of panic/abort. |
| Storage-version refusal: [`e4257a1`](https://github.com/pradeepmouli/infigraph/commit/e4257a123b8ae74c0ab838098e50f74862620702), core `lib.rs`, `graph/store.rs` | **Adapt**, covering initial and retry opens. Preserve the native cause and both storage versions in the error chain. | High data-integrity impact; low / small; incompatible data remains available to a compatible binary. |
| Busy/incompatible docs and combined stores: [`de33d9c`](https://github.com/pradeepmouli/infigraph/commit/de33d9cfb2809fe0a34f4375bb80a46e515a1253), core `multi/combined.rs`, docs `lib.rs`, `combined.rs` | **Adapt** the same shared guard at every existing wipe-on-open path. The source's broader WAL-race classification is deferred. | High data-integrity impact; low to moderate / small; a live document writer no longer loses its pathname to automatic cleanup. |
| Code-graph lock refusal: [`fc40d09`](https://github.com/pradeepmouli/infigraph/commit/fc40d09027b9bbe761bb4b2ddab462abd93bdcce), core `lib.rs` | **Already present** in Afterimage/upstream. Move the existing predicate beside the store to reuse it; preserve reader retry behavior. | High underlying impact; low reuse risk; avoid duplicate logic. |
| Worker crash containment: [`2d10693`](https://github.com/pradeepmouli/infigraph/commit/2d10693f49cc99961195d6504c13e9c4683fb643), MCP `main.rs`, `recovery.rs` | **Already addressed differently** by Afterimage #4/#7. The source worker now relies on an owning daemon; importing it alone would remove useful existing local recovery. | High underlying impact; high architecture mismatch; retain bounded launch-project recovery. |
| Atomic cache writes: [`3027d39`](https://github.com/pradeepmouli/infigraph/commit/3027d391faef7c4a36fcfdaf0e7ab45520e4f0ea), [`566b656`](https://github.com/pradeepmouli/infigraph/commit/566b6565653746614c689163cc6bf1f8a0574baa), `embed/mod.rs`, `search/mod.rs` | **Defer** as a coherent publication-consistency change. Do not copy fixed temporary filenames or confuse rename atomicity with power-loss durability. | Moderate integrity impact; moderate / medium; useful next candidate, but separate from safe reading. |
| Session atomicity and lost updates: [`82fe10e`](https://github.com/pradeepmouli/infigraph/commit/82fe10e2917a089cd76a9bdf17ea93acc3a66b1d), core `graph/session_store.rs`, MCP `tools/session.rs` | **Defer**. Afterimage still uses truncate/write; the source also changes locks, timestamp tolerance and degraded-mode reporting. Review the entire read/modify/write contract separately. | Moderate integrity impact; moderate / medium; valuable independent follow-up. |
| Process deadlines/group cleanup: [`59eaa10`](https://github.com/pradeepmouli/infigraph/commit/59eaa105fb2a2b8ff5ff326a25c7423781c651e7), [`5a5af3b`](https://github.com/pradeepmouli/infigraph/commit/5a5af3bd493d979b05f3e677a680f20c2cdc0ce7), core `child.rs`, CLI `index.rs`, plugin drivers | **Defer**. Valuable cancellation and bounded stderr handling, but requires child ownership, late-response and platform review. No source child/indexer/plugin was launched. | Moderate availability/process impact; moderate to high / medium; separate fixture-based project. |
| MCP unknown/path-like arguments: [`3155a03`](https://github.com/pradeepmouli/infigraph/commit/3155a030d9b077b29f2e20376cb82097cac04c54), MCP `lib.rs`, `tools/helpers.rs` | **Defer**. Refusing `project=` when the real key is `path` is useful, but the source assumes centralized project scoping and also changes schemas and search argument behavior absent here. | Moderate wrong-project impact; moderate / medium; requires a separate API compatibility sweep. |
| Index/watcher ignore rules: [`99175e0`](https://github.com/pradeepmouli/infigraph/commit/99175e05ce92b69a70c8977abf03a805b55c6df6), `ignore_rules.rs`; `81c52a1`, `87286a5`, watch sinks | **Defer / too large**. The source's centralized walkers and rescan sinks differ substantially from Afterimage. Preserve #7's language-registry and ignore-boundary corrections. | Moderate correctness impact; high / large; no watcher rewrite in this trial. |
| Quarantine, snapshots, dirty logs, storage generations: `9b736a7`, `32cd7f5`, `2bdfeab`, core `snapshot.rs`, `quarantine.rs`, `dirty.rs`, graph/watch code | **Too large**. These introduce retention/recovery semantics and interact with the source daemon and newer lbug. | High potential integrity value; high / large; do not transplant piecemeal. |
| Diagnostics / health: `1316d6b`, `0eaf7be`, source `doctor.rs`, MCP `health.rs`, `tools/search.rs` | **Defer**. Useful diagnostics depend on the source's instance registry, daemon, generations and dirty state. Avoid overlapping #10's unrelated freshness advisories. | Moderate diagnostic value; high / large; targeted diagnostics can be proposed later. |
| Security rules / path cleanup: `ca47ef5`, `3b56eeb`, core `security/rules.rs`, CLI worktree cleanup | **Defer** module-system coverage; **reject for this trial** new worktree cleanup commands. No deletion command was imported or executed. | Context-dependent; moderate / medium; no reason to add new destructive surfaces here. |
| CI, toolchain, hooks and agent integrations: source `.github/workflows/ci.yml`, `rust-toolchain.toml`, `assets/hooks`, CLI integration resources, `AGENTS.md` | **Reject for this trial**. Fork-specific routing, install/session policies and numerous integrations are outside the two fixes. | Potential operational/security impact; high / large; no new operational burden or inherited agent instructions. |
| Broad daemon/socket/lease architecture, newer lbug, graph renaming, docs linking, clone detection, language features | **Too large / unrelated**. No merge, dependency refresh, schema migration or format upgrade. | High integration risk; very large; evaluate only with a separate explicit goal. |

## Implementation and rationale

1. `crates/infigraph-core/src/search/mod.rs`: bound every BM25 length before
   slicing, each count before capacity reservation, and every posting's document
   index before admitting it. Local closures keep the change within the existing
   loader; no generic binary-format framework was introduced. Valid search scores
   and lossy string decoding are unchanged.
2. `crates/infigraph-core/src/embed/mod.rs`: bound HNSW ID count by remaining
   bytes before reservation. Validate embedding dimension against remaining
   bytes before multiplication, and use subtraction for length checks. Retain
   existing headerless embeddings and binary/JSON HNSW formats.
3. `crates/infigraph-core/src/graph/store.rs`: share the existing lock classifier
   and the source's storage-version refusal with docs and combined-store callers.
   Inspect the error chain so a context wrapper cannot hide a native refusal.
4. Core `lib.rs`, `multi/combined.rs`, docs `lib.rs`, `combined.rs`: return the
   recognized error before cleanup/rebuild, including errors first observed on a
   retry. Retain original causes; do not replace an incompatible store with an
   apparently healthy empty graph or schedule a destructive combined-doc rebuild.

The native lbug 0.19.1 source (`storage/database_header.cpp`) was checked for the
exact version-refusal text before using it. This still depends on error text;
lbug does not expose the required typed error here. No broad claim is made that
all open failures are now safe: other automatic corruption recovery remains as
before. A WAL race, permissions failure, native engine crash, concurrent mmap
mutation, or very large structurally valid cache is outside these fixes.

This improves failure containment and preservation; it adds no `fsync`, snapshot
or transactional-publication guarantee. No power-loss or cross-file crash
consistency claim is made.

## Validation

### Baseline red cases and candidate results

All corpora, DB files, caches, HOME and registry locations used for application
tests were disposable. No source-fork binary, installation script, integration
hook or test setup was executed. Forkara and production indices were not used.

| Check | Exact result |
| --- | --- |
| Baseline BM25 truncated-prefix regression | **FAIL as expected**, 0 passed / 1 failed: slice end 13 beyond length 9. |
| Baseline BM25 impossible document count | **ABORT as expected**, SIGABRT under 8 GiB address-space limit, allocation request 206,158,430,160 bytes. |
| Baseline HNSW impossible ID count | **ABORT as expected**, SIGABRT under the same limit, allocation request 103,079,215,080 bytes. |
| Baseline incompatible code/docs/combined-code stores | **FAIL as expected**, 0 passed / 3 failed: rebuilt or removed existing stores. |
| Baseline document store held by a second process | **FAIL as expected**, 0 passed / 1 failed: database bytes changed, despite the eventual error. |
| Initial consumer regression suite | **PASS**, 41 passed / 0 failed: 10 new test cases (including the subprocess-holder helper), embedding-format 2, existing hybrid/search 21, existing write-lock 8. |
| Native docs test target `hardening_trial` | **PASS**, 4 passed / 0 failed; these four also run in the consumer suite above. |
| Existing native `combined_docs` | **PASS**, 5 passed / 0 failed. |
| Existing native `tool_parity` | **PASS**, 8 passed / 0 failed, 1 deliberately filtered: `dispatch_handles_all_mcp_tool_names`. |
| Existing native concurrent writer/read-only reader | **PASS**, 1 passed / 0 failed, 12.939 s wrapper wall time. |
| Existing modified-code watcher | **PASS**, 1 passed / 0 failed, 7.890 s. |
| Existing new-code-file/new-directory watcher | **PASS**, 1 passed / 0 failed, 10.794 s. |
| Existing deleted-document pruning watcher | **PASS**, 1 passed / 0 failed, 1.206 s. |
| Existing supervisor containment harness | **PASS**, 8 subprocess scenarios and 2 unit tests; 9.175 s wrapper wall time. |
| CLI + MCP native binary build | **PASS**, offline/locked/no-default-features. |
| Strict Clippy, core/docs/MCP/CLI libraries and binaries | **PASS**, `-D warnings`. |
| Workspace Rust formatting | **PASS**, `cargo fmt --all -- --check`. |
| Final whitespace diff check | **PASS**, `git diff --check`. |

The new core tests cover every truncated prefix of a valid BM25 cache,
impossible document/term/posting counts, an absent posting document, valid score
preservation, invalid HNSW counts, and current binary/legacy JSON HNSW lookup
compatibility. Native store tests stamp an otherwise valid fixture's storage
version at bytes 4..12 with `kuzu::get_storage_version() + 1` and require the
same bytes afterward. Combined-doc query/search also preserve a sidecar sentinel.
The live-lock test spawns the same test executable, waits for a ready marker,
then kills/reaps only its owned child on completion or assertion failure.

No performance speedup is claimed. The A/B evidence is correctness under
identical malformed inputs and storage-version fixtures, not a throughput
benchmark. Allocation-request figures are requests rejected under a finite
address-space cap, not resident memory consumed.

### Commands, environment and reproducibility

Production compilation and lint commands executed:

```sh
cargo build --offline --locked --no-default-features \
  -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp
cargo clippy --offline --locked --no-default-features \
  -p infigraph-core -p infigraph-docs -p infigraph-mcp -p infigraph-cli \
  --lib --bins -- -D warnings
cargo fmt --all -- --check
git diff --check
```

Native regression targets were compiled with:

```sh
cargo test --offline --locked --no-default-features \
  -p infigraph-docs --test hardening_trial --test combined_docs \
  -p infigraph-mcp --test tool_parity --test tool_dispatch \
  --test watcher_reindex --test concurrent_writer_reader --no-run
```

Each resulting executable then ran separately with `--test-threads=1` under
`unshare -Urn`, a wall-clock/RSS controller and isolated fixture directories.
`tool_parity` additionally used `--skip dispatch_handles_all_mcp_tool_names`;
each watcher invocation used its exact test name and `--exact`.
The omitted parity case dispatches every tool with empty arguments and can invoke
real side-effecting handlers against its working directory; static schema/CLI
mapping parity is a separate result. MCP fixture executables were copied away
from the CLI executable so `find_infigraph_cli` exercised the existing inline
fallback, avoiding automatic indexer installation. Native CLI/MCP compilation
and static CLI mapping parity do not establish end-to-end CLI/MCP output parity.

The ordinary core test command was attempted first:

```sh
cargo test --offline --locked --no-default-features \
  -p infigraph-core -p infigraph-docs --test hardening_trial --no-run
```

It is **BLOCKED** by absent cached `cargo-husky v1.5.0`. No dependency was
downloaded, removed or replaced, and no hooks were installed. Instead, a
disposable consumer package depended on the actual checkout's `infigraph-core`
and `infigraph-docs`, `kuzu = { package = "lbug", version = "=0.19.1" }`,
`tempfile = "3"`, `anyhow = "1"`, and `fs2 = "0.4.3"`; it reused the checkout's
`esaxx-rs` path patch and linker configuration. Its test modules included the
five exact repository test files by `#[path]`: both `hardening_trial.rs` files
and core `embedding_format.rs`, `search_hybrid.rs`, `write_lock.rs`.
It began from the repository lockfile and resolved offline for
`x86_64-unknown-linux-gnu`; the package/version set differs only by the temporary
consumer package itself. Then `cargo test --offline --locked --manifest-path
<consumer>/Cargo.toml --no-run` compiled the 41 cases without activating core's
unavailable dev-only hook dependency. No test source was altered for the harness.

An initial run mistakenly put compilation under the 2 GiB runtime RSS cap;
the controller stopped the linker before tests began. Compilation and execution
were subsequently separated. Extending the temporary harness initially omitted
its direct `fs2` import; that manifest was corrected without repository changes.
A repeat run exposed two new fixtures using the registry override where the
combined-store helper actually uses HOME. Both tests now own a temporary HOME
and restore it with a drop guard; final 41/41 and native docs 4/4 are after that
correction. These were harness/fixture failures, not waived product assertions.

### Resource and provenance limits

- Platform: Linux x86_64; Rust `1.99.1` (`58cf8f9c35b355199727627d07ae34a4e2846d67`).
- Builds used two Cargo jobs, two CPU affinity slots, nice level 10, disabled
  core dumps, 8 GiB per-process address space, sampled 10 GiB process-group RSS
  cap, 4 GiB per-file cap, and finite 30–900 second deadlines. The main native
  CLI/MCP build finished in 139.950 s with about 4.20 GiB sampled group RSS;
  strict Clippy took 66.300 s and about 1.28 GiB.
- Native graph tests used 32 TiB virtual address space because lbug reserves a
  large sparse address range; this is not resident usage. The sampled group RSS
  cap was 2 GiB, per-file cap 256 MiB, and each selected suite had a 60–90 second
  deadline. Final consumer tests took 2.410 s / about 337 MiB; native docs took
  0.463 s / about 247 MiB. The initial successful runtime suites stayed below 600 MiB; the post-rebase
  49-case consumer suite stayed below 700 MiB.
- Later runtime suites used a network namespace with no external network route.
  This is not a claim of filesystem, socket, credential or full OS isolation.
  `INFIGRAPH_BACKEND=kuzu`, `INFIGRAPH_NO_WATCH=1`, `HF_HUB_OFFLINE=1`, disposable
  HOME/registry/TMPDIR, and disabled cargo-husky installation were set locally.
  Explicit watcher tests still start their own fixture watchers. No listeners
  were started by this task's application checks.
- The toolchain and caches were retained from prior authorized local work and
  copied to owned scratch outputs; no native build from another thread was
  active. Initial capacity was about 59 GiB available RAM and 1.3 TiB free disk.
- The Cargo crate is locked to lbug 0.19.1. Its retained prebuilt static archive
  has SHA-256 `9100591f7cf1de0f565c96b55e3293998763260fc936bfbb9e304bef6f4ec2b8`;
  it reports native storage version 47. The crate cache calls its artifact
  `latest`; this run reused it offline, did not establish an immutable upstream
  backend release identity, and did not rebuild the C++ backend from source.
- Resource measurements are sampled process-group RSS, not kernel-enforced
  aggregate limits. Detailed commands, result JSON and synthetic-test logs remain
  in the task-owned local evidence directory, outside Git; they contain local
  compiler paths and are intentionally not committed.

### Remaining failures and unrun gates

The existing native `tool_dispatch::test_graph_tools` exceeded its 90-second
deadline on the candidate (90.038 s, about 474 MiB sampled RSS). To investigate,
the exact `de02e762c5581055d2adcefe7efe1a58bc0c7cab` baseline was exported with
`git archive` into a disposable source directory, compiled offline/locked, and
its same unchanged test ran under the same network/CPU/memory controls in a
fresh HOME/registry/TMPDIR. It also timed out (90.049 s, about 470 MiB), with no
completed assertion. This establishes a pre-existing bounded-test failure under
these conditions, not its root cause or a pass. A fresh candidate repeat also timed out at 90.040 s (about 438 MiB).
The post-rebase repeat likewise timed out at 90.041 s. No test was weakened, given a larger
deadline, ignored in source, or relabeled successful.

Full workspace tests and all-target Clippy were not run; the missing cached
core dev dependency remains a real limitation. Most watcher cases were not run:
only the three named scenarios above were selected. macOS, Windows, remote
Neo4j/Postgres, optional `kompress`, HTTP transport, real external indexers,
power-loss durability and production indices are unverified. Native HNSW index
contents/dimensions and arbitrarily large structurally valid files remain
outside this bounded parser adaptation. The source's wider WAL/recovery changes
were not imported. No platform-wide or production readiness claim follows.

The existing workflows select `main`, not `afterimage-main`, for PR validation.
Consequently an absence of hosted checks on this draft is expected from current
routing and does not mean CI passed. CI PR #5 remains independent. Graph PR #9 and benchmark PR #11 merged
independently during this task and are preserved in the final delivery base. No CI/routing changes were made to get a green status.

### Refreshed-base integration validation

While this task ran, independently reviewed #9 and #11 merged. The trial was
rebased without conflicts from `de02e76` onto
`f4568cf15e004391f549bc8705112e37502bd3dc`; none of the seven adapted
implementation files changed in that base update. The source comparison still
has merge-base `c54d137`; the refreshed base/source history count is 69 / 1,095.
The implementation patch remains exactly nine files, +353 / -57 lines.

The consumer was extended with the unchanged current
`crates/infigraph-core/tests/typescript_imports.rs` and its existing path
`infigraph-languages`, `protobuf = "3"` and `scip = "0.7.1"` dependencies.
Again the package/version set differs from the repository lock only by the
consumer package. After adding those direct harness imports, locked compilation
passed. No product dependency or test file was modified for this extension.

| Post-rebase check | Result |
| --- | --- |
| Consumer: prior 41 cases plus all 8 merged import regressions | **49 passed / 0 failed**, 19.778 s, about 696 MiB sampled RSS |
| Native docs hardening | **4 passed / 0 failed**, 0.503 s |
| Combined docs | **5 passed / 0 failed**, 8.052 s |
| Concurrent writer/reader | **1 passed / 0 failed**, 13.671 s |
| Modified-code watcher | **1 passed / 0 failed**, 7.581 s |
| New-file/new-directory watcher | **1 passed / 0 failed**, 11.543 s |
| Deleted-document pruning watcher | **1 passed / 0 failed**, 1.218 s |
| CLI/MCP binary rebuild | **PASS**, 12.596 s |
| Production-library/binary strict Clippy | **PASS**, 2.777 s |
| Native CLI `index` against an incompatible owned fixture | **PASS**, expected exit 1 in 3.203 s; database SHA-256 and sibling sentinel preserved |
| Broad native `tool_dispatch` | **TIMEOUT**, 90.041 s, about 439 MiB; still not passed on refreshed base |

After that rebase, the target advanced again to
`7cf421a61e0838daa6b29f2e341c85241221b032` through #12. That update changes only
`docs/AFTERIMAGE_ACCEPTANCE.md`; source, Cargo configuration and workflows are
byte-identical to the tested delivery base. It is preserved as independent
base-side work; this trial does not rewrite its acceptance document. No further
code rebase or repeated identical testing is needed for that documentation-only
advance.

## Separate self-review pass

This was a second review pass by the implementing agent, not an external-agent
approval or a review of every source-branch change.

- **High, fixed:** reserve-before-validation and unchecked BM25 slicing allowed
  process abort/panic. Remaining-byte bounds now precede allocation/read, and
  invalid posting indices cannot reach search. The red cases and final
  compatibility cases exercise this boundary.
- **High, fixed:** storage-version and document-lock errors reached wipe paths.
  All five existing caller boundaries now use the same refusal classification;
  code init also checks the retry branch. Native tests verify byte preservation
  for initial version refusals and document lock contention; a version mismatch
  that first appears during backoff was inspected statically, not fault-injected.
- **Moderate, fixed in tests:** the registry override did not isolate combined
  paths. Both new group tests now own and restore HOME; the final repeated run
  passes. This correction did not change application path resolution.
- **Moderate, retained limitation:** lbug error classification is textual, the
  cached backend lacks a freshly established release identity, and the existing
  dispatch timeout remains unresolved. These limit acceptance beyond this draft.
- **Low, fixed:** database comparisons originally dumped whole byte arrays on
  failure. Tests now report a concise byte-preservation assertion. The subprocess
  fixture owns and reaps its child even on failure.

Final source review found no remaining high/moderate defect introduced by these
two adaptations. It confirmed no changes to manifests/lockfile, workflow files,
hooks, watcher code, supervisor code, deployment, permissions, production
configuration, signing, services, schedules or broker surfaces. Added-text checks
found no credential material or private home paths. The production diff is seven
files, 102 added / 57 removed lines; the implementation commit including two
test files is nine files, 353 added / 57 removed lines. Git commit/push hooks
are disabled only for the commands in this trial, consistent with not executing
repository-sourced hooks; the manual check results above remain explicit.

## Delivery, trial and rollback

Implementation commit:
[`05b40a8978273a59fbb7475a93e6034ccef3a4c8`](https://github.com/redxzeta/afterimage/commit/05b40a8978273a59fbb7475a93e6034ccef3a4c8).
The final test sources and implementation match this commit; documentation-only
commits may follow. The compiled dirty-tree build label is not a release or an
installation identity. The draft PR targets `afterimage-main`, with the report
and exact hosted-check readback linked in the delivery update.

Recommendation: **adopt these two adaptations for review as a draft trial**;
defer merge acceptance until the reviewer accepts the scoped evidence and the
dispatch-timeout disposition. Do not merge the source fork wholesale. Prioritize
atomic sidecar/session publication and bounded subprocess ownership as separate
future evaluations, followed by MCP argument/project scoping compatibility.

To try the code, check out the trial branch in a disposable checkout, build with
the locked dependencies, and run the two `hardening_trial` targets serially
against disposable HOME/registry/TMPDIR. The ordinary core target needs its
existing dev dependency available; the offline consumer method above records
the narrower substitute used here. Keep any broader native tests time/RSS
bounded. No installation or live index migration is required by this PR.

To roll back this trial, close the draft/discard the trial branch. Nothing was
installed, deployed or activated and no live index was touched. If separately
adopted later, reverting the implementation commit is a source rollback with no
schema/file-format migration; runtime binary changes would be a separate
authorized action. Reverting also restores the older unsafe failure behavior,
so it is not a data-recovery procedure. A version refusal should be handled by
using a compatible binary, not deleting the preserved store.

Nathan acquires no new service, container, credential, database, scheduler,
permission model, monitoring task, recovery mechanism or deployment obligation
from this change. Local compilation scratch data is disposable; no recurring
work was scheduled. No merge, release, tag, package publication or deployment
was performed.

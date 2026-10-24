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
- During final readback the coordinator merged PR #7, advancing the base to
  `585fd5d9221ced25a8bc2307c0b0e5fd9ff49010`. Only the experiment branch was
  updated, by normal merge `f2efc043cab718c235f4b2a3a6cfbe4c61e8ee52`;
  no force push or integration merge was performed by this trial.
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
| Watch-status formatting | Defer | It is not needed for the four-tool advisory. Existing status/doctor contracts remain unchanged. |
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

## Current-base combined validation

After these native checks, the target advanced again to
`de02e762c5581055d2adcefe7efe1a58bc0c7cab`, adding only
`docs/AFTERIMAGE_ACCEPTANCE.md` through PR #8. That documentation-only base was
incorporated into this experiment with a normal merge. The entire `crates/` tree
is identical to the tested combined checkpoint, so native checks were not
repeated for that documentation change. Evidence below remains bound to its
actual tested SHA rather than being relabeled as a rerun.

The following supersedes the historical watcher failure for the final trial
branch. The coordinator independently merged PR #7 during final readback. The
trial incorporated that base in its own branch, reacquired explicit Sable idle
clearance, and reran targeted checks at:

- Base: `585fd5d9221ced25a8bc2307c0b0e5fd9ff49010`.
- Combined code/test checkpoint: `f2efc043cab718c235f4b2a3a6cfbe4c61e8ee52`.
- Trial delta against this base: the same seven MCP/test/report files; no core,
  watcher, CLI, CI, dependency or operational changes in the trial diff.

CPU 0/1, two build jobs, isolated fixture execution, original test/12-second
watcher deadlines, offline dependencies and the same 8 GiB RSS cap were retained.
No full-workspace or performance benchmark was repeated.

```sh
cargo test --locked --offline --no-default-features -p infigraph-mcp \
  --test freshness_tools --test coverage_reporting --test watch_new_directory \
  --no-run --message-format=json
cargo test --manifest-path target/freshness-trial/base-585/Cargo.toml \
  --locked --offline --no-default-features -p infigraph-mcp \
  --test freshness_tools --no-run --message-format=json
cargo clippy --locked --offline --no-default-features -p infigraph-mcp \
  --lib --bin infigraph-mcp --test freshness_tools --test watch_new_directory \
  -- -D warnings
cargo build --locked --offline --no-default-features \
  -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp
```

Compiled executables were run from a disposable CWD:

```sh
watch_new_directory-da2f1310091b99bc --test-threads=1 --nocapture
freshness_tools-c36c7fd8b8ff5520 --test-threads=1
coverage_reporting-1171ed4747198ffc --test-threads=1
```

| Combined check | Exact result | Process wall seconds |
| --- | --- | ---: |
| Targeted test build | PASS, exit 0 | 20.653 |
| Four inherited watcher regressions | 4 passed, 0 failed | 36.401 |
| Final freshness regressions | 2 passed, 0 failed | 118.316 |
| Coverage reporting | 1 passed, 0 failed | 28.936 |
| Exact current-base archive build | PASS, exit 0 | 18.940 |
| Same freshness tests on exact current base without adaptation | 0 passed, 2 failed; expected exit 101 | 6.758 |
| Refreshed strict focused Clippy | PASS, exit 0 | 4.706 |
| Refreshed CLI/MCP build | PASS, exit 0 | 15.030 |
| Combined primary-worker watcher smoke | 14 passed, 0 failed; exit 0 | 40.188 |
| Final formatting/whitespace | PASS, exit 0 | Formatting 0.760 |

**Seven targeted tests pass on the combined branch.** The earlier 105-test
focused result belongs to the pre-#7 checkpoint below; it is not relabeled as a
full rerun on the combined head.

The four direct watcher tests cover populated directories, return checkout,
hidden ancestors with project ignores, and queued ignore-rule changes. Exact
return contents were observed after **2.031128054 s**, within the original
12-second deadline. Their captured polling log contains **four unsuccessful
native reads**: three checkpoint-in-progress errors and one missing-WAL read.
Subsequent correct snapshots do not establish error-free reads. No WAL repair,
error suppression, assertion weakening or deadline increase was performed.

The separate real-primary MCP smoke passed all 14 checks and recorded zero read
errors in that small sample:

| Combined watcher observation | Result |
| --- | --- |
| Existing-file edit | PASS, 3.152 s |
| Populated nested-directory checkout | PASS, 3.152 s |
| File deletion | PASS, 3.103 s |
| Exact return graph | PASS, 6.370 s |
| Advisory during edit/branch-switch/deletion | PASS |
| CLI help/index/doctor JSON, watcher registration, MCP EOF | PASS |

The populated-directory improvement comes from the inherited #7 fix, not this
advisory adaptation. General error-free concurrent reads remain unproven.
Combined smoke input binary SHA-256 values:

- `cli`: `1f762c47ef673d1c93daa7817ff816f17a8fc81560bf161e2258198e1371e11e`
- `candidate_mcp`: `42800d3db43a6c11b9f8b5333bc397e84ab3e630e01771e821ca04bb58b44143`
- `base_mcp`: `e90f2217ddf7b895f76411b41a2166969b740a601b6e2cc28c6f2acf1a2f8410`

The Sable slot was explicitly released again after these checks; only report,
formatting and Git readback work followed.

## Historical validation on pre-PR-7 base `4d5266f`

The benchmark hold was explicitly released at 15:31:58 UTC on October 24.
The PR #7 owner subsequently handed over an idle Sable slot. All native work
ran serially in that slot, on CPU 0/1, with two Cargo/Rayon jobs, offline Cargo,
disposable HOME/registry/fixtures, core dumps disabled, per-command timeouts and
an 8 GiB sampled process-group RSS cap. No toolchain/application installation was
performed. Cached artifacts were copied into this worktree; no shared build cache
was mutated. Cargo naturally rebuilt artifacts invalidated by relocated paths.
The slot was explicitly released after native work; no owned native processes
remain. Formatting, Git checks and report edits followed.

Production-code checkpoint: `603d955318eecb16e442e2db46e91b31c42e9286`.
Final regression checkpoint: `502909f979ac03d3f1175ec5ef9f8e356f1045b9` (test-only
optimization; production code unchanged). Rust/Cargo: 1.99.1, Linux x86_64.
Native defaults were used with `--no-default-features`; optional compression and
remote backends, cross-platform suites and the full workspace remain unverified.

### Build and lint commands

```sh
cargo test --locked --offline --no-default-features -p infigraph-mcp \
  --lib --bin infigraph-mcp --test freshness_tools --test coverage_reporting \
  --test diagnose --test tool_dispatch --no-run --message-format=json
cargo test --locked --offline --no-default-features -p infigraph-mcp \
  --test freshness_tools --no-run --message-format=json
cargo test --manifest-path target/freshness-trial/base/Cargo.toml \
  --locked --offline --no-default-features -p infigraph-mcp \
  --test freshness_tools --no-run --message-format=json
cargo clippy --locked --offline --no-default-features -p infigraph-mcp \
  --lib --bin infigraph-mcp --test freshness_tools -- -D warnings
cargo build --locked --offline --no-default-features \
  -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp
cargo fmt --all -- --check
git diff --check
```

| Build/check | Result | Wall seconds |
| --- | --- | ---: |
| Initial candidate test build | PASS, exit 0 | 216.522 |
| Optimized regression rebuild | PASS, exit 0 | 7.013 |
| Exact-base archive regression build | PASS, exit 0 | 15.590 |
| Strict focused Clippy | PASS, exit 0 | 119.378 |
| CLI/MCP native build | PASS, exit 0 | 138.259 |
| Formatting/whitespace | PASS, exit 0 | Final formatting 0.761 |

Do not compare these build times as speedups: cache state and selected targets
were different. Highest sampled build RSS across both validation rounds was 8,240,975,872 bytes, below the
8 GiB cap; no command hit the RSS cap.

### Executed test commands and counts

To avoid CWD-ancestor configuration/dedup discovery and the known hidden-ancestor
watcher issue, Cargo compiled test executables without running them. The resulting
executables were then run from a visible disposable CWD with isolated HOME and
registry. Before the CLI was built, existing graph dispatch tests used the normal
inline index path; no fake CLI, automatic SCIP installation, test disabling or
weakened assertion was used.

Each executable below is under `target/freshness-trial/native/debug/deps/`:

```sh
freshness_tools-c36c7fd8b8ff5520 --test-threads=1
coverage_reporting-1171ed4747198ffc --test-threads=1
diagnose-8febb214a51cf608 --test-threads=1
tool_dispatch-08f51234d74dcd9f --test-threads=1
infigraph_mcp-09eb49583889d21c --test-threads=1
infigraph_mcp-0a435b910c151ead compress::tests --test-threads=1
infigraph_mcp-0a435b910c151ead session_context::tests --test-threads=1
```

| Test | Result | Process wall seconds |
| --- | --- | ---: |
| Final candidate freshness regressions | 2 passed, 0 failed; exit 0 | 117.139 |
| Same final regressions copied into exact base `4d5266f` archive | 0 passed, 2 failed; expected exit 101 | 7.092 |
| Existing coverage reporting | 1 passed, 0 failed | 28.421 |
| Existing diagnostics/protocol | 5 passed, 0 failed | 0.110 |
| Existing graph-tool dispatch | 1 passed, 0 failed | 97.838 |
| MCP binary units | 7 passed, 0 failed | 0.109 |
| Compression units | 61 passed, 0 failed; 63 unrelated tests filtered | 0.541 |
| Session units | 28 passed, 0 failed; 96 unrelated tests filtered | 0.109 |

Pre-#7 candidate focused tests: **105 passed, 0 failed**. The historical watcher
probe below failed on both pre-#7 base and candidate; current combined results are
reported separately above.

The base failures are substantive: ordinary structural output lacks the freshness
advisory, and reference compression removes an appended advisory. Candidate tests
cover all four routes for populated/empty and changed-HEAD results, plus edits,
deletions, nested additions, subdirectory resolution, subset indexing, return
checkout with stale graph data, pending/busy/poisoned state, malformed registry,
non-Git state, unchanged registry/coverage bytes, repeated MCP responses and
long-result compression/deduplication.

### Native CLI/MCP and watcher evidence

The local disposable Python harness executed real binaries with these CLI
commands in synthetic Lua repositories:

```sh
infigraph --help
infigraph index --no-embed
infigraph doctor --json
infigraph symbols app.lua
infigraph-mcp --mcp
```

It sent MCP `initialize`, `tools/list`, structural queries, `get_symbols_in_file`
and explicit `watch_project` requests. CLI help, seed indexing, doctor JSON,
discovery, real caller edges and EOF exit checks passed. Lua has no SCIP installer
path. Update-check cache state belonged to the disposable HOME.

The A/B phase held only the disposable MCP instance lock so both servers used
the existing secondary-instance read path, preventing auto-watch from mutating
the frozen graph. Body parity passed in **nine comparisons**: all four tools on
the clean fixture, plus `trace_callers` across dirty, changed-commit, return,
deleted-file and new-nested-file states. The candidate showed advisory evidence;
the base did not. Offline caller additions remained absent from the frozen graph
in both versions: the improvement is visible uncertainty, not graph repair.

The actual-primary watcher phase used the unchanged core watcher with real
filesystem/Git events and an absolute 12-second acceptance budget. A native call
straddling the deadline could finish later, but could not count as a pass.

| Watcher observation | Candidate | Exact-base control |
| --- | --- | --- |
| Existing-file edit converged | PASS, 3.190 s | PASS, 3.134 s |
| Populated nested-directory checkout converged within 12 s | **FAIL**; still absent at last observation, 13.583 s | **FAIL**; still absent at last observation, 13.446 s |
| File deletion converged | PASS, 3.201 s | PASS, 3.158 s |
| Exact return-checkout graph | PASS, 6.471 s | PASS, 6.317 s |
| Explicit native read errors in this sample | 0 | 0 |

Candidate A/B plus watcher harness: **67 checks passed, 1 failed**, exit 1,
187.944 s, no outer timeout. Baseline watcher-only control: **13 passed, 1 failed**,
exit 1, 50.383 s. The shared failure corresponds to unchanged core
`watch/mod.rs:245-249`, owned by separate PR #7. No #7 code was imported, and no
native WAL repair or deadline increase was attempted. These small samples do not
establish general error-free native reads; the #7 owner separately reported
transient WAL/checkpoint errors in their own tests.

### Retained unsuccessful attempts

- Initial regression fixture: outer 180-second limit reached, exit 124 at
  180.048 s, before a final test result. The fixture redundantly opened all four
  native graphs for every shared-evidence case. Commit `502909f` retained all-four
  populated/empty/changed-HEAD routing checks and used one route for the remaining
  shared helper evidence. It passed in 117.139 s under the **same 180-second bound**.
  No production code, existing test, or watcher acceptance deadline changed.
- First baseline-only harness attempt: exit 1 at 7.296 s due to cleanup of a nested
  directory that this reduced mode never created. It stopped before watcher
  startup. The harness was corrected, the failed receipt retained, and the final
  control reproduced the shared watcher failure. This was a harness error, not
  application evidence.

Raw commands, stdout/stderr, Cargo artifact records, sampled-resource receipts,
fixture scripts and A/B samples remain locally under the ignored
`target/freshness-trial/` directory. No binary, database, private machine path,
credential or transcript is included in the PR.

## Pre-PR-7 measured A/B latency and output size

One disposable Lua file, two functions, CPU 0/1, persistent MCP secondary workers,
compression/dedup disabled for body comparison, three timed requests per tool
and variant after an initial request. The clean-fixture order was base then
candidate; this is a small descriptive sample, not a controlled speedup study.

| Tool | Base median ms | Candidate median ms | Observed difference ms |
| --- | ---: | ---: | ---: |
| `trace_callers` | 3150.031 | 3163.054 | 13.024 |
| `trace_callees` | 3126.583 | 3188.716 | 62.133 |
| `transitive_impact` | 3134.265 | 3173.459 | 39.194 |
| `find_all_references` | 3128.165 | 3186.330 | 58.165 |

Each candidate response added **455 UTF-8 bytes** of advisory on the clean fixture;
its existing body remained byte-identical. The observed median differences were
13-62 ms (approximately 0.4-2.0%); do not interpret them as stable causal overhead
or extrapolate to large repositories. There is **no speedup or graph-accuracy
improvement claim**. Large-repository Git-status cost remains unmeasured.

Binary SHA-256 values for the actual smoke inputs:

- `cli`: `7c65a7020f2bdbeb98f6c05abdb76e96974f5c7be334f8e38d4b34f18de94800`
- `candidate_mcp`: `40cde425bda9a747f4c29a503c52d5d319b4a97600c1b9528883dc329a396585`
- `base_mcp`: `ee891c4b8110b2ace8aae99251e5b608a7a7cd305f034b3e84830e403a9d9aef`

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
- **Verification completed:** compilation, strict focused Clippy, 105 candidate
  tests and exact-base red regressions ran on the pre-#7 checkpoint. Seven
  targeted combined tests, refreshed strict lint/build and a 14/14 live smoke
  pass after inheriting #7. The historical populated-directory failure was
  reproduced on its base and is resolved by #7 in the combined branch. Four
  transient native read errors remain explicit in the direct-test log.
  No independent second-agent review was requested/run.

## Operational and security limits

There is no new service, schedule, dependency, credential, database, deployment
step, permission model, or ongoing operator responsibility. Core indexing,
watchers, supervisor recovery, CLI behavior, diagnostics, CI, tags, releases,
upstream branches, and other applications are outside this change.

The advisory is deliberately conservative and can remain present after a correct
index. It adds two Git subprocesses and a local registry read per affected tool
call; large repositories may incur noticeable cost. Git status excludes submodule
inspection and is not a content-hash reconciliation. Queries and observations are
not atomic. Zero pending cross-file entries is not caught-up evidence. The advisory
itself does not fix hidden-ancestor watcher omissions, offline edits, event loss,
branch-switch convergence, ignored-file freshness, or incomplete SCIP enrichment.
Only four named MCP tools gain the advisory; this is partial issue #48 coverage.

Recommendation: the narrow advisory adaptation is supported for source review and
isolated trial; reject a wholesale import of the upstream branch. Keep the PR
experimental/draft and separate from the coordinator's acceptance gate. Confirm
the intended source branch and review the final diff. The merged #7 base has now
been retested as documented above; remaining native read errors, optional modes
and large-repository cost limit broader acceptance. No PR merge, installation,
deployment or runtime authority is granted by these results.

# Phase D follow-up: Linux lifecycle and working-tree freshness

## 2026-10-24 practical-gap closure on the merged #6 base

Current base: `4d5266fe10c2c39dbaac423c7a43731329601e85`.
Code/test checkpoint:
[`b65ad6cd086f76124bc6884308c6751f50bf2b3a`](https://github.com/redxzeta/afterimage/commit/b65ad6cd086f76124bc6884308c6751f50bf2b3a).
The unchanged rebased watcher used for before tests was
`2a3c3d005d8608ca433afabf15c275939651d2f2`. The original October 23 evidence
below is retained historically; its return-checkout and hidden-ancestor gaps
are superseded by these focused results, not by a full reliability claim.

### Causes and changes

- Every removal opened a transient graph **and rebuilt the entire language
  registry**. Multiple removal notifications delayed the restored-file batch.
  Reuse the immutable registry through existing `Infigraph::open_shared`;
  graph connections still close after each operation. Advisory writer locking,
  batch writing, and #6's receipt invalidation are unchanged.
- The hidden-path filter inspected absolute components, including ancestors
  outside the project. Match relative to the canonical project root. The
  existing locked `ignore` crate supplies hierarchical `.gitignore`, `.ignore`,
  `.infigraphignore`, nested negations, and git exclude/global-file semantics.
  Existing hard exclusions remain: private dot entries inside the project,
  `.git`, `.infigraph`, build directories, and the other established names.
  Registration and new-directory discovery use those same rules. The matcher
  is refreshed per notification and before batch flush, so newly ignored queued
  files are discarded before opening the graph. Rule notifications refresh
  directory subscriptions without eagerly indexing the project.
- The initial new-directory callback could read outside-root files through
  directory symlinks and repeatedly traverse a loop. Use non-following file
  types for both recursive entries and direct directory events, and skip
  symlinks. The regression proves no external marker enters the graph, the
  loop is skipped, and the outside file bytes/mtime and directory contents stay
  unchanged. This matches the scanner's default no-follow policy.

No dependency, graph-store, lock, worker-lifecycle, timeout, or other agent's
script was changed. These changes are independent of the provisional MCP
advisory trial and contain no code from it.

### Focused serial results

All runtime cases used disposable directories, no core dumps, two CPU affinity
slots, 32 TiB virtual-address allowance, a sampled 2 GiB owned-child RSS stop,
and finite outer deadlines. Builds used two jobs and the earlier bounded build
controller. Native work began only after explicit benchmark release at
15:31:58 UTC and occupied the Sable slot before the provisional trial.

| Direct-graph case | Before | After |
| --- | --- | --- |
| Populated directory, outside symlink, loop, later edit/removal | FAIL, exit 101, 6.447 s | PASS, exit 0, 8.378 s |
| Real Git branch return plus working edit/removal | FAIL, exit 101, 27.176 s | PASS, exit 0, 13.070 s |
| Hidden ancestor, private/build/git/index exclusions, nested negation, later edits | FAIL, exit 101, 15.580 s | PASS, exit 0, 8.837 s |
| Ignore change after a source file is queued and before flush | FAIL, exit 101, 16.882 s | PASS, exit 0, 6.676 s |

The return assertion starts its **single absolute 12-second deadline before
`git checkout`**, checks the clock after graph reads, and requires the exact
symbol set. Return converged in **2.183042707 seconds**; the 13.070-second total
case includes setup, forward checkout, later edits, and deletion. No timeout
was raised. Before failures were assertions, not outer timeout or memory stops;
the symlink case explicitly found the fake outside marker and loop duplicates.

The first fixed return probe aborted on an explicit native WAL-missing read
error. The final eventual-freshness loop records failed reads and retries only
within the same absolute deadline; an error never counts as a valid snapshot.
The final return case recorded one unsuccessful read during forward checkout,
two during return, zero during the later edit, and one during deletion. Exact
snapshots arrived within their budgets. Native checkpoint/WAL read errors remain
an observed limitation; no native replay/repair fix or error-free-read guarantee
is claimed. The failed first probe remains in the local packet.

- #6 receipt regression `reindexing_unchanged_files_invalidates_compiler_enrichment`:
  **1 passed**, 3.536 s, exit 0. It preserves enrichment for a true AST no-op,
  then requires invalidation when an unchanged watcher batch rewrites symbols.
  The unchanged `crates/infigraph-core/tests/typescript_callers.rs` was compiled
  with `rustc --test`, Cargo's exact core/language/native dependencies and the
  already cached `protobuf`, `scip`, and `tempfile` crates. This avoids fetching
  the uncached core dev dependency; neither the test source nor manifests were
  modified. Exact compiler arguments are retained locally.
- Supporting native writer/reader regression: **1 passed**, 13.219 s;
  52 correct reads, zero explicit errors, zero silently wrong results in this run.
- Ordinary duplicate-watch control: **1 passed**, 15.865 s. Nine-second
  `inotify_init` delay: **expected failure reproduced**, exit 101, 13.087 s.
  The existing five-second test cleanup budget still expires while startup
  holds the watch lock. This is a timing-sensitive startup/cleanup failure,
  not proof of a lock bypass. No retry, timeout, mutex, or existing assertion was
  relaxed. This delayed-start case remains unresolved deliberately.
- Focused strict Clippy: **PASS**, 67.239 s, exit 0. Formatting and whitespace:
  **PASS**, exit 0. No full-workspace matrix was run.

The exact build/test commands from the earlier reproduction section still
apply; select the four named tests in `watch_new_directory` individually with
`--exact NAME --test-threads=1 --nocapture`. Outer limits were respectively
40/60/40/35 seconds, while freshness assertions retained 12 seconds. The verbose
Cargo build used `--offline --locked -vv` and printed the executable identities.
Receipt execution used `--exact reindexing_unchanged_files_invalidates_compiler_enrichment
--test-threads=1 --nocapture`. The writer/reader and duplicate-watch commands are
unchanged. Raw bounded JSON/logs, source identities, and compiler arguments are
retained in the Sable packet announced in the thread; none is uploaded publicly.

Residuals: transient native read failures, the delayed-start test shutdown
budget, real crash-generated WAL recovery, and Windows/macOS remain outside the
closed freshness cases. Ignore changes are snapshots, not an atomic filesystem
policy transaction or a retroactive purge of previously indexed data. No native
backend or transaction redesign, release, deployment, production access, or new
operational responsibility is introduced.

---

## Original 2026-10-23 investigation (historical)

Tested on Sable (Linux x86_64), 2026-10-23, Rust `1.99.1 (58cf8f9c3 2026-10-13)`.
This is a bounded Sable-only follow-up to [Phase D](PHASE-D-RELIABILITY.md),
using integrated fork base
[`1a59c728ae3284f0af3b5c781e14a9e2736df6e0`](https://github.com/redxzeta/afterimage/commit/1a59c728ae3284f0af3b5c781e14a9e2736df6e0).
The fork target is `afterimage-main`; `origin/main` remains the upstream mirror.
This does not establish general native-engine or cross-platform acceptance.

## Narrow fix

A real Git branch switch removed the old symbol but left a new nested module
absent from direct graph queries. The directory-event handler registered only
subdirectories, omitting the new directory itself. It also missed files created
before registration. Search-based checks can conceal this because MCP `search`
can return live-file text matches without those symbols being in the graph.

The fix registers the directory and descendants, then queues supported existing
files through the ordinary change batch. Startup does not enqueue the project.
Hidden/build exclusions and ordinary file event processing retain their current
behavior. There is no eager wipe, full-project reindex, or connection-lifetime
change.

The new `watch_new_directory` regression moves a populated directory into a
watched root, checks actual graph symbols, edits a direct child, removes a
nested file, and deletes the directory. It also checks that hidden/build files
and unsupported extensions are excluded. It waits for a real event before
changing the fixture and stops/joins its watcher on exit. Production code is
tested with a small Python registry using the existing bundled grammar.

## Duplicate-watch classification

The existing isolated `test_code_watcher_branch_switch_new_dirs` passed both
with and without `INFIGRAPH_NO_WATCH` in this follow-up. A fixture-only preload
that delays `inotify_init` by nine seconds reproduced the original duplicate-lock
assertion: 0 passed / 1 failed, exit 101, 13.532 seconds. Two recorded injection
hits confirm that the delayed backend was exercised.

The test's cleanup polls for lock release for five seconds and silently proceeds
when that expires. The watcher holds the lock during startup, before checking
the stop channel. A slow startup can therefore outlive cleanup and the next
start's half-second lock retry. The lock correctly refuses another watcher.
Classification: a timing-sensitive test failure exposed by slow startup, rather
than a demonstrated lock-bypass regression. This mechanism reproduces the error;
it does not prove the precise historical scheduling of the original failure.

The earlier 4-pass/13-fail suite remains historical evidence: twelve failures
followed a poisoned test mutex. They are not twelve separately reproduced bugs.
No existing assertion, timeout, mutex behavior, or test was weakened or disabled.
The ordinary search-based test is also insufficient proof of graph freshness.

## Receipts

Code/test checkpoint:
[`292bc088931209a74e2fe048fb3bd4d9278e771c`](https://github.com/redxzeta/afterimage/commit/292bc088931209a74e2fe048fb3bd4d9278e771c).
All final runtime commands use binaries from that production source. The report
commit changes documentation only. Times and sampled aggregate RSS are single
observations, not service-level guarantees.

| Check | Result | Receipt |
| --- | --- | --- |
| Integrated offline build | PASS, exit 0 | 206.761 s; final code rebuild 6.922 s |
| Focused executable build | PASS, exit 0 | 20.296 s; no new dependencies |
| Populated-directory regression against baseline production code | FAIL, exit 101 | 16.137 s; no file event after moving the populated directory |
| Same regression with fix | PASS, 1/1 | 8.210 s and serial replay 8.212 s; 578 MiB replay peak |
| Supervisor process harness | PASS, exit 0 | 8 synthetic cases, 2 unit tests, actual native invalid-memory-access injection; 12.213 s / 165 MiB |
| Native worker restart boundedness | PASS | Two worker starts, one launch-project fake-CLI reindex, terminal exit 1; launch/unrelated project/group and registry sentinels preserved |
| Native writer/reader test | PASS, 1/1 | 49 attempts: 48 correct, 1 explicit native error, 0 silently wrong; 13.225 s / 565 MiB |
| Group watcher concurrency | PASS, 1/1 | Completed in 48.594 s within 60 s; 760 MiB. The previous 150 s stall was not reproduced |
| CLI doctor integration | PASS, 4/4 | 4.687 s / 551 MiB; real indexed graph, WAL, invalid graph, and remote-diagnostic refusal cases |
| MCP binary unit tests | PASS, 7/7 | 0.115 s, including direct recovery timeout/reap test |
| Original duplicate-watch test, ordinary controls | PASS, 1/1 each | 16.942 s with no-watch env, 15.776 s without it; final serial control 17.736 s |
| Original duplicate-watch test, delayed inotify startup | FAIL reproduced, exit 101 | 13.532 s; injected startup delay exceeds test cleanup budget |
| Diagnostic native mapping failure/timeout | PASS, exit 0 | Control readable; injected ENOMEM and stall yield UNKNOWN, bytes preserved; serial group 13.833 s / 396 MiB |
| Real Git checkout, working edits/deletion, EOF, diagnostics, sentinels | PARTIAL, exit 1 | 28 case checks passed / 1 failed; 60.293 s / 685 MiB; failure retained below |
| Hidden-ancestor watcher edit | FAIL reproduced, exit 1 | Armed watcher retained old symbol after edit; 13.088 s / 464 MiB |
| Focused strict Clippy | PASS, exit 0 | 65.761 s; CLI/MCP bins and new regression, `-D warnings` |
| Formatting and whitespace | PASS | `cargo fmt --all -- --check` and `git diff --check`, exit 0 |

The native lifecycle fixture passed forward checkout into a new nested directory,
working-tree edit and file deletion, active-watcher stdin EOF (exit 0 and released
lock), human/JSON doctor and MCP diagnose agreement, clean and both WAL-family
read-only manifests, protected unrelated repository sentinels, and launch session
preservation. The return checkout failed its 12-second polling budget: the final
sample at 14.178 s had no symbols. A later sample first confirmed the correct
restored symbol at 25.510 s. Logs show multiple directory/file removal events
being handled before the queued add-file batch. The failure is retained and the
completion harness still exits 1 despite collecting the independent checks;
there is no all-green lifecycle claim or raised passing deadline. The initial
strict fixture also exited 1 at 35.251 s when it stopped at that failed check.
This is measured latency, not proof of a permanent stale graph or attribution of
all the delay to this patch.

The writer/reader's explicit error was a native primary-key-index/local-node-offset
error. The test accepts explicit failures and rejects successful incomplete data;
this is not evidence that native concurrent opens are uniformly error-free.

Initial setup attempts also remain recorded: the core-test placement could not
build offline because existing `cargo-husky` was uncached (exit 101, no download);
a different-feature-set executable build reached its 90-second deadline (exit
137). The regression was placed beside existing MCP watcher tests and the matched
CLI/MCP feature selection built successfully. During fixture development, a
missing extension dot and an immediate read during checkpoint caused failures;
the final test uses the correct extension and retries only the identified
checkpoint conflict for at most three seconds. Existing tests were unchanged.

## Reproduction and isolation

Use disposable HOME, registry, and project directories and existing cached
dependencies. Do not use registered user projects or an installed CLI. No
benchmark target application or repository hook is needed. Builds and native
runs were separate. Native runtime tests ran serially; the initial exploratory
branch-switch failure overlapped compilation and was followed by an isolated
deterministic regression. The exploratory allocation probe was also repeated
serially for the final receipt.

The local controller disabled core dumps, limited CPU affinity to two CPUs and
file size to 256 MiB, imposed finite deadlines, and sampled aggregate owned-child
RSS every 10 ms with a 2 GiB stop. It used a Linux child subreaper to catch
detached fixture descendants, killed remaining owned children, and waited for
them. The RSS stop is sampled, not a kernel hard memory limit. The 32 TiB virtual
address allowance accommodates the native engine's large sparse reservation;
it is not a resident-memory allowance. Builds used two Cargo jobs, an 8 GiB
per-process address limit, 10 GiB sampled aggregate RSS, and finite deadlines.
An existing disposable toolchain and copied caches/build outputs were reused;
no toolchain, host package, or application was installed.

With the same environment, the exact build commands were:

```sh
cargo build --offline --locked -p infigraph-cli -p infigraph-mcp \
  --bin infigraph --bin infigraph-mcp
cargo test --offline --locked -p infigraph-mcp \
  --test watch_new_directory --test watcher_reindex --test watcher_concurrency \
  --test concurrent_writer_reader --bin infigraph-mcp \
  -p infigraph-cli --test doctor --no-run
```

Select the executables printed by `--no-run`; their hashes depend on the build.
For each runtime command below, use the stated finite deadline and an owned-child
memory/cleanup controller. A shell `timeout` alone cannot catch detached children
that start their own session. Set `INFIGRAPH_NO_WATCH=1` for CLI subprocesses;
explicit MCP watchers remain exercised. Use a visible disposable `TMPDIR` for
the new-directory test because the absolute hidden-ancestor bug remains open.

```sh
python3 scripts/test-mcp-supervisor.py --native-mcp "$BIN/infigraph-mcp"
"$NEW_DIR_TEST" --test-threads=1 --nocapture
"$REINDEX_TEST" --exact test_code_watcher_branch_switch_new_dirs \
  --test-threads=1 --nocapture
"$CONCURRENT_TEST" --exact concurrent_writer_reader_raw_query_correctness_under_load \
  --test-threads=1 --nocapture
"$WATCH_CONCURRENCY_TEST" --exact test_graph_tools_with_group_watchers \
  --test-threads=1 --nocapture
"$DOCTOR_TEST" --test-threads=1 --nocapture
"$MCP_UNIT_TEST" --test-threads=1
cargo clippy --offline --locked -p infigraph-cli -p infigraph-mcp \
  --bins --test watch_new_directory -- -D warnings
cargo fmt --all -- --check
git diff --check
```

The retained local native fixture scripts additionally construct two synthetic
Git commits (A has a Lua function; B deletes it and adds a function in a nested
directory), switch both ways, edit/delete the working file, and query
`MATCH (s:Symbol) RETURN s.name` through an independent CLI. MCP uses stdio only.
EOF must exit and release the watch lock. Doctor human/JSON and MCP diagnose
must agree and preserve the project manifest (inode, mode, size, nanosecond mtime,
SHA-256), registry bytes, and Git status bytes. Ordinary MCP startup logs/instance
locks are distinct from read-only diagnostic work.

Stale-WAL fixtures contain synthetic sentinel bytes in `graph.wal` and
`graph.wal.orphan`; the expected result is `GRAPH_STATUS_UNKNOWN` without opening
or changing them. The allocation/timeout fixture interposes the diagnostic
child's native mapping of at least 1 GiB: control succeeds, allocation denial
returns `MAP_FAILED`/`ENOMEM`, and a 30-second stall must be killed by the existing
10-second diagnostic deadline. Each injection must record a hit. These are
allocation-failure and timeout containment checks, not an organic host-OOM
reproduction or a supervisor-wide resource guarantee.

The additional exact local commands were `python3 native_acceptance.py`,
`python3 probe_boundary.py`, and `python3 hidden_case.py`, each through the
controller with the budgets recorded above (completion budget 130 s, mapping
probe 45 s, hidden-path fixture 35 s). The native lifecycle completion collects
independent checks after a latency failure but returns exit 1 if any case failed.

Raw commands, exit codes, elapsed time, sampled RSS, stop reasons, fixture case
JSON, and those local scripts are retained in this lane's evidence directory. Public
artifacts contain only synthetic data and summarized receipts, with no machine
paths, binaries, index databases, or credentials.

## Remaining acceptance limits and adjacent work

- Return checkout did not meet the fixture's 12-second observation budget;
  eventual correctness at 25.510 s is insufficient for prompt-freshness acceptance.
  No registry caching, removal batching, or unrelated latency redesign is added.
- Absolute hidden ancestors still suppress code-watcher events, including common
  hidden workspace paths. The replacement ignore/watch work in
  [upstream #54](https://github.com/intuit/infigraph/pull/54) overlaps this area;
  this PR does not replace the matcher.
- A synthetic stale-WAL refusal by doctor does not validate ordinary graph
  opens against a real crash-generated WAL. Both normal open paths still reach
  native replay without the guard proposed in
  [#64](https://github.com/intuit/infigraph/pull/64).
- The concurrent-reader test exercises one process with native writer/reader
  threads. It does not prove multi-process write atomicity or all transactions.
  Connection retention/checkpoint changes in
  [#43](https://github.com/intuit/infigraph/pull/43) interact with idle locks in
  [#46](https://github.com/intuit/infigraph/issues/46); neither is imported here.
- [#65](https://github.com/intuit/infigraph/issues/65) reports organic OOM and
  worker accumulation. The logging fix in
  [#66](https://github.com/intuit/infigraph/pull/66) addresses panic recursion,
  not those separate behaviors. No organic OOM or recursive-panic claim is made.
- Supervisor recovery still covers only the launch project, one restart, and
  its direct recovery child's 60-second deadline. Arbitrary SCIP descendants,
  termination of the supervisor while the client remains connected, non-launch
  corruption, and normal native tool-call deadlines remain outside this proof.
- Windows/macOS, remote backends, kompress inference, long-duration disk growth,
  and full-workspace acceptance remain unverified. Hosted fork CI is not used
  as evidence for local runtime acceptance.

No upstream branch, mirror branch, production checkout, service, container,
permission, deployment, release, tag, schedule, or benchmark script is changed.
There is no new operational responsibility or installation step.

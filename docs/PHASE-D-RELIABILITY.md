# Phase D: bounded reliability and lifecycle investigation

Tested on Sable, 2026-10-23. Base: `cf82f5d7440d9be158e219df1ef256d6359ed1f6`
(`origin/main`, freshly fetched). Rust: `1.99.1 (58cf8f9c3 2026-10-13)`.
This is a Linux investigation, not a default-feature, full-workspace, or
cross-platform acceptance result. The fix is intended for the Afterimage fork.

## Contained severe finding

**Observed:** the baseline MCP supervisor interprets SIGSEGV as corruption of
all registered projects and combined groups. It eagerly deletes their code and
document artifacts before invoking the CLI. A failed CLI does not undo deletion.
There is no worker restart limit, no reindex timeout, and CLI progress inherits
MCP stdout. A worker fault is not evidence that every registered index is corrupt.

The native reproduction injects an actual invalid memory access into the worker's
first stdin read using a fixture-only `LD_PRELOAD` library. It does not corrupt
or open a real project index. A sibling fake CLI deliberately exits unsuccessfully.
Four faults followed by an ordinary EOF demonstrate the following:

| Observation | Baseline | Containment fix |
| --- | --- | --- |
| Worker starts | 5 | 2 |
| Reindex invocations | 12, across two projects and one group | 1, launch project only |
| Unrelated project graph/docs | Deleted | Preserved |
| Unrelated group graph/docs | Deleted | Preserved |
| Launch index after fake CLI failure | Deleted | Preserved |
| Session sentinels | Preserved | Preserved |
| Repeated-fault terminal status | 0 after fifth worker receives EOF | 1 after second fault |

This establishes the supervisor failure and containment, not the cause of organic
native crashes. Sending SIGSEGV with `kill` did not terminate these Rust workers;
that preliminary probe did not reproduce the fault and is not counted as a pass.
The subsequent native invalid-access injection did reproduce it.

The fix moves supervision into a standard-library-only module so it can be tested
without linking the native graph engine. It makes one restart/recovery attempt,
selects only an existing non-symlinked launch-directory index, skips non-local
backends, and invokes ordinary `infigraph index` without an eager wipe. CLI stdin
and stdout are disconnected from MCP transport. The existing `INFIGRAPH_NO_WATCH`
mechanism prevents an independent recovery watcher. The direct reindex child has
a 60-second timeout and is killed and waited on if it exceeds that budget.

The normal per-project index recovery remains in place. Native tests show that
ordinary indexing rebuilds both a truncated and an invalid-header graph while
preserving session sentinels. An additional end-to-end test faults once, runs the
real CLI against a truncated launch graph, restarts the real worker, returns the
pending JSON-RPC request, and verifies the recovered symbol, a rebuilt corrupt document store, and preserved
session sentinels. No global registry or group scan remains in supervision.

Recovery cannot identify which tool target caused a native fault. A launch-root
reindex is a deliberately limited attempt, not crash attribution. A corrupt
non-launch project can still require an explicit reindex. The ordinary CLI's
existing error classification and destructive recovery are unchanged; notably,
a resource-related native open failure can still be misclassified as corruption
of the launch graph. The timeout covers the direct CLI child, not arbitrary
SCIP/runtime descendants. Windows exception detection is retained but untested.

## Capacity and isolation

Initial readback: 64,181 MiB RAM, 61,065 MiB available, 16 logical CPUs, about
1.3 TiB free disk, load averages 0.26/0.41/0.24. Existing agent jobs and services
were observed through process names/resource counts only and were not controlled.
The existing T3-bound fresh isolated worktree matched freshly fetched origin/main.
No other project checkout or running application was used as a corpus.

The host lacked Rust and a Cargo cache. A minimal toolchain, caches, build outputs,
HOME, registry, and synthetic repositories were created in disposable locations.
Builds used two Cargo jobs, two CPU affinity slots, nice level 10, no core dumps,
8 GiB per-process address space, a sampled 10 GiB process-group RSS stop, a 4 GiB
per-file limit, and finite timeouts. No host packages were installed. Missing
OpenSSL development linker names were supplied through disposable symlinks to
the installed runtime libraries and a command-local `LIBRARY_PATH`.

Default graph opens reserve 8 TiB of anonymous virtual address space using
`MAP_NORESERVE`; that reservation is not evidence of 8 TiB resident memory.
The original 8 GiB address-space cap therefore blocks even tiny valid graphs.
Small graph/watcher runs instead used a 32 TiB address-space cap, a sampled
2 GiB process-group RSS stop at 10 ms intervals, two CPUs, a 256 MiB per-file
limit, no cores, and 45–180 second deadlines. The RSS watchdog is sampled rather
than a kernel cgroup hard limit; cgroup control files were unavailable here.
Corrupt embedding allocation was tested separately under the strict 8 GiB cap.

One initial CLI `--help` invocation preceded HOME isolation. Help exits before
graph startup, but the CLI starts its update-cache lookup before argument parsing;
an outside-fixture cache access cannot be ruled out for that invocation. All
subsequent application/index/destructive probes used disposable HOME/registry
and corpus paths. No operational service, container, deployment, schedule,
credential, permission, or running trading application was changed.

## Test matrix and measurements

RSS figures below are sampled aggregate process-group peaks, rounded to MiB.
Timings are individual observations, not performance guarantees.

| Case | Result | Evidence / limitation |
| --- | --- | --- |
| Native CLI/MCP build, compression excluded explicitly | PASS | Initial cold build reached linking after 210.221 s, then failed for missing OpenSSL linker names. Baseline relink passed in 7.488 s / 4,313 MiB. Final fixed build passed in 6.513 s / 7,222 MiB. |
| Supervisor regressions | PASS | 8 synthetic-worker cases, 2 Rust unit tests, native baseline reproduction and native containment; final replay 17.893 s / 175 MiB. |
| Baseline initialization, discovery, stdin EOF | PASS | Clean JSON-RPC initialization/discovery; no-input EOF also exited 0, about 0.01–0.02 s. Empty isolated registry. |
| Small Lua corpus: index/query/edit/restore/delete | PASS | 9 process invocations; 23.467 s / 382 MiB total. Query observed the changed function; deletion left no symbols. Restore was a file checkout, not a real branch switch. |
| Concurrent native writer/readers | PASS | Existing `concurrent_writer_reader_raw_query_correctness_under_load`: 1 passed, 14.184 s / 545 MiB. This is the existing in-process writer/read-only-open test, not proof of multi-process write atomicity. |
| Truncated and invalid-header graph reads | PASS for these fixtures | Direct worker returned structured tool errors; graph bytes stayed unchanged. No claim about all malformed native files or stale WALs. |
| Ordinary native corrupt-graph recovery | PASS | Both fixtures rebuilt successfully, sessions preserved; read+repair group 17.045 s / 388 MiB. |
| Fault → real CLI recovery → real worker | PASS | One injected fault; two worker starts; one clean pending JSON-RPC response. Native code symbol and corrupt doc store rebuilt, session sentinel preserved. 7.107 s for recovery, 10.401 s including verification / 531 MiB. |
| Watcher idle read, edit/delete, active-watcher EOF | PASS in visible temporary path | Small Lua control: 10 assertions, 21.228 s / 582 MiB. Idle CLI read worked; edits/deletion became visible; EOF exited and released watch lock. |
| Same watcher under a hidden ancestor | FAIL | Watch lock held and idle reads worked, but edited symbol stayed stale. Static source explains this: `should_ignore` rejects any dot-prefixed component of the absolute event path, including `.t3`. Open upstream #54 replaces this path. |
| Existing watcher-reindex executable | FAIL | 4 passed / 13 failed, 77.045 s / 772 MiB. First failure: duplicate watcher lock in `test_code_watcher_branch_switch_new_dirs`; 12 later tests failed on the poisoned test mutex. |
| Isolated branch/new-directory test | FAIL | Same duplicate-watch error reproduced alone in 20.654 s / 562 MiB. No 13-independent-bug claim. Other existing branch simulation passed in the initial suite. |
| Existing watcher-concurrency executable | TIMED OUT | First active-watcher test passed; group-watcher case had not completed by 150 s. Only this disposable process group was killed; 795 MiB peak. Remaining cases unverified. |
| Corrupted embedding header | EXPECTED FAILURE reproduced | Unchanged extracted loader given `corrupt` tried to reserve 92,164,907,664 bytes, aborted with SIGABRT under address-space cap, 1.178 s / 2 MiB. Valid empty header succeeded. #82 owns the fix. |
| PR #81 doctor / MCP diagnose | PASS at separate PR head | Built `435723028002a9d294a01e113701136b4d4a954a` from a disposable source archive, without compression. CLI human/JSON exits 1 correctly for empty graph; MCP exits 0 with matching report. Reports say DEGRADED / freshness UNKNOWN. Index inode/mode/size/mtime/hash manifest and Git status bytes preserved. About 0.07 s each. Not merged into this fix or main. |
| Formatting / whitespace | PASS | `cargo fmt --all -- --check`, `git diff --check`. |
| Strict Clippy | FAIL, existing core lint | `chunks_exact_to_as_chunks` in unchanged `embed/mod.rs`; 143.654 s / 891 MiB. #83 owns it. No lint suppression added. |
| Default-feature / opt-in kompress inference | SKIPPED | No ORT/model download or inference was performed. Main still enables kompress by default; #78 makes it opt-in. Non-kompress native startup on this Ryzen/AVX2 host is not older-CPU compatibility proof. Synthetic SIGILL exits without reindexing. |
| Full workspace, remote backends, Windows/macOS, real WAL crash | NOT RUN | Bounded targeted investigation only. No green full-suite or remote/native-WAL safety claim. |

## Upstream comparison and deliberate non-changes

Current upstream state was read through GitHub APIs. Findings were compared with:

- [#46](https://github.com/intuit/infigraph/issues/46): idle graph-lock contention
  and orphan watchers. Current source opens a transient graph per change batch;
  the visible-path idle read and EOF control passed. This does not resolve the
  reported history or all lifecycle modes. `--ui` intentionally remains alive
  after stdin EOF; killing a supervisor while its worker is active was not tested.
- [#64](https://github.com/intuit/infigraph/pull/64): unsafe native WAL replay from
  a dead writer. Current read-only open has no equivalent guard. Not reproduced
  with a real crash-generated WAL; no competing fix added.
- [#65](https://github.com/intuit/infigraph/issues/65) and
  [#66](https://github.com/intuit/infigraph/pull/66): recursive panic logging,
  OOM, and worker accumulation. The panic hook remains unchanged; reported
  110 GiB RSS was not reproduced. Large virtual reservation alone is not that OOM.
- [#67](https://github.com/intuit/infigraph/issues/67),
  [#68](https://github.com/intuit/infigraph/pull/68), and
  [#69](https://github.com/intuit/infigraph/pull/69): shared COPY connection state
  and transaction no-ops. Existing advisory graph-write locking serializes
  participating writes but does not prove these logical transactions atomic.
  Database open/schema setup precedes some advisory locking; read-only opens
  are not participants in that write-lock protocol. No transaction redesign.
- [#53](https://github.com/intuit/infigraph/pull/53) and
  [#54](https://github.com/intuit/infigraph/pull/54): freshness and ignore/watch
  behavior. The hidden-ancestor observation overlaps #54's replacement matcher.
- [#78](https://github.com/intuit/infigraph/pull/78): CPU/kompress and MCP HTTP
  loopback defaults. Current UI defaults to `127.0.0.1`, MCP HTTP to `0.0.0.0`.
  HTTP listeners were not started in this investigation. No duplicate binding fix.
- [#81](https://github.com/intuit/infigraph/pull/81),
  [#82](https://github.com/intuit/infigraph/pull/82),
  [#83](https://github.com/intuit/infigraph/pull/83), and
  [#84](https://github.com/intuit/infigraph/pull/84): separate diagnostics,
  allocation, lint, and TypeScript caller work. No cherry-picks or duplicate fixes.

## Reproduction

All application commands must use a disposable HOME/registry and corpus. The
committed process harness creates and cleans its own fixtures. It requires
Python 3, rustc, and, for native Linux injection, `cc`:

```sh
python3 scripts/test-mcp-supervisor.py
python3 scripts/test-mcp-supervisor.py \
  --native-mcp target/debug/infigraph-mcp \
  --baseline-mcp /path/to/baseline/infigraph-mcp
```

The optional baseline argument explicitly reproduces unsafe behavior **only on
its own disposable sentinels**, using a sibling fake CLI. It never invokes an
installed CLI. The default harness does not link the native engine.

For this run, retained local commands/evidence are under `.phase-d/` (untracked):

```sh
# Resource controller records command, status, wall time, sampled RSS and stop reason.
python3 .phase-d/bounded.py 90 .phase-d/evidence/final-supervisor-regressions \
  python3 scripts/test-mcp-supervisor.py \
  --native-mcp .phase-d/native-target/debug/infigraph-mcp \
  --baseline-mcp .phase-d/baseline-bin/infigraph-mcp

# Exact native build, with isolated env variables as recorded in the run scripts:
cargo build --locked --no-default-features -p infigraph-cli -p infigraph-mcp \
  --bin infigraph --bin infigraph-mcp
cargo test --locked --no-default-features -p infigraph-mcp \
  --test watcher_reindex --test watcher_concurrency --test concurrent_writer_reader \
  --bin infigraph-mcp --no-run
cargo fmt --all -- --check
cargo clippy --locked --no-default-features -p infigraph-mcp \
  --bin infigraph-mcp -- -D warnings
```

The local scripts `native_lua.py`, `native_corrupt_recovery.py`,
`visible_watcher_matrix.py`, `real_recovery_flow.py`, and `doctor_acceptance.py`
preserve the remaining fixture construction and invocations. Their bounded run
JSON/logs and detailed case JSON are in `.phase-d/evidence/`. Native test executable
filenames are recorded there; they are build-specific. Preserve these logs when
cleaning disposable build/cache files. No private source, index databases, machine
paths, binaries, or credentials are included in the PR.

Confidence: high for the reproduced supervisor loss and tested Linux containment;
medium for broader lifecycle characterization. Recommendation: review the narrow
fork PR; hold broader reliability acceptance pending watcher/freshness, native
WAL/transaction, compression, strict lint, and full-suite work. No deployment,
installation, merge, release, or new ongoing operational responsibility is added.

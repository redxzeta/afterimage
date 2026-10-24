# Afterimage integration acceptance

Snapshot: 2026-10-24 UTC. Repository: `redxzeta/afterimage`; development/default
base: `afterimage-main`. Reviewed source base:
`f4568cf15e004391f549bc8705112e37502bd3dc`. This receipt grants no installation,
deployment, release, publishing, runtime, scheduling, or financial authority.

Protected mirror `main` remains
`cf82f5d7440d9be158e219df1ef256d6359ed1f6`. Upstream intuit/infigraph PRs
#78/#81/#82/#83/#84 are separate from fork integration; no upstream mutation
was performed. The provisional advisory trial #10 remains outside this gate.

## PR outcomes

| Fork PR | Exact reviewed head | Outcome / merge commit |
| --- | --- | --- |
| [#6: SCIP receipts](https://github.com/redxzeta/afterimage/pull/6) | `f85d9efa55447de4fa46af3bdd97b5fd6b3a7f01` | MERGED: `4d5266fe10c2c39dbaac423c7a43731329601e85` |
| [#7: watcher freshness](https://github.com/redxzeta/afterimage/pull/7) | `51de48ce7ccf9218ff91424f8c312a5c1a10ac38` | MERGED: `585fd5d9221ced25a8bc2307c0b0e5fd9ff49010` |
| [#8: acceptance checkpoint](https://github.com/redxzeta/afterimage/pull/8) | `83a700968e63d9737eec8461fab8b69ee876d2c6` | MERGED: `de02e762c5581055d2adcefe7efe1a58bc0c7cab` |
| [#9: relative module imports](https://github.com/redxzeta/afterimage/pull/9) | `78c7c64494661ae906565156b957d648e1820f79` | MERGED: `a5b19badfd4ae496965a803c7be770d2281bc573` |
| [#11: benchmark harness/evidence](https://github.com/redxzeta/afterimage/pull/11) | `eee27b712d19c745cd2950dc95a7470bab14c3f4` | MERGED: `f4568cf15e004391f549bc8705112e37502bd3dc` |
| [#5: focused CI](https://github.com/redxzeta/afterimage/pull/5) | `f9ffc6f199212c7c2e05d788ef0a8b2c8dfe6603` | HOLD: hosted SIGSEGV cause and new import-suite selection remain open; replacement must include current base and receive fresh review. |

Every merge above used an expected-head guard after independent source/test
review. #9 included #6/#7 before final combined validation; #11 then integrated
#9. Merge construction and diff whitespace checks passed. No known new failing
product test was waived. Missing checks under the inherited main-only CI filter
are NOT_CONFIGURED, not passes; #5 owns the pending target correction.

## Validation receipts

Author native executions below were independently reviewed, with tested-code
versus delivery-head equality checked. They were not relabeled as reviewer
executions. Detailed commands, retained failures and limits are in the linked
reports. No full workspace or cross-platform matrix is claimed.

- **#6:** code `e8bb61bf297d28349a055dae4cc7930f8ccac14b`; final commit adds
  documentation only. **53 focused tests pass:** 6 callers, 2 receipt, 35 CLI
  index, 4 doctor, 1 diagnose and 5 coverage-reporting tests. Formatting and
  focused strict Clippy pass. Regression proves a true no-op retains receipt
  and compiler edge, while unchanged batch rewrites invalidate lost enrichment.
  [Full receipt and upstream findings](UPSTREAM_PR_READINESS.md).
- **#7:** tested checkpoint `b65ad6cd086f76124bc6884308c6751f50bf2b3a`;
  final head changes docs only. Raw receipt source SHA-256 values match the
  reviewed files. Four direct-graph regressions fail before and pass after:
  populated directory/symlink/loop, return checkout/edit/removal, hidden ancestors
  with ignore exclusions/negation, and rule changes before queued-file flush.
  Return graph is exact in **2.183042707 s**, within the unchanged absolute
  **12 s** starting before checkout and checked after reads. Outside sentinel
  preserved; #6 receipt **1/1**; reader/writer **52 correct / 0 errors / 0 wrong**
  in that run. Strict Clippy/fmt pass. [Full report](PHASE-D-RELIABILITY-FOLLOWUP.md).
- **#9:** tested checkpoint `bba675c9f6fcfa60c2f2a9ae6ea120ebbed433aa` on
  base `de02e762`; final commit adds docs only. **8 import + 1 receipt + 4 watcher
  tests pass**, with focused strict Clippy/fmt. Watcher return is **2.367690088 s**
  inside the unchanged deadline; five unsuccessful native reads precede valid
  snapshots. Original missing-import and provisional-edge ordering regressions
  retain **exit 101** before their fixes. Earlier `df7a8687` has separately pinned
  8 import + 29 library + 6 caller passes. [Full receipt](TYPESCRIPT_MODULE_IMPORTS.md).
- **#11:** reviewer independently ran **19/19 synthetic harness tests**, exit 0,
  **3.805 s**, plus Python compilation and JSON parsing at
  `7e56fa1342feff9399623487309a0688e29e09ad`. Final delivery only integrates
  #9 and updates report wording; harness/tests/fixtures/summary bytes match.
  Runner/truth hashes, binary pins, published aggregate counts, six freshness
  records per lane, and all **30 CLI/MCP question scores** were checked.
  Public artifact inspection found no private host paths or credential material;
  fixed `/bench/home` paths are sandbox namespace constants.
- **#5:** reviewer independently passed **14 selector + 4 diagnostic tests** at
  `ffdd27af366140f40116a7bf525aa0063386a0c9`, then **14 + 5** at `f9ffc6f1`.
  Both selector gaps are corrected: unknown/workspace inputs retain watcher and
  graph-lock suites, and watcher changes select integrated `watch_new_directory`.
  Author native receipt at `0eb9e229fcebc0a2d054fcfcaa9706481bae8f81` has
  **4 CLI parity + 4 watcher tests**, 113.35 s including build; the `ffdd27af`
  documentation merge leaves relevant source/build/CI files identical.

Native work respected the benchmark's explicit release at **15:31:58 UTC** and
subsequent Sable slot handoff. No measured target apps, hooks or services ran.
The earlier Debian provider usage-limit interruption is historical: authors
resumed and completed #9/#11 delivery. It does not waive #5's unresolved failure.

## Remaining CI diagnosis

[Run 40127988676](https://github.com/redxzeta/afterimage/actions/runs/40127988676)
on `ffdd27af` fails CLI parity **3 passed / 1 failed**: fixture index dies from
**SIGSEGV 11**, core dumped, no exit code, empty stdout/stderr. Earlier runs
[40106285931](https://github.com/redxzeta/afterimage/actions/runs/40106285931) and
[40107320071](https://github.com/redxzeta/afterimage/actions/runs/40107320071)
retain the same failure. Unreached hosted suites remain NOT_RUN.

Reported hosted native archive and both header hashes match the passing local
cache; the native version is 0.21.2. Different downloaded bytes are unsupported
as the explanation. The probe selected `lldb-dap-21` instead of a debugger CLI,
so its exit zero supplied **no backtrace**. The reviewer reproduced that
selection defect; `f9ffc6f1` rejects helpers and its new discovery test passes.
Corrected hosted diagnosis remains pending. Underlying cause and inherited
versus environment regression remain **UNDETERMINED**. Production-source equality
alone does not prove inheritance; no OOM, SIGILL, timeout-only or green claim.

A combined-base probe also finds that `f9ffc6f1` omits merged #9's
`typescript_imports` target for core graph, language-query and CI changes when
the test file itself is unchanged. The owner received the reproducer and a
request to retain that critical regression with present/absent target tests.
This coverage gap must be resolved before final #5 acceptance.

## Benchmark and operational limits

[Report](../scripts/benchmark/results/2026-10-23-bce.md) and
[summary](../scripts/benchmark/results/2026-10-23-bce-summary.json) measure baseline
`cf82f5d7440d9be158e219df1ef256d6359ed1f6`, candidate
`1a59c728ae3284f0af3b5c781e14a9e2736df6e0`, runner
`286faf925590156f30b82372f4c0c94f103f10d4`; both builds use no default features.
Those measurements predate #6/#7/#9 and do not measure this integration base.

- Both repository lanes find **68/68 symbols**; positive calls are baseline
  **39/49**, candidate **46/49**; imports are **0/8** in both. Candidate's **621**
  unadjudicated calls are not false positives. The smaller IceCouncil fixture
  reproduces a labelled false edge in both revisions; projection scope matters.
- Candidate repository index commands still exit **1** for unavailable SCIP;
  pairs remain **INCOMPLETE_INDEX_PAIR**. Manual matching graph/fingerprint
  observations do not turn failed index commands into passes. Sampled CPU/RSS
  bounds and descriptive timings establish **no accepted indexing speedup**.
- Fixed MCP retrieval questions are deterministic tool checks, not model/agent
  task A/B. `agent_differential` remains **NOT_RUN**; no token or agent-time
  improvement is established. Historical default-feature baseline exit 132
  remains separate from this no-default-feature experiment.
- Current #9 fixes indexed relative import/re-export cases in synthetic fixtures.
  Bare workspace exports, aliases, ambiguous alternatives and remote parity stay
  unsupported/unverified. Full-repository import scores were not rerun. SCIP
  still writes no Module IMPORTS; duplicate CALLS and unpropagated import-write
  outcomes remain limitations. #6 fixes batch receipt reuse, not exhaustive
  compiler coverage or the direct single-file receipt boundary.
- Scoped watcher hidden-path/return/queued-rule/symlink cases pass. Transient
  native checkpoint/WAL read errors, delayed-start test cleanup, real crash-WAL
  recovery and platform coverage remain open. Ignore changes affect queued files
  and subscriptions, not retroactive graph purge. Successful snapshots within a
  deadline are not error-free-read or universal freshness proof.

**GO:** reviewed #6/#7/#9 source and #11 harness/evidence integration.
**HOLD:** #5 merge, blanket graph freshness/completeness/performance acceptance,
agent A/B acceptance, and operational use claims. No deployment or activation
was authorized or performed. No new service, container, schedule, credential,
database, monitoring duty, or installation step is introduced.

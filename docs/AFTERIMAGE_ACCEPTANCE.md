# Afterimage integration acceptance

Review date: 2026-10-24 UTC. This is a source-integration receipt for the personal
fork, not installation, deployment, release, or runtime authority.

## Bound identities and decisions

- Repository/base: `redxzeta/afterimage:afterimage-main`.
- Starting base: `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`.
- Current integrated base: `585fd5d9221ced25a8bc2307c0b0e5fd9ff49010`.
- Protected fork mirror `main`: `cf82f5d7440d9be158e219df1ef256d6359ed1f6`,
  unchanged after the merge. Upstream PRs #78/#81/#82/#83/#84 remain separate
  from fork acceptance; this review makes no upstream mutations.

| Fork PR/lane | Exact reviewed head | Outcome and remaining condition |
| --- | --- | --- |
| [#6: SCIP receipts](https://github.com/redxzeta/afterimage/pull/6) | `f85d9efa55447de4fa46af3bdd97b5fd6b3a7f01` | MERGED with expected-head guard; merge `4d5266fe10c2c39dbaac423c7a43731329601e85`. Independent source/test review passed; 53 focused test receipts remain applicable. |
| [#5: focused CI](https://github.com/redxzeta/afterimage/pull/5) | `17a5cc66aae0c3363c07780ebf1f3518a8dad1d1` | HOLD: both selector gaps corrected and independently validated (14 passes). Hosted fixture still dies from SIGSEGV; debugger probe did not run. Local author parity passes 4/4; causal disposition remains unresolved. |
| [#7: watcher directories](https://github.com/redxzeta/afterimage/pull/7) | `51de48ce7ccf9218ff91424f8c312a5c1a10ac38` | MERGED with expected-head guard; merge `585fd5d9221ced25a8bc2307c0b0e5fd9ff49010`. Four direct-graph regressions and #6 receipt pass; narrow Linux source integration accepted with explicit residuals below. |
| Three-repository benchmark follow-up | No delivery PR yet | Measurements finished; candidate pinned to `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`, baseline `cf82f5d7440d9be158e219df1ef256d6359ed1f6`, harness `286faf925590156f30b82372f4c0c94f103f10d4`; both binaries use no default features. Final artifacts/review pending. |
| [#9: TypeScript imports](https://github.com/redxzeta/afterimage/pull/9) | `df7a8687e3cc2024172c27bf1a66e1a408c96111` | DRAFT / HOLD: author reproduced both pre-fix failures and reports 8 import plus 29 focused library passes. Head does not yet include merged #7; caller/combined checks, final receipts and readiness remain pending. |

Every remaining delivery must include the current `afterimage-main` base and
receive a fresh diff/conflict review. Passing one PR does not establish combined
acceptance of still-unmerged changes. The issue-48 advisory trial (draft fork
PR #10) is provisional, outside this gate, and has no merge authorization here.
The benchmark explicitly released native builds at 15:31:58 UTC after its final
measurement. Clearance was forwarded to the existing lanes; Sable freshness
checks and the provisional trial coordinate their bounded native work.

PR #8 has no reported checks because the current base CI workflow filters PRs
to `main`, while this documentation-only PR targets `afterimage-main`. This is
NOT_CONFIGURED, not a test pass or failure. Its diff is whitespace-checked and
reviewed; no workflow change or broad native suite is added for this notice.
PR #5 owns the pending CI target correction.

## Review and validation receipts

The integration reviewer inspected all changed files, PR descriptions, comments,
reviews, check summaries, source call paths, and merge construction for #5–#7.
At the initial reviewed heads there were no submitted reviews or inline/issue
comments, and all three PRs were mergeable. Diff whitespace checks passed for
all three. `git merge-tree --write-tree` passed for #6 against the starting base
and #5/#7 against the advanced base.
Merge construction is not runtime validation.

For #6, production code is commit
`e8bb61bf297d28349a055dae4cc7930f8ccac14b`; the final commit adds only the
[readiness report](UPSTREAM_PR_READINESS.md). The reviewer confirmed code,
manifests, lockfile and tests are unchanged between those commits. Existing
serial, offline receipts were inspected, not rerun during active benchmark
timings:

| Command/check on #6 code | Result |
| --- | --- |
| `cargo test --locked --offline -p infigraph-core --test typescript_callers -- --test-threads=1` | PASS: 6, including unchanged rewrite invalidation and genuine no-op preservation. |
| `cargo test --locked --offline -p infigraph-core --lib index_status::tests:: -- --test-threads=1` | PASS: 2. |
| `cargo test --locked --offline -p infigraph-cli --bin infigraph index:: -- --test-threads=1` | PASS: 35. |
| `cargo test --locked --offline -p infigraph-cli --test doctor -- --test-threads=1` | PASS: 4. |
| `cargo test --locked --offline -p infigraph-mcp --test diagnose --test coverage_reporting -- --test-threads=1` | PASS: 1 + 5. |
| Formatting, core library Clippy with `-D warnings`, diff check | PASS in author receipt. Reviewer refreshed diff check. |

The new regression fails on the previous production implementation and passes
with the fix. It verifies a stored compiler edge disappears after an unchanged
batch rewrite, the fingerprint remains equal, and successful enrichment is then
invalidated. The genuine incremental no-op retains both edge and receipt. The
fix is a receipt reuse condition; it does not restore edges or run SCIP.

For #5, the reviewer independently ran
`python3 -m unittest discover -s scripts -p test_verify.py -v` in an archived
snapshot of initial head `019979b5f8bf7cb6b08d290b4786e234faf32050`:
**PASS, 12 tests, exit 0**. The diagnostic head above independently passed
**13 selector tests, exit 0**. Current head `17a5cc66` independently passed
**14 selector tests, exit 0**. A separate initial-head selector probe
confirmed that future watcher source changes omit #7's `watch_new_directory`
target once that test has landed; targeted feedback was sent to its owner.
The diagnostic head now selects this direct-graph regression when its target
exists, with present/absent test coverage. Another
probe confirmed CI/selector/unknown-input changes selected `workspace` but omitted
watcher and graph-lock suites. At `17a5cc66`, conservative workspace expansion
includes those suites, with its regression passing. Graph/SCIP/process/benchmark
coverage remains selected; source review, whitespace and merge construction pass.

[Hosted run 40068158249](https://github.com/redxzeta/afterimage/actions/runs/40068158249)
on #5's initial head passed formatting, all-target Clippy and CLI/MCP build,
then failed CLI parity: 2 passed / 1 failed. Fixture indexing failed with empty
captured stderr; child status/signal/stdout were not recorded. Later suites did
not run. Cause remains UNKNOWN; local passes do not establish hosted parity.
The author's earlier 90 integration/683 unit/14 benchmark-harness passes remain
reported evidence on the documented source, not fresh reviewer executions.
[Diagnostic run 40106285931](https://github.com/redxzeta/afterimage/actions/runs/40106285931)
on `d04471dd` failed: **3 passed / 1 failed**. Fixture indexing died from
**SIGSEGV (signal 11), no exit code, core dumped, empty stdout/stderr**. The
diagnostic regression passed and assertions remain strict. This establishes a
native crash, not its inherited-versus-environmental cause; that disposition
remains unresolved. Later suites did not run.

[Run 40107320071](https://github.com/redxzeta/afterimage/actions/runs/40107320071)
at `17a5cc66` again failed CLI parity: **3 passed / 1 failed**, signal 11,
core dumped, no exit code, empty stdout/stderr. The 14 selector tests, formatting,
all-target Clippy and CLI/MCP build passed. The selected integration command now
includes watcher and graph-lock suites, but execution stopped at CLI parity;
unreached suites remain NOT_RUN. The failure-only debugger step reported
`NOT_RUN: built CLI or gdb unavailable`; it supplied no backtrace and did not
mask the primary failure. The author separately reports **4/4 local CLI parity
passes** on this exact head (71.05 seconds including compilation, own cache,
one build job, serial tests after the benchmark release). That is not a fresh
reviewer native execution. Mutable `latest` prebuilt artifacts in `lbug 0.19.1`
are an investigation hypothesis, not an established crash cause. The existing
CI owner was resumed for bounded causal diagnosis; no duplicate fix was made.

For #7, [the original report](https://github.com/redxzeta/afterimage/blob/1c1fe4ffdf0838f11c61c6a7909a2ca606f23929/docs/PHASE-D-RELIABILITY-FOLLOWUP.md)
historically records the direct-graph directory regression passing, but lifecycle completion
returns exit 1 with **28 checks passed / 1 failed**. Return checkout exceeded
12 seconds and was only later correct at 25.510 seconds. Hidden-ancestor edits
also reproduced stale graph data. Neither a higher timeout nor eventual
correctness alone satisfies the follow-up. New direct-graph checks and explicit
residual-risk disposition are required before a merge decision.
An additional P2 source finding was sent to the watcher owner: directory
registration follows symlink directories, and the new existing-file callback
can queue outside-root source through those links, unlike the initial scanner.
The final patch skips symlinks during registration and directory events;
outside-root sentinel and loop regressions now pass.

At final head `51de48ce`, the independent review covered every changed source,
test and report file and verified their SHA-256 against the local raw receipt
packet. Code/tests are identical to validated checkpoint
`b65ad6cd086f76124bc6884308c6751f50bf2b3a`; the final commit changes only docs.
All four graph cases fail by assertion against the old watcher and pass after:
populated-directory/symlink/loop, real return checkout plus edit/removal, hidden
ancestor with nested ignore rules/exclusions, and a changed ignore rule before
queued-file flush. Return checkout reaches exact graph contents in **2.183042707
seconds**, inside the unchanged absolute **12-second** deadline starting before
checkout and checked after reads. Outside sentinel bytes/mtime/content count
are preserved. #6's unchanged receipt regression passes **1/1**; supporting
reader/writer test has **52 correct / 0 errors / 0 wrong** in that run. Strict
focused Clippy, formatting, whitespace and merge construction pass. These are
author executions with raw receipts independently checked, not reviewer reruns
during the provisional trial's Sable CPU window.

The integration reviewer accepts the scoped Linux merge with these explicit
residuals: active native writes can produce unsuccessful checkpoint/WAL-missing
reads; only later valid exact snapshots inside the same deadline count as fresh.
The ordinary duplicate-watch control passes, while an injected nine-second
startup still reproduces the existing five-second cleanup-budget assertion.
Ignore changes affect subscriptions/queued files, not a retroactive purge of
already indexed records. No error-free native reads, real crash-WAL recovery,
full-workspace or cross-platform acceptance is claimed. This is a source GO;
broader operational acceptance stays HOLD.

For #9, diff/merge construction checks pass. The initial review found a
reachable P2 was sent to its owner: small incremental batches delete every
changed module, then each per-file insert writes imports using only the modules
reinserted so far. With `main.ts` importing `./helper.js`, reindexing in order
`helper.ts`, `main.ts`, `helper.tsx` can create a temporary `main -> helper.ts`
edge. Final resolution correctly sees ambiguity, but its MERGE-only pass does
not remove that provisional edge. Resolve batch imports against the final
module set and add an order-sensitive graph regression. The owner accepted this
finding and changed small batches to defer IMPORTS until the final set exists,
with ambiguity and temporary-JavaScript-fallback regressions at `d2e07901`.
The owner reports the original import regression failing on base `4d5266fe`
and the ordering regression failing on `32459fd`, both exit 101. At `df7a8687`,
all **8 import tests** pass, including both batch orders and temporary JavaScript
fallback, followed by **29 focused resolver/storage/SCIP tests** passing. These
are author-reported executions, not reviewer native reruns. The only change
since reviewed `d2e07901` shares one immutable fixture registry through OnceLock;
production code is unchanged. The reviewer confirmed this diff and a clean
merge construction against current base, but the ancestry check shows #7 is not
yet included. The existing import thread stopped before the remaining caller
and combined acceptance receipts and was resumed to finish them on the current
base. Its PR body still describes an obsolete measurement hold. Missing hosted
checks are expected from main-only base CI; no workflow workaround is required.

## Limits and operational decision

- **Import graph:** labelled TypeScript dependencies are missing despite present
  endpoints. Empty IMPORTS does not establish absence. Pending fixes must test
  cold and incremental persistence, resolution ambiguity, and excluded paths.
- **Watcher freshness:** the scoped hidden-ancestor, return-checkout, queued-rule
  and symlink cases are closed on merged #7 by direct graph checks. This is not
  universal freshness or error-free-read proof. Search text fallback remains
  insufficient. Delayed-start duplicate-lock failure is test cleanup timing,
  not demonstrated duplicate watcher creation; preserve that distinction.
- **SCIP completeness:** #6 fixes batch receipt reuse only. Direct `index_file`
  mutation, duplicate stored edges, unpropagated import write errors, unresolved
  owners and unavailable compiler tools remain limitations. A successful stage
  receipt is not exhaustive graph or working-tree freshness proof.
- **Benchmark/agent A/B:** old default-feature baseline exit 132 and candidate
  partial-index exit 1 remain in the historical report. New no-default-feature
  measurements are a distinct experiment on pinned old source. No speedup,
  current-base graph acceptance, compiler completeness, token savings, or agent
  task differential is established. No fixed-model agent A/B has run.
- **Broader runtime:** cross-platform/full-workspace, organic OOM, real
  crash-generated WAL recovery and general native concurrency acceptance remain
  unverified. Focused Linux checks do not settle them.

**GO:** the reviewed narrow #6 and #7 source integrations.
**HOLD:** #5 and pending import/benchmark deliveries, remaining combined
freshness/completeness/performance acceptance, and operational use claims.
No deployment or activation is authorized or performed. No new service,
container, schedule, credential, database, monitoring duty, or installation step
is introduced by this receipt.

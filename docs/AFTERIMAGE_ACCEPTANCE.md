# Afterimage integration acceptance

Review date: 2026-10-24 UTC. This is a source-integration receipt for the personal
fork, not installation, deployment, release, or runtime authority.

## Bound identities and decisions

- Repository/base: `redxzeta/afterimage:afterimage-main`.
- Starting base: `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`.
- Current integrated base: `4d5266fe10c2c39dbaac423c7a43731329601e85`.
- Protected fork mirror `main`: `cf82f5d7440d9be158e219df1ef256d6359ed1f6`,
  unchanged after the merge. Upstream PRs #78/#81/#82/#83/#84 remain separate
  from fork acceptance; this review makes no upstream mutations.

| Fork PR/lane | Exact reviewed head | Outcome and remaining condition |
| --- | --- | --- |
| [#6: SCIP receipts](https://github.com/redxzeta/afterimage/pull/6) | `f85d9efa55447de4fa46af3bdd97b5fd6b3a7f01` | MERGED with expected-head guard; merge `4d5266fe10c2c39dbaac423c7a43731329601e85`. Independent source/test review passed; 53 focused test receipts remain applicable. |
| [#5: focused CI](https://github.com/redxzeta/afterimage/pull/5) | `019979b5f8bf7cb6b08d290b4786e234faf32050` | HOLD: hosted CLI parity failure still lacks root-cause disposition. Author revising diagnostics and future watcher-test selection. Review replacement head before merging. |
| [#7: watcher directories](https://github.com/redxzeta/afterimage/pull/7) | `1c1fe4ffdf0838f11c61c6a7909a2ca606f23929` | HOLD: follow-up for hidden ancestors and return-checkout latency in progress. Original direct-graph regression passes; lifecycle receipt retains a failed case. |
| Three-repository benchmark follow-up | No delivery PR yet | Measurements active; candidate pinned to `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`, baseline `cf82f5d7440d9be158e219df1ef256d6359ed1f6`, harness `286faf925590156f30b82372f4c0c94f103f10d4`; both binaries use no default features. Final evidence/review pending. |
| TypeScript import follow-up | No delivery PR yet | Reproduced missing IMPORTS with both Module endpoints present. Relative-path and incremental-write fixes/tests in progress; no merge decision. |

Every remaining delivery must include the current `afterimage-main` base and
receive a fresh diff/conflict review. Passing one PR does not establish combined
acceptance of still-unmerged changes. The issue-48 advisory trial is provisional,
outside this gate, and has no merge authorization here.

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
exact-head snapshot: **PASS, 12 tests, exit 0**. A separate selector probe
confirmed that future watcher source changes omit #7's `watch_new_directory`
target once that test has landed; targeted feedback was sent to its owner.
The replacement must retain this direct-graph regression while handling its
absence before #7 lands, plus graph/SCIP/process/benchmark coverage. Another
probe confirmed CI/selector/unknown-input changes select `workspace` but omit
watcher and graph-lock suites; conservative expansion needs its own regression.

[Hosted run 40068158249](https://github.com/redxzeta/afterimage/actions/runs/40068158249)
on #5's reviewed head passed formatting, all-target Clippy and CLI/MCP build,
then failed CLI parity: 2 passed / 1 failed. Fixture indexing failed with empty
captured stderr; child status/signal/stdout were not recorded. Later suites did
not run. Cause remains UNKNOWN; local passes do not establish hosted parity.
The author's earlier 90 integration/683 unit/14 benchmark-harness passes remain
reported evidence on the documented source, not fresh reviewer executions.

For #7, [the original report](https://github.com/redxzeta/afterimage/blob/1c1fe4ffdf0838f11c61c6a7909a2ca606f23929/docs/PHASE-D-RELIABILITY-FOLLOWUP.md)
records the direct-graph directory regression passing, but lifecycle completion
returns exit 1 with **28 checks passed / 1 failed**. Return checkout exceeded
12 seconds and was only later correct at 25.510 seconds. Hidden-ancestor edits
also reproduced stale graph data. Neither a higher timeout nor eventual
correctness alone satisfies the follow-up. New direct-graph checks and explicit
residual-risk disposition are required before a merge decision.

## Limits and operational decision

- **Import graph:** labelled TypeScript dependencies are missing despite present
  endpoints. Empty IMPORTS does not establish absence. Pending fixes must test
  cold and incremental persistence, resolution ambiguity, and excluded paths.
- **Watcher freshness:** original hidden-ancestor staleness and return-checkout
  latency remain open until the replacement is validated. Search text fallback
  is insufficient; assert actual graph state. Delayed-start duplicate-lock
  failure was classified as test cleanup timing, not demonstrated duplicate
  watcher creation; preserve that distinction.
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

**GO:** the reviewed narrow #6 source integration.
**HOLD:** #5/#7 follow-up merges, pending import/benchmark deliveries, combined
freshness/completeness/performance acceptance, and operational use claims.
No deployment or activation is authorized or performed. No new service,
container, schedule, credential, database, monitoring duty, or installation step
is introduced by this receipt.

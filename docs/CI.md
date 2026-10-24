# Afterimage verification

Run `python3 scripts/verify.py` from any directory with Rust **1.98.1**, rustfmt,
Clippy, Python 3.9+, CMake, a C/C++ toolchain, pkg-config and OpenSSL development
files available. Cargo uses the committed lockfile. Linux benchmark isolation
checks also require Bubblewrap; supervisor checks require Linux and rustc.
Cold native builds remain expensive. Dependencies/native build inputs must be
preseeded for offline use (`CARGO_NET_OFFLINE=true`). Default features exclude
kompress/ONNX Runtime. This is source validation, not installation or activation.

The command compares the merge base of `origin/afterimage-main` with the current
working tree, including staged, unstaged, untracked and deleted paths. Renames
include both paths. CI supplies the PR base SHA or push's previous SHA through
`VERIFY_BASE`; unavailable history expands coverage. Use `--base <SHA>` for a
pinned comparison, `--dry-run` for the selected commands, and `--full` for all
default-feature workspace unit, integration and doc tests. Each command prints
its duration and stops on failure. Run application tests serially to avoid
embedded-database resource contention.
Selected integration targets run in one workspace invocation to unify features
and avoid rebuilding shared crates between package-specific commands.

## Shared contract and coverage

The workspace's shared contract is executable code, not a separate shared npm
package: `infigraph-core` models, graph schema/backends, extraction, index/SCIP
receipts and diagnostics feed both CLI and MCP; language/plugin crates feed
extraction. CLI/MCP tool names, dispatch and output/diagnostic semantics must
agree. Cross-repository groups rely on a complete contract replacement and
rescanning unchanged consumers when another repository's producer changes.
These behaviors remain supported and their tests are not obsolete.

- Every PR: classifier and synthetic diagnostic regression tests, `cargo fmt --all -- --check`, locked
  workspace Clippy with **all targets** and warnings denied (also typechecks
  tests), build the actual CLI and MCP binaries, `embedding_format`, `cli_parity`
  (includes the process-level test-context regression), `tool_parity`, and
  `git diff --check`.
- Core/language/plugin changes: affected crate library/binary unit tests;
  `typescript_callers` and the integrated `typescript_imports` resolver/storage
  regressions (guarded by target existence);
  core `diagnostics::tests`, `index_status::tests`, `scip::tests`; interface
  checks below; plus `namespace_prefix`, `namespace_link`, `calls_service_edges`,
  `group_build_steps`, `combined_graph`, `grpc_cross_service`, `schema_parity`.
  In particular, `group_build_steps` retains the unchanged-consumer/changed-
  producer regression. The actual local graph/contract tests run, not substitutes.
- CLI/MCP changes: affected library/binary unit tests, CLI `doctor`, MCP `diagnose` and
  `coverage_reporting`, MCP `web::tests::`, and the cross-repo tests listed above.
  MCP changes also run the Linux
  synthetic supervisor process regressions. Other workspace crates run their
  library tests plus core/interface compatibility checks.
- Changed Rust integration-test files are also selected directly. Manifests,
  dependency locks, build configuration, fixtures, grammars, CI and unknown paths
  expand to every compatibility group plus workspace library and binary unit tests.
- Watcher paths additionally select real MCP watcher/reindex, parent-directory
  search and concurrent reader/writer process tests. Core graph/index/lock/watch
  paths select graph-query and lock/wiring regressions. Cargo manifests/lockfiles,
  native patches/config, CI/selector/unknown inputs and unavailable history select
  both of these groups. Watcher/shared-build coverage includes the integrated
  `watch_new_directory` direct-graph regression (guarded by target existence).
  Known watcher failures remain visible in these checks and the full suite;
  they are not disabled or reclassified as obsolete.
- Benchmark paths: compile the harness Python sources and run its synthetic
  unittest suite on Linux/Bubblewrap. This does not run benchmarks or execute
  the source repositories' applications. Supervisor script changes select its
  process tests. Docs-only changes keep the standard checks.

No source tests or supported cross-repo behavior were deleted. PRs stop running
the entire integration suite on all three platforms for every change. The full
workspace suite and macOS/Windows coverage remain available via **CI → Run
workflow → full=true** (default). Run it on the exact candidate before claiming
cross-platform or release readiness, plus focused optional-feature checks when
those interfaces change. Existing ignored remote Neo4j/Postgres tests still
require separately authorized services and are **not run** by a default suite.
Likewise all-feature linting alone cannot prove ONNX CPU/runtime compatibility.

## Workflow and release boundary

CI targets the actual fork development branch `afterimage-main`; `main` remains
the pristine upstream mirror. Ordinary PRs/pushes use one GitHub-hosted Linux
job; manual full validation uses Linux/macOS/Windows with fail-fast disabled.
Only superseded PR runs are cancelled. Cache writes are limited to pushes on
the fork development branch; PRs can restore dependencies but do not save them.
Build profiles omit debug symbols and native build parallelism is bounded.
The local command applies the same defaults, respects explicit environment
overrides and rejects a different Rust version or missing required Bubblewrap.

After a Linux verification failure, `scripts/diagnose_cli.py` reports the built
CLI's size/hash, recorded lbug source and candidate native archive/header hashes,
and separate CLI/debugger availability. It uses an already installed gdb or LLDB
(including the runner image's Swift toolchain) for one fresh parity fixture with
a 90-second bound. It never installs tools or rebuilds. Missing tools, timeout,
or a non-reproducing probe are incomplete diagnostic evidence; the original
verification failure remains fatal. Build metadata can include stale candidates;
its source label alone does not prove which bytes were linked or establish cause.
The failure step also records archive/bridge object compiler tags. The optional
`--compare-compilers` diagnostic, when GCC 13 and 15 are already installed, builds
one tiny C++ constructor/formatting fixture with each compiler against the same
archive/header/configuration. Each compile has a 30-second bound and each run
10 seconds, with separate disposable databases, core dumps disabled and process
groups killed on timeout. These are synthetic causal controls, not CLI test
passes. No compiler/environment pin is inferred automatically from their result.
Both standalone controls passed on hosted head `06fe48d`; they did not reproduce
the real CLI failure or validate a compiler pin. The standard failure follow-up
now preserves/hashes the original CLI and parity executable inside the job,
confirms that unchanged parity still reports index SIGSEGV (60 seconds), then
uses installed GCC13 to rebuild the same integration target context offline
(`--no-run`, 180-second bound). It runs the preserved, unchanged parity executable
against the rebuilt CLI (60 seconds), with the original runtime environment.
Native archive/header hashes, Cargo feature sets and linked libstdc++ hash are
compared before/after; drift is incomplete causal evidence even if tests pass.
No fresh target/dependency build or artifact/database upload is performed; the
original primary verification failure remains fatal. Copied original binaries
last until runner disposal; only logged evidence is durable.

The inherited broad cleanup (including Docker image pruning) is removed from
PR CI; native builds still need adequate disk. No browser/e2e browser job existed
in this repository, so none is claimed removed. The repository runner API
returned zero runners during the audit: Blackwall labels/access are unverified,
so no self-hosted runner is selected.

`release.yml` and `pages.yml` are unchanged, including fork publishing guards,
release platform matrix and optional remote release features. The inherited
release workflow has no dependency on CI: this PR does not invent a release
test gate or claim manual validation automatically gates publishing. Keep the
[release boundary](FORK_RELEASE_SAFETY.md); no tags, releases, publication,
deployment or runtime changes are part of CI simplification.

## Baseline evidence (2026-10-23 UTC)

Audited source base: `1a59c728ae3284f0af3b5c781e14a9e2736df6e0` on
`afterimage-main`. Existing CI filtered `main`, so integrated fork heads had no
new CI run. Recent PR runs (elapsed creation to completion):

- [40005432840](https://github.com/redxzeta/afterimage/actions/runs/40005432840):
  246 seconds, failure; head `054dcdd1f9f07eb150a3c69c61999ec6ead55be7`.
- [40008650490](https://github.com/redxzeta/afterimage/actions/runs/40008650490):
  167 seconds, failure; head `10ec96212791d68bd5340f54b406f7962cf2769c`.
- [40009264684](https://github.com/redxzeta/afterimage/actions/runs/40009264684):
  171 seconds, failure; head `a158bdf51b0fb3cc2828dc239fdfaf53a98cea74`.
- [40013598833](https://github.com/redxzeta/afterimage/actions/runs/40013598833):
  227 seconds, failure; head `f6906fa75148824ac5f6e0c5a867c08b99ac043e`.

The two inspected job breakdowns showed Ubuntu cleanup at 103/133 seconds,
format jobs at 16/13 seconds, and tests skipped after Clippy failure/cancellation.
The latest macOS failure log specifically identifies Rust **1.99.0**'s new
`chunks_exact_to_as_chunks` lint in `embed/mod.rs:402`. CI pins the available,
previously locally validated **1.98.1**; it retains `-D warnings` with no lint
allowances. Toolchain upgrades need explicit lint review. Failed/cancelled
baseline durations are not successful test-suite timings and do not support a
claimed end-to-end speedup percentage. New exact-head results belong in the PR.

Recent fork PR creation-to-merge times were #2: 6h27m51s, #3: 6h04m42s,
and #4: 5h59m52s (merged on 2026-10-24 UTC). Those include review/manual
integration delay and are not CI execution times.

The upstream-readiness lane owns runtime/SCIP fixes and its readiness document;
the A/B lane owns benchmark source/results; Sable reliability code is untouched.
Their branches, application checkouts and parallel PRs are outside this change.

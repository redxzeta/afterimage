# Bounded indexing for agents

Use an explicit project root and select the stages you need:

```sh
infigraph --root /path/to/checkout index --ast-only
infigraph --root /path/to/checkout index --enriched --no-embed
```

`--ast-only` runs the existing syntax graph and local analyses, skips embeddings
and document indexing, and never launches or provisions SCIP tools. It also skips
watcher startup and the update check. Exit 0 means the requested syntax stages
succeeded. Compiler coverage remains partial: each applicable SCIP stage is
`{"state":"skipped","reason":"NOT_REQUESTED"}`. It does not mean all graph
relationships were discovered or that the working tree is fresh.

`--enriched` checks all applicable compiler tools, runtimes and project
prerequisites before launching any indexer. It uses existing executables from the
normal PATH and existing Infigraph runtime/cache directories. The SCIP path does
not install or download tools/runtimes; `--no-embed` also skips embedding model
setup. Missing prerequisites fail with exit 1 and a stable
`SCIP_TOOL_UNAVAILABLE` or `PROJECT_PREREQUISITE_MISSING` reason. Provide the named
tool/runtime and project prerequisites deliberately, or choose `--ast-only`.
Retry only after prerequisites change. A present executable can still fail at
runtime; availability is not compiler qualification. External compiler/build
tools may themselves require the project's dependencies and normal build setup.

Enrichment runs in the foreground in explicit enriched mode; process or import
failure stays failed. `--no-embed` only disables embeddings; it does **not** select
AST-only indexing. With neither new flag, existing indexing behavior is retained,
including its automatic provisioning/background enrichment path. The flags are
mutually exclusive. No new service, schedule or configuration is required.

The existing `.infigraph/index-status.json` receipt remains schema v1 and is the
machine-readable status contract. Human progress stays on stdout; warnings,
preflight explanations and coverage notices use stderr. Check the receipt after
indexing rather than parsing progress output. `resolution` describes the recorded
local call-resolution stage; `scip` is keyed by indexer name. Existing failed,
pending, running and unknown outcomes remain incomplete. Signal cancellation is
`failed/SCIP_PROCESS_CANCELLED` with its signal number. A receipt describes the
indexed generation, not working-tree freshness or exhaustive coverage. MCP must
consume this receipt rather than maintaining a second tracker.

Local `index_file` now uses the existing AST receipt lock and generation identity.
An unchanged source/grammar fingerprint retains compiler edges and successful
stages. Changed source invalidates compiler success before graph writes, and late
workers cannot import into a newer AST generation. A failed read/extraction before
mutation leaves the graph and receipt intact.

Successful automatic/explicit enrichment is checked again under the receipt lock
immediately before storage mutation. A competing worker for the same run/indexer
cannot append the same compiler edges a second time or overwrite success with
its later failure. Independent indexers still have independent receipt entries.

## Scoped follow-up: raw importer atomicity

This change does not make arbitrary `scip-import` calls idempotent. The raw native
importer uses multiple COPY/UNWIND writes, native retry helpers and ignored write
errors. Raw repeated artifacts can still append duplicate CALLS/INHERITS. A native
failure partway through import is not guaranteed to roll back earlier writes.
Malformed protobuf is rejected before native writes; that limited failure case
preserves previous graph data.

A separate change must define compiler-edge ownership and atomic replacement,
retain AST-owned edges and other files/projects, propagate native errors, and test
rollback using the actual supported native transaction API. Do not delete all
edges or infer success from display deduplication. This PR confines idempotence to
successful imports for one receipt-owned run/indexer; it does not claim this
follow-up is complete.

The synthetic `tests/fixtures/scip/compiler-reference.scip` is a minimal SCIP
protobuf document for `app.ts`: a helper definition on zero-based line 0 and a
reference inside a multiline caller on line 2. It exercises the real importer;
it was generated locally, not captured from an external compiler.

## Focused validation

Run serially with a disposable HOME, TMPDIR and index root. Native compile/test
slots are shared with the MCP lane; do not overlap builds. Typical commands:

```sh
cargo test --locked --offline --no-default-features -p infigraph-cli --bin infigraph --no-run
cargo test --locked --offline --no-default-features -p infigraph-cli --bin infigraph index::pipeline_tests -- --test-threads=1
cargo test --locked --offline --no-default-features -p infigraph-cli --bin infigraph index::outcome_tests -- --test-threads=1
cargo test --locked --offline --no-default-features -p infigraph-cli --bin infigraph scip_download::tests::existing_tool_preflight -- --test-threads=1
cargo test --locked --offline --no-default-features -p infigraph-core --test typescript_callers -- --test-threads=1
```

For a smoke test, project the pinned source fixtures into a disposable repository,
record source hashes before/after, run AST-only twice, and inspect the receipt.
Then request enriched mode with no external tools: expect exit 1 and the missing
tool code while syntax symbols remain queryable. Do not provision tools just to
turn an offline test green. Real external SCIP is NOT_RUN unless explicitly run
with an already available compiler tool. Hosted CI is not a prerequisite for
publishing this implementation; no test or protection is disabled.

Local execution evidence (2026-10-27, base
`dc45de6e5769c1d86116167dcb1660d9561736c8`):

- Targeted rustfmt check and `git diff --check`: passed. An initial full formatting
  check identified formatting in the new edits; targeted rustfmt corrected it.
- Initial CLI test `--no-run`: timeout at 300.076 seconds, exit -9 (wrapper 137),
  while compiling `pdf_oxide`; tests did not run. No source/native error was
  emitted before the cap. The native slot was released to the MCP lane.
- Compiler: Rust 1.99.1 (`58cf8f9c3`), Cargo 1.99.1. One Cargo/CMake job,
  dev/test debug=0, existing repository linker flags, offline/locked dependency
  resolution, disposable HOME/TMPDIR/target copied from a prior disposable cache.
- lbug 0.19.1 cached build output reports
  `LBUG_PRECOMPILED_SOURCE=release:LadybugDB/ladybug/latest`;
  cached `liblbug.a` SHA-256:
  `9100591f7cf1de0f565c96b55e3293998763260fc936bfbb9e304bef6f4ec2b8`.
  This is cached native provenance, not a reproducible upstream release pin.
- Source Cargo.lock SHA-256:
  `f1c33ba66829fb179a1ceb48d3c2036776812953e6d8b6cc1d0a277c45689eca`.
- Final native slot reused the MCP lane's completed debug0 dependency cache,
  rebuilt this checkout's core/CLI source, and passed CLI test `--no-run` in
  11.674 seconds. CLI `cargo build --locked --offline --no-default-features
  -p infigraph-cli --bin infigraph` passed in 5.969 seconds.
- Focused commands above: pipeline **5 passed**, outcomes **3 passed**, existing
  tool preflight **1 passed**. Existing CLI argument regression
  `index::tests::scip_enrich_args_parse_as_positional_language`: **1 passed**.
  All commands used `--test-threads=1`; total **10 passed, 0 failed, 0 ignored**.
  Command wall times: 24.965s, 4.178s, 4.161s and 4.157s respectively.
- Core `typescript_callers` command: **BLOCKED**, exit 101 in 0.102s because
  `cargo-husky v1.5.0` is absent from the offline cache. No core integration test
  ran; the dependency was neither removed nor downloaded, and no retry was made.
  The CLI tests exercised the actual core receipt/import paths successfully.
- Pinned four-file `tests/fixtures/python-simple` projection from the base above:
  **PASS**, smoke driver exit 0 in 14.254s. AST-only first/repeat returned 0;
  explicit enriched/missing tool returned 1 with `SCIP_TOOL_UNAVAILABLE`;
  explicit AST-only recovery returned 0; `doctor --json` returned 0. Resolution
  remained `succeeded`; compiler status changed from `skipped/NOT_REQUESTED` to
  `failed/SCIP_TOOL_UNAVAILABLE` only when requested, then to `NOT_REQUESTED` on
  the explicit AST-only request. No indexer launch, provisioning or automatic
  retry occurred. No watcher lock, global Infigraph directory, embeddings or
  generated CLAUDE.md appeared. All four source hashes and retained `index.scip`
  bytes were unchanged. First graph: 14 symbols, 4 files/modules, 6 CALLS,
  1 INHERITS; repeat indexed 0 files. The resolver reported 5/9 cross-file calls
  resolved and 4 unresolved builtins/externals; these counts are not quality or
  exhaustive-coverage scores.
- Synthetic real importer guard: first import retained the AST-owned call and
  added one compiler reference (stored CALLS count 2); repeated same-attempt
  import kept count 2. Missing-file reads/malformed protobuf preserved the valid
  receipt bytes; no-op retained success and both edges; source mutation removed
  success, preserved the independent file and rejected obsolete workers.
- Built CLI SHA-256:
  `c393a67ec9e2aff1bd25511efacbb9a5a5f518fd1e493c22b7b87f2f8cef739c`.
  Focused test executable SHA-256:
  `829e7039cb47dc2ea10c8db164a7b46729cbe6547134fd7a81bc2c3e0a12da55`.
  Synthetic SCIP fixture SHA-256:
  `5b95eb1c6c4478344fba6e1a3a8b01acbb1e3fbf18d8b3e5196cda4f8adfd067`.
- Real external SCIP compiler: NOT_RUN. The synthetic protobuf fixture and fake
  subprocess tests do not establish external compiler quality or readiness.

# Reproducible offline A/B benchmark

`bench.py` is a Python 3.11+ standard-library runner. It never builds Infigraph or
executes a fixture project's code, tests, install scripts, hooks, or applications.
Linux `/proc`, Bubblewrap (`bwrap` with unprivileged namespaces), `git`, and `which` are
required. GNU `time` is optional. Missing prerequisites have no unsafe fallback.

## Source truth

The three manifests under `truth/` contain source-reviewed definitions, positive
and negative CALLS relationships, module imports, and five impact questions each:

| Project | Pinned commit | Symbols | CALLS labels | Import labels | Questions |
| --- | --- | ---: | ---: | ---: | ---: |
| IceCouncil | `10435a8c19f84549a0206f6b249ea13166aff6d0` | 21 | 14 | 2 | 5 |
| Nightwire | `80985e6fc508f309b17ba1889ff2ca15b0253c08` | 23 | 21 | 3 | 5 |
| GigLab | `02cee19056c8ee902289892727a6acbbcd30a7d3` | 24 | 19 | 3 | 5 |

These are the locally available default remote refs when curated, rather than
the possibly older/dirty checkout HEADs. The runner requires full commit SHAs.
Every declaration/relationship/question links to exact source lines and full-file
SHA-256 digests. Impact questions include explicit source-labelled call chains.
The public manifests store line hashes rather than source excerpts.
`verify` checks these anchors against Git objects, independent of graph output:

```sh
python3 scripts/benchmark/bench.py verify \
  --repo "$ICE_REPO" --truth scripts/benchmark/truth/ice-council.json
python3 scripts/benchmark/bench.py verify \
  --repo "$NIGHTWIRE_REPO" --truth scripts/benchmark/truth/nightwire.json
python3 scripts/benchmark/bench.py verify \
  --repo "$GIGLAB_REPO" --truth scripts/benchmark/truth/giglab.json
```

Covered examples include arrow helpers, `map`/`sort` callbacks, recursion, nested
function ownership, import aliases, same-name serializers in separate modules,
test-local helpers, and workspace subpath imports. A callback passed to `map`
counts as a source relationship of its lexical owner. Named nested functions own
their own calls. Anonymous test callbacks have source anchors, but no invented
stable graph IDs: those callback caller IDs still need manual adjudication.

This is an **open-world subset**, not whole-repository truth. `recall` is recall
over verified positives. `false_edges` includes only observed, explicitly negative
labels. `labelled_precision` is TP/(TP+labelled FP); overall `precision` remains
`null` because other observed edges are `unadjudicated`. `missing` is a known
positive absent from query output. An absent negative edge is recorded separately
and does not make an empty graph correct. Query failures, malformed rows, and
inconsistent repeated answers are `UNRESOLVED`, not empty successful answers.
Missing symbol discovery and engine-reported unresolved call counts are separate.

`--fixture-scope truth-files` exports only files with truth anchors, preserving
their relative paths and exact bytes. It is an explicit smaller fixture, without
the rest of the repository or a dependency closure; it is never silently chosen
after a failure. The default `repository` projection remains unchanged. Always
retain `controls.fixture_scope` and `exported_files` with results; timing or quality
on these smaller fixtures is not repository-wide acceptance.

## Run a pair

The build owner must finish **all native builds first**, save immutable, distinct
baseline/candidate CLI and MCP binaries, and reserve the host for timing. Building
both revisions into the same target path without saving the first binary destroys
the baseline. Record the build feature/profile/toolchain manifest externally.
Binary SHA-256 is computed from the copy actually run; supplied source revisions
are operator attestations, not inferred from `--version`.

Set `BASELINE_CLI`, `CANDIDATE_CLI`, `BASELINE_MCP`, `CANDIDATE_MCP`, `ICE_REPO`,
`NIGHTWIRE_REPO`, `GIGLAB_REPO`, and `BENCH_SCRATCH` to operator-owned paths.
Prefer persistent scratch storage for large repositories; RAM-backed `/tmp` is
appropriate for tiny smoke fixtures, not Cargo targets. No Cargo command runs here.

```sh
mkdir -p "$BENCH_SCRATCH"
python3 scripts/benchmark/bench.py run \
  --baseline "$BASELINE_CLI" --baseline-mcp "$BASELINE_MCP" \
  --baseline-revision cf82f5d7440d9be158e219df1ef256d6359ed1f6 \
  --candidate "$CANDIDATE_CLI" --candidate-mcp "$CANDIDATE_MCP" \
  --candidate-revision 4ed6708c4a9fbb693435079288d7242b3f30f1ef \
  --repo "$ICE_REPO" --commit 10435a8c19f84549a0206f6b249ea13166aff6d0 \
  --truth scripts/benchmark/truth/ice-council.json \
  --temp-root "$BENCH_SCRATCH" --output "$BENCH_SCRATCH/ice-pair-01.json" \
  --no-embed --threads 1 --repeats 3 --timeout 120
```

Run Nightwire and GigLab sequentially with their corresponding repo, truth, commit,
and unique output names from the table. Add `--freshness` to exercise the opt-in
synthetic fixture in each disposable copy: commit A, branch B with a renamed arrow,
return to A, an uncommitted symbol edit, a file rename, and deletion. Results
require exactly the expected symbol set after each reindex. Each case retains the
source fixture digest, synthetic Git HEAD and available index receipt fingerprint
and run ID before and after reindex. Missing receipts are unknown
metadata coverage. These tests cover manual reindex freshness;
they do not claim automatic watcher correctness. The normal incremental case
appends a comment to a real tracked TS file, reindexes, restores, and reindexes.

Add `--test-context` to probe CLI `test-context --file <first anchored file>
--limit 1` and direct-worker MCP `generate_test_context`. Nonzero exits, signals,
missing expected output and MCP errors are failures. The direct-worker probe
avoids supervisor recovery hiding a worker crash. Core dumps are disabled in every
lane. Pass `--model-dir models/potion-base-8M` alongside `--no-embed` to make the
same offline model assets available to these probes without generating embeddings
during indexing. The normal MCP graph probe still exercises the supervisor path.

Omit `--no-embed` and pass `--model-dir models/potion-base-8M` to include embeddings.
The runner copies/hashes the same `config.json`, `tokenizer.json`, and
`model.safetensors` into each lane. Missing offline model assets are an error.
The current query suite measures structural graph quality, not semantic search.

One user-wide, nonblocking file lock serializes harness invocations. A/B refuses
to start when native builder processes are visible. Commands sample builder names
at their boundaries; any detected build invalidates the timing comparison. This
sampling cannot prove host idleness between boundaries: coordination with the
build owner is still required. Reverse order with `--order candidate,baseline` in
a second pair with a new output name to investigate order/cache bias. No parallel
builds/runs, filesystem cache flushing, or statistically significant speedup claims.

## Isolation and measurement

Each lane exports only regular tracked code files and selected package/ignore
configuration from `git archive`. See `CODE`/`source_file` for the exact allowlist.
Symlinks, submodules, installed dependencies, reports, databases, local sessions,
and existing index state are omitted. This export is a declared source projection,
not a dependency-installed checkout. Actual source checkouts receive only Git
object reads, with optional locks disabled. Index writes occur inside the export.

Bubblewrap exposes system libraries read-only and one private writable `/bench`
root. It gives the subprocess an isolated network/PID namespace, scrubbed
environment, separate HOME, registry, state/cache/tmp roots, and a tool PATH
containing only the copied binaries, Git, and `which`. Host home directories,
credentials, source checkouts, runtime sockets and deployed data are not mounted.
Namespaces die with the process, including detached watchers/enrichment children.

**`--no-embed` does not disable SCIP in Infigraph.** No SCIP indexer/runtime is
provisioned by this harness. Bootstrap may be attempted and fail offline; retain
that failure rather than calling it successful compiler enrichment. SCIP execution
is `NOT_RUN`. No-embed is therefore AST-only; embedding mode is AST+embeddings.
Full compiler enrichment needs a separate approved offline tooling fixture.

Metrics distinguish cold (empty index + HOME), warm (same index), changed-file
incremental, and restored indexing. `indexed_files` records the CLI count if its
known text format matches, otherwise `null`; warm is not assumed to be a no-op.
Cold does not mean a cold OS page cache. Each impact query is repeated serially;
its median includes CLI startup, graph open, and traversal. Default per-command
limits are 120 CPU seconds and 4096 MiB summed descendant RSS, adjustable with `--cpu-seconds` and `--memory-mib`; `--timeout` caps wall time.
`--threads` also constrains inherited CPU affinity for the entire sandbox,
including native graph workers; Rayon/OpenMP environment hints alone cannot
enforce this. The Linux `/proc` monitor samples every 50 ms and kills the private sandbox on a
breach (`BUDGET_EXCEEDED`), including MCP commands. These are sampled limits,
not a cgroup allocation cap: short-lived children and between-sample peaks can
be missed, and shared pages can be counted more than once. Sampled CPU and peak
tree RSS are separate from optional GNU-time max RSS. GNU-time max RSS is
reported in KiB when available, otherwise `null`; it is not summed concurrent RSS.
MCP uses stdio initialize, tools/list and query_graph. Session wall time includes
startup/handshake/list/query, not per-request latency. The client kills its private
server after all expected responses; `stopped_after_responses` records this normal
cleanup, separately from a protocol timeout or tool error.

## Artifacts and handoff contract

`afterimage-benchmark-v1` JSON uses sorted keys, UTF-8 and stable field shapes; times
are observed samples and naturally vary. The sibling `.txt` is the compact summary
and `.artifacts/` contains bounded stdout/stderr logs. Existing output/artifact
roots are refused. Temporary roots are removed; no absolute host paths appear in
the handoff JSON. PR #84's `*status*.json` files are retained verbatim per index step
and at lane end, including unknown future fields. Their absence is unknown coverage.
CLI exit zero does not mean all analysis stages succeeded.

PR #84 can exit 1 after completing syntax indexing when an analysis stage fails.
The runner preserves that failed command and may collect syntax-only evidence
when a schema-v1 receipt explicitly records successful resolution, a nonempty
fingerprint, and failed SCIP stages. `index_disposition=PARTIAL_SYNTAX_GRAPH` makes
this visible. Signals, timeouts and unknown/absent receipts are never salvaged.
Freshness assertions separate `graph_status` from `index_status`; matching graph
rows after a partial index are `PARTIAL_GRAPH_MATCH`, not a passed index command.
An index failure makes the pair `INCOMPLETE_INDEX_PAIR`; its failure duration is
not a successful cold-index performance sample.

`COMPLETED` describes harness execution, not graph acceptance. Read quality scores,
stage outcomes, missing positives, unadjudicated observations, and freshness cases
separately. Failed steps return exit 1; refused prerequisites/inputs return exit 2.
Identical CLI hashes are labelled `SMOKE_ONLY_IDENTICAL_BINARIES`. The harness does
not execute agents or report fabricated model tokens, task time, or answer quality.
`agent_differential` remains `NOT_RUN` until an actual fixed-model adapter is supplied.
The existing `tests/compression_eval/run_eval.py` exercises MCP text compression and
estimates tokens from whitespace; it is not a measured model/agent task executor.

Upstream/integration owner: provide both exact SHAs, immutable binary paths/hashes,
feature/profile/toolchain metadata and an idle-host handoff. Sable/reporting owner:
consume JSON + `.artifacts/`, retain source pin, truth hash, comparison kind, stage
outcomes and unresolved/unadjudicated fields; do not turn partial scores into a
release, operational-readiness, or model-quality gate. This lane does not edit
other worktrees, merge, release, deploy, or activate anything.

## Local validation / installed binary smoke

```sh
python3 -m unittest discover -s scripts/benchmark -v
python3 -m py_compile scripts/benchmark/bench.py scripts/benchmark/test_bench.py
python3 scripts/benchmark/bench.py smoke \
  --binary "$INSTALLED_CLI" --mcp "$INSTALLED_MCP" \
  --output "$BENCH_SCRATCH/installed-smoke.json" --timeout 15
git diff --check
```

`smoke` indexes a two-file synthetic TS import/arrow fixture and checks CLI graph
queries plus MCP. It records installed binary hashes with source revision
`UNKNOWN`, never substitutes them for an exact-SHA A/B pair, and makes no timing
comparison during a build. Tests use tiny synthetic executables and Git repos to
exercise orchestration, timeout cleanup, source drift checks, branch/deletion
fixtures, scoring, and partial-status preservation. Those fake-binary tests are
explicitly harness tests; their values are never benchmark evidence.

## Engineering evidence, 2026-10-23

All 68 symbols, 54 call labels, eight import labels and 15 impact questions passed
source-anchor verification. The 14 Python harness tests and Python compilation
passed; `cargo fmt --all -- --check` and `git diff --check` passed. Full Rust tests
and Clippy were not run in this lane: the independent integration owner holds the
single-job native build lane. This PR changes no Rust implementation or CI workflow.

The installed `infigraph 3.2.16` binaries completed CLI indexing/query and MCP
initialize/list/query on the tiny synthetic fixture. Both expected arrow symbols
were present, but the known `caller -> helper` cross-file relationship was absent.
See `installed-smoke-20261023.json` for hashes and the bounded result. Its source
revision is unknown. Timings were excluded from that summary because the native
build was active; RSS was unavailable. This result is not exact-SHA baseline or
candidate evidence. The first client trial timed out after successful MCP replies
because the installed server retained its private UI loop; the final runner waits
for responses and explicitly terminates that server, covered by a regression test.

Following the build owner's explicit idle-host handoff, the exact binaries were
exercised serially on all three projects. See the [measured evidence and exact
commands](results/2026-10-23.md) and [machine-readable summary](results/2026-10-23-summary.json).
The installed-smoke JSON above is a historical snapshot from before that handoff;
its `NOT_RUN` field is not the status of the later exact-binary runs.

Every baseline cold attempt exited 132. Candidate commands exited 1 with explicit
partial syntax-graph receipts because offline SCIP tools were unavailable. The
candidate's measured fixtures recovered 68/68 symbols and 48/49 labelled positive
calls, with one labelled false call edge, 550 unadjudicated calls and eight missing
positive imports. IceCouncil quality uses eight anchored files; its failed full
projection attempt is retained separately. Overall precision remains `null`.

Candidate CLI/MCP probes completed and 15 manual freshness graph assertions
matched, all as `PARTIAL_GRAPH_MATCH` after failed index commands. These are not
complete compiler enrichment, baseline quality evidence, automatic-watcher
acceptance, semantic-search/model quality, exhaustive impact or agent task results.
Confidence is high in these bounded observations; recommendation is to hold
comparative-performance and full-acceptance claims. No speedup is established.

## B/C/E follow-on validation

See [matched no-default-feature evidence](results/2026-10-23-bce.md) and its
[summary](results/2026-10-23-bce-summary.json). It supplements the historical
default-feature crash runs above; source revisions and build features differ.

`run --tool-questions` adds a deterministic, bounded MCP retrieval protocol:
initialize, list tools, then one `query_graph(detail=true)` for each of the first
five manifest questions in their declared order, with no retries. Scores use
the same source-labelled positives as CLI impact queries. Response UTF-8 bytes
and whole-session time are observed; they are not model tokens, per-tool latency,
agent reasoning quality or a controlled agent A/B. `agent_differential` remains
`NOT_RUN`. Nonzero command exits and malformed/error tool replies remain failures.

Cold import diagnostics independently query Module IDs/files, total IMPORTS
count and labelled sources by ID, before warm indexing. The normal file-based
score remains unchanged. The [synthetic import fixture](fixtures/imports/README.md)
separates quote/extension, re-export and workspace resolution cases without
executing target code. No source truth labels are weakened to match the graph.

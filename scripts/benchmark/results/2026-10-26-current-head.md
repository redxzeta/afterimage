# Current-head graph evaluation, 2026-10-26

**Tested candidate: `dc45de6e5769c1d86116167dcb1660d9561736c8`.** The refreshed fork default `afterimage-main` resolved to this SHA before the build. This report supplies current-head source-projection evidence after merged import PR #9 and watcher PR #7. It does not test the separate replacement for PR #13. The [older BCE report](2026-10-23-bce.md) and its tested candidate `1a59c728ae3284f0af3b5c781e14a9e2736df6e0` remain unchanged.

The campaign ran on 2026-10-26 in America/Los_Angeles (2026-10-27 UTC). Exact binary hashes, controls, stage receipts, source labels, relationship traces and question scores are in the [sanitized summary](2026-10-26-current-head-summary.json). Raw reports and bounded stdout/stderr remain in the operator-owned `$BENCH_ROOT/runs/`; only their filenames and SHA-256 hashes are published. Unadjudicated source-derived names, source excerpts, credentials and private host paths are excluded.

## Scope and reproducibility

- Baseline: original `cf82f5d7440d9be158e219df1ef256d6359ed1f6` CLI/MCP binaries, verified against the immutable hashes in the older summary and executed again in this campaign. Old measurements are not substituted for new runs.
- Current CLI/MCP: built from an exact disposable `git archive` of `dc45de6e`, using cached native dependencies, `cargo build --locked --offline --no-default-features -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp`, dev profile debug=0, one Cargo/CMake job and one CPU affinity. Rust/Cargo 1.98.1, CMake 3.31.6; Intel Core i7-3770. Both revisions retain their own lockfiles; hashes are recorded. An initial owned build was stopped during incomplete cache preparation; the private cache was recreated and the final build succeeded before any benchmark.
- Existing merged runner is used without edits: source revision `dc45de6e5769c1d86116167dcb1660d9561736c8`, `bench.py` SHA-256 `0ab5e2c2b20ad9ca6c9c090f980ad695443086fae736127473eb73339154e864`. Harness engineering and product implementation are outside this change.
- All three original truth manifests and target commits are unchanged and source-verified: 116 anchors, 68 symbols, 54 CALLS labels (49 positive, five negative), eight positive import labels and 15 impact questions. Source line/full-file hashes anchor every relationship and question.
- The original `repository` source projection is used: regular tracked code plus selected package/ignore configuration. This includes all eligible tracked source, not dependencies or a runnable installed checkout. The smaller IceCouncil truth-files diagnostic is not rerun. Export counts are retained; this is not whole installed repository acceptance.
- Serial baseline then candidate, one pair per project, one inherited CPU affinity, Rayon/OpenMP=1, no embeddings. Per command: 120 seconds wall, 120 sampled descendant CPU seconds, 4096 MiB sampled summed RSS. Sampling is every 50 ms and is not a cgroup hard cap. No page-cache flush or randomized/reverse-order repetition.
- Reviewer acknowledged the native hold before builds and measurements. Builder sampling at command boundaries is retained; it does not prove an otherwise idle host. No target apps, hooks, tests, services or installers ran. Target checkouts received only pinned Git-object reads.
- Bubblewrap fail-closed network/PID isolation, scrubbed environment, private HOME, registry, cache, temp and index; no live checkout, host home, credential or runtime socket mount. SCIP/Node/compiler tooling is not exposed in PATH. No installation or target dependency execution.
- Freshness/watchers, test-context, semantic search and actual LLM agent A/B are `NOT_RUN` in this campaign. The prior manual freshness and synthetic SCIP probe retain their original SHA labels; they do not validate this head.

## Measured graph quality

The current candidate recovers 68/68 symbols, 46/49 positive CALLS and 6/8 imports; 0 explicit negative CALLS labels were observed as false edges. 14/15 impact questions recover their complete labelled answer subsets.

The baseline recovers 68/68 symbols, 39/49 positive CALLS and 0/8 imports; 0 explicit negative CALLS labels were observed as false edges. 12/15 impact questions recover their complete labelled answer subsets.

All scores describe an open-world source-labelled subset. Only observed explicit negative labels count as false edges. Unadjudicated output remains unknown and overall precision stays `null`. Symbol discovery, relationship recall and engine-reported unresolved calls are separate.

| Project / lane | Exported files | Symbols | Positive CALLS | Labelled false CALLS | Unadjudicated CALLS | Imports |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| ice-council / baseline | 811 | 21/21 | 9/12 | 0 | 82 | 0/2 |
| ice-council / candidate | 811 | 21/21 | 9/12 | 0 | 90 | 2/2 |
| nightwire / baseline | 333 | 23/23 | 13/20 | 0 | 273 | 0/3 |
| nightwire / candidate | 333 | 23/23 | 20/20 | 0 | 364 | 3/3 |
| giglab / baseline | 275 | 24/24 | 17/17 | 0 | 153 | 0/3 |
| giglab / candidate | 275 | 24/24 | 17/17 | 0 | 167 | 1/3 |

The summary traces every labelled relationship to its source anchors and classifies its observed origin as correct, missing positive, incorrect relationship, or absent labelled negative. These classifications do not adjudicate unlabelled edges.

## Index command outcomes and timing

Elapsed numbers below include failed analysis attempts. They are descriptive durations, not successful full-index samples and not evidence of speedup. Baseline exit zero is `COMPLETE_COMMAND`; the old baseline lacks the candidate stage receipt, so compiler completeness is unknown.

| Project / lane | Cold wall / sampled CPU seconds / peak RSS MiB | Warm / changed / restored wall seconds | Cold result |
| --- | --- | --- | --- |
| ice-council / baseline | 56.40 / 54.91 / 592.6 | 5.98 / 16.84 / 17.44 | OK, exit 0 |
| ice-council / candidate | 51.15 / 49.76 / 569.0 | 7.03 / 16.86 / 15.78 | FAILED, exit 1 |
| nightwire / baseline | 23.48 / 22.50 / 478.9 | 5.33 / 10.64 / 10.26 | OK, exit 0 |
| nightwire / candidate | 22.23 / 21.31 / 505.0 | 5.46 / 10.87 / 12.51 | FAILED, exit 1 |
| giglab / baseline | 15.72 / 15.06 / 472.8 | 5.25 / 7.97 / 7.20 | OK, exit 0 |
| giglab / candidate | 15.33 / 14.57 / 471.6 | 5.24 / 7.39 / 8.29 | FAILED, exit 1 |

- `ice-council`: runner exit `1`, `INCOMPLETE_INDEX_PAIR`, `COMPLETED_WITH_FAILURES`; raw `ice-council-pair-01.json`, SHA-256 `252b885cfa392702925425afd467b1a36cf9fefaa4ed2f68a07d829cdd963ed9`.
- `nightwire`: runner exit `1`, `INCOMPLETE_INDEX_PAIR`, `COMPLETED_WITH_FAILURES`; raw `nightwire-pair-01.json`, SHA-256 `51ce78995b0a2df2ab2e3d4410e4fd9c5ba0de5d3259cc3e466f4fff6166a55e`.
- `giglab`: runner exit `1`, `INCOMPLETE_INDEX_PAIR`, `COMPLETED_WITH_FAILURES`; raw `giglab-pair-01.json`, SHA-256 `ffbdaac1ddfb031e5b4f4df242deec0a14faa3ed1d19e221b9adf18bf952363b`.

Candidate failed index commands stay failed. Where schema-v1 receipts establish successful syntax resolution and failed SCIP tooling, the runner collects `PARTIAL_SYNTAX_GRAPH` evidence. No compiler enrichment was run: `SCIP_TOOL_UNAVAILABLE` is not a successful compiler stage. Neither `--no-embed` nor AST query success turns it into one. Full compiler-enriched correctness, comparative indexing performance and release acceptance remain on hold.

## Origin and missing-relationship findings

Each import is checked by Module ID immediately after cold indexing and by Module.file after warm indexing. Independent endpoint queries distinguish an absent Module from an absent relationship. Total edges elsewhere in a graph do not validate those edges.

| Project / lane | Missing labelled Module endpoints | Total IMPORTS edges | Labelled imports matched by ID |
| --- | ---: | ---: | ---: |
| ice-council / baseline | 0 | 36 | 0/2 |
| ice-council / candidate | 0 | 3368 | 2/2 |
| nightwire / baseline | 0 | 7 | 0/3 |
| nightwire / candidate | 0 | 1257 | 3/3 |
| giglab / baseline | 0 | 0 | 0/3 |
| giglab / candidate | 0 | 645 | 1/3 |

Current candidate missing positive calls (source anchors retained in the unchanged manifests and JSON):

- `ice-council`: `src/equity/offline/groundingDataset.ts::json` → `src/utils/canonicalJson.ts::canonicalJson`; anchors `src/equity/offline/groundingDataset.ts:40`, `src/utils/canonicalJson.ts:5`.
- `ice-council`: `src/equity/offline/groundingDataset.ts::jsonl` → `src/equity/offline/groundingDataset.ts::json`; anchors `src/equity/offline/groundingDataset.ts:41`, `src/equity/offline/groundingDataset.ts:40`.
- `ice-council`: `src/cli/gptCatalystComparison.ts::parseGptCatalystComparisonArgs` → `src/cli/args.ts::parseOptionalArg`; anchors `src/cli/gptCatalystComparison.ts:16`, `src/cli/args.ts:10`, `src/cli/gptCatalystComparison.ts:20`.

Current candidate missing positive module dependencies:

- `giglab`: `packages/datasets/src/index.ts` → `packages/contracts/src/match.ts`; anchors `packages/datasets/src/index.ts:2`.
- `giglab`: `packages/datasets/src/index.ts` → `packages/controllers/src/prompt.ts`; anchors `packages/datasets/src/index.ts:3`.

The current [ModuleImports resolver](https://github.com/redxzeta/afterimage/blob/dc45de6e5769c1d86116167dcb1660d9561736c8/crates/infigraph-core/src/graph/imports.rs#L23) strips quotes and resolves relative JS/TS file IDs; [TypeScript extraction](https://github.com/redxzeta/afterimage/blob/dc45de6e5769c1d86116167dcb1660d9561736c8/crates/infigraph-languages/languages/typescript/relations.scm#L13) includes re-exports. Bare package specifiers intentionally abstain instead of guessing from basenames. GigLab’s two anchored workspace imports require package export-map metadata and remain missing despite both Module endpoints existing.

The [CALLS resolver](https://github.com/redxzeta/afterimage/blob/dc45de6e5769c1d86116167dcb1660d9561736c8/crates/infigraph-core/src/resolve/calls.rs#L352) still derives import stems from raw extraction targets independently of ModuleImports. That code and the call-expression capture rules are unchanged from the old candidate. This source inspection is consistent with the remaining import-binding/callback gaps; the module fix does not supply compiler binding. In IceCouncil, the positive callback origin is `jsonl` and its target is the same-file `json`; a module import edge cannot substitute for that CALLS relationship.

`optional-args` finds all four expected impacted callers via transitive routes, but two of its four labelled source chains remain unsupported because the direct comparison-parser → optional-parser edge is absent. `canonical-arrow-chain` misses `json` and `jsonl`. The explicit negative separating the test-local serializer from the utility serializer is checked separately. Unlabelled alternative bindings are not automatically declared correct or false.

## Fifteen fixed impact questions

Each CLI impact query ran three times; its median includes process startup, graph open and traversal. Each lane then used MCP initialize/tools-list and exactly five `query_graph` calls per project, manifest order, detail=true, no retries. MCP session duration includes handshake and startup; response bytes are not model tokens. This is a fixed tool protocol, not LLM agent A/B.

| Project / question | Baseline matched / labelled answers | Candidate matched / labelled answers | Candidate source chains supported |
| --- | ---: | ---: | --- |
| ice-council / canonical-arrow-chain | 1/3 | 1/3 | 1/3 |
| ice-council / optional-args | 4/4 | 4/4 | 2/4 |
| ice-council / required-args | 2/2 | 2/2 | 2/2 |
| ice-council / json-render | 1/1 | 1/1 | 1/1 |
| ice-council / duplicate-local | 1/1 | 1/1 | 1/1 |
| nightwire / quantity-arrow | 1/8 | 8/8 | 8/8 |
| nightwire / scale-transitive | 4/5 | 5/5 | 5/5 |
| nightwire / minimum-size | 3/3 | 3/3 | 3/3 |
| nightwire / sort-callback | 2/2 | 2/2 | 2/2 |
| nightwire / duplicate-local | 1/1 | 1/1 | 1/1 |
| giglab / source-digest | 1/1 | 1/1 | 1/1 |
| giglab / workspace-prompt | 3/3 | 3/3 | 3/3 |
| giglab / workspace-decoder | 5/5 | 5/5 | 5/5 |
| giglab / hash-arrow | 3/3 | 3/3 | 3/3 |
| giglab / config-validation | 1/1 | 1/1 | 1/1 |

All 30 scored MCP question calls agree with the corresponding CLI scores. Separate MCP smoke queries are outside those 30 scored calls. The harness stops its private persistent MCP server after all expected responses; `stopped_after_responses=true` distinguishes that intentional exit -9 from a crash or timeout.

Source-chain support follows each unchanged manifest chain in order. An endpoint reached through another route does not prove that labelled chain. Extra impact results remain unadjudicated; exhaustive blast-radius precision is not measured. Per-question missing IDs, matched source origins, latency and CLI/MCP agreement are in JSON.

## Minimal actual agent A/B follow-on (`NOT_RUN`)

No fixed-model adapter or isolated credential-free agent executor with measured usage receipts is configured here. The benchmarking agent has already inspected the answers, so answering the questions itself would not be an independent controlled A/B. No model costs, tokens, completion time or agent quality numbers are invented.

1. Use three predeclared prompts, one manifest question per pinned project (`canonical-arrow-chain`, `scale-transitive`, `workspace-decoder`), with expected answers hidden from both agents. Identical prompt template: "QUESTION. Identify affected callable IDs and the source call chains supporting them. Cite file:line evidence; distinguish missing evidence from verified absence. Read source only; do not execute project code." Substitute only the selected manifest question text.
2. Pin one exact provider model snapshot, reasoning setting, system prompt and source projection manifest. Start two clean independent sessions per prompt. Arm A receives current-head MCP graph tools plus read-only files; arm B receives standard file search/read tools over identical bytes. Neither sees truth manifests, this report, prior sessions or another arm. Prepare indexes outside task timing and disclose their partial AST status.
3. Fix a 20-tool-call / 10-minute task budget per arm; use AB/BA/AB task order without retries. Capture exact prompts, tool schemas, outputs, model ID, real provider usage receipts and wall time. Record setup/index time separately. Do not calculate money without an explicit price snapshot and actual billed usage.
4. Blind-score the frozen answers against the existing manifests: supported answer recall, explicit negative mistakes, correct source origin/chain and honest abstentions; adjudicate additional claims separately. Report each of the six outcomes and all failures. This tiny paired pilot supports only those tasks, not a general productivity claim.

## Acceptance and release hold

- Current-head evidence gap: closed for these pinned source projections and the labelled AST subset. The exact tested SHA is backed by new execution.
- Graph completeness: HOLD. Missing positive CALLS and workspace imports remain. Symbol discovery and zero observed labelled false edges do not establish exhaustive relationship correctness.
- Complete compiler enrichment and indexing performance: HOLD. Candidate SCIP stages failed, all pairs are incomplete, and no successful-pair speedup is established.
- Actual agent A/B and productivity/cost claims: HOLD / `NOT_RUN`. Only the fixed tool protocol was measured.
- Release, deployment and runtime activation: no authorization or readiness follows. No production action, CI workflow edit or implementation cherry-pick was performed.

## Reproduction and validation

Set `BASELINE_CLI`, `BASELINE_MCP`, `CANDIDATE_CLI`, `CANDIDATE_MCP` to binaries matching the summary hashes; set the three repository variables to authorized Git object stores. Use fresh output names:

```bash
BENCH_COMMON=(python3 scripts/benchmark/bench.py run
  --baseline "$BASELINE_CLI" --baseline-mcp "$BASELINE_MCP"
  --baseline-revision cf82f5d7440d9be158e219df1ef256d6359ed1f6
  --candidate "$CANDIDATE_CLI" --candidate-mcp "$CANDIDATE_MCP"
  --candidate-revision dc45de6e5769c1d86116167dcb1660d9561736c8
  --temp-root "$BENCH_ROOT/scratch" --no-embed --threads 1 --repeats 3
  --timeout 120 --cpu-seconds 120 --memory-mib 4096
  --tool-questions --fixture-scope repository)
mkdir -p "$BENCH_ROOT/scratch" "$BENCH_RUNS"
"${BENCH_COMMON[@]}" --repo "${ICE_REPO:?}" \
  --commit 10435a8c19f84549a0206f6b249ea13166aff6d0 \
  --truth scripts/benchmark/truth/ice-council.json \
  --output "$BENCH_RUNS/ice-council-pair-01.json"
# Observed exit 1
"${BENCH_COMMON[@]}" --repo "${NIGHTWIRE_REPO:?}" \
  --commit 80985e6fc508f309b17ba1889ff2ca15b0253c08 \
  --truth scripts/benchmark/truth/nightwire.json \
  --output "$BENCH_RUNS/nightwire-pair-01.json"
# Observed exit 1
"${BENCH_COMMON[@]}" --repo "${GIGLAB_REPO:?}" \
  --commit 02cee19056c8ee902289892727a6acbbcd30a7d3 \
  --truth scripts/benchmark/truth/giglab.json \
  --output "$BENCH_RUNS/giglab-pair-01.json"
# Observed exit 1
```

Local validation: all 19 existing harness tests and all three pinned source manifests passed. Python source compilation and `git diff --check` passed. The verifier selector tests (15) and diagnose orchestration tests (5) also passed with synthetic inputs. `scripts/verify.py --dry-run` selected benchmark checks; the full local native verifier was not rerun to avoid duplicating native work. No product Rust tests or full native verifier campaign is claimed by this report-only change; the exact candidate CLI/MCP build passed. Hosted PR CI, merge, release and runtime evidence remain separate.

Self-review covers exact source/binary pins, unchanged old reports/manifests, real failed exits, open-world scores, source origin chains, serial ownership, and exclusion of raw names/logs/source excerpts/private machine paths from publication. Sable’s original-commit cherry-pick lane is separate; this report neither approves nor merges its PR.

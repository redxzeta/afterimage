# M3 integrated product and optional-use handoff

**Product integration:** #16, #18, #20 and #19 are merged. Actual `afterimage-main` product head `5260ebc206294f5c55ca56e1be649a57bdd98f0c` has tree `abcdd0ef46b3a2806d6beede5e3ca8beaad71f94`, identical to the private M3 test candidate `d9b1804e13e85a7bc73ea8b394cee7fe0794d899`. Scoped checks and limitations are recorded below; adoption here means product source merged, not installation, release or runtime activation.

## M3 exact-tree smoke — HOLD on indexed read-only contract

The merged product tree was built offline/locked with Rust/Cargo 1.98.1, no default features, one Cargo/CMake job, two CPUs, Rayon 2, 180-second wall and sampled 4 GiB resident-memory limits; no virtual-address cap or aggregate CPU-seconds cap. CLI/MCP build exited 0 in 34.017s. CLI SHA-256 `c5f8d3532095121ecaf6ee74a8fba5fe1c47a439d6f6c879d2226eb8a74502f9`; MCP SHA-256 `17df4d0846d24349542f6dd0bb1045908684c1bc735aa8248e570013b5e515d8`. Cargo.lock SHA-256 `f1c33ba66829fb179a1ceb48d3c2036776812953e6d8b6cc1d0a277c45689eca`. These are M3 binaries; the older M2 client is pinned to a different binary.

| Scoped check | M3 outcome |
| --- | --- |
| Hardening cache/store refusals | 9 passed; exit 0 |
| TypeScript callers, including stale receipt and imported helper | 10 passed; exit 0 |
| Workspace import resolution | 1 passed; exit 0 |
| Linked-worktree scope; ten-tool discovery and manual denial | Each 1 passed; exit 0 |
| Disposable stdio/HTTP smoke | Exit 0; discovery, raw/mutation denial, HTTP 403 and fixture bytes unchanged |
| Two-file isolated AST-only | Exit 0; compiler stage `NOT_REQUESTED` |
| Explicit enrichment with missing indexer | Exit 1; compiler stage `FAILED/SCIP_TOOL_UNAVAILABLE` |
| Indexed warnings and byte-preservation contract | **UNRESOLVED:** both bounded invocations hit the 180-second limit, including the already-built test; no pass |

The first timed-out invocation spent 25.35 seconds compiling; the separate already-built functional run also timed out after 180.116 seconds, with sampled peak tree RSS 657868 KiB. All ten in-process tool markers were reached; remaining assertions did not finish. The previous M2 pass on another tree does not clear this result. No source defect or runtime safety outcome is inferred from a timeout. External SCIP was not run. No new three-repository benchmark or agent trial was conducted.

Optional local handoff: in a disposable source copy with an isolated HOME, use the tested executable to build a syntax-only index and start the read-only stdio profile only when explicitly needed:

```sh
cd "$DISPOSABLE_PROJECT"
HOME="$ISOLATED_HOME" INFIGRAPH_NO_WATCH=1 "$INFIGRAPH_CLI" index --ast-only
HOME="$ISOLATED_HOME" INFIGRAPH_MCP_PROFILE=read-only "$INFIGRAPH_MCP" --mcp
```

Stop the foreground process to disable it and remove only the disposable project and isolated HOME when finished. Until the indexed contract is resolved, treat this as experimental optional use and verify answers with ordinary source reads such as `rg -n 'symbol' "$DISPOSABLE_PROJECT"`. Do not infer graph completeness, deploy or install a service.

The following M2 benchmark and pilot are historical evidence under their original tested pins; merging product source does not relabel them.

# M2 agent-ready pilot: historical results

**Targeted combined functional checks are accepted within their scope; six pilot sessions completed and blind content ratings are frozen.** The completed post-mapping audit found one GigLab session invalid for exceeding the 20-call budget and five budget-pass sessions audit-limited. Isolation is not certified and token usage is unknown. No merge or adoption occurred.

Practical recommendation: the read-only client may be tried as an optional local investigation aid with source verification; this small, incompletely audited pilot does not establish an advantage over ordinary file search.

## Frozen blind ratings and pilot audit

An independent Sol/high reviewer froze the content ratings before receiving the arm mapping or activity records. The unchanged rating file SHA-256 is `0ffa932024c7f331cb3a55d7e44925115ae9e413abe1840f6f2795d5d62d4c58`. Ratings count supported expected callable/dependency chains in the frozen labelled subset; intermediates named in a chain count. They are source-backed content scores, not proof of session validity or exhaustive precision. The reviewer found no confirmed material false claim in the checked set; other claims were not exhaustively adjudicated.

| Packet | Projection | Arm | Supported expected chains | Recorded command executions | Run elapsed seconds | Validity |
| --- | --- | --- | ---: | ---: | ---: | --- |
| P2 | IceCouncil | A | 3/3 | 13 | 68.610 | Budget PASS; AUDIT_LIMITED |
| P6 | IceCouncil | B | 3/3 | 11 | 42.309 | Budget PASS; AUDIT_LIMITED |
| P4 | Nightwire | B | 5/5 | 9 | 37.947 | Budget PASS; AUDIT_LIMITED |
| P7 | Nightwire | A | 5/5 | 4 | 32.447 | Budget PASS; AUDIT_LIMITED |
| P1 | GigLab | A | 3/5 | 21 | 79.683 | INVALID: over 20 calls |
| P9 | GigLab | B | 4/5 | 19 | 66.261 | Budget PASS; AUDIT_LIMITED |

Arm A used the shell-mediated read-only client; arm B used ordinary source reads. All six completed on `codex_aux` / `gpt-6.1-terra`; reasoning `medium` is recorded in the runner task receipt, but the rendered activity does not expose effort. Run elapsed is T3 start to completion, with index/setup separate. Provider token usage and cost are UNKNOWN. The P1 participant reported the 21-call violation, and it is invalid without replacement; its blind content score remains visible but cannot support an arm comparison. The other five passed the observed call and wall budgets but are AUDIT_LIMITED: T3's rendered command activity omits working-directory arguments, no pre-session index-byte hashes were captured, and client HOME was shared (empty after the sessions). Source inventories matched before and after. The independent audit found no forbidden target execution, truth/global-server access or secret command in rendered command strings. No assertion of unchanged index bytes or fully isolated sessions is justified.

The P2 and P1 answers said their client returned no usable output, although the private activity contains JSON responses; P7 received one non-error impact JSON response; its next invocation used literal `TOOL` as the tool name and failed the allowed-tool assertion. The rendered invocation does not show a forbidden `path` key; the answer's “no usable result” is a subjective assessment, not an empty transport. These are recorded response/answer discrepancies, not fabricated tool outcomes. The reviewer credited both IceCouncil callback chains as callback-mediated. No speedup, general productivity, adoption, or pilot-usefulness conclusion follows from these six small and incompletely audited sessions. The independent post-mapping audit is complete; the five budget-pass sessions remain audit-limited.

## 2026-10-28 continuation: functional checks and pilot

The former #19 E0282 blocker was corrected at [head `9ec80e8a27ddc969ae47d6114eaa7c0091c2e1c0`](https://github.com/redxzeta/afterimage/pull/19); its sole change from `93881dd9` is a test-only type annotation. Private combined commit `6fd340588e5fe93080885d389b6b4daf8fefac53` has tree `f58844b938171cf41317c15e947a47085355b00e`; product bytes and lockfile match the previously measured tree. Rust/Cargo 1.98.1, offline/locked, no default features: the caller target compiled (exit 0, 11.174s); the stale-receipt test passed (exit 0, 1 test, 5.683s); focused imported-caller and workspace-import tests each passed (exit 0, 6.872s and 14.424s). [#19 receipt](https://github.com/redxzeta/afterimage/pull/19#issuecomment-6470971988).

The previously unresolved indexed read-only warning and byte-preservation test passed unchanged with two CPUs and Rayon threads, 180s wall and 4 GiB sampled RSS, no virtual address or separate CPU-seconds cap: exit 0, one passed, 165.558s and 645856 KiB sampled peak tree RSS. Test executable SHA-256 `5bdd59791228d184f6a1b713faba1f8044926fafdbd2b8427de6a40bcc765f17`; its test-profile MCP binary SHA-256 `0d46080d742787977799f2ee7a46f527d341458cb058f9cff969b405cf58020b`. The earlier 8 GiB virtual-address failure, 180s timeout and 150 CPU-second termination remain historical. Diagnostic markers showed writer closure and all ten in-process tools completing; a pipe-waiting parent, waiting supervisor and runnable worker were observed before the budget termination. This is a functional calibration, not a proven root cause or speed claim. [#20 receipt](https://github.com/redxzeta/afterimage/pull/20#issuecomment-6471268431).

A single unscored setup probe launched the candidate dev MCP binary (`1a688148e32a71876adaa1c9ad393240b0ed9b1331ea3fa9ec432d2495e6073c`) with the read-only environment and a disposable indexed fixture. The launcher's process executable matched that binary; initialization, exact ten-tool discovery and `get_stats` succeeded, showing failed compiler coverage and freshness warnings. The finite shell-mediated client is [m2-pilot-client.py](../m2-pilot-client.py), SHA-256 `cc978e62fc6212f6d49912479973c17c212b19063d654f6a5759b835884f6bf3`; it accepts only explicit prepared roots and the ten actions, checks the binary hash, and starts no service. To reproduce the same setup in a private disposable directory, place that exact script at the directory root, the pinned binary at `bin/infigraph-mcp`, and an already prepared fixture index at `smoke/repo`, then run:

```sh
python3 m2-pilot-client.py "$PWD/smoke/repo" get_stats '{}'
```

Stop the foreground process if necessary and remove only the disposable directory and its contents. Do not install or register a global server. This shell-mediated trial does not establish native MCP tool parity.

Six independent sessions were dispatched and completed under `codex_aux`, `gpt-6.1-terra`; the runner task receipts specify reasoning `medium`, with the fixed three questions, source pins and 20-call/600-second limits. The two arms had ordinary source read/search; the candidate-client arm could also invoke the read-only client. The frozen source inventories matched between arms. One session exceeded the budget at 21 calls and is **INVALID**, without replacement. Neutral answer packets were frozen and independently scored before mapping; see the newer section above. The post-mapping audit is complete; five budget-pass sessions remain **AUDIT_LIMITED**. Post-session source inventories still match. The shared client HOME was empty after the sessions, but per-session isolation and unchanged index bytes cannot be established from the captured setup: index hashes were not frozen before dispatch, and the T3 activity rendering does not expose each shell call’s working-directory argument. Those limitations prevent certification of isolation. Provider token usage is **UNKNOWN**. No productivity or speedup conclusion is available. Earlier no-session and failed receipts below remain historical.

## Historical appendix: Sable setup attempt

This appendix records the earlier Sable setup attempt at its original pins. Its zero sessions and blocked build are historical; see the current results above.

The three source lanes were composed locally, but the combined candidate has not
been built or tested. The practical A/B and acceptance are **NOT_RUN**; no GO
recommendation can be made. The host has no `cargo` on PATH or in the checked
standard locations. A single bounded offline build attempt exited 127 immediately
(`cargo: command not found`). No installation or repeated build was attempted.

### Historical inputs and composition

Refreshed default `afterimage-main`: `dc45de6e5769c1d86116167dcb1660d9561736c8`.
PR heads: [#18](https://github.com/redxzeta/afterimage/pull/18)
`fef14d9ca7a2d38f9861be5bb7e20c1148150438`,
[#19](https://github.com/redxzeta/afterimage/pull/19)
`93881dd9c94b08052958b525568d49ec3885eecb`,
[#20](https://github.com/redxzeta/afterimage/pull/20)
`3ec1ea3d813e89dfd2a47d68d230701006e09f94`.
The candidate is a disposable local clone at commit
`85f48fd8cc79cc6936060d0759b97109513a5d0b`, tree
`5afe1b6db0fd1ded716c4102af6b98d87a455156`. Neither binary exists; hashes
are **NOT_AVAILABLE**. Local commit identities depend on cherry-pick metadata;
the tree is the reproducible content identity.

From a clone with those objects, check out the base detached and cherry-pick in
order the seven full commit IDs in the accompanying manifest. Disable local hooks
for these test-only operations. `c046613d` conflicted only at the end of
`crates/infigraph-core/tests/typescript_callers.rs`: both branches independently
appended a test. Preserve the complete #18 tests, then append #19's
`direct_index_file_change_invalidates_success_and_obsolete_worker` with its own
`#[test]`; continue the sequence. `git diff --check` passed and the candidate
worktree was clean. No product commits were added to this report branch.

The active process snapshot on this host found no active Sable or Afterimage
build. #18's author reported the Debian native slot released, 9 caller, 8 import,
16 resolver and 4 module resolver passes at code head `2d6d403`, with only docs
changed in the final commit; its prior hosted CI failed on missing liblbug.
These are lane receipts, not candidate results. [#19 source and narrow local review was approved](https://github.com/redxzeta/afterimage/pull/19#pullrequestreview-5678081785), with its core offline dependency block and external SCIP NOT_RUN. [#20 source and focused author-local review was approved](https://github.com/redxzeta/afterimage/pull/20#pullrequestreview-5678081689). These are not combined-candidate results; raw SCIP importer transaction and idempotence remain deferred.

### Historical evidence and remaining checks

The existing #17 truth verifier passed against each exact Git pin: IceCouncil
32 anchors/21 symbols/14 relations; Nightwire 39/23/21; GigLab 45/24/19. This
verifies source anchors, not projection inventories or candidate graph output.
The baseline #17 measurement of 46/49 positive call labels belongs to product
`dc45de6e`, not this candidate. The frozen 49 includes the `jsonl` to `json`
callback dependency; a passed callback is not automatically a direct CALLS edge.
Unadjudicated edges are neither false positives nor true negatives.

Combined syntax-only/enrichment, linked-worktree, denied mutation/raw query,
HTTP, byte-preservation, visible-warning, imported-call/workspace and single-file
receipt smoke: **NOT_RUN**. Labelled candidate AST check: **NOT_RUN**. Projection
inventory verification: **NOT_RUN**. No target application, hook or test ran.

| Frozen task | Arm A | Arm B |
| --- | --- | --- |
| canonical-arrow-chain | NOT_RUN | NOT_RUN |
| scale-transitive | NOT_RUN | NOT_RUN |
| workspace-decoder | NOT_RUN | NOT_RUN |

Zero sessions were dispatched: a candidate binary and prepared read-only index
are prerequisites to comparing the arms. T3 capabilities listed provider `codex`,
model `gpt-6.1-terra` with `reasoningEffort=medium`; no returned session model,
usage, timing, transcript or answer exists. Sol adjudication is **NOT_RUN**.
Run the existing frozen protocol with the user's model override only after the
combined smoke and projection inventory pass; keep six independent sessions,
20-call/10-minute limits and no replacement for invalid runs. No general
productivity or model-ranking conclusion follows from three pairs.

For an opt-in focused test after composing the candidate in a properly provisioned
local Rust/native environment, run:

```sh
CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 cargo test --offline --locked -p infigraph-core --test typescript_callers --test typescript_imports -- --test-threads=1
```

This is one part of the required smoke, not a complete acceptance command.
Read `docs/agents/indexing.md` and `docs/READ_ONLY_MCP.md` in the candidate for
explicit local index/profile setup. To disable the pilot, stop its explicitly
started process and remove only the disposable clone, projections and indexes;
no global registration, daemon, deployment or production index was created.
Default-branch adoption, merges, releases and runtime activation: **NOT DONE**.


## Historical appendix: first Debian combined tree



The local test-only merge commit `586e53c907dd45df61ebcc3c8bb60962947ee94d` has tree `5afe1b6db0fd1ded716c4102af6b98d87a455156`, identical to Sable's tree. The heads above were refreshed and unchanged. This composition excluded #16 and preserved both appended tests. It was not pushed as product source. Source review #18 and #20 remains approved; **#19 is HOLD**, superseding the earlier scoped approval: its appended test fails compilation with E0282 because the panic-only `update_current` closure has an unconstrained return type. The combined caller/import test command exited 101 and ran zero tests. See the [#19 review](https://github.com/redxzeta/afterimage/pull/19).

A single offline, locked, no-default-features Cargo build used Rust/Cargo 1.98.1, one Cargo/CMake job, dev debug=0 and the existing cache. Cargo logged `Finished` in 1m 16s, and produced the binaries below. The measurement wrapper then failed reading its lockfile from the wrong directory; consequently **Cargo exit and sampled resource receipt are UNKNOWN**, not a clean build receipt. No rebuild was performed. The cached lbug 0.19.1 native archive SHA-256 was `9100591f7cf1de0f565c96b55e3293998763260fc936bfbb9e304bef6f4ec2b8`; wrapper archive SHA-256 was `25b950992a082045e77078bdbaf113f48cd8d552ad1da2ee1deab8f1f0abf9d6`. These are cached-byte identities, not upstream release provenance. Cargo.lock SHA-256 was `f1c33ba66829fb179a1ceb48d3c2036776812953e6d8b6cc1d0a277c45689eca`.

| Binary | SHA-256 |
| --- | --- |
| CLI | `02b420d510f3be066fe227ee5a0107bb8d815ca66a9f5832651a4a81a0fb1955` |
| MCP | `1a688148e32a71876adaa1c9ad393240b0ed9b1331ea3fa9ec432d2495e6073c` |

On disposable fixtures, the actual CLI AST-only command exited 0 with `NOT_REQUESTED`; requested enriched mode exited 1 with `FAILED/SCIP_TOOL_UNAVAILABLE`. Actual MCP stdio and loopback HTTP smoke passed discovery of ten tools, manual raw and mutation denial, both HTTP 403 checks, thin-client metadata removal and fixture-byte preservation. The read-only Rust suite had four passes and one failure because an 8 GiB virtual-address cap prevented Kuzu's 8 TiB sparse mapping. A targeted recheck with a sampled 4 GiB process-tree RSS cap timed out after 180 seconds (peak 585088 KiB); warning and byte preservation on an indexed graph therefore remain **unresolved**. Single-file receipt and combined consumer/import unit tests are **NOT_RUN**; the E0282 compile failure prevented the requested core test target from running. External SCIP remains **NOT_RUN**.

All three existing truth verifications passed. Tracked source/config projections reproduced the protocol's inventory hashes (811, 333 and 275 files respectively); targets were not executed. The existing harness ran one AST index and one labelled query set per projection, serially, in isolated Bubblewrap lanes with one CPU, 120 seconds and 4096 MiB sampled RSS per command. Setup/index timing is descriptive, not a speed comparison.

| Projection | Index seconds | Symbols | Frozen relationship labels | Imports | Complete impact subsets |
| --- | ---: | ---: | ---: | ---: | ---: |
| IceCouncil | 88.8 | 21/21 | 11/12 | 2/2 | 4/5 |
| Nightwire | 38.8 | 23/23 | 20/20 | 3/3 | 5/5 |
| GigLab | 28.2 | 24/24 | 17/17 | 3/3 | 5/5 |
| Total | — | 68/68 | 48/49 | 8/8 | 14/15 |

No labelled false call or import edges were observed; unlabelled edges are not adjudicated. IceCouncil's one missing frozen relationship is `src/equity/offline/groundingDataset.ts::jsonl` to `::json`, a source-backed callback dependency rather than a direct CALLS edge. Both previously missing direct imported calls and both GigLab workspace imports were observed. Compiler coverage remains partial and empty graph results do not prove absence. Earlier #17 measurements retain their original `dc45de6e` labels.

**Pilot: NOT_RUN, 0/6 sessions.** The available T3 child interface exposes model/provider settings but no per-child MCP server configuration. The existing globally connected Infigraph is not established to be this candidate binary with its ten-tool read-only profile and isolated index. Offering it would invalidate arm A; no baseline-only sessions, substitutions, invented usage or blind scores were produced. A follow-on needs a supported per-session MCP binding to the exact candidate and prepared isolated index, then the unchanged six-session protocol and blind adjudication. Token usage and latency are UNKNOWN. Integration acceptance, pilot usefulness, merge, adoption and runtime activation remain **NOT_DONE**.

For a local opt-in after independently obtaining a tested candidate and disposable source projection, the following is a single manual sequence; provide private paths in the shell, use no live checkout, and retain the generated receipt:

```sh
cd "$DISPOSABLE_SOURCE"
"$CANDIDATE_CLI" index --ast-only
INFIGRAPH_MCP_PROFILE=read-only "$CANDIDATE_MCP" --mcp
```

Stop that foreground process to disable it, then remove only the explicitly created disposable source, index and private session files. Do not register a global service or hook. This illustrative sequence does not substitute for the isolated benchmark sandbox or six-session protocol.

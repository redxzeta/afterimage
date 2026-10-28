# M2 agent-ready pilot: HOLD

The next section is the earlier Sable setup receipt. The Debian continuation below supersedes its statements about the availability of a binary and tests, but does not erase the original exit 127.

The three source lanes were composed locally, but the combined candidate has not
been built or tested. The practical A/B and acceptance are **NOT_RUN**; no GO
recommendation can be made. The host has no `cargo` on PATH or in the checked
standard locations. A single bounded offline build attempt exited 127 immediately
(`cargo: command not found`). No installation or repeated build was attempted.

## Inputs and composition

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

## Evidence and remaining checks

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

## Debian continuation: combined tree and bounded checks

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

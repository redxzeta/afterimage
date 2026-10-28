# M2 agent-ready pilot: HOLD

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

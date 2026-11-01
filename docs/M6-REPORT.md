# M6: freshness, explicit refresh, and JSX references

M6 state: **partial**. Implementation and local evidence are delivered; the
controlled native-agent comparison is **INVALID / n=0**, not a measured ROI win.
Recommendation: **KEEP_OPTIONAL**, with no expansion or automatic rollout.

## Frozen identities and scope

Afterimage base: `e59380d3c7cff5f6e7a28c85b0164ec6451a5c62`, branch
`afterimage-main`, repository `redxzeta/afterimage`. Implementation candidate:
`553140f4da37480c487e7edcfcc0c0aef8e8d0de`. T3Code source was clean at
`07f00e9b1c27f411c80359c8525f7f835d01cbc0` in `redxzeta/t3code`.
Only Afterimage source was changed. Existing T3Code graph/configuration and
unrelated branches/worktrees were preserved. No installed M6 binary or graph was
activated, and no production-consumption claim is made.

The installed project configuration identified a versioned PR27 MCP executable;
there were no attached MCP processes or active T3Code threads at preflight.
The legacy graph has no authoritative checkout-revision manifest: its indexed
revision remains **UNKNOWN**. Historical pilot timings and issue trials are not
M6 measurements.

## Delivered behavior

See [freshness/refresh contract](M6-FRESHNESS-REFRESH.md) and
[read-only MCP contract](READ_ONLY_MCP.md). The existing graph store, language
registry, ignore rules, receipt and advisory-lock mechanisms are reused.

* Explicit `freshness` compares selected supported source content/file sets,
  including relevant untracked files, with graph hashes and a ready manifest.
  HEAD equality alone never establishes freshness.
* Explicit `refresh` builds separately, validates, atomically selects a generation
  for new read-only MCP requests, and retains old readers/graphs for rollback.
  Failed builds stay `NOT_READY`. Legacy CLI graph readers retain their old scope.
* `get_stats` and `diagnose` verify content; ordinary queries return
  `UNKNOWN / SOURCE_NOT_CHECKED` rather than reuse an unsafe freshness cache.
* `JSX_USES` records exact runtime named-import/local component value references.
  `find_all_references` is available in the eleven-tool read-only profile and
  labels relationship kinds and owner-declaration locations. `trace_callers`
  remains limited to `CALLS`. Extractor fingerprint version 5 invalidates old
  extraction results on ordinary explicit reindexing.

No object-call heuristic was added for the concrete `outbox`/`gitCore` examples:
their Effect service receivers do not establish implementation identity through
lexical syntax. Existing proven namespace/class resolution is retained. Default
and member JSX tags, unsupported re-exports, and imported JSX in the optional
Neo4j backend deliberately abstain.

## Validation and review

Rust **1.98.1** was installed only in the private task tooling cache, without
changing system/toolchain defaults. All **24 focused tests passed**: Effect 3,
JSX 3, snapshots 4, TS members 2, read-only MCP 10, CLI refresh 2. The member
suite passed again with computed/dynamic property negatives. Workspace formatting
passed. The initial Rust 1.99.1 verifier refusal is superseded by the final
exact-toolchain verification receipt recorded in the PR.

Coverage includes aliases, type-only imports, local/hoisted shadows, intrinsics,
unknown and computed receivers, `Math.max/min`, `Effect.exit`, Effect factories,
real content-changing consumer/dependency reindexing, export removal, legacy
table compatibility, separate-storage imported calls/JSX, old-reader consistency,
wrong checkout/version, incomplete state, lock contention, timeout/retry,
rollback, truncated snapshots, preserved legacy graph bytes, and mutation denial.

Independent read-only review approved the cumulative implementation candidate
after fixing source-root propagation, duplicate local references, parser-stack
setup, provenance fallback, request-generation pinning and pre-open validation.
No dependencies, CI/release workflows, services, schedules, global MCP settings,
or T3Code tracked files were changed. Optional remote runtime remains untested.

## Controlled comparison: invalid, no scored results

[Preregistration](M6-PREREGISTRATION.json) froze five real investigation prompts,
one novel favicon holdout, scoring/false-edge penalties, counterbalanced order,
Sol/High, per-task bounds and recommendation thresholds before scored responses.
No scored responses were run. A fresh native child discovered **zero Afterimage
tools**. Attempts to target the existing T3Code project through this parent were
rejected: `A local task runs in this thread's project.` The environment-qualified
attempt received the same rejection. No new top-level thread, development
worktree, alternate provider or CLI-agent substitute was created.

Therefore controlled n=0; source-only/optional scores, paired latencies, shell
read counts, tokens and costs are **NOT_RUN / unavailable**. The parent identity
was verified as Codex GPT-6.1 Sol/High; the probe child did not independently expose
its model settings. This cannot establish matched-context or independent timing
evidence. Shell protocol checks below do not repair that validity failure.

## Bounded real-source fallback

[Sanitized exact protocol evidence](M6-FALLBACK-EVIDENCE.json) records tool
arguments/returns, per-request times, binary hashes, source hashes and setup cost.
Eleven files were byte-checked against Git blobs at the exact T3Code SHA and
exported to a temporary fixture. This is an **AST-only sample**, not a complete
T3Code index or a second agent experiment. No target application was run.

Refresh took **11.082 s**, with reported maximum child RSS **335,260 KiB**;
this is not simultaneous process-family RSS or a full-checkout estimate. A second
refresh took **3.431 s**, returned `NO_OP`, and preserved the selection pointer.
The graph contained 11 modules/files, 1,722 symbols and 639 CALLS edges.
Index resolution reported 915 resolved and 3,382 unresolved relations; these
counters are not unique graph-edge counts or an accuracy denominator.

| Probe | Exact observed result | Request time |
|---|---|---:|
| `get_stats` | `fresh`, `CONTENT_AND_FILE_SET_MATCH` | 3.198 s |
| Component references | One `ChatComposer → ComposerCommandMenu` `JSX_USES` | 0.128 s |
| Component callers | No CALLS callers in this graph | 0.121 s |
| `statusDetailsRemote` | Function in `GitVcsDriverCore.ts:2345` | 0.127 s |
| `WorktreeMcpService` | Class in `WorktreeMcpService.ts:27` | 0.126 s |
| `ProviderRuntimeRecoveryService` | Class in source at line 69 | 0.130 s |
| `ProjectFavicon` | No results despite source declarations | 0.133 s |
| `claimNext` | No results despite interface/object-property source | 0.131 s |

The actual JSX use is in `ChatComposer.tsx:3298`; graph output correctly labels
line 506 as the owner's declaration. `claimNext` appears in the EffectOutbox
interface at line 224 and its object-property implementation at line 521. Favicon
factory functions exist at lines 86/117: the partial search miss is not proof of
absence. No consequential fabricated edge was observed in these probes; this is
not a general false-positive rate.

The smoke exposed eleven read-only tools, denied `index`, rejected a missing
checkout, and preserved all fixture source/graph bytes. Ten tool calls plus
initialize/discovery took **4.130 s** total. Native agent query count was zero.

## Decision and remaining work

**KEEP_OPTIONAL**: the exact JSX example and explicit refresh remove concrete
friction, while cheap static navigation can help. There is no evidence here to
claim agent speed gains or expand mandatory graph usage. Expansion requires a
valid fresh-context native comparison with no score loss/false consequential
edges and either higher supported accuracy or at least 15% lower median paired
latency; five tasks would still be provisional evidence.

Remaining work: establish native MCP availability in independent benchmark
contexts, then execute the frozen protocol on a fresh immutable full target;
investigate object-property extraction/search misses only against concrete
fixtures. No benchmark follow-up, watcher, automation or scheduled status turn
was installed.

Source verification remains synchronous and outside the builder timeout; there
is no hard total-RSS cap. Retained generations consume disk and require explicit
removal after readers close. Explicit manual refresh/rollback and this retention
are the only new operational responsibilities, required by this milestone.

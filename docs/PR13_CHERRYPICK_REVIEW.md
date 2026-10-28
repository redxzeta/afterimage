# Independent review: attributed PR #13 replacement

Review date: 2026-10-26 (America/Los_Angeles).

Policy update, 2026-10-27: the source/provenance audit below is complete.
Nathan subsequently authorized execution and accepted bypassing hosted CI.
The connector/tool refused a merge action, so merges and alternate routes remain
unexecuted at that platform boundary; this is not withheld owner permission. The
historical failed run below is preserved; it is not a new V1 hosted-CI gate.
Current scoped acceptance, pilot and runbook live in [AGENTIC_V1.md](AGENTIC_V1.md).
No further #16 rebuild/review campaign is required by that milestone.

**Disposition: HOLD / replacement CI failed.** Independent source review of
[PR #16](https://github.com/redxzeta/afterimage/pull/16) at
`662b24515fd94e5f0cd28fc2e63bdb95ce85263a` found no concrete blocking product
defect, but its overall hosted check is red. The passing integration results
below do not establish a complete focused-CI pass or implementation approval.
The reviewer may coordinate an expected-head guarded merge only after the gates
below pass. Old draft [PR #13](https://github.com/redxzeta/afterimage/pull/13)
must remain open until the replacement is independently validated and merged.

## Pinned evidence

- Fork review base: `dc45de6e5769c1d86116167dcb1660d9561736c8`,
  `redxzeta/afterimage:afterimage-main`, verified against GitHub.
- Source: [pradeepmouli/infigraph:feat/hardening](https://github.com/pradeepmouli/infigraph/tree/d1b8bc0edc1e4645d11e51c3370ad27e10c0591c),
  pinned at `d1b8bc0edc1e4645d11e51c3370ad27e10c0591c`.
- Old PR #13 head: `f322556403e4f51448cd16233fff303a142f02c8`.
  Its three commits are authored by redxzeta; its implementation is
  `05b40a8978273a59fbb7475a93e6034ccef3a4c8`. It is an adaptation, not an
  original-author cherry-pick series.
- Method: `git show --format=fuller` for all five nominated source commits,
  their implementation/test diffs, source/base tree comparison, old PR's full
  code/test diff, and current verification/workflow source. No source-fork
  executable, installer, hook, daemon, or application test was executed.
- Independent branch: `review/afterimage-source-cherrypicks-20261026`.
  This report owns only this file; implementation and benchmark paths have
  separate owners. The benchmark owner explicitly released the Debian native
  lane after its dc45de6e work. Reviewer native builds/tests: **NOT_RUN**;
  exact-head hosted logs were used instead of duplicating native builds.

The complete source branch has 1,095 commits absent from the review base and a
634-file tree diff (185,455 insertions, 29,708 deletions). Those counts describe
the pinned comparison, not the proposed scope. A whole-branch merge would pull
in unrelated daemon/socket, recovery, settings, format, and installation work.

## Source provenance and dependency matrix

Author identities below are Git object metadata, not an inference from the
repository owner. Preserve original author names, emails, author dates, commit
messages, and coauthor trailers on actual picks; a different committer is normal.

| Original source commit | Verified author | Independent disposition and dependencies |
| --- | --- | --- |
| [`fc40d09027b9bbe761bb4b2ddab462abd93bdcce`](https://github.com/pradeepmouli/infigraph/commit/fc40d09027b9bbe761bb4b2ddab462abd93bdcce) | Pradeep Mouli, `pmouli@mac.com` | **Skip as already covered.** This exact SHA is not an ancestor of the fork base, but the base contains Pradeep's corresponding `0080982f8e3fcce9acc344b79a0d0d10da0063f5` and the lock refusal/tests. Full patch IDs differ because of surrounding build-hash/context changes; do not claim exact SHA identity or manufacture an empty pick. Preserve the existing initial and retry lock guards. |
| [`e4257a123b8ae74c0ab838098e50f74862620702`](https://github.com/pradeepmouli/infigraph/commit/e4257a123b8ae74c0ab838098e50f74862620702) | GitHub Copilot, `copilot@github.com` | **Pick with bounded conflict resolution.** Refuse storage-version mismatches before recovery and when first seen during backoff. Source assumes lock-context helpers, quarantine, and a richer recovery stack absent here. Keep current recovery structure; do not import that stack to satisfy context. Retain native byte-preservation regression behavior and both-version diagnostics. Source design/plan documentation must not imply absent daemon or quarantine features exist in this fork. |
| [`de33d9cfb2809fe0a34f4375bb80a46e515a1253`](https://github.com/pradeepmouli/infigraph/commit/de33d9cfb2809fe0a34f4375bb80a46e515a1253) | GitHub Copilot, `copilot@github.com` | **Pick after e4257a1.** Share refusal classification across code init, docs init, combined code open, combined docs query and search. Source references `lock_probe`, `ProbeFor`, transient-WAL helpers, WAL guards, schema validation and quarantine that this base lacks. Resolve those references explicitly; do not add those systems. Source `wal_guard.rs` is absent here, so adapt its added storage-version test without importing unrelated WAL tests. |
| [`7908e299a3caf7796d4c809634b4db663f8cc2c8`](https://github.com/pradeepmouli/infigraph/commit/7908e299a3caf7796d4c809634b4db663f8cc2c8) | GitHub Copilot, `copilot@github.com` | **Pick, but insufficient alone.** Bounds corrupt embedding/HNSW/BM25 counts before reservation. Base already bounds the headerless embedding count. Source embeddings return triples with hashes; fork returns pairs. Preserve the fork format/API. This commit still leaves raw BM25 length slices and unchecked posting indices; stopping here loses old #13 protections. |
| [`106184a396b517f12c09f3c1974f57cd136d80c8`](https://github.com/pradeepmouli/infigraph/commit/106184a396b517f12c09f3c1974f57cd136d80c8) | GitHub Copilot, `copilot@github.com` | **Include its bounded-reader protections after 7908e29, or retain an explicitly reviewed equivalent.** `ByteReader` itself depends only on existing anyhow. It supplies checked reads, checked dimension multiplication, count bounds and posting-index validation missing above. Adapt only the current headerless embedding reader and HNSW/BM25 loaders. Fork docs BM25 has no persisted save/load path; omit that inapplicable hunk with a receipt rather than introduce persistent docs caching. Preserve strict embedding-ID UTF-8 and existing lossy BM25/HNSW decoding. |

All four Copilot commits also carry Claude coauthor trailers; fc40d09 carries
a Claude coauthor trailer. Do not relabel these four commits as Pradeep-authored.
Do not attribute fresh fork-only tests, documentation, or independently authored
fixes to the original source author by copying author fields onto a new patch.
Keep such additions in honest follow-up commits. A `-x` trailer is necessary
for this owner's requested workflow but does not by itself prove patch origin;
compare each picked patch and every resolved hunk with the original.

## Required conflict and behavior review

1. **Provenance:** enumerate fork commit -> full source SHA; verify author/email,
   author date, original message/coauthors, and full `(cherry picked from commit
   <SHA>)` trailers. Obtain the implementation agent's actual cherry-pick and
   conflict-resolution receipt. No old user-authored implementation disguised
   as a source-author pick; no squash/rebase merge that discards the attributed
   series when eventually integrating it.
2. **Parser equivalence:** reject every truncated prefix of a valid BM25 cache,
   impossible document/term/posting/HNSW counts, absent-document posting indices,
   and truncated/overflowing dimensions before slicing or allocation. Valid BM25
   scores, headerless embedding roundtrips, binary HNSW and legacy JSON lookup
   must remain compatible. No new hashed/versioned embedding writer or cache
   format should enter through conflict resolution.
3. **Storage preservation:** initial and retry-time lock/version errors must
   return before wipe, quarantine or combined-store rebuild. Check all five
   caller boundaries, underlying error causes and context wrappers. Preserve
   code/docs/combined bytes and sidecar sentinels on refusal. A real lock test
   needs two owned OS processes; two handles in one process are insufficient.
   Source-only WAL predicates are not evidence of native WAL recovery safety.
4. **Scope and integration:** preserve #9's stored TypeScript import endpoints
   and SCIP enrichment preservation, #7 watcher fixes, diagnostics and existing
   supervisor behavior. Require an explicit reason for any diff in their code.
   No daemon, new service/stack, installer, dependency upgrade, schema migration,
   release, tag, workflow workaround, or live index operation belongs here.
5. **Test provenance:** retained old #13 tests must be named and compared;
   the old report's 49 consumer cases are historical evidence, not proof of the
   replacement revision. Do not claim all 49 are 49 newly added regressions:
   they combine new hardening with existing embedding/search/lock/import cases.

## License and notices

At both pinned trees, `LICENSE` is the same Apache-2.0 blob
`9ecfccb2705e510fca8ec9b9b06207d9958ac5ae`; `NOTICE` is the same blob
`dcd0b029f3e2fae8d622463bcba0af7870cc0bb6`. NOTICE retains Intuit's
2024-2026 attribution and the bundled model notice. No additional
Pradeep-specific NOTICE text is present in this source snapshot.

[Apache-2.0 section 4](https://www.apache.org/licenses/LICENSE-2.0#redistribution)
requires distributing the license, prominent modification notices in changed
files, retention of applicable source notices, and readable applicable NOTICE
attribution. Preserve existing notices and identify fork conflict adaptations;
git authorship/trailers serve the owner's provenance requirement and do not
replace these distribution obligations. Do not invent new copyright ownership
from an author name or remove model notices because no model files changed.

## Validation and guarded integration gates

### Exact-head review of replacement #16

Reviewed base `dc45de6e5769c1d86116167dcb1660d9561736c8` through head
`662b24515fd94e5f0cd28fc2e63bdb95ce85263a`, including each attributed pick,
the separate integration edits, retained old fixtures, and conflict receipt.
Automated Git-object comparison confirmed original author/email/date, complete
original message/coauthors and full `-x` trailer for all four mappings:

| Original | Replacement commit |
| --- | --- |
| e4257a1 | e119b69748b009ba76af02716e9342de51636ac8 |
| de33d9c | ba76d6fcb7cbf3a9d79bf2906ef3950b32e657c9 |
| 7908e29 | 0a24b918134e081cfea0d71312bc0d4ab1fe1009 |
| 106184a | e83fe4f9ecd6577205b81662fe8738e5d842f064 |

ByteReader and the historical old report are byte-identical to their original
blobs. The current headerless embedding pairs/API and HNSW formats are retained;
the absent docs-cache loader and source-only daemon/WAL/lock-probe dependencies
are excluded. Checked BM25 reads/posting indices, initial/retry version refusals,
all five caller boundaries, cause-chain classification and owned child cleanup
were inspected. Old hardening fixtures remain with additive coverage. Honest
redxzeta integration commits add fixtures/cause-chain compatibility and restore
an existing embedding diagnostic without changing its assertion. No concrete
reachable P1/P2 product defect was identified in this reviewed range.

The report-only final commit leaves product/tests identical to the author's
tested `fa98356a296d49f4a268db88e13247e5c609fccc` tree. Manifests, lockfile,
workflows, scripts, installer, import resolution, language and MCP implementation
paths are unchanged from the base. Replacement `git diff --check` passed.

[CI run 40441950687, job 121984120369](https://github.com/redxzeta/afterimage/actions/runs/40441950687/job/121984120369)
is a completed **FAILURE** with run head equal to the reviewed head. Its log
establishes 20 script tests, fmt, strict workspace/all-target Clippy, CLI/MCP
build and the entire selected integration batch passed. The batch has 23 result
blocks totaling 217 passed tests, including core hardening 9, docs hardening 5,
docs version 1, combined graph 14, embedding format 2, TypeScript imports 8
(including synthetic SCIP preservation), and all 9 tool-parity cases. These
are hosted results, distinct from the author's 74-case local consumer receipt.

The next command, `cargo test --locked -p infigraph-core --lib --bins --
--test-threads=1`, failed while compiling lbug 0.19.1: **could not find native
static library `lbug`, perhaps an -L flag is missing?** Exit was 101. Core units
and later selected crate-unit commands therefore have no passing receipt from
this run. This is a build/validation blocker, not a demonstrated parser defect.
An [actionable HOLD comment](https://github.com/redxzeta/afterimage/pull/16#issuecomment-6432780518)
asks the implementation owner to diagnose archive/search-path/cache or feature
provenance and obtain green focused CI without dropping required tests.

Selected watcher/concurrent-reader/supervisor results remain **author-reported**;
the reviewer did not rerun them. The benchmark owner's separate results remain
pinned to dc45de6e and cannot validate replacement code. No merge or old-PR
closure was performed. Any corrected head requires fresh review of its delta
and a current-head CI readback. Optional report PR #15 remains unmerged.

| Gate | Evidence required | Current disposition |
| --- | --- | --- |
| Replacement target/head | Fresh GitHub PR URL, base `afterimage-main`, exact head and complete commit list | **VERIFIED: #16 / 662b2451** |
| Source/conflict review | Per-commit comparison against pinned originals and explicit omitted/adapted hunks above | **REVIEWED; no concrete product blocker** |
| Focused CI | Green current-head required checks, run/job URLs, tested checkout SHA and relevant test log evidence | **FAILURE: missing native lbug on core-unit build** |
| Hardening regressions | Replacement-head native or traceable focused CI parser/store coverage, including old #13 compatibility and preservation cases | **Hosted integration PASS; core units unverified** |
| Existing integration | All 8 `typescript_imports` cases (including synthetic SCIP), relevant graph/lock tests, selected watcher cases, build/interfaces | **Hosted selected integrations PASS; watchers author-reported** |
| Reviewer local native work | Explicit native-lane release before any build/test; bounded serial fixtures only | **RELEASED; NOT_RUN, used hosted logs** |
| Runtime/deployment | Separate authorization; no live indices, installation or activation | **OUT_OF_SCOPE** |

Current `.github/workflows/ci.yml` targets `afterimage-main`, uses Rust 1.98.1
and Linux GCC13, and invokes `scripts/verify.py`. The old report's assertion
that workflow routing excludes this branch is historical and no longer current.
Inspect actual run logs: `verify.py` selects changed integration targets and
crate unit tests, but test names/results must establish the hardening coverage.
A green formatting/build step alone is insufficient. A cached binary without
revision provenance or the benchmark at dc45de6e cannot validate replacement code.

The old report records a baseline-and-candidate 90-second `tool_dispatch`
timeout and incomplete ordinary core/full-workspace validation. Keep those
limitations explicit. Do not extend timeouts, skip a failing required check or
equate historical consumer runs with native replacement-head CI. Any accepted
focused scope must be stated separately from unestablished full acceptance.

Before integration, coordinate the source revision boundary with the benchmark
owner and re-read GitHub PR state, base/head, draft status, check suites and
reviews. Hold on red/missing evidence, unresolved findings or a changed head.
Use a merge commit with an expected-head guard so original author commits remain
in ancestry. If the base changes materially, re-evaluate the combined diff and
required validation. Verify merged state, merge SHA and attributed ancestry
afterward. Only then close old #13 with the merged replacement link and read back
its final state. Leave experimental #10 and fork `main` untouched.

## Small subsequent work

- **Native WAL preservation:** one disposable two-process checkpoint/dead-holder
  fixture, byte/sibling snapshots and bounded child cleanup. Define refuse versus
  recover behavior before expanding recovery policy; no new daemon.
- **Corrupt sidecars:** extend the small malformed corpus to header/dimension
  boundaries and legacy JSON/type errors under a finite allocation/time budget.
  Native HNSW internals, concurrent mmap mutation and structurally valid huge
  files remain separate from these length-reader checks.
- **CLI/MCP parity:** one equivalent read-only query and one incompatible-store
  refusal through each interface against owned fixtures; assert error/diagnostic
  semantics and unchanged bytes. Static tool-name parity is narrower evidence.
- **Current-head real repositories:** consume the independent lane's pinned
  three-repository results, preserving its tested dc45de6e label even if the
  branch advances. Keep AST, synthetic SCIP, external SCIP, fixed tool questions,
  actual agent A/B and speed comparisons separate. If replacement changes shared
  behavior, plan a bounded replacement-head check after native-lane release.

No implementation source, old draft state, release, deployment or live index was
changed by this review. Findings were posted to #16. The next gate is a corrected
head or native-build receipt with green focused CI; this report makes no
implementation approval.

# Original hardening cherry-pick integration

Status: DRAFT; replacement validation PENDING. No independent approval or merge.
Base: `dc45de6e5769c1d86116167dcb1660d9561736c8` (`origin/afterimage-main`),
refreshed before starting in the clean T3-bound worktree.
Source: [pradeepmouli/infigraph](https://github.com/pradeepmouli/infigraph/tree/d1b8bc0edc1e4645d11e51c3370ad27e10c0591c)
at immutable `d1b8bc0edc1e4645d11e51c3370ad27e10c0591c`; fetched by HTTPS
into a local review ref without adding a push remote. Latest source HEAD was not used.

This replaces [PR #13](https://github.com/redxzeta/afterimage/pull/13), whose
head remains `f322556403e4f51448cd16233fff303a142f02c8`.
The original `PRADEEPMOULI-HARDENING-TRIAL.md` is retained byte-for-byte as
historical evidence, including its old base and old CI-routing limitations.
Those are not replacement-head test results. PR #10 is separate and untouched.
Reviewed the independent [PR #15 checklist](https://github.com/redxzeta/afterimage/pull/15)
at `a6dce749179ce0df8becb1876eb73a2aa1685440`; it is not an implementation
or an approval prerequisite. The decisions below address its source audit.

## Genuine cherry-picks and authors

Each applicable original was executed separately using `git cherry-pick -x
<FULL_SOURCE_SHA>`. Conflicts were staged and completed with `git cherry-pick
--continue`; no author override, copied adaptation commit, squash, source merge
or unrelated dependency cherry-pick was used. The four commands all conflicted;
the resolved boundaries are listed below. Commit messages retain the full
`(cherry picked from commit <FULL_SOURCE_SHA>)` line.

| Source SHA | Resulting fork commit | Original author/date (preserved) | Preserved coauthor |
| --- | --- | --- | --- |
| `e4257a123b8ae74c0ab838098e50f74862620702` | `e119b69748b009ba76af02716e9342de51636ac8` | GitHub Copilot <copilot@github.com>, 2026-09-01T22:52:07-04:00 | Claude Fable 5.1 <noreply@anthropic.com> |
| `de33d9cfb2809fe0a34f4375bb80a46e515a1253` | `ba76d6fcb7cbf3a9d79bf2906ef3950b32e657c9` | GitHub Copilot <copilot@github.com>, 2026-09-02T10:35:04-04:00 | Claude Fable 5.1 <noreply@anthropic.com> |
| `7908e299a3caf7796d4c809634b4db663f8cc2c8` | `0a24b918134e081cfea0d71312bc0d4ab1fe1009` | GitHub Copilot <copilot@github.com>, 2026-09-04T13:57:02-04:00 | Claude Opus 5 <noreply@anthropic.com> |
| `106184a396b517f12c09f3c1974f57cd136d80c8` | `e83fe4f9ecd6577205b81662fe8738e5d842f064` | GitHub Copilot <copilot@github.com>, 2026-09-04T14:20:14-04:00 | Claude Opus 5 <noreply@anthropic.com> |

`git log --reverse --format=fuller BASE..HEAD` was inspected. An automated
comparison verified exact original author name/email/date and all original
Co-Authored-By and Claude-Session trailers on 4/4 picks. The committer is
redxzeta <xzetanzero@gmail.com>; that does not replace the original author.
The ByteReader source blob is unchanged from 106184a.

Skipped `fc40d09027b9bbe761bb4b2ddab462abd93bdcce`: the equivalent lock
refusal is already present in ancestor `0080982f8e3fcce9acc344b79a0d0d10da0063f5`,
with Pradeep Mouli <pmouli@mac.com> as author and Claude Sonnet 5 coauthor.
The source SHA itself is not claimed to be an ancestor or patch-ID-identical.
No empty commit or synthetic Pradeep authorship was created.

## Conflict receipt and compatibility

- **e4257a1:** retained its initial/retry version-refusal branches, two classifier
  tests and native byte-preservation test. Removed conflict context containing
  source-only WAL classifiers, lock probes, quarantine/daemon tests. Did not
  import the absent DESIGN-hardening document or the source implementation plan;
  their claims require a recovery architecture absent here.
- **de33d9c:** moved lock/version classification into graph/store and guarded
  docs init, combined code open and combined docs query/search. Kept current
  graph exports and generic lock diagnostic instead of lock_probe/ProbeFor.
  Omitted transient-WAL classification, schema guards and WAL recovery context.
  In the absent wal_guard target, retained only the actual storage-version test
  added by de33; no unrelated pre-existing source WAL tests were imported.
  Kept its combined-graph version test. The original commit message describes
  the original source scope; this receipt defines the smaller fork resolution.
- **7908e29:** kept BM25 and HNSW count bounds and all three new source
  regressions. Headerless embedding count protection already existed; retained
  it with the corruption diagnostic. Did not import source embedding magic,
  checksum, input-hash field, triple return type, writer or format upgrade.
- **106184a:** kept original ByteReader, checked BM25 reads/counts/posting indices,
  HNSW reader and overflow-safe embedding dimension reads. Bound the embedding
  reader to the existing headerless `(id, vector)` format and existing as_chunks
  convention. Omitted docs/search.rs because Afterimage has no persisted docs
  BM25 save/load path; importing its conflict context would add a new cache and
  atomic-writer mechanism. Strict embedding-ID UTF-8 and lossy BM25/HNSW strings
  remain as before. This pick is needed: 7908e29 alone leaves raw BM25 slices
  and unchecked posting indices exposed.

Separate integration work (authored by redxzeta, not relabeled as a source pick)
retains the old #13 core/docs hardening fixtures, adds real two-process
code/combined/docs lock preservation coverage and a wrapped-error classifier
case, and traverses anyhow causes so context cannot hide native refusals.
Formatting and fork-compatible diagnostics are also separate integration edits.
Based-on refs: the four source commits above and old #13's
`05b40a8978273a59fbb7475a93e6034ccef3a4c8` for its exact retained fixtures.

| Old #13 essential behavior | Replacement disposition |
| --- | --- |
| BM25 truncated prefix, impossible counts, invalid posting index return errors | Preserved by 7908 + 106184; same old fixture retained |
| HNSW impossible ID count returns error | Preserved by original source sidecar reader; same fixture retained |
| Valid BM25 scores, headerless embeddings, binary/legacy JSON HNSW | Formats/API preserved; exact compatibility fixtures retained |
| Version-incompatible code/docs/combined stores, sidecars remain intact | All five refusal boundaries preserved; original and old fixtures retained |
| Live document lock never wipes the store | Preserved; coverage extended to code/combined lock holders |
| Native error hidden by anyhow context still refuses | Preserved in separate integration commit |
| Cross-repo imports, watcher/supervisor behavior | No implementation changes; fresh relevant verification pending |
| Wider source WAL/daemon/quarantine/recovery, persisted docs cache | Deliberately omitted; not old #13 behavior |

## Validation receipts

PENDING: exact replacement fixtures, relevant integration checks, strict Clippy,
binary build, final fmt/diff and hosted CI. The ordinary offline core target
was attempted and is BLOCKED: cached `cargo-husky v1.5.0` is missing. No hook
was installed or dependency removed. Disposable consumer compilation is in
progress using exact repository test files and identical locked dependency
versions. Linux local toolchain is Rust 1.99.1; hosted CI requires Rust 1.98.1
and GCC13. Local results cannot establish that hosted gate.

The historical broad `tool_dispatch` timed out at 90 seconds on both baseline
and candidate; its disposition remains existing NOT_PASSED. No pass is inferred
from an absent/filtered test. Full workspace, macOS/Windows, native WAL/power-loss,
real external SCIP and arbitrary native HNSW internals remain unverified.

## Next steps and scope

1. Independent exact-head review against originals and current-head CI. Reviewer
   checks real metadata and every conflict hunk; no self-approval claim.
2. Coordinator may merge with an expected-head guard and a merge commit only
   after review/CI acceptance, preserving original authored ancestry. Then close
   #13 with the merged replacement link. Leave both unmerged/open meanwhile.
3. Coordinate current-head IceCouncil/Nightwire/GigLab source benchmarks and
   actual agent A/B with the separate reserved native lane. Pin each tested SHA;
   its dc45de6e results do not validate this candidate. No target applications
   or production indices are run by this task.
4. Evaluate residual cross-platform, native WAL and real SCIP limitations as
   separate bounded follow-ups. Cache parser checks do not establish native
   engine integrity, crash durability or broad production readiness.

LICENSE/NOTICE and all existing source notices remain unchanged. Both base and
pinned source have LICENSE blob `9ecfccb2705e510fca8ec9b9b06207d9958ac5ae`
and NOTICE blob `dcd0b029f3e2fae8d622463bcba0af7870cc0bb6` (Apache-2.0).
No dependency, CI, release, install, tag, deployment, service, LXC, schedule,
production index or live trading action is added. Nathan acquires no new
operational responsibility. Fork main and upstream are never pushed.

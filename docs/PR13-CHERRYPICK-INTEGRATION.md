# Original hardening cherry-pick integration

Status: DRAFT; focused local validation complete on product/test revision
`fa98356a296d49f4a268db88e13247e5c609fccc`. Hosted CI and independent exact-head
review remain gates. No approval, merge or old-PR closure is claimed.
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
Commit `fa98356` separately preserves the established embedding count diagnostic
asserted by the unchanged Afterimage embedding_format fixture.
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
| Cross-repo imports, watcher/supervisor behavior | No implementation changes; all 8 import cases and 3 selected watchers pass |
| Wider source WAL/daemon/quarantine/recovery, persisted docs cache | Deliberately omitted; not old #13 behavior |

## Validation receipts

Local tests use the committed product/test tree at `fa98356`; subsequent report
edits do not change that code. These results do not establish hosted CI or
independent approval. Local Linux compiler: Rust 1.99.1, GCC 13.3.0. Hosted CI
uses Rust 1.98.1 and GCC13.

| Check | Actual result | Seconds | Sampled group MiB |
| --- | --- | --- | --- |
| Consumer, exact repository sources (includes old 49 cases) | 74 passed / 0 failed | 37.496 | 1027.7 |
| Native docs hardening | 5 passed / 0 failed | 0.705 | 279.9 |
| Native source docs version test (wal_guard target) | 1 passed / 0 failed | 0.182 | 143.7 |
| Native combined docs | 5 passed / 0 failed | 7.380 | 381.9 |
| Static tool parity | 8 passed / 0 failed; 1 filtered | 0.038 | 18.5 |
| Concurrent writer/reader | 1 passed / 0 failed | 12.792 | 572.4 |
| Modified-code watcher | 1 passed / 0 failed | 7.805 | 409.8 |
| New-code/new-directory watcher | 1 passed / 0 failed | 10.645 | 410.5 |
| Deleted-document pruning watcher | 1 passed / 0 failed | 1.370 | 265.7 |
| Broad tool_dispatch | TIMEOUT; NOT_PASSED | 90.034 | 452.1 |
| Strict Clippy (core/docs/MCP/CLI libraries + binaries) | PASS, exit 0 | 101.513 | 1699.5 |
| Native CLI + MCP binary build | PASS, exit 0 | 89.317 | 4541.4 |
| Unchanged supervisor: 8 process scenarios + 2 unit tests | PASS, exit 0 | 9.312 | 171.1 |
| Native CLI incompatible-store probe | PASS: expected exit 1; database SHA-256 and sibling sentinel preserved | 3.161 | 233.9 |
| Verification-script unit tests | 15 passed | | |
| Diagnostic-script unit tests | 5 passed | | |
| Cargo fmt and full diff whitespace check | PASS | | |
| Ordinary core target / all-target Clippy | BLOCKED offline: cargo-husky 1.5.0 missing | | |
| Current-head hosted CI / independent review | PENDING; no approval | | |

The 74-case consumer comprises core hardening 9, docs hardening 5, embedding
format 2, hybrid search 21, write-lock 8, TypeScript imports 8, original
ByteReader tests 7 and combined graph 14. Old #13's 49 cases are a subset,
not 49 newly added tests. Native docs runs duplicate some consumer coverage;
counts must not be summed as unique assertions.

The consumer depends on the actual checkout's core/docs/languages crates and
includes exact repository test files using `#[path]`. The embedding/import/
search/lock fixtures are unchanged. It adds tree-sitter-python only to its
temporary manifest for the existing combined-graph fixture. Every selected
dependency version matches Cargo.lock; 96 workspace-only packages are absent,
and the only extra package is the consumer itself. Repository manifests and
lockfile are unchanged. Including the original ByteReader module emits a
dead-code warning for its unused public cursor helpers in that private test
module; production strict Clippy passes without an allowance. Ordinary core
unit tests (including source e425/7908 additions) and full-workspace acceptance
are not claimed from this consumer. The source docs-version and combined-graph
regressions did run in the native/consumer targets shown above.

Initial failures and corrections remain in the local receipt:
- Ordinary core compilation failed offline on missing cargo-husky 1.5.0. No
  dependency was removed and no hook was installed.
- The first scratch-cache build was stopped to avoid duplicate compilation;
  later builds reused the retained Cargo cache into task-owned outputs.
- The first expanded consumer compilation lacked the existing combined fixture's
  direct tree-sitter-python import. Only its temporary manifest was corrected.
- The first runtime run was **64 passed / 10 failed**. One real compatibility
  regression was the count diagnostic asserted by embedding_format; separate
  integration commit `fa98356` restored that diagnostic as context around the
  source reader, without changing, disabling or ignoring the test.
- The first combined fixture failed an 8 TiB mmap reservation under the 32 TiB
  virtual limit; eight later cases hit its poisoned mutex. The unchanged full
  74-case suite then passed with 64 TiB virtual space, the same 2 GiB sampled
  group RSS cap and the same 90-second deadline. This adjusts the native
  fixture's sparse-address reservation budget, not corruption acceptance.
- Broad tool_dispatch again exceeded 90 seconds (90.034 s, 452.1 MiB). Old #13
  reproduced this timeout on baseline and candidate. It remains existing
  **NOT_PASSED**, not a pass or a diagnosed root cause.

Commands used (scratch paths abbreviated):

```sh
cargo test --offline --locked --no-default-features -p infigraph-core --test hardening_trial --no-run
# BLOCKED: cargo-husky missing
cargo test --offline --locked --manifest-path <consumer>/Cargo.toml --no-run
cargo test --offline --locked --no-default-features \
  -p infigraph-docs --test hardening_trial --test wal_guard --test combined_docs \
  -p infigraph-mcp --test tool_parity --test tool_dispatch \
  --test watcher_reindex --test concurrent_writer_reader --no-run
cargo clippy --offline --locked --no-default-features \
  -p infigraph-core -p infigraph-docs -p infigraph-mcp -p infigraph-cli \
  --lib --bins -- -D warnings
cargo clippy --offline --locked --workspace --all-targets -- -D warnings
# BLOCKED: cargo-husky missing
cargo build --offline --locked --no-default-features \
  -p infigraph-cli -p infigraph-mcp --bin infigraph --bin infigraph-mcp
cargo fmt --all -- --check
git diff --check origin/afterimage-main
python3 scripts/verify.py --base origin/afterimage-main --dry-run
```

Each compiled runtime target was copied away from the CLI and invoked serially
as `unshare -Urn <test-executable> --test-threads=1` from disposable storage.
This preserves the existing MCP inline fallback without installing an indexer.
Selected watchers used their exact names and `--exact`. Tool parity used
`--skip dispatch_handles_all_mcp_tool_names`: eight static mapping checks are
narrower than dispatching every potentially side-effecting handler with empty
arguments. That ninth case was not run or claimed passed. The separate
unfiltered tool_dispatch attempt timed out as recorded above.

The native CLI probe copies a retained disposable incompatible-store fixture
(LBUG version 48) into a fresh owned project, invokes the newly built `index`
under network-isolated bounds, and checks exit 1, the native refusal plus
`left untouched`, unchanged database SHA-256 and a sibling sentinel. Regression
tests create fresh native stores and stamp their actual native version +1.
The retained lbug static archive reports storage version 47; its SHA-256 is
`9100591f7cf1de0f565c96b55e3293998763260fc936bfbb9e304bef6f4ec2b8`.
The Rust crate is locked to lbug 0.19.1; its cached archive is labeled `latest`,
not a freshly established immutable native release. No C++ backend rebuild
or download was performed.

Builds used two CPU affinity slots, two Cargo jobs, nice 10, core dumps off,
8 GiB per-process address space, 10 GiB sampled process-group RSS, 4 GiB file
caps and finite 60-600 second deadlines. Runtime used owned HOME/TMPDIR/registry,
core dumps off, `INFIGRAPH_BACKEND=kuzu`, `INFIGRAPH_NO_WATCH=1`,
`HF_HUB_OFFLINE=1`, no external network namespace route, 64 TiB virtual space,
2 GiB sampled group RSS, 256 MiB file caps and 90-second deadlines (30 seconds
for the CLI probe). Limits are sampled, not kernel-enforced aggregate RSS or
full filesystem isolation. Only owned children were killed/reaped. No target
project application or production index was run.

Current verify.py selects core + cross-repo + interfaces and includes the new
hardening_trial/wal_guard targets. Its plan and 20 script tests passed; that is
not execution of the full CI command set. Offline all-target Clippy is blocked
by cargo-husky. Full workspace/core units, broader watcher cases, macOS/Windows,
remote backends, HTTP, optional kompress, native HNSW internals, concurrent mmap
mutation, WAL/power-loss durability and real external SCIP remain unverified.
Synthetic SCIP/import cases do not establish external indexer acceptance.

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

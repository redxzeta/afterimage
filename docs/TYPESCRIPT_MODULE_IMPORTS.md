# TypeScript module import correction

Scope: embedded graph module dependencies, tested with synthetic sources. This
change does not establish real-repository benchmark improvement, compiler
completeness, deployment readiness, or upstream acceptance.

## Reproduction and cause

On base `1a59c728ae3284f0af3b5c781e14a9e2736df6e0`, a two-file fixture:

```ts
// helpers.ts
export function helper() { return 1; }
// main.ts
import { helper } from './helpers';
export function main() { return helper(); }
```

produced both Module nodes, with `file == id` for each, and no rows from either:

```cypher
MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.file, b.file
MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.id, b.id
```

The saved base CLI indexed two syntax files and resolved the function call. It
exited 1 for unavailable SCIP tooling in a network-isolated fixture. That exit
remains a partial-index failure; graph evidence is assessed separately.

The parser captures the whole string literal, including quotes. Bulk storage
looked up a basename stem containing those quotes, or the `js` suffix, instead
of the imported source file. The incremental writer instead matched the raw
`file::specifier` extraction target against Module IDs, which are file paths.
Re-export sources were absent from TS/TSX/JS relation queries altogether.
Missing labels outside a benchmark projection are a different issue: this
reproduction verifies both Module endpoints before asserting the edge.

## Corrected behavior

A shared lookup resolves only against indexed Module IDs. JS/TS relative imports
use the importing file's directory, normalize `.`/`..`, preserve case, and support
extensionless sources, explicit `.ts`/`.tsx`, and directory `index` files. `.js`
can refer to `.ts`/`.tsx`/`.d.ts`; `.mjs` and `.cjs` can refer to their TypeScript
counterparts when those Module IDs are present (the bundled registry does not
currently index `.mts`/`.cts`). Multiple source alternatives remain unresolved.
TS/TSX/JS re-exports now produce module dependencies using the same path.

Cold Parquet, bulk UNWIND, and single-file embedded writes share this lookup.
Small incremental batches defer per-file import writes and resolve only after all
Module nodes exist. An order-sensitive regression covers both an ambiguous
`.ts`/`.tsx` set and a temporary `.js` fallback; early provisional edges must
never survive final resolution. Updating a module preserves incoming imports from unchanged files; changing an importer
replaces its outgoing imports. MERGE prevents duplicate module edges from
repeated import/re-export statements. Query changes invalidate the affected
language packs' existing extraction fingerprints automatically.

`crates/infigraph-core/tests/typescript_imports.rs` asserts actual
`Module-[:IMPORTS]->Module` endpoints by both `file` and `id`, checks duplicate
basenames, missing/case-mismatched/ambiguous paths, both incremental write
branches, and repeated SCIP enrichment preserving syntax import edges.

## Validation checkpoints

Final tested implementation:
[`bba675c9f6fcfa60c2f2a9ae6ea120ebbed433aa`](https://github.com/redxzeta/afterimage/commit/bba675c9f6fcfa60c2f2a9ae6ea120ebbed433aa),
rebased onto `afterimage-main` at
`de02e762c5581055d2adcefe7efe1a58bc0c7cab`. This base includes the #7 merge
`585fd5d9221ced25a8bc2307c0b0e5fd9ff49010` and #8 acceptance documentation.
The delivery commit following this checkpoint changes only this document; the
exact final PR head and base are recorded in PR #9's delivery comment.

All native checks ran after the benchmark's explicit measurement release, using
the reserved benchmark Cargo cache, one build job, serial tests, locked/offline
dependencies, disabled Git hooks and a 240-second outer limit per final command.
No CI cache or target application was used. No existing assertion/deadline was
weakened. The five rebased implementation/test patches are unchanged by
`git range-diff`.

| Revision and check | Result |
| --- | --- |
| Base `4d5266fe10c2c39dbaac423c7a43731329601e85`, native two-file extensionless fixture | Expected failure, 0 passed / 1 failed, exit 101; actual stored edge missing |
| Pre-order-fix `32459fd2aeb24469afb874744b5294821567157e`, new order-sensitive fixture | Expected failure, 0 passed / 1 failed, exit 101; ambiguous final set retained a provisional `main.ts -> helper.ts` edge |
| Prior head `df7a8687e3cc2024172c27bf1a66e1a408c96111`, import suite | 8 passed, exit 0 |
| Same prior head, resolver/storage utilities/SCIP library tests | 29 passed, exit 0 |
| Same prior head, unchanged TypeScript callers suite including #6 receipt regression | 6 passed, exit 0 |
| Final tested implementation, import suite | 8 passed, exit 0; 27.91 seconds test runtime |
| Final tested implementation, exact #6 `reindexing_unchanged_files_invalidates_compiler_enrichment` regression | 1 passed, exit 0; 6.05 seconds test runtime |
| Final tested implementation, unchanged #7 `watch_new_directory` suite | 4 passed, exit 0; 51.01 seconds test runtime |
| Final tested implementation, strict Clippy for core import/caller targets | PASS, exit 0 |
| Final tested implementation, strict Clippy for MCP watcher target | PASS, exit 0 |
| Formatting and whitespace checks | PASS, exit 0 |

Final commands, with `--test-threads=1` for each test target:

```text
cargo test --locked --offline -p infigraph-core --test typescript_imports
cargo test --locked --offline -p infigraph-core --test typescript_callers reindexing_unchanged_files_invalidates_compiler_enrichment -- --exact
cargo test --locked --offline -p infigraph-mcp --test watch_new_directory -- --nocapture
cargo clippy --locked --offline -p infigraph-core --test typescript_imports --test typescript_callers -- -D warnings
cargo clippy --locked --offline -p infigraph-mcp --test watch_new_directory -- -D warnings
```

The watcher return case converged in **2.367690088 seconds** within the existing
single 12-second return deadline. Across the watcher suite, five explicit
unsuccessful native reads (missing WAL/checkpoint conflict) preceded valid
snapshots. They are retained limitations, not successful reads or a native
repair claim. The inherited delayed-start cleanup, crash/WAL and platform
limitations in `PHASE-D-RELIABILITY-FOLLOWUP.md` remain; global operational HOLD
is unchanged.

The initial green attempt at `d2e07901` was deliberately terminated while the
fixture repeatedly constructed the whole language registry. Only the fixture
helper was changed to share an immutable registry before the successful
`df7a8687` replay. The terminated attempt is not counted as a passing run.

All four saved baseline/candidate benchmark executables retain their recorded
SHA-256 hashes. Measured candidate source remains
`1a59c728ae3284f0af3b5c781e14a9e2736df6e0`; none of these checks establishes a
current-base speedup or real-repository import score. Full workspace tests,
optional-feature/remote parity and other-platform validation were not run in
this bounded follow-up. Hosted CI is NOT_RUN: the base workflow targets `main`,
so an `afterimage-main` PR is expected to have no checks. No CI workaround was
introduced here.

## Deliberate limits

- Bare/scoped workspace names and package subpath exports remain unresolved in
  syntax-only indexing, including explicit `package.json` exports. The regression
  fixture records this boundary. No basename fallback, package installation,
  node_modules traversal, or general Node/TypeScript resolver is introduced.
- tsconfig paths/baseUrl, conditional/wildcard exports, package imports, dynamic
  imports and TS `import = require` are outside this correction. Existing
  JS/TSX literal require captures use the same relative lookup; TypeScript
  require extraction is unchanged.
- This bounded candidate matching does not implement TypeScript's complete
  module-resolution precedence. Multiple indexed source alternatives are skipped.
  Escaped string literals are not decoded by the lookup.
- SCIP import currently enriches symbols and writes CALLS/INHERITS, with no
  Module IMPORTS writer. A successful SCIP receipt does not repair a missing
  syntax module dependency. The synthetic preservation test does not establish
  compiler-generated workspace resolution.
- Repeated identical SCIP artifacts can still duplicate stored CALLS edges:
  SCIP's seen-edge set is per import invocation. UI deduplication is not graph
  idempotency. That independent issue is deferred to keep this fix scoped.
- Neo4j writes retain their existing import-resolution implementation; remote
  module-edge parity is not claimed. A previously unresolved import whose target
  is newly added still needs its importer reindexed. Target removal prunes edges;
  adding a new competing source alternative may require a full index.

Existing real-repository benchmark results remain pinned to their original
binary revisions. No IceCouncil, Nightwire, or GigLab checkout/application was
used to validate this patch.

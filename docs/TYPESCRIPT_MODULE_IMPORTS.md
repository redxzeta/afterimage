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
currently index `.mts`/`.cts`). Multiple source alternatives remain unresolved. TS/TSX/JS
re-exports now produce module dependencies using the same path.

Cold Parquet, bulk UNWIND, and single-file embedded writes share this lookup.
Small incremental batches resolve after all Module nodes exist. Updating a
module preserves incoming imports from unchanged files; changing an importer
replaces its outgoing imports. MERGE prevents duplicate module edges from
repeated import/re-export statements. Query changes invalidate the affected
language packs' existing extraction fingerprints automatically.

`crates/infigraph-core/tests/typescript_imports.rs` asserts actual
`Module-[:IMPORTS]->Module` endpoints by both `file` and `id`, checks duplicate
basenames, missing/case-mismatched/ambiguous paths, both incremental write
branches, and repeated SCIP enrichment preserving syntax import edges.

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

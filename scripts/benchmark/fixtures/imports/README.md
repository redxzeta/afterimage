# Import-graph diagnostic fixture

This is synthetic source for static indexing only. Do not run these modules or
install dependencies. It contains no application scripts or hooks.

Expected source module dependencies:

- `extensionless.ts -> helpers.ts`: quoted relative import without an extension.
- `js-suffix.ts -> helpers.ts`: Node-style `.js` source specifier for a TS module.
- `ts-suffix.ts -> helpers.ts`: explicit `.ts` source specifier.
- `barrel.ts -> helpers.ts`: re-export, labelled as a module dependency.
- `workspace.ts -> packages/shared/src/entry.ts`: workspace package `exports`.

Query both `a.file,b.file` and `a.id,b.id` through `Module-[:IMPORTS]->Module`,
and list `Module.id,Module.file` to rule out endpoint/property mismatch. Inspect
these immediately after cold indexing, before incremental indexing can delete
or replace edges. A failed optional analysis stage still makes indexing fail.

The separate SCIP prerequisite probe uses only `helpers.ts` and
`extensionless.ts`, a synthetic package manifest and an explicit `tsconfig.json`.
It copies an already cached indexer plus its dependencies and Node into the
private Bubblewrap lane. This fixture does not imply compiler completeness on
any repository source projection. See the [B/C/E report](../../results/2026-10-23-bce.md)
for commands, exact inputs and observed outcomes.

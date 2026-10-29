# Bounded JavaScript and TypeScript relationships

The syntax index distinguishes a module dependency from a callable binding.
Named imports, including aliases, keep their quoted module specifier and exported
name in the unresolved call target. CALLS then uses the same module lookup as
Module IMPORTS, including importer-relative paths and `.js` to `.ts` counterparts.
Duplicate filenames or exported names elsewhere cannot select a different file.
Unknown unqualified JS/TS callees have no global name fallback. Parameters,
loop declarations and named function/generator expressions shadowing an imported
binding remain unresolved. Imported targets must be directly exported named
callables: extraction marks internal top-level functions private in the existing
visibility field. An alias export cannot select an unrelated private function
with the exported name. No graph schema or receipt API changes are needed.

Workspace lookup reads root `package.json` workspaces or `pnpm-workspace.yaml`
packages, then package manifests above indexed files in matching directories.
It accepts a unique package name and an exact string `exports` entry, such as
`@giglab/contracts` -> `./src/match.ts` or
`@giglab/controllers/prompt` -> `./src/prompt.ts`. Destinations must already be
indexed, exist locally, and canonicalize inside the project. The standard
`.infigraph/graph` layout supplies the project root; standalone graph stores
without that layout keep syntax-only relative resolution.

Conditional objects, arrays, wildcards, duplicate package names, missing exports,
missing sources, paths escaping the package, and symlinks outside the project
return unresolved (`None` at the module-resolver boundary). No dependency edge
is inferred for those requests. This does not implement Node's resolver,
node_modules discovery, tsconfig aliases, runtime package conditions, or network
lookup. Package metadata is read as data; no package manager or target code runs.
Manifest-only edits require explicitly reindexing affected consumers; a no-op
source scan does not detect metadata changes. Aliased/default exports through
barrels and namespace call binding still require compiler enrichment.

A callback value is not a direct invocation. In the pinned IceCouncil source,
`jsonl = v => v.map(json).join("")` refers to `json`, but syntax extraction does
not establish that the receiver's `map` invokes that callback. This lane does
not add `jsonl -> json` CALLS or relabel the frozen truth. Call expressions inside
anonymous callbacks retain the enclosing named owner, as in existing extraction.

Target-only reindexing preserves incoming CALLS from unchanged consumers when
both endpoint IDs still exist and the destination remains callable (and directly
exported for JS/TS import consumers). Reversed
batches resolve after all changed modules exist. This is structural preservation;
compiler enrichment and source resolution receipts remain separate evidence.

## Reproduction and validation

Fixed minimized fixtures cover two direct CALLS and two workspace IMPORTS.
The unchanged implementation is base `dc45de6e5769c1d86116167dcb1660d9561736c8`.
The direct-call fixture uses duplicate filenames, duplicate exported names, an
import alias, a same-name local callable, and template interpolation. Workspace
fixtures cover both root declaration formats, re-exports, incremental batches,
missing/conditional mappings, duplicate packages, escapes and external symlinks.
The callback fixture asserts absence of an invented invocation.

The author ran the following checks at reviewed code commit
`2d6d4039945d76caea3d569a83918c280f5dc677`, with two build jobs, serial
runtime tests, offline/locked dependencies and disposable fixture roots. The
receipt-only documentation commit that follows it changes no source or tests.

- CALLS fixtures: **0/2** on the unchanged base and **2/2** at the reviewed code
  commit. Workspace IMPORTS fixtures: **0/2** on base and **2/2** at that commit.
  These are minimized cases, not whole-corpus benchmark scores.
- The caller suite passed 9 tests, imports 8, existing call resolver 16, and
  module-resolver units 4. Formatting and strict core/languages Clippy passed.
- The full frozen benchmark and a cold native build were **NOT_RUN**. Existing
  lbug 0.19.1 cached archives were reused; their provenance is reported in the
  PR handoff, not established by these tests.
- Hosted [CI run 40627540497](https://github.com/redxzeta/afterimage/actions/runs/40627540497)
  at the reviewed code commit **FAILED**: a subsequent lbug build could not find
  the native static library. Its selected caller/import tests passed earlier in
  that run; the overall job remains failed. The debugger diagnostic is not a
  test pass. Hosted CI is not a prerequisite under the owner's policy.

The commands below reproduce the focused checks on a serialized native build
slot using an existing trusted target cache or a separately provisioned native
build environment:

```sh
CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 cargo test --offline --locked -p infigraph-core --test typescript_callers --test typescript_imports -- --test-threads=1
CARGO_BUILD_JOBS=2 CMAKE_BUILD_PARALLEL_LEVEL=2 cargo test --offline --locked -p infigraph-core --test resolve_calls -- --test-threads=1
cargo test --offline --locked -p infigraph-core --lib graph::imports:: -- --test-threads=1
cargo fmt --all -- --check
cargo clippy --offline --locked -p infigraph-core -p infigraph-languages --all-targets -- -D warnings
```

Source-only reference pins: IceCouncil
`10435a8c19f84549a0206f6b249ea13166aff6d0`; GigLab
`02cee19056c8ee902289892727a6acbbcd30a7d3`; original three-project benchmark also
pins Nightwire `80985e6fc508f309b17ba1889ff2ca15b0253c08`.
This lane does not rerun that full benchmark, change truth manifests, or claim
new whole-corpus scores. Exact-head local results and cache provenance belong
in the PR receipt. No merge, release, deployment, target application execution,
or runtime activation follows from these checks.

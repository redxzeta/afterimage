# Read-only MCP instructions v1

Opt in for one existing MCP server process using its environment:

```sh
INFIGRAPH_MCP_PROFILE=read-only infigraph-mcp --mcp
```

No installation or user-global agent configuration is needed. Unset the variable
(or use `normal`) for the normal profile. Unknown profile values fail closed.
The read-only profile supports existing **local** graphs; it does not expose the UI.
Existing HTTP MCP calls use the same profile; readiness/reindex mutation endpoints
return HTTP 403 in read-only mode.

1. Pass the exact checkout directory as `path` on every call, including diagnostics.
   Paths are canonicalized. A linked Git worktree (`.git` file) or nested checkout
   is a boundary: an unindexed worktree never borrows its parent/sibling's graph.
   A parent directory does not select a registered child. Use explicit group tools
   in the normal profile for intentionally authorized multi-project operations.
2. Start with `diagnose`, then `search_symbols` (lexical BM25 in this profile;
   default 10 matches, maximum 1000) or `get_symbols_in_file`. Copy the exact
   `symbol_id` into `symbol_context`, `get_code_snippet`, `trace_callers`,
   `trace_callees`, or `transitive_impact`. Use `get_file_deps` for import
   dependencies and `get_stats` for graph counts. `find_all_references` adds
   indexed `JSX_USES` component value references alongside lexical `CALLS`;
   JSX usage is not an invocation. Locations identify enclosing declarations,
   not exact use sites. These are the eleven exposed tools.
3. Read the text's **Project**, **Freshness**, coverage and truncation warnings
   before acting. Compiler `skipped/NOT_REQUESTED` means enrichment was not
   requested; `failed` is a failed attempt. Both leave compiler coverage partial
   even if the requested AST pass succeeded. Indexed syntax files and successful
   stages never establish whole-graph completeness. Empty results do not prove
   absence; inspect current source before making a change.
4. `NOT_INDEXED` names the checkout that needs explicit indexing outside this
   profile. `INVALID_PATH`, `PATH_ESCAPE` and `UNKNOWN_ARGUMENT` require correcting
   the request, not switching projects. Manual calls to hidden tools, including
   raw graph queries, fail with `READ_ONLY_TOOL_DENIED`.

Reads do not index, start watchers, download models, recover/repair stores, write
sessions/activity/metrics/log files, delete projects, or edit the registry. A
crashed read-only worker exits without automatic reindex. Source snippets reject
paths escaping the selected project, including symlinks. This is a tool profile,
not an OS sandbox for an untrusted graph or process.

Graph results retain their existing text and add compact scope/coverage warnings.
Read-only result bodies are bounded to 200 lines or 32768 UTF-8 bytes, plus the
scope/warning footer; overflow is explicit. Narrow the query when truncated.
Machine-readable supplemental data is `result._meta.afterimage`, schema version 1:
project/worktree root; validated receipt generation when available; syntax,
resolution and compiler coverage; warnings, error code, source file/range where
known, and limits/truncation. Unknown facts are null/unknown. `get_stats` and
`diagnose` verify selected source content for explicit refreshed snapshots;
other tools report unknown freshness without scanning source. See
[M6 freshness and refresh](M6-FRESHNESS-REFRESH.md). Git HEAD equality and watcher
liveness are insufficient. Receipt evidence does not promise an atomic source
snapshot.

The server still negotiates `2024-11-05`. That version's official
[Result schema](https://github.com/modelcontextprotocol/specification/blob/2024-11-05/schema/schema.ts)
allows `_meta`; newer `structuredContent` is not assumed. Essential evidence also
appears in text for clients that discard metadata. `diagnose` retains versioned JSON text without a prose footer; in read-only mode
it adds `analysis_evidence` containing the same scope and coverage warnings. No full graph is serialized twice.

For raw JSON-RPC, inspect the top-level `error` first, then `result.isError`.
For example, a successful graph call may return:

```json
{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"No callers found\nProject: /workspace/topic-checkout\nFreshness: unknown. Empty graph results do not prove absence."}],"_meta":{"afterimage":{"schema_version":1}}}}
```

Read `result.content[0].text` as a string and check its warnings. Graph tool
text is prose; `diagnose` text is JSON and can be parsed as such. Metadata is
optional for consumers; a client wrapper may already unwrap `result`. Discover
the actual names and schemas with `tools/list` rather than invoking a placeholder
such as `TOOL`. These tools are optional aids: verify current source, including
when a graph result is empty. Confirm the process and profile you are using;
do not assume a global MCP registration points to this server.

Example text (project paths are illustrative):

```text
No callers found in the current graph for 'src/api.py::helper'

Project: /workspace/topic-checkout
Freshness: unknown. Warning: partial or unknown analysis coverage: scip-python Skipped (NOT_REQUESTED). Pending/running stages may have been interrupted; empty results do not prove absence.
Empty graph results do not prove absence; verify current source before acting.
```


## JSX reference coverage

TSX and JSX uppercase identifier tags with direct runtime named imports or direct
module-local function/variable bindings produce `JSX_USES`, separate from `CALLS`.
Import aliases and memo-wrapped component values are supported. Imported targets
must be direct named exports; type-only imports, lexical shadows, lowercase
intrinsics, unbound tags, default imports, namespace/member tags and unsupported
re-exports remain unresolved. This is syntax evidence, not a proof that a value
is a valid React component or that React invokes it at runtime.

The local graph resolver uses exact indexed module/export identity. Remote
Neo4j imported JSX resolution abstains; it does not guess by globally unique
names. Existing graphs without a JSX table remain readable; reindexing or an
explicit refresh with this extractor is needed to obtain JSX edges. The extractor
schema fingerprint advances so ordinary incremental indexing re-extracts unchanged
source after the upgrade. No automatic indexing is started.

Unknown Effect service receivers such as `outbox.claimNext` and
`gitCore.statusDetailsRemote` remain unresolved. No new object-call heuristic is
added: these concrete receivers are runtime service bindings, and syntax alone
does not prove their implementation. `Math.max`, `Math.min`, and `Effect.exit`
continue to avoid same-name unrelated local targets.

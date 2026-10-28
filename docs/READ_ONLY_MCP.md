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
   dependencies and `get_stats` for graph counts. These are the ten exposed tools.
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
known, and limits/truncation. Unknown facts are null/unknown. Freshness remains
unknown: Git HEAD equality and watcher liveness are insufficient. Receipt evidence
is observed after the query and does not promise an atomic source snapshot.

The server still negotiates `2024-11-05`. That version's official
[Result schema](https://github.com/modelcontextprotocol/specification/blob/2024-11-05/schema/schema.ts)
allows `_meta`; newer `structuredContent` is not assumed. Essential evidence also
appears in text for clients that discard metadata. `diagnose` retains versioned JSON text without a prose footer; in read-only mode
it adds `analysis_evidence` containing the same scope and coverage warnings. No full graph is serialized twice.

Example text (project paths are illustrative):

```text
No callers found in the current graph for 'src/api.py::helper'

Project: /workspace/topic-checkout
Freshness: unknown. Warning: partial or unknown analysis coverage: scip-python Skipped (NOT_REQUESTED). Pending/running stages may have been interrupted; empty results do not prove absence.
Empty graph results do not prove absence; verify current source before acting.
```

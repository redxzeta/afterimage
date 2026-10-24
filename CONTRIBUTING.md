# Contributing to Infigraph

Thank you for your interest in contributing!

## AI-assisted contributions

If you're using Claude Code, see [`CLAUDE.md`](CLAUDE.md) — architecture, build/test commands, and cross-cutting invariants, plus deeper guides under `.claude/skills/` (`code-indexing-pipeline`, `analysis-subsystems`, `review-pr-against-issue`). Cursor users get the same content via `.cursor/rules/*.mdc`, auto-loaded per file.

## Prerequisites

- Rust stable (via [rustup](https://rustup.rs/))
- `cmake` — required by the graph database
  - macOS: `brew install cmake`
  - Linux: `sudo apt install cmake`

## Building

```bash
cargo build --release -p infigraph-cli -p infigraph-mcp
```

## Running Tests

For the Afterimage fork, use Rust **1.98.1** and the same entry point as PR CI:

```bash
python3 scripts/verify.py
```

This runs formatting, locked workspace Clippy/typechecking, a CLI/MCP build,
small smoke tests, and compatibility tests selected from changes against
`origin/afterimage-main` (including local changes). See [CI coverage](docs/CI.md)
for prerequisites, path rules, timings and the retained broad validation.
Use `--base <commit>` to reproduce an exact comparison and `--dry-run` to inspect it.
Missing base history expands checks. The complete default-feature suite remains:

```bash
python3 scripts/verify.py --full
```

## Code Style

```bash
cargo fmt --all
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Both are enforced in CI. Optional remote/kompress features need separate checks
when changed; default-feature tests do not establish their runtime compatibility.

## Adding a Language

**Tree-sitter path** — add a `tree-sitter-<lang>` crate dependency and write two query files:
- `crates/infigraph-languages/languages/<lang>/entities.scm` — symbols
- `crates/infigraph-languages/languages/<lang>/relations.scm` — call edges, imports

**Grammar plugin path** — for languages without a tree-sitter grammar, write ANTLR `.g4` grammars and a Java extractor. See `GRAMMAR_PLUGINS.md` for a full walkthrough.

## Submitting a PR

1. For Afterimage, branch from and target `afterimage-main`; `main` is the upstream mirror (see [fork workflow](docs/FORK_WORKFLOW.md))
2. Write tests for your change
3. Run `python3 scripts/verify.py` and any additional checks needed for the changed behavior
4. Submit a pull request — the template will guide you through the checklist

## Reporting Bugs

Open an issue using the [bug report template](https://github.com/intuit/infigraph/issues/new?template=bug_report.md).

## License

By contributing, you agree that your contributions will be licensed under the [Apache 2.0 license](LICENSE).

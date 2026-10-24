# Afterimage fork workflow

`redxzeta/afterimage` uses two long-lived branches:

- `main` mirrors `intuit/infigraph:main`. Update it only by fast-forwarding
  upstream history; never merge fork changes or fork PRs into it.
- `afterimage-main` is the default fork development and integration branch.
  Branch from it and target fork PRs at it. Pending upstream changes integrated
  here do not imply that their upstream PRs have merged.

## Syncing upstream

From a clean, dedicated checkout (leave other worktrees alone), configure the
upstream remote once with `git remote add upstream https://github.com/intuit/infigraph.git`.
Then:

```sh
git fetch --no-tags upstream main
git fetch --no-tags origin
git merge-base --is-ancestor origin/main upstream/main
# Pushes only if the remote update is a fast-forward; never add --force.
git push origin upstream/main:refs/heads/main
git switch afterimage-main
git merge --ff-only origin/afterimage-main
git merge upstream/main
# Review any conflicts and run focused checks before pushing.
git push origin HEAD:refs/heads/afterimage-main
```

If the ancestry check fails, stop and investigate. Do not reset or force-push
`main`. If a branch is checked out in another worktree, use that branch's
dedicated checkout or a fresh integration branch instead of switching it here.

## Local build and bounded checks

Rust, CMake, a C/C++ toolchain, pkg-config and OpenSSL development files are
required. Offline builds also need the Cargo dependencies and native build
inputs cached beforehand. `--offline` prevents Cargo network access; native
build scripts may still require their own preseeded artifacts. Default builds
exclude optional `kompress`/ONNX Runtime; all-feature linting compiles it but
does not establish runtime inference compatibility.

```sh
export CARGO_BUILD_JOBS=1 CMAKE_BUILD_PARALLEL_LEVEL=1
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo build --locked --offline -p infigraph-cli -p infigraph-mcp
cargo fmt --all -- --check
cargo test --locked --offline -p infigraph-core --test embedding_format --test typescript_callers -- --test-threads=1
cargo test --locked --offline -p infigraph-cli --test doctor --test cli_parity -- --test-threads=1
cargo test --locked --offline -p infigraph-mcp --test diagnose --test coverage_reporting -- --test-threads=1
cargo test --locked --offline -p infigraph-mcp --lib web::tests:: -- --test-threads=1
python3 -m unittest discover -s scripts/benchmark -p 'test_*.py'
python3 scripts/test-mcp-supervisor.py
cargo clippy --locked --offline --all-targets --all-features -- -D warnings
git diff --check
```

Use direct candidate binaries with disposable HOME/registry/project fixtures
for application probes. SCIP enrichment can need separately installed tools;
`--no-embed` alone does not guarantee offline indexing. The benchmark harness
uses isolated copies; never benchmark against original source directories.

The inherited Actions filters still target `main`; hosted CI is not an
acceptance gate for this personal experimental integration. Local focused
checks are not full-suite acceptance. Known watcher failures (hidden ancestor
paths and duplicate watch locks), native WAL/transaction gaps, and incomplete
SCIP/full-repository graphs remain open. See [reliability evidence](PHASE-D-RELIABILITY.md)
and [benchmark evidence](../scripts/benchmark/results/2026-10-23.md).

## Release boundary

Do not release or publish this fork to `@intuit/*`. Existing package names and
installers still identify upstream Infigraph. Keep the
[fork release guards](FORK_RELEASE_SAFETY.md) when syncing. A source merge or
default-branch change authorizes no installation, deployment or runtime activation.

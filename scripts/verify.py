#!/usr/bin/env python3
"""Bounded PR verification; --full retains the complete default-feature suite."""
import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]
ALL = {"interfaces", "cross-repo", "core", "benchmark", "supervisor"}
CORE_TESTS = [
    "namespace_prefix", "namespace_link", "calls_service_edges",
    "group_build_steps", "combined_graph", "grpc_cross_service", "schema_parity",
]


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, stderr=subprocess.DEVNULL)


def changed_paths(base):
    """Include deletions, both sides of renames, and local uncommitted work."""
    try:
        ancestor = git("merge-base", base, "HEAD").decode().strip()
        paths = git("diff", "--no-renames", "--name-only", "-z", ancestor)
        paths += git("ls-files", "--others", "--exclude-standard", "-z")
        return {p.decode("utf-8", "surrogateescape") for p in paths.split(b"\0") if p}
    except subprocess.CalledProcessError:
        return None  # Missing history must expand checks, never skip them.


def selection(paths):
    if paths is None:
        return set(ALL), {"workspace"}
    suites, crates = set(), set()
    for path in paths:
        if path.startswith("scripts/benchmark/"):
            suites.add("benchmark")
        elif path.startswith("scripts/test-mcp-supervisor"):
            suites.add("supervisor")
        elif path.startswith("crates/"):
            crate = path.split("/")[1]
            crates.add(crate)
            if crate in {"infigraph-core", "infigraph-languages",
                         "infigraph-grammar-plugin", "infigraph-pipeline-plugin"}:
                suites.update({"core", "cross-repo", "interfaces"})
            elif crate in {"infigraph-cli", "infigraph-mcp"}:
                suites.update({"interfaces", "cross-repo"})
                if crate == "infigraph-mcp":
                    suites.add("supervisor")
            else:
                suites.update({"core", "interfaces"})
        elif path.startswith(("docs/", ".claude/", ".cursor/")) or path in {
            "README.md", "CONTRIBUTING.md", "CLAUDE.md", "AGENTS.md", "LICENSE",
            ".github/PULL_REQUEST_TEMPLATE.md",
        }:
            continue
        else:
            # Manifests, lockfiles, fixtures, grammars, CI itself and unknown
            # paths can affect the shared contract. Expand conservatively.
            suites.update(ALL)
            crates.add("workspace")
    return suites, crates


def test_command(package, targets=(), unit_filter=None):
    cmd = ["cargo", "test", "--locked", "-p", package]
    for target in targets:
        cmd += ["--test", target]
    if unit_filter is not None:
        cmd += ["--lib", unit_filter]
    return cmd + ["--", "--test-threads=1"]


def commands(paths, full=False):
    suites, crates = selection(paths)
    result = [
        ["python3", "-m", "unittest", "discover", "-s", "scripts", "-p", "test_verify.py"],
        ["cargo", "fmt", "--all", "--", "--check"],
        ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"],
        ["cargo", "build", "--locked", "-p", "infigraph-cli", "-p", "infigraph-mcp",
         "--bin", "infigraph", "--bin", "infigraph-mcp"],
    ]
    if full:
        result.append(["cargo", "test", "--locked", "--workspace", "--", "--test-threads=1"])
    else:
        result += [
            test_command("infigraph-core", ["embedding_format"]),
            test_command("infigraph-cli", ["cli_parity"]),
            test_command("infigraph-mcp", ["tool_parity"]),
        ]
        if "workspace" in crates:
            result.append(["cargo", "test", "--locked", "--workspace", "--lib", "--bins", "--", "--test-threads=1"])
        else:
            for crate in sorted(crates):
                if (ROOT / "crates" / crate / "src/lib.rs").exists():
                    result.append(test_command(crate, unit_filter=""))
                else:
                    result.append(["cargo", "test", "--locked", "-p", crate,
                                   "--bins", "--", "--test-threads=1"])
        if "interfaces" in suites:
            result += [test_command("infigraph-cli", ["doctor"]),
                       test_command("infigraph-mcp", ["diagnose", "coverage_reporting"]),
                       test_command("infigraph-mcp", unit_filter="web::tests::")]
        if "core" in suites:
            result.append(test_command("infigraph-core", ["typescript_callers"]))
            for group in ["diagnostics::tests", "index_status::tests", "scip::tests"]:
                result.append(test_command("infigraph-core", unit_filter=group))
        if "cross-repo" in suites:
            result.append(test_command("infigraph-core", CORE_TESTS))
        # Changed integration targets outside the fixed contract lists must run.
        for path in sorted(paths or []):
            parts = Path(path).parts
            if (len(parts) == 4 and parts[0] == "crates" and parts[2] == "tests"
                    and path.endswith(".rs") and (ROOT / path).exists()):
                cmd = test_command(parts[1], [Path(path).stem])
                if not any("--test" in existing and parts[1] in existing
                           and Path(path).stem in existing for existing in result):
                    result.append(cmd)
    if full or "benchmark" in suites:
        # Harness isolation needs Linux + Bubblewrap; it never runs target apps.
        if sys.platform == "linux":
            result += [["python3", "-c", "from pathlib import Path; "
                        "[compile(p.read_bytes(), str(p), 'exec') for p in Path('scripts/benchmark').glob('*.py')]"],
                       ["python3", "-m", "unittest", "discover", "-s", "scripts/benchmark", "-p", "test_*.py"]]
    if (full or "supervisor" in suites) and sys.platform == "linux":
        result.append(["python3", "scripts/test-mcp-supervisor.py"])
    result.append(["git", "diff", "--check"])
    return result


def main():
    os.environ["PYTHONDONTWRITEBYTECODE"] = "1"
    for key, value in {"CARGO_BUILD_JOBS": "2", "CMAKE_BUILD_PARALLEL_LEVEL": "2",
                       "CARGO_PROFILE_DEV_DEBUG": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
                       "CARGO_INCREMENTAL": "0"}.items():
        os.environ.setdefault(key, value)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=os.environ.get("VERIFY_BASE") or "origin/afterimage-main")
    parser.add_argument("--full", action="store_true", default=os.environ.get("VERIFY_FULL") == "1")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()
    paths = changed_paths(args.base)
    suites, _ = selection(paths)
    print("Verification: " + ("full workspace" if args.full else "focused + " + ", ".join(sorted(suites))), flush=True)
    if paths is None:
        print("Base unavailable; expanding to all compatibility suites.", flush=True)
    if not args.dry_run:
        version = subprocess.check_output(["rustc", "--version"], text=True).strip()
        print(version, flush=True)
        if version.split()[1] != "1.98.1":
            parser.error("verification requires Rust 1.98.1, matching CI; use a Rustup override")
        if sys.platform == "linux" and (args.full or "benchmark" in suites) and not shutil.which("bwrap"):
            parser.error("selected benchmark isolation checks require Bubblewrap (bwrap)")
    for cmd in commands(paths, args.full):
        print("+ " + shlex.join(cmd), flush=True)
        if not args.dry_run:
            start = time.monotonic()
            subprocess.run(cmd, cwd=ROOT, check=True)
            print(f"PASS ({time.monotonic() - start:.1f}s)", flush=True)


if __name__ == "__main__":
    try:
        main()
    except subprocess.CalledProcessError as error:
        sys.exit(error.returncode if error.returncode > 0 else 1)

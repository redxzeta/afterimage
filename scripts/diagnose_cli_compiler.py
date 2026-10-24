#!/usr/bin/env python3
"""Failure-only actual-CLI control; original verification stays fatal."""
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import tempfile

from diagnose_cli import bounded_command, fingerprint, inspect, lbug_builds


def compile_command(plan):
    # Recompile the SAME integration target/feature context, without running
    # other suites. The original unchanged CLI parity executable runs below.
    selected = next((cmd for cmd in plan if "--test" in cmd and "cli_parity" in cmd), None)
    if selected is None:
        raise ValueError("primary integration context unavailable")
    return selected[:selected.index("--")] + ["--offline", "--no-run"]


def integration_compile_command():
    import verify
    paths = verify.changed_paths(os.environ.get("VERIFY_BASE") or "origin/afterimage-main")
    return compile_command(verify.commands(paths, full=os.environ.get("VERIFY_FULL") == "1"))


def feature_signature(target):
    return sorted({json.loads(path.read_text())["features"]
                   for path in (target / "debug/.fingerprint").glob("lbug-*/lib-lbug.json")})


def stdlib_identity(executable, label):
    ldd = shutil.which("ldd")
    if ldd:
        result = bounded_command([ldd, str(executable)], 10)
        match = re.search(r"libstdc\+\+\.so\.[^ ]+\s+=>\s+(\S+)", result.stdout)
        if result.returncode == 0 and match:
            return fingerprint(label, Path(match[1]))
    return None


def cleanup_watchers(target):
    # Successful parity indexing may detach a watcher. Stop only this target's
    # watcher whose disposable /tmp fixture has already been deleted.
    stopped = 0
    for proc in Path("/proc").iterdir():
        if not proc.name.isdigit():
            continue
        try:
            if (proc / "exe").resolve() != target / "debug/infigraph":
                continue
            args = (proc / "cmdline").read_bytes().split(b"\0")
            cwd = os.readlink(proc / "cwd")
            if args[1:2] == [b"watch"] and cwd.startswith("/tmp/") and cwd.endswith(" (deleted)"):
                os.kill(int(proc.name), signal.SIGTERM)
                stopped += 1
        except OSError:
            continue
    print(f"REAL_CLI_CONTROL owned deleted-fixture watchers stopped={stopped}", flush=True)


def run(target):
    compiler = shutil.which("g++-13")
    cli = target / "debug/infigraph"
    tests = [p for p in (target / "debug/deps").glob("cli_parity-*")
             if p.is_file() and os.access(p, os.X_OK)]
    builds = lbug_builds(target)
    libraries = {Path(meta["LBUG_PRECOMPILED_LIBRARY_DIR"])
                 for _, meta in builds if "LBUG_PRECOMPILED_LIBRARY_DIR" in meta}
    if not compiler or not cli.is_file() or len(tests) != 1 or len(libraries) != 1:
        raise ValueError("need installed GCC13, original CLI, one parity executable and native directory")
    if (os.environ.get("LBUG_BUILD_FROM_SOURCE") or any(
            not meta.get("LBUG_PRECOMPILED_SOURCE", "").startswith(("release:", "run:"))
            for _, meta in builds)):
        raise ValueError("prebuilt native context required; no native-source rebuild permitted")
    library = libraries.pop()
    native = [library / name for name in ["liblbug.a", "lbug.h", "lbug.hpp"]]
    if not all(path.is_file() for path in native):
        raise ValueError("original native archive/headers unavailable")
    before_features = feature_signature(target)
    before_stdlib = stdlib_identity(cli, "ORIGINAL linked libstdc++")
    if not before_features or not before_stdlib:
        raise ValueError("original feature/runtime-library identity unavailable")
    build_command = integration_compile_command()
    # Keep binaries inside the job until runner disposal; never overwrite the
    # originals. No databases, binaries or machine-state uploads are performed.
    required = cli.stat().st_size + tests[0].stat().st_size + 2 * 1024**3
    if shutil.disk_usage(target).free < required:
        raise ValueError("insufficient room to preserve original executables and relink")
    snapshots = target / "ci-native-control"
    snapshots.mkdir(exist_ok=True)
    saved = Path(tempfile.mkdtemp(prefix="original-", dir=snapshots))
    for name, source in [("infigraph", cli), ("cli_parity", tests[0])]:
        shutil.copy2(source, saved / name)
        if fingerprint(f"ORIGINAL {name}", source) != fingerprint(f"PRESERVED {name}", saved / name):
            raise ValueError("original executable copy differs")
    for i, (output, _) in enumerate(builds):
        for bridge in (output.parent / "out").glob("liblbug*.a"):
            shutil.copy2(bridge, saved / f"bridge-{i}-{bridge.name}")
    before_native = [fingerprint(f"ORIGINAL {path.name}", path) for path in native]
    print(f"REAL_CLI_CONTROL before_features={before_features!r}; original binaries preserved", flush=True)
    test_command = [str(saved / "cli_parity"), "--test-threads=1"]
    original = bounded_command(test_command, 60)
    print(original.stdout, original.stderr, sep="\n", flush=True)
    print(f"REAL_CLI_CONTROL original_test_exit={original.returncode}", flush=True)
    diagnostic = original.stdout + original.stderr
    if original.returncode == 0 or not all(
            token in diagnostic for token in ["index failed:", "unix_signal: Some(11)"]):
        print("REAL_CLI_CONTROL NOT_REPRODUCED: unchanged test must reproduce index SIGSEGV before rebuild",
              flush=True)
        return 1
    version = bounded_command([compiler, "--version"], 5)
    print(f"REAL_CLI_CONTROL compiler={version.stdout.splitlines()[:1]}", flush=True)
    print(f"REAL_CLI_CONTROL compile_command={build_command!r}; only build CXX overridden", flush=True)
    build = bounded_command(build_command, 180, env=dict(os.environ, CXX=compiler))
    print(build.stdout, build.stderr, sep="\n", flush=True)
    print(f"REAL_CLI_CONTROL compile_exit={build.returncode}", flush=True)
    if build.returncode:
        return build.returncode
    inspect(target)
    after_features = feature_signature(target)
    after_native = [fingerprint(f"CONTROL {path.name}", path) for path in native]
    after_stdlib = stdlib_identity(cli, "CONTROL linked libstdc++")
    unchanged = (before_features == after_features and before_native == after_native
                 and before_stdlib == after_stdlib)
    print(f"REAL_CLI_CONTROL after_features={after_features!r}; native/features/runtime equal={unchanged}",
          flush=True)
    # CARGO_BIN_EXE is compiled into the preserved original test executable.
    # It still names target/debug/infigraph, now rebuilt with controlled CXX.
    # Do not pass the compile-only CXX override into the test's runtime env.
    result = bounded_command(test_command, 60)
    print(result.stdout, result.stderr, sep="\n", flush=True)
    print(f"REAL_CLI_CONTROL preserved_test_exit={result.returncode}; primary failure retained", flush=True)
    if not unchanged:
        print("REAL_CLI_CONTROL INCOMPLETE: native/feature/runtime inputs changed", flush=True)
        return 2
    return result.returncode


def main():
    target = Path(os.environ.get("CARGO_TARGET_DIR", "target")).resolve()
    try:
        return run(target)
    except subprocess.TimeoutExpired as error:
        print(f"REAL_CLI_CONTROL INCOMPLETE: timeout; group killed; "
              f"stdout={error.stdout!r} stderr={error.stderr!r}", flush=True)
        return 124
    except (OSError, ValueError, KeyError) as error:
        print(f"REAL_CLI_CONTROL NOT_RUN/INCOMPLETE: {error}", flush=True)
        return 1
    finally:
        cleanup_watchers(target)


if __name__ == "__main__":
    raise SystemExit(main())

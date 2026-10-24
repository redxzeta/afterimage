#!/usr/bin/env python3
"""Failure-only native CLI evidence; never builds, installs, or changes tests."""
import argparse
import hashlib
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import tempfile


def lbug_builds(target):
    return [(output, dict(line.removeprefix("cargo:rustc-env=").split("=", 1)
                         for line in output.read_text().splitlines()
                         if line.startswith("cargo:rustc-env=LBUG_PRECOMPILED_")))
            for output in sorted((target / "debug/build").glob("lbug-*/output"))]


def compiler_tags(label, archive):
    reader = shutil.which("readelf")
    if not reader or not archive.is_file():
        print(f"{label} compiler tags: NOT_RUN (readelf or archive unavailable)", flush=True)
        return
    try:
        result = subprocess.run([reader, "-p", ".comment", str(archive)],
                                capture_output=True, text=True, timeout=10, check=False)
        tags = sorted(set(re.findall(r"\]\s+((?:GCC:|.*clang version).*)", result.stdout)))
        print(f"{label} compiler tags: status={result.returncode}, tags={tags}", flush=True)
    except subprocess.TimeoutExpired:
        print(f"{label} compiler tags: NOT_RUN (timeout)", flush=True)


def fingerprint(label, path):
    if not path.is_file():
        print(f"{label}: MISSING", flush=True)
        return
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    print(f"{label}: bytes={path.stat().st_size} sha256={digest.hexdigest()}", flush=True)
    return digest.hexdigest()


def inspect(target):
    print(f"Platform: {platform.system()} {platform.release()} {platform.machine()}", flush=True)
    print(f"libc: {platform.libc_ver()}", flush=True)
    cli = target / "debug/infigraph"
    print(f"CLI exists: {cli.is_file()}; executable: {os.access(cli, os.X_OK)}", flush=True)
    fingerprint("CLI", cli)
    builds = lbug_builds(target)
    print(f"lbug build metadata candidates: {len(builds)} (may include stale builds)", flush=True)
    for i, (output, metadata) in enumerate(builds):
        for bridge in sorted((output.parent / "out").glob("liblbug*.a")):
            compiler_tags(f"lbug candidate {i} bridge {bridge.name}", bridge)
        print(f"lbug candidate {i} source: "
              f"{metadata.get('LBUG_PRECOMPILED_SOURCE', 'UNKNOWN')}", flush=True)
        directory = metadata.get("LBUG_PRECOMPILED_LIBRARY_DIR")
        if not directory:
            print("Native library directory: NOT_RECORDED", flush=True)
            continue
        library = Path(directory)
        for name in ["liblbug.a", "lbug.h", "lbug.hpp"]:
            fingerprint(f"lbug candidate {i} {name}", library / name)
        compiler_tags(f"lbug candidate {i} native archive", library / "liblbug.a")
        ar = shutil.which("ar")
        if ar and (library / "liblbug.a").is_file():
            version = subprocess.run([ar, "p", str(library / "liblbug.a"), "version.cpp.o"],
                                     capture_output=True, timeout=10, check=False)
            versions = sorted(set(v.decode() for v in re.findall(
                rb"(?<![0-9.])0\.[0-9]+\.[0-9]+(?![0-9.])", version.stdout)))
            print(f"Native version object: status={version.returncode}, versions={versions}", flush=True)
    return cli


def debugger():
    candidates = [shutil.which("gdb"), shutil.which("lldb")]
    candidates += [str(p) for pattern in ["/usr/bin/lldb-[0-9]*", "/usr/lib/llvm-*/bin/lldb",
                                          "/usr/share/swift/usr/bin/lldb",
                                          "/usr/local/swift/usr/bin/lldb", "/opt/swift/usr/bin/lldb"]
                   for p in sorted(Path("/").glob(pattern.lstrip("/")))]
    available = []
    for candidate in dict.fromkeys(candidates):
        # lldb-dap/server/instr/argdumper also accept --version and may return
        # success while ignoring batch commands. Only the debugger CLI can run
        # this probe; a helper's exit zero is not a backtrace receipt.
        if (candidate and re.fullmatch(r"gdb|lldb(?:-[0-9]+)?", Path(candidate).name)
                and os.access(candidate, os.X_OK)):
            try:
                result = subprocess.run([candidate, "--version"], capture_output=True,
                                        text=True, timeout=5, check=False)
                print(f"Debugger {Path(candidate).name}: status={result.returncode}; "
                      f"{result.stdout.splitlines()[:1]}", flush=True)
                if result.returncode == 0:
                    available.append(candidate)
            except (OSError, subprocess.TimeoutExpired) as error:
                print(f"Debugger {Path(candidate).name}: {type(error).__name__}", flush=True)
    print(f"Usable debugger count: {len(available)}; no installation attempted", flush=True)
    selected = available[0] if available else None
    print(f"Selected debugger CLI: {Path(selected).name if selected else 'NONE'}", flush=True)
    return selected


def bounded_command(command, timeout, env=None):
    def child_limits():
        import resource  # Only the Linux failure probe executes this child hook.
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               text=True, start_new_session=True, preexec_fn=child_limits, env=env)
    try:
        stdout, stderr = process.communicate(timeout=timeout)
    except subprocess.TimeoutExpired as error:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        error.stdout, error.stderr = process.communicate()
        raise
    return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)


def compare_compilers(target):
    """One synthetic constructor/format probe, two installed compilers, same archive.

    This is diagnostic evidence only, separate from CLI acceptance. It exercises
    weak std::format definitions emitted by the bridge and prebuilt library.
    """
    compilers = [shutil.which(name) for name in ["g++-13", "g++-15"]]
    libraries = {Path(meta["LBUG_PRECOMPILED_LIBRARY_DIR"])
                 for _, meta in lbug_builds(target) if "LBUG_PRECOMPILED_LIBRARY_DIR" in meta}
    if not all(compilers) or len(libraries) != 1:
        print("COMPILER_COMPARISON NOT_RUN: need installed g++-13/g++-15 and one native directory",
              flush=True)
        return
    library = libraries.pop()
    if not all((library / name).is_file() for name in ["lbug.hpp", "liblbug.a"]):
        print("COMPILER_COMPARISON NOT_RUN: native archive/header unavailable", flush=True)
        return
    source = '''#include "lbug.hpp"
#include <format>
#include <iostream>
int main(int argc, char** argv) {
    if (argc != 2) return 2;
    std::cout << "cplusplus=" << __cplusplus << " glibcxx=" << __GLIBCXX__
              << " cxx11_abi=" << _GLIBCXX_USE_CXX11_ABI
              << " format_arg_bytes=" << sizeof(std::basic_format_arg<std::format_context>)
              << std::endl;
    std::cout << std::format("probe {}", std::string("ready")) << std::endl;
    lbug::main::Database db(argv[1]);
    std::cout << "database opened" << std::endl;
}
'''
    with tempfile.TemporaryDirectory(prefix="native-format-control-") as fixture:
        root = Path(fixture)
        (root / "probe.cpp").write_text(source)
        for compiler in compilers:
            name = Path(compiler).name
            executable = root / name
            command = [compiler, "-std=c++20", "-O0", "-I", str(library), str(root / "probe.cpp"),
                       "-Wl,--allow-multiple-definition", "-Wl,--whole-archive",
                       str(library / "liblbug.a"), "-Wl,--no-whole-archive", "-lssl", "-lcrypto",
                       "-lpthread", "-ldl", "-o", str(executable)]
            try:
                version = bounded_command([compiler, "--version"], 5)
                print(f"COMPILER_COMPARISON {name} version={version.stdout.splitlines()[:1]}", flush=True)
                build = bounded_command(command, 30)
                print(f"COMPILER_COMPARISON {name} build_exit={build.returncode}", flush=True)
                if build.returncode:
                    print(build.stderr[-2000:], flush=True)
                    continue
                result = bounded_command([str(executable), str(root / f"database-{name}")], 10)
                child_signal = -result.returncode if result.returncode < 0 else None
                print(f"COMPILER_COMPARISON {name} run_exit={result.returncode} signal={child_signal} "
                      f"stdout={result.stdout!r} stderr={result.stderr!r}", flush=True)
            except subprocess.TimeoutExpired as error:
                print(f"COMPILER_COMPARISON {name} INCOMPLETE: timeout; process group killed; "
                      f"stdout={error.stdout!r} stderr={error.stderr!r}", flush=True)
            except OSError as error:
                print(f"COMPILER_COMPARISON {name} NOT_RUN: {type(error).__name__}", flush=True)
    print("Compiler comparison is not a CLI test pass or an automatic environment fix.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inspect-only", action="store_true")
    parser.add_argument("--compare-compilers", action="store_true")
    args = parser.parse_args()
    target = Path(os.environ.get("CARGO_TARGET_DIR", "target")).resolve()
    cli = inspect(target)
    if args.compare_compilers:
        compare_compilers(target)
    tool = debugger()
    if args.inspect_only:
        return
    if not cli.is_file() or not os.access(cli, os.X_OK):
        raise SystemExit("NOT_RUN: CLI unavailable; original verification failure retained")
    if tool is None:
        raise SystemExit("NOT_RUN: neither usable gdb nor lldb; original verification failure retained")
    with tempfile.TemporaryDirectory() as fixture:
        root = Path(fixture)
        (root / "src").mkdir()
        (root / "src/calculator.py").write_text("def add(left, right):\n    return left + right\n")
        child = [str(cli), "--root", fixture, "index", "--no-embed"]
        if Path(tool).name == "gdb":
            command = [tool, "--batch", "--nx", "--return-child-result",
                       "-ex", "set pagination off", "-ex", "set disable-randomization off",
                       "-ex", "run", "-ex", "bt 30", "-ex", "thread apply all bt 8", "--args"]
        else:
            command = [tool, "--batch", "--no-lldbinit",
                       "-o", "settings set target.disable-aslr false", "-o", "run",
                       "-k", "thread backtrace all -c 30", "--"]
        print("Fresh parity fixture under debugger (90s bound); primary check remains fatal.", flush=True)
        try:
            result = subprocess.run(command + child, timeout=90, check=False)
        except subprocess.TimeoutExpired:
            raise SystemExit("Diagnostic timed out; original verification failure retained")
        print(f"Debugger return code: {result.returncode}; not a test pass receipt", flush=True)
        raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()

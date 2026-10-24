#!/usr/bin/env python3
"""Linux/Unix process regressions for the real std-only supervisor module.

Requires rustc and Python 3; does not link/open a native graph database.
Every worker, CLI, HOME, registry and graph sentinel belongs to a temp fixture.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import resource
import shutil
import time
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "crates/infigraph-mcp/src/supervisor.rs"


def hashes(path):
    return {str(p.relative_to(path)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in path.rglob("*") if p.is_file()}


def limits():
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024**2, 512 * 1024**2))


def run_case(binary, base, name, *, always_crash=False, cli_exit=0,
             indexed=True, symlink=False, backend="kuzu", signal=11):
    case = base / name
    active = case / "active"
    unrelated = case / "unrelated"
    home = case / "home"
    group = home / ".infigraph/groups/untouched"
    for project in (active, unrelated, group):
        (project / ".infigraph").mkdir(parents=True)
        for artifact in ("graph", "graph.wal", "docs.kuzu", "docs_embeddings.bin", "sessions.txt"):
            (project / ".infigraph" / artifact).write_bytes(b"disposable-sentinel")
    registry = home / ".infigraph/registry.json"
    registry.write_text(json.dumps({"repos": {"other": {"path": str(unrelated)}}, "groups": {}}))
    if not indexed or symlink:
        for p in (active / ".infigraph").iterdir():
            p.unlink()
        (active / ".infigraph").rmdir()
    if symlink:
        (active / ".infigraph").symlink_to(unrelated / ".infigraph", target_is_directory=True)
    before_other = hashes(unrelated)
    before_group = hashes(group)
    registry_before = registry.read_bytes()
    active_before = hashes(active)
    count, calls = case / "workers", case / "calls"
    env = {**os.environ, "HOME": str(home), "INFIGRAPH_REGISTRY_HOME": str(home),
           "INFIGRAPH_BACKEND": backend, "TEST_COUNT": str(count), "TEST_CALLS": str(calls),
           "TEST_ALWAYS_CRASH": str(int(always_crash)), "TEST_CLI_EXIT": str(cli_exit),
           "TEST_SIGNAL": str(signal)}
    result = subprocess.run([str(binary), "--mcp", "--port=31415"], cwd=active, env=env,
                            input=b"client-request\n", capture_output=True, timeout=10,
                            preexec_fn=limits)
    expected_attempts = 2 if signal == 11 else 1
    assert len(count.read_text().splitlines()) == expected_attempts, result.stderr
    assert result.returncode == (1 if always_crash or signal != 11 else 0), result.stderr
    assert result.stdout == b"", result.stdout  # CLI output never enters MCP transport.
    expected_reindex = signal == 11 and indexed and not symlink and backend == "kuzu"
    assert calls.exists() == expected_reindex
    if expected_reindex:
        assert calls.read_text().splitlines() == [str(active), "index", "1"]
    if expected_reindex and cli_exit == 0:
        assert (active / ".infigraph/graph").read_bytes() == b"recovered"
        active_before[".infigraph/graph"] = hashlib.sha256(b"recovered").hexdigest()
    assert hashes(active) == active_before, name
    assert hashes(unrelated) == before_other, name
    assert hashes(group) == before_group, name
    assert registry.read_bytes() == registry_before, name
    print(f"PASS {name}: workers={expected_attempts}, reindex={int(expected_reindex)}, exit={result.returncode}")



def run_native(binary_source, base, injection, cli_source, *, baseline=False):
    case = base / ("native-baseline" if baseline else "native-fixed")
    active, other, home = case / "active", case / "other", case / "home"
    group = home / ".infigraph/groups/other"
    for project in (active, other, group):
        (project / ".infigraph").mkdir(parents=True)
        for artifact in ("graph", "graph.wal", "docs.kuzu", "docs_embeddings.bin", "sessions.txt"):
            (project / ".infigraph" / artifact).write_bytes(b"disposable-sentinel")
    repos = {p.name: {"name": p.name, "path": str(p), "languages": [],
                      "symbol_count": 1, "module_count": 1} for p in (active, other)}
    registry = home / ".infigraph/registry.json"
    registry.write_text(json.dumps({"repos": repos, "groups": {}}))
    before = [hashes(p) for p in (active, other, group)]
    registry_before = registry.read_bytes()
    binary = case / "bin/infigraph-mcp"
    binary.parent.mkdir()
    shutil.copy2(binary_source, binary)
    native_cli = binary.with_name("infigraph")
    native_cli.write_text(cli_source.read_text().replace("if read -r request; then exit 9; fi\n", ""))
    native_cli.chmod(0o700)
    count, calls = case / "injections", case / "calls"
    env = {**os.environ, "HOME": str(home), "INFIGRAPH_REGISTRY_HOME": str(home),
           "INFIGRAPH_BACKEND": "kuzu", "RAYON_NUM_THREADS": "2",
           "LD_PRELOAD": str(injection), "TEST_INJECTION_COUNT": str(count),
           "TEST_CALLS": str(calls), "TEST_CLI_EXIT": "1"}
    def native_limits():
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        resource.setrlimit(resource.RLIMIT_AS, (8 * 1024**3, 8 * 1024**3))
    proc = subprocess.Popen([str(binary), "--mcp"], cwd=active, env=env,
                            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, preexec_fn=native_limits)
    try:
        deadline = time.monotonic() + 10
        while proc.poll() is None and time.monotonic() < deadline:
            if count.exists() and int(count.read_text()) >= 5:
                break
            time.sleep(.02)
        proc.stdin.close()
        proc.stdin = None
        stdout, stderr = proc.communicate(timeout=5)
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()
    starts = int(count.read_text())
    records = calls.read_text().splitlines()
    assert starts == (5 if baseline else 2), stderr
    assert len(records) == (36 if baseline else 3), records
    assert proc.returncode == (0 if baseline else 1), stderr
    assert registry.read_bytes() == registry_before
    if baseline:
        assert not (other / ".infigraph/graph").exists()
        assert not (group / ".infigraph/docs.kuzu").exists()
    else:
        assert [hashes(p) for p in (active, other, group)] == before
        assert stdout == b"", stdout
    print(f"PASS native {'baseline reproduction' if baseline else 'containment'}: "
          f"workers={starts}, reindexes={len(records)//3}, exit={proc.returncode}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native-mcp", type=Path, help="Also test a built fixed Linux MCP binary")
    parser.add_argument("--baseline-mcp", type=Path, help="Also reproduce unsafe baseline behavior")
    options = parser.parse_args()
    if os.name != "posix":
        raise SystemExit("This signal-level harness requires Unix; Windows remains unverified.")
    with tempfile.TemporaryDirectory(prefix="infigraph-supervisor-") as tmp:
        base = Path(tmp)
        source = base / "worker.rs"
        # Only the worker is synthetic. Supervision/recovery is the production module.
        source.write_text('''#[path = MODULE] mod supervisor;
fn log(_: &str, _: &str) {}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--worker") {
        assert_eq!(&args[2..], &["--mcp", "--port=31415"]);
        let path = std::env::var("TEST_COUNT").unwrap();
        let mut value = std::fs::read_to_string(&path).unwrap_or_default();
        value.push_str("worker\\n"); std::fs::write(path, &value).unwrap();
        if value.lines().count() == 1 || std::env::var("TEST_ALWAYS_CRASH").unwrap() == "1" {
            unsafe extern "C" { fn signal(sig:i32,handler:usize)->usize; fn raise(sig:i32)->i32; }
            let sig = std::env::var("TEST_SIGNAL").unwrap().parse().unwrap();
            unsafe { signal(sig,0); raise(sig); }
        }
    } else { std::process::exit(supervisor::run(&args, std::env::var_os("HOME").map(std::path::PathBuf::from).as_deref(), log).unwrap()); }
}
'''.replace("MODULE", json.dumps(str(MODULE))))
        binary = base / "infigraph-mcp"
        subprocess.run(["rustc", "--edition=2021", str(source), "-o", str(binary)], check=True, timeout=60)
        cli = base / "infigraph"
        cli.write_text('''#!/bin/sh
printf '%s\\n' "$PWD" "$*" "$INFIGRAPH_NO_WATCH" >> "$TEST_CALLS"
# A failed reindex must retain all existing files. Successful index recovery
# is modeled here; native corrupt-store rebuilding is a separate integration test.
if read -r request; then exit 9; fi
printf 'CLI progress must not enter MCP stdout\\n'
if [ "$TEST_CLI_EXIT" = 0 ]; then printf recovered > .infigraph/graph; fi
exit "$TEST_CLI_EXIT"
''')
        cli.chmod(0o700)
        run_case(binary, base, "recover-and-restart")
        run_case(binary, base, "repeated-crash-is-bounded", always_crash=True)
        run_case(binary, base, "failed-reindex-preserves-index", cli_exit=1)
        run_case(binary, base, "no-launch-index", indexed=False)
        run_case(binary, base, "symlink-index", symlink=True)
        run_case(binary, base, "remote-backend", backend="neo4j")
        run_case(binary, base, "sigill-does-not-trigger-reindex", signal=4)
        run_case(binary, base, "sigterm-does-not-trigger-reindex", signal=15)
        tests = base / "supervisor-tests"
        subprocess.run(["rustc", "--edition=2021", "--test", str(MODULE), "-o", str(tests)], check=True, timeout=60)
        subprocess.run([str(tests), "--test-threads=1"], check=True, timeout=10, preexec_fn=limits)
        if options.native_mcp or options.baseline_mcp:
            if not sys.platform.startswith("linux"):
                raise SystemExit("Native LD_PRELOAD fault injection requires Linux.")
            injection_source, injection = base / "inject.c", base / "inject.so"
            injection_source.write_text(r'''
#define _GNU_SOURCE
#include <unistd.h>
#include <dlfcn.h>
#include <stdlib.h>
#include <fcntl.h>
#include <stdio.h>
ssize_t read(int fd, void *buffer, size_t size) {
    static ssize_t (*next_read)(int,void*,size_t);
    static int checked;
    if (!next_read) next_read=dlsym(RTLD_NEXT,"read");
    if (fd==0 && !checked) {
        checked=1;
        const char *path=getenv("TEST_INJECTION_COUNT");
        if (path) {
            int file=open(path,O_RDWR|O_CREAT,0600); char text[32]={0};
            next_read(file,text,31); int count=atoi(text)+1;
            lseek(file,0,SEEK_SET); int len=snprintf(text,32,"%d\n",count); write(file,text,len); close(file);
            if (count<=4) { volatile int *bad=(int*)1; *bad=123; }
        }
    }
    return next_read(fd,buffer,size);
}
''')
            subprocess.run(["cc", "-shared", "-fPIC", "-O0", str(injection_source),
                            "-o", str(injection), "-ldl"], check=True, timeout=30)
            if options.baseline_mcp:
                run_native(options.baseline_mcp.resolve(), base, injection, cli, baseline=True)
            if options.native_mcp:
                run_native(options.native_mcp.resolve(), base, injection, cli)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Serialized, offline Infigraph A/B benchmark. Python 3.11+ stdlib only."""
import argparse
import fcntl
import hashlib
import io
import json
import math
import os
from pathlib import Path, PurePosixPath
import platform
import re
import resource
import select
import shutil
import signal
import statistics
import subprocess
import tarfile
import tempfile
import time

SCHEMA = "afterimage-benchmark-v1"
SHA = re.compile(r"[0-9a-f]{40}")
CODE = {".ts", ".tsx", ".mts", ".cts", ".js", ".jsx", ".mjs", ".cjs", ".rs", ".py", ".go"}
BUILDERS = {"cargo", "rustc", "cc1", "cc1plus", "cmake", "ninja", "make", "gcc", "g++", "clang", "clang++", "ld", "ld.lld"}


def active_builds():
    found = set()
    for p in Path("/proc").glob("[0-9]*/comm"):
        try:
            name = p.read_text().strip()
            if name in BUILDERS:
                found.add(name)
        except OSError:
            pass
    return sorted(found)


def canonical(value):
    return json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def digest(path):
    h = hashlib.sha256()
    with open(path, "rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def git(repo, *args):
    # Never run hooks, checkout, fetch, install, or project commands in the source repo.
    env = {"PATH": "/usr/bin:/bin", "HOME": "/nonexistent", "GIT_CONFIG_NOSYSTEM": "1",
           "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_OPTIONAL_LOCKS": "0", "LC_ALL": "C"}
    return subprocess.check_output(["git", "-C", str(repo), *args], env=env, stderr=subprocess.PIPE)


def pinned(repo, commit):
    if not SHA.fullmatch(commit):
        raise ValueError("use a full explicit 40-character commit SHA")
    if git(repo, "rev-parse", "--verify", commit + "^{commit}").decode().strip() != commit:
        raise ValueError("commit does not resolve exactly")


def safe_path(name):
    p = PurePosixPath(name)
    if p.is_absolute() or ".." in p.parts or not p.parts:
        raise ValueError("unsafe fixture path")
    return p


def source_file(name):
    p = safe_path(name)
    if any(part in {".git", ".infigraph", "node_modules", "target"} for part in p.parts):
        return False
    return (p.suffix in CODE or p.name in {"package.json", "Cargo.toml", "go.mod", ".gitignore",
            ".infigraphignore", "pnpm-workspace.yaml"} or
            (p.name.startswith("tsconfig") and p.suffix == ".json"))


def export(repo, commit, destination, paths=None):
    """Copy tracked code/config only; ignore symlinks, submodules, reports and local state."""
    pinned(repo, commit)
    count = 0
    with tarfile.open(fileobj=io.BytesIO(git(repo, "archive", "--format=tar", commit))) as archive:
        for member in archive:
            if not member.isfile() or not source_file(member.name):
                continue
            if paths is not None and member.name not in paths:
                continue
            path = destination / str(safe_path(member.name))
            path.parent.mkdir(parents=True, exist_ok=True)
            with archive.extractfile(member) as source, open(path, "wb") as target:
                shutil.copyfileobj(source, target)
            count += 1
    return count


def verify_truth(repo, truth):
    pinned(repo, truth["commit"])
    anchors = {a["id"]: a for a in truth["anchors"]}
    if len(anchors) != len(truth["anchors"]):
        raise ValueError("duplicate anchor id")
    cache = {}
    for anchor in anchors.values():
        path = str(safe_path(anchor["path"]))
        if path not in cache:
            cache[path] = git(repo, "show", truth["commit"] + ":" + path)
        content = cache[path]
        if hashlib.sha256(content).hexdigest() != anchor["file_sha256"]:
            raise ValueError("source digest mismatch: " + anchor["id"])
        lines = content.decode().splitlines()
        if not 1 <= anchor["line"] <= len(lines) or hashlib.sha256(lines[anchor["line"] - 1].encode()).hexdigest() != anchor["line_sha256"]:
            raise ValueError("source anchor mismatch: " + anchor["id"])
    ids = {s["id"] for s in truth["symbols"]}
    if len(ids) != len(truth["symbols"]):
        raise ValueError("duplicate symbol id")
    for item in truth["symbols"] + truth["relations"] + truth["questions"] + truth.get("imports", []):
        if not item["evidence"] or any(a not in anchors for a in item["evidence"]):
            raise ValueError("missing source evidence")
    for relation in truth["relations"]:
        if relation["from"] not in ids or relation["to"] not in ids:
            raise ValueError("relation endpoint missing from symbol truth")
        if relation["label"] not in {"positive", "negative"}:
            raise ValueError("unknown relation label")
    for question in truth["questions"]:
        if type(question["depth"]) is not int or not 1 <= question["depth"] <= 20:
            raise ValueError("impact depth must be an integer from 1 to 20")
        if question["target"] not in ids or any(s not in ids for s in question["expected"]):
            raise ValueError("question endpoint missing from symbol truth")
        for chain in question.get("chains", []):
            if chain[0] not in question["expected"] or chain[-1] != question["target"] or len(chain) - 1 > question["depth"]:
                raise ValueError("invalid source impact chain")
            for a, b in zip(chain, chain[1:]):
                if not any(r["from"] == a and r["to"] == b and r["label"] == "positive" for r in truth["relations"]):
                    raise ValueError("impact chain lacks source-labelled relation")
    return {"anchors": len(anchors), "symbols": len(ids), "relations": len(truth["relations"]),
            "questions": len(truth["questions"])}


def score(observed, positive, negative=()):
    """Open-world labels: an unlabelled observation is NEVER automatically false."""
    if observed is None:
        return {"status": "UNRESOLVED", "precision": None, "recall": None,
                "false_edges": None, "missing": None}
    observed, positive, negative = set(observed), set(positive), set(negative)
    if positive & negative:
        raise ValueError("contradictory truth")
    tp, fp = observed & positive, observed & negative
    return {"status": "SOURCE_LABELLED_SUBSET", "precision": None,
            "labelled_precision": len(tp) / (len(tp) + len(fp)) if tp or fp else None,
            "recall": len(tp) / len(positive) if positive else None,
            "matched": sorted(tp), "missing": sorted(positive - observed),
            "false_edges": sorted(fp), "unadjudicated": sorted(observed - positive - negative),
            "labelled_negative_absent": sorted(negative - observed)}


def parse_rows(result, columns=1, symbols=True):
    if result["status"] != "OK":
        return None
    lines = result["stdout"].strip().splitlines()
    if lines == ["(no results)"]:
        return []
    rows = [tuple(part.strip() for part in line.split(" | ")) for line in lines]
    if not rows or any(len(row) != columns or any(not v or (symbols and "::" not in v) for v in row) for row in rows):
        return None
    return [row[0] if columns == 1 else row for row in rows]


def syntax_available(index):
    if index["status"] == "OK":
        return True
    # PR84's explicit partial-index contract permits reading the syntax graph.
    # Never salvage a signal, timeout, or unrecognised/absent status receipt.
    if index["status"] != "FAILED" or index["exit_code"] != 1:
        return False
    try:
        report = json.loads(index["index_status_artifacts"]["index-status.json"])
        return (report["schema_version"] == 1 and bool(report["indexed_file_fingerprint"])
                and report["resolution"]["state"] == "succeeded"
                and any(s["state"] == "failed" for s in report["scip"].values()))
    except (ValueError, KeyError, TypeError):
        return False


def process_usage(pid):
    """Linux descendant RSS/CPU sample, including already reaped child CPU."""
    rows = {}
    for path in Path('/proc').glob('[0-9]*/stat'):
        try:
            fields = path.read_text().rsplit(')', 1)[1].split()
            rows[int(path.parent.name)] = (int(fields[1]),
                sum(int(fields[i]) for i in (11, 12, 13, 14)), max(0, int(fields[21])))
        except (OSError, ValueError, IndexError):
            continue
    descendants = {pid}
    while True:
        children = {p for p, (parent, _, _) in rows.items() if parent in descendants}
        if children <= descendants:
            break
        descendants.update(children)
    ticks = sum(rows[p][1] for p in descendants if p in rows)
    pages = sum(rows[p][2] for p in descendants if p in rows)
    return ticks / os.sysconf('SC_CLK_TCK'), pages * os.sysconf('SC_PAGE_SIZE') // 1024


class Lane:
    def __init__(self, root, binary, mcp, timeout, threads, model_dir=None, cpu_seconds=120, memory_mib=4096):
        # Failures such as baseline SIGILL must not leave large memory/core artifacts.
        resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        self.root, self.timeout, self.steps = root, timeout, []
        self.cpu_seconds, self.memory_mib = cpu_seconds, memory_mib
        allowed = sorted(os.sched_getaffinity(0))
        if threads > len(allowed):
            raise ValueError('requested threads exceed available CPU affinity')
        self.cpus = set(allowed[:threads])
        for name in ["repo", "home/.infigraph", "registry", "state", "cache", "tmp", "bin"]:
            (root / name).mkdir(parents=True, exist_ok=True)
        shutil.copyfile(binary, root / "bin/infigraph")
        (root / "bin/infigraph").chmod(0o755)
        if mcp:
            shutil.copyfile(mcp, root / "bin/infigraph-mcp")
            (root / "bin/infigraph-mcp").chmod(0o755)
        for name in ["git", "which"]:
            (root / "bin" / name).symlink_to(Path(shutil.which(name)).resolve())
        # No curl/npm/node/gh/indexer in PATH: no bootstrap or project execution.
        (root / "home/.infigraph/update_check.json").write_text(canonical({"checked_at": int(time.time())}))
        self.env = {"HOME": "/bench/home", "PATH": "/bench/bin", "TMPDIR": "/bench/tmp",
                    "XDG_CONFIG_HOME": "/bench/home/config", "XDG_CACHE_HOME": "/bench/cache",
                    "XDG_DATA_HOME": "/bench/home/data", "XDG_STATE_HOME": "/bench/state",
                    "INFIGRAPH_REGISTRY_HOME": "/bench/registry", "INFIGRAPH_NO_WATCH": "1",
                    "CI": "1", "RAYON_NUM_THREADS": str(threads), "OMP_NUM_THREADS": str(threads),
                    "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null", "LC_ALL": "C",
                    "TZ": "UTC"}
        if model_dir:
            (root / "model").mkdir()
            for name in ["config.json", "tokenizer.json", "model.safetensors"]:
                shutil.copyfile(model_dir / name, root / "model" / name)
            self.env["INFIGRAPH_MODEL_DIR"] = "/bench/model"

    def sandbox(self, command):
        # Expose system libraries/tools read-only, no host HOME, repos, credentials, sockets.
        args = [shutil.which("bwrap"), "--die-with-parent", "--unshare-all", "--new-session"]
        for name in ["/usr", "/lib", "/lib64"]:
            if Path(name).exists():
                args += ["--ro-bind", name, name]
        args += ["--symlink", "usr/bin", "/bin", "--proc", "/proc", "--dev", "/dev",
                 "--tmpfs", "/tmp", "--bind", str(self.root), "/bench", "--chdir", "/bench/repo",
                 "--clearenv"]
        for key, value in self.env.items():
            args += ["--setenv", key, value]
        return args + ["--"] + command

    def run(self, name, args, stdin=None, executable="infigraph", stop_after_ids=None):
        command = ["/bench/bin/" + executable] + args
        rss = self.root / "rss.txt"
        rss.unlink(missing_ok=True)
        if Path("/usr/bin/time").exists():
            command = ["/usr/bin/time", "-f", "%M", "-o", "/bench/rss.txt"] + command
        started = time.perf_counter_ns()
        builders = set(active_builds())
        with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
            proc = subprocess.Popen(self.sandbox(command), stdin=subprocess.PIPE,
                                    stdout=subprocess.PIPE if stop_after_ids else out, stderr=err,
                                    start_new_session=True, env={"PATH": "/usr/bin:/bin"},
                                    preexec_fn=lambda: os.sched_setaffinity(0, self.cpus))
            timed_out = False
            stopped_after_responses = False
            budget_exceeded = None
            cpu_s, peak_tree_rss = 0, 0
            try:
                try:
                    if stdin:
                        proc.stdin.write(stdin.encode())
                    proc.stdin.close()
                except BrokenPipeError:
                    # A crashing/unsupported MCP must be recorded, not abort the pair.
                    pass
                deadline = time.monotonic() + self.timeout
                pending, buffer = set(stop_after_ids or ()), b""
                while True:
                    cpu, memory = process_usage(proc.pid)
                    cpu_s, peak_tree_rss = max(cpu_s, cpu), max(peak_tree_rss, memory)
                    if cpu_s > self.cpu_seconds or peak_tree_rss > self.memory_mib * 1024:
                        budget_exceeded = "CPU" if cpu_s > self.cpu_seconds else "RAM"
                        raise subprocess.TimeoutExpired(command, self.timeout)
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise subprocess.TimeoutExpired(command, self.timeout)
                    if not stop_after_ids:
                        try:
                            proc.wait(timeout=min(0.05, remaining))
                            break
                        except subprocess.TimeoutExpired:
                            continue
                    if not select.select([proc.stdout], [], [], min(0.05, remaining))[0]:
                        continue
                    chunk = os.read(proc.stdout.fileno(), 65536)
                    if not chunk:
                        break
                    out.write(chunk)
                    buffer += chunk
                    while b"\n" in buffer:
                        line, buffer = buffer.split(b"\n", 1)
                        try:
                            reply = json.loads(line)
                            if isinstance(reply, dict):
                                pending.discard(reply.get("id"))
                        except (ValueError, TypeError):
                            pass
                    if not pending or out.tell() > 2_000_000 or len(buffer) > 2_000_000:
                        break
                if stop_after_ids:
                    # MCP is a persistent server. Completion means replies arrived, not
                    # that the server exits on EOF (older versions keep a private UI alive).
                    stopped_after_responses = not pending
                    if proc.poll() is None:
                        os.killpg(proc.pid, signal.SIGKILL)
                    proc.wait()
            except subprocess.TimeoutExpired:
                timed_out = budget_exceeded is None
                os.killpg(proc.pid, signal.SIGKILL)
                if stop_after_ids:
                    proc.wait()
                    out.write(proc.stdout.read(2_000_001))
                else:
                    proc.wait()
            finally:
                if proc.stdout:
                    proc.stdout.close()
            elapsed = (time.perf_counter_ns() - started) / 1e6
            builders.update(active_builds())
            logs = {}
            for key, stream in [("stdout", out), ("stderr", err)]:
                stream.seek(0)
                data = stream.read(2_000_001)
                logs[key] = data[:2_000_000].decode(errors="replace")
                logs[key + "_truncated"] = len(data) > 2_000_000
        peak = None
        if rss.exists():
            last = rss.read_text().strip().splitlines()[-1:]
            if last and last[0].isdigit():
                peak = int(last[0])
        result = {"name": name, "argv": [executable] + args, "wall_ms": elapsed,
                  "max_rss_kib": peak, "exit_code": proc.returncode,
                  "sampled_tree_cpu_s": cpu_s, "sampled_peak_tree_rss_kib": peak_tree_rss,
                  "budget_exceeded": budget_exceeded,
                  "host_builds_at_boundaries": sorted(builders),
                  "stopped_after_responses": stopped_after_responses,
                  "status": "BUDGET_EXCEEDED" if budget_exceeded else "TIMEOUT" if timed_out else "OK" if proc.returncode == 0 or stopped_after_responses else "FAILED", **logs}
        if result["stdout_truncated"] or result["stderr_truncated"]:
            result["status"] = "OUTPUT_LIMIT"
        if args and args[0] == "index":
            match = re.search(r"Indexed (\d+) files \((\d+) up-to-date, (\d+) total\)", result["stdout"])
            noop = re.search(r"All (\d+) files up-to-date, nothing to reindex", result["stdout"])
            result["indexed_files"] = int(match[1]) if match else 0 if noop else None
            resolve = re.search(r"(\d+) cross-file calls, (\d+) resolved, (\d+) unresolved", result["stdout"])
            result["engine_call_resolution"] = dict(zip(["cross_file", "resolved", "unresolved"], map(int, resolve.groups()))) if resolve else None
            result["index_status_artifacts"] = {p.name: p.read_text(errors="replace")
                                                for p in (self.root / "repo/.infigraph").glob("*status*.json")}
        self.steps.append(result)
        return result

    def query(self, name, cypher):
        return self.run(name, ["query", cypher])


def literal(value):
    # Graph ids in this curated corpus use simple paths/names; reject ambiguous quoting.
    if not re.fullmatch(r"[a-zA-Z0-9_./:$<>-]+", value):
        raise ValueError("unsupported graph id character")
    return "'" + value + "'"


def impact_query(question):
    return ("MATCH (a:Symbol)-[:CALLS*1.." + str(question["depth"]) + "]->(b:Symbol) WHERE b.id = " +
            literal(question["target"]) + " RETURN DISTINCT a.id")


def import_diagnostics(lane, truth):
    """Independent id/file queries distinguish label mismatch from missing edges."""
    labels = truth.get("imports", [])
    if not labels:
        return None
    endpoints = sorted({r[k] for r in labels for k in ("from", "to")})
    values = ','.join(map(literal, endpoints))
    modules = parse_rows(lane.query("import_modules",
        f"MATCH (m:Module) WHERE m.file IN [{values}] RETURN m.id,m.file"), 2, False)
    sources = ','.join(map(literal, sorted({r['from'] for r in labels})))
    by_id = parse_rows(lane.query("imports_by_id",
        f"MATCH (a:Module)-[:IMPORTS]->(b:Module) WHERE a.id IN [{sources}] RETURN a.id,b.id"), 2, False)
    total = parse_rows(lane.query("import_edge_count",
        "MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN count(*)"), symbols=False)
    return {"module_id_file_rows": modules, "missing_modules": sorted(set(endpoints) - {r[1] for r in modules}) if modules is not None else None,
            "by_id": score(by_id, [(r['from'], r['to']) for r in labels if r['label'] == 'positive']),
            "total_edges": int(total[0]) if total and len(total) == 1 and total[0].isdigit() else None}


def evaluate(lane, truth, repeats):
    ids = [s["id"] for s in truth["symbols"]]
    values = ",".join(map(literal, ids))
    observations = parse_rows(lane.query("symbols", f"MATCH (s:Symbol) WHERE s.id IN [{values}] RETURN s.id"))
    symbols = score(observations, ids)
    edges = parse_rows(lane.query("calls", f"MATCH (a:Symbol)-[:CALLS]->(b:Symbol) WHERE b.id IN [{values}] RETURN a.id,b.id"), 2)
    relation = lambda label: [(r["from"], r["to"]) for r in truth["relations"] if r["label"] == label]
    calls = score(edges, relation("positive"), relation("negative"))
    files = ",".join(map(literal, sorted({r["from"] for r in truth.get("imports", [])})))
    imports = None
    if files:
        rows = parse_rows(lane.query("imports", f"MATCH (a:Module)-[:IMPORTS]->(b:Module) WHERE a.file IN [{files}] RETURN a.file,b.file"), 2, False)
        imports = score(rows, [(r["from"], r["to"]) for r in truth["imports"] if r["label"] == "positive"],
                        [(r["from"], r["to"]) for r in truth["imports"] if r["label"] == "negative"])
    questions = []
    for question in truth["questions"]:
        cypher = impact_query(question)
        samples = [lane.query(question["id"] + ":" + str(i), cypher) for i in range(repeats)]
        parsed = [parse_rows(s) for s in samples]
        consistent = all(p is not None for p in parsed) and all(set(p) == set(parsed[0]) for p in parsed)
        questions.append({"id": question["id"], "score": score(parsed[0] if consistent else None, question["expected"]),
                          "median_wall_ms": statistics.median(s["wall_ms"] for s in samples)
                          if all(s["status"] == "OK" for s in samples) else None})
    return {"symbols": symbols, "calls": calls, "imports": imports, "impact": questions}


def tool_questions(lane, truth):
    """Fixed five-question retrieval protocol, no model or agent claims."""
    questions = truth['questions'][:5]
    messages = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": SCHEMA, "version": "1"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
    ]
    for i, question in enumerate(questions, 3):
        messages.append({"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {
            "name": "query_graph", "arguments": {"path": "/bench/repo", "cypher": impact_query(question), "detail": True}}})
    result = lane.run("mcp_questions", ["--mcp"], ''.join(json.dumps(m) + '\n' for m in messages),
                      "infigraph-mcp", set(range(1, len(questions) + 3)))
    outcomes = []
    try:
        replies = {m['id']: m for m in map(json.loads, result['stdout'].splitlines()) if 'id' in m}
        handshake = ('result' in replies[1] and any(t['name'] == 'query_graph' for t in replies[2]['result']['tools']))
    except (ValueError, KeyError, TypeError):
        replies, handshake = {}, False
    for i, question in enumerate(questions, 3):
        reply = replies.get(i, {})
        content = reply.get('result', {})
        if not isinstance(content, dict):
            content = {}
        blocks = content.get('content', [])
        if not isinstance(blocks, list):
            blocks = []
        text = '\n'.join(b['text'] for b in blocks if isinstance(b, dict) and b.get('type') == 'text' and isinstance(b.get('text'), str))
        valid = handshake and result['status'] == 'OK' and 'error' not in reply and not content.get('isError', False)
        rows = parse_rows({'status': 'OK' if valid else 'FAILED', 'stdout': text})
        outcomes.append({'id': question['id'], 'score': score(rows, question['expected']),
                         'source_evidence': question['evidence'], 'response_utf8_bytes': len(text.encode())})
    ok = all(q['score']['status'] != 'UNRESOLVED' for q in outcomes) and bool(outcomes)
    if result['status'] == 'OK' and not ok:
        result['status'] = 'FAILED_OUTPUT'
    return {'status': 'OK' if ok else 'FAILED', 'protocol': 'five manifest questions in manifest order; one query_graph call each; detail=true; no retries',
            'tool_calls': len(questions), 'questions': outcomes,
            'timing_scope': 'whole MCP session; no per-request latency or model tokens'}


def mcp_smoke(lane):
    messages = [
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": SCHEMA, "version": "1"}}},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/list"},
        {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "query_graph", "arguments": {
            "path": "/bench/repo", "cypher": "MATCH (s:Symbol) RETURN s.id LIMIT 1"}}},
    ]
    result = lane.run("mcp_session", ["--mcp"], "".join(json.dumps(m) + "\n" for m in messages), "infigraph-mcp", {1, 2, 3})
    try:
        replies = {m["id"]: m for m in map(json.loads, result["stdout"].splitlines()) if "id" in m}
        ok = (result["status"] == "OK" and all("result" in replies[i] and "error" not in replies[i] for i in [1, 2, 3])
              and any(t["name"] == "query_graph" for t in replies[2]["result"]["tools"])
              and not replies[3]["result"].get("isError", False)
              and bool(replies[3]["result"].get("content")))
    except (ValueError, KeyError, TypeError):
        ok = False
    return {"status": "OK" if ok else "FAILED", "timing_scope": "whole stdio session including startup; not per-query"}


def test_context_probes(lane, file, has_mcp):
    cli = lane.run("cli_test_context", ["test-context", "--file", file, "--limit", "1"])
    if cli["status"] == "OK" and "Test Generation Context" not in cli["stdout"]:
        cli["status"] = "FAILED_OUTPUT"
    outcomes = {"cli": cli["status"], "mcp": "NOT_RUN"}
    if has_mcp:
        messages = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
                "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": SCHEMA, "version": "1"}}},
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "generate_test_context", "arguments": {"path": "/bench/repo", "file": file, "limit": 1}}},
        ]
        result = lane.run("mcp_test_context", ["--worker", "--mcp"],
                          "".join(json.dumps(m) + "\n" for m in messages), "infigraph-mcp", {1, 2})
        try:
            replies = {m["id"]: m for m in map(json.loads, result["stdout"].splitlines()) if "id" in m}
            content = replies[2]["result"]
            valid = not content.get("isError", False) and "Test Generation Context" in json.dumps(content)
        except (ValueError, KeyError, TypeError):
            valid = False
        if result["status"] == "OK" and not valid:
            result["status"] = "FAILED_OUTPUT"
        outcomes["mcp"] = result["status"]
    return outcomes


def freshness(lane, index_args):
    """Synthetic opt-in only. No source project code is executed."""
    path = lane.root / "repo/afterimage_benchmark_fixture.ts"
    if path.exists():
        raise ValueError("freshness fixture collision")
    # A tiny git repo is initialized only inside the disposable exported copy.
    for args in [["init", "-q"], ["config", "user.email", "benchmark@example.invalid"],
                 ["config", "user.name", "Benchmark"], ["config", "core.hooksPath", "/dev/null"]]:
        git(lane.root / "repo", *args)
    outcomes = []

    def metadata():
        receipt = lane.root / 'repo/.infigraph/index-status.json'
        try:
            data = json.loads(receipt.read_text())
        except (OSError, ValueError):
            data = {}
        return {'fixture_path': path.name, 'source_sha256': digest(path) if path.exists() else None,
                'git_head': git(lane.root / 'repo', 'rev-parse', 'HEAD').decode().strip(),
                'indexed_file_fingerprint': data.get('indexed_file_fingerprint'), 'run_id': data.get('run_id')}

    def check(name, expected):
        before = metadata()
        indexed = lane.run(name + ":index", index_args)
        result = lane.query(name + ":query", "MATCH (s:Symbol) WHERE s.file IN ['afterimage_benchmark_fixture.ts','afterimage_benchmark_renamed.ts'] RETURN s.id")
        actual = parse_rows(result)
        wanted = {path.name + "::" + s for s in expected}
        graph_status = "UNRESOLVED" if actual is None or not syntax_available(indexed) else "PASS" if set(actual) == wanted else "FAIL"
        status = graph_status if indexed["status"] == "OK" or graph_status == "UNRESOLVED" else "PARTIAL_GRAPH_" + ("MATCH" if graph_status == "PASS" else "MISMATCH")
        outcomes.append({"case": name, "status": status, "graph_status": graph_status,
                         "index_status": indexed["status"], "observed": actual, "expected": sorted(wanted),
                         "before_reindex": before, "after_reindex": metadata()})

    path.write_text("export const branchA = () => 1;\n")
    for args in [["add", "-f", path.name], ["commit", "-qm", "fixture A"], ["branch", "fixture-a"]]:
        git(lane.root / "repo", *args)
    check("branch_a", ["branchA"])
    git(lane.root / "repo", "checkout", "-qb", "fixture-b")
    path.write_text("export const branchB = () => 2;\n")
    git(lane.root / "repo", "commit", "-qam", "fixture B")
    check("branch_b", ["branchB"])
    git(lane.root / "repo", "checkout", "-q", "fixture-a")
    check("branch_return", ["branchA"])
    path.write_text("export const uncommitted = () => 3;\n")
    check("uncommitted", ["uncommitted"])
    renamed = path.with_name('afterimage_benchmark_renamed.ts')
    if renamed.exists():
        raise ValueError('freshness rename fixture collision')
    path.rename(renamed)
    path = renamed
    check('file_renamed', ['uncommitted'])
    path.unlink()
    check("deleted", [])
    return outcomes


def run(args):
    if active_builds():
        raise ValueError("native build active; defer A/B until the build owner reports an idle host")
    if args.output.suffix != ".json":
        raise ValueError("--output must end with .json to keep metrics and summary separate")
    truth = json.loads(args.truth.read_text())
    if truth["commit"] != args.commit:
        raise ValueError("truth and fixture commit differ")
    verified = verify_truth(args.repo, truth)
    if not args.no_embed and not args.model_dir:
        raise ValueError("use --no-embed or supply an explicit offline --model-dir")
    for revision in [args.baseline_revision, args.candidate_revision]:
        if not SHA.fullmatch(revision):
            raise ValueError("binary revisions must be full commit SHAs")
    for path in [args.baseline, args.candidate, args.baseline_mcp, args.candidate_mcp]:
        if path is not None and not path.is_file():
            raise ValueError("explicit binary input is not a regular file")
    if not shutil.which("bwrap") or not shutil.which("git") or not shutil.which("which"):
        raise ValueError("Linux bubblewrap, git, and which are required; no unsafe fallback")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.output.exists() or args.output.with_suffix(".txt").exists():
        raise ValueError("output already exists; use a new run name")
    artifacts = args.output.with_suffix(".artifacts")
    artifacts.mkdir(exist_ok=False)
    report = {"schema": SCHEMA, "project": truth["project"], "commit": args.commit,
              "host": {"system": platform.system(), "machine": platform.machine(), "python": platform.python_version()},
              "truth_sha256": digest(args.truth), "verified": verified, "lanes": [],
              "model_files": {n: digest(args.model_dir / n) for n in ["config.json", "tokenizer.json", "model.safetensors"]} if args.model_dir else None,
              "controls": {"order": args.order, "serialized": True, "threads": args.threads,
                           "no_embed": args.no_embed, "timeout_s": args.timeout,
                           "network": "isolated namespace", "scip": "NOT_RUN: external tooling absent from PATH",
                           "core_dumps": False,
                           "cpu_seconds": args.cpu_seconds, "memory_mib": args.memory_mib,
                           "cpu_affinity": "first threads CPUs in inherited allowed set; all sandbox descendants inherit",
                           "budget_enforcement": "50ms Linux descendant CPU/RSS sampling; kills sandbox; RSS is summed, may overshoot between samples",
                           "fixture_scope": args.fixture_scope,
                           "cold": "empty index and HOME; OS page cache is NOT flushed",
                           "query_latency": "CLI process + graph open + traversal; no LLM",
                           "rss": "GNU time max RSS KiB if available; maximum, not concurrent sum",
                           "build_control": "refuse if builder active at start; sample command boundaries; operator must reserve idle host",
                           "export": "tracked code + selected package/ignore config; no install/dependency tree"},
              "agent_differential": {"status": "NOT_RUN", "reason": "no fixed-model agent adapter configured"}}
    with tempfile.TemporaryDirectory(prefix="afterimage-bench-", dir=args.temp_root) as temporary:
        work = Path(temporary)
        for label in args.order.split(","):
            binary, revision, mcp = getattr(args, label), getattr(args, label + "_revision"), getattr(args, label + "_mcp")
            lane = Lane(work / label, binary, mcp, args.timeout, args.threads, args.model_dir, args.cpu_seconds, args.memory_mib)
            paths = {a["path"] for a in truth["anchors"]} if args.fixture_scope == "truth-files" else None
            files = export(args.repo, args.commit, lane.root / "repo", paths)
            record = {"label": label, "revision": revision, "revision_provenance": "operator supplied; verify build manifest separately",
                      "binary_sha256": digest(lane.root / "bin/infigraph"),
                      "mcp_sha256": digest(lane.root / "bin/infigraph-mcp") if mcp else None, "exported_files": files}
            record["version"] = lane.run("version", ["--version"])["stdout"].strip()
            index_args = ["index"] + (["--no-embed"] if args.no_embed else [])
            cold = lane.run("cold_index", index_args)
            record["index_disposition"] = "COMPLETE_COMMAND" if cold["status"] == "OK" else "PARTIAL_SYNTAX_GRAPH" if syntax_available(cold) else "UNAVAILABLE"
            if syntax_available(cold):
                record["import_diagnostics"] = import_diagnostics(lane, truth)
                lane.run("warm_index", index_args)
                record["quality"] = evaluate(lane, truth, args.repeats)
                record["tool_questions"] = tool_questions(lane, truth) if args.tool_questions and mcp else {"status": "NOT_RUN"}
                # Change an actual tracked TS file, without executing it or creating a new file.
                change = lane.root / "repo" / truth["anchors"][0]["path"]
                original = change.read_bytes()
                change.write_bytes(original + b"\n// afterimage incremental benchmark\n")
                lane.run("changed_file_index", index_args)
                change.write_bytes(original)
                lane.run("restore_index", index_args)
                record["mcp"] = mcp_smoke(lane) if mcp else {"status": "NOT_RUN", "reason": "no explicit MCP binary"}
                record["test_context"] = test_context_probes(lane, truth["anchors"][0]["path"], bool(mcp)) if args.test_context else {"status": "NOT_RUN"}
                record["freshness"] = freshness(lane, index_args) if args.freshness else {"status": "NOT_RUN"}
            else:
                record["quality"] = {"status": "UNRESOLVED", "reason": "cold index failed"}
                record["mcp"] = {"status": "NOT_RUN", "reason": "cold index failed"}
            # Preserve stage outcomes opaquely: PR84 may add fields. No exit=0 => SCIP-success inference.
            statuses = {}
            for p in (lane.root / "repo/.infigraph").glob("*status*.json"):
                statuses[p.name] = p.read_text(errors="replace")
            record["index_status_artifacts"] = statuses
            record["steps"] = lane.steps
            report["lanes"].append(record)
            shutil.rmtree(lane.root)
    comparable = len({r["binary_sha256"] for r in report["lanes"]}) == 2
    report["distinct_binaries"] = comparable
    report["comparison"] = "DESCRIPTIVE_SINGLE_PAIR" if comparable else "SMOKE_ONLY_IDENTICAL_BINARIES"
    if comparable and any(next(s for s in r["steps"] if s["name"] == "cold_index")["status"] != "OK" for r in report["lanes"]):
        report["comparison"] = "INCOMPLETE_INDEX_PAIR"
    if any(s["host_builds_at_boundaries"] for r in report["lanes"] for s in r["steps"]):
        report["comparison"] = "INVALID_BUILD_CONTENTION"
    failures = sum(s["status"] != "OK" for r in report["lanes"] for s in r["steps"])
    failures += sum(r["mcp"]["status"] == "FAILED" for r in report["lanes"])
    failures += sum(c["status"] != "PASS" for r in report["lanes"] for c in r.get("freshness", []) if isinstance(c, dict))
    report["status"] = "COMPLETED_WITH_FAILURES" if failures else "COMPLETED"
    # Logs separate from canonical metric JSON; all paths inside logs are sandbox paths.
    for record in report["lanes"]:
        for i, step in enumerate(record["steps"]):
            for key in ["stdout", "stderr"]:
                name = f"{record['label']}-{i:03d}-{key}.txt"
                (artifacts / name).write_text(step.pop(key))
                step[key + "_artifact"] = artifacts.name + "/" + name
    args.output.write_text(canonical(report))
    lines = [f"{report['project']} @ {args.commit}: {report['comparison']} ({report['status']})"]
    for record in report["lanes"]:
        timings = "; ".join(f"{s['name']}={s['wall_ms']:.1f}ms/{s['status']}"
                            for s in record["steps"] if s["name"] in {"cold_index", "warm_index", "changed_file_index"})
        lines.append(f"{record['label']} {record['revision']}: {timings}; MCP={record['mcp']['status']}")
    summary = "\n".join(lines) + "\n"
    args.output.with_suffix(".txt").write_text(summary)
    print(summary, end="")
    return 1 if failures else 0


def smoke(args):
    """Unattested installed binaries: functionality only, never an A/B performance claim."""
    if args.output.suffix != ".json":
        raise ValueError("--output must end with .json")
    if not shutil.which("bwrap") or not shutil.which("git") or not shutil.which("which"):
        raise ValueError("Linux bubblewrap, git, and which are required; no unsafe fallback")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.output.exists():
        raise ValueError("output already exists; use a new run name")
    artifacts = args.output.with_suffix(".artifacts")
    artifacts.mkdir(exist_ok=False)
    with tempfile.TemporaryDirectory(prefix="afterimage-smoke-", dir=args.temp_root) as tmp:
        lane = Lane(Path(tmp), args.binary, args.mcp, args.timeout, 1)
        (lane.root / "repo/helper.ts").write_text("export const helper = () => 1;\n")
        (lane.root / "repo/main.ts").write_text("import { helper } from './helper';\nexport const caller = () => helper();\n")
        lane.run("version", ["--version"])
        index = lane.run("index", ["index", "--no-embed"])
        observed = None
        calls = None
        mcp = {"status": "NOT_RUN", "reason": "index failed or no explicit MCP binary"}
        if index["status"] == "OK":
            observed = parse_rows(lane.query("symbols", "MATCH (s:Symbol) RETURN s.id"))
            calls = parse_rows(lane.query("calls", "MATCH (a:Symbol)-[:CALLS]->(b:Symbol) RETURN a.id,b.id"), 2)
            if args.mcp:
                mcp = mcp_smoke(lane)
        for i, step in enumerate(lane.steps):
            for key in ["stdout", "stderr"]:
                name = f"{i:03d}-{key}.txt"
                (artifacts / name).write_text(step.pop(key))
                step[key + "_artifact"] = artifacts.name + "/" + name
        report = {"schema": SCHEMA, "kind": "SYNTHETIC_SMOKE_ONLY", "revision": "UNKNOWN",
                  "binary_sha256": digest(lane.root / "bin/infigraph"), "mcp_sha256": digest(lane.root / "bin/infigraph-mcp") if args.mcp else None,
                  "performance_comparison": "NOT_RUN", "scip": "NOT_RUN: external tooling absent from PATH",
                  "symbols": score(observed, ["helper.ts::helper", "main.ts::caller"]),
                  "calls": score(calls, [("main.ts::caller", "helper.ts::helper")]), "mcp": mcp, "steps": lane.steps}
        ok = all(s["status"] == "OK" for s in lane.steps) and observed is not None and mcp["status"] != "FAILED"
        report["status"] = "COMPLETED" if ok else "COMPLETED_WITH_FAILURES"
        args.output.write_text(canonical(report))
        print(f"Synthetic installed-binary smoke: {report['status']}; MCP={mcp['status']}; revision UNKNOWN; no A/B claim")
        return 0 if ok else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    verify = sub.add_parser("verify", help="verify pinned source digests and line anchors only")
    verify.add_argument("--repo", type=Path, required=True)
    verify.add_argument("--truth", type=Path, required=True)
    sm = sub.add_parser("smoke", help="two-file synthetic functionality smoke; revision UNKNOWN; no comparison")
    sm.add_argument("--binary", type=Path, required=True)
    sm.add_argument("--mcp", type=Path)
    sm.add_argument("--output", type=Path, required=True)
    sm.add_argument("--temp-root", type=Path, default=Path(tempfile.gettempdir()))
    sm.add_argument("--timeout", type=float, default=30)
    bench = sub.add_parser("run", help="serialized A/B; no builds or source project execution")
    for label in ["baseline", "candidate"]:
        bench.add_argument("--" + label, type=Path, required=True)
        bench.add_argument("--" + label + "-revision", required=True)
        bench.add_argument("--" + label + "-mcp", type=Path)
    for name in ["repo", "truth", "output"]:
        bench.add_argument("--" + name, type=Path, required=True)
    bench.add_argument("--commit", required=True)
    bench.add_argument("--temp-root", type=Path, default=Path(tempfile.gettempdir()))
    bench.add_argument("--no-embed", action="store_true")
    bench.add_argument("--model-dir", type=Path, help="offline potion-base-8M assets, identical in both lanes")
    bench.add_argument("--freshness", action="store_true")
    bench.add_argument("--test-context", action="store_true", help="probe CLI and direct-worker MCP test-context; preserve crashes as failures")
    bench.add_argument("--tool-questions", action="store_true", help="fixed five-question MCP retrieval protocol; no model/agent execution")
    bench.add_argument("--cpu-seconds", type=float, default=120)
    bench.add_argument("--memory-mib", type=int, default=4096)
    bench.add_argument("--fixture-scope", choices=["repository", "truth-files"], default="repository",
                       help="repository source projection or only exact files with source-truth anchors; never an implicit fallback")
    bench.add_argument("--timeout", type=float, default=120)
    bench.add_argument("--threads", type=int, default=1)
    bench.add_argument("--repeats", type=int, default=3)
    bench.add_argument("--order", choices=["baseline,candidate", "candidate,baseline"], default="baseline,candidate")
    args = parser.parse_args()
    try:
        if args.command == "verify":
            print(canonical(verify_truth(args.repo, json.loads(args.truth.read_text()))), end="")
            return 0
        if (not math.isfinite(args.timeout) or args.timeout <= 0 or getattr(args, "threads", 1) < 1
                or getattr(args, "repeats", 1) < 1 or not math.isfinite(getattr(args, "cpu_seconds", 1))
                or getattr(args, "cpu_seconds", 1) <= 0 or getattr(args, "memory_mib", 1) < 1):
            raise ValueError("budgets, timeout, threads and repeats must be finite and positive")
        # All harness invocations by this user share this nonblocking host lock, even across temp roots.
        with open(Path(tempfile.gettempdir()) / f"afterimage-benchmark-{os.getuid()}.lock", "a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return smoke(args) if args.command == "smoke" else run(args)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        # Avoid leaking host paths / git stderr in machine-readable handoffs.
        print(canonical({"schema": SCHEMA, "status": "NOT_RUN", "reason": type(error).__name__,
                         "detail": str(error) if isinstance(error, ValueError) else "check inputs, sandbox prerequisites, and serialization lock"}), end="")
        return 2


if __name__ == "__main__":
    raise SystemExit(main())

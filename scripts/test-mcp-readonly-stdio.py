#!/usr/bin/env python3
"""Thin stdio client for an already built MCP binary. Uses disposable state only."""
import argparse
import json
import os
from pathlib import Path
import resource
import subprocess
import tempfile


def snapshot(root):
    return {str(p.relative_to(root)): p.read_bytes() for p in root.rglob("*") if p.is_file()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    options = parser.parse_args()
    binary = options.binary.resolve()
    with tempfile.TemporaryDirectory(prefix="afterimage-readonly-stdio-") as tmp:
        root = Path(tmp)
        home, project = root / "home", root / "project"
        home.mkdir()
        project.mkdir()
        (project / ".git").write_text("gitdir: ../worktrees/disposable\n")
        (project / "example.py").write_text("def helper(): return 1\n")
        (home / ".infigraph").mkdir()
        (home / ".infigraph/registry.json").write_text('{"repos":{},"groups":{}}')
        before = snapshot(root)
        requests = [
            {"id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "thin-contract-client", "version": "1"}}},
            {"id": 2, "method": "tools/list"},
            {"id": 3, "method": "tools/call", "params": {"name": "get_stats", "arguments": {"path": str(project)}}},
            {"id": 4, "method": "tools/call", "params": {"name": "delete_project", "arguments": {"path": str(project)}}},
            {"id": 5, "method": "tools/call", "params": {"name": "query_graph", "arguments": {"path": str(project), "cypher": "MATCH (n) DELETE n"}}},
            {"id": 6, "method": "tools/call", "params": {"name": "diagnose", "arguments": {"path": str(project)}}},
            {"id": 7, "method": "tools/call", "params": {"name": "search_symbols", "arguments": {"project": str(project), "query": "helper"}}},
        ]
        env = {**os.environ, "HOME": str(home), "INFIGRAPH_REGISTRY_HOME": str(home),
               "INFIGRAPH_MCP_PROFILE": "read-only", "INFIGRAPH_BACKEND": "kuzu",
               "INFIGRAPH_METRICS": "1", "HF_HUB_OFFLINE": "1", "RAYON_NUM_THREADS": "2"}
        def limits():
            resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
        output = subprocess.run([str(binary), "--mcp"], input="".join(json.dumps({"jsonrpc": "2.0", **r}) + "\n" for r in requests),
                                cwd=project, env=env, capture_output=True, text=True, timeout=45, preexec_fn=limits)
        assert output.returncode == 0, output.stderr
        responses = {r["id"]: r for r in map(json.loads, output.stdout.splitlines())}
        assert len(responses) == len(requests), output.stdout
        assert responses[1]["result"]["protocolVersion"] == "2024-11-05"
        assert "instructions v1" in responses[1]["result"]["instructions"]
        names = {t["name"] for t in responses[2]["result"]["tools"]}
        assert len(names) == 10 and "delete_project" not in names and "query_graph" not in names
        # Deliberately drop metadata as a thin client may do.
        for response in responses.values():
            response["result"].pop("_meta", None)
        text = lambda i: responses[i]["result"]["content"][0]["text"]
        assert "NOT_INDEXED" in text(3) and str(project) in text(3)
        assert "Freshness: unknown" in text(3)
        assert "READ_ONLY_TOOL_DENIED" in text(4) and "READ_ONLY_TOOL_DENIED" in text(5)
        assert "UNKNOWN_ARGUMENT" in text(7)
        assert json.loads(text(6))["schema_version"] == 1
        assert before == snapshot(root), "read-only stdio changed source/index/registry/HOME bytes"
        print("PASS stdio: 7 requests; unchanged protocol; curated tools; NOT_INDEXED scope; mutation/raw-write denials; path typo; metadata discarded; all fixture bytes unchanged")


if __name__ == "__main__":
    main()

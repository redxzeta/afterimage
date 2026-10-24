"""Harness tests use tiny synthetic repos/binaries, never application tests."""
import importlib.util
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("bench", Path(__file__).with_name("bench.py"))
bench = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bench)


class ScoringTests(unittest.TestCase):
    def test_import_diagnostics_distinguish_missing_modules_from_edges(self):
        class Queries:
            def query(self, name, cypher):
                return {'status': 'OK', 'stdout': {'import_modules': 'a.ts | a.ts\nb.ts | b.ts',
                    'imports_by_id': '(no results)', 'import_edge_count': '0'}[name]}
        result = bench.import_diagnostics(Queries(), {'imports': [{'from': 'a.ts', 'to': 'b.ts', 'label': 'positive'}]})
        self.assertEqual(result['missing_modules'], [])
        self.assertEqual(result['by_id']['missing'], [('a.ts', 'b.ts')])
        self.assertEqual(result['total_edges'], 0)

    def test_open_world_precision_is_not_invented(self):
        value = bench.score(["a", "b", "unlabelled"], ["a", "missing"], ["b", "absent"])
        self.assertIsNone(value["precision"])
        self.assertEqual(value["labelled_precision"], 0.5)
        self.assertEqual(value["recall"], 0.5)
        self.assertEqual(value["false_edges"], ["b"])
        self.assertEqual(value["unadjudicated"], ["unlabelled"])
        self.assertEqual(value["missing"], ["missing"])

    def test_failed_query_is_unresolved_not_empty_correct(self):
        self.assertEqual(bench.score(None, ["a"])["status"], "UNRESOLVED")
        self.assertIsNone(bench.score(None, ["a"])["recall"])
        self.assertEqual(bench.score([], ["a"])["recall"], 0)
        self.assertIsNone(bench.score([], [], ["a"])["labelled_precision"])

    def test_contradictory_labels_are_rejected(self):
        with self.assertRaises(ValueError):
            bench.score([], ["a"], ["a"])

    def test_query_parser_refuses_noise_and_errors(self):
        self.assertEqual(bench.parse_rows({"status": "OK", "stdout": "a.ts::foo | b.ts::bar\n"}, 2),
                         [("a.ts::foo", "b.ts::bar")])
        self.assertEqual(bench.parse_rows({"status": "OK", "stdout": "(no results)\n"}), [])
        for status, text in [("OK", ""), ("OK", "Warning: partial\na.ts::foo"), ("FAILED", "(no results)")]:
            self.assertIsNone(bench.parse_rows({"status": status, "stdout": text}))
        self.assertEqual(bench.parse_rows({"status": "OK", "stdout": "a.ts | b.ts"}, 2, False), [("a.ts", "b.ts")])

    def test_partial_syntax_receipt_never_hides_failure_or_salvages_a_crash(self):
        result = {"status": "FAILED", "exit_code": 1, "index_status_artifacts": {"index-status.json": json.dumps({
            "schema_version": 1, "indexed_file_fingerprint": "fixture", "resolution": {"state": "succeeded"},
            "scip": {"typescript": {"state": "failed", "reason": "SCIP_TOOL_UNAVAILABLE"}}})}}
        self.assertTrue(bench.syntax_available(result))
        self.assertEqual(result["status"], "FAILED")
        for code in [-4, 132, 2]:
            result["exit_code"] = code
            self.assertFalse(bench.syntax_available(result))
        result["exit_code"] = 1
        result["index_status_artifacts"] = {}
        self.assertFalse(bench.syntax_available(result))

    def test_export_filter_and_paths(self):
        for name in [".infigraph/state.ts", "node_modules/x.ts", "session.json", "data.db", ".env", "docs/report.md"]:
            self.assertFalse(bench.source_file(name))
        for name in ["src/a.ts", "tests/x.test.ts", "package.json", "packages/a/tsconfig.json"]:
            self.assertTrue(bench.source_file(name))
        for name in ["/absolute.ts", "../escape.ts", "src/../../escape.ts"]:
            with self.assertRaises(ValueError):
                bench.safe_path(name)

    def test_build_contention_refuses_ab_before_reading_inputs(self):
        with patch.object(bench, "active_builds", return_value=["rustc"]), self.assertRaisesRegex(ValueError, "native build active"):
            bench.run(None)


FAKE = '''#!/usr/bin/python3
import json,os,socket,sys,time
from pathlib import Path
assert os.environ['HOME']=='/bench/home'
assert os.environ['PATH']=='/bench/bin'
assert 'INFIGRAPH_BACKEND' not in os.environ
assert not Path('/home').exists()
if sys.argv[0].endswith('-mcp'):
 for line in sys.stdin:
  m=json.loads(line)
  if 'id' not in m: continue
  if m['method']=='initialize': result={'protocolVersion':'2024-11-05','capabilities':{},'serverInfo':{'name':'fake','version':'1'}}
  elif m['method']=='tools/list': result={'tools':[{'name':'query_graph'}]}
  else:
   assert m['params']['arguments']['cypher'].startswith('MATCH')
   result={'content':[{'type':'text','text':'a.ts::a'}]}
  print(json.dumps({'jsonrpc':'2.0','id':m['id'],'result':result}),flush=True)
 time.sleep(60) # Persistent server: client must stop it after receiving replies.
elif '--version' in sys.argv: print('synthetic-harness-test')
elif sys.argv[1]=='index':
 Path('.infigraph').mkdir(exist_ok=True)
 Path('.infigraph/index-status.json').write_text(json.dumps({'scip':{'outcome':'partial','unknown_future_field':True}}))
 print('synthetic indexing test')
elif sys.argv[1]=='query':
 print('(no results)' if 'CALLS' in sys.argv[2] or 'IMPORTS' in sys.argv[2] else 'a.ts::a')
elif sys.argv[1]=='sleep': time.sleep(60)
elif sys.argv[1]=='crash':
 import resource,signal
 assert resource.getrlimit(resource.RLIMIT_CORE)==(0,0)
 os.kill(os.getpid(),signal.SIGILL)
'''


@unittest.skipUnless(shutil.which("bwrap") and Path("/usr/bin/python3").exists(), "Linux sandbox required")
class IsolationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="afterimage-harness-test-")
        self.root = Path(self.temp.name)
        self.repo = self.root / "source"
        self.repo.mkdir()
        bench.git(self.repo, "init", "-q")
        bench.git(self.repo, "config", "user.email", "benchmark@example.invalid")
        bench.git(self.repo, "config", "user.name", "Benchmark")
        bench.git(self.repo, "config", "core.hooksPath", "/dev/null")
        (self.repo / "a.ts").write_text("export const a = () => 1;\n")
        (self.repo / "secret.env").write_text("DO_NOT_EXPORT=fixture\n")
        (self.repo / "link.ts").symlink_to("/etc/passwd")
        bench.git(self.repo, "add", ".")
        bench.git(self.repo, "commit", "-qm", "fixture")
        self.commit = bench.git(self.repo, "rev-parse", "HEAD").decode().strip()
        self.truth = {"project": "synthetic-test", "commit": self.commit,
                      "anchors": [{"id": "a", "path": "a.ts", "line": 1,
                                   "line_sha256": hashlib.sha256(b"export const a = () => 1;").hexdigest(),
                                   "file_sha256": bench.digest(self.repo / "a.ts")}],
                      "symbols": [{"id": "a.ts::a", "evidence": ["a"]}], "relations": [], "questions": []}
        self.binary = self.root / "fake"
        self.binary.write_text(FAKE)
        self.binary.chmod(0o755)

    def tearDown(self):
        self.temp.cleanup()

    def test_pins_anchors_and_export_ignore_dirty_files_symlinks(self):
        (self.repo / "a.ts").write_text("dirty work must remain\n")
        destination = self.root / "export"
        self.assertEqual(bench.export(self.repo, self.commit, destination), 1)
        self.assertEqual((destination / "a.ts").read_text(), "export const a = () => 1;\n")
        self.assertFalse((destination / "link.ts").exists())
        self.assertFalse((destination / "secret.env").exists())
        self.assertEqual(bench.export(self.repo, self.commit, self.root / "empty", paths=set()), 0)
        self.assertEqual(bench.verify_truth(self.repo, self.truth)["symbols"], 1)
        self.truth["anchors"][0]["line"] = 0
        with self.assertRaises(ValueError):
            bench.verify_truth(self.repo, self.truth)

    def test_refuse_non_sha_and_anchor_drift(self):
        with self.assertRaises(ValueError):
            bench.pinned(self.repo, "HEAD")
        self.truth["anchors"][0]["file_sha256"] = "0" * 64
        with self.assertRaises(ValueError):
            bench.verify_truth(self.repo, self.truth)

    def test_timeout_kills_sandbox_and_reports_failure(self):
        lane = bench.Lane(self.root / "lane", self.binary, None, 0.15, 1)
        result = lane.run("bounded", ["sleep"])
        self.assertEqual(result["status"], "TIMEOUT")

    def test_cpu_and_ram_budgets_kill_descendants(self):
        for kind, child in [('CPU', 'while True: pass'), ('RAM', 'import time; x=bytearray(64*1024*1024); time.sleep(30)')]:
            self.binary.write_text('#!/usr/bin/python3\nimport subprocess\nsubprocess.run(["/usr/bin/python3","-c",' + repr(child) + '])\n')
            lane = bench.Lane(self.root / kind, self.binary, None, 5, 1,
                              cpu_seconds=0.15 if kind == 'CPU' else 5, memory_mib=32 if kind == 'RAM' else 256)
            result = lane.run('budget', [])
            self.assertEqual(result['status'], 'BUDGET_EXCEEDED')
            self.assertEqual(result['budget_exceeded'], kind)
            self.assertLess(result['wall_ms'], 5000)

    def test_unavailable_namespace_has_no_unsandboxed_fallback(self):
        lane = bench.Lane(self.root / 'refused', self.binary, None, 3, 1)
        with patch.object(lane, 'sandbox', return_value=['/usr/bin/false']):
            result = lane.run('refused', ['--version'])
        self.assertEqual(result['status'], 'FAILED')
        self.assertEqual(result['stdout'], '')

    def test_fixed_mcp_questions_score_source_subset_and_tool_errors(self):
        question = {'id': 'one', 'depth': 1, 'target': 'a.ts::a', 'expected': ['a.ts::a'], 'evidence': ['a']}
        lane = bench.Lane(self.root / 'questions', self.binary, self.binary, 3, 1)
        result = bench.tool_questions(lane, {'questions': [question] * 5})
        self.assertEqual(result['status'], 'OK')
        self.assertEqual(result['tool_calls'], 5)
        self.assertTrue(all(q['score']['recall'] == 1 for q in result['questions']))
        self.binary.write_text(FAKE.replace("result={'content'", "result={'isError':True,'content'"))
        lane = bench.Lane(self.root / 'errors', self.binary, self.binary, 3, 1)
        result = bench.tool_questions(lane, {'questions': [question] * 5})
        self.assertEqual(result['status'], 'FAILED')
        self.assertTrue(all(q['score']['recall'] is None for q in result['questions']))
        self.assertEqual(lane.steps[-1]['status'], 'FAILED_OUTPUT')

    def test_mcp_timeout_with_closed_stdin_preserves_output(self):
        lane = bench.Lane(self.root / "lane", self.binary, None, 0.15, 1)
        result = lane.run("mcp_timeout", ["sleep"], "{}\n", stop_after_ids={1})
        self.assertEqual(result["status"], "TIMEOUT")

    def test_signal_crash_is_failure_and_core_dumps_disabled(self):
        lane = bench.Lane(self.root / "lane", self.binary, None, 3, 1)
        result = lane.run("crash", ["crash"])
        self.assertEqual(result["status"], "FAILED")
        self.assertNotEqual(result["exit_code"], 0)
        self.assertFalse(list(lane.root.rglob("core*")))

    def test_mcp_crash_during_input_is_a_failed_step(self):
        lane = bench.Lane(self.root / 'early-crash', self.binary, None, 3, 1)
        result = lane.run('early_crash', ['crash'], 'x' * 100_000, stop_after_ids={1})
        self.assertEqual(result['status'], 'FAILED')
        self.assertFalse(result['stopped_after_responses'])

    def test_freshness_branches_uncommitted_and_delete_are_disposable(self):
        dynamic = FAKE.replace("elif sys.argv[1]=='query':", """elif sys.argv[1]=='query' and "WHERE s.file" in sys.argv[2]:
 import re
 p=next((p for p in [Path('afterimage_benchmark_fixture.ts'),Path('afterimage_benchmark_renamed.ts')] if p.exists()),Path('afterimage_benchmark_fixture.ts'))
 names=re.findall(r'export const (\\w+)',p.read_text()) if p.exists() else []
 print('\\n'.join(p.name+'::'+n for n in names) or '(no results)')
elif sys.argv[1]=='query':""")
        for partial in [False, True]:
            script = dynamic
            if partial:
                receipt = json.dumps({"schema_version": 1, "indexed_file_fingerprint": "fixture",
                                      "resolution": {"state": "succeeded"},
                                      "scip": {"typescript": {"state": "failed"}}})
                script = script.replace("print('synthetic indexing test')",
                                        f"Path('.infigraph/index-status.json').write_text({receipt!r})\n print('partial fixture')\n sys.exit(1)")
            self.binary.write_text(script)
            lane = bench.Lane(self.root / str(partial), self.binary, None, 3, 1)
            cases = bench.freshness(lane, ["index", "--no-embed"])
            self.assertEqual([c["status"] for c in cases], ["PARTIAL_GRAPH_MATCH" if partial else "PASS"] * 6)
            self.assertEqual([c["index_status"] for c in cases], ["FAILED" if partial else "OK"] * 6)
            self.assertEqual([c["graph_status"] for c in cases], ["PASS"] * 6)
            self.assertEqual([c["case"] for c in cases], ["branch_a", "branch_b", "branch_return", "uncommitted", "file_renamed", "deleted"])
            self.assertEqual(cases[0]['after_reindex']['git_head'], cases[2]['after_reindex']['git_head'])
            self.assertNotEqual(cases[0]['after_reindex']['git_head'], cases[1]['after_reindex']['git_head'])
            self.assertEqual(cases[0]['after_reindex']['source_sha256'], cases[2]['after_reindex']['source_sha256'])
            self.assertEqual(cases[3]['after_reindex']['source_sha256'], cases[4]['after_reindex']['source_sha256'])
            self.assertIsNone(cases[5]['before_reindex']['source_sha256'])
        self.assertFalse((self.repo / "afterimage_benchmark_fixture.ts").exists())

    def test_end_to_end_artifacts_and_mcp_preserve_partial_stage(self):
        truth_file = self.root / "truth.json"
        truth_file.write_text(bench.canonical(self.truth))
        output = self.root / "result.json"
        # Fake-binary orchestration tests do not measure native performance; permit the
        # owner's concurrent native build without adding a bypass to the production CLI.
        launcher = "import sys; sys.path.insert(0,sys.argv.pop(1)); import bench; bench.active_builds=lambda: []; raise SystemExit(bench.main())"
        argv = ["python3", "-c", launcher, str(Path(bench.__file__).parent), "run", "--repo", str(self.repo), "--commit", self.commit,
                "--truth", str(truth_file), "--output", str(output), "--temp-root", str(self.root),
                "--baseline", str(self.binary), "--candidate", str(self.binary),
                "--baseline-revision", self.commit, "--candidate-revision", self.commit,
                "--baseline-mcp", str(self.binary), "--candidate-mcp", str(self.binary), "--no-embed"]
        completed = subprocess.run(argv, capture_output=True, text=True)
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        report = json.loads(output.read_text())
        self.assertEqual(report["comparison"], "SMOKE_ONLY_IDENTICAL_BINARIES")
        self.assertEqual(report["status"], "COMPLETED")
        for record in report["lanes"]:
            self.assertEqual(record["mcp"]["status"], "OK")
            self.assertTrue(next(s for s in record["steps"] if s["name"] == "mcp_session")["stopped_after_responses"])
            self.assertIn('"outcome": "partial"', record["index_status_artifacts"]["index-status.json"])
            self.assertEqual(record["quality"]["symbols"]["recall"], 1)
        self.assertNotIn(str(self.root), output.read_text())
        self.assertFalse((self.repo / ".infigraph").exists())
        # Existing output/artifact roots are refused, never overwritten.
        self.assertEqual(subprocess.run(argv, capture_output=True).returncode, 2)


if __name__ == "__main__":
    unittest.main()

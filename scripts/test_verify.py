"""Regression guards for CI coverage decisions, not application behavior."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("verify", Path(__file__).with_name("verify.py"))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class CoverageTests(unittest.TestCase):
    def test_docs_keep_build_lint_and_process_smoke(self):
        plan = verify.commands({"docs/DOCTOR.md"})
        self.assertTrue(any(cmd[:2] == ["cargo", "build"] for cmd in plan))
        self.assertTrue(any("--all-targets" in cmd for cmd in plan))
        for name in ["embedding_format", "cli_parity", "tool_parity"]:
            self.assertTrue(any(name in cmd for cmd in plan), name)
        self.assertFalse(any("group_build_steps" in cmd for cmd in plan))

    def test_shared_engine_and_language_changes_retain_real_contracts(self):
        for path in ["crates/infigraph-core/src/multi/mod.rs",
                     "crates/infigraph-core/src/index_status.rs",
                     "crates/infigraph-languages/languages/typescript/entities.scm",
                     "tests/fixtures/microservices/payment-service/src/main.rs",
                     "Cargo.lock", ".cargo/config.toml", "new-build-input"]:
            with self.subTest(path=path):
                plan = verify.commands({path})
                for name in verify.CORE_TESTS + ["doctor", "diagnose", "typescript_callers",
                                                "coverage_reporting", "scip::tests"]:
                    self.assertTrue(any(name in cmd for cmd in plan), name)

    def test_missing_base_expands_coverage(self):
        self.assertEqual(verify.selection(None), (verify.ALL, {"workspace"}))

    def test_mcp_changes_keep_process_and_supervisor_tests(self):
        with patch.object(verify.sys, "platform", "linux"):
            plan = verify.commands({"crates/infigraph-mcp/src/main.rs"})
        self.assertIn(["python3", "scripts/test-mcp-supervisor.py"], plan)
        self.assertTrue(any("diagnose" in cmd for cmd in plan))

    def test_benchmark_checks_do_not_execute_real_benchmarks(self):
        with patch.object(verify.sys, "platform", "linux"):
            plan = verify.commands({"scripts/benchmark/bench.py"})
        self.assertTrue(any("scripts/benchmark" in cmd and "unittest" in cmd for cmd in plan))
        self.assertFalse(any("scripts/benchmark/bench.py" in cmd for cmd in plan))

    def test_full_retains_workspace_integration_and_doc_tests(self):
        plan = verify.commands(set(), full=True)
        self.assertIn(["cargo", "test", "--locked", "--workspace", "--", "--test-threads=1"], plan)

    def test_changed_integration_target_is_not_lost(self):
        target = "crates/infigraph-mcp/tests/memory_context.rs"
        plan = verify.commands({target})
        self.assertTrue(any("memory_context" in cmd for cmd in plan))

    def test_binary_only_cli_keeps_its_unit_tests(self):
        plan = verify.commands({"crates/infigraph-cli/src/main.rs"})
        self.assertTrue(any("infigraph-cli" in cmd and "--bins" in cmd for cmd in plan))

    def test_git_diff_includes_deleted_renamed_and_untracked_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.DEVNULL)
            git("init")
            git("config", "user.name", "CI Fixture")
            git("config", "user.email", "ci@example.invalid")
            (root / "old.rs").write_text("original")
            git("add", ".")
            git("commit", "-m", "fixture")
            (root / "old.rs").rename(root / "renamed.rs")
            git("add", "-A")
            (root / "untracked.rs").write_text("new")
            with patch.object(verify, "ROOT", root):
                self.assertEqual(verify.changed_paths("HEAD"), {"old.rs", "renamed.rs", "untracked.rs"})
                self.assertIsNone(verify.changed_paths("absent-ref"))


if __name__ == "__main__":
    unittest.main()

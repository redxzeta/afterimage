"""Synthetic diagnostic tests; no native process, debugger, or installation."""
import contextlib
import hashlib
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch
import diagnose_cli_compiler as compiler_control

spec = importlib.util.spec_from_file_location("diagnose_cli", Path(__file__).with_name("diagnose_cli.py"))
diagnose = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diagnose)


class RealCliControlTests(unittest.TestCase):
    def test_compile_preserves_primary_integration_context_without_running_other_suites(self):
        integration = ["cargo", "test", "--locked", "--workspace", "--test", "cli_parity",
                       "--test", "typescript_imports", "--test", "watch_new_directory",
                       "--", "--test-threads=1"]
        self.assertEqual(compiler_control.compile_command([["cargo", "fmt"], integration]),
                         integration[:-2] + ["--offline", "--no-run"])
        with self.assertRaisesRegex(ValueError, "context unavailable"):
            compiler_control.compile_command([["cargo", "test", "--workspace"]])

    def test_original_executables_and_runtime_env_are_preserved_and_input_drift_is_incomplete(self):
        for drift, reproduces in [(False, True), (True, True), (False, False)]:
            with self.subTest(drift=drift, reproduces=reproduces), tempfile.TemporaryDirectory() as tmp:
                target = Path(tmp)
                cli = target / "debug/infigraph"
                test = target / "debug/deps/cli_parity-fixture"
                test.parent.mkdir(parents=True)
                cli.write_bytes(b"original CLI")
                test.write_bytes(b"original test")
                cli.chmod(0o700)
                test.chmod(0o700)
                library = target / "native"
                library.mkdir()
                for name in ["liblbug.a", "lbug.h", "lbug.hpp"]:
                    (library / name).write_text("fixed native input")
                build = ["cargo", "test", "--workspace", "--test", "cli_parity", "--offline", "--no-run"]
                def command(argv, timeout, **kwargs):
                    if "--version" in argv:
                        return subprocess.CompletedProcess(argv, 0, "fixture compiler", "")
                    if argv == build:
                        self.assertEqual(timeout, 180)
                        self.assertEqual(kwargs["env"], dict(compiler_control.os.environ, CXX="/fixture/g++-13"))
                        saved = next((target / "ci-native-control").iterdir())
                        self.assertEqual((saved / "infigraph").read_bytes(), b"original CLI")
                        self.assertEqual((saved / "cli_parity").read_bytes(), b"original test")
                        cli.write_bytes(b"rebuilt CLI")
                        return subprocess.CompletedProcess(argv, 0, "fixture build", "")
                    self.assertEqual(timeout, 60)
                    self.assertEqual(argv[-1], "--test-threads=1")
                    self.assertNotIn("env", kwargs)  # No CXX override in runtime environment.
                    self.assertEqual(Path(argv[0]).read_bytes(), b"original test")
                    if cli.read_bytes() == b"original CLI":
                        return subprocess.CompletedProcess(argv, 101 if reproduces else 0,
                            "index failed:\nunix_signal: Some(11)" if reproduces else "fixture pass", "")
                    self.assertEqual(cli.read_bytes(), b"rebuilt CLI")
                    return subprocess.CompletedProcess(argv, 0, "fixture test result: 4 passed", "")
                receipt = io.StringIO()
                with patch.object(compiler_control.shutil, "which", return_value="/fixture/g++-13"), \
                        patch.object(compiler_control.shutil, "disk_usage", return_value=Mock(free=3 * 1024**3)), \
                        patch.object(compiler_control, "lbug_builds", return_value=[
                            (target / "output", {"LBUG_PRECOMPILED_LIBRARY_DIR": str(library),
                                                 "LBUG_PRECOMPILED_SOURCE": "release:fixture/latest"})]), \
                        patch.object(compiler_control, "feature_signature", side_effect=[
                            ["same"], ["changed" if drift else "same"]]), \
                        patch.object(compiler_control, "stdlib_identity", return_value="same-runtime-sha"), \
                        patch.object(compiler_control, "integration_compile_command", return_value=build), \
                        patch.object(compiler_control, "inspect"), \
                        patch.object(compiler_control, "bounded_command", side_effect=command), \
                        contextlib.redirect_stdout(receipt):
                    self.assertEqual(compiler_control.run(target), 1 if not reproduces else 2 if drift else 0)
                if not reproduces:
                    self.assertIn("NOT_REPRODUCED", receipt.getvalue())
                    self.assertEqual(cli.read_bytes(), b"original CLI")
                if drift:
                    self.assertIn("INCOMPLETE", receipt.getvalue())

    def test_missing_installed_compiler_does_not_mutate_or_build(self):
        with tempfile.TemporaryDirectory() as tmp, \
                patch.object(compiler_control.shutil, "which", return_value=None), \
                patch.object(compiler_control, "bounded_command") as command:
            with self.assertRaisesRegex(ValueError, "installed GCC13"):
                compiler_control.run(Path(tmp))
            command.assert_not_called()
            self.assertFalse((Path(tmp) / "ci-native-control").exists())


class DiagnosticTests(unittest.TestCase):
    def test_compiler_comparison_holds_source_archive_and_configuration_constant(self):
        with tempfile.TemporaryDirectory() as tmp:
            library = Path(tmp) / "native"
            library.mkdir()
            for name in ["lbug.hpp", "liblbug.a"]:
                (library / name).write_text("synthetic fixture")
            seen = []
            def command(argv, timeout):
                seen.append(argv)
                if "--version" in argv:
                    return subprocess.CompletedProcess(argv, 0, "fixture compiler", "")
                if "-o" in argv:
                    self.assertEqual(Path(argv[argv.index("-I") + 1]), library)
                    self.assertIn(str(library / "liblbug.a"), argv)
                    source = Path(argv[argv.index("-I") + 2]).read_text()
                    self.assertIn("std::format", source)
                    self.assertIn("lbug::main::Database db(argv[1]);", source)
                    self.assertEqual(timeout, 30)
                    return subprocess.CompletedProcess(argv, 0, "", "")
                self.assertEqual(timeout, 10)
                status = 0 if Path(argv[0]).name == "g++-13" else -11
                return subprocess.CompletedProcess(argv, status, "fixture stdout", "fixture stderr")
            receipt = io.StringIO()
            with patch.object(diagnose, "lbug_builds", return_value=[
                    (Path(tmp), {"LBUG_PRECOMPILED_LIBRARY_DIR": str(library)})]), \
                    patch.object(diagnose.shutil, "which", side_effect=lambda name: "/fixture/" + name), \
                    patch.object(diagnose, "bounded_command", side_effect=command), \
                    contextlib.redirect_stdout(receipt):
                diagnose.compare_compilers(Path(tmp))
            builds = [argv for argv in seen if "-o" in argv]
            self.assertEqual(len(builds), 2)
            self.assertEqual(builds[0][1:-1], builds[1][1:-1])
            self.assertIn("g++-13 run_exit=0 signal=None", receipt.getvalue())
            self.assertIn("g++-15 run_exit=-11 signal=11", receipt.getvalue())
            self.assertIn("not a CLI test pass", receipt.getvalue())

    def test_compiler_comparison_refuses_missing_tools_and_ambiguous_native_directory(self):
        for compiler, libraries in [(None, ["one"]), ("installed", ["one", "two"])]:
            receipt = io.StringIO()
            with patch.object(diagnose.shutil, "which", return_value=compiler), \
                    patch.object(diagnose, "lbug_builds", return_value=[
                        (Path("fixture"), {"LBUG_PRECOMPILED_LIBRARY_DIR": library}) for library in libraries]), \
                    patch.object(diagnose, "bounded_command") as run, contextlib.redirect_stdout(receipt):
                diagnose.compare_compilers(Path("fixture"))
            run.assert_not_called()
            self.assertIn("COMPILER_COMPARISON NOT_RUN", receipt.getvalue())

    def test_compiler_timeout_kills_process_group_before_reporting_incomplete(self):
        process = Mock(pid=12345)
        process.communicate.side_effect = [subprocess.TimeoutExpired("fixture", 1), ("", "")]
        with patch.object(diagnose.subprocess, "Popen", return_value=process) as spawn, \
                patch.object(diagnose.os, "killpg", create=True) as kill, \
                patch.object(diagnose.signal, "SIGKILL", 9, create=True):
            with self.assertRaises(subprocess.TimeoutExpired):
                diagnose.bounded_command(["fixture"], 1)
        self.assertTrue(spawn.call_args.kwargs["start_new_session"])
        kill.assert_called_once_with(12345, 9)
        self.assertEqual(process.communicate.call_count, 2)

    def test_version_success_from_lldb_helpers_does_not_select_them_as_debugger(self):
        # Reproduce the hosted listing/order: all helpers returned --version 0
        # and lldb-dap was previously selected, yielding no launch/backtrace.
        helpers = [Path("/fixture") / name for name in
                   ["lldb-dap-21", "lldb-instr-21", "lldb-server-21", "lldb-argdumper-21"]]
        cli = Path("/fixture/lldb-21")
        def glob(root, pattern):
            return helpers + [cli] if pattern.startswith("usr/bin/lldb-") else []
        receipt = io.StringIO()
        with patch.object(diagnose.shutil, "which", return_value=None), \
                patch.object(diagnose.Path, "glob", glob), \
                patch.object(diagnose.os, "access", return_value=True), \
                patch.object(diagnose.subprocess, "run", return_value=subprocess.CompletedProcess(
                    [], 0, "lldb version 21.1.8\n", "")) as run, contextlib.redirect_stdout(receipt):
            self.assertEqual(diagnose.debugger(), str(cli))
        run.assert_called_once_with([str(cli), "--version"], capture_output=True,
                                    text=True, timeout=5, check=False)
        self.assertIn("Selected debugger CLI: lldb-21", receipt.getvalue())

    def test_recorded_build_source_and_library_bytes_are_reported(self):
        with tempfile.TemporaryDirectory() as tmp:
            target = Path(tmp)
            library = target / "native"
            library.mkdir()
            archive = b"fixture archive"
            (library / "liblbug.a").write_bytes(archive)
            output = target / "debug/build/lbug-fixture/output"
            output.parent.mkdir(parents=True)
            output.write_text("cargo:rustc-env=LBUG_PRECOMPILED_SOURCE=release:fixture/latest\n"
                              f"cargo:rustc-env=LBUG_PRECOMPILED_LIBRARY_DIR={library}\n")
            receipt = io.StringIO()
            with contextlib.redirect_stdout(receipt), patch.object(diagnose.shutil, "which", return_value=None):
                diagnose.inspect(target)
            text = receipt.getvalue()
            self.assertIn("source: release:fixture/latest", text)
            self.assertIn(hashlib.sha256(archive).hexdigest(), text)
            self.assertIn("lbug.h: MISSING", text)
            self.assertIn("CLI exists: False", text)

    def test_missing_cli_is_distinguished_from_missing_debugger(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(sys, "argv", ["diagnose_cli.py"]), \
                patch.object(diagnose, "inspect", return_value=Path(tmp) / "absent"), \
                patch.object(diagnose, "debugger", return_value=None):
            with self.assertRaisesRegex(SystemExit, "CLI unavailable"):
                diagnose.main()
            cli = Path(tmp) / "cli"
            cli.write_text("fixture")
            cli.chmod(0o700)
            with patch.object(diagnose, "inspect", return_value=cli):
                with self.assertRaisesRegex(SystemExit, "neither usable gdb nor lldb"):
                    diagnose.main()

    def test_both_debuggers_use_exact_parity_fixture_and_bounded_execution(self):
        for tool in ["/fixture/gdb", "/fixture/lldb"]:
            with self.subTest(tool=tool), tempfile.TemporaryDirectory() as tmp:
                cli = Path(tmp) / "cli"
                cli.write_text("fixture")
                cli.chmod(0o700)

                def run(command, **kwargs):
                    fixture = Path(command[command.index("--root") + 1])
                    self.assertEqual((fixture / "src/calculator.py").read_text(),
                                     "def add(left, right):\n    return left + right\n")
                    self.assertEqual(command[-2:], ["index", "--no-embed"])
                    self.assertEqual(kwargs["timeout"], 90)
                    self.assertFalse(kwargs["check"])
                    return subprocess.CompletedProcess(command, 17)

                with patch.object(sys, "argv", ["diagnose_cli.py"]), \
                        patch.object(diagnose, "inspect", return_value=cli), \
                        patch.object(diagnose, "debugger", return_value=tool), \
                        patch.object(diagnose.subprocess, "run", side_effect=run):
                    with self.assertRaises(SystemExit) as result:
                        diagnose.main()
                    self.assertEqual(result.exception.code, 17)

    def test_timeout_is_reported_as_incomplete_evidence(self):
        with tempfile.TemporaryDirectory() as tmp:
            cli = Path(tmp) / "cli"
            cli.touch(mode=0o700)
            with patch.object(sys, "argv", ["diagnose_cli.py"]), \
                    patch.object(diagnose, "inspect", return_value=cli), \
                    patch.object(diagnose, "debugger", return_value="gdb"), \
                    patch.object(diagnose.subprocess, "run", side_effect=subprocess.TimeoutExpired("gdb", 90)):
                with self.assertRaisesRegex(SystemExit, "timed out; original verification failure retained"):
                    diagnose.main()


if __name__ == "__main__":
    unittest.main()

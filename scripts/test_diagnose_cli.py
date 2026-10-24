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
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("diagnose_cli", Path(__file__).with_name("diagnose_cli.py"))
diagnose = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diagnose)


class DiagnosticTests(unittest.TestCase):
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

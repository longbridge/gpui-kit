"""The Shell integration step must not hide an earlier Cargo failure."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ShellIntegrationExitStatus(unittest.TestCase):
    def test_cargo_failure_stops_the_workflow_step(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        step = re.search(
            r"^      - name: Test LLRT module integration\n((?:        .*\n)+)",
            workflow, re.MULTILINE,
        )
        self.assertIsNotNone(step)
        config, script = step.group(1).split("        run: |\n", 1)
        self.assertIn("        shell: bash\n", config)
        script = textwrap.dedent(script)
        commands = script.strip().splitlines()
        self.assertTrue(all(line.startswith("cargo test ") for line in commands))
        self.assertGreater(len(commands), 1)
        # Use Git Bash, not a possible WSL bash.exe, on Windows runners.
        bash = (str(Path(os.environ["ProgramFiles"]) / "Git/bin/bash.exe")
                if os.name == "nt" else shutil.which("bash"))
        self.assertIsNotNone(bash)

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "step.sh").write_text(script, newline="\n")
            cargo = root / "cargo"
            cargo.write_text(
                '#!/usr/bin/env bash\n'
                'echo "cargo $*" >> calls.txt\n'
                'if [[ "cargo $*" == "$FAIL_COMMAND" ]]; then exit 37; fi\n'
                'exit 0\n', newline="\n",
            )
            cargo.chmod(0o755)
            # Every command can fail, including one followed by successful commands.
            for failure in [None, *range(len(commands))]:
                with self.subTest(failure=failure):
                    trace = root / "calls.txt"
                    trace.unlink(missing_ok=True)
                    env = dict(os.environ, FAIL_COMMAND=(
                        commands[failure] if failure is not None else ""
                    ))
                    env["PATH"] = str(root) + os.pathsep + env["PATH"]
                    result = subprocess.run(
                        [bash, "--noprofile", "--norc", "-e", "-o", "pipefail", "step.sh"],
                        cwd=root, env=env, capture_output=True, text=True, timeout=30,
                    )
                    self.assertEqual(result.returncode, 37 if failure is not None else 0,
                                     result.stderr)
                    expected = commands[:failure + 1] if failure is not None else commands
                    self.assertEqual(trace.read_text().splitlines(), expected)


if __name__ == "__main__":
    unittest.main()

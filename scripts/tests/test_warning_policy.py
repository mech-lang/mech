import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CHECKER = ROOT / "scripts/check-warning-policy.py"


class WarningPolicyTests(unittest.TestCase):
    def fixture(self, source: str, lint_exceptions: list[dict]) -> Path:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        (root / ".cargo").mkdir()
        (root / "scripts").mkdir()
        (root / "src").mkdir()
        (root / ".cargo/config.toml").write_text(
            'rustflags = ["-D", "warnings"]\n'
            'rustdocflags = ["-D", "warnings"]\n',
            encoding="utf-8",
        )
        (root / "scripts/warning-exceptions.json").write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "lint_exceptions": lint_exceptions,
                    "deprecated_apis": [],
                }
            ),
            encoding="utf-8",
        )
        (root / "src/lib.rs").write_text(source, encoding="utf-8")
        return root

    def run_checker(self, root: Path) -> subprocess.CompletedProcess[str]:
        environment = dict(os.environ)
        environment["WARNING_POLICY_ROOT"] = str(root)
        return subprocess.run(
            ["python3", str(CHECKER)],
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_structural_scan_ignores_comments_and_literals(self):
        reason = "frozen compatibility name"
        root = self.fixture(
            f'''// #[allow(dead_code)]
const TEXT: &str = r##"#[expect(unused_variables)]"##;
#[expect(non_upper_case_globals, reason = "{reason}")]
pub const OldName: usize = 1;
''',
            [
                {
                    "directive": "expect",
                    "expiry_condition": "remove after the compatibility window",
                    "lint": "non_upper_case_globals",
                    "occurrences": 1,
                    "owner": "test owner",
                    "path": "src/lib.rs",
                    "reason": reason,
                }
            ],
        )
        process = self.run_checker(root)
        self.assertEqual(process.returncode, 0, process.stderr)

    def test_unreviewed_exception_is_rejected(self):
        root = self.fixture("#[allow(dead_code)]\nfn hidden() {}\n", [])
        process = self.run_checker(root)
        self.assertNotEqual(process.returncode, 0)
        self.assertIn("must name one lint and a literal reason", process.stderr)


if __name__ == "__main__":
    unittest.main()

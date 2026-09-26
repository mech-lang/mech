from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
import tomllib
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "check-production-resident-routing.py"
SPEC = importlib.util.spec_from_file_location("check_production_resident_routing", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECKER)
REPOSITORY = SCRIPT.parents[1]


class TerminalProductClosureTests(unittest.TestCase):
    def setUp(self):
        self.root = tomllib.loads((REPOSITORY / "Cargo.toml").read_text(encoding="utf-8"))
        self.terminal = tomllib.loads(
            (REPOSITORY / "hosts/terminal/Cargo.toml").read_text(encoding="utf-8")
        )

    def check(self, root=None, terminal=None):
        return CHECKER.check_terminal_product_closure(
            self.root if root is None else root,
            self.terminal if terminal is None else terminal,
        )

    def assert_failure(self, expected, root=None, terminal=None):
        failures = self.check(root, terminal)
        self.assertTrue(any(expected in failure for failure in failures), failures)

    def test_repository_terminal_closure_passes(self):
        self.assertEqual(self.check(), [])

    def test_coordinated_future_prerelease_does_not_require_checker_update(self):
        version = "0.5.0-rc.1"
        self.root["package"]["version"] = version
        self.root["dependencies"]["mech-terminal"]["version"] = version
        self.terminal["package"]["version"] = version
        self.assertEqual(self.check(), [])

    def test_stale_or_loose_dependency_version_fails(self):
        for version in ["0.3.5", "*", "^" + self.root["package"]["version"]]:
            with self.subTest(version=version):
                root = copy.deepcopy(self.root)
                root["dependencies"]["mech-terminal"]["version"] = version
                self.assert_failure("invalid mech-terminal dependency", root=root)

    def test_terminal_package_version_must_match_root(self):
        self.terminal["package"]["version"] = "0.0.0-stale"
        self.assert_failure("at root package version")

    def test_terminal_package_identity_is_required(self):
        self.terminal["package"]["name"] = "mech-browser"
        self.assert_failure("retained terminal package must be mech-terminal")

    def test_root_requires_explicit_nonempty_version(self):
        for version in [None, "", False, 4, {"workspace": True}]:
            with self.subTest(version=version):
                root = copy.deepcopy(self.root)
                root["package"]["version"] = version
                self.assert_failure("requires an explicit package version", root=root)
        del self.root["package"]["version"]
        self.assert_failure("requires an explicit package version")

    def test_dependency_path_and_flags_remain_exact(self):
        for key, value in [
            ("path", "hosts/browser"),
            ("default-features", True),
            ("optional", False),
            ("features", ["provider"]),
            ("package", "another-terminal"),
        ]:
            with self.subTest(key=key, value=value):
                root = copy.deepcopy(self.root)
                root["dependencies"]["mech-terminal"][key] = value
                self.assert_failure("invalid mech-terminal dependency", root=root)

    def test_missing_dependency_fields_fail(self):
        for key in ["version", "path", "default-features", "optional"]:
            with self.subTest(key=key):
                root = copy.deepcopy(self.root)
                del root["dependencies"]["mech-terminal"][key]
                self.assert_failure("invalid mech-terminal dependency", root=root)

    def test_missing_or_shorthand_dependency_fails(self):
        root = copy.deepcopy(self.root)
        del root["dependencies"]["mech-terminal"]
        self.assert_failure("invalid mech-terminal dependency", root=root)
        self.root["dependencies"]["mech-terminal"] = self.root["package"]["version"]
        self.assert_failure("invalid mech-terminal dependency")

    def test_cli_host_feature_closure_remains_exact(self):
        expected = self.root["features"]["cli_host"]
        for replacement in [[], expected[:-1], [*expected, "mech-terminal/default"]]:
            with self.subTest(replacement=replacement):
                root = copy.deepcopy(self.root)
                root["features"]["cli_host"] = replacement
                self.assert_failure("invalid cli_host feature", root=root)


if __name__ == "__main__":
    unittest.main()

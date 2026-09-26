from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import textwrap
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SCRIPTS = Path(__file__).resolve().parents[1]
ROOT = SCRIPTS.parent
sys.path.insert(0, str(SCRIPTS))
from release_metadata import release_channel, resolve_channel


def load_script(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), SCRIPTS / f"{name}.py")
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


TAG = load_script("check-release-tag")
PACKAGER = load_script("package-distribution")
BUILDER = load_script("build-distribution-artifact")


def job_block(source, name):
    match = re.search(rf"(?ms)^  {re.escape(name)}:\n(.*?)(?=^  [a-z0-9-]+:\n|\Z)", source)
    assert match is not None, f"missing job {name}"
    return match.group(1)


class ReleaseMetadataTests(unittest.TestCase):
    def test_semver_channel_including_build_metadata(self):
        for version in ("0.4.0", "1.2.3+build-beta", "0.4.0+001"):
            with self.subTest(version=version):
                self.assertEqual(release_channel(version), "stable")
        for version in ("0.4.0-beta", "0.4.0-beta.1", "0.4.0-rc.0+build.5", "0.4.0-0"):
            with self.subTest(version=version):
                self.assertEqual(release_channel(version), "preview")

    def test_invalid_versions_fail_closed(self):
        for version in ("v0.4.0-beta", "0.4", "01.4.0", "0.4.0-01", "0.4.0-beta.01",
                        "0.4.0-", "0.4.0+", "0.4.0-beta..1", "0.4.0-beta\n"):
            with self.subTest(version=version):
                with self.assertRaises(ValueError):
                    release_channel(version)

    def test_auto_resolves_and_explicit_channels_cannot_mislabel(self):
        for version, expected in (("0.4.0", "stable"), ("0.4.0-beta", "preview")):
            self.assertEqual(resolve_channel("auto", version), expected)
            self.assertEqual(resolve_channel(expected, version), expected)
            self.assertEqual(resolve_channel("nightly", version), "nightly")
            for wrong in {"stable", "preview", "unknown"} - {expected}:
                with self.subTest(version=version, wrong=wrong), self.assertRaises(ValueError):
                    resolve_channel(wrong, version)

    def test_packager_rejects_wrong_channel_before_writing(self):
        for version, channel in (("0.4.0-beta", "stable"), ("0.4.0", "preview")):
            with self.subTest(version=version), self.assertRaises(ValueError):
                PACKAGER.validate_args(SimpleNamespace(version=version, channel=channel))

    def test_builder_rejects_mislabeled_beta_before_build(self):
        args = SimpleNamespace(channel="stable", distribution="standard")
        with patch.object(BUILDER, "parse_args", return_value=args), \
             patch.object(BUILDER, "root_version", return_value="0.4.0-beta"), \
             patch.object(BUILDER, "build") as build, redirect_stderr(io.StringIO()):
            self.assertEqual(BUILDER.main(), 1)
            build.assert_not_called()

    def run_tag(self, version, tag, commits):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "github-output"
            with patch.object(TAG, "root_version", return_value=version), \
                 patch.object(TAG.subprocess, "check_output", side_effect=commits) as git, \
                 patch.object(sys, "argv", ["check-release-tag.py", "--tag", tag,
                                            "--github-output", str(output)]), \
                 redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
                status = TAG.main()
                return status, output.read_text() if output.exists() else "", git.call_count

    def test_exact_beta_tag_emits_preview_after_commit_check(self):
        self.assertEqual(self.run_tag("0.4.0-beta", "v0.4.0-beta", ["abc\n", "abc\n"]),
                         (0, "channel=preview\n", 2))

    def test_exact_stable_tag_emits_stable_after_commit_check(self):
        self.assertEqual(self.run_tag("0.4.0", "v0.4.0", ["abc\n", "abc\n"]),
                         (0, "channel=stable\n", 2))

    def test_version_mismatch_emits_no_channel(self):
        self.assertEqual(self.run_tag("0.4.0-beta", "v0.4.0", []), (1, "", 0))

    def test_wrong_commit_emits_no_channel(self):
        self.assertEqual(self.run_tag("0.4.0-beta", "v0.4.0-beta", ["abc\n", "def\n"]),
                         (1, "", 2))


class ReleaseWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.workflow = (ROOT / ".github/workflows/release.yml").read_text()

    def test_exact_tag_full_validation_still_gates_publication(self):
        verify = job_block(self.workflow, "verify-tag")
        full = job_block(self.workflow, "full-validation")
        publish = job_block(self.workflow, "publish")
        self.assertIn('scripts/check-release-tag.py --tag "$RELEASE_TAG" --github-output "$GITHUB_OUTPUT"', verify)
        self.assertIn("channel: ${{ steps.tag.outputs.channel }}", verify)
        self.assertIn("needs: verify-tag", full)
        self.assertIn("uses: ./.github/workflows/ci-full.yml", full)
        self.assertIn("release_channel: ${{ needs.verify-tag.outputs.channel }}", full)
        self.assertIn("needs:\n      - verify-tag\n      - full-validation", publish)
        self.assertIn("--verify-tag", publish)
        self.assertNotIn("continue-on-error", self.workflow)
        self.assertNotIn("if: always()", publish)

    def test_actual_publish_script_marks_preview_not_latest(self):
        publish = job_block(self.workflow, "publish")
        script = textwrap.dedent(publish.rsplit("        run: |\n", 1)[1])
        # Execute only the checked-in publication shell, replacing gh with an
        # in-process recorder. No network calls or release mutations occur.
        for channel in ("preview", "stable", "nightly", "unknown"):
            with self.subTest(channel=channel):
                environment = dict(os.environ, RELEASE_CHANNEL=channel, RELEASE_TAG="v0.4.0-beta")
                result = subprocess.run(
                    ["bash", "-e", "-c", 'gh() { printf "%s\\n" "$@"; }\n' + script],
                    cwd=ROOT, env=environment, capture_output=True, text=True,
                )
                if channel not in ("stable", "preview"):
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, "")
                    continue
                self.assertEqual(result.returncode, 0, result.stderr)
                arguments = result.stdout.splitlines()
                self.assertEqual(arguments[:3], ["release", "create", "v0.4.0-beta"])
                self.assertEqual("--prerelease" in arguments, channel == "preview")
                self.assertEqual("--latest=false" in arguments, channel == "preview")

    def test_normal_full_validation_auto_selects_package_channel(self):
        normal = (ROOT / ".github/workflows/ci.yml").read_text()
        full = (ROOT / ".github/workflows/ci-full.yml").read_text()
        self.assertIn("release_channel: auto", job_block(normal, "full-validation"))
        self.assertEqual(full.count("default: auto"), 2)
        self.assertIn("- preview", full)
        self.assertIn('scripts/tests/test_release_metadata.py', normal)
        self.assertIn('scripts/tests/test_release_metadata.py', full)

    def test_publish_uses_versioned_notes_with_general_fallback(self):
        publish = job_block(self.workflow, "publish")
        script = textwrap.dedent(publish.rsplit("        run: |\n", 1)[1])
        for available in (False, True):
            with self.subTest(available=available), tempfile.TemporaryDirectory() as directory:
                if available:
                    notes = Path(directory) / "docs/releases/v0.4.0-beta.md"
                    notes.parent.mkdir(parents=True)
                    notes.write_text("Preview qualification and limitations.\n")
                result = subprocess.run(
                    ["bash", "-e", "-c", 'gh() { printf "%s\\n" "$@"; }\n' + script],
                    cwd=directory,
                    env=dict(os.environ, RELEASE_CHANNEL="preview", RELEASE_TAG="v0.4.0-beta"),
                    capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                arguments = result.stdout.splitlines()
                self.assertEqual("--notes-file" in arguments, available)
                self.assertEqual("--notes" in arguments, not available)
                self.assertIn("--prerelease", arguments)
                self.assertIn("--latest=false", arguments)
                if available:
                    self.assertEqual(arguments[arguments.index("--notes-file") + 1],
                                     "docs/releases/v0.4.0-beta.md")


if __name__ == "__main__":
    unittest.main()

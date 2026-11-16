"""Executable archive compatibility and integrity tests; no compression runs."""
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest


CLI = Path(__file__).with_name("experiment.py")


def digest(data):
    return hashlib.sha256(data).hexdigest()


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cache = self.root / "cache"
        self.cache.mkdir()
        self.index = {"repository": "mech-lang/mech", "release_tag": "evidence", "rounds": {}}

    def archive(self, name, files):
        path = self.cache / name
        with tarfile.open(path, "w:gz") as archive:
            for name, data in files.items():
                info = tarfile.TarInfo(name)
                info.mode, info.size = 0o644, len(data)
                archive.addfile(info, io.BytesIO(data))
        return {"bytes": path.stat().st_size, "sha256": digest(path.read_bytes())}

    def current(self, number, missing=False, wrong_result=False):
        output = b'{"answer": 17}\n'
        script = ("from pathlib import Path\n"
                  "r=Path(__file__).resolve().parent\n"
                  "(r/'executed').write_text('yes')\n"
                  f"(r/'report.json').write_bytes({b'wrong' if wrong_result else output!r})\n").encode()
        files = {"replay.py": script, "report.json": output, "input.json": b"{}\n"}
        manifest = {"files": [{"path": name, "bytes": len(data), "sha256": digest(data)}
                              for name, data in files.items()]}
        contents = {"evidence/manifest.json": json.dumps(manifest).encode(),
                    **{"evidence/" + name: data for name, data in files.items()}}
        if missing:
            del contents["evidence/input.json"]
        asset = f"r{number}.tar.gz"
        receipt = self.archive(asset, contents)
        self.index["rounds"][str(number)] = {"review_packet": {
            **receipt, "url": f"https://github.com/mech-lang/mech/releases/download/evidence/{asset}",
            "files": len(files), "reports_replayed": 1,
            "replay": {"script": "replay.py", "expected_outputs": {"report.json": digest(output)}}}}

    def invoke(self, number, success=True):
        index = self.root / "index.json"
        index.write_text(json.dumps(self.index))
        output = self.root / f"round-{number}"
        result = subprocess.run([sys.executable, str(CLI), "replay-round", "--index", str(index),
                                 "--round", str(number), "--output", str(output),
                                 "--packet-cache", str(self.cache)], capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        self.assertNotIn("KeyError", result.stderr)
        return output, result

    def test_retained_and_current_schemas(self):
        for number in [256, 258, 259]:
            with self.subTest(round=number):
                self.current(number)
                output, _ = self.invoke(number)
                report = json.loads((output / "replay.json").read_text())
                self.assertEqual(report["verified_file_occurrences"], 3)
                self.assertEqual(report["byte_identical_outputs"], ["report.json"])
                self.assertTrue((output / "evidence/executed").exists())

    def test_bad_digest_rejects_before_execution(self):
        self.current(256)
        self.index["rounds"]["256"]["review_packet"]["sha256"] = "0" * 64
        output, result = self.invoke(256, False)
        self.assertIn("archive size or SHA-256 mismatch", result.stderr)
        self.assertFalse((output / "evidence/executed").exists())

    def test_missing_manifest_member_rejects_before_execution(self):
        self.current(258, missing=True)
        output, result = self.invoke(258, False)
        self.assertIn("manifest member", result.stderr)
        self.assertFalse((output / "evidence/executed").exists())

    def test_changed_replayed_report_is_rejected(self):
        self.current(259, wrong_result=True)
        output, result = self.invoke(259, False)
        self.assertIn("replayed output differs", result.stderr)
        self.assertTrue((output / "evidence/executed").exists())
        self.assertFalse((output / "replay.json").exists())

    def test_missing_recipe_is_an_error(self):
        self.current(256)
        del self.index["rounds"]["256"]["review_packet"]["replay"]
        _, result = self.invoke(256, False)
        # A missing required recipe is diagnosed explicitly, not silently skipped.
        self.assertIn("replay recipe", result.stderr)

    def test_legacy_events_keep_chronological_overlay_and_output_checks(self):
        script = b"from pathlib import Path\nimport sys\nr=Path(sys.argv[1]);o=Path(sys.argv[2]);o.mkdir();(o/'report.json').write_bytes((r/'input.json').read_bytes())\n"
        events = []
        for position, value in enumerate([b"17\n", b"29\n"]):
            files = {"replay.py": script, "input.json": value}
            rows = [{"path": name, "bytes": len(data), "sha256": digest(data), "mode": "100644",
                     "git_blob": hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest()}
                    for name, data in files.items()]
            manifest = {"source_revision": "test", "file_count": len(rows), "bytes": sum(row["bytes"] for row in rows), "files": rows}
            encoded = json.dumps(manifest).encode()
            asset = f"r79-{position}.tar.gz"
            receipt = self.archive(asset, {"MANIFEST.json": encoded, **files})
            events.append({**receipt, "asset": asset, "event": str(position), "source_revision": "test", "manifest_sha256": digest(encoded)})
        self.index["rounds"]["79"] = {"events": events, "analysis": {"script": "replay.py", "expected_outputs": {"report.json": digest(b"29\n")}}}
        output, _ = self.invoke(79)
        report = json.loads((output / "replay.json").read_text())
        self.assertEqual(report["verified_archives"], 2)
        self.assertEqual(report["paths_with_multiple_preserved_versions"], 1)
        self.assertEqual((output / "derived/report.json").read_bytes(), b"29\n")


if __name__ == "__main__":
    unittest.main()

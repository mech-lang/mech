"""Prepare a pinned, isolated paper-backend build without changing this checkout.

Prints the generated manifest location. Build separately with cargo; no benchmark
or compilation is started here. The snapshot is extracted directly from git and
never modifies the branch containing the benchmark harness.
"""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile


REVISION = "0d85319d60c460cdecb81e03c181d62ecfe1ef39"
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--refresh-manifest", action="store_true",
                        help="reuse this script's existing pinned snapshot")
    args = parser.parse_args()
    destination = args.destination or Path(
        tempfile.mkdtemp(prefix="mech-numpy-paper-0d85319d-", dir="/private/tmp")
    )
    destination.mkdir(parents=True, exist_ok=True)
    source = destination / "source"
    build = destination / "build"
    archive_command = [
        "git", "archive", REVISION, "Cargo.toml", "Cargo.lock", ".cargo",
        "src", "machines", "hosts/gpu", "tests/fixtures/resident-ekf",
    ]
    if args.refresh_manifest:
        prior = json.loads((destination / "build-provenance.json").read_text())
        assert prior["revision"] == REVISION
        assert source.is_dir() and build.is_dir()
    else:
        source.mkdir()
        build.mkdir()
        archive = subprocess.Popen(archive_command, cwd=ROOT, stdout=subprocess.PIPE)
        with tarfile.open(fileobj=archive.stdout, mode="r|") as files:
            files.extractall(source, filter="data")
        if archive.wait() != 0:
            raise RuntimeError("git archive failed")
    manifest = (HERE / "Cargo.toml.in").read_text()
    manifest = manifest.replace("@MECH_SOURCE@", str(source))
    manifest = manifest.replace("@DRIVER_SOURCE@", str(HERE / "src/main.rs"))
    manifest = manifest.replace("@RESIDENT_SOURCE@", str(HERE / "resident-probe/main.rs"))
    (build / "Cargo.toml").write_text(manifest)
    metadata = {
        "revision": REVISION,
        "source": str(source),
        "manifest": str(build / "Cargo.toml"),
        "archive_command": archive_command,
        "driver_source": str(HERE / "src/main.rs"),
        "resident_probe_source": str(HERE / "resident-probe/main.rs"),
        "fixture": str(HERE.parent / "mech-driver/ekf-numerical.mec"),
        "manifest_sha256": hashlib.sha256(manifest.encode()).hexdigest(),
    }
    (destination / "build-provenance.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps(metadata, indent=2))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Prove distribution archives are complete, named correctly, and reproducible."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path, PurePosixPath
import tarfile
import tempfile
from types import SimpleNamespace
import zipfile


ROOT = Path(__file__).resolve().parents[1]
PACKAGER = ROOT / "scripts/package-distribution.py"


class PackagingContractError(RuntimeError):
    pass


def load_packager():
    spec = importlib.util.spec_from_file_location("mech_distribution_packager", PACKAGER)
    if spec is None or spec.loader is None:
        raise PackagingContractError("cannot load distribution packager")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_archive(path: Path) -> dict[str, bytes]:
    result: dict[str, bytes] = {}
    if path.suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            for info in archive.infolist():
                if info.is_dir():
                    continue
                if info.date_time != (1980, 1, 1, 0, 0, 0):
                    raise PackagingContractError(f"zip timestamp is not normalized: {info.filename}")
                result[PurePosixPath(info.filename).name] = archive.read(info)
    else:
        with tarfile.open(path, mode="r:gz") as archive:
            for info in archive.getmembers():
                if info.isdir():
                    continue
                if info.mtime != 0 or info.uid != 0 or info.gid != 0:
                    raise PackagingContractError(f"tar metadata is not normalized: {info.name}")
                handle = archive.extractfile(info)
                if handle is None:
                    raise PackagingContractError(f"archive entry is unreadable: {info.name}")
                result[PurePosixPath(info.name).name] = handle.read()
    return result


def validate_payload(path: Path, executable: str, expected_manifest: dict[str, object]) -> None:
    files = read_archive(path)
    expected_files = {
        "LICENSE",
        "README.txt",
        "SHA256SUMS",
        "distribution-manifest.json",
        executable,
    }
    if set(files) != expected_files:
        raise PackagingContractError(f"archive file set drifted: {sorted(files)}")

    manifest = json.loads(files["distribution-manifest.json"])
    if manifest != expected_manifest:
        raise PackagingContractError(f"distribution manifest drifted: {manifest}")

    checksum_lines = files["SHA256SUMS"].decode("ascii").splitlines()
    checksums = {}
    for line in checksum_lines:
        digest, name = line.split("  ", 1)
        checksums[name] = digest
    expected_checksums = {
        name: digest_bytes(value)
        for name, value in files.items()
        if name != "SHA256SUMS"
    }
    if checksums != expected_checksums:
        raise PackagingContractError("embedded SHA256SUMS does not match the payload")


def prove_reproducible(packager, args: SimpleNamespace, expected_name: str) -> Path:
    with tempfile.TemporaryDirectory(prefix="mech-package-contract-") as temp:
        root = Path(temp)
        binary = root / "input-binary"
        binary.write_bytes(b"deterministic mech fixture\n")
        args.binary = binary
        args.output_dir = root / "first"
        first = packager.package(args)
        args.output_dir = root / "second"
        second = packager.package(args)
        if first.name != expected_name or second.name != expected_name:
            raise PackagingContractError(
                f"archive name drifted: {first.name}, {second.name}"
            )
        first_bytes = first.read_bytes()
        second_bytes = second.read_bytes()
        if first_bytes != second_bytes:
            raise PackagingContractError(f"archive is not reproducible: {expected_name}")
        retained = root / expected_name
        retained.write_bytes(first_bytes)
        validate_payload(retained, "mech.exe" if retained.suffix == ".zip" else "mech", args.manifest)
        return retained


def main() -> int:
    try:
        packager = load_packager()
        stable_manifest = {
            "channel": "stable",
            "commit": "0123456789abcdef",
            "distribution": "standard",
            "runtime_factory_count": 1300,
            "source_specializer_count": 63,
            "target": "x86_64-unknown-linux-gnu",
            "toolchain": "nightly-2026-03-03",
            "version": "0.3.6",
        }
        stable = SimpleNamespace(
            binary=None,
            channel="stable",
            distribution="standard",
            version="0.3.6",
            commit="0123456789abcdef",
            toolchain="nightly-2026-03-03",
            target="x86_64-unknown-linux-gnu",
            runtime_factory_count=1300,
            source_specializer_count=63,
            output_dir=None,
            date=None,
            manifest=stable_manifest,
        )
        prove_reproducible(
            packager,
            stable,
            "mech-0.3.6-standard-x86_64-unknown-linux-gnu.tar.gz",
        )

        nightly_manifest = {
            "channel": "nightly",
            "commit": "fedcba9876543210",
            "distribution": "full",
            "runtime_factory_count": 9022,
            "source_specializer_count": 119,
            "target": "x86_64-pc-windows-msvc",
            "toolchain": "nightly-2026-03-03",
            "version": "0.3.6",
        }
        nightly = SimpleNamespace(
            binary=None,
            channel="nightly",
            distribution="full",
            version="0.3.6",
            commit="fedcba9876543210",
            toolchain="nightly-2026-03-03",
            target="x86_64-pc-windows-msvc",
            runtime_factory_count=9022,
            source_specializer_count=119,
            output_dir=None,
            date="2026-08-06",
            manifest=nightly_manifest,
        )
        prove_reproducible(
            packager,
            nightly,
            "mech-nightly-2026-08-06-fedcba987-full-x86_64-pc-windows-msvc.zip",
        )
    except (PackagingContractError, OSError, ValueError, zipfile.BadZipFile, tarfile.TarError) as error:
        print(f"distribution packaging contract failed: {error}")
        return 1
    print("distribution packaging contract passed (stable/nightly, standard/full, tar/zip)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

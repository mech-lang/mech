#!/usr/bin/env python3
"""Create a deterministic Mech standard or full binary distribution archive."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import shutil
import tarfile
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[1]
LICENSE = ROOT / "LICENSE"
TOOL_NAME = "mech"
ARCHIVE_MTIME = 0
ZIP_TIMESTAMP = (1980, 1, 1, 0, 0, 0)


class DistributionPackageError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--channel", choices=("stable", "nightly"), required=True)
    parser.add_argument("--distribution", choices=("standard", "full"), required=True)
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--toolchain", required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--runtime-factory-count", type=int, required=True)
    parser.add_argument("--source-specializer-count", type=int, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument(
        "--date",
        help="UTC release date in YYYY-MM-DD form; required for nightly archives",
    )
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def archive_stem(args: argparse.Namespace) -> str:
    if args.channel == "stable":
        return f"mech-{args.version}-{args.distribution}-{args.target}"
    if not args.date:
        raise DistributionPackageError("--date is required for nightly archives")
    if len(args.commit) < 7:
        raise DistributionPackageError("--commit must contain at least seven characters")
    return (
        f"mech-nightly-{args.date}-{args.commit[:9]}-"
        f"{args.distribution}-{args.target}"
    )


def validate_args(args: argparse.Namespace) -> None:
    if not args.binary.is_file() or args.binary.is_symlink():
        raise DistributionPackageError(
            f"binary must be a regular, non-symlink file: {args.binary}"
        )
    if not LICENSE.is_file() or LICENSE.is_symlink():
        raise DistributionPackageError("repository LICENSE is missing or unsafe")
    if args.runtime_factory_count < 0 or args.source_specializer_count < 0:
        raise DistributionPackageError("surface counts must be non-negative")
    if args.channel == "stable" and args.date:
        raise DistributionPackageError("--date is only valid for nightly archives")


def write_payload(args: argparse.Namespace, directory: Path) -> None:
    executable = "mech.exe" if "windows" in args.target else TOOL_NAME
    shutil.copyfile(args.binary, directory / executable)
    os.chmod(directory / executable, 0o755)

    shutil.copyfile(LICENSE, directory / "LICENSE")
    os.chmod(directory / "LICENSE", 0o644)

    manifest = {
        "channel": args.channel,
        "commit": args.commit,
        "distribution": args.distribution,
        "runtime_factory_count": args.runtime_factory_count,
        "source_specializer_count": args.source_specializer_count,
        "target": args.target,
        "toolchain": args.toolchain,
        "version": args.version,
    }
    (directory / "distribution-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    (directory / "README.txt").write_text(
        "Mech binary distribution\n"
        "========================\n\n"
        f"Channel: {args.channel}\n"
        f"Distribution: {args.distribution}\n"
        f"Version: {args.version}\n"
        f"Commit: {args.commit}\n"
        f"Target: {args.target}\n\n"
        f"Place {executable} on PATH, then run `{executable} --version`.\n"
        "The standard distribution is recommended for normal use. The full\n"
        "distribution adds the broad scalar, machine, and robot-arm surface.\n",
        encoding="utf-8",
    )

    checksum_inputs = sorted(
        path for path in directory.iterdir() if path.name != "SHA256SUMS"
    )
    (directory / "SHA256SUMS").write_text(
        "".join(f"{sha256(path)}  {path.name}\n" for path in checksum_inputs),
        encoding="ascii",
    )


def tar_info(path: Path, arcname: str) -> tarfile.TarInfo:
    info = tarfile.TarInfo(arcname)
    info.size = path.stat().st_size
    info.mode = 0o755 if path.name in ("mech", "mech.exe") else 0o644
    info.mtime = ARCHIVE_MTIME
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    return info


def write_tar_gz(payload: Path, output: Path, stem: str) -> None:
    with output.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=ARCHIVE_MTIME) as zipped:
            with tarfile.open(fileobj=zipped, mode="w", format=tarfile.PAX_FORMAT) as archive:
                directory = tarfile.TarInfo(f"{stem}/")
                directory.type = tarfile.DIRTYPE
                directory.mode = 0o755
                directory.mtime = ARCHIVE_MTIME
                directory.uid = 0
                directory.gid = 0
                directory.uname = ""
                directory.gname = ""
                archive.addfile(directory)
                for path in sorted(payload.iterdir(), key=lambda item: item.name):
                    with path.open("rb") as handle:
                        archive.addfile(tar_info(path, f"{stem}/{path.name}"), handle)


def write_zip(payload: Path, output: Path, stem: str) -> None:
    with zipfile.ZipFile(
        output,
        mode="w",
        compression=zipfile.ZIP_DEFLATED,
        compresslevel=9,
    ) as archive:
        for path in sorted(payload.iterdir(), key=lambda item: item.name):
            info = zipfile.ZipInfo(f"{stem}/{path.name}", ZIP_TIMESTAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            mode = 0o755 if path.name in ("mech", "mech.exe") else 0o644
            info.external_attr = (mode & 0xFFFF) << 16
            info.create_system = 3
            archive.writestr(info, path.read_bytes(), compress_type=zipfile.ZIP_DEFLATED, compresslevel=9)


def package(args: argparse.Namespace) -> Path:
    validate_args(args)
    stem = archive_stem(args)
    extension = ".zip" if "windows" in args.target else ".tar.gz"
    args.output_dir.mkdir(parents=True, exist_ok=True)
    output = args.output_dir / f"{stem}{extension}"

    with tempfile.TemporaryDirectory(prefix="mech-package-") as temp:
        payload = Path(temp) / stem
        payload.mkdir()
        write_payload(args, payload)
        temporary_output = Path(temp) / output.name
        if extension == ".zip":
            write_zip(payload, temporary_output, stem)
        else:
            write_tar_gz(payload, temporary_output, stem)
        os.replace(temporary_output, output)
    return output


def main() -> int:
    try:
        output = package(parse_args())
    except (DistributionPackageError, OSError, ValueError) as error:
        print(f"distribution packaging failed: {error}")
        return 1
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

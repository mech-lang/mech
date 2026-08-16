#!/usr/bin/env python3
"""Install and verify only the tools that affect F0 measurements."""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

from f0_evidence import (
    PROTOCOL_VERSION,
    ROOT,
    EvidenceError,
    canonical_json_bytes,
    controlled_build_environment,
    controlled_thread_environment,
    environment_identity,
    load_json,
    physical_machine,
    sha256_file,
    uncontrolled_build_environment,
)


DEFAULT_MANIFEST = (
    ROOT / "tests/architecture/qualification/f0-measurement-toolchain.json"
)


def download(url: str, destination: Path, expected_sha256: str) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists() and sha256_file(destination) == expected_sha256:
        return
    temporary = destination.with_suffix(destination.suffix + ".partial")
    with urllib.request.urlopen(url) as response, temporary.open("wb") as output:
        shutil.copyfileobj(response, output)
    actual = sha256_file(temporary)
    if actual != expected_sha256:
        temporary.unlink(missing_ok=True)
        raise EvidenceError(
            f"downloaded NumPy wheel sha256 {actual} != {expected_sha256}"
        )
    temporary.replace(destination)


def install(lock: dict, installation_root: Path) -> None:
    python = lock["python"]
    executable = Path(python["executable"])
    if sha256_file(executable) != python["executable_sha256"]:
        raise EvidenceError("measurement Python executable hash changed")
    version = subprocess.check_output(
        [str(executable), "-I", "-c", "import platform; print(platform.python_version())"],
        text=True,
    ).strip()
    if version != python["version"]:
        raise EvidenceError(f"measurement Python {version} != {python['version']}")
    numpy = lock["numpy"]
    wheel = installation_root / "downloads" / numpy["archive"]
    download(numpy["url"], wheel, numpy["archive_sha256"])
    with zipfile.ZipFile(wheel) as contents:
        records = [name for name in contents.namelist() if name.endswith(".dist-info/RECORD")]
        if len(records) != 1:
            raise EvidenceError("locked NumPy wheel does not contain exactly one RECORD")
        if hashlib.sha256(contents.read(records[0])).hexdigest() != numpy["record_sha256"]:
            raise EvidenceError("locked NumPy wheel RECORD hash changed")
    environment = installation_root / "python"
    subprocess.run(
        [str(executable), "-I", "-m", "venv", "--clear", str(environment)],
        check=True,
    )
    subprocess.run(
        [
            str(environment / "bin/python"),
            "-I",
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--force-reinstall",
            "--no-compile",
            "--no-cache-dir",
            "--no-deps",
            "--no-index",
            str(wheel),
        ],
        check=True,
    )
    subprocess.run(
        ["rustup", "toolchain", "install", lock["rust"]["channel"], "--profile", "minimal"],
        check=True,
    )


def verified_numpy(lock: dict, installation_root: Path) -> dict:
    wheel = installation_root / "downloads" / lock["numpy"]["archive"]
    with zipfile.ZipFile(wheel) as contents:
        records = [name for name in contents.namelist() if name.endswith(".dist-info/RECORD")]
        if len(records) != 1:
            raise EvidenceError("locked NumPy wheel does not contain exactly one RECORD")
        record_bytes = contents.read(records[0])
    if hashlib.sha256(record_bytes).hexdigest() != lock["numpy"]["record_sha256"]:
        raise EvidenceError("locked NumPy wheel RECORD hash changed")
    python = installation_root / "python/bin/python"
    environment = (installation_root / "python").resolve()
    site_packages = Path(
        subprocess.check_output(
            [
                str(python),
                "-I",
                "-c",
                "import sysconfig; print(sysconfig.get_paths()['purelib'])",
            ],
            text=True,
        ).strip()
    ).resolve()
    module = Path(
        subprocess.check_output(
            [str(python), "-I", "-c", "import numpy; print(numpy.__file__)"],
            text=True,
        ).strip()
    ).resolve()
    if not module.is_relative_to(environment):
        raise EvidenceError("NumPy imported from outside the authenticated environment")
    module_relative = module.relative_to(environment).as_posix()
    if module_relative != lock["numpy"]["module_relative_path"]:
        raise EvidenceError("NumPy module path differs from the lock")
    verified = []
    for relative, encoded_hash, expected_size in csv.reader(
        record_bytes.decode("utf-8").splitlines()
    ):
        if not encoded_hash:
            continue
        path = site_packages.joinpath(*PurePosixPath(relative).parts).resolve()
        if not path.is_relative_to(environment):
            raise EvidenceError(f"NumPy RECORD escapes its environment: {relative}")
        algorithm, separator, encoded = encoded_hash.partition("=")
        if not separator or algorithm != "sha256":
            raise EvidenceError(f"unapproved NumPy RECORD hash: {relative}")
        payload = path.read_bytes()
        actual = base64.urlsafe_b64encode(hashlib.sha256(payload).digest()).rstrip(b"=").decode()
        if actual != encoded or len(payload) != int(expected_size):
            raise EvidenceError(f"installed NumPy differs from wheel RECORD: {relative}")
        verified.append((relative, hashlib.sha256(payload).hexdigest(), len(payload)))
    tree = hashlib.sha256(
        json.dumps(sorted(verified), separators=(",", ":")).encode()
    ).hexdigest()
    if tree != lock["numpy"]["installed_tree_sha256"]:
        raise EvidenceError("installed NumPy tree differs from the locked wheel")
    probe = """
import contextlib
import io
import json
import numpy
import platform
import warnings
output = io.StringIO()
with warnings.catch_warnings(), contextlib.redirect_stdout(output):
    warnings.simplefilter('ignore', UserWarning)
    numpy.show_config()
config_text = output.getvalue().strip()
config = getattr(numpy.__config__, 'CONFIG', {})
dependencies = config.get('Build Dependencies', {})
providers = sorted({
    dependency.get('name', 'unknown').lower()
    for kind, dependency in dependencies.items()
    if kind.lower() in {'blas', 'lapack'}
})
print(json.dumps({
    'python': platform.python_version(),
    'numpy': numpy.__version__,
    'blas_lapack_provider': ', '.join(providers),
    'numpy_config': config_text,
}, sort_keys=True))
"""
    result = json.loads(
        subprocess.check_output([str(python), "-I", "-c", probe], text=True)
    )
    if result["python"] != lock["python"]["version"]:
        raise EvidenceError("measurement Python version changed")
    if result["numpy"] != lock["numpy"]["version"]:
        raise EvidenceError("NumPy version changed")
    if result["blas_lapack_provider"] != lock["numpy"]["blas_lapack_provider"]:
        raise EvidenceError("NumPy BLAS/LAPACK provider changed")
    return {
        **result,
        "record_sha256": lock["numpy"]["record_sha256"],
        "installed_tree_sha256": tree,
        "module_relative_path": module_relative,
        "verified_file_count": len(verified),
    }


def cargo_home_configuration() -> dict:
    cargo_home = Path(os.environ.get("CARGO_HOME") or Path.home() / ".cargo").resolve()
    return {
        name: sha256_file(cargo_home / name) if (cargo_home / name).is_file() else None
        for name in ("config", "config.toml")
    }


def active_power_settings(output: str, source: str) -> dict[str, str]:
    active = False
    result = {}
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.endswith(":"):
            active = stripped.removesuffix(":") == source
        elif active:
            key, separator, value = stripped.rpartition(" ")
            if separator and key:
                result[key.strip()] = value.strip()
    return result


def verify(lock: dict, installation_root: Path) -> dict:
    errors = []
    executable = Path(lock["python"]["executable"])
    if Path(sys.executable).resolve() != executable.resolve():
        errors.append("measurement installer must run with the locked Python")
    if sha256_file(executable) != lock["python"]["executable_sha256"]:
        errors.append("measurement Python executable hash changed")
    machine = physical_machine()
    expected = lock["canonical_platform"]
    if platform.system() != expected["operating_system"]:
        errors.append("canonical qualification requires Darwin")
    if machine.get("architecture") != expected["architecture"]:
        errors.append("canonical qualification requires arm64")
    if machine.get("model_identifier") != expected["model_identifier"]:
        errors.append("canonical qualification machine changed")
    if sha256_file(ROOT / "Cargo.lock") != lock["cargo_lock_sha256"]:
        errors.append("Cargo.lock hash changed")
    rust = lock["rust"]
    rustc = subprocess.check_output(["rustc", f"+{rust['channel']}", "-Vv"], text=True).strip()
    cargo = subprocess.check_output(["cargo", f"+{rust['channel']}", "-V"], text=True).strip()
    if rust["rustc_commit"] not in rustc:
        errors.append("rustc commit changed")
    if rust["cargo_commit"] not in cargo:
        errors.append("cargo commit changed")
    try:
        numpy = verified_numpy(lock, installation_root)
    except (EvidenceError, OSError, subprocess.CalledProcessError) as error:
        errors.append(f"NumPy verification failed: {error}")
        numpy = {}
    threads = controlled_thread_environment()
    if threads != lock["thread_environment"]:
        errors.append(f"thread environment is not controlled: {threads}")
    compiler = controlled_build_environment()
    if compiler != lock["compiler_environment"]:
        errors.append(f"compiler environment is not controlled: {compiler}")
    uncontrolled = uncontrolled_build_environment(os.environ, lock["compiler_environment"])
    if uncontrolled:
        errors.append(f"uncontrolled build environment is present: {uncontrolled}")
    cargo_configuration = cargo_home_configuration()
    if cargo_configuration != lock["cargo_home_configuration"]:
        errors.append(f"Cargo home configuration changed: {cargo_configuration}")
    power_configuration = subprocess.check_output(["pmset", "-g", "custom"], text=True).strip()
    policy = lock["measurement_conditions"]
    settings = active_power_settings(power_configuration, policy["power_source"])
    for name, expected_value in policy["ac_power_settings"].items():
        if settings.get(name) != expected_value:
            errors.append(f"unapproved power setting {name}={settings.get(name)!r}")
    if errors:
        raise EvidenceError("; ".join(errors))
    fingerprint = {
        "protocol_version": PROTOCOL_VERSION,
        "machine": machine,
        "rustc": rustc,
        "cargo": cargo,
        "python": lock["python"],
        "numpy": numpy,
        "cargo_lock_sha256": lock["cargo_lock_sha256"],
        "thread_environment": threads,
        "compiler_environment": compiler,
        "cargo_home_configuration": cargo_configuration,
        "power_configuration": power_configuration,
    }
    return {
        "schema_version": 1,
        "qualification_environment_id": environment_identity(fingerprint),
        "fingerprint": fingerprint,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--install-root", type=Path, default=ROOT / "target/f0-toolchain")
    parser.add_argument("--verify-only", action="store_true")
    parser.add_argument("--write-environment", type=Path)
    args = parser.parse_args(argv)
    manifest = args.manifest if args.manifest.is_absolute() else ROOT / args.manifest
    installation_root = args.install_root.resolve()
    lock = load_json(manifest)
    if lock.get("protocol_version") != PROTOCOL_VERSION:
        print("F0 measurement protocol version changed", file=sys.stderr)
        return 1
    try:
        if not args.verify_only:
            install(lock, installation_root)
        environment = verify(lock, installation_root)
    except (EvidenceError, OSError, subprocess.CalledProcessError) as error:
        print(f"F0 measurement toolchain verification failed: {error}", file=sys.stderr)
        return 1
    if args.write_environment:
        output = args.write_environment.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(canonical_json_bytes(environment))
    print(f"F0 measurement toolchain verified: environment={environment['qualification_environment_id']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

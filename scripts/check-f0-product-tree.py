#!/usr/bin/env python3
"""Bind evidence-only F0 changes to the exact E4 shipping product tree."""

from __future__ import annotations

import argparse
import hashlib
import subprocess
from pathlib import Path

from f0_evidence import PRODUCT_TREE_MANIFEST, ROOT, EvidenceError, load_json


def protected(path: str, manifest: dict) -> bool:
    roots = tuple(f"{root.rstrip('/')}/" for root in manifest["protected_roots"])
    if path.startswith(roots):
        return True
    if path == "Cargo.lock" or path.endswith("/Cargo.lock"):
        return True
    if path == "Cargo.toml" or path.endswith("/Cargo.toml"):
        return True
    return any(
        path == exact or path.startswith(f"{exact.rstrip('/')}/")
        for exact in manifest["protected_paths"]
    )


def ls_tree(commit: str, root: Path = ROOT) -> list[tuple[str, str, str, str]]:
    process = subprocess.run(
        ["git", "ls-tree", "-r", "-z", "--full-tree", commit],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
    )
    rows = []
    for raw in process.stdout.split(b"\0"):
        if not raw:
            continue
        metadata, path = raw.decode("utf-8").split("\t", 1)
        mode, object_type, object_id = metadata.split(" ", 2)
        rows.append((mode, object_type, object_id, path))
    return rows


def protected_digest(commit: str, manifest: dict, root: Path = ROOT) -> str:
    digest = hashlib.sha256()
    selected = [row for row in ls_tree(commit, root) if protected(row[3], manifest)]
    for mode, object_type, object_id, path in sorted(selected, key=lambda row: row[3]):
        digest.update(f"{mode}\0{object_type}\0{object_id}\0{path}\0".encode())
    return digest.hexdigest()


def changed_protected_paths(base: str, head: str, manifest: dict, root: Path = ROOT) -> list[str]:
    output = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", base, head],
        cwd=root,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout
    return [path for path in output.splitlines() if protected(path, manifest)]


def validate(manifest: dict, head: str = "HEAD", root: Path = ROOT) -> list[str]:
    errors = []
    base = manifest["baseline_commit"]
    try:
        base_tree = subprocess.check_output(
            ["git", "rev-parse", f"{base}^{{tree}}"], cwd=root, text=True
        ).strip()
        head_tree_digest = protected_digest(head, manifest, root)
        paths = changed_protected_paths(base, head, manifest, root)
    except (OSError, subprocess.CalledProcessError, KeyError) as error:
        raise EvidenceError(f"product-tree guard could not inspect Git state: {error}") from error
    if base_tree != manifest["baseline_tree"]:
        errors.append(f"baseline tree {base_tree} != {manifest['baseline_tree']}")
    if head_tree_digest != manifest["protected_tree_sha256"]:
        errors.append(
            f"protected product digest {head_tree_digest} != "
            f"{manifest['protected_tree_sha256']}"
        )
    if paths:
        errors.append("F0 changed protected product paths: " + ", ".join(paths))
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=PRODUCT_TREE_MANIFEST)
    parser.add_argument("--head", default="HEAD")
    parser.add_argument("--print-digest", action="store_true")
    args = parser.parse_args(argv)
    path = args.manifest if args.manifest.is_absolute() else ROOT / args.manifest
    manifest = load_json(path)
    if args.print_digest:
        print(protected_digest(args.head, manifest))
        return 0
    errors = validate(manifest, args.head)
    if errors:
        print("F0 product-tree guard failed:")
        print(*errors, sep="\n")
        return 1
    print(
        "F0 product-tree guard passed: "
        f"{manifest['protected_tree_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

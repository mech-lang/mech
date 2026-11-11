#!/usr/bin/env python3
"""Recreate the saved 5k/20k/60k inputs from the pinned upstream generator.

Requires Faker==37.12.0, nanoid==2.0.0, tzdata==2026.6. The 5k fixture is
copied byte-for-byte from git. Generated inputs use the original generator,
seeded Python/Faker randomness, and a separately seeded nanoid byte source.
No benchmark source, checkout input, or global Python installation is changed.
"""

import argparse
import hashlib
import importlib
import json
from pathlib import Path
import random
import subprocess
import types

from faker import Faker

UPSTREAM_SHA = "fc2982b0c929b1b701209d401a1dc30c773ef4ab"


def pinned_file(upstream, name):
    return subprocess.check_output(["git", "-C", str(upstream), "show", f"{UPSTREAM_SHA}:{name}"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, required=True, help="Checkout containing the pinned upstream commit")
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "posts-5000.json").write_bytes(pinned_file(args.upstream, "posts.json"))
    (args.out / "correct-5000.json").write_bytes(pinned_file(args.upstream, "correct_related.json"))
    generator = types.ModuleType("gen_fake_posts")
    # Upstream uses PEP 604 annotations; postponed annotations also support 3.9.
    source = "from __future__ import annotations\n" + pinned_file(args.upstream, "gen_fake_posts.py").decode()
    exec(compile(source, "upstream/gen_fake_posts.py", "exec"), generator.__dict__)
    for count in (20000, 60000):
        seed = 20261102 + count
        random.seed(seed)
        Faker.seed(seed)
        id_rng = random.Random(seed + 1)
        importlib.import_module("nanoid.algorithm").urandom = lambda size: bytes(id_rng.randrange(256) for _ in range(size))
        destination = args.out / f"posts-{count}.json"
        # Redirect the original output helper rather than changing the checkout.
        generator.json_to_file = lambda _path, data: destination.write_text(json.dumps(data), encoding="utf-8")
        generator.gen_posts(count)
    manifest = {"upstream_sha": UPSTREAM_SHA, "datasets": []}
    for count in (5000, 20000, 60000):
        path = args.out / f"posts-{count}.json"
        posts = json.loads(path.read_text())
        manifest["datasets"].append({
            "posts": count, "filename": path.name,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "seed": None if count == 5000 else 20261102 + count,
            "id_seed": None if count == 5000 else 20261103 + count,
            "unique_tags": len({tag for post in posts for tag in post["tags"]}),
            "tag_counts": sorted({len(post["tags"]) for post in posts}),
            "bytes": path.stat().st_size,
        })
    text = json.dumps(manifest, indent=2) + "\n"
    (args.out / "manifest.json").write_text(text)
    print(text, end="")


if __name__ == "__main__":
    main()

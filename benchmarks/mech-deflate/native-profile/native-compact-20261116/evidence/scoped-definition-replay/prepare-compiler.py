"""Build the full control or parser candidate from the isolated review source."""
from argparse import ArgumentParser
from hashlib import file_digest
import os
from pathlib import Path
from shutil import copy2, copyfile
import subprocess


HERE = Path(__file__).resolve().parent
Q = HERE.parents[1]
CHECKOUT = Q / "native-review-pr/checkout"
PACKET = CHECKOUT / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
BRANCH = "refs/heads/codex/native-compact-source-handoff"
TARGET = HERE / "target"


def sha(path):
    with path.open("rb") as file:
        return file_digest(file, "sha256").hexdigest()


def git(*args):
    return subprocess.check_output(["git", *args], cwd=Q.parent)


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("mode", choices=("control", "candidate"))
    cli.add_argument("--patch", type=Path)
    cli.add_argument("--overlay", type=Path)
    args = cli.parse_args()
    pinned = git("rev-parse", BRANCH).decode().strip()
    assert pinned.startswith("d3c1203669")
    layout = CHECKOUT / "src/syntax/src/document/parser/journal/mod.rs"
    assert layout.read_bytes() == git("show", f"{pinned}:{layout.relative_to(CHECKOUT)}")
    proof = HERE / args.mode
    proof.mkdir(exist_ok=True)
    assert not any(proof.iterdir())
    if args.mode == "control":
        assert args.patch is None and args.overlay is None and not TARGET.exists()
        cached = HERE.parent / "facts-owner/target"
        subprocess.run(["/bin/cp", "-cR", cached, TARGET], check=True)
    else:
        assert args.patch is not None and args.overlay is not None and TARGET.is_dir()
        assert (HERE / "control/rebuild/rebuild.json").is_file()
        text = args.patch.read_text()
        paths = [line.removeprefix("+++ b/") for line in text.splitlines() if line.startswith("+++ b/")]
        assert len(paths) == len(set(paths)) and all(path.startswith("src/syntax/src/") for path in paths)
        sources = {name: args.overlay / Path(name).relative_to("src/syntax/src") for name in paths}
        assert all(path.is_file() for path in sources.values())
        for name in paths:
            current = CHECKOUT / name
            if current.exists() and current.read_bytes() != sources[name].read_bytes():
                assert current.read_bytes() == git("show", f"{pinned}:{name}")
        for name in paths:
            current = CHECKOUT / name
            current.parent.mkdir(parents=True, exist_ok=True)
            temp = current.with_suffix(".definition-replay")
            copyfile(sources[name], temp)
            os.replace(temp, current)
        copy2(args.patch, proof / "definition-replay.patch")

    command = [
        "python3", "-B", PACKET / "rebuild.py", proof / "rebuild",
        "--variant", "fresh-owner-counts-cold-tails-zero-branches", "--compiler-opt-level", "3",
        "--direct-source", "--preselect-root-inlining", "--skip-refused-general-ssa",
        "--target-dir", TARGET,
    ]
    with (proof / "rebuild.log").open("wb") as log:
        subprocess.run(command, env=os.environ | {"CARGO_NET_OFFLINE": "true"},
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    compiler = proof / "compile_profile"
    copy2(TARGET / "release/compile_profile", compiler)
    if args.mode == "candidate":
        assert sha(compiler) != sha(HERE / "control/compile_profile"), "cached control compiler reused"
    (proof / "compiler.sha256").write_text(sha(compiler) + "\n")
    print(compiler, sha(compiler), flush=True)


if __name__ == "__main__":
    main()

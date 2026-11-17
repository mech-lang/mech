"""Verify the source-emitted control and shared-copy objects through fixed linking."""

from argparse import ArgumentParser
from hashlib import file_digest
from pathlib import Path
import json
import os
import subprocess

study = Path(__file__).resolve().parent
q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
control = q / "native-size-20261117/source-aligned"
prototype = study / "static-candidate"


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def main():
    command = ArgumentParser(description=__doc__)
    command.add_argument("emitter", type=Path)
    command.add_argument("profile", type=Path)
    command.add_argument("output", type=Path)
    args = command.parse_args()
    output = args.output.resolve()
    output.mkdir()
    environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    original = json.loads((control / "runtime.json").read_text())
    bytecode = Path(original["emit"][1])
    assert sha(bytecode) == original["image_sha256"]
    build = {}
    for label, expected in (("before", control), ("shared-copies", prototype)):
        artifact = output / label
        artifact.mkdir()
        flags = original["flags"].copy()
        if label == "shared-copies":
            flags.update(MECH_COMPACT_SHARE_FRAME_COPIES="1", MECH_COMPACT_ROOT_TAIL_PROFILE=str(args.profile.resolve()))
        emit = [str(args.emitter.resolve()), str(bytecode), str(artifact)]
        with (artifact / "emit.log").open("wb") as log:
            subprocess.run(emit, cwd=q, env=environment | flags, check=True, stdout=log, stderr=subprocess.STDOUT)
        info = {name: sha(artifact / name) for name in ("program.o", "metadata.json")}
        build[label] = {"emit": emit, "flags": flags, "artifact_sha256": info}
        (output / "partial.json").write_text(json.dumps(build, indent=2) + "\n")
        assert info["program.o"] == sha(expected / "program.o"), (label, info["program.o"])
        assert json.loads((artifact / "metadata.json").read_text()) == json.loads((expected / "metadata.json").read_text())
        (artifact / "runtime.json").write_text(json.dumps(original | {"flags": flags, "emit": emit}, indent=2)+"\n")
        link = original["link"]
        for filename in ("program.o", "mech-worker"):
            old, new = str(control / filename), str(artifact / filename)
            assert sum(argument.count(old) for argument in link) == 1
            link = [argument.replace(old, new) for argument in link]
        subprocess.run(link, cwd=q, env=environment, check=True)
        assert sha(artifact / "mech-worker") == sha(expected / "mech-worker")
        build[label]["link"] = link
        build[label]["artifact_sha256"]["mech-worker"] = sha(artifact / "mech-worker")
        build[label]["metadata_byte_equivalent"] = info["metadata.json"] == sha(expected / "metadata.json")
    (output / "exact-source-identity.json").write_text(json.dumps(build, indent=2) + "\n")
    print("Maintained source reproduces both fixed objects and linked workers byte for byte.")


if __name__ == "__main__":
    main()

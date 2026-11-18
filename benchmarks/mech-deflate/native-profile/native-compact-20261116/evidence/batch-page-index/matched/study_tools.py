"""Build isolated parser probes with the measured profile and identical dependencies."""

from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess

study = Path(__file__).resolve().parent
q = study.parent
checkout = q / "native-review-pr/checkout"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
source = packet / "program.mec"
prior = q / "equivalent-vector-phases/parser-stages"
toolchain = json.loads((prior / "report.json").read_text())
expected = toolchain["trials"][0]
env = os.environ | {"CARGO_MANIFEST_DIR": str(checkout / "src/syntax"), "CARGO_PKG_VERSION": "0.3.5"}


def sha(file):
    with file.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def write(file, wanted):
    file.write_text(json.dumps(wanted, indent=2) + "\n")


def run(args, log):
    command = list(map(str, args))
    with log.open("wb") as stream:
        subprocess.run(command, env=env, stdout=stream, stderr=subprocess.STDOUT, check=True)
    return command


def replace(text, original, changed):
    assert text.count(original) == 1, original
    return text.replace(original, changed, 1)


def build(out, crate, mode):
    library = out / "libmech_syntax.rlib"
    build = [str(crate / "lib.rs") if arg.endswith("/syntax/lib.rs")
             else str(library) if arg.endswith("libmech_syntax_profile.rlib")
             else f"metadata={mode}" if arg == "metadata=parser-stages" else arg
             for arg in toolchain["build_command"]]
    run(build, out / "build.log")
    return library, build


def link(out, library):
    parser = out / "parser"
    link = [f"mech_syntax={library}" if arg.startswith("mech_syntax=")
            else str(parser) if arg == str(prior / "instrumented") else arg
            for arg in toolchain["link_commands"]["instrumented"]]
    run(link, out / "link.log")
    return parser, link


def probe(binary, out, input_file=source):
    command = run([binary, input_file, out], out.with_suffix(".log"))
    row = json.loads(out.read_text())
    if input_file == source:
        for name in ("tree_sha256", "source_sha256", "parser_steps", "events"):
            assert row[name] == expected[name], (name, row[name], expected[name])
    return {"command": command, **row}

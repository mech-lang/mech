"""Profile one prebuilt diagnostic compiler with the frozen direct-source recipe."""

import argparse
import json
import os
from pathlib import Path
import subprocess

from replay import digest


Q = Path("/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification")
REFERENCE = Q / "backend-work-hardened/worker-checks/refused-ssa"
SOURCE_SHA = "653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299"
OBJECT_SHA = "7bf9df1aa610ba130490c1092db15fceeace77609c16a60d35ff04e47a48ab7b"


def checked(command, log, env):
    with log.open("xb") as stream:
        subprocess.run(command, check=True, env=env, stdout=stream, stderr=subprocess.STDOUT)


def capture(compiler, output, role):
    runtime = REFERENCE / "runtime.json"
    plan = json.loads(runtime.read_text())
    source = Path(plan["emit"][1])
    assert digest(source) == SOURCE_SHA
    assert digest(REFERENCE / "program.o") == OBJECT_SHA
    assert plan["flags"]["MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA"] == "1"
    assert plan["flags"]["MECH_COMPACT_COMPILE_THREADS"] == "2"
    output.mkdir()
    environment = {key: value for key, value in os.environ.items() if not key.startswith("MECH_")}
    knobs = plan["flags"] | {"MECH_NATIVE_EXCLUSIVE_TRACE": str(output / "native-exclusive.json")}
    command = [str(compiler), str(source), str(output), "compact"]
    link = list(plan["link"])
    for name in ("program.o", "mech-worker"):
        original = str(REFERENCE / name)
        assert sum(value.count(original) for value in link) == 1, name
        link = [value.replace(original, str(output / name)) for value in link]
    record = {
        "role": role, "compiler_sha256": digest(compiler), "flags": knobs,
        "reference_recipe_sha256": digest(runtime), "source_sha256": SOURCE_SHA,
        "compile": command, "link": link,
    }
    (output / "capture-inputs.json").write_text(json.dumps(record, indent=2) + "\n")
    checked(command, output / "compile.log", environment | knobs | {"PATH": "/usr/bin:/bin"})
    assert digest(output / "program.mecb") == plan["image_sha256"]
    for name in ("program.o", "metadata.json"):
        assert digest(output / name) == digest(REFERENCE / name), name
    checked(link, output / "link.log", environment)
    assert digest(output / "mech-worker") == digest(REFERENCE / "mech-worker")
    print(f"Verified the exact compiled and linked output for {role} at {output}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("compiler", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--role", choices=("old", "shared"), required=True)
    parser.add_argument("--host-cleared", action="store_true")
    options = parser.parse_args()
    if not options.host_cleared:
        parser.error("the profile requires a quiet measurement slot")
    capture(options.compiler.resolve(strict=True), options.output.resolve(), options.role)


if __name__ == "__main__":
    main()

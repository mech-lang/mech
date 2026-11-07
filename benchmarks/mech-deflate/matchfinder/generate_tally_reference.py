#!/usr/bin/env python3
"""Execute the pinned forward tally/code builder on the qualified path nodes."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources
from generate_path_reference import pretty

FIELDS = ["freqs_litlen", "freqs_offset", "litlen_lens", "litlen_codewords",
          "offset_lens", "offset_codewords"]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_root", type=Path)
    parser.add_argument("--roc", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    root = args.reference_root.resolve()
    try:
        package = pinned_package_sources(root)
    except ValueError as error:
        parser.error(str(error))
    fixtures = Path(__file__).resolve().parents[3] / "hosts/gpu/tests/fixtures/native"
    saved = fixtures / "deflate-optimal-path-reference.json"
    paths = json.loads(saved.read_text())
    if paths["reference"]["source_pin"] != PIN:
        parser.error("path input fixture differs from the pinned revision")
    cases = [dict(name=c["name"], block_length=c["block_length"], nodes=c["nodes"])
             for c in paths["cases"]]
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import CompressOptimal
main! = |mode| {
'''
    for index, case in enumerate(cases):
        app += f'    if mode == {index} {{\n        nodes = ['
        app += ", ".join(f"{v}.U64" for v in case["nodes"]) + "]\n"
        app += f'        result = CompressOptimal.tally_and_build_codes(nodes, {case["block_length"]})\n'
        for field in FIELDS:
            app += f'''        var $i_{field} = 0.U64
        while $i_{field} < result.{field}.len() {{
            echo!("{field} ${{(List.get(result.{field}, $i_{field}) ?? 0).to_str()}}")
            $i_{field} = $i_{field} + 1
        }}
'''
        app += "    } else {}\n"
    app += "}\n"
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / "package/MechTallyOracle.roc"
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / "tally-reference"
    command = [str(args.roc), "build", "--opt=speed", f"--output={binary}", str(source)]
    subprocess.run(command, check=True)
    for index, case in enumerate(cases):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{case["name"]}.txt').write_text(raw)
        case.update({field: [] for field in FIELDS})
        for line in raw.splitlines():
            field, value = line.split()
            case[field].append(int(value))
        for field, size in zip(FIELDS, [513, 270, 288, 288, 32, 32]):
            if len(case[field]) != size:
                raise ValueError(f'unexpected {field} size for {case["name"]}')
    tables = package["package/DeflateTables.roc"].decode()
    literal = tables.split("length_slot_tab = [", 1)[1].split("]", 1)[0]
    lengths = [int(v) for v in re.findall(r"\d+", literal)]
    if len(lengths) != 259:
        raise ValueError("unexpected pinned length slot table")
    output = {"reference": {
        "source_pin": PIN, "compiler_pin": COMPILER_PIN,
        "package_source_sha256": {name: digest(data) for name, data in package.items()},
        "input_path_fixture_sha256": digest(saved.read_bytes()),
        "generated_source_sha256": digest(app.encode()),
        "binary_sha256": digest(binary.read_bytes()), "command": command,
    }, "length_slots": lengths, "offset_slots": paths["offset_slots"], "cases": cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + "\n")
    print(json.dumps({"cases": len(cases), "largest_block": max(c["block_length"] for c in cases),
                      "source_pin": PIN, "output": str(args.out)}))


if __name__ == "__main__":
    main()

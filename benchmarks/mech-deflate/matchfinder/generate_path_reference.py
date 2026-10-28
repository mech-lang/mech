#!/usr/bin/env python3
"""Execute the pinned backward path search with real-size cache/cost buffers."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pretty(value, level=0):
    indent = "  " * level
    child = indent + "  "
    if isinstance(value, dict):
        return "{\n" + ",\n".join(child + json.dumps(key) + ": " + pretty(item, level + 1)
                                   for key, item in value.items()) + "\n" + indent + "}"
    if isinstance(value, list) and value and isinstance(value[0], dict):
        return "[\n" + ",\n".join(child + pretty(item, level + 1) for item in value) + "\n" + indent + "]"
    return json.dumps(value)


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
    saved = fixtures / "deflate-block-reference.json"
    blocks = json.loads(saved.read_text())
    if blocks["reference"]["source_pin"] != PIN:
        parser.error("block reference inputs differ from the pinned revision")
    cases = [dict(name="empty", block_length=0, cache=[], cache_end=0, cache_size=1500513)]
    for block in blocks["cases"]:
        cases.append(dict(
            name=block["name"],
            block_length=block["scalars"]["block-end"] - block["scalars"]["block-begin"],
            cache=block["cache"], cache_end=block["scalars"]["cache-end"],
            cache_size=block["shapes"]["cache"],
        ))
    for case in cases:
        case.update(literal_costs=[128] * 256, length_costs=[144] * 259,
                    offset_costs=[78] * 32)
    cases.append({**cases[-1], "name": "320-tied-costs",
                  "length_costs": [128 * n for n in range(259)], "offset_costs": [0] * 32})
    cases.append({**cases[1], "name": "eight-wrapping-costs",
                  "literal_costs": [4294967280] * 256})
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import CompressOptimal
main! = |mode| {
'''
    for index, case in enumerate(cases):
        app += f'    if mode == {index} {{\n        var $cache = List.repeat(0.U32, {case["cache_size"]})\n'
        for at, value in case["cache"]:
            app += f'''        $cache = match List.set($cache, {at}, {value}.U32) {{
            Ok(next) => next
            Err(_) => crash "reference cache input"
        }}
'''
        for name in ["literal_costs", "length_costs", "offset_costs"]:
            app += f'        {name} = [' + ", ".join(f"{value}.U32" for value in case[name]) + "]\n"
        app += f'        result = CompressOptimal.find_min_cost_path({case["block_length"]}, $cache, {case["cache_end"]}, List.with_capacity(305257), literal_costs, length_costs, offset_costs)\n'
        app += '''        var $at = 0.U64
        while $at < result.nodes.len() {
            echo!("${(List.get(result.nodes, $at) ?? 0).to_str()}")
            $at = $at + 1
        }
    } else {}
'''
    app += "}\n"
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / "package/MechPathOracle.roc"
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / "path-reference"
    command = [str(args.roc), "build", "--opt=speed", f"--output={binary}", str(source)]
    subprocess.run(command, check=True)
    for index, case in enumerate(cases):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{case["name"]}.txt').write_text(raw)
        case["nodes"] = [int(line) for line in raw.splitlines()]
        if len(case["nodes"]) != case["block_length"] + 258:
            raise ValueError(f'unexpected node count for {case["name"]}')
    tables = package["package/DeflateTables.roc"].decode()
    literal = tables.split("offset_slot_tab = [", 1)[1].split("]", 1)[0]
    slots = [int(value) for value in re.findall(r"\d+", literal)]
    if len(slots) != 256:
        raise ValueError("unexpected pinned offset slot table")
    output = {"reference": {
        "source_pin": PIN, "compiler_pin": COMPILER_PIN,
        "package_source_sha256": {name: digest(data) for name, data in package.items()},
        "input_block_fixture_sha256": digest(saved.read_bytes()),
        "generated_source_sha256": digest(app.encode()),
        "binary_sha256": digest(binary.read_bytes()), "command": command,
    }, "offset_slots": slots, "cases": cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + "\n")
    print(json.dumps({"cases": len(cases), "largest_block": max(c["block_length"] for c in cases),
                      "output": str(args.out), "source_pin": PIN}))


if __name__ == "__main__":
    main()

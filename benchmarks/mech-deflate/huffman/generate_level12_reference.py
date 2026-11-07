#!/usr/bin/env python3
"""Execute the pinned Huffman builder with the level-12 allocation layout."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "matchfinder"))
from generate_block_reference import COMPILER_PIN, PIN, pinned_package_sources
from generate_path_reference import pretty


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
    saved = fixtures / "deflate-huffman-reference.json"
    old = json.loads(saved.read_text())
    cases = []
    for case in old["cases"]:
        symbols = case["num_symbols"]
        if symbols == 2:
            continue  # Level 12 calls only the 19, 32, and 288 symbol alphabets.
        n = 32 if symbols == 30 else symbols
        size = 513 if n == 288 else 270 if n == 32 else 19
        freqs = case["frequencies"][:symbols] + [0] * (size - symbols)
        cases.append(dict(name=case["name"], n=n,
                          maximum=14 if n == 288 else 15 if n == 32 else 7,
                          frequencies=freqs))
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import HuffmanEncode
main! = |mode| {
'''
    for index, case in enumerate(cases):
        app += f'    if mode == {index} {{\n        frequencies = ['
        app += ", ".join(f"{v}.U32" for v in case["frequencies"]) + "]\n"
        app += f'        code = HuffmanEncode.make_code({case["n"]}, {case["maximum"]}, frequencies, List.repeat(0.U8, {case["n"]}), List.repeat(0.U32, {case["n"]}))\n'
        app += '''        var $at = 0.U64
        while $at < code.lens.len() {
            echo!("${(List.get(code.lens, $at) ?? 0).to_str()} ${(List.get(code.codewords, $at) ?? 0).to_str()}")
            $at = $at + 1
        }
    } else {}
'''
    app += f'''    if mode == {len(cases)} {{
        var $at = 0.U64
        while $at < HuffmanEncode.bitreverse_tab.len() {{
            echo!("${{(List.get(HuffmanEncode.bitreverse_tab, $at) ?? 0).to_str()}}")
            $at = $at + 1
        }}
    }} else {{}}
}}
'''
    args.work_dir.mkdir(parents=True, exist_ok=True)
    source = root / "package/MechLevel12HuffmanOracle.roc"
    source.write_text(app)
    (args.work_dir / source.name).write_text(app)
    binary = args.work_dir.resolve() / "huffman-reference"
    command = [str(args.roc), "build", "--opt=speed", f"--output={binary}", str(source)]
    subprocess.run(command, check=True)
    for index, case in enumerate(cases):
        raw = subprocess.check_output([str(binary), str(index)], text=True)
        (args.work_dir / f'{case["name"]}.txt').write_text(raw)
        rows = [list(map(int, line.split())) for line in raw.splitlines()]
        if len(rows) != case["n"] or any(len(row) != 2 for row in rows):
            raise ValueError(f'unexpected code size for {case["name"]}')
        case["lens"] = [row[0] for row in rows]
        case["words"] = [row[1] for row in rows]
    raw = subprocess.check_output([str(binary), str(len(cases))], text=True)
    (args.work_dir / "reversal-table.txt").write_text(raw)
    reversal = [int(line) for line in raw.splitlines()]
    if len(reversal) != 256:
        raise ValueError("unexpected pinned reversal table size")
    output = {"reference": {
        "source_pin": PIN, "compiler_pin": COMPILER_PIN,
        "package_source_sha256": {name: digest(data) for name, data in package.items()},
        "input_frequency_fixture_sha256": digest(saved.read_bytes()),
        "generated_source_sha256": digest(app.encode()),
        "binary_sha256": digest(binary.read_bytes()), "command": command,
    }, "reversal_table": reversal, "cases": cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(pretty(output) + "\n")
    print(json.dumps({"cases": len(cases), "alphabet_sizes": sorted({c["n"] for c in cases}),
                      "frequency_sizes": sorted({len(c["frequencies"]) for c in cases}),
                      "source_pin": PIN, "output": str(args.out)}))


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Observe the exact pinned first-block loop, before parsing or block writing."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

PIN = "671077457ff7ef3a3d44caf715a3fc595addf379"
COMPILER_PIN = "d99c7b838fb05bb1047eb0c18d4d06d0f86f345d"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def pinned_package_sources(root, revision=PIN):
    """Check the full tracked package, including transitive oracle imports."""
    names = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", revision, "--", "package"],
        cwd=root, text=True,
    ).splitlines()
    sources = {}
    for name in names:
        if not name.endswith(".roc"):
            continue
        pinned = subprocess.check_output(["git", "show", f"{revision}:{name}"], cwd=root)
        try:
            actual = (root / name).read_bytes()
        except FileNotFoundError as error:
            raise ValueError(f"pinned package source is missing: {name}") from error
        if actual != pinned:
            raise ValueError(f"package source differs from the pin: {name}")
        sources[name] = pinned
    if "package/CompressOptimal.roc" not in sources:
        raise ValueError("pinned collection source is absent from the package")
    return sources


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_root", type=Path)
    parser.add_argument("--roc", required=True, type=Path)
    parser.add_argument("--work-dir", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()
    root = args.reference_root.resolve()
    try:
        package_sources = pinned_package_sources(root)
    except ValueError as error:
        parser.error(str(error))
    pinned = package_sources["package/CompressOptimal.roc"]
    source = pinned.decode()
    start = source.index("\t\t\t# Starting a new block.")
    end = source.index("\t\t\tblock_length = $block_end - $in_block_begin", start)
    loop = source[start:end]
    # This is the complete original first-block body, including search/skip,
    # statistics, decisions, rewind and final merging. Only its entry state and
    # observation point are supplied here; no parser or writer is substituted.
    app = '''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import Matchfinder
import BtMatchfinder
import DeflateTables
import CompressLazy
import CompressOptimal
collect = |input| {
    in_end = List.len(input)
    params = { max_search_depth: 300.U64, nice_match_length: 258.U64 }
    var $mf = Matchfinder.init_nodes(262144)
    var $nh3 = 0.U64
    var $nh4 = 0.U64
    var $cache = List.repeat(0.U32, 1500513)
    var $cache_ptr = 0.U64
    var $match_len_freqs = List.repeat(0.U32, 259)
    var $new_match_len_freqs = List.repeat(0.U32, 259)
    var $new_observations = List.repeat(0.U32, 10)
    var $observations = List.repeat(0.U32, 10)
    var $num_new_observations = 0.U64
    var $num_observations = 0.U64
    var $in_next = 0.U64
    var $in_block_begin = 0.U64
    var $in_cur_base = 0.U64
    var $in_next_slide = in_end.min(32768)
    var $max_len = 258.U64
    var $nice_len = params.nice_match_length.min(258)
    var $prev_block_used_only_literals = 0.U64
'''
    scalars = {
        "next-position": "$in_next", "cache-ptr": "$cache_ptr",
        "next-hash3": "$nh3", "next-hash4": "$nh4", "base": "$in_cur_base",
        "next-slide": "$in_next_slide", "maximum": "$max_len", "nice": "$nice_len",
        "number-observations": "$num_observations", "number-fresh": "$num_new_observations",
        "block-begin": "$in_block_begin", "block-end": "$block_end", "cache-end": "$cache_end",
        "rewound": "$rewound", "is-final": "$is_final", "changed": "$change_detected",
        "previous-check": "$prev_end_block_check", "have-previous-check": "$have_prev_end_block_check",
    }
    vectors = {"tables": "$mf", "cache": "$cache", "frequencies": "$match_len_freqs",
               "fresh-frequencies": "$new_match_len_freqs", "observations": "$observations",
               "fresh-observations": "$new_observations"}
    fields = {**vectors, **scalars}
    app += loop + "    { " + ", ".join(f"{key.replace('-', '_')}: {value}" for key, value in fields.items()) + " }\n}\n"
    app += '''main! = |mode| {
    input = if mode == 0 { "abcdefgh".to_utf8() } else if mode == 1 { List.repeat(97.U8, 8) } else { List.repeat(97.U8, 320) }
    state = collect(input)
'''
    for key in scalars:
        field = key.replace("-", "_")
        app += f'    echo!("scalar\\t{key}\\t${{state.{field}.to_str()}}")\n'
    for key in vectors:
        field = key.replace("-", "_")
        app += f'''    echo!("shape\\t{key}\\t${{state.{field}.len().to_str()}}")
    var $i_{field} = 0.U64
    while $i_{field} < state.{field}.len() {{
        value = (List.get(state.{field}, $i_{field}) ?? 0).to_u64()
        if value != 0 {{ echo!("{key}\\t${{$i_{field}.to_str()}}\\t${{value.to_str()}}") }} else {{}}
        $i_{field} = $i_{field} + 1
    }}
'''
    app += "}\n"
    args.work_dir.mkdir(parents=True, exist_ok=True)
    generated = root / "package/MechFullBlockOracle.roc"
    generated.write_text(app)
    (args.work_dir / generated.name).write_text(app)
    binary = args.work_dir.resolve() / "full-block-oracle"
    command = [str(args.roc), "build", "--opt=speed", f"--output={binary}", str(generated)]
    subprocess.run(command, check=True)
    cases = []
    for mode, name, data in [(0, "eight-literals", list(b"abcdefgh")), (1, "eight-overlap", [97]*8), (2, "320-overlap", [97]*320)]:
        raw = subprocess.check_output([str(binary), str(mode)], text=True)
        (args.work_dir / f"{name}.tsv").write_text(raw)
        case = {"name": name, "input": data, "scalars": {}, "shapes": {}, **{key: [] for key in vectors}}
        for line in raw.splitlines():
            kind, key, value = line.split("\t")
            if kind in ("scalar", "shape"):
                case["scalars" if kind == "scalar" else "shapes"][key] = int(value)
            else:
                case[kind].append([int(key), int(value)])
        cases.append(case)
    output = {"reference": {"source_pin": PIN, "compiler_pin": COMPILER_PIN,
                           "package_source_sha256": {name: digest(data) for name, data in package_sources.items()},
                           "source_sha256": digest(pinned), "exact_block_fragment_sha256": digest(loop.encode()),
                           "generated_source_sha256": digest(app.encode()), "binary_sha256": digest(binary.read_bytes()),
                           "command": command, "purpose": "first complete block before optimal parsing/writing"}, "cases": cases}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(output, indent=2) + "\n")
    print(json.dumps({"cases": len(cases), "output": str(args.out), "source_pin": PIN}))


if __name__ == "__main__":
    main()

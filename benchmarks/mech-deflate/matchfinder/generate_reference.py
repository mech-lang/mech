#!/usr/bin/env python3
"""Execute the pinned libdeflate match-finder helpers to produce test fixtures."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile

PIN = "b122c8be1d78b19f6d0a6efc5bb79bfcbb30dd51"
HEADERS_PIN = "4fabfa52e359977973c0402b1ff12e22c4d2396dfa89375056c766be471b742b"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def matching_cases():
    rng = random.Random(0xDEF1A7E)
    settings = [(0, 0, 0, 1, 0)]
    for maximum in [1, 7, 8, 9, 31, 32, 33, 257, 258]:
        settings.append((maximum, maximum, 0, 17, maximum % 8))
    for position in list(range(9)) + list(range(24, 40)) + list(range(248, 258)):
        settings.append((258, position, min(position, 3), 3, position % 8))
    for _ in range(25):
        maximum = rng.randrange(259)
        mismatch = rng.randrange(maximum + 1)
        settings.append((maximum, mismatch, rng.randrange(mismatch + 1),
                         rng.choice([1, 3, 8, 17, 257]), rng.randrange(8)))
    for index, (maximum, mismatch, start, distance, alignment) in enumerate(settings):
        str_at = 288 + alignment
        match_at = str_at - distance
        data = [rng.randrange(256) for _ in range(str_at + maximum)]
        # Copy forwards, preserving deliberately overlapping matches.
        for offset in range(maximum):
            data[str_at + offset] = data[match_at + offset]
        if mismatch < maximum:
            data[str_at + mismatch] ^= 1 << (index % 8)
        sequence = [0, 0xFFFFFFFF, 0x80FE007F, rng.randrange(1 << 32)][index % 4]
        yield dict(name=f"extend-{index}", sequence=sequence,
                   hash_bits=[1, 8, 15, 17, 24, 31, 32][index % 7], input=data,
                   str_at=str_at, match_at=match_at, start=start, maximum=maximum)


DRIVER = r'''
#include <stdio.h>
#include <stdlib.h>
#include SOURCE_FILE
int main(int argc, char **argv) {
    if (argc != 2) return 2;
    if (argv[1][0] == 'e') {
        unsigned seq, bits, str_at, match_at, start, maximum, size;
        u8 data[1024];
        if (scanf("%u%u%u%u%u%u%u", &seq, &bits, &str_at, &match_at,
                  &start, &maximum, &size) != 7) return 3;
        if (bits < 1 || bits > 32 || start > maximum || match_at >= str_at ||
            str_at + maximum > size || size > sizeof(data)) return 4;
        for (unsigned i = 0, v; i < size; i++) {
            if (scanf("%u", &v) != 1 || v > 255) return 5;
            data[i] = v;
        }
        printf("%u %u\n", lz_hash(seq, bits),
               lz_extend(data + str_at, data + match_at, start, maximum));
    } else {
        mf_pos_t nodes[512] __attribute__((aligned(32)));
        for (unsigned i = 0, v; i < 512; i++) {
            if (scanf("%u", &v) != 1 || v > 65535) return 6;
            nodes[i] = (mf_pos_t)((int)v - 32768);
        }
        matchfinder_rebase(nodes, sizeof(nodes));
        for (unsigned i = 0; i < 512; i++) printf("%u\n", (unsigned)((int)nodes[i] + 32768));
        matchfinder_init(nodes, sizeof(nodes));
        for (unsigned i = 0; i < 512; i++) if (nodes[i] != -32768) return 7;
    }
    return 0;
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_root", type=Path)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parents[3]
                        / "hosts/gpu/tests/fixtures/native/deflate-matchfinder-reference.json")
    args = parser.parse_args()
    root = args.reference_root.resolve()
    source = root / "lib/deflate_compress.c"
    headers = {str(path.relative_to(root)): digest(path) for path in sorted(root.rglob("*.h"))}
    manifest = dict(source_sha256=digest(source), header_sha256=headers)
    actual = hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    if actual != HEADERS_PIN:
        parser.error(f"source and headers do not match pinned libdeflate {PIN}")
    flags = ["-O3", "-DNDEBUG", "-ffunction-sections", "-fdata-sections", f"-I{root}"]
    flags += ["-Wl,-dead_strip"] if sys.platform == "darwin" else ["-Wl,--gc-sections"]
    cases = list(matching_cases())
    rng = random.Random(32768)
    nodes = [0, 1, 32767, 32768, 32769, 65534, 65535] + [rng.randrange(65536) for _ in range(505)]
    with tempfile.TemporaryDirectory(prefix="mech-matchfinder-oracle-") as temporary:
        driver, binary = Path(temporary) / "oracle.c", Path(temporary) / "oracle"
        driver.write_text(DRIVER.replace("SOURCE_FILE", json.dumps(str(source))))
        subprocess.run([args.cc, *flags, str(driver), "-o", str(binary)], check=True)
        for case in cases:
            fields = [case[key] for key in ["sequence", "hash_bits", "str_at", "match_at", "start", "maximum"]]
            result = subprocess.check_output([str(binary), "extend"], text=True,
                input=" ".join(map(str, fields + [len(case["input"])] + case["input"])))
            case["hash"], case["matched"] = map(int, result.split())
            prefix = case["start"]
            while prefix < case["maximum"] and case["input"][case["str_at"] + prefix] == case["input"][case["match_at"] + prefix]:
                prefix += 1
            assert case["matched"] == prefix
            assert case["hash"] == ((case["sequence"] * 506832829) & 0xFFFFFFFF) >> (32 - case["hash_bits"])
        rebased = list(map(int, subprocess.check_output([str(binary), "nodes"], text=True,
            input=" ".join(map(str, nodes))).split()))
        assert rebased == [max(0, node - 32768) for node in nodes]
        binary_hash = digest(binary)
    result = dict(reference=dict(project="libdeflate", commit=PIN,
        functions=["lz_hash", "lz_extend", "matchfinder_init", "matchfinder_rebase"],
        **manifest, compiler=subprocess.check_output([args.cc, "--version"], text=True).splitlines()[0],
        flags=flags, binary_sha256=binary_hash), cases=cases, nodes=nodes, rebased=rebased)
    # One case per line keeps the immutable byte fixtures reviewable without
    # exploding every byte into a separate diff line.
    text = json.dumps(result, separators=(",", ":"))
    args.out.write_text(text.replace('},{"name":', '},\n{"name":') + "\n")
    print(f"Wrote {len(cases)} match cases and 512 node entries to {args.out}")


if __name__ == "__main__":
    main()

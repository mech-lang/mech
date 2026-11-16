#!/usr/bin/env python3
"""Snapshot complete pinned C binary-tree search/skip transitions in Roc's layout."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile

from generate_reference import HEADERS_PIN, PIN, digest


DRIVER = r'''
#include <stdio.h>
#include <stdlib.h>
#include SOURCE_FILE
static struct bt_matchfinder mf;
static u8 data[40000];
static struct lz_match matches[258];
static unsigned biased(unsigned i) {
    mf_pos_t value;
    if (i < 65536) value = mf.child_tab[i];
    else if (i < 131072) value = mf.hash4_tab[i - 65536];
    else value = mf.hash3_tab[(i - 131072) / 2][(i - 131072) % 2];
    return (unsigned)((int)value + 32768);
}
static void snapshot(void) {
    unsigned comma = 0;
    putchar('[');
    for (unsigned i = 0; i < 262144; i++) {
        unsigned value = biased(i);
        if (value) {
            printf("%s[%u,%u]", comma ? "," : "", i, value);
            comma = 1;
        }
    }
    putchar(']');
}
int main(void) {
    unsigned size, position, maximum, nice, depth, skip;
    if (scanf("%u%u%u%u%u%u", &size, &position, &maximum, &nice, &depth, &skip) != 6)
        return 2;
    if (size > sizeof(data) || position + maximum > size || maximum < 5 ||
        maximum > 258 || nice < 5 || nice > maximum || depth == 0 || skip > 1)
        return 3;
    for (unsigned i = 0, value; i < size; i++) {
        if (scanf("%u", &value) != 1 || value > 255) return 4;
        data[i] = value;
    }
    // Roc initializes all three tables to biased zero, including child slots.
    matchfinder_init((mf_pos_t *)&mf, sizeof(mf));
    u32 hashes[2] = {lz_hash(get_unaligned_le32(data) & 0xFFFFFF, 16),
                     lz_hash(get_unaligned_le32(data), 16)};
    unsigned base = 0;
    for (unsigned at = 0; at < position; at++) {
        if (at - base == 32768) {
            bt_matchfinder_slide_window(&mf);
            base += 32768;
        }
        bt_matchfinder_skip_byte(&mf, data + base, at - base, 32, 64, hashes);
    }
    if (position - base == 32768) {
        bt_matchfinder_slide_window(&mf);
        base += 32768;
    }
    printf("{\"base\":%u,\"position\":%u,\"hashes\":[%u,%u],\"before\":",
           base, position - base, hashes[0], hashes[1]);
    snapshot();
    unsigned count = 0;
    if (skip)
        bt_matchfinder_skip_byte(&mf, data + base, position - base, nice, depth, hashes);
    else
        count = bt_matchfinder_get_matches(&mf, data + base, position - base,
                                          maximum, nice, depth, hashes, matches) - matches;
    printf(",\"next_hashes\":[%u,%u],\"matches\":[", hashes[0], hashes[1]);
    for (unsigned i = 0; i < count; i++)
        printf("%s%u", i ? "," : "", (unsigned)matches[i].length |
               ((unsigned)matches[i].offset << 16));
    printf("],\"after\":");
    snapshot();
    puts("}");
    return 0;
}
'''


def cases():
    rng = random.Random(0xB17DEF1A7E)
    random_bytes = [rng.randrange(256) for _ in range(800)]
    branching = []
    for at in range(50):
        # Many equal 4-byte prefixes, distinct later bytes, and shared suffixes.
        branching += list(b"abcd") + [rng.randrange(8)] + list(b"abcdefghijk")
    short_fallback = random_bytes.copy()
    prefix = int.from_bytes(b"abc", "little")
    wanted = ((prefix * 506832829) & 0xFFFFFFFF) >> 16
    collision = next(n for n in range(1 << 24) if n != prefix
                     and ((n * 506832829) & 0xFFFFFFFF) >> 16 == wanted)
    short_fallback[84:88] = list(b"abcX")
    short_fallback[100:104] = list(collision.to_bytes(3, "little")) + [253]
    short_fallback[200:204] = list(b"abcY")
    patterns = [
        ("fresh", random_bytes, 0, 258, 32, 64),
        ("overlap", [97] * 800, 1, 258, 32, 64),
        ("periodic", list(b"abc") * 300, 33, 258, 32, 64),
        ("branch-depth-1", branching, 320, 258, 128, 1),
        ("branch-depth-2", branching, 320, 258, 128, 2),
        ("branch-depth-64", branching, 320, 258, 128, 64),
        ("unaligned", branching, 323, 258, 16, 64),
        ("maximum-five", branching, 320, 5, 5, 64),
        ("short-second-way", short_fallback, 200, 258, 32, 64),
        ("slid-window", list(b"abcdxabcdyabcdzab") * 2100, 32771, 258, 32, 64),
    ]
    for name, data, position, maximum, nice, depth in patterns:
        for skip in (False, True):
            yield dict(name=f"{name}-{'skip' if skip else 'search'}", input=data,
                       absolute_position=position, maximum=maximum, nice=nice,
                       depth=depth, skip=skip)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_root", type=Path)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parents[3]
                        / "hosts/gpu/tests/fixtures/native/deflate-binary-tree-reference.json")
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
    results = list(cases())
    with tempfile.TemporaryDirectory(prefix="mech-bt-oracle-") as temporary:
        driver, binary = Path(temporary) / "oracle.c", Path(temporary) / "oracle"
        driver.write_text(DRIVER.replace("SOURCE_FILE", json.dumps(str(source))))
        subprocess.run([args.cc, *flags, str(driver), "-o", str(binary)], check=True)
        for case in results:
            fields = [len(case["input"]), case["absolute_position"], case["maximum"],
                      case["nice"], case["depth"], int(case["skip"])]
            result = json.loads(subprocess.check_output([str(binary)], text=True,
                input=" ".join(map(str, fields + case["input"]))))
            lengths = [word & 65535 for word in result["matches"]]
            assert lengths == sorted(set(lengths))
            assert all(3 <= n <= case["maximum"] for n in lengths)
            for word in result["matches"]:
                length, offset = word & 65535, word >> 16
                at = case["absolute_position"]
                assert 1 <= offset <= 32767
                assert case["input"][at:at + length] == case["input"][at - offset:at - offset + length]
            if case["skip"]:
                assert result["matches"] == []
            elif case["name"] == "short-second-way-search":
                assert result["matches"][0] == 3 | (116 << 16)
            case.update(result)
        binary_hash = digest(binary)
    output = dict(reference=dict(project="libdeflate", commit=PIN,
        functions=["bt_matchfinder_get_matches", "bt_matchfinder_skip_byte",
                   "bt_matchfinder_slide_window", "matchfinder_init"],
        layout="Roc: child[65536], hash4[65536], hash3[131072], U16 bias 32768",
        **manifest, compiler=subprocess.check_output([args.cc, "--version"], text=True).splitlines()[0],
        flags=flags, binary_sha256=binary_hash), cases=results)
    args.out.write_text(json.dumps(output, separators=(",", ":")).replace(
        '},{"name":', '},\n{"name":') + "\n")
    print(f"Wrote {len(results)} full binary-tree transition cases to {args.out}")


if __name__ == "__main__":
    main()

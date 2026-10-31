#!/usr/bin/env python3
"""Produce exact Huffman fixtures by executing the pinned libdeflate C function."""
import argparse
import hashlib
import json
from pathlib import Path
import random
import subprocess
import sys
import tempfile

PIN = "b122c8be1d78b19f6d0a6efc5bb79bfcbb30dd51"
PINNED_SOURCE_HEADERS_SHA256 = "4fabfa52e359977973c0402b1ff12e22c4d2396dfa89375056c766be471b742b"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cases():
    yield "empty", 19, 7, [0] * 19
    for symbol in [0, 1, 18]:
        freq = [0] * 19
        freq[symbol] = 17
        yield f"single-{symbol}", 19, 7, freq
    yield "two-equal", 2, 15, [1, 1]
    yield "two-unequal", 2, 15, [1, 900]
    yield "precode-equal", 19, 7, [1] * 19
    yield "offset-ties", 30, 15, [0, 1, 2, 3, 30, 31] * 5
    yield "offset-reserved", 32, 15, [0, 1, 2, 3, 30, 31] * 5 + [0, 0]
    yield "literal-equal", 288, 15, [1] * 288
    yield "saturated-counters", 288, 15, [288 + (i * 37) % 701 for i in range(288)]
    fib = [1, 1]
    for _ in range(17):
        fib.append(fib[-1] + fib[-2])
    yield "precode-clamped", 19, 7, fib
    yield "literal-clamped", 288, 15, fib + [0] * (288 - len(fib))
    rng = random.Random(0xDEF1A7E)
    for index, (n, limit) in enumerate([(19, 7), (30, 15), (288, 15)] * 2):
        yield f"random-{index}", n, limit, [rng.choice([0, 0, 1, 2, rng.randrange(5000)]) for _ in range(n)]


def pretty(value, level=0):
    indent = "  " * level
    child_indent = indent + "  "
    if isinstance(value, dict):
        return "{\n" + ",\n".join(child_indent + json.dumps(key) + ": " + pretty(item, level + 1)
            for key, item in value.items()) + "\n" + indent + "}"
    if isinstance(value, list) and value and isinstance(value[0], dict):
        return "[\n" + ",\n".join(child_indent + pretty(item, level + 1) for item in value) + "\n" + indent + "]"
    return json.dumps(value)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reference_root", type=Path)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--out", type=Path, default=Path(__file__).resolve().parents[3] / "hosts/gpu/tests/fixtures/native/deflate-huffman-reference.json")
    args = parser.parse_args()
    root = args.reference_root.resolve()
    source = root / "lib/deflate_compress.c"
    if not source.is_file():
        parser.error("reference_root must contain lib/deflate_compress.c")
    headers = {str(path.relative_to(root)): sha256(path)
        for path in sorted(root.rglob("*.h"))}
    manifest = dict(source_sha256=sha256(source), header_sha256=headers)
    digest = hashlib.sha256(json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    if digest != PINNED_SOURCE_HEADERS_SHA256:
        parser.error(f"reference source/headers do not match pinned libdeflate {PIN}")
    wrapper = r'''
#include <stdio.h>
#include <stdlib.h>
#include SOURCE_FILE
int main(int argc, char **argv) {
    if (argc != 3) return 2;
    unsigned n = (unsigned)strtoul(argv[1], NULL, 10);
    unsigned max_len = (unsigned)strtoul(argv[2], NULL, 10);
    if (n < 2 || n > 288 || max_len < 1 || max_len > 15) return 3;
    u32 freqs[288] = {0}, words[288] = {0};
    u8 lens[288] = {0};
    for (unsigned i = 0; i < n; i++) if (scanf("%u", &freqs[i]) != 1) return 4;
    deflate_make_huffman_code(n, max_len, freqs, lens, words);
    for (unsigned i = 0; i < 288; i++) printf("%u %u\n", lens[i], words[i]);
    return 0;
}
'''.replace("SOURCE_FILE", json.dumps(str(source)))
    compiler = subprocess.check_output([args.cc, "--version"], text=True).splitlines()[0]
    flags = ["-O3", "-DNDEBUG", "-ffunction-sections", "-fdata-sections", f"-I{root}"]
    flags += ["-Wl,-dead_strip"] if sys.platform == "darwin" else ["-Wl,--gc-sections"]
    fixtures = []
    with tempfile.TemporaryDirectory(prefix="mech-huffman-oracle-") as temporary:
        directory = Path(temporary)
        driver = directory / "oracle.c"
        binary = directory / "oracle"
        driver.write_text(wrapper)
        subprocess.run([args.cc, *flags, str(driver), "-o", str(binary)], check=True)
        for name, n, maximum, frequencies in cases():
            assert len(frequencies) == n and sum(frequencies) < 1 << 22
            assert sum(value != 0 for value in frequencies) <= 1 << maximum
            result = subprocess.check_output([str(binary), str(n), str(maximum)],
                input=" ".join(map(str, frequencies)), text=True)
            rows = [tuple(map(int, row.split())) for row in result.splitlines()]
            assert len(rows) == 288
            fixtures.append(dict(name=name, num_symbols=n, max_len=maximum,
                frequencies=frequencies, lengths=[row[0] for row in rows],
                codewords=[row[1] for row in rows]))
        binary_digest = sha256(binary)
    output = dict(reference=dict(project="libdeflate", commit=PIN,
        function="deflate_make_huffman_code", source_sha256=sha256(source),
        header_sha256=headers, compiler=compiler, flags=flags,
        oracle_binary_sha256=binary_digest,
        comparison_contract="All lengths and codewords with positive lengths; zero-length scratch words differ between C bit-reversal implementations."), cases=fixtures)
    args.out.write_text(pretty(output) + "\n")
    print(f"wrote {len(fixtures)} exact reference cases to {args.out}")


if __name__ == "__main__":
    main()

"""Untimed semantic counters and real Huffman inputs from pinned libdeflate.

This builds a separate diagnostic library. It never replaces a latency worker.
The original source tree is read only; every edit is an asserted local copy.
"""

import argparse
import ctypes
from hashlib import sha256
import json
from pathlib import Path
import subprocess
import zlib


PIN = "92e6a0db9fa848d742f9eb286c92afc60f2c3dda"
SOURCE_PINS = {
    "lib/deflate_compress.c": "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541",
    "lib/bt_matchfinder.h": "6e3834eb2de5c759479bbdab9471f3569f8aa2bcbba81f4f917c84ebff0050ad",
    "lib/utils.c": "3d4c9f6d488610d4d9ff314b2dc783e24079325b78af13d3ee37b18fcc0b0d8a",
    "lib/arm/cpu_features.c": "ab7fb53d8b5b991200e986f20b66f71549a28bbe54b904f37ef81a4696780048",
}
CASES = (
    "prefix-natural-1024", "prefix-natural-4096", "prefix-natural-8192",
    "prefix-random-4096", "prefix-random-16384", "prefix-random-65536",
    "prefix-random-1048576", "16384-distribution-change", "320-overlap",
)
COUNTERS = (
    "search_positions", "skip_positions", "search_tree_visits", "skip_tree_visits",
    "extend_calls", "extend_matched_bytes", "cache_match_records", "cache_headers",
    "blocks", "dp_positions", "dp_literal_only_positions", "dp_match_positions",
    "dp_match_records", "candidate_lengths", "chosen_literals", "chosen_matches",
    "chosen_match_bytes", "literal_histogram_bytes", "huffman_builds",
    "sort_input_symbols", "sort_nonzero_symbols", "overflow_heap_symbols",
)


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def replace_once(text, old, new):
    assert text.count(old) == 1, (old, text.count(old))
    return text.replace(old, new)


def function(text, name, transform):
    """Find a definition, ignoring braces in C comments and strings."""
    start = text.index("\n" + name + "(")
    brace = text.index("{", start)
    at, depth, state = brace, 0, "code"
    while at < len(text):
        ch, pair = text[at], text[at:at + 2]
        if state == "code":
            if pair in ("/*", "//"):
                state = pair
                at += 2
                continue
            if ch in ('"', "'"):
                state = ch
            elif ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    return text[:brace] + transform(text[brace:at + 1]) + text[at + 1:]
        elif state == "/*" and pair == "*/":
            state = "code"
            at += 2
            continue
        elif state == "//" and ch == "\n":
            state = "code"
        elif state in ('"', "'"):
            if ch == "\\":
                at += 2
                continue
            if ch == state:
                state = "code"
        at += 1
    raise AssertionError(name)


HEADER = r'''
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
enum probe_counter { @ENUM@ PROBE_COUNT };
static uint64_t probe_counts[PROBE_COUNT];
struct probe_pass {
    unsigned purpose, block, length;
    uint64_t positions, literal_only, match_positions, records, candidates;
    uint64_t literals, matches, match_bytes;
};
struct probe_vector {
    unsigned context, purpose, block, count;
    uint32_t freqs[288];
};
static struct probe_pass probe_passes[256];
static struct probe_vector probe_vectors[4096];
static unsigned probe_num_passes, probe_num_vectors;
static unsigned probe_purpose, probe_block, probe_stage;
static struct probe_pass *probe_active;
#define PCOUNT(name) (probe_counts[PC_##name]++)
#define PADD(name, value) (probe_counts[PC_##name] += (value))
static void probe_frequency(unsigned count, const uint32_t *freqs) {
    assert(count <= 288 && probe_num_vectors < 4096);
    struct probe_vector *v = &probe_vectors[probe_num_vectors++];
    v->context = probe_stage; v->purpose = probe_active ? probe_purpose : 99;
    v->block = probe_block; v->count = count;
    memcpy(v->freqs, freqs, count * sizeof(*freqs));
}
void probe_reset(void) {
    memset(probe_counts, 0, sizeof(probe_counts));
    memset(probe_passes, 0, sizeof(probe_passes));
    probe_num_passes = probe_num_vectors = probe_block = 0;
    probe_purpose = 99; probe_stage = 1; probe_active = NULL;
}
void probe_set_stage(unsigned stage) { probe_stage = stage; }
void probe_dump(const char *path) {
    static const char *names[] = { @NAMES@ };
    FILE *f = fopen(path, "w"); assert(f);
    fprintf(f, "{\"counters\":{");
    for (unsigned i=0; i<PROBE_COUNT; ++i)
        fprintf(f, "%s\"%s\":%llu", i ? "," : "", names[i],
                (unsigned long long)probe_counts[i]);
    fprintf(f, "},\"passes\":[");
    for (unsigned i=0; i<probe_num_passes; ++i) {
        const struct probe_pass *p = &probe_passes[i];
        fprintf(f, "%s{\"purpose\":%u,\"block\":%u,\"length\":%u,"
                   "\"positions\":%llu,\"literal_only\":%llu,"
                   "\"match_positions\":%llu,\"records\":%llu,"
                   "\"candidate_lengths\":%llu,\"chosen_literals\":%llu,"
                   "\"chosen_matches\":%llu,\"chosen_match_bytes\":%llu}",
                i ? "," : "", p->purpose, p->block, p->length,
                (unsigned long long)p->positions,
                (unsigned long long)p->literal_only,
                (unsigned long long)p->match_positions,
                (unsigned long long)p->records,
                (unsigned long long)p->candidates,
                (unsigned long long)p->literals,
                (unsigned long long)p->matches,
                (unsigned long long)p->match_bytes);
    }
    fprintf(f, "],\"frequency_vectors\":[");
    for (unsigned i=0; i<probe_num_vectors; ++i) {
        const struct probe_vector *v = &probe_vectors[i];
        fprintf(f, "%s{\"stage\":%u,\"purpose\":%u,\"block\":%u,\"freqs\":[",
                i ? "," : "", v->context, v->purpose, v->block);
        for (unsigned j=0; j<v->count; ++j)
            fprintf(f, "%s%u", j ? "," : "", v->freqs[j]);
        fprintf(f, "]}");
    }
    fprintf(f, "]}\n"); assert(fclose(f) == 0);
}
'''


def instrument(source, matcher):
    header = HEADER.replace("@ENUM@", ",".join("PC_" + c for c in COUNTERS) + ",")
    header = header.replace("@NAMES@", ",".join(json.dumps(c) for c in COUNTERS))
    source = header + source
    source = function(source, "sort_symbols", lambda b: b.replace(
        "{", "{\n\tPADD(sort_input_symbols, num_syms);", 1))
    source = function(source, "sort_symbols", lambda b: replace_once(
        b, "\t/* Sort the symbols counted in the last counter. */",
        "\tPADD(sort_nonzero_symbols, num_used_syms);\n"
        "\tPADD(overflow_heap_symbols, counters[num_counters - 1] - counters[num_counters - 2]);\n"
        "\t/* Sort the symbols counted in the last counter. */"))
    source = function(source, "deflate_make_huffman_code", lambda b: b.replace(
        "{", "{\n\tPCOUNT(huffman_builds); probe_frequency(num_syms, freqs);", 1))
    source = function(source, "deflate_choose_all_literals", lambda b: b.replace(
        "{", "{\n\tPADD(literal_histogram_bytes, block_length);", 1))

    def dp(b):
        b = b.replace("{", "{\n\tassert(probe_num_passes < 256);\n"
                         "\tstruct probe_pass *p = &probe_passes[probe_num_passes++];\n"
                         "\tp->purpose=probe_purpose; p->block=probe_block; p->length=block_length;\n"
                         "\tprobe_active=p;\n", 1)
        b = replace_once(b, "\t\tliteral = cache_ptr->offset;", "\t\tliteral = cache_ptr->offset;\n"
                         "\t\tPCOUNT(dp_positions); p->positions++;\n"
                         "\t\tif (num_matches) { PCOUNT(dp_match_positions); p->match_positions++; }\n"
                         "\t\telse { PCOUNT(dp_literal_only_positions); p->literal_only++; }\n"
                         "\t\tPADD(dp_match_records, num_matches); p->records += num_matches;")
        b = replace_once(b, "\t\t\t\t\tcost_to_end = offset_cost +", "\t\t\t\t\tPCOUNT(candidate_lengths); p->candidates++;\n\t\t\t\t\tcost_to_end = offset_cost +")
        b = replace_once(b, "\tdeflate_make_huffman_codes(&c->freqs, &c->codes);", "\tdeflate_make_huffman_codes(&c->freqs, &c->codes);\n\tprobe_active=NULL;")
        return b
    source = function(source, "deflate_find_min_cost_path", dp)

    def tally(b):
        return replace_once(b, "\t\tcur_node += length;", "\t\tif (length == 1) {\n"
                            "\t\t\tPCOUNT(chosen_literals); if (probe_active) probe_active->literals++;\n"
                            "\t\t} else {\n"
                            "\t\t\tPCOUNT(chosen_matches); PADD(chosen_match_bytes, length);\n"
                            "\t\t\tif (probe_active) { probe_active->matches++; probe_active->match_bytes += length; }\n"
                            "\t\t}\n\t\tcur_node += length;")
    source = function(source, "deflate_tally_item_list", tally)

    def optimize(b):
        b = b.replace("{", "{\n\tprobe_block++; PCOUNT(blocks); probe_purpose=99;", 1)
        call = "deflate_find_min_cost_path(c, block_length, cache_ptr);"
        parts = b.split(call)
        assert len(parts) == 5
        result = parts[0]
        for purpose, tail in enumerate(parts[1:]):
            result += f"probe_purpose={purpose}; {call}" + tail
        return result
    source = function(source, "deflate_optimize_and_flush_block", optimize)

    def collect(b):
        return replace_once(b, "\t\t\tcache_ptr->length = cache_ptr - matches;",
                            "\t\t\tPADD(cache_match_records, cache_ptr - matches);\n"
                            "\t\t\tPCOUNT(cache_headers);\n"
                            "\t\t\tcache_ptr->length = cache_ptr - matches;").replace(
            "\t\t\t\t\tcache_ptr->length = 0;",
            "\t\t\t\t\tPCOUNT(cache_headers);\n\t\t\t\t\tcache_ptr->length = 0;")
    source = function(source, "deflate_compress_near_optimal", collect)

    def advance(b):
        b = b.replace("{", "{\n\tif (record_matches) PCOUNT(search_positions); else PCOUNT(skip_positions);", 1)
        b = replace_once(b, "\t\tmatchptr = &in_base[cur_node];", "\t\tif (record_matches) PCOUNT(search_tree_visits); else PCOUNT(skip_tree_visits);\n\t\tmatchptr = &in_base[cur_node];")
        b = replace_once(b, "\t\t\tlen = lz_extend(in_next, matchptr, len + 1, max_len);", "\t\t\tuint32_t probe_start_len = len;\n\t\t\tPCOUNT(extend_calls);\n\t\t\tlen = lz_extend(in_next, matchptr, len + 1, max_len);\n\t\t\tPADD(extend_matched_bytes, len - probe_start_len);")
        return b
    matcher = function(matcher, "bt_matchfinder_advance_one_byte", advance)
    return source, matcher


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("--libdeflate-root", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--current-profiles", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, out = args.libdeflate_root.resolve(), args.output.resolve()
    assert {name: digest(root / name) for name in SOURCE_PINS} == SOURCE_PINS
    out.mkdir(parents=True, exist_ok=False)
    original = root / "lib/deflate_compress.c"
    matcher = root / "lib/bt_matchfinder.h"
    source, bt = instrument(original.read_text(), matcher.read_text())
    (out / "deflate_compress.c").write_text(source)
    (out / "bt_matchfinder.h").write_text(bt)
    # Quoted headers are found beside the local copied source first.
    command = ["/usr/bin/clang", "-arch", "arm64", "-std=gnu11", "-O2", "-DNDEBUG",
               "-Wall", "-Werror", "-UNDEBUG", "-dynamiclib", "-I", str(out),
               "-I", str(root / "lib"), "-I", str(root), str(out / "deflate_compress.c"),
               str(root / "lib/utils.c"), str(root / "lib/arm/cpu_features.c"),
               "-o", str(out / "diagnostic.dylib")]
    with (out / "build.log").open("w") as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    library = ctypes.CDLL(str(out / "diagnostic.dylib"))
    library.libdeflate_alloc_compressor.argtypes = [ctypes.c_int]
    library.libdeflate_alloc_compressor.restype = ctypes.c_void_p
    library.libdeflate_free_compressor.argtypes = [ctypes.c_void_p]
    library.libdeflate_deflate_compress.argtypes = [ctypes.c_void_p, ctypes.c_void_p,
        ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
    library.libdeflate_deflate_compress.restype = ctypes.c_size_t
    library.probe_dump.argtypes = [ctypes.c_char_p]
    library.probe_set_stage.argtypes = [ctypes.c_uint]
    fixtures = json.loads(args.fixtures.read_text())
    results = {}
    for name in CASES:
        case, = [c for c in fixtures if c["name"] == name]
        payload = Path(case["input"]).read_bytes()
        prefix = Path(case["prefix"]).read_bytes()
        before = ctypes.create_string_buffer(payload)
        target = ctypes.create_string_buffer(max(case["capacity"], len(payload) + 5000))
        library.probe_reset()
        compressor = library.libdeflate_alloc_compressor(12)
        assert compressor
        library.probe_set_stage(2)
        size = library.libdeflate_deflate_compress(compressor, before, len(payload), target, len(target))
        assert size > 0
        library.libdeflate_free_compressor(compressor)
        raw = target.raw[:size]
        encoded = prefix + raw
        assert before.raw[:-1] == payload
        assert zlib.decompress(raw, -15) == payload
        peer = json.loads((args.current_profiles / name / "libdeflate/result.json").read_text())
        expected_sha = peer["verified_calls"][0]["output_sha256"]
        assert sha256(encoded).hexdigest() == expected_sha, name
        trace_path = out / (name + ".json")
        library.probe_dump(str(trace_path).encode())
        trace = json.loads(trace_path.read_text())
        counts = trace["counters"]
        assert counts["dp_positions"] == sum(p["length"] for p in trace["passes"])
        assert all(p["positions"] == p["length"] == p["chosen_literals"] + p["chosen_match_bytes"]
                   for p in trace["passes"])
        assert counts["huffman_builds"] == len(trace["frequency_vectors"])
        results[name] = {"input_sha256": sha256(payload).hexdigest(),
            "input_bytes": len(payload), "output_sha256": expected_sha,
            "raw_bytes": size, "independent_inflation": True,
            "matches_pinned_worker": True, "trace_sha256": digest(trace_path),
            "counters": counts, "passes": trace["passes"]}
        print(name, json.dumps(counts), flush=True)
    inputs = {str(p.relative_to(root)): digest(p) for p in root.rglob("*")
              if p.is_file() and p.suffix in (".c", ".h")}
    result = {"pinned_libdeflate_commit": PIN, "original_source_sha256": digest(original),
        "source_inputs": inputs, "command": command,
        "diagnostic_library_sha256": digest(out / "diagnostic.dylib"),
        "generated_sources": {p: digest(out / p) for p in ("deflate_compress.c", "bt_matchfinder.h")},
        "script_sha256": digest(__file__), "cases": results, "complete": True,
        "purpose_codes": {"0": "static trial", "1": "iterative dynamic trial",
                          "2": "recover static winner", "3": "recover previous dynamic winner",
                          "99": "outside a minimum-cost pass"},
        "stage_codes": {"1": "fresh compressor construction", "2": "compression"},
        "scope": "Untimed diagnostic source instrumentation only. Per-pass and per-node counters perturb execution; never compare these calls as latency. extend_matched_bytes counts newly established equal-prefix bytes, not memory traffic or all comparison attempts. Setup allocation/mapping/first-touch and output bit work remain unmeasured."}
    (out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()

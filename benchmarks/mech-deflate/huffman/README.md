# DEFLATE Huffman kernel

The [Mech program](../../../hosts/gpu/tests/fixtures/native/deflate-huffman.mec)
implements the complete length-limited canonical Huffman builder used by
libdeflate's level-12 compressor. It retains the counting sort with a saturated
bucket and heapsort, packed stripped tree, length limiting, canonical assignment,
and reversed codewords. It executes Mech code through the canonical native
compiler. The C function is used only to generate reference values.

This is a compressor component. The full matchfinder, near-optimal parser,
block encoder, bit writer, and the `reymont` compression timing remain outstanding.

## Inputs and scratch

`#MakeHuffman` accepts an alphabet size from 2 through 288, maximum length from
1 through 15, and 288 `u32` frequencies. Only the first `n` entries participate.
As in libdeflate, the frequency sum must fit 22 bits and the used alphabet must
fit the selected maximum code length. It returns 288 code lengths (`u8`) and
288 codewords (`u32`). The fixed capacity supports the literal/length, offset,
and precode alphabets; smaller alphabets occupy a prefix.

Caller-provided scratch inputs start at zero: 288 `u64` counters, 288 `u8`
lengths, 288 `u32` words, and 17 `u64` length counters. Input values remain
immutable. The current value-copy implementation and native call metadata are
expensive; the qualification uses an explicit 2 GiB memory account and larger
finite work allowances. Host defaults are unchanged.

Native work policy 68 reads U8/U16/U32/U64 elements directly into native scalar storage
through the maintained indexed-access operation. It keeps the original visits,
checked selectors, source admission, and logical output limits, while avoiding
transient selector/result snapshots. The twelve checked conversions between
these unsigned widths also execute as native scalar instructions. Buffer writes
and structural buffer carriers still use the existing canonical value path.

The ordinary resident route retains its whole-control comparison-work cap. The
full builder exceeds that cap even on a two-symbol input; a separate regression
checks that this is reported as `ResourceLimit`, preserves the last published
state, and remains safe to retry. Indexed replacement previously mislabeled
this denial as `InvalidOutput`. Smaller nested-buffer programs are also checked
against independent expected values through the resident route.

## Qualification

The fixtures live with the GPU test crate so packaged tests can include them.
The generator checks the pinned C source/header hashes, compiles the original
`deflate_make_huffman_code`, and executes it for deterministic reference cases.
They include empty and single-symbol codes, ties, saturated counters, dense
alphabets, seeded random frequencies, and length limits that actually bind.

Every code length and every positive-length codeword must match C. For a
zero-length symbol, ARM's `rbit32` path can leave high scratch bits, while C's
portable table and Roc return zero. Those symbols emit no bits. The Mech port
retains Roc's deterministic zero value and the raw C values remain in the
reference fixture for inspection.

```sh
python3 benchmarks/mech-deflate/huffman/generate_reference.py /path/to/pinned/libdeflate
cargo test --locked --offline -p mech-gpu --no-default-features \
  --features canonical-native-program --test canonical_artifact_program \
  huffman_qualification -- --test-threads=1 --nocapture
```

The test reuses one compiled program across changing inputs and checks original
and decoded artifacts. Printed elapsed times are single-call diagnostics from
the test harness, including native invocation admission and publication. They
are not a controlled compression benchmark or a C/Roc timing comparison.

Source licenses are retained in
[the fixture's notices](../../../hosts/gpu/tests/fixtures/native/deflate-huffman-LICENSES.md).

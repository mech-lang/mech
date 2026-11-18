# Where the encoder size gap comes from

The current LLVM-free Mech encoder occupies **71,192 B** of ARM64 object
text. The matched libdeflate object occupies **37,080 B**. Mech is
**1.920× libdeflate** and needs **34,112 B** less emitted text for parity.
The two objects are the fixed inputs from the M1 source-through-link
comparison. A fresh deployment link with dead stripping retains 71,192 B
of Mech encoder text and 37,072 B of libdeflate encoder text. The gap
survives linking. [Matched build](../inline-syntax-child-traversal/catalog/README.md) ·
[Object census](size-comparison.json) · [Link receipts](linked-size.json)

| Mech emission | ARM64 text |
| --- | ---: |
| Checked entry extent, including 56 B of tail and alignment | 50,688 B |
| Match collection kernel | 6,768 B |
| Composed, no-growth and general scorer entries | 4,112 B |
| Other helpers and alignment | 9,624 B |
| **Total** | **71,192 B** |

The checked entry contains **50,632 B** of compiler-reported body text.
Its disassembly contains **14,088 B** of stack-relative load and store
instructions, including spills, frame saves and call packet traffic. It
contains another **12,324 B** of calls and branches. The whole libdeflate
object contains **2,400 B** of stack-relative memory instructions and
**4,804 B** of calls and branches by the same count. These counts classify
instruction encodings. An instruction in either class may do required work.
[Instruction classes and symbol groups](size-summary.json)

## What libdeflate shares

Libdeflate allocates a compressor context with typed scratch buffers,
frequency tables, costs and match nodes. Its internal functions operate on
that context and use preconditions established by the parser and writer.
For example, it checks the space needed to write a block, then its bit
writer can fill the output through the current pointer. The common Huffman
builder takes alphabet size, width limit and caller-provided buffers, and
has one **1,272 B** normal body in the C object. The 37,080 B also includes
**13,556 B** for the fastest, greedy, lazy, lazy2 and stored-only parsers.
The benchmark invokes level 12 through the normal public API.
[Pinned libdeflate source](https://github.com/ebiggers/libdeflate/blob/92e6a0db9fa848d742f9eb286c92afc60f2c3dda/lib/deflate_compress.c)

Mech source also defines reusable functions. The current source lowering
expands local calls into the caller's canonical graph. The checked emitter
then carries typed buffer views, ownership and failure state through that
graph, with separately emitted kernels for admitted cases. The match
collector's phase boundary has as many as 55 input and 77 output words in
its declared packets. Maintaining smaller live interfaces at repeated
checked boundaries can reduce call traffic and the register pressure in the
checked entry. The most frequently repeated small shape in the exact
control's cleaned IR is a defaulted word read, with 51 matching shapes
before instruction selection. This count is a lead for checked region
analysis. Its native byte effect needs a linked comparison.
[Mech source lowering](https://github.com/mech-lang/mech/blob/c2e1b803d82401d28c2aa15dad875bfbfd3e492b/src/engine/src/source_semantics/document_functions.rs#L689-L727) ·
[Compressor source](../../program.mec#L1137-L1167) · [Exact control structure receipt](control-structure.json)

Apple Clang emits 23 machine outlines in the C build. Disabling only
outlining increases that object to **38,536 B**. The **1,456 B** difference
accounts for **4.3%** of the current Mech gap on this pair.
A scan of exact matching Mech root tails found **424 B** of gross sharing
opportunity in the current machine code. The scan covers repeated
terminal suffixes that end in the same jump or return, within a single block. [Root tail census](root-tails.json)

## Where to apply the compiler work

The earlier LLVM-assisted 62-win encoder used the same canonical bytecode
and occupied **55,700 B**. Both builds have the same 8,223 source sites and
exact origin sequence. Of the **15,492 B** difference, **3,660 B** is in
the checked entry itself and **11,832 B** is elsewhere. The earlier pipeline
coalesced 16 phase frames into the checked entry. The artifacts also differ
in instruction selection and phase placement, so this comparison gives a
scale for the opportunity across the combined passes.
[Earlier build and full runtime record](../runtime-62-high-water/README.md)

Recover that scale with the LLVM-free compiler by giving repeated checked
regions a reusable typed boundary and emitting one body when their effects,
owners and continuations agree. Start with larger shared regions such as
Huffman preparation and block writing, and pass only the live views and
scalars. Reuse an existing checked continuation for a refused kernel when
it can resume at the original entry before any phase work. Measure the
whole linked encoder after each change and qualify the resulting worker in
both orders at **81/81 Roc** and **at least 62/81 libdeflate**.
[Runtime qualification policy](../../qualification-policy-v3.json)

## Replay

On the matched Mac, pass the output directory containing the recorded
`program.o` and `metadata.json`, the pinned libdeflate source directory
and a new output directory. The replay builds both C configurations from
the same source. The optional historical input accepts the historical
Mech artifact directory.

```sh
python3 -B census.py /tmp/mech-frame-copies-71192/native \
  /tmp/libdeflate-92e6a0d /tmp/encoder-size-census
```

The pinned Mech encoder SHA-256 is
`96696eeed083f0f11b51a8b26a01fe9077735821f65e7f08decea56881c6329b`.
The original libdeflate encoder SHA-256 is
`fd35728446cd309af78c9334d8af6539810f099b8e764bc769b852980441f726`.

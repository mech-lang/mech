# Reuse the successful Julia safe compile; seal options and ABI offline

The actual static-I/O compiler command PID 53013 exited 0 with empty stdout
and stderr. It produced object/bitcode archives and the ABI descriptor. The
Python controller then failed parsing its options text as TOML. The failed
controller remains a failure, and its successful native command remains the
original successful command. This additive two-file packet repeats neither.

Root closed 10,072 pins in
`continuation/jobs/julia-static-io-safe-compile-closed-review-v1/receipt.json`,
SHA `1be37bffac06a836065ce98fe5f65eab43458c5db2e6a0f3d7a7d18942ef84fa`.
The original controller outcome is c8df203a, durable terminal a91abacb,
request a6430bf6 and native compile receipt af1f0626. Full hashes are literal
in `seal.py`. Exact worker 52972/target 52976, saved source argument digests,
request/start/worker/terminal identity, normal controller exit 1, actual native
exit 0 and native PID/birth/argv are bound. All original prerequisite and
available files are rebound, then checked again before and after sealing.
The actual source adaptation receipt 4695acce and all original source spans,
static I/O edits, full positive output and unchanged shared JIT module remain
bound. No file from the failed job or compiled tree is rewritten.

The source cause is the metadata wrapper's `Base.TOML.print` call. In the
retained Julia 1.13.1 `base/toml/toml.jl`, serialization lives in nested Printer
to avoid colliding with ordinary Base.print. The documented TOML stdlib
exports its own print alias to that Printer. The wrapper used the internal
module name, so the actual file contains ordinary `Dict{String, Any}(...)`
display text. The official trimming patches contain no TOML serializer
replacement. This is a wrapper mistake after an otherwise successful safe
compile, not a compiler retry requirement. The raw file remains named
`actual-build-options.toml` for historical identity, with its actual observed
format disclosed separately.

`parse_options` accepts only the saved bounded ASCII Dict syntax: the literal
header/closing parenthesis, quoted lowercase/underscore keys, unescaped quoted
strings, nonnegative integer literals and true/false, separated by exact
commas. Duplicate keys, empty maps, trailing separators, extra syntax,
escapes, floats and unsupported forms reject. It performs no evaluation,
Julia call or general expression parsing. The result must have exactly the
fifteen original options keys, with type-sensitive equality to the expected
safe 1/strip_ir 1/metadata 0/nonincremental 0/O3/g2/thread 8/root/state protocol and
every literal compiled source/sysimage/object/bitcode/ABI path. No field can
be inferred from the requested command alone. The raw SHA 1093912c file is
preserved; a new JSON receipt records its decoded values and provenance.

Each of the four actual artifacts is checked against root's exact path,
SHA, size and first 16 bytes: object 9,444,136 bytes e25e9ace; bitcode-named
archive 9,672,560 bytes fbf05e04; ABI 823 bytes 894267e7; options 1184 bytes 1093912c.
Both native outputs begin with the ar archive signature. The `.bc` suffix is
not treated as proof of a raw LLVM bitcode header or qualified archive members.
Member/native-owner inspection and actual IR removal remain pending.

The prior accepted worker is SHA-checked and cold-loaded at its real path
solely for pure SHA/bind/verify/save, exact span derivation and `entry_abi`.
Neither its main/build/owned service nor a Julia reader is called. The existing
ABI validator requires signed 32-bit Cint argument/result, Ptr{Ptr{Int8}} argv,
one clbg_entry export and the exact type relationships. No ABI repair,
object rewrite, linker, compiler, program execution, fixture or native
inspection occurs. The original builder, safe-trim flag/root protocol and
compiled source/image closure remain unchanged.

A closed shared original-job lease, a separate private owner lease and the
common inode 79859134 span all pin/metadata checks and journals. Fresh output
rejects an existing path; no old-output overwrite or automatic retry exists.
Errors retain new partial metadata and all original bytes. Terminal integrity
failure prevents a successful result. Root must accept/freeze both files and
capture their actual source digests before this metadata-only invocation:

```
bundled-python3.12 -I -S -B -u saved-safe-build-seal-v1/seal.py
  --revision official-static-io-safe-build-seal-v1
  --worker-sha256 <actual reviewed seal SHA>
  --review-sha256 <actual reviewed note SHA>
```

Successful output is `safe-build-result.json` plus the parsed options JSON
and a complete outcome. It means a successful safe build with exact actual
options and ABI, awaiting native-image/archive-member/IR/fallback review.
Runtime AOT, output equivalence, negative cases, native owner, actual IR
removal, fallback closure, required work, LOC and chart admission remain
unqualified. New native commands, compile repeats, workload/benchmark runs,
linker/LLDB attempts and fixture reruns are all zero. Source authoring did
not execute, import, hash, parse, test or freeze this packet.

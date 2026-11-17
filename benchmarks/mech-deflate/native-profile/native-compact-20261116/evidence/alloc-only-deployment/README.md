# Alloc-only Mech deployment on Apple arm64

The same 71,768-byte Cranelift encoder object, C file driver and mimalloc
object link with a private `core` / `alloc` runtime. The command still
reads one file and writes one raw level-12 DEFLATE stream.
[Common deployment](../minimal-deployment/README.md) · [Source patch](alloc-only.patch.gz) ·
[Build, inputs and binary hashes](measurements/build.json) · [Build plan](measurements/plan.json)

| Bytes | Mech std | Mech core+alloc | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Unstripped executable | 668,392 | 301,384 | 199,680 | 161,400 |
| Stripped executable | 575,632 | **292,576** | 191,920 | 155,760 |
| Linked machine code | 341,012 | **148,456** | 122,416 | 98,848 |

The stripped Mech executable is **49.2% smaller**, **1.524× Roc** and
**1.878× libdeflate**. The link map attributes 199,224 → 6,876 bytes
to Rust library and symbolization objects and 4,508 → 4,284 bytes to the
compact runtime. Linked encoder text is 71,768 bytes in both builds.
[Live symbol and section attribution](measurements/binary-audit.json)

All 81 raw streams were exact for both Mech core+alloc binaries and both
variants of each peer. Independent zlib checks consumed the complete streams
and reproduced the inputs. A separate probe against the same compact runtime
object member reproduced all six checked allocation failures, including
status, origin, allocation counters, unmodified buffers and zero output.
The CLI has no interface for supplying a prepared output or injecting an
allocation failure.
[81-case correctness](measurements/verify.json) · [Checked failures](measurements/checked-failures.json)

## Timings

Apple M1, macOS 15.6.1, Apple Clang 17.0.0 (clang-1700.3.19.1). Eight
serial blocks alternate the Mech link order. Each invocation links
prebuilt objects and writes the executable and linker map. The Mech
measurement filenames affect the Mach-O UUID and ad hoc signature.
All other bytes match the corresponding deployment binary. Every
repeated link produced the same actor binary.
[Raw final-link measurements and commands](measurements/link.json) ·
[Binary identity checks](measurements/binary-audit.json)

| Boundary | Mech std | Mech core+alloc | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Final link | 43.716 ms | **34.789 ms** | 34.258 ms | 33.832 ms |

Paired medians give **0.794× Mech std**, **1.012× Roc**, and
**1.026× libdeflate** for the core+alloc link. This boundary begins
at the final Clang call.

The same C timing driver preloads each input and batches the deployment
encode and output-release calls around `mach_absolute_time`. File I/O,
startup and output verification are outside the timing window. The timed
calls include each adapter's setup, including libdeflate's per-call
compressor allocation. Each input has 24 measured blocks covering
all actor orders, after two warm-up blocks. All timer executables
matched the exact raw oracles and independent inflation before timing.
[Driver](loop.c) · [Raw ticks, call counts and per-block orders](measurements/runtime.json)

| Frozen input | Mech std | Mech core+alloc | Roc | libdeflate | Paired core+alloc / std |
| --- | ---: | ---: | ---: | ---: | ---: |
| Empty | 35.378 ns | 35.142 ns | 20.034 ns | 4,088.483 ns | 0.996× |
| Natural 4 KiB | 378.852 µs | 382.175 µs | 595.558 µs | 317.046 µs | 1.008× |
| Random 4 KiB | 111.153 µs | 110.993 µs | 365.731 µs | 69.643 µs | 0.997× |

These three deployment inputs are a separate measurement from the
81-case maintained worker timings. The original compiler timings also
use their own source-through-artifact boundary.

## Reproduce

Use Apple arm64 with a macOS 15 SDK, Python 3.11 or later and
`nightly-2026-03-03` with the `rust-src` component installed. The stock
`alloc` archive imports `_rust_eh_personality`. The fallback builds
`core`, `alloc` and `compiler_builtins` offline with `panic=abort`.
This is a diagnostic host archive around the pinned encoder object.
[Stock link log](measurements/stock-link.log.gz) · [Offline build log](measurements/compile-bridge-abort-core.log.gz)

Run from this directory after restoring the repository's fixture data.
The parent deployment kit restores the fixed encoder inputs by SHA-256.
The preparation script checks the published frozen runtime sources,
applies the source changes and checks each source against the measured
hashes. The paths and toolchain revisions can affect the linked bytes.

~~~sh
work=$(mktemp -d /tmp/mech-alloc-deploy.XXXXXX)
deployment=../minimal-deployment
inventory=../../fixtures.json
python3 -B "$deployment/prepare-inputs.py" "$work/inputs"
python3 -B "$deployment/source/build.py" "$work/inputs/manifest.json" "$work/std"
python3 -B prepare.py "$work/alloc" --baseline "$work/std" --manifest "$work/inputs/manifest.json"
python3 -B build.py --trial "$work/alloc" --mode abort-core
python3 -B "$deployment/source/verify.py" "$work/verify.json" \
  --inventory "$inventory" --libdeflate-oracle "$deployment/libdeflate-output-oracle.json" \
  --mech-unstripped "$work/alloc/mech" --mech-stripped "$work/alloc/mech-stripped" \
  --roc-unstripped "$work/std/roc" --roc-stripped "$work/std/roc-stripped" \
  --libdeflate-unstripped "$work/std/libdeflate" --libdeflate-stripped "$work/std/libdeflate-stripped"
python3 -B measure.py "$work/alloc" "$work/timing" --baseline "$work/std" --runtime
python3 -B audit.py "$work/alloc" "$work/timing" --baseline "$work/std"
~~~

Use `faults.py` with the same manifest and inventory to replay the
separate native provider failure probe.

~~~sh
python3 -B faults.py "$work/alloc" --manifest "$work/inputs/manifest.json" \
  --inventory "$inventory" \
  --previous ../owner-exchange-71288/backend-work-hardened/linked-proofs/checked-failures.json
~~~

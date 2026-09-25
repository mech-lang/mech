# Checked dylib size diagnostic

Inspected on 2026-09-25. This diagnostic explains the on-disk size difference;
it does not replace the archived performance or memory measurements.

## Artifacts and provenance

The archived campaign in
`results/apple-m1-aot-vs-rust-dylib-2026-09-24.json` records:

| Artifact | Bytes | Recorded SHA-256 |
| --- | ---: | --- |
| Mech scalar AOT | 33,544 | `7e1edcc4769c6cdb27e6f53f913e887c1c9819ce4f1106018fd663d8a2cf22bc` |
| Rust scalar cdylib | 50,016 | `797cd587914ea925f1b9c7283c3721772948962ffa714e32f4e191be4fb83e82` |
| Mech SIMD-4 AOT | 33,864 | `fb14e7396252f934b5fbb86639bcb33f063193feae8fe86f83877a3c72437323` |

Those exact binaries were no longer available at the recorded paths or in the
searched worktrees and temporary artifact caches. The diagnostic therefore
uses a rebuilt Rust control and the retained checked Mech SIMD artifact from
the subsequent same-source backend campaign:

| Inspected artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| Rebuilt Rust scalar | 50,016 | `165672d40d46d551705528561de4a4500d17893ade5d80a5095f99fcbe9a7fcd` |
| Retained Mech SIMD-4, 2026-09-25 | 33,864 | `013c2140594d20ad327195e2f4894f91237d2ae3806cc02afaf62d10b842b80b` |

The rebuilt Rust path is
`/private/tmp/iros-dylib-size-audit-zK37H7/librust_ekf.dylib`.
The retained Mech path, relative to the workshop worktree, is
`target/aot-pairs-20260925/mech-simd-21c33ed119b346f43595cec1960d1a4905b14c536f827604df428cc9e092789a.dylib`.
Its hash, byte count and checked mode are also recorded in
`results/apple-m1-mech-backend-pairs-n10-2026-09-25.json`.

The Rust source is `rust-dylib/rust-ekf-dylib.rs`, SHA-256
`5e32fcc5e2b17bf923718860756b233d4da145b20cff48a20d0c18ec51ccc9a7`,
matching the archived campaign. The compiler also matches:

```text
rustc 1.96.0-nightly (ec818fda3 2026-03-02)
commit: ec818fda361ca216eb186f5cf45131bd9c776bb4
host: aarch64-apple-darwin
LLVM: 22.1.0
```

The exact recorded build options were used, with a separate diagnostic output:

```sh
rustc --edition=2024 --crate-type cdylib \
  -C opt-level=3 -C target-cpu=native -C codegen-units=1 \
  -o /private/tmp/iros-dylib-size-audit-zK37H7/librust_ekf.dylib \
  benchmarks/iros-2026/rust-dylib/rust-ekf-dylib.rs
```

No stripping, panic-strategy change or other size optimization was applied.
The rebuild reproduces the historical byte count, not its binary hash. Its
load-command install path differs, and its observed UUID is
`58224FA6-622B-3FE9-B8D6-E13693FCB6BB`.

The retained Mech artifact comes from `examples/embedded_ekf/ekf.mec`, SHA-256
`da531cddcb25d002d49f1a77800122e84b573e2685e191c1955908ef6fccd625`.
The AOT emitter selects Cranelift `opt_level=speed` and PIC, then invokes
`cc -dynamiclib OBJECTS -o OUTPUT -lm`. Its SIMD math helper is compiled with
`cc -O3 -fPIC -c`. These are the paths in
`hosts/gpu/src/batched/aot.rs` and `simd_aot.rs`.

## Mach-O observations

The following are file sizes from `otool -l`, not virtual-memory segment sizes:

| Segment | Rebuilt Rust scalar | Retained Mech SIMD-4 |
| --- | ---: | ---: |
| `__TEXT` | 16,384 B | 16,384 B |
| `__DATA_CONST` | 16,384 B | 16,384 B |
| `__DATA` | 16,384 B | absent |
| `__LINKEDIT` | 864 B | 1,096 B |
| Total file | 50,016 B | 33,864 B |

Rust's extra `__DATA` segment contains only 24 bytes of section payload:
16 bytes of lazy-symbol pointers (`__la_symbol_ptr`) and 8 bytes of
`__dyld_private` (`__data`). Its file extent is padded to 16 KiB.
Thus the inspected file-size difference is exactly
`16,384 + 864 - 1,096 = 16,152` bytes.

Rust uses `LC_DYLD_INFO_ONLY` with lazy binding and records a macOS 11.0
minimum deployment version. Mech uses `LC_DYLD_CHAINED_FIXUPS` and records
macOS 15.0. Both record SDK 26.0 and Apple linker 1221.4. These observed
packaging differences have not been normalized in this diagnostic.

Rust's actual `__text` section is 1,528 bytes. Mech SIMD's is 3,636 bytes,
including its turn function and four exported math helpers. Rust additionally
has 24 bytes of stubs, 48 bytes of stub helpers, 16 bytes of constants,
88 bytes of compact unwind information and 192 bytes of `__eh_frame`.
Mech has 48 bytes of stubs and 104 bytes of compact unwind information.
The scalar and four-lane implementations are different execution strategies;
these instruction counts are not a matched scalar compiler comparison.

Both libraries depend only on `/usr/lib/libSystem.B.dylib`. Rust exports the
turn function and imports `__sincosf_stret`, `atan2f` and `dyld_stub_binder`.
Mech exports its turn plus four math helpers, importing `sinf`, `cosf`,
`atan2f` and `__sincosf_stret`. No large linked Rust runtime appears in this
symbol and section inspection.

## Interpretation and limits

For these inspected artifacts, the smaller Mech file is primarily a
linker/segment-layout result, not evidence that Mech emits fewer numerical
instructions. It should not be attributed to Rust standard-library overhead,
nor generalized to other platforms or matched deployment/linker settings.
The old Mech scalar artifact was not inspected, and the newer SIMD artifact
is not a byte-identical substitute for its archived predecessor.

The retained memory measurement remains whole-loader-process peak RSS:
all three archived rows have median 2,818,048 bytes and MAD zero. An on-disk
size difference does not establish a corresponding resident-memory advantage.
No performance or memory measurements were rerun for this diagnostic.

The inspection commands, applied separately to each artifact, were:

```sh
shasum -a 256 ARTIFACT
stat -f '%z %N' ARTIFACT
xcrun size -m ARTIFACT
otool -l ARTIFACT
otool -L ARTIFACT
nm -m ARTIFACT
```

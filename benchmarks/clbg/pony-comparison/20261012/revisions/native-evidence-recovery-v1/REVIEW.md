# Pony missing scored allocation evidence

This two-file additive packet is source-only and unexecuted. It uses the
existing bounded owned-command service, named common mutex and completed
original-evidence capture. It does not invoke any application, full benchmark,
trace generator, Pi/K/tree observation, fixture, bootstrap or old work runner.
Every allocation/work conclusion stays null/false pending independent review.

The original capture is
`revisions/scored-allocation-recovery-v1/jobs/original-six-traces-v1/`.
Its ownership proposal SHA is
`e4fb58118389cc53c196a6c743a2a49e1a8e5d96e96be647364e94e75af2955d`.
Root must supply the actual captured outcome and closed durable terminal SHAs.
The successful original capture argv, source/review digests, normal terminal,
all source/image/build/raw trace pins, original stopped work-v5 report and
manifest, 16 surviving IR files/17 writes and both scored source images remain
bound. The six original qualified traces are retained verbatim. Both old
capture and failed work owner leases must be closed, independently held and
match their named inode. No old report or file is overwritten or relabelled.

The original qualification is `qualification/private-v8/report.json`, SHA
`4da05f6dd351609923f5f1ca227d3db2b33a354925614b74f9c5dae39839d159`.
The exact compact and optimized source/binary/production receipts are reused:

| Profile | Actual source SHA | Original scored binary SHA |
|---|---|---|
| pony-binarytrees-compact-precedence-v4 | 9fe1b9d288f21559aaf6d40706cd6fe3c969bfdddcbbaa0900fa3d97cc670a20 | 32e8e965c377dfe9b872adbd0777bcaf8d34686b75606d4038c8cf0d0e0e7e30 |
| pony-binarytrees-optimized-precedence-v4 | 351177026d084edcdabdb79651db63fffcb68edaee223b908f31e55f606f295a | d2e3f363a13d4d7b7f56ffc9dc10f31f58d89f28358ee79e1a0de518ba2aa033 |

The scored compiler is the unchanged private-v6 Pony0.74.0-95ad180 image with
recorded SHA `e35646fa9e56bc246e7cf3a867783def6374ce5d0192292d38627d768efa575a`.
The original compiler backend is LLVM23.1.2. Existing application bitcode is
not available: the production compiler removes package bitcode after linking.
The faulty IR emitter assigns both null-symbol entry/builtin packages the
same `.main.ll` suffix. The stopping bitcode emitter instead distinguishes
entry index0 (`.main.bc`) from the builtin package (`.builtin.bc`), and returns
before link/cleanup. These exact compiler sources explain the recovery:
[IR emitter](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genobj.c),
[bitcode writer](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/gensplit.cc),
[compiler lifecycle](https://raw.githubusercontent.com/ponylang/ponyc/95ad180/src/libponyc/codegen/genexe.cc).

Both modes cold-load only byte-verified original `guarded_prepare_v6.py`,
`revisions/work-full-v5/owned_service_work_v2.py` and original guard/harness
service. Their real paths remain intact; no source identity is spoofed and
preloaded benchmark modules are rejected. All original service source and
passed fixture evidence stays in the existing pin closure. No tests are
rerun. The original file-backed stdout/stderr, new owned PGID, exact native
exit0, bounded wait/cleanup/reap, no remaining group, actual common-FD
inheritance and exclusive receipt rules remain in charge.

First, `--qualify-reader` records a separately owned `llvm-dis --version` and
recursively inspects its native linked image closure using the existing service
`lipo -archs`/`otool -L` path. Every non-OS linked image must uniquely resolve
inside the explicit Homebrew Cellar root and is freshly pinned; each image
must have an ARM64 slice. This is current tool evidence, never a historical
compiler-image substitution. The actual capture's reader is
`/opt/homebrew/Cellar/llvm/23.1.3_1/bin/llvm-dis`, SHA
`a5f9b5c9bdd04dd73e3778a0dc46b89579dfc7686b292e9ff0db8881b492d693`.
Changed bytes or an actual non-LLVM23 version stop qualification. Environment
injection/override variables are removed. Original runtime and source pins are
rechecked before final evidence. Native image inspection records declared
transitive linker dependencies; it does not claim a new debugger memory map.

After root reviews that successful reader result, `--recover` requires its
actual `reader.json` SHA, body terminal SHA, closed durable owner and terminal
SHA. Its saved request/start/terminal must match this exact worker, source
digests, capture digests, reader mode and exclusive revision. All actual reader
version/image/owned receipts and final available pins revalidate before any
compiler/decoder invocation. A merely existing reader file or prepared packet
cannot authorize decoding.

For each of the two original production build receipts, only the exclusive
`--output` directory changes and `--pass bitcode` is inserted before the
unchanged scored source package argument. Target triple
`aarch64-apple-macosx14.4.0`, sysroot, strip flag, library paths, source paths,
bin name, compiler bytes and all other argv are unchanged. The exact authored
directory remains one pinned main; every compiler `--path` tree and bundled
Pony library must contain exactly its saved pinned `.pony` input set. The
actual stderr must name the original authored package. Emitted filenames must
match the complete duplicate-free write inventory; missing, unknown, aliased
or non-bitcode outputs reject. Both distinct entry and builtin bitcode are
preserved and decoded separately. The entry's actual target/source filename
and defined Node/Worker/Main symbols are required; the builtin must lack those
unprefixed authored symbols. Every vendor bitcode remains pinned for review.
No glob-first selection or ambiguity exemption is introduced.

Compact scored ARM reuses the original owned `otool -tvV` output with actual
SHA `3053732417af086c0754d2c82a7fc1c1cb92a4046f0d40b9ef9dec400e5fe289`.
Only the missing optimized scored ARM is captured with one new bounded
`otool -tvV ORIGINAL_OPTIMIZED_BINARY` command. No diagnostic executable is
built, executed or substituted for either scored image. Bitcode precedes final
LTO/link optimization, so IR alone cannot prove final native allocation. Root
and an independent semantic reviewer must compare the resulting IR with each
unchanged original scored ARM before populating any of the four pending
allocation/retention booleans or creating later work qualification.

Each attempt uses a fresh exclusive `jobs/<revision>` directory and holds
common continuously, with original closed owner leases retained through final
pin review. Failure preserves every available output byte, original expected
pins, actual readable pins and per-path integrity errors. Final statuses are
only `complete-reader-evidence-awaiting-root-review` or
`complete-scored-evidence-awaiting-semantic-review`; they do not grant LOC,
source work, full studies or chart admission.

After independent source review, root freezes these two files and captures
their actual digests plus the original capture outcome/terminal digests under
common. The first reviewed durable command uses the bundled isolated Python:

```text
python3.12 -I -S -B -u ABSOLUTE/recover.py --qualify-reader
  --revision reader-llvm23-v1
  --worker-sha256 ACTUAL_WORKER_SHA --review-sha256 ACTUAL_REVIEW_SHA
  --capture-outcome-sha256 ACTUAL_CAPTURE_OUTCOME_SHA
  --capture-owner-terminal-sha256 ACTUAL_CAPTURE_DURABLE_TERMINAL_SHA
```

The future recovery command has the same four digest arguments, mode
`--recover`, fresh revision `scored-bitcode-v1` and these additional actual
inputs, produced and reviewed only after the first job closes:

```text
  --reader-receipt ABSOLUTE/reader-llvm23-v1/reader.json
  --reader-sha256 ACTUAL_READER_RECEIPT_SHA
  --reader-terminal-sha256 ACTUAL_READER_BODY_TERMINAL_SHA
  --reader-owner ABSOLUTE/CLOSED_READER_DURABLE_JOB
  --reader-owner-terminal-sha256 ACTUAL_READER_DURABLE_TERMINAL_SHA
```

Both recipes must use the actual bundled Python path recorded in the worker;
the common-lock scheduling and exact durable launch are root's decisions.
No future receipt/digest is fabricated in this source packet.

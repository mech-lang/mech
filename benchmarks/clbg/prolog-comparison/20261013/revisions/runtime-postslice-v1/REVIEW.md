# SWI runtime discovery after the preserved ARM64 slice

Mutable two-file source candidate: `discover_runtime.py` and this note. Source
only, unexecuted/unhashed/unfrozen. Root and independent peer must read the full
source, review the closed actual input pins and literal launch arguments,
freeze the accepted source, and authorize one fresh native discovery job.
No download, extraction, mount, app copy, slicing, fixture, benchmark or runtime
command was run while preparing this packet.

## Actual closed prior run

`continuation/owned-jobs/prolog-private-runtime-directory-v1` failed normally
with exit 1 at 2026-10-13T19:47:00.811342+00:00, without requested stops. It had
already completed source extraction, app copies and ARM64 slicing. The exact
failure was `Actual bundle home differs; retain and review this layout`, at
the assertion for guessed `Contents/swipl`. It did not create its `runtime`
command directory, so no successful version/probe command exists to rerun.

The actual failure stays unchanged at `.build/swi-runtime-directory-v1`:

- Report SHA256: `3acb7a2c6dc10618e9eead402575bdc0b60015c6dc1feadca926f871289ed69c`.
- Terminal SHA256: `9d99b5a07bc771979b4b0ed52e7e76d73918e9a5c41126c00664a3e96f01c64e`.
- Successful original slice receipt SHA256: `d6bc7b7920a451de54c6160450adc135324e9d7027b0497ad353c014f7bf0f50`.

The actual attach and exact requested-mount detach commands both exited zero.
The original report records `exact-requested-mount-detached`; under-common
inspection independently confirmed the requested path is no longer a mount.
No guessed device or second detach is introduced.

## Actual directory evidence

Under-common metadata inspection verified 13,179 source/result/image pins and
captured all 2,977 entries of the original ARM64 private application. The unique
`boot.prc` is under `arm64-app/SWI-Prolog.app/Contents/Resources/swipl`; that same
home contains the `library` directory. The old `Contents/swipl` path is absent.
The command-line executable is the exact publisher-selected
`Contents/MacOS/swipl`, bound by the successful original fat-to-ARM64 slice:

- Original fat CLI SHA256: `e47fc404978145ade0ddde49a19542ca3f92515b4eea7051d33d5ff368896e4b`.
- Existing ARM64 CLI SHA256: `554814b1da7c0c30595adafb758145e42a6134c696b47c8870d359008246443e`.

Its actual thin Mach-O header reports ARM64 independently of a repeated lipo
query. The exact layout and symlink inventory is at
`continuation/jobs/prolog-postslice-layout-review-v3/native-layout.json`, bound
by its closed review:

- Review SHA256: `da4b6493af38e5af06cbd080346577292eb4113e51632bfd15394cf509fc3f4d`.
- Review terminal SHA256: `02439e159cc1d5665ee5ededc7e241130ca72cec308f63484e715120b5ecda95`.

The publisher's Info.plist is retained as exact raw metadata: its XML is rejected
by plistlib, so no invented parsed property is used to select the CLI/home.
The failed metadata inspectors v1/v2 are preserved with explicit failure notes;
v3 applies the actual original C3 receipt contract and completed normally.
No inspector started a native process.

## Minimal additive continuation

The new worker requires exact original report/terminal and layout-review/terminal
SHA arguments. It consumes the original closed private lease and the exact
old durable request/launch/worker-start/command-start/terminal, including literal
bundled Python 3.12 argv/cwd, zero requested stops and normal failed exit. Every
original pin is rechecked, as are the successful attach/detach/slice receipts,
full layout/symlink inventory, unique boot/library home, existing thin/fat images,
original four extraction fixture closure, and original six runtime/preparation
plus eleven ownership fixture closures. No passed fixture is executed again.

The frozen original directory bootstrap is cold-loaded only to reuse its
already reviewed source/receipt helpers, service and runtime/image validators.
Its `main`, extractor, attach/copy/slice code and metadata sealer are never
called. Its module file identity is preserved. The new packet's own source pin
also enters the strict reviewed import closure; no original file is edited.

The new exclusive output is `.build/swi-runtime-postslice-v1`. It uses the
existing private ARM64 app in place and sets only its owned command environment's
`SWI_HOME_DIR` to the actually observed home. The first required runtime version
command and bounded feature probe, then the original loaded-image closure
commands, are the only remaining native work. The unchanged validators require
SWI 10.0.2, unbounded integers, default GC, actual GMP/hash/regex primitives,
private foreign images, source closure, and ARM64 image/rpath/install-ID closure.
The original scored source/work/timing admission remains pending.

One common mutex covers all hashes/imports/native commands and final terminal
pin closure, with the named inode check. The original private lease is acquired
nonblockingly inside common and retained until this additive job closes. Every
native command uses the unchanged reviewed owned-command body and actual common
FD, with its original finite bounds and cleanup. The full original native app
entry set and all original source/result bytes must remain unchanged; failures
retain new output/partial pins without relabelling the prior report.

Downloads, re-extractions, mounts, copies, slices, fixture reexecutions, version
reexecutions, benchmark runs, global installs and automatic retries are zero.
Even successful discovery leaves runtime configuration, source work and chart
admission false, with full processes authorized zero and source/library semantic
review pending.

## Literal deferred entry

Run only after root/peer acceptance and freeze through the existing durable
launcher, using bundled Python 3.12 `-I -S -B -u` and suggested fresh durable ID
`prolog-runtime-postslice-discovery-v1`. The required arguments are:

- `--proposal-sha256 5b9186bb51e748fe69b16dcd35a632dd9ed8556abdbc30cbab45159a36a1bf88`
- `--runtime-proposal-sha256 c72f7ab0d41b195acafff131e5f2cd7c61577a9fa14898af15e4c543b8daa322`
- `--worker-sha256` the accepted new source's actual frozen SHA (not yet captured).
- `--failed-report-sha256 3acb7a2c6dc10618e9eead402575bdc0b60015c6dc1feadca926f871289ed69c`
- `--failed-terminal-sha256 9d99b5a07bc771979b4b0ed52e7e76d73918e9a5c41126c00664a3e96f01c64e`
- `--layout-review-sha256 da4b6493af38e5af06cbd080346577292eb4113e51632bfd15394cf509fc3f4d`
- `--layout-terminal-sha256 02439e159cc1d5665ee5ededc7e241130ca72cec308f63484e715120b5ecda95`

This source release adds no native launch authorization by itself.

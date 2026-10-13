# Rebol SDK route from the actual retained source

Source-only research, not executable intake/generation authorization. No
hash, parser/import, download, native command, header generation, fixture,
compiler, benchmark, Git or dataset mutation ran while writing this note.
Original source/runtime bytes and all prior success/failure receipts remain
unchanged.

The actual completed source intake is
`runtime-qualification/saved-runtime-source-intake-v1/report.json`, SHA
`0b467ff8c3fc42ba66e8a83b24d1ad412b37bc5eae43c4a990ee93f4c8d9fe42`,
body terminal
`21059d5c19ee7d26b8f50d9475e105a7249781c1b690adac776ab7f4444ab373`,
durable terminal
`56e4ff4edc603ab063e85934baae0cc70d2bfec94eb92df7cd39637567a51da7`.
Root's1,064-pin review is
`continuation/jobs/rebol-saved-source-closed-review-v1/receipt.json`, SHA
`3f7b4fdd2eeaacc8c2a86e87b33276a76eea0f1d45a8046f47a9114a3fe45b86`.
It verifies898 source files and archive SHA
`48df5f2c4cc62d7b4085c908427dd6a698540c32ec4c2a7893a2e5c0d639f671`,
with all five native runtime successes reused and zero native reexecution.
Archive digest was newly captured from the official full-commit URL; independent
Git-tree identity remains unproved, as the actual receipt explicitly records.

The source authority is
`.build/saved-runtime-source-intake-v1/source-original/`. Its preserved
`make/rebol3.nest` explicitly names the Bulk macOS ARM64 executable recipe,
composing the macOS settings, complete Bulk feature set and ARM64 executable
settings. Its pre-make action supplies Builder's resolved spec to
`make/pre-make.r3`. No usable `spec-core.reb` is present. The raw nest must not
be treated as that resolved spec or replaced with a guessed hand-built list.

The preserved `pre-make.r3` consumes resolved core/host/mezz/boot arrays and
platform/architecture/product/configuration, then executes the full six
vendor stages. Those generators create include files, boot data, host/OS
extension code and the complete external ABI header. Generation writes into
the source tree, so only a new898-file byte-equal isolated copy may be used.
Original pins must be rechecked afterward. The copied source is not a newly
scored implementation and generated vendor headers are not authored benchmark
LOC. The whole authored primitive C/loader/support remains separately counted.

The official Builder dependency is now concrete. The tagged Rebol workflow
selects the installer0.21.15. Its [installer source](https://raw.githubusercontent.com/Oldes/install-siskin/v0.21.15/action.yml)
selects the corresponding release asset. The [publisher listing](https://github.com/Siskin-framework/Builder/releases/expanded_assets/0.21.15)
records `siskin-macos-aarch64.gz`, SHA
`a252cfc599ddd591651d626a85c82a36d20b5c41d9fc8dd3b4b8b4d878c9fcb8`.
The [release commit](https://github.com/Siskin-framework/Builder/commit/5dd9ec60cb96b1ae3cda0ba56ba033d2cf86042e)
is `5dd9ec60cb96b1ae3cda0ba56ba033d2cf86042e`. These are primary publisher
observations, not locally downloaded/verified Builder bytes. Future source
intake should use that full commit, with no invented prior archive SHA.

The [tagged Builder core](https://raw.githubusercontent.com/Siskin-framework/Builder/0.21.15/tree/rebol/Siskin/siskin.reb)
offers `--test`. It still writes the resolved target spec before suppressing
command evaluation. Its source banner says0.21.2, despite release0.21.15;
actual version output remains an observation to capture. Proposed spec-only
argv, after separate Builder image/source intake and review:

```text
ABSOLUTE_BUILDER ABSOLUTE_ISOLATED_COPY/make/rebol3.nest --test %rebol3-bulk-macos-arm64
```

Require an actual resolved spec, no object/executable output, no child
compiler/generator execution, exact source correspondence and contained paths.
Keep explicit target selection and disable interactive input. Any unexpected
repository/dependency action must stop. This source-derived route needs actual
process/output review; the flag alone is not proof of a completed safe run.

The next minimal additive controller should reuse the existing closed runtime,
six contract and eleven owned-service fixture receipts. Under common it may
intake/inspect the exact official Builder image/source, copy the898 original
files, run that single spec-only command and retain the actual spec/journal.
The fixed-cwd DEVNULL service requires an explicit reviewed copied-directory
launcher; effective cwd and actual native child identities must be recorded,
not silently reassigned to an old receipt. No original runtime download,
five native runtime queries, archive extraction or fixture rerun is needed.
This intake/spec worker is distinct from header generation and all workloads.

Before generation, root must read the entire actual resolved spec and verify
its Bulk/macOS/ARM64/version3.22.1 settings; every core/host/mezz/boot input,
root/source/temp/output path and conditional feature must correspond to the
retained official source and exclusive copy. Preserve unknown fields for
review rather than resolving them through defaults. Keep all tool/source and
result hashes closed under the common lock and old private leases.

The local generator `make/tools/utils.reb` introduces two concrete shell routes:
macOS version via `sw_vers -productVersion`, and the attempted libc inspection
`ldd --version`. Its fallback Linux-release query is reachable if the macOS
probe fails. These are actual future subprocess/OS-input gates, not zero-child
pure-Rebol assumptions. No global permissions or host configuration change is
proposed. Pin the exact shell/tool paths and record attempted/missing tools and
all bounded process-group cleanup before interpreting generated output.

After the actual spec closes, a separate reviewed generation command may
invoke the unchanged vendor pre-make route with the selected retained3.22.1
runtime on that copy. Root must then consume all generated files, full vendor
dependency arrays, private include/API layout and original preservation before
an extension build. The future GMP/PCRE/byte-I/O primitives remain generic;
all benchmark algorithms stay authored Rebol. Header/runtime ABI compatibility,
pipe stdin/stdout, error cleanup/GC, primitive fixtures, source counts, bounded
programs, work, full90, runtime metadata and chart admission remain pending.

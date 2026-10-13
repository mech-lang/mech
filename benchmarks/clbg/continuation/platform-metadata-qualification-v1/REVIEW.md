# Preserved platform metadata qualification v1

Source-only candidate. No hash, import, fixture, compiler, version probe, native
program or benchmark has run. The existing platform proposal and original
dataset/reports/samples/sources remain unchanged.

`expected-program-ids.json` lists exactly the 100 existing displayed identities:
Java 17, Erlang 14, C# JIT 20, C# NativeAOT 19, F# JIT 15 and F# NativeAOT 15.
These literal IDs were transcribed from the existing read-only candidate
extractions. Qualification reads the admitted dataset, requires exact identities
and per-language counts, then independently selects one preserved report row by
language/benchmark/id. A candidate's `verified:false` is not qualification.

The direct controller holds both a nonblocking owner lock and the actual common
`/tmp/mech-clbg-timing.lock` before hashing, snapshot creation, candidate imports,
fixtures or reconciliation. It freezes copies of the qualifier, six fixtures,
literal inventory, review, original dataset and both candidate extractions. It
requires exactly six discovered and actual started test IDs, zero failures,
errors, skips, expected failures or unexpected successes, and pins the original
raw fixture log/receipt. It has no subprocess API. It requires a fresh output and
checks every observed input byte pin again before writing results.

Every admitted profile must match its unchanged report provenance pin, exact
three saved positive finite samples and median, displayed source bundle/pins and
saved command. Changed report/source/sample identity stops with preserved failed
receipt. Missing historical backend evidence or unavailable preserved backend
artifacts produces an explicit unresolved annotation and concrete reason; it
cannot receive `verified:true`. Missing report/display source is an integrity
failure. Output is a separate `measured-backends.json` with annotations keyed by
the 100 identities, additive report provenance and complete local byte pins.
The controller never changes the admitted dataset or admits new profiles.

Java retime rows directly preserve `OpenJDK JIT` execution fields, `java -cp`
commands and corresponding `javac -d` commands. The current historical report
has no `toolchains.java` entry, so those profiles receive the generic label
`OpenJDK JVM JIT`. Their saved `command_assets` bind the launcher and complete
classpath class-file inventory to `asset_hashes`. Original reports are pinned
through `retime.original_reports` and reconciled for command/source lineage;
their old runtime version is not attributed to the new retime samples. The two
regex/Pi reports retain actual saved OpenJDK 27 strings and can show that stored
version. No current executable version or path-derived version substitutes for
historical evidence.

Erlang reconciles saved timing/build rows, source/module pins, launcher/compiler/
engine receipts, `+S 8:8`, module lookup and the actual saved OTP 29.1.1,
ERTS 17.1, native aarch64 and JIT receipt. Existing Mach-O engine bytes must be
native ARM64. .NET reconciles timing/source-audit/summary host receipts and saved
artifact pins. JIT requires the pinned dotnet host plus application DLL and a
successful saved build. NativeAOT requires matching saved publish receipt,
native executable command, explicit `osx-arm64`, self-contained publication,
the preserved project's `PublishAot=true` and actual native ARM64 Mach-O binary
bytes. The project has no saved historical digest: its newly observed pin is an
additional present-day consistency check, explicitly labelled that way. The
historical backend binding comes from the saved publish receipt, source-audit
copy, explicit saved AOT execution field and exact executed native binary pin.
The preserved SDK 10.0.401 is recorded as build provenance. This packet does not
infer runtime 10.0.12 from producer prose or SDK version; labels are generic
`.NET C#/F# JIT` and `.NET C#/F# NativeAOT · native ARM64`.

The frontend evidence contract is preserved: literal verified flags, exact
timing report/source pins/command and a separately pinned saved runtime receipt
with an exact JSON pointer and the preserved value (original string, or explicit
JSON serialization for structured runtime/build records). Qualified runtime
receipt report pins are emitted in additive `provenance`. A later reviewed
producer may overlay these annotations and labels; this worker performs no
frontend integration, source normalization, source-work qualification or timing.

Proposed single direct locked command after root review (not executed):

```
/opt/homebrew/opt/python@3.14/bin/python3.14 -B benchmarks/clbg/continuation/platform-metadata-qualification-v1/guarded_qualify.py --output benchmarks/clbg/continuation/jobs/platform-metadata-qualification-v1
```

Six fixed fixture IDs are declared in `guarded_qualify.EXPECTED`: historical Java
JIT/classpath evidence, .NET host/DLL versus NativeAOT, actual ARM64 publish/
native command, native Mach-O architecture, changed/partial/NaN/sample median,
and exact 100 identities/counts. Synthetic fixtures are not published evidence.

# Additive Crystal release-layout repair

The approved private-v1 qualification stopped safely before any native compiler
or benchmark process. Its publisher-digest-verified archive contains both
`crystal-1.21.1-1/bin/crystal`, a Bash launcher, and
`crystal-1.21.1-1/embedded/bin/crystal`, the actual universal Mach-O compiler.
The broad bin/crystal discovery matched both and rejected the layout. Preserve
the sealed v1 proposal, all24 actual passed fixtures, stopped report, downloaded
archive, extraction and immutable source snapshot byte-for-byte.

`guarded_prepare_v2.py` is an additive script. It selects the exact embedded
compiler and `src/prelude.cr` from the official versioned package, observes
the shell launcher without running it, and requires guarded actual ARM64 lipo
and version checks before compilation. Source text, compiler flags, ownership
guard, old error/restoration callbacks, all20 applications and work proofs are
unchanged. General I/O, GMP, PCRE2 and GC APIs stay ordinary standard support.

The explicit library search order is recorded GMP6.3.0 and PCRE2 10.49 Cellar
directories first, then the bundled GC directory. The latter also contains
`libpcre2-8.a`; placing it first could silently select that unrecorded PCRE2
instead of the intended dynamic10.49 library. Runtime uses the same real paths.
Actual dynamic lookup and real-target pins remain required, along with the
complete immutable release package. Bundled compiler dependencies may differ
from application dependencies and must be labeled as their actual images.

Eight new pure-Python mocked layout tests require exact paths, reject the shell
wrapper as a compiler, check the private source/GC layout and Cellar-first
ordering, and prove unchanged text for the previously tested generic guard
functions. Run those once under the common mutex, then seal a fresh
`proposals/private-layout-v2.json`. Reuse the unchanged actual12+12 fixture
receipts; do not rerun them. Root reviews the new concrete digest before any
manual `--run --revision private-layout-v2` bootstrap/qualification launch.
No automatic retry, archive mutation, source replacement, full performance
sample, result admission or website update is authorized by this repair.

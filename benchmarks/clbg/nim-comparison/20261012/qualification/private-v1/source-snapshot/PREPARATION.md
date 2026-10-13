This is an additive preparation proposal. No bootstrap, compilation, fixture,
qualification, full sample or score has run. Root reviews the scripts before
queuing one bounded sealing job, and reviews the emitted exact proposal digest
before the separate private preparation job.

The sealing command is:

    python3 -B guarded_prepare.py --seal --proposal-file proposals/private-v1.json

It holds the common mutex, runs ten mocked process ownership/restoration
fixtures, resolves the installed SDK through its own guarded xcrun process,
and pins the twenty source profiles, generic adapters, scripts, existing
immutable harness/control inputs, Clang resources and SDK inputs. Every child
inherits the held open descriptor. It never replaces an existing proposal or
fixture receipt. Source revisions require a new proposal path and revision.

After exact root approval, the proposed preparation command is:

    python3 -B guarded_prepare.py --run --proposal-file proposals/private-v1.json \
      --proposal-sha256 <reviewed-sha256> --revision private-v1

The owner takes its own preparation lease and common mutex before verifying
inputs, then copies the reviewed sources to an immutable revision directory.
It privately downloads the official native Nim 2.2.12 archive, verifies its
publisher digest, and vendors the exact pure BigInts and public nim-gmp commits.
No global Nim/Nimble install or user configuration is used. Every compiler has
its own cache, a new process group, inherited mutex descriptor, timeout, complete
output journal and unconditional cleanup beginning before identity/journal
setup. SDK 14.4 is passed explicitly to LLVM 22 for compilation and linking.
Floating contraction is disabled. PCRE1 and GMP paths are explicit and pinned.

The twenty release sources are built and checked against complete independent
small/adversarial outputs, including odd Mandelbrot dimensions, PBM tails,
N200, zero and partial pi lines, empty maps, exact THREE token boundaries,
comments, CR/LF/CRLF, reverse-complement IUPAC and regex replacement cases.
Each actual attempt uses the reviewed common sampler/native guard. Output,
status, pins, environment, resource/monitor or cleanup failure is fatal.
No retry or implicit resume occurs; actual failed bytes and attempt ledgers
remain available. Temporary input/work/validator overrides are restored even
if setup or execution raises, preserving absent keys as absent.

Further unscored primitive/ownership proofs check pure BigInts arithmetic,
GMP LP64 layout, signed division and destination aliasing. Seven simultaneous
ORC workers return growable tables and strings that the parent reads and resets
after joining. AddressSanitizer checks this fixture and actual compact/optimized
nucleotide audit copies. Those copies retain every source map update, join and
stdout statement, dump all seven completed maps, reconcile every overlapping
window against an independent map oracle, then destroy the parent-owned maps.
No production source is edited or replaced by these audit copies.

Passing these checks seals only bounded output and ownership evidence. Actual
work traces for the remaining tasks, the canonical LOC audit, independent
source review and per-benchmark manifest review remain explicit gates. This
proposal cannot start full timings, issue chart scores or alter frozen cohorts.

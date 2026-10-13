# Prolog reader31: actual qualified runtime and successful halt outside catch

Source-only additive repair. The never-executed v1/v2 reader packets and every
actual native success/failure are preserved. No imports, hashes, parser,
fixtures, native command, compiler, Git or data write ran while authoring.

`foreign_prolog.py` and `test_prolog.py` are exact copies of v2; the 31 fixture
methods, IDs, projector meaning and assertions are unchanged. `scan_source.pl`
only moves successful `halt(0)` after catch, leaving checked ending in `nl`.
That is the concrete SWI halt-unwind fix already established by the actual
runtime probe. Error printing and native nonzero remain unchanged. The reader
still treats source as data, captures the native term-position/operator/reader
inventory and never loads source directives or invokes subject predicates.

The runner replaces only the stale runtime-v2 prerequisite/loader block. It
uses the narrow actual prepared-work-v2/qualified_preparation.py helper and
its 139d/cab6/716b/21d closed preparation, real qualified runtime schema7fa,
seven image/35-command closure and old 6+11 fixtures. The helper's raw-source
imports preserve actual module paths. No old bootstrap, preparation, compiler,
fixture or workload is reexecuted. All original 20 source/QLF/WAM inventories
and 156 complete native preparation receipts are verified before reader work.

The remaining original runner mechanics are retained: same-process unittest
source snapshot and module-origin checks, exactly 31 discovered/started methods,
owned scanner facade inside only the freshly loaded fixture module, exact argv,
20-second native reader bound, original source copies/raw packet/projection
archives, and terminal input/artifact pin closure. Expected parser errors must
be real clean nonzero native statuses with empty remaining owned groups; a
timeout/supervisor/cleanup error cannot satisfy those fixtures. Scanner stdout
is observed, not replaced by a synthetic AST. Every successful read must now
exit native0 after catch. The two parser-error scenarios still require failure.

Root and independent peer must read both this five-file packet and the shared
two-file actual preparation authority (`qualified_preparation.py`, its work
`REVIEW.md`) before capture. Root creates a read-only packet with exactly
`schema` and `pins`: schema `frozen-prolog-reader31-actual-preparation-v3`, pins
the five absolute reader files, the authority/helper note and the actual
resolved Python executable (eight paths). Actual source digests are root's
future captures; no SHA has been invented. Shared source inputs are read-only
before execution and are rechecked at terminal.

```text
bundled-python3.12 -I -S -B -u prolog-proposal-v3/run_fixtures.py
  --packet ABSOLUTE_SOURCE_PACKET --packet-sha256 ACTUAL_PACKET_SHA
  --output ABSOLUTE_PROLOG_PROPOSAL_V3/jobs/reader31-v1
```

The output must be fresh within this packet's jobs directory. Common inode
79859134 and three original closed private leases span prerequisite imports,
all reader attempts and terminal journal. Run once; preserve any failure and
review its actual cause before considering a new additive repair. No source
LOC replay occurs here. Actual all-20 whole-bundle source reading, helper/import
ownership review, numeric canonical counts, work admission and full90 remain
separate pending gates after this exact fixture receipt is consumed.

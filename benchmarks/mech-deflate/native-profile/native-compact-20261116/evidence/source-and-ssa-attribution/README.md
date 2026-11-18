# Source semantics and SSA construction

Six balanced blocks run the same probe executable with instrumentation
disabled, enabled for source semantics and catalog, or enabled for SSA.
Every process reproduces the maintained bytecode, encoder object,
and native metadata. Timings below are diagnostic scopes within the
compiler. The maintained source-through-final-link comparison remains
the deployment measurement. [Probe plan](plan.json) ·
[Compiler build](build.json) · [Raw logs](logs/) · [Summary](summary.json)

## Source semantics

| Scope | Median elapsed |
| --- | ---: |
| Collect document units | 0.712 ms |
| Create builder | 0.019 ms |
| Register types and FSMs | 0.547 ms |
| Register functions and imports | 0.280 ms |
| Declare inputs and scan units | 3.721 ms |
| Lower document units | 162.148 ms |
| Publish and order document | 0.078 ms |
| Resolve pending values | 0.000 ms |
| Finalize schemas | 8.998 ms |
| Finalize constant store | 3.857 ms |
| Pack inputs | 0.001 ms |
| Canonicalize requirement table | 0.007 ms |
| Materialize nodes including requirement lookups | 2.293 ms |
| Pack outputs and graph | 4.933 ms |

The median frontend interval with scope tracing was
**231.9 ms**,
with **44.2 ms** outside
these scopes. This remainder includes the surrounding calls and cleanup.

The current requirement scan made **0 lookups**
and **0 comparisons** in a measured
**0.000 ms**. It is included in node materialization.

The cold catalog took **0.106 ms** to start its
worker thread. Builder creation was **0.000 ms**,
runtime installation **60.818 ms**,
source installation **1.495 ms**, and catalog
finalization and validation **0.010 ms**.
The enclosing interval also includes thread completion and handoff.

## SSA occupancy and provenance

The probes saw **21,609 variable records** across finalized builders,
including **1,773 with one cached definition**.
**994** used dense storage.
There were **106,843** sparse capacity growth
events and **2,798** dense resizing events.
The front end submitted **50,309** variable use
requests. The state machine recorded **89,745**
local hits and **3,040,050** nonlocal requests,
including recursive state-machine visits. Raw observations preserve
sample counts and times for reads and explicit definitions. SSA probes
increase the native phase from
**1.390 s** to
**1.789 s**. Use the
census for work counts and compare optimization timings with probes off.

| Largest checked root, entry origin | Lanes | Reads | Writes after seeding | Non-entry parameter reads |
| --- | ---: | ---: | ---: | ---: |
| constant never written | 1,522 | 2,118 | 0 | 1,919 |
| constant written | 17 | 7,033 | 7,012 | 7,030 |
| input never written | 10 | 23 | 0 | 5 |
| input written | 2 | 22 | 18 | 12 |
| seed never written | 144 | 0 | 0 | 0 |
| seed written | 14,648 | 31,698 | 31,546 | 27,416 |
| table never written | 30 | 23 | 0 | 22 |

The lane census covers actual emission calls, including synthetic,
ownership and failure paths seen by the emitter. Non-entry parameter
reads can revisit the same provisional value. The count is a starting
point for selecting a value family and proving dominance and immutability
on all relevant paths. The outcome prioritizes document-unit lowering
over indexing the unused requirement lookup. Within the checked root,
unwritten constant lanes account for 1,919 non-entry parameter reads.
Price that family before choosing a construction change.

# Replay a winning function definition

The compiler records the existing default `MECH_CODE_ALT` function winner
on sealed one-shot input. It uses the same bounded recipe as variable
definitions and checks the exact function cache root and delimiter. A nested
trial can borrow the raw observer once its parent has recorded the first
access. It restores that witness on exit. Pending first accesses retain the
ordinary grammar path. Definitions that exceed a resource or identity guard
continue through the original selection and parse.

On the prior parser, function winners accounted for
313.1 ms inclusive and
285.4 ms exclusive. All 369 matched shapes
fitted the census bounds. The timed diagnostic and plain parsers took
1.285 s and 1.306 s in
four balanced pairs. The timed probe measures elapsed spans and the
separate census measures shape counts.
[Post-variable replay profile](remaining-replay-profile.json)

| Standalone parser, eight balanced orders | Median |
| --- | ---: |
| Published variable replay control | 1.30545 s |
| Function replay with nested capture declined | 1.17786 s |
| Function replay with settled nested witness | **1.00932 s** |
| Settled nested witness with function application disabled | 1.31526 s |

The full differential reproduces raw events and live cache publications,
node/token/diagnostic identities, tree and indexes, diagnostics, fuel,
rewinds, poll grants and resource peaks. All 369 function recipes apply.
The outcome includes the original nested variable path, finite grants,
segmented UTF-8, three forms of function body, recovery, and the first-raw
witness transitions. All 535 existing syntax tests pass. The published
compiler uses the uninstrumented parser.
[Both standalone experiments and exact test source](standalone-evidence.tar.gz)

| Eight matched Apple M1 builds | Mech variable replay | Mech function replay | Roc | libdeflate |
| --- | ---: | ---: | ---: | ---: |
| Source through final link | 3.13523 s | 2.84147 s | 2.98471 s | 0.48041 s |
| Source through strip | 3.14594 s | 2.85210 s | 2.99527 s | 0.49103 s |

The matched source-through-link improvement is **9.39%**.
Mech takes **0.951× Roc** and **5.912× libdeflate** time.
The compilers and host support are prebuilt in these measurements. Cargo's
separate optimized compiler rebuild took 626.70 s.
All 16 Mech builds reproduce the exact bytecode, 71,768-byte encoder,
complete metadata, and stripped deployment file. The reproduced benchmark
worker passed 162 exact calls. Its archived orders retain 81/81 against Roc
and 61/81 against libdeflate.
[Matched call boundary and input pins](source-through-link-plan.json)
· [Raw observations](source-through-link-observations.json.gz)
· [Artifact and RSS summary](summary.json)
· [Exact worker](worker.json)

| Mech phase | Variable replay | Function replay |
| --- | ---: | ---: |
| parse | 1.31359 s | 1.01802 s |
| catalog | 0.05870 s | 0.06171 s |
| frontend | 0.22706 s | 0.22695 s |
| artifact | 0.00696 s | 0.00698 s |
| encode | 0.00262 s | 0.00258 s |
| native | 1.46383 s | 1.46224 s |

Run `python3 -B replay-evidence.py` from this evidence directory to replay the
archived comparison and check the recorded identities and ratios. The
maintained compiler source and the existing `rebuild.py` recipe reproduce
code generation. The standalone tar contains the commands and input pins
from the original local toolchain.

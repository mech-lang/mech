# Skip empty parser transactions for resolved terminal misses

A sealed terminal cache miss leaves the parser cursor, event journal, and marker stack intact. The base continuation records the same parser work, rewind count, and peak rule depth directly.

The standalone parse saved a median **8.3 ms** in eight paired runs. The syntax tree, parser steps, and event count match the control. All 538 syntax tests passed. Eight additional streamed inputs matched through finalization, including bounded work and malformed input. [Parser timings](parser-timing.json) · [Stream accounting](stream-accounting.json) · [Syntax tests](syntax-tests.log)

Fresh source through final link takes **2.182 s** with a median paired saving of **15.5 ms** in 7/8 faster trials. [Matched builds and phases](deployment.json)

The **65,276 B** encoder, canonical bytecode, metadata, and linked worker are byte-identical to the qualified compact matcher. The matched worker holds 81/81 against Roc and 62/81 against libdeflate in both full orders. [Source replay](verified-replay.json) · [Linked checks](linked-checks.json) · [Original runtime qualification](../compact-match-equality/full-screen.json)

Apply the compiler patch to the pinned parent and run the recorded compiler rebuild and replay. [Pinned source](source.json) · [Compiler command](compiler-build.json) · [Flags and profiles](candidate.json)

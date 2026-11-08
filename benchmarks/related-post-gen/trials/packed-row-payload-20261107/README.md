# IR277: checked packed row payload sizing, unselected

Fixed-row transport uses checked count*width for canonical packed scalar banks
instead of reading every element to total its payload. Text/generic cells retain
the scan. Every original shape, account, alias, capacity, output and work check
remains; the exact original work prices are retained. No source algorithm change.
This composes with IR276 matrix length, uses Cranelift0.131.3 and adds no dependency.
All51 native arithmetic functions and raw Object text are exactly parentIR276/264.

Source80c91ec89ec0b57ddc1820a57fec4063c920de06 passes405 actual CI names,8 focused
original/decoded/restored/generic-scan ablation checks,25 image/caller checks,
4 explicit optionalLLVM checks,133 exact scalar/DEFLATE rows and35 actual related
outputs. Test fixture mistakes and disk-full postprocessing failures are retained
with corrections. No complete GPU/hostedCI claim; no scalar entry retired.

All837 latency and196 resource outputs match Rust with no exclusions or overlapping
builds/tests/profiles/archives. All22Rust controls remain. Preparation, allocation,
worker creation/run/join and materialization stay in continuous postJSON; restoration,
JSONIO and teardown stay in launch-to-exit. References are selected independently.

Processing is2.626209ms/1.740123x
fastestRust. Selected242 is2.806917ms/
1.859860x in this cohort. Processing beats all
three required controls; whole-process noninferiority passes against immediate276,
but fails against264 and selected242. No unchanged confirmation or promotion.

Run python3 verify.py to rederive1033 cohort comparisons,35 qualification outputs,
bootstrap intervals, fastestRust at both boundaries, actual test names and Object
text. Resources retain allocations, requested/peak heap, instructions/cycles, RSS,
restore metrics and binary/code/stack. Rust parity remains unmet and the goal active.

# Isolated Uiua fannkuch parallel candidates

These additive sources preserve the frozen fast-v2 primitive reversal-index and
flat-gather recurrence. No original source, suite audit, coordinator or timing
report is edited. All workload logic uses ordinary Uiua operations and explicit
`spawn`/`wait`; there is no native benchmark executor or new host helper.

`sources/parallel.ua` has 27 LOC, including all worker helpers. It partitions
global ranks into `[floor(j*Total/T), floor((j+1)*Total/T))`, where
`T=min(8,Total)`. Each worker keeps bounded blocks of at most 1024 permutations,
materializes every assigned rank, completes its actual prefix reversals, and
uses global rank parity. Only the parent prints, after waiting for every worker.

`sources/block.ua` has 35 LOC, including all block-generator and worker helpers.
It applies widths in descending order `N,N-1,...,2`. For each width, it forms
native broadcast rotation indices, adds `row*N`, gathers the complete bounded
matrix, and restores its shape. Each selected column is
`col<w ? (col+rotation[row]) mod w : col`. It performs the same canonical
permutations and prefix flips, with no permutation or flip-result cache.

Run `python3 validate.py`. One job holds `/tmp/mech-clbg-timing.lock` throughout
preparation, hashing, independent reference computation, runtime probes and
manifest sealing. Its native children inherit the lock descriptor. The job has
a 15-minute execution budget after mutex acquisition. Each probe has a
120-second timeout and a 2-GiB resident-memory ceiling; interrupted owned
probe groups are killed and reaped. Existing benchmark processes are untouched.

Qualification compares complete n1..8 outputs at every requested worker cap
1..8 against independent incremental enumeration and the retained v2 source.
Workers return ordered diagnostic records to the parent for exhaustive initial
permutation and actual prefix-reversal checks, including finished lanes, odd
global starts, uneven partitions and 1024-block tails. Separate range fixtures
check exact coverage around 1024/8192. Bounded n12 permutation matrices compare
both Uiua generators and an independent factoradic reference, without enumerating
12! ranks. Native blocked-worker gates observe exactly the expected added worker
threads, relative to the runtime baseline, for caps 1..8 and tiny adaptive cases.
The unchanged 27-LOC candidate's 64 complete outputs, five work traces, eleven
worker gates and eight retained-v2 controls are admitted from their already
passing records in an incomplete earlier attempt. The new report records the
exact original report, archived validator and log digests; only these passing
records are reused, and the earlier failing block source is excluded. The
corrected block candidate receives fresh complete qualification. Failed attempts
and their source/validator snapshots remain in separate result directories.

N9 pilots run only for planning, are marked `never_chart_eligible`, and are not
performance samples. No full n12 run, source selection, suite migration, Git
action or chart update is part of this preparation. Root must review the sealed
manifest before further benchmarking.

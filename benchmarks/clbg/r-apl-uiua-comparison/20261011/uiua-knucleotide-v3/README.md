# Additive Uiua k-nucleotide record-parser repair

These three candidates repair semicolon comment lines at EOF in the original
compact, bounded optimized, and native-group optimized Uiua applications.
The old cleaner requires a newline after every comment. For the exact input
`>THREE\nACGT\n;ACGT` with no final newline, it incorrectly admits the comment
into the sequence. Each candidate changes source line 3 so the ordinary comment
regex accepts newline or EOF. Line 4 now captures complete non-header lines and
stops only at a line-leading `>`, so `>` inside an ignored comment cannot truncate
the selected record. CR/LF cleanup, case conversion, all seven growing maps,
window loops, and output stay unchanged.

The sources are `sources/compact.ua` (19 LOC), `sources/bounded.ua` (27 LOC), and
`sources/native-group.ua` (29 LOC). Their predecessors, source hashes, runtime
hashes, frozen manifests, and timing evidence remain preserved. The compact
variant generates the whole-record windows; bounded variants process at most
65,536 starts plus `width-1` overlap per chunk. Native-group counting still
classifies every window and folds each distinct full key with its grouped count.

Reproduce preparation and small validation from this directory:

```sh
python3 prepare.py
python3 validate.py
python3 prepare_manifest.py
```

Validation holds `/tmp/mech-clbg-timing.lock` throughout all reference and runtime
work. It first confirms both exact original parser failures on all three original
sources, retaining six defect reproductions. Each repaired profile then runs the original fourteen fixtures plus
nine EOF/comment/header fixtures: the exact counterexample, an empty EOF comment,
a CRLF/case EOF comment containing a query, consecutive empty comments, sequence
without its final newline, an unterminated target header, embedded header markers
in comments, CRLF comments containing target/header markers, and an empty THREE
record followed by a short header. Existing cases
cover canonical small input, missing/empty/short records, lexical ties, comments,
arbitrary DNA, every six-base key, and six lengths around the chunk boundaries.

Separate diagnostic sources serialize every complete final map, comparing all
keys and counts at widths `1,2,3,4,6,12,18`. Both bounded variants additionally
trace the exact width/start/count partition; native-group traces grouped weights
and distinct keys. Every diagnostic retains the application's counting logic.
Diagnostic output is never part of the timed source. The report contains sixty-nine
output checks and sixty-nine complete-map/work checks across twenty-three fixtures per
profile. Pinned native source proofs for windows, class/group operations, map
equality, collision probing, and growth are retained from v2.

This package records no full workload timing. The canonical THREE record has
125,000,000 bases and requires all 874,999,961 overlapping windows. Every row
remains ineligible for a chart until explicit selection through a new frozen
audit and qualifying fresh full candidate/control measurements.

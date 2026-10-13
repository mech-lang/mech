This is a source-only, unqueued work-review draft. It changes no sealed Nim
source, proposal, binary, report or reference. The v4 native preparation remains
a separate prerequisite. Output/primitive proof alone cannot approve a full
study; the additional source-work and worker/allocation gates below remain.

Use audit copies of the exact selected compact/optimized sources, preserving
all production arithmetic, comparisons, carry steps, joins and stdout order.
Insertions must have unique source anchors and retain source/copy/command hashes.
Each actual audit process and its trace callback uses the existing owned
sampler under the common mutex. Trace failure rejects the record before ledger
admission. Every observer is unscored and makes no timing or GC-pressure claim.

Pi: open a trace before the candidate loop. Record q/r/t/k/printed at its start,
then both actual numerator and quotient values immediately after the second
division and before the equality test. Pure BigInts use their public decimal
conversion; the GMP copy uses ordinary mpz decimal conversion through the
retained public binding without modifying operands. Reconcile each candidate
against an independent integer recurrence, including rejected candidates,
both quotient values and old-q production updates. N0/1/9/10/11/27/97 include
empty output and complete/partial formatting lines. The trace is separate from
complete canonical stdout and does not claim a guard skipped either division.

The actual v4 primitive probe stopped because it expected truncating signed
division from pure BigInts. Pinned BigInts1.1.0 explicitly implements floor
division, so(-17)div5 is-4; GMP support intentionally uses mpz_tdiv_q, yielding
-3 on that separate negative primitive example. The application sources stay
unchanged. Both application extractions have nonnegative numerators: initially
L=3*q+r is3; a production update makes L'=10*(L-digit*t)>=0; a consumption
update makes L'=(2*k+1)*L+(k-1)*q>=0 for k>=1 and q>0. Upper numerator is L+q.
Thus floor/truncation coincide for both actual quotients. Trace reconciliation
must verify these invariants on every candidate and retain the failed primitive
record; changing a primitive expectation must never relabel its failed run or
repeat the154 already passed production records.

Fann: compact opens one trace; optimized workers each open a separate file
identified by their exact [first,last) interval. Record the actual permutation
before the flip loop, each resulting prefix reversal, final flip count and
global checksum sign. Capture the initialized factoradic/carry digits and every
worker's announced range. Reconcile all ranks for N1/3/7/8 against an independent
canonical enumeration, including odd worker starts, 1024 edges and uneven
intervals. Prove intervals are disjoint and cover factorial(N), all permutations
occur once, each flip is performed and every sign uses the global rank. Any
separate N12 edge-only callable probe must be labelled partial and unscored;
it cannot be counted as canonical N12 output or a full measurement.

Trees: observe each newly completed root immediately before the unchanged
check call. Walk that alive root's actual Node graph using temporary integer
addresses, reject null/repeated addresses within a root, and reconcile exactly
2^(depth+1)-1 distinct allocated nodes including 2^depth leaves. Preserve no
temporary Node references after the observer returns; address reuse across
different roots is allowed. Record the stretch, every iteration root and the
same long-lived root address before/after all churn, plus each worker's exact
iteration interval. N0/4/7/9 cover the minimum depth and uneven depth schedule.
Audit-copy address checks do not establish allocation in the unobserved scored
binary: retain the actual generated C and final LLVM/ARM creator/leaf paths,
and separately verify real allocation calls and two distinct child creations.

Spectral norm: capture each completed multiply's input/output arrays and
transpose flag, plus actual (row,column) denominator calls. Independently
reconcile all forty dense N-by-N applications in their twenty A-transpose-A
products, ten alternating iterations and both final dot reductions. N1/2/3/9
cover minimum/odd/uneven worker shapes; there must be exactly40*N*N coefficient
uses, disjoint row intervals and joins before each dependent multiply.

Nbody: capture all five bodies after momentum offset and each actual advance,
plus pair and position visit counters. Reconcile every state component against
an independent binary64 implementation at N0/1/2/17, ten pair interactions and
five position updates per step, and both actual energy calculations. Complete
stdout still uses the canonical energy tolerance; the audit cannot claim
bitwise equivalence for an independently grouped floating reference.

FASTA: record the actual binary64 cumulative probability tables, every updated
LCG seed/draw/selected symbol for both random records and the ALU index/character
stream. Reconcile all2*N repeat characters and8*N random draws, with the same
seed spanning IUB and Homo records, using N0/1/29/30/31/287. Complete canonical
output independently proves headers,60-column wrapping and final line tails.

Mandelbrot: workers record each valid lane's evolving zr/zi/tr/ti and each
five-step block stop decision in the optimized form; compact records its
per-pixel escape steps. Independently reconcile all requested coordinates,
valid tail lanes, group stop behavior, final bits and complete PBM padding at
N1/3/7/8/9/17. Capture real row intervals and joined parent output. Uniform
extra lane work is reported honestly; it is not relabelled as per-step escape.

K-nucleotide retains the actual all-seven-map/overlapping-window reconciliation
already drafted for v4. Reverse complement and regex need trace copies that
record exact selected input bytes, each completed transformed record or regex
stage, all nine independent match counts and ordered replacement intermediates.
CR/LF/CRLF, comments, header EOF, ties, NUL-safe complete output and exact THREE
space/tab/end boundaries stay tied to their independent byte oracles.

For all optimized parallel forms, ordinary thread identity observations must
show at most eight simultaneous compute workers and all joins before parent
reads. The already proposed ORC seven-map/parent-reset ASAN proof remains;
the all-seven-map audit retains both widths1/2 and every overlapping window.
Full manifest sealing additionally requires the canonical LOC/helper audit,
all required work cases, exact production build/source/tool/runtime identity,
and independent review. No native process, test, hash sweep or score has run
from this draft.

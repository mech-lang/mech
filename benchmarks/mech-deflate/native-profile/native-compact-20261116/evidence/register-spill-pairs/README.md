# AArch64 register spill pairing

The diagnostic compiler pairs consecutive integer moves to or from adjacent
private spill slots. The offset must fit the ARM64 pair instruction and reload
destinations must be distinct. Debug and stack map observers keep ordinary
spill emission. The experiment translates the two pinned block profiles after
checking every instruction and branch target against both prelayout objects.

The resulting encoder text is **71,864 bytes**, down from **71,960 bytes**.
The paired matcher executes **246,428,466** instructions on the 1 MiB random
fixture, down from **254,842,426**. The complete build medians are 4.52776 and
4.52721 seconds with fixed compilers, runtimes, and adapters prebuilt.

Both full timing orders record **81/81 Roc and 61/81 libdeflate** for both Mech
workers, with all individual peer wins preserved. The paired image is rejected
under the per-case policy. The estimated slowdowns are 0.576% on
`prefix-natural-65535` and 0.599% on `prefix-natural-262144`, with both lower
interval bounds above 0.5%. The complete policy replay includes eight other
cases requiring more evidence. The archived implementation stays separate
from the pinned recipe.

The direct allocator timers put unmapping at 2.0 microseconds for the 4 KiB
random case and 3.4 for the 8 KiB natural case. PC sample proportions therefore
provide location evidence. They substantially overstate direct unmap time on
these inputs. Dynamic counts for the 8 KiB natural case put 7,752,576
instructions in the composed path scorer and 3,496,564 in match finding.

## Function placement diagnostics

Restoring all 40 native function addresses by appending unreachable padding
brings the paired leaf back to the control's 71,960-byte encoder text size.
A second diagnostic adds 32 bytes of padding and preserves every native
function's address modulo 64. That object emits **71,896 bytes**.
Both formerly regressing cases are within the 0.5% resolution in the
nine-case, two-order replay. Their estimated changes are **+0.148%** and
**+0.341%**. `random-64` remains unresolved in that screen. The 71,896-byte
binary is a placement diagnostic with its own identities and focused timing
receipt. A compiler-emitted version requires complete qualification.

## Reproduce the emitted object

Apply `compiler-as-run.patch.gz` at `cf0984ac50e7a1d7e4da47380f42e582d15f1fb0`
in an isolated checkout. The patch is a gzipped Git diff. Run the normal
`rebuild.py` using `--compiler-opt-level 3 --variant fresh-owner-counts-cold-tails`
to prepare the baseline and compiler. Pass that result directory and a fresh
output directory to `replay_paired_spills.py`. The replay pins the bytecode,
encoder object, and complete metadata. The timed executable identities and
their link commands are in the receipts under `control` and `paired`.

[Artifact identities and summary](report.json) · [Complete policy replay](full-runtime-policy.json)

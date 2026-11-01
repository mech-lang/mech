"""Render the rejected scoped-context experiment from verified observations only."""
from pathlib import Path
import json,sys
p=Path(sys.argv[1]);d=json.loads(p.read_text());out=Path(sys.argv[2]);r=p.parent.parent
s=d['primary'];c=d['changes_percent'];v=d['validation'];i=d['implementation'];scope=d['diagnostic_mechanism_counts']
def cpu(label,key):return d['cpu'][label][key]['median']
def alloc(label,key):return d['allocation'][label][key]['median']
lines=['# R74: scoped current-context experiment — rejected','',
 '**Decision:** reject this runtime candidate. The lifetime-bound context is correct in the executed controls, but elapsed time and instructions regress. R75 tests only the two newly outlined boundaries found in the actual host code. The accepted runtime remains R70 until a successor passes qualification.','',
 '| Metric | Roc | Accepted baseline | R74 candidate | Change |',
 '|---|---:|---:|---:|---:|',
 f"| Complete-call elapsed | {s['reference_seconds']*1000:.6f} ms | {s['medians']['before']['seconds']*1000:.7f} ms ({s['medians']['before']['multiple_of_reference']:.6f}×) | {s['medians']['after']['seconds']*1000:.7f} ms ({s['medians']['after']['multiple_of_reference']:.6f}×) | {c['primary_seconds']:+.6f}% |"]
for key,title in [('instructions','Retired instructions'),('cycles','CPU cycles')]:
 b=cpu('before',key);a=cpu('after',key);ref=cpu('reference',key)
 lines.append(f'| {title}, separate cohort | {ref:,.1f} | {b:,.1f} ({b/ref:.6f}×) | {a:,.1f} ({a/ref:.6f}×) | {c[key]:+.6f}% |')
for key,title in [('allocation_requests','Allocation requests'),('requested_bytes','Requested bytes'),('reallocations','Reallocations, included above')]:
 lines.append(f"| {title}, separate cohort | {alloc('reference',key):,.0f} | {alloc('before',key):,.0f} | {alloc('after',key):,.0f} | {alloc('after',key)-alloc('before',key):+,.0f} |")
comparison=s['comparisons']['after_over_before']
lines+=['',f"The primary measurement has four independently prepared pairs, {s['primary_observations']:,} timed calls and {s['all_observations']:,} total calls. Only {comparison['improving_groups']}/4 groups and {comparison['improving_blocks']}/64 blocks improve. Blocks within a prepared pair are descriptive, not independent experiments. CPU counters, allocations, full stack censuses, samples and instruction windows are separate serialized cohorts. No percentages from independent cohorts are multiplied.",'',
 '## Mechanism and design','',
 'The callback borrows the existing thread-local context with a lifetime that cannot escape its scope or cross threads. No account, cap or reservation authority is cached. Each debit uses the original live account/limit checks and original failure order. A borrowed reference is carried by Invocation; no RefCell borrow is held across callbacks. Ordinary entry behavior and non-native builds remain supported.','',
 'Four separate diagnostic calls agree on 39,030 shared debit attempts: 12,450 routed through the borrowed context and 26,580 ordinary attempts. There are two scope entries. These counts do not claim that all ordinary attempts perform TLS lookup. Diagnostic counters are excluded from performance measurements.','',
 'The actual linked ARM64 audit identifies two added call boundaries. The ordinary debit is 158 own static instructions in the accepted baseline; R74 has 104 ordinary and 64 scoped own instructions, each calling the shared debit helper (130 own). The control visitor also calls the newly outlined error adapter (122 own) instead of containing that body. Static counts are not executed instruction counts or a complete regression attribution. Exact symbols/instructions are retained in host-assembly and control-assembly.','',
 'R75 changes only the inlining annotations of that shared debit helper and control error adapter. It is compared with both this unfixed binary and the accepted baseline, so merely recovering the R74 regression cannot count as a speedup.','',
 '## Revisions and invariants','',
 f"- Accepted checkpoint: `{i['baseline_checkpoint']}`; actual accepted release source: `{i['baseline_build_source']}`.",
 f"- R74 actual release source: `{i['candidate_build_source']}`. Engine, compute, stdlib, runtime and GPU are coherently rebuilt; Core/math/syntax and other project dependencies remain hash-frozen.",
 '- Source commits: `955c7d8c6` (borrowed context and five controls), `180abd6b8` (control admission integration), `e42c2ce1d` (CI selection for the five tests). These remain experimental pending R75; this report does not claim a production restoration.',
 '- IR/work/ABI/bytecode remain 132/95/37/14. Fresh Object, descriptor and metadata are byte-identical to the accepted baseline. The pin and library manifests retain exact hashes and feature/profile flags.',
 '- Complete eight-byte `abcdefgh`, level12; bytes `[75,76,74,78,73,77,75,207,0,0]`; independent inflate; output capacity13 with clear/reuse; logical work60260219686 and comparison work47612; final managed bytes/nodes0. Source, tables, search limits, scratch capacities, allocator, input aliases and evaluate_into timer are unchanged. No larger timing is performed.','',
 '## Validation and disposition','',
 f"Bounded rejected-package checks: **{len(v['checks'])}**, status **{v['status']}**. Fresh candidate Rust compilation executes Budget113 including five new controls; the parent executes Budget108 after full staged-source and dependency proofs for its cached compiler output. Planner22, projection12 and shape6 are additional overlapping selections. The preliminary Engine-only API witness also executes Budget113/scoped5/planner22; it is identified separately from integrated-source results.",
 '',
 'The five new cases cover ordinary/scoped boundary equivalence, live account/caps and caller/foreign-account precedence, observer high-water with nesting, worker denial atomicity/retry/settlement, and unwind restoration/recovery. Actual cross-crate Rust consumers succeed with cleanup and reject context escape and cross-thread use at compile time. Non-native Engine compilation passes warnings-as-errors. No fresh Core unit execution is claimed.','']
for row in v['skipped_coverage']:lines.append(f"- {row['coverage']}: {row['disposition']}")
lines+=['','Introduced setup failure: the initial generated driver had a Python whitespace syntax error before integration build or timing; it was corrected before continuation. No production compiler or semantic failure is hidden. The failed setup and its disposition are retained in prior-failures.json.','',
 '## Reproduction','',
 '`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies every archived file version and reconstructs the scorecard, allocation ledger, sample summary, instruction windows, validation and diagnostic counts without executing a compiler or compressor. Raw calls, complete stack censuses, build/test commands, sources, assembly and SHA-256 manifests are committed with the packet. `reconstruction.json` records the observed replay result.','']
out.write_text('\n'.join(lines))
print(out)

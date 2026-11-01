"""Render the verified three-way experiment and exact rejected disposition."""
from pathlib import Path
import json,sys
p=Path(sys.argv[1]);d=json.loads(p.read_text());out=Path(sys.argv[2]);s=d['primary'];v=d['validation'];i=d['implementation'];c=d['changes_percent'];decision=d['decision']
lines=['# R75: scoped budget call boundaries — rejected','',
 '**Decision:** reject the complete scoped-context candidate. Two targeted inlining annotations recover most of R74’s regression, but the result still loses against the accepted R70 runtime. Retain the raw evidence and restore the accepted production source.','',
 '## Matched complete-call timing','',
 f"Fresh Roc reference: **{s['reference_seconds']*1000:.6f} ms**. All variants run the complete pinned eight-byte level12 computation.",'',
 '| Runtime | Median elapsed | Multiple of Roc |', '|---|---:|---:|']
for label,title in [('before','Accepted R70'),('unfixed','Unfixed R74'),('after','R75: two targeted annotations')]:
 row=s['medians'][label];lines.append(f"| {title} | {row['seconds']*1000:.7f} ms | **{row['multiple_of_reference']:.6f}×** |")
lines+=['','| Comparison | Pooled change | Improving prepared groups | Improving blocks |','|---|---:|---:|---:|']
for key,title in [('after_over_before','R75 / accepted'),('after_over_unfixed','R75 / unfixed'),('unfixed_over_before','Unfixed / accepted')]:
 row=s['comparisons'][key];lines.append(f"| {title} | {100*(row['pooled_ratio']-1):+.6f}% | {row['improving_groups']}/4 | {row['improving_blocks']}/64 |")
lines+=['',f"Four independently prepared executable triples, all six block-order permutations, {s['primary_observations']:,} timed calls ({s['observations_per_variant']:,} per variant), {s['all_observations']:,} total. All three executables are ready before timing starts. Blocks within a triple are descriptive. The fourth prepared group favors R75 versus accepted; that contrary result is preserved. Earlier timing reference clocks are not substituted into this cohort.",'',
 '## Separate instruction, cycle and allocation cohorts','',
 '| Metric | Roc | Accepted R70 | R75 | Change |','|---|---:|---:|---:|---:|']
for section,key,title in [('cpu','instructions','Retired instructions'),('cpu','cycles','CPU cycles'),('allocation','allocation_requests','Allocation requests'),('allocation','requested_bytes','Requested bytes'),('allocation','reallocations','Reallocations, already in requests')]:
 ref=d[section]['reference'][key]['median'];b=d[section]['before'][key]['median'];a=d[section]['after'][key]['median'];change=c[key]
 def val(x):return f'{x:,.1f}' if x%1 else f'{x:,.0f}'
 ratio=lambda x:f' ({x/ref:.6f}×)' if ref else ''
 lines.append(f"| {title} | {val(ref)} | {val(b)}{ratio(b)} | {val(a)}{ratio(a)} | {change:+.6f}% |" if change is not None else f"| {title} | {val(ref)} | {val(b)} | {val(a)} | unchanged |")
lines+=['',
 'These CPU/heap cohorts compare accepted R70 and R75 only. R74 has primary triple observations and its separate R74 hardware cohorts; no matched three-way hardware claim is made. Full allocation censuses reconcile with heap counters. Requested capacity is not copied or live bytes. Post-call OS memory includes retained compiler/catalog/image storage and is not peak RSS. Diagnostic CPU elapsed, samples and coarse instruction windows are separately retained, including adverse results. No sample percentage is converted into a function instruction count.','',
 '## Change and observed mechanism','',
 'R74 borrows the existing work context for the synchronous entry and routes control debits through that reference. Each debit still observes live account/caps, worker ledger and observers; no RefCell borrow is held across callbacks, and no authority is cached. Four diagnostic calls consistently count 39,030 shared debit attempts: 12,450 scoped and 26,580 ordinary, with two borrowed scope entries. Those counts are diagnostic, outside performance cohorts.','',
 'R74’s actual host code introduced separate calls for the shared debit validator and the control error adapter. R75 adds exactly two `#[inline(always)]` annotations to those existing functions. It does not contain the rejected R71/R72 arithmetic factoring or R73 public-boundary annotations. The actual ARM64 audits show both extra helper calls disappearing, while the selected control visitor grows from472 to544 own static instructions against accepted R70. The audits are retained in host-assembly and control-assembly; static instruction counts are not dynamic attribution.','',
 'Removing a TLS lookup alone did not make this control path cheaper overall. The matched triple distinguishes recovery from R74 from improvement over the accepted implementation. This implementation remains adverse on both primary elapsed and whole-call retired instructions.','',
 '## Provenance, restoration and invariants','',
 f"- Accepted checkpoint `{i['baseline_checkpoint']}`; actual accepted release source `{i['baseline_build_source']}`.",
 '- Unfixed scoped release source `e42c2ce1daa4d7a22ff12eda5ecbcce7582a83aa`.',
 f"- R75 release source `{i['candidate_build_source']}`; change `33caedc032e26f67ac016cdc23f3f4152592d921` is exactly the two annotations versus R74.",
 f"- Disposition: {decision['source_disposition']}",
 '- Engine, compute, stdlib, runtime and GPU rebuild coherently under the retained exact flags. Core/math/syntax and external inputs are hash-frozen. Exact revisions, dependency hashes, source diffs and all feature/profile configurations are archived.',
 '- IR/work/ABI/bytecode remain132/95/37/14. Fresh generated Object, descriptor and metadata are byte-identical across accepted/candidate. No scalar ceiling or resource allowance is increased.',
 '- Complete `abcdefgh`, level12; exact raw bytes `[75,76,74,78,73,77,75,207,0,0]`, independent inflate, outputcapacity13 clear/reuse, logical work60260219686, comparison work47612, final managed bytes/nodes0. Source algorithm, tables, search limits, scratch capacities, allocation sites, allocator, aliases and evaluate_into timer are unchanged. No larger timing.','',
 '## Validation and failed attempts','',
 f"**{len(v['checks'])} bounded rejected-package checks**, status **{v['status']}**. Candidate Rust unit compilation is fresh; Budget113 includes the new five context contracts. Parent Budget108 executes after full-source/library-verified reuse of its unchanged unit binary. Planner22, projection12 and shape6 are overlapping additional selections. Actual Rust consumer checks verify success/cleanup, context escape rejection and cross-thread rejection. Non-native Engine compilation passes warnings-as-errors. Four diagnostic compression calls also verify exact output and zero final managed memory.",'',
 'The candidate unit compilation succeeded before saving its receipt was denied by the initial execution permissions. No tests had run. The original failure is archived; the complete staged Engine tree and linked libraries were verified unchanged, the existing archive was committed with authorized permissions, and all selections then executed on that same freshly compiled binary. There was no unnecessary recompilation and no compiler test failure.','']
for row in v['skipped_coverage']:lines.append(f"- {row['coverage']}: {row['disposition']}")
lines+=['',
 '## Reproduction','',
 '`python3 reconstruct.py /absolute/path/to/new-replay-directory` verifies every archived file version and replays primary triples, hardware counters, full censuses, samples, validation, instruction windows and event counts without a compiler or compressor. `reconstruction.json` records observed archive/file counts and byte-identical derived outputs. Original commands, stdout/stderr, source pins and bounded disassembly are retained with the raw archives.','']
out.write_text('\n'.join(lines));print(out)

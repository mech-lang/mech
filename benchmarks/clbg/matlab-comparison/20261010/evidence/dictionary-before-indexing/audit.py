"""Pin the reviewed MATLAB work, source LOC, runtime and allocation evidence."""
import hashlib,json,re
from pathlib import Path
from prepare import BENCHES,sha
D=Path(__file__).resolve().parent
REASONS={
 'spectralnorm':'Four separate coefficient/multiply/transpose/composition functions. Forty complete N-by-N products recompute every coefficient; bounded broadcasting is ordinary array evaluation, with no cached coefficient matrix.',
 'mandelbrot':'Every pixel uses the ordinary recurrence and escape test for at most fifty iterations. No analytic interior, periodicity, or symmetry shortcut; complete packed PBM output is checked.',
 'regexredux':'Nine separate case-insensitive non-overlapping regex searches, then five ordered case-sensitive regex substitutions. Whole input, cleaned length and final length are validated.',
 'revcomp':'Read every complete FASTA record, preserve headers, uppercase and reverse every nucleotide with the complete complement map, and emit all output in sixty-column lines.',
 'fasta':'Every random base consumes a fresh sequential LCG update and direct double division by 139968. Cumulative probabilities remain unscaled floating point. Ordinary discretize performs interval selection; no cached LCG cycle, lookup table, or integer probability scaling.',
 'nbody':'All fifty million sequential timesteps, ten symmetric pair interactions and all five position updates per step, with both energies. Exact canonical constants, timestep and momentum adjustment; no precomputed final state.',
 'pidigits':'Sequential unbounded linear-fractional-transform spigot. Both extract(3) and extract(4) are computed on every extraction attempt. Java BigInteger only supplies standard arbitrary-precision arithmetic; MATLAB controls every transformation and digit.',
 'fannkuchredux':'All 479001600 permutations in the canonical rotation order, complete prefix flips, alternating checksum and maximum. Scalar swaps or ordinary array reversal change implementation only.',
 'knucleotide':'Build all seven complete overlapping word tables for lengths 1,2,3,4,6,12,18, using default growing MATLAB dictionaries. Each valid window performs a hash lookup and counter update in every reading frame. Every completed table asserts its total equals the full number of windows. No indexed-count histogram, sort/reduce substitute, period shortcut, or query-only scan.',
 'binarytrees':'Every interior and leaf is a newly allocated two-slot cell array. Complete construction precedes traversal; temporaries and stretch tree are released at their prescribed lifetimes, while the long-lived root survives all temporary work. Constructor unrolling preserves every node. Untimed pointer-identity probes confirm independent headers and backing storage at three arbitrary depths; no flat array, shared leaf, custom pool or benchmark-native executor.'
}
FULL_WORK={
 'spectralnorm':dict(n=5500,matrix_products=40,coefficient_evaluations=1210000000),
 'mandelbrot':dict(n=16000,pixels=256000000,max_iterations=50),
 'regexredux':dict(input_bytes=50833411,count_patterns=9,ordered_substitutions=5),
 'revcomp':dict(input_bytes=254166745,records=3,nucleotides=250000000),
 'fasta':dict(n=25000000,nucleotides=250000000,lcg_updates=200000000),
 'nbody':dict(steps=50000000,pair_interactions=500000000,position_updates=250000000),
 'pidigits':dict(digits=10000,arithmetic='Unbounded Java BigInteger LFT, extract(3) and extract(4)'),
 'fannkuchredux':dict(n=12,permutations=479001600),
 'knucleotide':dict(three_sequence_length=125000000,window_lengths=[1,2,3,4,6,12,18],hash_table_window_updates=874999961),
 'binarytrees':dict(max_depth=21,node_slots=2,total_nodes=613766494,stretch_depth=22,long_lived_depth=21)
}
def main():
 built=json.loads((D/'results/builds.json').read_text());small=json.loads((D/'results/small-checks.json').read_text())
 assert small['status']=='passed' and len(small['programs'])==20
 diagnostics=json.loads((D/'evidence/syntax-check.json').read_text())
 assert set(diagnostics)=={r['id'] for r in built['programs']}
 for messages in diagnostics.values():
  if isinstance(messages,dict):messages=[messages]
  assert all('Consider preallocating for speed' in m['message'] for m in messages),messages
 allocation=json.loads((D/'evidence/allocation-identity.json').read_text())
 expected=[[2**(depth+1)-1]*3+[2**depth,1] for depth in [4,8,12]]
 assert allocation==dict(compact=expected,optimized=expected)
 evidence_files=['probe.py','build-probe.py','evidence/probe-build.json','evidence/work_probe.m','evidence/allocation_identity.cpp','evidence/allocation_identity.mexmaca64','evidence/allocation-identity.json','evidence/syntax-check.json','results/java-install.json','matlab-stdin.sh']
 evidence={f:sha(D/f) for f in evidence_files}
 probe=(D/'evidence/work_probe.m').read_text()
 rows=[]
 for r in built['programs']:
  source=Path(r['source']);text=source.read_text();assert sha(source)==r['source_sha256']
  assert small['programs'][r['key']]['source_sha256']==r['source_sha256']
  if r['benchmark']=='binarytrees':
   functions=text[text.index('function t = make_tree'):]
   functions=re.sub(r'\bmake_tree\b','make_'+r['profile'],functions)
   functions=re.sub(r'\bcheck\b','walk_'+r['profile'],functions)
   assert functions.strip() in probe
  a=r['loc_audit'];rows.append(dict(key=r['key'],id=r['id'],source=r['source'],sha256=r['source_sha256'],loc=a['normalized_loc'],physical_loc=a['physical_loc'],expansions=a['expansions'],admitted=True,reason=REASONS[r['benchmark']]))
 requirements={b:dict(strict=True,status='reviewed',reason=REASONS[b],full_work=FULL_WORK[b],native_benchmark_kernel=False,requirements_url='https://benchmarksgame-team.pages.debian.net/benchmarksgame/description/'+b+'.html') for b in BENCHES}
 report=dict(status='reviewed',builds_sha256=sha(D/'results/builds.json'),small_checks_sha256=sha(D/'results/small-checks.json'),requirements=requirements,rows=rows,controls=built['controls'],evidence=evidence,
  allocation_evidence=dict(status='passed',depths=[4,8,12],inventory_columns=['total_cells','unique_headers','unique_backing_storage','leaves','all_nodes_two_slots'],results=allocation,finding='Both exact source constructors keep independent two-slot allocations for every interior and leaf. The native diagnostic only inventories arbitrary supplied nested cells; it neither constructs trees nor executes a benchmark and is excluded from timing.'),
  syntax_check=dict(status='passed',diagnostics=diagnostics,note='One optional preallocation advisory in the compact pi source; no syntax or API errors. All sources execute and match the small complete reference outputs.'),
  policy=dict(loc='Nonblank/noncomment physical source lines plus independent packed statements. Algorithm code, function declarations, stream opens and corrections count; standard library and stream-only runtime launcher excluded.',runtime=built['runtime_policy'],timing=built['timing_boundary'],upstream='Twenty local source implementations of the official algorithms. No published MATLAB performance ranking or timing is reused.',output='Complete small and full stdout reference validation; N-body energies within 1e-8. The first of three full samples is also full-output validation; no warm-run filtering.',input='Full canonical files redirected to descriptor 3 by a stream-only shell launcher before MATLAB disconnects descriptor 0.',tree_memory='Default MATLAB cell allocation/reference counting. No benchmark MEX, custom memory pool or changed garbage-collection flags. Generic untimed diagnostic compiled with the existing Apple Command Line Tools.'))
 (D/'source-audit.json').write_text(json.dumps(report,indent=2)+'\n');print('Reviewed twenty MATLAB sources, ten controls, full-window invariants and six allocation identity inventories.')
if __name__=='__main__':main()

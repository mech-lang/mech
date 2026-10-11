"""Pin source, runtime and library snapshots; expand packed statements for LOC."""
import hashlib,json,re,subprocess
from pathlib import Path
D=Path(__file__).resolve().parent
BENCHES=['spectralnorm','regexredux','pidigits','mandelbrot','revcomp','fasta','binarytrees','knucleotide','fannkuchredux']
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
# Explicit statement audit for packed imperative bodies. Structural separators
# in cases, tuples, lists, parameters and arguments never create extra lines.
EXTRAS={
 'spectralnorm-faithful.bqn':{4:(2,'Av binding, vector binding, return'),5:(2,'Atv binding, vector binding, return')},
 'fannkuchredux.bqn':{5:(2,'Flip binding, size binding, return'),23:(1,'increment flips, then flip the permutation')},
 'fannkuchredux.growler.k':{12:(2,'flip binding, size binding, return'),13:(2,'flipcount binding, flips assignment, return')},
 'knucleotide-faithful.bqn':{16:(1,'hash-table update, then fold accumulator'),24:(2,'Fixed binding, s binding, return')},
 'knucleotide-faithful.growler.k':{9:(2,'inc binding, hash-table update, return'),34:(3,'fixed binding, s binding, s assignment, return')},
 'pidigits.bqn':{11:(4,'Extract binding, multiply, add, divide, return')},
 'regexredux.bqn':{10:(2,'Substitute binding, sequence assignment, return')},
 'regexredux.growler.k':{10:(1,'substitute binding plus sequence assignment')},
}
REASONS={
 'spectralnorm':'Four separate coefficient, multiplication, transposed multiplication, and composed multiplication functions; coefficients evaluated during all 40 products, without precomputed matrices.',
 'regexredux':'Read full input; header/newline removal; nine independent regex traversals; five ordered global substitutions. PCRE2 JIT is ordinary library regex compilation, not a combined benchmark kernel.',
 'pidigits':'Sequential, unbounded LFT spigot in the language; generic GMP operations; both extract(3) and extract(4) checked before each emitted digit, with the published q>r guard.',
 'mandelbrot':'Each pixel has up to fifty escape iterations; vector form performs all fifty and retains an AND of membership; output contains the full PBM, including padding.',
 'revcomp':'Read stdin via growing line/stream buffers, preserve headers, complement and reverse all complete sequences, then wrap at sixty columns. No file-size preallocation or mmap.',
 'fasta':'Fixed ALU repetition plus every one of 200,000,000 random-sequence LCG updates; divide by 139968.0 and compare original cumulative floating-point probabilities; no random-period caching or integer cutoff substitution.',
 'binarytrees':'Fresh two-slot boxed pairs for all nodes, including leaves, in the language; every tree is traversed and reclaimed. Generic process map applies the same depth function in at most eight interpreter workers. No shared leaf or closed-form check.',
 'knucleotide':'Read lines and extract THREE; all seven widths (1,2,3,4,6,12,18); each reading frame divided into eight ranges, each growing its own real hash table from default capacity. Every one of 874,999,961 overlapping keys is looked up and updated in BQN or K; only after updates are requested counts returned and combined. BQN uses built-in HashMap; K uses the unmodified khash library through one-key get/set bindings, never a native key-counting loop.',
 'fannkuchredux':'All 479,001,600 permutations in canonical rotation order; independent ranks initialized with factoradic rotations; complete prefix reversal and alternating checksum for every permutation, in eight language processes. Generic fork support contains no permutation or flip logic.',
}
old=json.loads((D/'source-audit.json').read_text())
rows=[r for r in old['rows'] if r.get('algorithm',{}).get('strict') is False]
for bench in BENCHES:
 for lang,suffix in [('bqn','bqn'),('k','growler.k')]:
  stem=bench+'-faithful' if bench in ('spectralnorm','knucleotide') else bench
  p=D/'sources'/f'{stem}.{suffix}'
  lines=p.read_text().splitlines();physical=sum(bool(l.strip()) and not l.lstrip().startswith(('#',) if lang=='bqn' else ('/','#!')) for l in lines)
  expansions=[dict(line=i,extra_lines=n,reason=why) for i,(n,why) in EXTRAS.get(p.name,{}).items()]
  rows.append(dict(benchmark=bench,language=lang,file=p.name,sha256=sha(p),physical_loc=physical,normalized_loc=physical+sum(e['extra_lines'] for e in expansions),expansions=expansions,upstream=None,
     algorithm=dict(strict=True,status='required-work-reviewed',reason=REASONS[bench],native_benchmark_kernel=False),
     stdlib_files=['stdlib/standard.'+('bqn' if lang=='bqn' else 'k'), 'stdlib/'+('libarray-standard.dylib' if lang=='bqn' else 'libgrowler-standard.dylib')] if bench in ('knucleotide','fannkuchredux','binarytrees','regexredux','pidigits') else []))
report=dict(status='faithful-sources-reviewed',policy='Nonblank, noncomment physical LOC plus separately audited packed statements, inline/chained assignments and imperative function headers; no compressed-size metric. Structural separators are unchanged. Runtime/library bindings are pinned and shown separately; all benchmark algorithms remain in the listed source.',rows=rows,
 runtimes={l:dict(path=str(D/'binaries'/exe),sha256=sha(D/'binaries'/exe),commit=c) for l,exe,c in [('bqn','bqn-ffi','c893d3e7828899a50150df03ad33c9052fa51d3c'),('k','growler-ffi','48067b30de7c63f2d457727245ba34f7a714d1ce')]},
 libraries={str(p.relative_to(D)):sha(p) for p in (D/'stdlib').glob('*') if p.name in ('native.c','khash.h','standard.bqn','standard.k','libarray-standard.dylib','libgrowler-standard.dylib')},
 runtime_support='Generic GMP arithmetic, PCRE2 compile/count/replace, khash one-key get/set, and a process map that executes language callbacks. No input-specific constants, parsing, encoding, or benchmark computation in native bindings. Process map is reused unchanged by three benchmarks in both languages.')
(D/'source-audit.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps([dict(benchmark=r['benchmark'],language=r['language'],loc=r['normalized_loc']) for r in rows if r['algorithm']['strict']],indent=2))

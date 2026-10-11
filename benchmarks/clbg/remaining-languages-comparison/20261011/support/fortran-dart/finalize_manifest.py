#!/usr/bin/env python3
"""Finalize prepared candidate provenance and strict-work audit without timings."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
S=Path(__file__).resolve().parent
D=S.parents[1]
sys.path.insert(0,str(D))
from metrics import normalize
path=D/'fortran_dart_manifest.json'
m=json.loads(path.read_text())
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
upstream={p['id']:p for p in m['upstream']}
tree=json.loads((S/'tree-work-audit.json').read_text())
tree_records={p['id']:p for p in tree['tests']}
programs=[]
for p in m['programs']:
    assert p['build_passed'] and p['small_validation']['reference_match'],p['id']
    assert p['source_sha256']==sha(p['source']) and p['binary_sha256']==sha(p['binary']),p['id']
    assert p['small_validation_source_sha256']==p['source_sha256'] and p['small_validation_binary_sha256']==p['binary_sha256'],p['id']
    p['key']=p['benchmark']+':'+p['id']
    original=upstream.get(p.get('upstream_id'))
    if original:
        p['upstream_source']=original['path']
        p['upstream_sha256']=original['sha256']
        p['source_url']=original['url']
        assert sha(original['path'])==p['upstream_sha256']
    else:p['source_url']='https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/knucleotide-ifx-1.html'
    p['local_adaptation']=' '.join(p['adaptations'])
    p.update(normalize(Path(p['source']).read_text(),'dart' if p['language'].startswith('dart') else p['language']))
    if p['language']=='fortran':p['runtime_environment']={'OMP_NUM_THREADS':'8','OMP_THREAD_LIMIT':'8','OMP_DYNAMIC':'FALSE'}
    a={'strict':True,'native_benchmark_kernel':False,'max_workers':8,'complete_application_counted':True}
    b=p['benchmark']
    if b=='nbody':a.update(binary64=True,full_steps=True,pair_forces='all ten unordered pairs in every requested step')
    if b=='mandelbrot':a.update(binary64=True,max_iterations=50,full_pbm=True,tail_padding_zero=True,independent_edge_width_checks=str(S/'mandelbrot-work-checks.json'))
    if b=='knucleotide':a.update(widths=[1,2,3,4,6,12,18],all_overlapping_windows=True,full_growable_maps=True,small_initial_capacity=True,long_key_bits=36,independent_collision_case_record_growth_checks=str(S/'knucleotide-work-checks.json'))
    if b=='spectralnorm':a.update(binary64=True,iterations=10,full_implicit_matrix=True)
    if b=='revcomp':a.update(all_fasta_records=True,full_iub_complement=True,full_output=True)
    if b=='fannkuchredux':a.update(permutations='n!',checksum=True,maximum_flip_count=True)
    if b=='fasta':a.update(binary64_probability=True,explicit_random_division=True,all_lcg_steps=True,total_symbols='10*n',random_symbols='8*n',integer_scaled_thresholds=False)
    if b=='binarytrees':
        a.update(distinct_leaf_and_interior_objects=True,equal_leaf_and_interior_record=True,full_tree_nodes='2**(depth+1)-1',full_iterations=True,long_root_retained_through_temporary_iterations=True)
        parent=p.get('jit_source_parent',p['id'])
        if parent in tree_records:a['allocation_and_optimized_compiler_audit']=tree_records[parent]
    if b=='regexredux':a.update(all_nine_pattern_counts=True,all_five_required_substitutions=True,full_input=True)
    if b=='pidigits':a.update(true_arbitrary_precision=True,both_extract_3_and_4_quotients=True,every_candidate_attempt=True)
    p['algorithm']=a
    if p['id']=='knucleotide-fortran-parallel':
        p['adaptations']=[s.replace('A module-scoped worker avoids a GNU 16 Arm64 OpenMP host-association defect for deferred-length character arguments.','An explicit integer8 sequence array avoids a GNU 16 Arm64 OpenMP capture defect for deferred-length character arguments.') for s in p['adaptations']]
        p['local_adaptation']=' '.join(p['adaptations'])
    if p['id']=='binarytrees-dart-3':
        m['excluded'].append(p|{'reason':'Per-depth isolate spawning exceeds the eight-worker cohort limit at N=21, and it computes the long-lived check in a separate isolate before temporary iterations finish. Complete source/build is retained but is not a strict timing candidate.'})
    else:programs.append(p)
libs={}
queue=[Path(v) for v in ['/opt/homebrew/opt/apr/lib/libapr-1.0.dylib','/opt/homebrew/opt/gmp/lib/libgmp.10.dylib','/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib','/opt/homebrew/opt/gcc/lib/gcc/current/libgfortran.5.dylib','/opt/homebrew/opt/gcc/lib/gcc/current/libgomp.1.dylib','/opt/homebrew/opt/gcc/lib/gcc/current/libquadmath.0.dylib']]
while queue:
    p=queue.pop()
    if str(p) in libs:continue
    libs[str(p)]=sha(p)
    lines=subprocess.check_output(['otool','-L',str(p)],text=True).splitlines()[1:]
    for line in lines:
        dep=line.strip().split(' (')[0]
        if dep.startswith('/opt/homebrew/') and Path(dep).is_file() and dep not in libs:queue.append(Path(dep))
sdk=json.loads((S/'sdk.json').read_text())
vm=S/'dart-sdk/bin/dartvm';dart=S/'dart-sdk/bin/dart';gfortran=Path('/opt/homebrew/bin/gfortran-16')
m['runtimes']={
 'fortran':{'path':str(gfortran),'sha256':sha(gfortran),'version':m['toolchains']['fortran']['version'],'compiler':'GNU Fortran 16.2.0 native macOS Arm64, official sources listed for Intel ifx','compile_flags':['-O3','-mcpu=native','-isysroot /Library/Developer/CommandLineTools/SDKs/MacOSX.sdk','-fopenmp','-ffree-line-length-none','-fno-unsafe-math-optimizations','-ffp-contract=off'],'fasta_additional_flags':['-fdefault-real-8','-fdefault-double-8']},
 'dart':{'path':str(dart),'sha256':sha(dart),'version':sdk['version'],'sdk_archive_sha256':sdk['archive_sha256'],'sdk_archive_url':sdk['url'],'execution':'Native macOS Arm64 AOT executable, compiler and SDK creation outside timing','aot_runtime_path':str(S/'dart-sdk/bin/dartaotruntime'),'aot_runtime_sha256':sha(S/'dart-sdk/bin/dartaotruntime'),'aot_compiler_path':str(S/'dart-sdk/bin/utils/gen_snapshot'),'aot_compiler_sha256':sha(S/'dart-sdk/bin/utils/gen_snapshot')},
 'dart-jit':{'path':str(vm),'sha256':sha(vm),'version':sdk['version'],'sdk_archive_sha256':sdk['archive_sha256'],'execution':'Fresh native Dart VM process from untimed standard kernel snapshot, ordinary optimizing JIT and startup included in measured process'}
}
m['libraries']=libs
m['programs']=programs
recommended=set()
for language in ['fortran','dart','dart-jit']:
    for benchmark in ['nbody','mandelbrot','knucleotide','spectralnorm','revcomp','fannkuchredux','fasta','binarytrees','regexredux','pidigits']:
        candidates=[p for p in programs if p['language']==language and p['benchmark']==benchmark]
        smallest=min(candidates,key=lambda p:(p['normalized_loc'],p['id']))
        fastest=min(candidates,key=lambda p:p['published_seconds'] if p.get('published_seconds') is not None else (0 if 'parallel' in p['id'] else 1e100))
        recommended.update([smallest['id'],fastest['id']])
m['recommended_timing_ids']=sorted(recommended)
m['recommended_timing_note']='Smallest normalized listed accepted candidate and published-fastest candidate for each language/workload. All accepted alternatives remain in programs for native local timing if required; published ordering is never presented as native measured ordering.'
m['status']='prepared'
m['full_timings_performed']=False
m['preparation_validation']={'canonical_small_executions':85,'strict_timing_candidates':len(programs),'knucleotide_canary_cases':28,'mandelbrot_canary_cases':60,'binarytree_allocation_audits':4,'reports':[str(S/f) for f in ['knucleotide-work-checks.json','mandelbrot-work-checks.json','tree-work-audit.json']]}
path.write_text(json.dumps(m,indent=2)+'\n')
print('PREPARED',len(programs),'strict candidates,',len(recommended),'compact/published-fast recommended timing candidates')
for language in ['fortran','dart','dart-jit']:print(language,len([p for p in programs if p['language']==language]))

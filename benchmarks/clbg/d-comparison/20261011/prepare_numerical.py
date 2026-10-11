"""Rebuild and validate the pinned native D numerical candidate cohort."""
import hashlib,json,pathlib,subprocess,sys
D=pathlib.Path(__file__).resolve().parent
C=D.parents[1]
sys.path.insert(0,str(C/'remaining-languages-comparison/20261011'))
from probe import validate_small
PINS=json.loads((D/'numerical-upstream-pins.json').read_text())
COMMON=['/opt/homebrew/bin/ldc2','-O3','-release','-boundscheck=off','-mcpu=native']
def sha(p):return hashlib.file_digest(pathlib.Path(p).open('rb'),'sha256').hexdigest()
rows=[]
def add(name,benchmark,tag,upstream,changes,reason,flags=()):
    source=D/'sources/numerical'/name
    binary=D/'build/numerical'/source.stem
    if flags:binary=binary.with_name(binary.name+'-'+tag)
    command=COMMON+list(flags)+[str(source),'-of='+str(binary)]
    r=subprocess.run(command,capture_output=True,text=True)
    assert r.returncode==0,(name,r.stderr)
    key=benchmark+':d-'+tag
    row=dict(id=benchmark+'-d-'+tag,key=key,benchmark=benchmark,language='d',source=str(source),binary=str(binary),
        command=[str(binary),'{n}'],build_command=command,source_sha256=sha(source),binary_sha256=sha(binary),
        adaptations=changes,algorithm=dict(strict=True,native_benchmark_kernel=False,reason=reason))
    repository='kragen/shootout' if upstream.startswith('archive-') else 'hanabi1224/Programming-Language-Benchmarks'
    label='archive' if upstream.startswith('archive-') else 'modern'
    if label=='archive':
        stem=upstream.removeprefix('archive-').removesuffix('.d')
        path='bench/'+benchmark+'/'+stem
    else:
        bs='spectral-norm' if benchmark=='spectralnorm' else 'fannkuch-redux' if benchmark=='fannkuchredux' else benchmark
        path='bench/algorithm/'+bs+'/'+upstream.removeprefix(bs+'-')
    row.update(source_url='https://github.com/'+repository+'/blob/'+PINS[label]+'/'+path,
        upstream_source=str(D/'upstream/numerical'/upstream),upstream_sha256=sha(D/'upstream/numerical'/upstream),upstream_commit=PINS[label])
    row['small_validation']=validate_small(row)
    print('SMALL PASS',row['id'],flush=True)
    rows.append(row)

nbody='All five bodies, all ten pair interactions on every step, floating-point positions/velocities and square roots; canonical momentum offset and two energy outputs. No integer rescaling or input shortcut.'
add('nbody-1.d','nbody','modern-scalar','nbody-1.d',['Added explicit module declaration required by hyphenated local filename.'],nbody)
add('nbody-2.d','nbody','modern-vector-arrays','nbody-2.d',['Added explicit module declaration required by hyphenated local filename.'],nbody)
add('archive-nbody.dlang-2.dlang.d','nbody','archive-classless','archive-nbody.dlang-2.dlang.d',[
'Updated D1 typedef/struct initializers/argument conversion to D2 alias/constructors/std.conv.',
'Added core.stdc.stdio printf import, explicit module name, replaced obsolete slice length with $.',
'Pass static planet array by ref to preserve original D1 typedef-reference behavior and mutation semantics.'],nbody)
add('nbody_compact.d','nbody','compact-array','archive-nbody.dlang-2.dlang.d',[
'Authored D2 compact equivalent of archived classless source using standard three-element array operations and std.numeric.dotProduct.',
'Kept all published constants as one constructor expression per body; independent program assignments remain separate statements.',
'Initialized floating-point accumulators explicitly to zero (D floating default is NaN).'],nbody)
add('nbody_unrolled.d','nbody','unrolled','archive-nbody.dlang-2.dlang.d',[
'Compact-array equivalent with compile-time static foreach unrolling of the complete five-body pair loops.',
'Fused multiply-add contraction enabled by compiler flag; all distances remain double-precision square roots.'],nbody,['-fp-contract=fast'])
add('nbody_unrolled.d','nbody','unrolled-precise','archive-nbody.dlang-2.dlang.d',[
'Compact-array equivalent with compile-time static foreach unrolling of the complete five-body pair loops.',
'Ordinary precise floating-point compilation (no global fast-math or aggressive FMA contraction flag).'],nbody)
spectral='Ten alternating AtA power iterations (40 matrix-vector products), formula-derived integer denominator, complete n-by-n products with float64 values and canonical nine-decimal result.'
add('spectral-norm-1.d','spectralnorm','modern','spectral-norm-1.d',['Added explicit module declaration required by hyphenated local filename.'],spectral)
add('archive-spectralnorm.dlang.d','spectralnorm','archive','archive-spectralnorm.dlang.d',[
'Updated D1 char[][]/atoi/inout foreach to D2 string[]/std.conv.to/ref and added explicit module declaration.'],spectral)
add('spectralnorm_compact.d','spectralnorm','compact','archive-spectralnorm.dlang.d',[
'Authored compact D2 equivalent using one compile-time transpose-specialized multiply function.',
'Explicit zero initialization for floating-point accumulators; scratch vector reused across canonical products.'],spectral)
add('spectralnorm_parallel.d','spectralnorm','parallel-neon','archive-spectralnorm.dlang.d',[
'Authored row-parallel equivalent with two-lane float64 SIMD division and independent-lane accumulation.',
'Formula-derived adjacent denominators are computed in integer arithmetic; all n columns are traversed with scalar tail for odd n.',
'Phobos task pool capped at seven worker threads plus the participating main thread (eight participants).'],spectral)
mandel='Canonical 50-iteration double-precision escape calculation for every output pixel, with PBM header and complete packed row bitmap written to stdout. SIMD grouping shares early exit only when every lane has escaped.'
add('mandelbrot-1.d','mandelbrot','modern-array-eight','mandelbrot-1.d',[
'Replaced modern repository MD5-only output with stdout.rawWrite(pixels); complete canonical PBM output is measured.',
'Removed blanket @safe annotation because std.stdio global stdout.rawWrite is classified @system.',
'Added explicit module name; algorithm unchanged.',
'Preserve true n dimensions/coordinates for non-byte-aligned sizes; zero unused PBM row-tail bits.'],mandel)
add('archive-mandelbrot.dlang-2.dlang.d','mandelbrot','archive-complex','archive-mandelbrot.dlang-2.dlang.d',[
'Updated obsolete built-in complex literals/cdouble to std.complex(Complex!double).',
'Updated argument conversion and byte cast for D2, used core.stdc.stdio.putchar for raw packed bytes.',
'Split two independent Z and C declarations onto separate lines and added module name.'],mandel)
add('mandelbrot_parallel.d','mandelbrot','parallel-array-eight','mandelbrot-1.d',[
'Same eight-pixel array algorithm with Phobos parallel row work capped at seven workers plus participating main thread.',
'Replaced MD5-only output with full raw PBM; coordinate ci is private to each row.',
'Preserve true n dimensions/coordinates for non-byte-aligned sizes; zero unused PBM row-tail bits.'],mandel)
fann='Enumerates every n! permutation in canonical order, performs prefix reversals for all required pancake flips, and calculates alternating checksum and maximum flips; no hardcoded result or skipped permutation.'
add('fannkuch-redux-1.d','fannkuchredux','compact-scalar','fannkuch-redux-1.d',['Added explicit module declaration required by hyphenated local filename.'],fann)
add('fannkuchredux_parallel_neon.d','fannkuchredux','parallel-neon','fannkuch-redux-2-im.d',[
'Replaced unavailable x86 inteli PSHUFB binding with native LDC float-independent 16-byte NEON TBL intrinsic; same mask tables/permutation/checksum algorithm.',
'Replaced vector construction/extraction helpers with core.simd ubyte16 casts and .array[0].',
'Capped Phobos pool at seven workers; split work into eight factorial-index blocks rather than hardware-dependent CPU count.'],fann)
report=dict(status='prepared',programs=rows,pins=PINS,compiler=subprocess.check_output(['/opt/homebrew/bin/ldc2','--version'],text=True),
unsupported_candidates=[dict(source=str(D/'upstream/numerical'/p),reason='Published x86 inteli-specific variant retained as upstream; native ARM equivalent of parallel2-im is included.') for p in ['fannkuch-redux-1-i.d','fannkuch-redux-1-im.d','fannkuch-redux-2-i.d']],
notes='Source-faithful archived and modern inputs preserved byte-for-byte. Full-workload validation and timings are performed centrally by the root runner; no long timing was executed in numerical preparation.')
(D/'numerical-manifest.json').write_text(json.dumps(report,indent=2)+'\n')

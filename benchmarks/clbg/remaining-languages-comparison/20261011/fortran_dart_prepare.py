#!/usr/bin/env python3
"""Pin current listed Fortran and Dart CLBG programs; prepare native macOS candidates."""
import argparse, concurrent.futures, hashlib, html, json, re, subprocess, urllib.request
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path

D = Path(__file__).resolve().parent
CLBG = D.parents[1]
BASE = 'https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
BENCHES = ['nbody','mandelbrot','knucleotide','spectralnorm','revcomp','fannkuchredux','fasta','binarytrees','regexredux','pidigits']
SUPPORT = D / 'support/fortran-dart'
UPSTREAM = D / 'upstream/fortran-dart'
MANIFEST = D / 'fortran_dart_manifest.json'

def sha(p):
    with Path(p).open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()

class Pre(HTMLParser):
    def __init__(self): super().__init__(); self.blocks=[]; self.active=False
    def handle_starttag(self,tag,attrs):
        if tag=='pre': self.blocks.append(''); self.active=True
    def handle_endtag(self,tag):
        if tag=='pre': self.active=False
    def handle_data(self,text):
        if self.active: self.blocks[-1]+=text

def download(url):
    with urllib.request.urlopen(url,timeout=90) as response: return response.read()

def fetch():
    SUPPORT.mkdir(parents=True,exist_ok=True); UPSTREAM.mkdir(parents=True,exist_ok=True)
    def listing(bench):
        url=BASE+'performance/'+bench+'.html'
        raw=download(url); (UPSTREAM/(bench+'-listing.html')).write_bytes(raw)
        rows=[]
        for row in re.split(r'<tr\b[^>]*>',raw.decode())[1:]:
            match=re.search(r'program/('+bench+r'-(ifx|dartexe|dart)-\d+)\.html',row)
            if not match: continue
            identifier,language=match.groups()
            cells=[html.unescape(re.sub(r'<[^>]*>','',s)).strip() for s in re.split(r'<td\b[^>]*>',row)[1:]]
            seconds=None
            try: seconds=float(cells[2])
            except (ValueError,IndexError): pass
            rows.append(dict(id=identifier,benchmark=bench,upstream_language=language,url=BASE+'program/'+identifier+'.html',published_seconds=seconds,upstream_bad_output='Bad' in row))
        return rows
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
        rows=[r for batch in pool.map(listing,BENCHES) for r in batch]
    def source(row):
        raw=download(row['url']); (UPSTREAM/(row['id']+'.html')).write_bytes(raw)
        parser=Pre(); parser.feed(raw.decode())
        ext='f90' if row['upstream_language']=='ifx' else 'dart'
        path=UPSTREAM/(row['id']+'.'+ext)
        path.write_text(parser.blocks[0].strip()+'\n')
        return row|dict(path=str(path),sha256=sha(path),build_notes=parser.blocks[1:] )
    with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool: rows=list(pool.map(source,rows))
    report=dict(schema_version=1,retrieved_at=datetime.now(timezone.utc).isoformat(),selection='Every current listed Intel Fortran and Dart native-AOT source across ten official CLBG tables. Original source pins are immutable; local compiler/native ARM adaptations are identified separately.',upstream=rows,programs=[],excluded=[])
    MANIFEST.write_text(json.dumps(report,indent=2)+'\n')
    print('Pinned',len(rows),'listed source programs',flush=True)
    for b in BENCHES:
        subset=[r for r in rows if r['benchmark']==b]
        print(b,[(r['id'],r['published_seconds'],r['upstream_bad_output']) for r in subset],flush=True)

def install_dart():
    SUPPORT.mkdir(parents=True,exist_ok=True)
    release=json.loads((D/'support/dart-release.json').read_text())
    version=release['version']; archive=SUPPORT/('dartsdk-macos-arm64-'+version+'.zip')
    url='https://storage.googleapis.com/dart-archive/channels/stable/release/'+version+'/sdk/dartsdk-macos-arm64-release.zip'
    expected=download(url+'.sha256sum').decode().split()[0]
    if not archive.exists():
        subprocess.run(['curl','--fail','--location','--retry','2','--output',str(archive),url],check=True)
    if sha(archive)!=expected: raise RuntimeError('Dart SDK archive SHA-256 mismatch')
    sdk=SUPPORT/'dart-sdk'
    if not sdk.exists(): subprocess.run(['ditto','-x','-k',str(archive),str(SUPPORT)],check=True)
    version_text=subprocess.check_output([str(sdk/'bin/dart'),'--version'],text=True,stderr=subprocess.STDOUT).strip()
    record=dict(url=url,archive=str(archive),archive_sha256=expected,sdk=str(sdk),runtime=str(sdk/'bin/dart'),version=version_text,release=release)
    (SUPPORT/'sdk.json').write_text(json.dumps(record,indent=2)+'\n')
    print(version_text,flush=True)

def adapt_fortran(source,bench,identifier):
    notes=['Compile the listed Fortran algorithm with native GNU Fortran 16 on macOS Arm64 instead of Linux Intel ifx.']
    if 'buffered=' in source:
        source=re.sub(r",\s*buffered\s*=\s*['\"]yes['\"]",'',source,flags=re.I)
        notes.append('Remove Intel-only buffered OPEN specifier; GNU standard stream I/O remains buffered.')
    if re.search(r'open\s*\(unit\s*=\s*input_unit',source,re.I):
        source=re.sub(r'open\s*\(unit\s*=\s*input_unit\s*,', 'open(unit=input_unit,file="/dev/stdin",',source,flags=re.I)
        notes.append('Reopen stdin explicitly as the existing /dev/stdin byte stream using GNU-compatible standard OPEN.')
    if re.search(r'open\s*\(unit\s*=\s*output_unit',source,re.I):
        source=re.sub(r'open\s*\(unit\s*=\s*output_unit\s*,', 'open(unit=output_unit,file="/dev/stdout",',source,flags=re.I)
        notes.append('Reopen stdout explicitly as /dev/stdout for unformatted byte output.')
    if bench=='nbody' and 'sqrt(real(d2))' in source:
        source=source.replace('sqrt(real(d2))','sqrt(d2)')
        notes.append('Retain binary64 squared distance in reciprocal square root and Newton refinement; remove the upstream float32 conversion.')
    if identifier=='mandelbrot-ifx-4':
        source=source.replace('PRIVATE(y, x, bit_num, pos, byte, Zr, Cr, Ci, inside, i)','PRIVATE(y, x, bit_num, pos, byte, Zr, Zi, Tr, Ti, Cr, Ci, inside, i)')
        notes.append('Privatize all per-pixel state, including Zi/Tr/Ti that the original OpenMP clause omitted, to remove a data race.')
    if identifier=='mandelbrot-ifx-6':
        source=source.replace('tr(k)+ti(k)<threshold .and. j<=w','tr(k)+ti(k)<=threshold .and. j+k-1<w')
        notes.append('Use the prescribed radius test and mask padded SIMD pixels beyond the requested image width; complete PBM output is retained.')
    if identifier=='mandelbrot-ifx-7':
        source=source.replace('tr(k)+ti(k) < threshold .and. j <= width','tr(k)+ti(k) <= threshold .and. j+k-1 <= width')
        notes.append('Use the prescribed radius test and explicitly mask padded SIMD pixels at arbitrary image widths.')
    if identifier=='mandelbrot-ifx-5':
        source=source.replace('real(Z*conjg(Z))','real(Z*conjg(Z),kind=dp)')
        notes.append('Keep the complex squared-radius comparison in binary64 instead of narrowing the norm to the default float32 real kind.')
    if bench=='fasta':
        notes.append('Compile all probability constants, probabilities, CDF sums and random divisions in binary64 using GNU -fdefault-real-8 -fdefault-double-8; one fresh LCG step and explicit binary64 division per random symbol.')
        if identifier.endswith('-4'):
            source=source.replace('probs = a%p * IM','probs = a%p').replace('getRandom(IM*1.0)','getRandom(1.0)')
            notes.append('Use unscaled binary64 CDF probabilities and direct random division; eliminate upstream integer-scaled threshold comparison.')
    if bench=='regexredux':
        # PCRE2_SIZE is size_t, not int; Apple Arm64 uses a 64-bit size_t ABI.
        source=source.replace('integer(c_int), intent(in), value :: length, options','integer(c_size_t), intent(in), value :: length\n            integer(c_int), intent(in), value :: options')
        source=source.replace('integer(c_int), intent(in), value :: length, startoffset, options','integer(c_size_t), intent(in), value :: length, startoffset\n            integer(c_int), intent(in), value :: options')
        source=source.replace('len(pattern), 0_c_int','int(len(pattern),c_size_t), 0_c_int')
        source=source.replace('src_string%size, pos, 0,','int(src_string%size,c_size_t), int(pos,c_size_t), 0,')
        source=source.replace('len(count_info(i)%data),','int(len(count_info(i)%data),c_size_t),')
        source=source.replace('sequences%size, pos, 0,','int(sequences%size,c_size_t), int(pos,c_size_t), 0,')
        notes.append('Correct naked PCRE2 FFI length/start-offset declarations and call casts to native 64-bit size_t. All FFI interfaces and substitution loops count toward application LOC.')
    if bench=='pidigits':
        source=source.replace('integer function  mpz_get_ui','integer(c_long) function  mpz_get_ui')
        source=source.replace('integer :: mp_alloc','integer(c_int) :: mp_alloc').replace('integer :: mp_size','integer(c_int) :: mp_size')
        notes.append('Correct GMP unsigned-long return ABI to native c_long. All arbitrary-precision arithmetic remains in GMP standard library calls; Fortran computes every spigot step.')
    if identifier=='binarytrees-ifx-1':
        source=source.replace('type(treeNode), target, intent(in) :: left, right','type(treeNode), pointer, intent(in) :: left, right')
        source=source.replace('node = treeNode(left, right)','node%left => left\n    node%right => right')
        notes.append('Associate tree pointers explicitly, including null leaves, instead of passing null pointer actuals to nonpointer dummy arguments; every leaf and interior node retains the same allocated derived type.')
    return source,notes

def adapt_dart(source,bench,identifier):
    notes=['Run the listed Dart application on Dart 3.13.5 native Arm64; retain every application helper in its LOC.']
    if bench=='pidigits' and identifier.endswith('-2'):
        source=source.replace('u = m % d + n;', 'u = (m + n) ~/ d;')
        source=source.replace('if (d.compareTo(u) > 0)', 'if (t == u)')
        notes.append('Compute both arbitrary-precision quotients, extract(3) and extract(4), at every candidate digit check instead of the published remainder-based stability shortcut.')
    if bench=='fasta':
        source=source.replace('probabilities[i] = sum * iM','probabilities[i] = sum')
        source=source.replace('nextLcgNumber(iM)','nextLcgNumber(1)')
        source=source.replace('return max / iM * seed;','return seed.toDouble() * max / iM;')
        source=source.replace('return max * last * oneOverIM;','return max * last / IM;')
        source=source.replace('const double oneOverIM = (1.0 / IM);','')
        if identifier.endswith('-8'):
            source=source.replace('   int len = seq.length;\n   int i, j;', '   int len = seq.length;\n   int i, j;\n   for (j=1; j<len; j++) probability[j] += probability[j-1];')
            source=source.replace('         v -= probability[j];\n         if (v<0) break;', '         if (v<probability[j]) break;')
            notes.append('Construct a binary64 cumulative probability table and compare the unchanged binary64 random value against it instead of subtracting each probability.')
        if 'sum * iM' in Path(UPSTREAM/(identifier+'.dart')).read_text():
            notes.append('Use binary64 CDF values and direct binary64 LCG division instead of integer-scaled thresholds; retain all full seed-advance steps.')
        notes.append('The source explicitly performs a binary64 random division for each random character; compiler optimizations are allowed but no integer-scaled or periodic lookup shortcut is used.')
    if bench=='mandelbrot' and identifier.endswith(('-2','-3','-4')):
        source=source.replace('final bytesPerRow = w >> 3;','final bytesPerRow = (w + 7) >> 3;')
        if identifier.endswith(('-3','-4')):
            source=source.replace('final initialR = new Float64List(w);','final initialR = new Float64List(bytesPerRow * 8);')
            source=source.replace('    return row;', '    if ((w & 7) != 0) row[bytesPerRow-1] &= (255 << (8-(w & 7))) & 255;\n    return row;')
        notes.append('Round the PBM row size upward and zero the unused tail bits for arbitrary widths, preserving all requested pixels and avoiding truncated final bytes.')
    if 'Platform.numberOfProcessors' in source:
        source=source.replace('Platform.numberOfProcessors','(Platform.numberOfProcessors > 8 ? 8 : Platform.numberOfProcessors)')
        notes.append('Cap the dynamic isolate worker count at eight, matching the benchmark cohort worker limit.')
        source=source.replace('2 * (Platform.numberOfProcessors > 8 ? 8 : Platform.numberOfProcessors)','(Platform.numberOfProcessors > 8 ? 8 : Platform.numberOfProcessors)')
    if bench=='knucleotide':
        source=source.replace('return _b - _a;','return _b == _a ? a.compareTo(b) : _b - _a;')
        source=source.replace('keysValues.sort((a, b) => b.value.compareTo(a.value));','keysValues.sort((a, b) => a.value == b.value ? a.key.compareTo(b.key) : b.value.compareTo(a.value));')
        source=source.replace('(freq[string])','(freq[string] ?? 0)').replace('baseCounts(code.length, seq)[code];','baseCounts(code.length, seq)[code] ?? 0;')
        if identifier.endswith('-1'):
            source=re.sub(r'String readInput\(\) \{.*?\n\}', '''String readInput() {
  String? line;
  while ((line = stdin.readLineSync()) != null && !line!.startsWith('>THREE')) {}
  final lines = <String>[];
  while ((line = stdin.readLineSync()) != null && !line!.startsWith('>')) {
    lines.add(line!);
  }
  return lines.join('').toUpperCase();
}''',source,count=1,flags=re.S)
            source=source.replace("String readLine() => stdin.readLineSync() ?? '>>>>>>';",'')
        notes.append('Use explicit lexical tie ordering, report missing requested words as zero, and retain complete growable-map construction for every overlapping window at all seven required widths.')
    if bench=='revcomp' and identifier.endswith(('-5','-8')):
        begin=source.index('    var sequenceStart = block.indexOf(def);')
        end=source.index('    sequence.add(block);',begin)+len('    sequence.add(block);')
        dispatch='''Isolate.spawn(other,
            Request(sequenceNumber++, sequence.takeBytes(), mainIsolate.sendPort));''' if identifier.endswith('-5') else 'reverseAndComplement(sequence.takeBytes());'
        replacement='''    var start = 0;
    for (var i = 0; i < block.length; i++) {
      if (block[i] != def) continue;
      if (sequence.length > 0 || i > start) {
        sequence.add(block.sublist(start,i));
        DISPATCH
      }
      start = i;
    }
    sequence.add(block.sublist(start));'''.replace('DISPATCH',dispatch)
        source=source[:begin]+replacement+source[end:]
        notes.append('Detect every FASTA header in each stdin block, preserving complete record boundaries even when a block contains several short records; the original only split at the first header in a block and failed the canonical small input.')
    if bench=='binarytrees' and identifier.endswith(('-4','-5')):
        source=source.replace('final TreeNode? left, right;','TreeNode? left, right;').replace('const TreeNode(','TreeNode(')
        notes.append('Use ordinary mutable node records and non-const constructors, avoiding implicit immutable-leaf canonicalization; each leaf and interior node is an independently allocated instance of the same class.')
    return source,notes

def prepare():
    report=json.loads(MANIFEST.read_text()); records=[];excluded=[]
    old={r['id']:r for r in report['programs']}
    for row in report['upstream']:
        if row['upstream_bad_output']:
            excluded.append(row|dict(reason='Official table marks this original source Bad Output; source retained for provenance.'));continue
        source=Path(row['path']).read_text(); bench=row['benchmark'];language='fortran' if row['upstream_language']=='ifx' else 'dart'
        if not any(l.strip() and not l.strip().startswith('!') for l in source.splitlines()):
            excluded.append(row|dict(reason='Official source is a placeholder without an executable implementation.'));continue
        result,notes=(adapt_fortran if language=='fortran' else adapt_dart)(source,bench,row['id'])
        folder=D/'sources'/language;folder.mkdir(parents=True,exist_ok=True)
        name=row['id'].replace('-ifx-','-fortran-').replace('-dartexe-','-dart-')
        path=folder/(name+('.f90' if language=='fortran' else '.dart'))
        path.write_text(result)
        record=dict(id=name,benchmark=bench,language=language,source=str(path),source_sha256=sha(path),upstream_id=row['id'],url=row['url'],upstream_sha256=row['sha256'],published_seconds=row['published_seconds'],adaptations=notes,strict=True)
        if name in old and old[name]['source_sha256']==record['source_sha256']:
            record=old[name]|record
        records.append(record)
    for r in old.values():
        if r.get('local_source'):records.append(r)
    report['programs']=records;report['excluded']=excluded
    MANIFEST.write_text(json.dumps(report,indent=2)+'\n');print('Prepared',len(records),'application sources',flush=True)

def build(only=None):
    report=json.loads(MANIFEST.read_text());(SUPPORT/'binaries').mkdir(exist_ok=True)
    dart=SUPPORT/'dart-sdk/bin/dart'
    compiler='/opt/homebrew/bin/gfortran-16'
    sysroot=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip()
    report['toolchains']=dict(fortran=dict(path=compiler,version=subprocess.check_output([compiler,'--version'],text=True).strip()),dart=json.loads((SUPPORT/'sdk.json').read_text()))
    for row in report['programs']:
        if only and row['id'] not in only and row['language'] not in only: continue
        if row.get('build_passed') and row.get('source_sha256')==sha(row['source']) and Path(row['binary']).exists() and sha(row['binary'])==row['binary_sha256']:
            print('BUILD',row['id'],'reused identical build',flush=True);continue
        binary=SUPPORT/'binaries'/row['id']; source=Path(row['source']); work=SUPPORT/'build'/row['id'];work.mkdir(parents=True,exist_ok=True)
        if row['language']=='fortran':
            argv=[compiler,'-O3','-mcpu=native','-isysroot',sysroot,'-fopenmp','-ffree-line-length-none','-fno-unsafe-math-optimizations','-ffp-contract=off']
            if row['benchmark']=='fasta': argv+=['-fdefault-real-8','-fdefault-double-8']
            argv+=[str(source),'-o',str(binary)]
            if row['id']=='binarytrees-fortran-2':argv+=['-L/opt/homebrew/opt/apr/lib','-lapr-1']
            if row['benchmark']=='pidigits':argv+=['-L/opt/homebrew/opt/gmp/lib','-lgmp']
            if row['benchmark']=='regexredux':argv+=['-L/opt/homebrew/opt/pcre2/lib','-lpcre2-8']
        elif row['language']=='dart-jit':
            binary=SUPPORT/'binaries'/(row['id']+'.dill')
            argv=[str(dart),'compile','kernel',str(source),'-o',str(binary)]
        else:argv=[str(dart),'compile','exe',str(source),'-o',str(binary)]
        done=subprocess.run(argv,cwd=work,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        row['compile_command']=argv;row['build_log']=str(work/'build.log');(work/'build.log').write_text(done.stdout)
        row['build_passed']=done.returncode==0;row['source_sha256']=sha(source)
        if row['build_passed']:
            row['binary']=str(binary);row['binary_sha256']=sha(binary)
            row['command']=([str(SUPPORT/'dart-sdk/bin/dartvm'),str(binary)] if row['language']=='dart-jit' else [str(binary)])+([] if row['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}'])
        else:row['build_error']=done.stdout[-5000:]
        print('BUILD',row['id'],'PASS' if row['build_passed'] else done.stdout[-1800:],flush=True)
        MANIFEST.write_text(json.dumps(report,indent=2)+'\n')

def add_local():
    report=json.loads(MANIFEST.read_text());by_id={r['id']:r for r in report['programs']}
    compact=D/'sources/fortran/knucleotide-fortran-compact.f90'
    fast=compact.with_name('knucleotide-fortran-parallel.f90')
    text=compact.read_text()
    text=text.replace('dna(used:used) = achar(code)','code = index(alphabet,achar(code))-1\n    if (code < 0) error stop "Non-DNA input"\n    dna(used:used) = achar(code)')
    text=text.replace('  do job=1,7\n','  !$omp parallel do num_threads(7)\n  do job=1,7\n',1)
    text=text.replace('  do job=1,2\n','  !$omp end parallel do\n  do job=1,2\n',1)
    text=text.replace('symbol = index(alphabet,sequence(i:i))-1','symbol = iachar(sequence(i:i))')
    text=text.replace('call count_windows(dna(:used),widths(job),trim(targets(job)),counts(:,job),answers(job))','call count_windows(dna,used,widths(job),targets(job),counts(:,job),answers(job))')
    text=text.replace('subroutine count_windows(sequence,k,target,frequencies,answer)','subroutine count_windows(sequence,n,k,target,frequencies,answer)')
    text=text.replace('integer, intent(in) :: k','integer, intent(in) :: n,k')
    text=text.replace('do i=1,len(sequence)','do i=1,n').replace('do i=1,len(target)','do i=1,k')
    text=text.replace('  subroutine ', '  recursive subroutine ').replace('  integer function ', '  recursive integer function ').replace('  integer(int64) function ', '  recursive integer(int64) function ')
    # Avoid a GCC 16 Arm64 OpenMP defect in the static chain for host-associated
    # character arguments: the independently callable worker belongs in the module.
    marker='\ncontains\n  recursive subroutine count_windows'
    begin=text.index(marker)
    helper=text[begin+len('\ncontains\n'):text.rindex('\nend program')]
    text=text[:begin]+'\nend program\n'
    insertion=text.index('\nend module')
    text=text[:insertion]+'\n'+helper+text[insertion:]
    declaration="  character(len=*), parameter :: alphabet = 'ACGT'\n"
    text=text.replace(declaration,'')
    text=text.replace('  implicit none\n','  implicit none\n'+declaration,1)
    text=text.replace('only: int64','only: int8,int64')
    text=text.replace('  character(len=:), allocatable :: dna','  character(len=:), allocatable :: dna\n  integer(int8), allocatable :: encoded(:)')
    text=text.replace('  !$omp parallel do num_threads(7)','  allocate(encoded(used))\n  do i=1,used\n    encoded(i) = int(iachar(dna(i:i)),int8)\n  end do\n  deallocate(dna)\n  !$omp parallel do num_threads(7)')
    text=text.replace('call count_windows(dna,used,','call count_windows(encoded,used,')
    text=text.replace('    character(len=*), intent(in) :: sequence,target','    integer(int8), intent(in) :: sequence(:)\n    character(len=*), intent(in) :: target')
    text=text.replace('symbol = iachar(sequence(i:i))','symbol = int(sequence(i))')
    fast.write_text(text)
    for path in [compact,fast]:
        name=path.stem
        record=dict(id=name,benchmark='knucleotide',language='fortran',source=str(path),source_sha256=sha(path),local_source=True,upstream_id=None,url=None,upstream_sha256=None,published_seconds=None,strict=True,adaptations=['Locally implement the missing listed Fortran k-nucleotide program in Fortran; generic growable open-addressed map code is counted as application code.','All seven widths 1,2,3,4,6,12,18 build full frequency maps; all overlapping windows participate; full integer64/36-bit long keys, initial capacity16 and generic doubling rehashing.','Parse the entire THREE FASTA record with uppercase and CRLF handling; print explicit lexical ties and absent target counts as zero.'])
        if path==fast:record['adaptations'].append('Encode DNA once into an explicit integer8 array and compute the seven independent complete maps with seven OpenMP workers; each map follows the unchanged recurrence and output. The typed sequence array avoids a GNU 16 Arm64 OpenMP capture defect for deferred-length character arguments.')
        if name in by_id and by_id[name]['source_sha256']==record['source_sha256']:record=by_id[name]|record
        by_id[name]=record
    report['programs']=list(by_id.values());MANIFEST.write_text(json.dumps(report,indent=2)+'\n')

def probe(only=None):
    import importlib.util, os, tempfile
    spec=importlib.util.spec_from_file_location('fd_compact_runner',CLBG/'compact/run.py')
    compact=importlib.util.module_from_spec(spec);spec.loader.exec_module(compact)
    work=compact.WORK|{'regexredux':(30,5000000),'pidigits':(30,10000)}
    report=json.loads(MANIFEST.read_text())
    for row in report['programs']:
        if not row.get('build_passed') or (only and row['id'] not in only and row['language'] not in only):continue
        if row.get('small_validation') and row.get('small_validation_source_sha256')==sha(row['source']) and row.get('small_validation_binary_sha256')==sha(row['binary']):continue
        bench=row['benchmark']; n=work[bench][0]
        argv=[s.replace('{n}',str(n)) for s in row['command']]
        input_path=compact.SMALL_INPUT if bench in ('knucleotide','revcomp') else CLBG/'regexredux/inputs/regexredux-input-small.txt' if bench=='regexredux' else None
        try:
            with tempfile.TemporaryFile() as out,tempfile.TemporaryFile() as err:
                with (input_path.open('rb') if input_path else open(os.devnull,'rb')) as inp:
                    compact.run_process(argv,inp,out,err,40,compact.ENV|row.get('runtime_environment',{}))
                out.seek(0,2); size=out.tell();out.seek(0);digest=hashlib.file_digest(out,'sha256').hexdigest()
                out.seek(0); actual=out.read()
                if bench=='nbody':compact.common.validate(actual.decode(),[-0.169075164,-0.169087605])
                elif bench=='spectralnorm':assert actual==b'1.274219991\n',repr(actual)
                elif bench=='fannkuchredux':assert actual==b'228\nPfannkuchen(7) = 16\n',repr(actual)
                elif bench in ('regexredux','pidigits'):
                    ref=CLBG/bench/'inputs'/(bench+'-output-small.txt')
                    assert size==ref.stat().st_size and digest==sha(ref),(size,digest,ref.read_text())
                else:assert (size,digest)==compact.HASHES[bench][0],(size,digest,compact.HASHES[bench][0],actual[:300])
            row['small_validation']=dict(reference_match=True,bytes=size,sha256=digest,formal_sample=False)
            row['small_validation_source_sha256']=sha(row['source']);row['small_validation_binary_sha256']=sha(row['binary'])
            row.pop('small_error',None); print('SMALL PASS',row['id'],flush=True)
        except Exception as error:
            row.pop('small_validation',None);row['small_error']=str(error)
            print('SMALL FAIL',row['id'],str(error)[:2200],flush=True)
        MANIFEST.write_text(json.dumps(report,indent=2)+'\n')

def add_jit():
    from metrics import normalize
    report=json.loads(MANIFEST.read_text());by_id={r['id']:r for r in report['programs']}
    selected=[]
    for bench in BENCHES:
        candidates=[r for r in report['programs'] if r['language']=='dart' and r['benchmark']==bench and r.get('small_validation')]
        smallest=min(candidates,key=lambda r:(normalize(Path(r['source']).read_text(),'dart')['normalized_loc'],r['id']))
        fastest=min(candidates,key=lambda r:r['published_seconds'] if r['published_seconds'] is not None else 1e100)
        selected.extend([smallest,fastest])
    for parent in {p['id']:p for p in selected}.values():
        name=parent['id'].replace('-dart-','-dart-jit-')
        row={k:v for k,v in parent.items() if k not in ('binary','binary_sha256','build_log','build_passed','compile_command','command','small_validation','small_validation_source_sha256','small_validation_binary_sha256')}
        row|=dict(id=name,language='dart-jit',jit_source_parent=parent['id'],execution='Dart VM fresh-process JIT from an untimed compiled kernel snapshot; startup, JIT compilation, isolate setup, GC, computation and all I/O are timed.',local_source=True)
        row['adaptations']=parent['adaptations']+['Compile the same complete Dart source to a standard kernel snapshot outside timing; run the native Dart VM with its ordinary optimizing JIT in each fresh measured process.']
        if name in by_id and by_id[name]['source_sha256']==row['source_sha256']:row=by_id[name]|row
        by_id[name]=row
    report['programs']=list(by_id.values());MANIFEST.write_text(json.dumps(report,indent=2)+'\n')
    print('Prepared',len(set(p['id'] for p in selected)),'compact/published-fast JIT snapshots',flush=True)

if __name__=='__main__':
    parser=argparse.ArgumentParser(); parser.add_argument('phase',choices=['fetch','install-dart','prepare','build','add-local','add-jit','probe']); parser.add_argument('--only',nargs='+'); args=parser.parse_args()
    if args.phase=='build':build(args.only)
    elif args.phase=='probe':probe(args.only)
    else:{'fetch':fetch,'install-dart':install_dart,'prepare':prepare,'add-local':add_local,'add-jit':add_jit}[args.phase]()

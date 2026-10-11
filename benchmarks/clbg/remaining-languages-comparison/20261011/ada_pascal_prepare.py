"""Pin Ada/Free Pascal CLBG sources and native macOS toolchains; no full timing."""
import argparse, concurrent.futures, hashlib, importlib.util, json, os, re, shutil, subprocess, sys, tarfile, urllib.request
from datetime import datetime, timezone
from pathlib import Path

D=Path(__file__).resolve().parent
C=D.parents[1]
S=D/'support/ada-pascal'
U=D/'upstream/ada-pascal'
M=D/'ada_pascal_manifest.json'
BASE='https://benchmarksgame-team.pages.debian.net/benchmarksgame/'
LANGS={'gnat':('ada','.adb'),'fpascal':('pascal','.pas')}
PYTHON=sys.executable

def sha(p):
    with Path(p).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def request(url,headers=None):
    return urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'Mech CLBG language research',**(headers or {})}),timeout=180)

def download(url,path,expected=None,headers=None):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    if not path.exists():
        partial=path.with_suffix(path.suffix+'.partial')
        with request(url,headers) as response,partial.open('wb') as f:
            while chunk:=response.read(1024*1024):f.write(chunk)
        partial.replace(path)
    digest=sha(path)
    if expected:assert digest==expected,(path,digest,expected)
    return digest

def save(value):
    M.write_text(json.dumps(value,indent=2)+'\n')

def install():
    S.mkdir(parents=True,exist_ok=True)
    def fpc():
        info=json.load(request('https://formulae.brew.sh/api/formula/fpc.json'))
        item=info['bottle']['stable']['files']['arm64_tahoe']
        token=json.load(request('https://ghcr.io/token?service=ghcr.io&scope=repository:homebrew/core/fpc:pull'))['token']
        archive=S/'fpc-3.2.2.arm64_tahoe.bottle.tar.gz'
        digest=download(item['url'],archive,item['sha256'],{'Authorization':'Bearer '+token})
        target=S/'fpc'
        if not target.exists():
            target.mkdir()
            with tarfile.open(archive) as f:f.extractall(target,filter='data')
        prefix=next(p for p in (target/'fpc').iterdir() if p.is_dir())
        binary=next(prefix.rglob('ppca64'))
        return 'pascal',dict(path=str(binary),sha256=sha(binary),version='Free Pascal 3.2.2',prefix=str(prefix),architecture='native arm64 macOS',archive=str(archive),archive_sha256=digest,source_url=item['url'],checksum_verified=True)
    def gnat():
        url='https://github.com/alire-project/GNAT-FSF-builds/releases/download/gnat-16.1.0-1/gnat-aarch64-darwin-16.1.0-1.tar.gz'
        archive=S/url.rsplit('/',1)[1]
        checksum=request(url+'.sha256').read().decode().split()[0]
        digest=download(url,archive,checksum)
        target=S/'gnat'
        if not target.exists():
            target.mkdir()
            with tarfile.open(archive) as f:f.extractall(target,filter='data')
        binaries=[p for p in target.rglob('gnatmake') if p.is_file()]
        assert len(binaries)==1,binaries
        binary=binaries[0]
        return 'ada',dict(path=str(binary),sha256=sha(binary),version='GNAT/GCC 16.1.0-1 Alire native Apple Silicon distribution',prefix=str(binary.parent.parent),architecture='native arm64 macOS',archive=str(archive),archive_sha256=digest,source_url=url,checksum_verified=True,checksum_url=url+'.sha256',distribution_release='gnat-16.1.0-1')
    runtimes=dict(concurrent.futures.ThreadPoolExecutor(max_workers=2).map(lambda fn:fn(),[fpc,gnat]))
    for name,r in runtimes.items():
        print('INSTALLED',name,r['path'],flush=True)
    data=json.loads(M.read_text()) if M.exists() else dict(programs=[])
    data['runtimes']=runtimes;save(data)

def fetch():
    from html import unescape
    spec=importlib.util.spec_from_file_location('clbg_parser',C/'nbody/fetch_sources.py')
    parser=importlib.util.module_from_spec(spec);spec.loader.exec_module(parser)
    U.mkdir(parents=True,exist_ok=True)
    items=[];tables={}
    for upstream,(lang,ext) in LANGS.items():
        p=U/(upstream+'-measurements.html')
        url=BASE+'measurements/'+upstream+'.html';download(url,p)
        tables[upstream]=dict(url=url,sha256=sha(p))
        ids={}
        for fragment in re.split(r'<tr\b[^>]*>',p.read_text())[1:]:
            match=re.search(r'program/(\w+-'+upstream+r'-\d+)\.html',fragment)
            if not match:continue
            identifier=match[1]
            cells=[unescape(re.sub(r'<[^>]*>','',s)).strip() for s in re.split(r'<td\b[^>]*>',fragment)[1:]]
            try:elapsed=float(re.match(r'[\d.,]+',cells[2])[0].replace(',',''));n=int(re.match(r'[\d,]+',cells[6])[0].replace(',',''))
            except (ValueError,IndexError,TypeError):elapsed=n=None
            rec=ids.setdefault(identifier,dict(id=identifier,upstream=upstream,published=[]))
            rec['published'].append(dict(elapsed=elapsed,n=n))
        items.extend(ids.values())
    def one(item):
        upstream=item['upstream'];lang,ext=LANGS[upstream];identifier=item['id']
        url=BASE+'program/'+identifier+'.html';page=U/(identifier+'.html');download(url,page)
        parsed=parser.Preformatted();parsed.feed(page.read_text())
        assert len(parsed.blocks)>=2,identifier
        original=U/(identifier+ext);original.write_text(parsed.blocks[0].strip()+'\n')
        target=D/'sources'/lang/(identifier+ext);target.parent.mkdir(parents=True,exist_ok=True);target.write_bytes(original.read_bytes())
        benchmark=identifier.split('-'+upstream+'-')[0]
        return dict(id=identifier,key=benchmark+':'+identifier,language=lang,benchmark=benchmark,upstream_language=upstream,variant=int(identifier.rsplit('-',1)[1]),source=str(target),source_sha256=sha(target),original_source=str(original),original_source_sha256=sha(original),source_url=url,page_sha256=sha(page),upstream_notes=parsed.blocks[1].strip(),published=item['published'],adaptations=[])
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:rows=list(pool.map(one,items))
    data=json.loads(M.read_text()) if M.exists() else {};data.update(status='sources-pinned',programs=rows,tables=tables,retrieved_at=datetime.now(timezone.utc).isoformat(),policy='Native Ada and Free Pascal only. Every application helper/FFI declaration counts. Full output and common workload obligations must pass preflight before primary sequential timing. No compressed-size metric.');save(data)
    for r in rows:print('SOURCE',r['id'],len(Path(r['source']).read_text().splitlines()),flush=True)

def prepare():
    data=json.loads(M.read_text());rows=[];excluded=[]
    for record in data['programs']:
        row=record.copy();text=Path(row['original_source']).read_text();notes=[]
        if '__builtin_ia32' in text:
            row['exclusion']='Published hand-written x86 intrinsics are unavailable in the native ARM64 compiler; scalar/native Ada candidates remain included.';excluded.append(row);continue
        if 'PasMP' in text or 'mtprocs' in text or 'apr_' in text:
            row['exclusion']='Published application depends on a separate third-party thread/pool library, outside this cohort standard runtime + GMP/PCRE policy; native standard-library replacements are prepared separately.';excluded.append(row);continue
        if row['id']=='knucleotide-gnat-2':
            row['exclusion']='Published fixed-capacity GNAT.HTable.Static_HTable does not satisfy the current growing-table common-workload requirement; native growable-map replacements are prepared separately.';excluded.append(row);continue
        if row['id']=='regexredux-gnat-6' or row['id']=='regexredux-fpascal-1':
            row['exclusion']='Published PCRE1 FFI is replaced by native standard PCRE2 candidates, with every application FFI declaration counted.';excluded.append(row);continue
        if row['benchmark']=='fasta':
            old='(Max * Real (Signed (State))) * (1.0 / Real (IM))'
            if old in text:text=text.replace(old,'(Max * Real (Signed (State))) / Real (IM)');notes.append('Perform one direct binary64 division for every fresh LCG draw, preserving unscaled cumulative probabilities and full output.')
            old='Limit * Seed * (1 / IM)'
            if old in text:text=text.replace(old,'Double(Limit * Seed) / Double(IM)');notes.append('Perform one direct binary64 division for every fresh LCG draw, preserving unscaled cumulative probabilities and full output.')
        if row['language']=='pascal' and 'SetPrecisionMode(pmDouble)' in text:
            text=text.replace('  SetPrecisionMode(pmDouble);','');notes.append('Remove x87-only precision-mode setup; native ARM64 Double operations already use binary64.')
        if row['id']=='pidigits-fpascal-2':
            text=text.replace('{$linklib libgmp.so}','{$linklib gmp}')
            text=text.replace('mp_limb_t        = cuint;','mp_limb_t        = culong;')
            notes.append('Link the native macOS GMP dylib; use the ABI-correct 64-bit C unsigned-long GMP limb type.')
        if row['id']=='pidigits-fpascal-1':
            row['exclusion']='Historical custom bignum variant is retained in the pool but replaced by standard GMP variants for initial admission; its arithmetic and full obligations require a separate audit.';excluded.append(row);continue
        target=Path(row['source'])
        if notes:
            target=target.with_name(target.stem+'-faithful'+target.suffix);target.write_text(text)
            row.update(id=row['id']+'-faithful',key=row['benchmark']+':'+row['id']+'-faithful',source=str(target),source_sha256=sha(target),local_adaptation=True,adaptations=notes)
        else:row['local_adaptation']=False
        row['algorithm']=dict(strict=True,native_benchmark_kernel=False,application_language=row['language'],output='Complete canonical output.',obligations='Full prescribed CLBG input and all required algorithmic work; application and FFI helpers included in LOC.')
        rows.append(row)
    data.update(programs=rows,excluded_candidates=excluded,status='native-sources-prepared');save(data)
    print('PREPARED',len(rows),'native published candidates; excluded',len(excluded),flush=True)

def build():
    data=json.loads(M.read_text());runtimes=data['runtimes'];sdk=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip()
    lib=S/'library-aliases';lib.mkdir(exist_ok=True)
    alias=lib/'libgmp.3.dylib'
    if not alias.exists():alias.symlink_to('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib')
    for row in data['programs']:
        source=Path(row['source']);folder=S/'builds'/row['id'];folder.mkdir(parents=True,exist_ok=True)
        if row.get('source_sha256')!=sha(source):row.pop('small_validation',None)
        row['source_sha256']=sha(source)
        binary=folder/row['id'];runtime=runtimes[row['language']]
        env=os.environ.copy();env['PATH']=str(Path(runtime['path']).parent)+':'+env['PATH']
        if row['language']=='ada':
            chop=Path(runtime['path']).with_name('gnatchop')
            chopped=subprocess.run([str(chop),'-w',str(source)],cwd=folder,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            (folder/'chop.log').write_text(chopped.stdout)
            if chopped.returncode:row.update(build_status='failed',build_error=chopped.stdout);continue
            main=folder/(row['benchmark']+'.adb')
            assert main.exists(),(row['id'],[p.name for p in folder.glob('*.adb')])
            cmd=[runtime['path'],'-f','-q','-gnat2022','-gnatp','-gnatn2','-O3','-march=native','-funroll-loops','-fomit-frame-pointer','-fno-math-errno',str(main),'-o',str(binary),'-largs','-L/opt/homebrew/opt/gmp/lib','-L/opt/homebrew/opt/pcre2/lib']
            if row.get('uses_apr'):cmd+=['-L/opt/homebrew/opt/apr/lib','-lapr-1']
            names=re.findall(r'^\s+([\w-]+\.ad[bs])\s*$',chopped.stdout,re.M)
            row['compiled_source_files']={str(folder/name):sha(folder/name) for name in names}
        else:
            units=Path(runtime['prefix'])/'lib/fpc/3.2.2/units/aarch64-darwin'
            mode='-Mdelphi' if re.search(r'class\s+(?:function|procedure)',source.read_text(),re.I) else '-Mobjfpc'
            cmd=[runtime['path'],'-n',mode,'-Sh','-Sc','-Sj','-O3','-OoLOOPUNROLL','-Fu'+str(units)+'/*','-Fu'+str(S/'library-sources'),'-Fl'+str(lib),'-Fl/opt/homebrew/opt/gmp/lib','-Fl/opt/homebrew/opt/pcre2/lib','-k-syslibroot','-k'+sdk,'-FE'+str(folder),'-FU'+str(folder),'-o'+str(binary),str(source)]
        row['build_command']=cmd
        previous=folder/'build-pin.json'
        if previous.exists():
            pin=json.loads(previous.read_text())
            if pin.get('source_sha256')==sha(source) and pin.get('build_command')==cmd and binary.exists() and pin.get('binary_sha256')==sha(binary):
                row.update(binary=str(binary),binary_sha256=sha(binary),build_status='passed',command=[str(binary)]+([] if row['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']));print('REUSE BUILD',row['id'],flush=True);continue
        result=subprocess.run(cmd,cwd=folder,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
        (folder/'build.log').write_text(result.stdout)
        row.update(build_status='passed' if result.returncode==0 else 'failed',build_log=str(folder/'build.log'))
        if result.returncode:row['build_error']=result.stdout[-2200:];print('FAIL BUILD',row['id'],result.stdout[-1600:],flush=True)
        else:
            row.update(binary=str(binary),binary_sha256=sha(binary),command=[str(binary)]+([] if row['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']),compiler_sha256=runtime['sha256'])
            previous.write_text(json.dumps(dict(source_sha256=sha(source),binary_sha256=sha(binary),build_command=cmd),indent=2)+'\n');print('BUILT',row['id'],flush=True)
        save(data)
    save(data)

def probe():
    spec=importlib.util.spec_from_file_location('canonical',C/'array-comparison/all-20261009/run.py')
    canonical=importlib.util.module_from_spec(spec);spec.loader.exec_module(canonical)
    data=json.loads(M.read_text())
    for row in data['programs']:
        if row.get('build_status')!='passed':continue
        bench=row['benchmark'];n=canonical.WORK[bench][0]
        command=[a.replace('{n}',str(n)) for a in row['command']]
        inp=canonical.compact.SMALL_INPUT if bench in ('knucleotide','revcomp') else C/'regexredux/inputs/regexredux-input-small.txt' if bench=='regexredux' else None
        result=subprocess.run(command,input=inp.read_bytes() if inp else None,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=90,env=canonical.compact.ENV|row.get('runtime_environment',{}))
        output=result.stdout;digest=hashlib.sha256(output).hexdigest()
        if bench=='nbody':reference=b'-0.169075164\n-0.169087605\n'
        elif bench in ('spectralnorm','fannkuchredux'):reference={'spectralnorm':b'1.274219991\n','fannkuchredux':b'228\nPfannkuchen(7) = 16\n'}[bench]
        elif bench in ('regexredux','pidigits'):reference=(C/bench/'inputs'/(bench+'-output-small.txt')).read_bytes()
        else:reference=None
        expected=(len(reference),hashlib.sha256(reference).hexdigest()) if reference is not None else canonical.compact.HASHES[bench][0]
        valid=result.returncode==0 and (len(output),digest)==tuple(expected)
        row['small_validation']=dict(reference_match=valid,bytes=len(output),sha256=digest,n=n,command=command,returncode=result.returncode,stderr=result.stderr.decode(errors='replace')[:1800])
        if not valid:
            row['small_validation']['output_excerpt']=repr(output[:160]);row['small_validation']['expected']=expected
        print('SMALL',row['id'],'PASS' if valid else 'FAIL',row['small_validation'] if not valid else '',flush=True)
        save(data)
    data['status']='prepared' if all(r.get('build_status')=='passed' and r.get('small_validation',{}).get('reference_match') for r in data['programs']) else 'partial';save(data)

def written():
    data=json.loads(M.read_text())
    pool=data['programs']+data['excluded_candidates']
    existing={r['id'] for r in data['programs']}
    replacements={
        'knucleotide-ada-compact':('ada','knucleotide','knucleotide-gnat-2',['Replace the published static GNAT hash table with seven standard Ada.Containers.Hashed_Maps, starting at capacity 16 and growing as required. Fully count every overlapping window at widths 1,2,3,4,6,12,18 using collision-safe full 36-bit keys; uppercase input, retain THREE boundaries, and sort frequency ties lexically.']),
        'knucleotide-pascal-compact':('pascal','knucleotide',None,['Native Pascal implementation of all seven complete growing standard TDictionary maps, each starting at capacity 16. Full unsigned 64-bit key equality, uppercase THREE record, lexical ties, and zero for absent requested motifs. No current official Free Pascal k-nucleotide source is listed.']),
        'regexredux-ada-pcre2':('ada','regexredux','regexredux-gnat-6',['Use the standard native PCRE2/JIT engine rather than retired PCRE1; retain all nine nonoverlapping counts and five ordered global substitutions. Every application binding, pattern, buffer and sequencing step is Ada source included in LOC. Complete stdin bytes and all output are retained.']),
        'regexredux-pascal-pcre2':('pascal','regexredux','regexredux-fpascal-1',['Use the standard native PCRE2/JIT engine rather than retired PCRE1; retain all nine nonoverlapping counts and five ordered global substitutions. Every application binding, pattern, buffer and sequencing step is Pascal source included in LOC. Complete stdin bytes and all output are retained.']),
        'binarytrees-pascal-compact':('pascal','binarytrees','binarytrees-fpascal-6',['Replace separate PasMP/PooledMM dependencies with ordinary native Pascal allocation and traversal. Allocate one distinct equal-size two-pointer record for every leaf and interior. Build each tree before a separate traversal; retain the complete long-lived tree through all temporary trees, then check/free it.'])
    }
    for identifier,(lang,bench,parent,notes) in replacements.items():
        if identifier in existing:continue
        source=D/'sources'/lang/(identifier+('.adb' if lang=='ada' else '.pas'))
        origin=next((r for r in pool if r['id']==parent),None)
        row=dict(id=identifier,key=bench+':'+identifier,benchmark=bench,language=lang,source=str(source),source_sha256=sha(source),source_url=origin['source_url'] if origin else BASE+'description/knucleotide.html',adaptations=notes,local_adaptation=True,algorithm=dict(strict=True,native_benchmark_kernel=False,application_language=lang,output='Complete canonical stdout.',obligations=notes))
        if origin:row.update(original_source=origin['original_source'],original_source_sha256=origin['original_source_sha256'])
        data['programs'].append(row)
    old=next((r for r in data['programs'] if r['id']=='mandelbrot-fpascal-5'),None)
    if old:
        data['programs'].remove(old);old['exclusion']='External MTProcs thread library unavailable under native standard-runtime policy.';data['excluded_candidates'].append(old)
    data['libraries']={p:sha(p) for p in ['/opt/homebrew/opt/gmp/lib/libgmp.10.dylib','/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib']}
    data['library_aliases']={str(S/'library-aliases/libgmp.3.dylib'):'/opt/homebrew/opt/gmp/lib/libgmp.10.dylib'}
    data['library_alias_notes']='The stock Free Pascal GMP unit requests the historical gmp.3 link name. A scoped symlink selects the current ABI-compatible native GMP 10 dylib; no host benchmark computation or application wrapper is introduced.'
    save(data)

def libraries():
    data=json.loads(M.read_text());restored=[]
    for row in data['excluded_candidates']:
        source=Path(row['original_source']);text=source.read_text();notes=[]
        if row['id']=='binarytrees-gnat-4':
            row=row.copy();row['uses_apr']=True
            text=re.sub(r'               Short_Lived_Tree_2 :=\n                  Create\n                    \(Short_Lived_Pool,\n                     Depth => Depth\);\n','',text)
            notes.append('Retain standard APR memory-pool allocation and all counted Ada FFI. Remove the extra untraversed second tree in each temporary iteration, so the constructor/traversal counts match the common workload exactly.')
        elif 'PasMP' in text:
            row=row.copy()
            text=text.replace('  with TPasMP.CreateGlobalInstance() do','  GlobalPasMPMaximumCountThreads := 8;\n  with TPasMP.CreateGlobalInstance() do')
            notes.append('Use pinned native Pascal PasMP generic job scheduling; PooledMM is the stock FPC FCL allocator. Set the standard maximum worker count to eight. All tree/pixel work remains explicit in the application; no package benchmark executor is called.')
            if row['benchmark']=='mandelbrot':
                if row['id']=='mandelbrot-fpascal-7':
                    text=text.replace('SetLength(Initial_R, Size);','SetLength(Initial_R, ((Size+7) div 8)*8);')
                    text=text.replace('for XY := 0 to Pred(Size) do begin','for XY := 0 to High(Initial_R) do begin')
                    text=text.replace('    Initial_I[XY] := InvScaled - 1.0;','    if XY<Size then Initial_I[XY] := InvScaled - 1.0;')
                    text=text.replace('  BytesPerRow := Size shr 3;','  BytesPerRow := (Size+7) div 8;')
                    text=text.replace('    FileWrite(StdOutputHandle, Rows[Y][0], BytesPerRow);','    begin\n      if Size mod 8<>0 then Rows[Y][BytesPerRow-1] := Rows[Y][BytesPerRow-1] and (255 shl (8-Size mod 8));\n      FileWrite(StdOutputHandle, Rows[Y][0], BytesPerRow);\n    end;')
                    notes.append('Handle arbitrary widths with padded coordinate storage, ceil-sized rows, and zero PBM padding bits; no remainder pixels are dropped.')
        elif 'mtprocs' in text.lower():
            row=row.copy()
            text=text.replace('begin\n  val(ParamStr(1)', 'begin\n  ProcThreadPool.MaxThreadCount := 8;\n  val(ParamStr(1)')
            text=text.replace('begin\n    Val(paramstr(1)', 'begin\n    ProcThreadPool.MaxThreadCount := 8;\n    Val(paramstr(1)')
            text=text.replace('begin\n  val(paramstr(1)', 'begin\n  ProcThreadPool.MaxThreadCount := 8;\n  val(paramstr(1)')
            if row['id']=='spectralnorm-fpascal-3':
                start=text.index('  up := @u[len];');end=text.index('\n  writeln',start)
                text=text[:start]+'''  for i := 0 to len do begin
    vBv := vBv + u[i]*v[i];
    vv := vv + v[i]*v[i];
  end;
'''+text[end:]
                notes.append('Include index zero in the final dot products; the published pointer loop terminated before reducing it.')
            notes.append('Use pinned generic MTProcs/MTPCPU runtime components from the official Lazarus repository; native Pascal retains all application loops, with standard maximum worker count eight.')
        else:continue
        row.pop('exclusion',None)
        identifier=row['id']+'-native';target=D/'sources'/row['language']/(identifier+source.suffix);target.write_text(text)
        row.update(id=identifier,key=row['benchmark']+':'+identifier,source=str(target),source_sha256=sha(target),adaptations=notes,local_adaptation=True,algorithm=dict(strict=True,native_benchmark_kernel=False,application_language=row['language'],obligations=notes))
        if identifier not in {r['id'] for r in data['programs']}:restored.append(row)
    data['programs']+=restored
    data['external_package_sources']={str(p):sha(p) for p in sorted((S/'library-sources').glob('*.pas'))}
    data['package_provenance']={'PasMP':dict(repository='https://github.com/BeRo1985/pasmp',commit='d58e2e661fef5e37d7e8f1ea8df536f6f34bda1f',source=str(S/'library-sources/PasMP.pas'),role='Generic native Pascal job/thread scheduler; no benchmark computation.'),'MTProcs':dict(repository='https://github.com/fpc/Lazarus',commit='1f218c5dcb0db5e133350e8cb8a994f64cc6e752',source=str(S/'library-sources/mtprocs.pas'),role='Generic Pascal parallel-for runtime; no benchmark computation.'),'PooledMM':dict(role='Stock Free Pascal FCL generic fixed-size memory allocator, already part of the pinned FPC bottle.'),'APR':dict(path='/opt/homebrew/opt/apr/lib/libapr-1.0.dylib',role='Generic system memory-pool allocator; all application FFI is counted.')}
    data['libraries']['/opt/homebrew/opt/apr/lib/libapr-1.0.dylib']=sha('/opt/homebrew/opt/apr/lib/libapr-1.0.dylib')
    for row in data['programs']:
        if row['id'] in ['fannkuchredux-gnat-3','fannkuchredux-fpascal-1']:
            text=Path(row['original_source']).read_text()
            if row['language']=='ada':
                text=text.replace('No_of_Tasks : constant := 12;','No_of_Tasks : constant := (if N<4 then 1 else 8);').replace('Max_Flips := 1;','Max_Flips := 0;')
                text=text.replace('Put_Line (C_image(2..C_image\'Last));','if C_Image(C_Image\'First)=\'-\' then Put_Line(C_Image); else Put_Line(C_Image(2..C_Image\'Last)); end if;')
                notes=['Limit ordinary Ada tasks to eight, with a generic sequential fallback for small permutation sets; retain full rank coverage and alternating checksum parity. Preserve a negative checksum sign and initialize the maximum to zero for small-input correctness.']
            else:
                text=text.replace('    tt[i] := BeginThread(@run, @td[i]);','    if i>=8 then WaitForThreadTerminate(tt[i-8],0);\n    tt[i] := BeginThread(@run, @td[i]);').replace('    WaitForThreadTerminate(tt[i], 0);','    if i>=n-8 then WaitForThreadTerminate(tt[i], 0);')
                notes=['Schedule every one of the N disjoint (N-1)! permutation chunks while keeping at most eight native Pascal workers active. Join every chunk exactly once, then aggregate all checksums and maxima.']
            identifier=row['id']+'-faithful';target=D/'sources'/row['language']/(identifier+Path(row['source']).suffix);target.write_text(text)
            row.update(id=identifier,key=row['benchmark']+':'+identifier,source=str(target),source_sha256=sha(target),adaptations=notes,local_adaptation=True)
    save(data)
    print('RESTORED',len(restored),'native library-backed published sources',flush=True)

def optimized():
    data=json.loads(M.read_text())
    for lang in ('ada','pascal'):
        parent=next(r for r in data['programs'] if r['id']=='knucleotide-'+lang+'-compact')
        text=Path(parent['source']).read_text()
        if lang=='pascal':
            text=text.replace('uses SysUtils, Generics.Collections;','uses CThreads, CMem, SysUtils, Generics.Collections, MTProcs;')
            text=text.replace('  OutBuf: array[0..65535] of Byte;','  OutBuf: array[0..65535] of Byte;\n  Raw: AnsiString;\n  ReadBuf: array[0..65535] of Byte;\n  Got,Used,Capacity,Start,Finish: LongInt;')
            start=text.index('  for I := 0 to 6 do begin\n    Maps[I] := TCounts.Create(16);')
            end=text.index('  for I := 0 to 1 do begin',start)
            count=text[start:end]
            count=count.replace('  for I := 0 to 6 do begin','  for I := Index to Index do begin',1)
            count=count.replace('QWord(Pos(DNA[J],Alphabet)-1)','QWord(Byte(DNA[J]))')
            callback='''procedure CountMaps(Index: PtrInt; Data: Pointer; Item: TMultiThreadProcItem);
var I,J,K,V: LongInt; Code,Mask: QWord;
begin
'''+count+'''end;
'''
            text=text[:start]+'  ProcThreadPool.MaxThreadCount := 8;\n  ProcThreadPool.DoParallel(@CountMaps,0,6,nil);\n'+text[end:]
            main=text.index('begin\n  SetTextBuf(Output,OutBuf);')
            text=text[:main]+callback+text[main:]
            start=text.index('  while not EOF(Input) do begin');end=text.index('  N := Length(DNA);',start)
            reader='''  Used := 0;
  Capacity := 65536;
  SetLength(Raw,Capacity);
  repeat
    Got := FileRead(StdInputHandle,ReadBuf,SizeOf(ReadBuf));
    if Got<=0 then Break;
    if Used+Got>Capacity then begin
      Capacity := Capacity*2;
      SetLength(Raw,Capacity);
    end;
    Move(ReadBuf,Raw[Used+1],Got);
    Inc(Used,Got);
  until False;
  SetLength(Raw,Used);
  Start := 1;
  N := 0;
  while Start<=Used do begin
    Finish := Start;
    while (Finish<=Used) and (Raw[Finish]<>#10) do Inc(Finish);
    if Raw[Start]='>' then begin
      if Active then Break;
      Active := Copy(Raw,Start,6)='>THREE';
    end else if Active then
      for J := Start to Finish-1 do
        if Raw[J]<>#13 then begin
          Inc(N);
          Raw[N] := Char(Pos(UpCase(Raw[J]),Alphabet)-1);
        end;
    Start := Finish+1;
  end;
  SetLength(Raw,N);
  DNA := Raw;
'''
            text=text[:start]+reader+text[end:]
        else:
            text=text.replace('with Interfaces; use Interfaces;','with Interfaces; use Interfaces;\nwith GNAT.OS_Lib;')
            text=text.replace('   Active : Boolean := False;','   Active : Boolean := False;\n   DNALength : Natural;\n   Buffer : String(1..65536);\n   Got : Integer;')
            text=text.replace('or Base(Data(I))','or Unsigned_64(Character\'Pos(Data(I)))')
            start=text.index('   while not End_Of_File loop');end=text.index('   for I in 1..2 loop',start)
            reader='''   loop
      Got := GNAT.OS_Lib.Read(GNAT.OS_Lib.Standin,Buffer'Address,Buffer'Length);
      exit when Got=0;
      if Got<0 then raise Constraint_Error; end if;
      Append(DNA,Buffer(1..Got));
   end loop;
   declare
      Data : String := To_String(DNA);
      First : Positive := 1;
      Last : Natural;
      Used : Natural := 0;
   begin
      DNA := Null_Unbounded_String;
      while First<=Data'Last loop
         Last := First;
         while Last<=Data'Last and then Data(Last)/=ASCII.LF loop Last := Last+1; end loop;
         if Data(First)='>' then
            exit when Active;
            Active := Last-First>=6 and then Data(First..First+5)=">THREE";
         elsif Active then
            for J in First..Last-1 loop
               if Data(J)/=ASCII.CR then
                  Used := Used+1;
                  Data(Used) := Character'Val(Integer(Base(Ada.Characters.Handling.To_Upper(Data(J)))));
               end if;
            end loop;
         end if;
         First := Last+1;
      end loop;
      DNALength := Used;
      declare
         task type Worker(Index : Positive);
         task body Worker is
         begin
            Count(Index,Data(1..Used));
         end Worker;
         type Worker_Access is access Worker;
         Workers : array(1..7) of Worker_Access;
      begin
         for I in 1..7 loop Workers(I) := new Worker(I); end loop;
      end;
   end;
'''
            text=text[:start]+reader+text[end:]
            text=text.replace('Length(DNA)-Widths(I)+1','DNALength-Widths(I)+1')
        identifier='knucleotide-'+lang+'-optimized';source=D/'sources'/lang/(identifier+Path(parent['source']).suffix);source.write_text(text)
        notes=parent['adaptations']+['Read generic buffered stdin in native language, compact and encode the entire THREE record once, and schedule all seven full width-specific growable-map scans with at most eight native workers. Full 36-bit keys, all overlapping updates, lexical ties and complete output are retained.']
        row=parent.copy();row.update(id=identifier,key='knucleotide:'+identifier,source=str(source),source_sha256=sha(source),adaptations=notes,local_adaptation=True,algorithm=dict(strict=True,native_benchmark_kernel=False,application_language=lang,obligations=notes))
        for k in ['small_validation','binary','binary_sha256','command','build_command','build_status','build_log']:row.pop(k,None)
        if identifier not in {r['id'] for r in data['programs']}:data['programs'].append(row)
    save(data)

def repair():
    data=json.loads(M.read_text())
    for row in data['programs']:
        source=Path(row['source']);text=source.read_text();notes=[]
        if row['id']=='fannkuchredux-gnat-3-faithful':
            text=text.replace('No_of_Tasks : constant := (if N<4 then 1 else 8);','No_of_Tasks : constant Positive := (if N<4 then 1 else 8);')
            text=text.replace('type Task_id_Range is range 1 .. No_of_Tasks;','subtype Task_id_Range is Positive range 1 .. No_of_Tasks;')
            text=text.replace('Chunk_Size := Factorial(N) / No_of_Tasks;','Chunk_Size := Factorial(N) / Perm_id_Range(No_of_Tasks);')
            text=re.sub(r'   if not \(N > 3 or \(not Multitasking_Desired and No_of_Tasks = 1\)\) then.*?   end if;\n','',text,flags=re.S)
            notes.append('Use a dynamic Positive worker-count subtype, retaining one worker for N<4 and eight otherwise; the obsolete small-N guard is removed because the generic single-task path covers every rank.')
        if row['language']=='pascal' and 'PasMP' in text and row['id'].endswith('-native'):
            text=text.replace('PooledMM, PasMP','PooledMM, MTProcs').replace('Math, PasMP','Math, MTProcs')
            text=re.sub(r'(class )?procedure (TNode\.)?DoTrees\(const Job: PPasMPJob;\s*const ThreadIndex: Int32;\s*const (UserData|Data): Pointer;\s*const FromIndex, ToIndex: (PtrInt|SizeInt)\)',lambda m: (m[1] or '')+'procedure '+(m[2] or '')+'DoTrees(Index: PtrInt; '+m[3]+': Pointer; Item: TMultiThreadProcItem)',text)
            text=re.sub(r'procedure (DoMandelbrot|RenderRows)\(const Job: PPasMPJob;\s*const ThreadIndex: Int32;\s*const Data: Pointer;\s*const FromIndex, ToIndex: SizeInt\)',lambda m:'procedure '+m[1]+'(RowNumber: PtrInt; Data: Pointer; Item: TMultiThreadProcItem)',text)
            text=text.replace('for Index := FromIndex to ToIndex do','for Index := RowNumber to RowNumber do')
            text=text.replace('FromIndex','Index')
            text=text.replace('  GlobalPasMPMaximumCountThreads := 8;\n  with TPasMP.CreateGlobalInstance() do\n    Invoke(ParallelFor(','  ProcThreadPool.MaxThreadCount := 8;\n  ProcThreadPool.DoParallel(')
            text=text.replace('(@Data, 0, High(Data), @TNode.DoTrees));','(@TNode.DoTrees, 0, High(Data), @Data);')
            text=text.replace('(@Data, 0, HighIndex, @TNode.DoTrees));','(@TNode.DoTrees, 0, HighIndex, @Data);')
            text=text.replace('(nil, 0, Pred(N), @DoMandelbrot));','(@DoMandelbrot, 0, Pred(N), nil);')
            text=text.replace('(nil, 0, Pred(Size), @RenderRows));','(@RenderRows, 0, Pred(Size), nil);')
            if row['benchmark']=='mandelbrot' and row['id']=='mandelbrot-fpascal-6-native':
                text=text.replace('TR + TI >= Limit','TR + TI > Limit')
                text=text.replace('      end;\n    end;\n  end;\n\nvar','      end;\n      if N mod 8<>0 then TextBuf[BufIndex] := (not Bits) and (255 shl (8-N mod 8));\n    end;\n  end;\n\nvar')
                notes.append('Retain the final partial PBM byte with zero padding bits, and use the prescribed strict escape test magnitude-squared >4.')
            notes.append('Port only the ordinary scheduler callback API to pinned MTProcs, because upstream PasMP ARM64 CAS assembly triggers FPC 3.2.2 internal error 2009112903. Every published application tree/pixel loop and native PooledMM allocation remains counted Pascal code; cap native workers at eight.')
            row['adaptations']=[n for n in row['adaptations'] if not n.startswith('Use pinned native Pascal PasMP')]
        if row['id']=='pidigits-gnat-2':
            text=text.replace('type mp_limb_t is new Interfaces.C.unsigned;','type mp_limb_t is new Interfaces.C.unsigned_long;')
            text=text.replace('Src2: in Interfaces.C.unsigned)','Src2: in Interfaces.C.unsigned_long)')
            text=text.replace('mpz_submul_ui(Acc, Den, D);','mpz_submul_ui(Acc, Den, Interfaces.C.unsigned_long(D));')
            text=text.replace('D, K, I : Interfaces.C.unsigned := 0;','D, D4, K, I : Interfaces.C.unsigned := 0;')
            text=text.replace('      if mpz_cmp(Num, Acc) > 0 then\n        goto Work_Loop;\n      end if;\n\n      D := Extract(3);\n      if D /= Extract(4) then','      if mpz_cmp(Num,Acc)>0 then goto Work_Loop; end if;\n      D := Extract(3);\n      D4 := Extract(4);\n      if D /= D4 then')
            notes.append('Retain the generic impossible-candidate prefilter; evaluate both extract(3) and extract(4), including both arbitrary-precision divisions, on every tested candidate. Use ABI-correct unsigned-long GMP limb and scalar arguments on ARM64.')
        if row['id']=='pidigits-fpascal-3':
            text=text.replace('d, k, i: LongWord;','d, d4, k, i, padding: LongWord;')
            text=text.replace('    if mpz_cmp(num, acc) > 0 then continue;\n\n    d := ExtractDigit(3);\n    if d <> ExtractDigit(4) then continue;','    if mpz_cmp(num,acc)>0 then continue;\n    d := ExtractDigit(3);\n    d4 := ExtractDigit(4);\n    if d<>d4 then continue;')
            text=text.replace("    if (i mod 10 = 0) or (i = n) then writeln(#9+':', i);","    if i mod 10=0 then writeln(#9+':', i)\n    else if i=n then begin\n      for padding := 1 to 10-i mod 10 do write(' ');\n      writeln(#9+':', i);\n    end;")
            notes.append('Retain the generic impossible-candidate prefilter; evaluate both arbitrary-precision extractions 3 and 4 on every tested candidate. Preserve required spaces and count for a partial final output row.')
        if row['id']=='pidigits-fpascal-2-faithful':
            text=text.replace('  i, k, digit, c: integer;','  i, k, digit, c, padding: integer;')
            text=text.replace('    SetLength(line, c);\n    writeln(line);',"    write(Copy(line,1,c));\n    for padding := c+1 to 10 do write(' ');\n    writeln(#9':',i);")
            notes.append('Preserve required spaces and the digit count for a partial final output row; both arbitrary-precision extraction divisions already execute on every candidate attempt.')
        if notes:
            source.write_text(text)
            row.update(source_sha256=sha(source),local_adaptation=True)
            row['adaptations']+=notes
            row['algorithm']['obligations']=row['adaptations']
            row.pop('small_validation',None)
    save(data)

if __name__=='__main__':
    ap=argparse.ArgumentParser();ap.add_argument('action',choices=['install','fetch','prepare','written','libraries','optimized','repair','build','probe']);a=ap.parse_args()
    {'install':install,'fetch':fetch,'prepare':prepare,'written':written,'libraries':libraries,'optimized':optimized,'repair':repair,'build':build,'probe':probe}[a.action]()

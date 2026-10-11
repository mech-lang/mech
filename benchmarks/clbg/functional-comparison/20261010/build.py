"""Compile the pinned functional-language sources before any measurement."""
import concurrent.futures,hashlib,json,os,re,subprocess
from datetime import datetime,timezone
from pathlib import Path
from fetch import metrics
from loc import normalize
D=Path(__file__).resolve().parent
GHC=Path('/opt/homebrew/opt/ghc@9.10/bin/ghc')
SWITCH=D/'toolchains/opam/functional-flambda'
OCAML=SWITCH/'bin/ocamlopt'
ENV=os.environ.copy();ENV['PATH']=str(D/'toolchains/bin')+':/opt/homebrew/opt/ghc@9.10/bin:/opt/homebrew/opt/llvm@19/bin:'+str(SWITCH/'bin')+':/opt/homebrew/bin:'+ENV['PATH']
ENV['OCAMLLIB']=subprocess.check_output([str(OCAML),'-where'],text=True).strip()
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def build(row):
    row=row.copy();source=Path(row['source']);binary=D/'binaries'/row['id']
    directory=D/'builds'/row['id'];directory.mkdir(parents=True,exist_ok=True)
    if row['language']=='haskell':
        flags=['--make','-fllvm','-O2','-XBangPatterns','-threaded','-rtsopts']
        flags+=sorted(set(re.findall(r'-X[A-Za-z0-9]+|-funfolding-use-threshold=\d+',row['upstream_notes']))-set(flags))
        flags+=['-package-env',str(D/'toolchains/ghc-env'),'-odir',str(directory),'-hidir',str(directory)]
        for package in ['bytestring','containers','array','deepseq','ghc-compact','text','unix','pvar']:flags+=['-package',package]
        if row['benchmark']=='binarytrees':
            if row.get('local_adaptation'):flags+=['-fno-cse','-fno-full-laziness']
            elif row['id'] in ('binarytrees-ghc-4','binarytrees-ghc-6'):flags+=['-fno-cse']
            flags+=['-ddump-simpl','-ddump-stg-final','-ddump-to-file','-dumpdir',str(directory)+'/', '-keep-llvm-files','-keep-s-files']
        if row['benchmark']=='regexredux':flags+=['-I/opt/homebrew/opt/pcre2/include','-L/opt/homebrew/opt/pcre2/lib','-lpcre2-8','-optc-DPCRE2_CODE_UNIT_WIDTH=8']
        if row['id'].startswith('pidigits-ghc-5'):flags+=['-I/opt/homebrew/opt/gmp/include','-L/opt/homebrew/opt/gmp/lib','-lgmp']
        cmd=[str(GHC),*flags,str(source),'-o',str(binary)]
    else:
        flags=['-noassert','-unsafe','-ccopt','-fPIC','-nodynlink','-inline','100','-O3']
        if row['benchmark']=='binarytrees':flags+=['-S']
        packages=['unix']
        if row['id'].startswith('pidigits-ocaml-5'):packages+=['zarith']
        if row['id'].startswith('pidigits-ocaml-7'):packages+=['gmp']
        if row['id'].startswith('regexredux-ocaml-2'):packages+=['str']
        if row['id'].startswith('regexredux-ocaml-3'):packages+=['re.pcre']
        cmd=[str(SWITCH/'bin/ocamlfind'),'ocamlopt',*flags,'-package',','.join(packages),'-linkpkg',str(source),'-o',str(binary)]
    p=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,env=ENV,cwd=directory)
    log=D/'results'/(row['id']+'.build.log');log.write_text(p.stdout)
    row.update(source_sha256=sha(source),source_metrics=metrics(source.read_text(),row['language']),loc_audit=normalize(source.read_text(),row['language']),
        build_command=cmd,build_status='passed' if p.returncode==0 else 'failed',build_log=str(log))
    if p.returncode==0:
        row.update(binary=str(binary),binary_sha256=sha(binary))
        n=[] if row['benchmark'] in ('knucleotide','revcomp','regexredux') else ['{n}']
        rts=[]
        if row['language']=='haskell' and re.search(r'\+RTS\s+-N',row['upstream_notes']):rts=['+RTS','-N8','-RTS']
        row['command']=[str(binary),*n,*rts]
    print('BUILD',row['id'],row['build_status'],flush=True)
    return row
def main():
    manifest=json.loads((D/'manifest.json').read_text());corrections=json.loads((D/'corrections.json').read_text())
    assert corrections['manifest_sha256']==sha(D/'manifest.json')
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:rows=list(pool.map(build,manifest['programs']+corrections['programs']))
    compilers={n:dict(path=str(p.resolve()),sha256=sha(p),version=subprocess.check_output([str(p),'--numeric-version' if n=='haskell' else '-version'],env=ENV,text=True).strip()) for n,p in [('haskell',GHC),('ocaml',OCAML)]}
    frontend=Path('/opt/homebrew/opt/ghc@9.10/lib/ghc-9.10.3/bin/ghc-9.10.3').resolve()
    compilers['haskell'].update(frontend_path=str(frontend),frontend_sha256=sha(frontend))
    libs=[Path('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib'),Path('/opt/homebrew/opt/mpfr/lib/libmpfr.6.dylib'),Path('/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib'),Path('/opt/homebrew/opt/libffi/lib/libffi.8.dylib')]
    libs+=list((SWITCH/'lib').glob('zarith/*.a'))+list((SWITCH/'lib').glob('gmp/*.a'))+list((SWITCH/'lib').glob('re/*.a'))
    report=dict(status='built',created_at=datetime.now(timezone.utc).isoformat(),manifest_sha256=sha(D/'manifest.json'),corrections_sha256=sha(D/'corrections.json'),
        compilers=compilers,llvm=dict(path='/opt/homebrew/opt/llvm@19/bin/llc',sha256=sha('/opt/homebrew/opt/llvm@19/bin/llc'),version=subprocess.check_output(['/opt/homebrew/opt/llvm@19/bin/llc','--version'],text=True).splitlines()[0]),
        haskell_packages=subprocess.check_output(['/opt/homebrew/opt/ghc@9.10/bin/ghc-pkg','list','--package-db',str(D/'toolchains/cabal-store/ghc-9.10.3-inplace/package.db')],env=ENV,text=True),
        ocaml_packages=subprocess.check_output([str(SWITCH/'bin/ocamlfind'),'list'],env=ENV,text=True),ocaml_config=subprocess.check_output([str(OCAML),'-config'],env=ENV,text=True),libraries={str(p.resolve()):sha(p) for p in libs},
        support_files={str(p.relative_to(D)):sha(p) for p in [D/'toolchains/bin/clang',D/'toolchains/ghc-env']},programs=rows)
    (D/'results/builds.json').write_text(json.dumps(report,indent=2)+'\n')
    print('BUILT',sum(r['build_status']=='passed' for r in rows),'of',len(rows))
if __name__=='__main__':main()

#!/usr/bin/env python3
"""Compile all CPU candidates before any timed measurement."""
import argparse,hashlib,json,subprocess
from pathlib import Path
HERE=Path(__file__).resolve().parent
MOJO='/tmp/mech-extra-languages-20261009/mojo-runtime/bin/mojo'
FUTHARK='/opt/homebrew/bin/futhark'
CLANG='/opt/homebrew/opt/llvm@22/bin/clang-22'
SDK='/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk'
NATIVE=HERE/'stdlib/libnative.dylib'
def run(command,log):
    result=subprocess.run(command,cwd=HERE,capture_output=True,text=True)
    log.write_text(result.stdout+result.stderr)
    if result.returncode:
        print(result.stderr[-6000:],flush=True)
        raise RuntimeError('Build failed: '+str(log))
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    ap=argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--only',nargs='+',help='Rebuild these source filenames; retain other build pins.')
    args=ap.parse_args()
    records=json.loads((HERE/'results/builds.json').read_text()) if args.only else []
    if not args.only:
        run([CLANG,'-O3','-mcpu=apple-m3','-isysroot',SDK,'-dynamiclib','-install_name',str(NATIVE),'-I/opt/homebrew/opt/gmp/include','-I/opt/homebrew/opt/pcre2/include',str(HERE/'stdlib/native.c'),'-L/opt/homebrew/opt/gmp/lib','-lgmp','-L/opt/homebrew/opt/pcre2/lib','-lpcre2-8','-o',str(NATIVE)],HERE/'builds/native.log')
    def selected(source):
        return not args.only or source.name in args.only
    if args.only:
        assert set(args.only) <= {p.name for p in (HERE/'sources').iterdir()},args.only
        records=[r for r in records if Path(r['source']).name not in args.only]
    for source in sorted((HERE/'sources').glob('*.mojo')):
        if not selected(source):continue
        output=HERE/'binaries'/(source.stem+'-mojo')
        command=[MOJO,'build',str(source),'-I',str(HERE/'stdlib'),'-O3','--fp-mode=contract=off','--target-cpu','apple-m3','-o',str(output)]
        if source.stem in ('regexredux','pidigits'):command+=['-Xlinker',str(NATIVE)]
        run(command,HERE/'builds'/(source.stem+'-mojo.log'))
        records.append(dict(language='mojo',source=str(source.relative_to(HERE)),source_sha256=sha(source),command=command,binary=str(output),binary_sha256=sha(output)))
        print('BUILT Mojo',source.stem,flush=True)
    for source in sorted((HERE/'sources').glob('*.fut')):
        if not selected(source):continue
        directory=HERE/'builds'/(source.stem+'-futhark');directory.mkdir(exist_ok=True)
        command=[FUTHARK,'multicore','--library','-o',str(directory/'program'),str(source)]
        run(command,directory/'compile.log')
        output=HERE/'binaries'/(source.stem+'-futhark')
        ccommand=[CLANG,'-O3','-mcpu=apple-m3','-ffp-contract=off','-isysroot',SDK,'-I',str(directory),str(HERE/'adapters/futhark.c'),str(directory/'program.c'),'-o',str(output),'-lm','-lpthread']
        run(ccommand,directory/'link.log')
        records.append(dict(language='futhark',source=str(source.relative_to(HERE)),source_sha256=sha(source),command=command,c_command=ccommand,binary=str(output),binary_sha256=sha(output),generated_c_sha256=sha(directory/'program.c')))
        print('BUILT Futhark',source.stem,flush=True)
    (HERE/'results/builds.json').write_text(json.dumps(records,indent=2)+'\n')
if __name__=='__main__':main()

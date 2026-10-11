#!/usr/bin/env python3
"""Compile unchanged Mech sources with source-independent tuning policies."""
import argparse, hashlib, json, shutil, subprocess
from pathlib import Path

HERE=Path(__file__).resolve().parent
NBODY=HERE.parents[1]
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--driver',type=Path,default=NBODY/'mech-driver/target/release/mech-clbg-nbody-compiled')
    ap.add_argument('--tag',required=True);ap.add_argument('--flags',default='');ap.add_argument('--manifest',type=Path,required=True)
    args=ap.parse_args();rows=json.loads(args.manifest.read_text()) if args.manifest.exists() else json.loads((HERE/'broadcast-programs.json').read_text())
    rows=[x for x in rows if 'broadcast' not in x['id']]
    frozen=HERE/('driver-'+args.tag)
    if not frozen.exists():shutil.copy2(args.driver,frozen)
    assert sha(frozen)==sha(args.driver)
    sdk=subprocess.check_output(['xcrun','--show-sdk-path'],text=True).strip()
    for form,name,loc,tail in [('compact','nbody-matrix-short.mec',20,False),('pairwise','nbody-matrix-pairwise.mec',25,True)]:
        ident=form+'-'+args.tag;assert not any(x['id']==ident for x in rows)
        src=HERE/'sources'/name;obj=HERE/(ident+'.o');generated=HERE/(ident+'.c');binary=HERE/ident
        flags=['--fp-contract','--algebraic-powers']+(['--pack-state-tails'] if tail else [])+args.flags.split()
        generation=[str(frozen),'0',str(src),'emit-cranelift-simd',str(generated),str(obj),*flags]
        subprocess.run(generation,check=True,capture_output=True)
        build=['/opt/homebrew/opt/llvm@22/bin/clang-22','-O3','-isysroot',sdk,str(generated),str(obj),'-lm','-o',str(binary)]
        subprocess.run(build,check=True,capture_output=True)
        rows.append(dict(id=ident,language='mech',source=str(src),source_sha256=sha(src),loc=loc,generation_command=generation,
            driver_sha256=sha(frozen),compile_command=build,object_sha256=sha(obj),binary_sha256=sha(binary),command=[str(binary),'{n}'],state_control=form+'-before',preserves_bits=True))
    args.manifest.write_text(json.dumps(rows,indent=2)+'\n')

if __name__=='__main__':main()

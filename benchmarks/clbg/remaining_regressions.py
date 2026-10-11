#!/usr/bin/env python3
"""Rebuild prior endpoint programs without overwriting historical artifacts."""
import hashlib, json, re, shutil, sys, tarfile
from pathlib import Path
import run_remaining as r

HERE=r.CLBG/'remaining-conformance';BUILD=HERE/'.build/regressions'

def main():
    BUILD.mkdir(parents=True,exist_ok=True)
    historical=json.loads((r.CLBG/'retime/results/apple-m3-ac-normal-20261008.json').read_text())
    checks=[]
    for benchmark in r.guard.WORK:
        if benchmark=='nbody':continue
        rows=[p for p in historical['programs'] if p['benchmark']==benchmark and p['language']=='mech' and not p.get('excluded_full')]
        endpoints={p['id']:p for p in (min(rows,key=lambda p:p['median_seconds']),min(rows,key=lambda p:(p['source']['nonblank_noncomment_lines'],p['median_seconds'])))}
        for old in endpoints.values():
            source=r.CLBG/benchmark/(old.get('path') or old['source']['paths'][0])
            binary=BUILD/old['id'];project=Path(str(binary)+'.project')
            if project.exists():shutil.rmtree(project)
            flags=['--fp-contract-loop'] if benchmark=='mandelbrot' and 'loop-fma' in old['id'] else ['--scalar'] if benchmark=='mandelbrot' and 'scalar' in old['id'] else []
            if old.get('simd') is False:flags=['--scalar']
            env=r.ENV|{k:v for k,v in (old.get('build_environment') or {}).items() if k not in ('RUSTFLAGS','CARGO_TARGET_DIR')}
            r.command([r.MECH,'build',source,'--backend','cranelift','--keep-project','--no-config','-o',binary,*flags],env=env)
            obj=next(project/name for name in ('grid.o','mech-sequence.o','mech-control.o') if (project/name).exists())
            digest=r.sha(obj)
            identical=digest==old['native_object_sha256']
            instruction_note=None
            if not identical:
                previous=Path(old['command'][0]+'.project')/obj.name
                assert previous.exists(),(old['id'],'missing historical object')
                def instructions(path):
                    dis=r.command(['/opt/homebrew/opt/llvm@22/bin/llvm-objdump','-d','--no-show-raw-insn','--no-leading-addr',path]).stdout.splitlines()[3:]
                    return '\n'.join(re.sub(r'(_mech_numeric_\d+)_f64',r'\1',line) for line in dis)
                assert instructions(previous)==instructions(obj),(old['id'],'native instructions changed')
                instruction_note='Only numeric helper symbol names differ: existing f64 suffix introduced by the earlier 32-bit array path. All emitted instructions are identical after normalizing those names.'
            command=[str(binary),*old['command'][1:]]
            row=dict(old,command=command)
            small=r.guard.execute(row,False,600);full=r.guard.execute(row,True,600)
            checks.append(dict(benchmark=benchmark,id=old['id'],source_sha256=r.sha(source),native_object_sha256=digest,native_object_identical=identical,machine_instructions_identical=True,instruction_note=instruction_note,small=small['validation'],full=full['validation']))
            print('passed',benchmark,old['id'],'identical instructions and full output',flush=True)
    # The f64 emitter was finalized before the new sequence/control features.
    frozen=r.CLBG/'nbody/experiments/20261009-local-simd/final-confirmation-50m.compiler.tar.gz'
    count=0
    with tarfile.open(frozen) as archive:
        for member in archive:
            if member.isfile() and member.name.startswith('hosts/gpu/src/fused_f64/'):
                assert archive.extractfile(member).read()==(r.ROOT/member.name).read_bytes(),member.name
                count+=1
    checks.append(dict(benchmark='nbody',unchanged_f64_compiler_files=count,confirmed_full_workload_report='nbody/experiments/20261009-local-simd/final-confirmation-50m.json'))
    (HERE/'regressions.json').write_text(json.dumps(dict(compiler_sha256=r.sha(r.MECH),checks=checks,passed=len(checks)),indent=2)+'\n')

if __name__=='__main__':main()

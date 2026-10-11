#!/usr/bin/env python3
"""Constant byte-index errors and laziness across single/multiple vector shapes."""
import json, os, shutil, subprocess
from pathlib import Path
import run

folder=run.BUILD/'constant-conformance'
folder.mkdir(exist_ok=True)
compiler=run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'
env=os.environ|{'RUSTFLAGS':'-D warnings','CARGO_TARGET_DIR':str(compiler.parents[1])}
cases=[]
for simd in (False,True):
    for n in (1,16,17,64):
        source=''
        for name,index in (('Zero',0),('Past',n+1),('Huge',(1<<64)-1),('Last',n)):
            source+=f'{name}(which<u64>, p<[u8]:{n},1>) = (v<u64>) :=\n  v := sequence/get(p, {index}).\n'
        source+=f'''Third(which<u64>, p<[u8]:{n},1>) = (v<u64>) :=
  v := control/if(which == 2, Huge, Last, (which, p)).
Second(which<u64>, p<[u8]:{n},1>) = (v<u64>) :=
  v := control/if(which == 1, Past, Third, (which, p)).
v := control/if(io/argument-u64(1) == 0, Zero, Second, (io/argument-u64(1), sequence/iota-bytes({n})))
io/write(text/format("{{}}\\n", v))
'''
        binary=folder/f'indices-{n}-{int(simd)}'
        path=binary.with_suffix('.mec'); path.write_text(source)
        project=Path(str(binary)+'.project')
        if project.exists(): shutil.rmtree(project)
        command=[str(compiler),'build',str(path),'--backend','cranelift','--offline',
                 '--no-config','--keep-project','-o',str(binary)]
        if not simd: command+=['--scalar']
        run.common.command(command,env=env)
        for which in range(5):
            result=subprocess.run([str(binary),str(which)],env=env,capture_output=True,timeout=30)
            if which<3: assert result.returncode!=0 and not result.stdout,(n,simd,which,result)
            else: assert result.returncode==0 and result.stdout==f'{n-1}\n'.encode(),(n,simd,which,result)
            cases.append(dict(shape=n,simd=simd,selector=which,passed=True))
        print('constant indices and lazy branches passed',n,simd,flush=True)
output=run.HERE/'results/constant-conformance.json'
output.write_text(json.dumps(dict(all_passed=True,checked_results=len(cases),
    compiler_executable_sha256=run.sha(compiler),cases=cases),indent=2)+'\n')

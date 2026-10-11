#!/usr/bin/env python3
"""Prepare separately labeled NEON translation and portable Rust LTO controls."""
import json, os, subprocess
from types import SimpleNamespace
import run

args = SimpleNamespace(
    mech=str(run.ROOT/'benchmarks/clbg/nbody/mech-driver/target/release/mech'),
    clang='/opt/homebrew/opt/llvm@22/bin/clang-22', gcc='/opt/homebrew/bin/gcc-16',
    gpp='/opt/homebrew/bin/g++-16', julia='/tmp/mech-clbg-julia/julia-1.13.1/bin/julia',
    java_home='/opt/homebrew/opt/openjdk', threads=8, timeout=300)
deps_target=run.BUILD/'rust-lto-deps'
subprocess.run(['cargo','build','--release','--locked','--offline','--manifest-path',
    str(run.HERE.parent/'mandelbrot/rust-deps/Cargo.toml')],cwd=run.ROOT,
    env=os.environ|{'RUSTFLAGS':'-Cembed-bitcode=yes -Ctarget-cpu=native',
                    'CARGO_TARGET_DIR':str(deps_target)},check=True)
args.rust_deps=str(deps_target/'release/deps')
prepared = run.BUILD/'prepared.json'
data = json.loads(prepared.read_text())
rows = {r['id']: r for r in data['programs']}
excluded = {r['id']: r for r in data['excluded']}
originals = json.loads((run.HERE/'manifest.json').read_text())['programs']
variants = [dict(r, id=r['id']+'-lto', lto='fat') for r in originals
            if r['language']=='rust' and r['id']!='fannkuchredux-rust-6']
parent = next(r for r in originals if r['id']=='fannkuchredux-rust-6')
assert run.sha(run.HERE/parent['path'])==parent['sha256']
for suffix, lto in (('',None),('-lto','fat')):
    port = dict(parent, id='fannkuchredux-rust6-neon'+suffix,
                path='comparisons/fannkuchredux-rust6-neon.rs', lto=lto,
                source_modified=True, parent_source_sha256=parent['sha256'],
                modification='Only SSE byte shuffle/lane extraction replaced with ARM NEON equivalents.')
    port.pop('sha256')
    variants.append(port)
env = os.environ | {k:'8' for k in ('RAYON_NUM_THREADS','CHPL_RT_NUM_THREADS_PER_LOCALE',
                                  'GOMAXPROCS','JULIA_NUM_THREADS','OMP_NUM_THREADS')}
for variant in variants:
    try:
        rows[variant['id']] = run.prepare(variant,args,env)
        excluded.pop(variant['id'],None)
        print('prepared',variant['id'],flush=True)
    except Exception as error:
        excluded[variant['id']] = dict(id=variant['id'],language='rust',reason=str(error))
        print('excluded',variant['id'],str(error),flush=True)
prepared.write_text(json.dumps(dict(programs=list(rows.values()),excluded=list(excluded.values())),indent=2)+'\n')

def reference(n):
    def permutations(a):
        if len(a)==1:
            yield a; return
        for _ in a:
            for p in permutations(a[:-1]): yield p+a[-1:]
            a=a[1:]+a[:1]
    checksum=maximum=0
    for i,p in enumerate(permutations(list(range(n)))):
        flips=0
        while p[0]: p=p[:p[0]+1][::-1]+p[p[0]+1:]; flips+=1
        checksum+=flips if i%2==0 else -flips; maximum=max(maximum,flips)
    return f'{checksum}\nPfannkuchen({n}) = {maximum}\n'.encode()

checked=[]
for row in rows.values():
    if not row.get('source_modified'): continue
    for n in range(1,9):
        output=subprocess.check_output(row['command']+[str(n)],env=env)
        assert output==reference(n),(row['id'],n,output)
        checked.append(dict(id=row['id'],n=n,passed=True))
out=run.HERE/'results/arm-port-conformance.json'
out.write_text(json.dumps(dict(all_passed=bool(checked),checked_results=len(checked),
    parent_source_sha256=parent['sha256'],cases=checked),indent=2)+'\n')

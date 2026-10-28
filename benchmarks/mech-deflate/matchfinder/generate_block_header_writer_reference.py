#!/usr/bin/env python3
"""Execute the pinned static/dynamic block-header emission verbatim."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
from generate_block_reference import pinned_package_sources, PIN, COMPILER_PIN
from generate_header_reference import literal, print_list
from generate_path_reference import pretty


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root',type=Path)
    parser.add_argument('--roc',type=Path,required=True)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args(); root=args.reference_root.resolve()
    package=pinned_package_sources(root)
    original=package['package/BlockOut.roc'].decode()
    start=original.index('\t\t\tif use_static {',original.index('flush_block ='))
    end=original.index('\n\n\t\t\tw_litlen_lens',start)
    emission=original[start:end]
    fixtures=Path(__file__).resolve().parents[3]/'hosts/gpu/tests/fixtures/native'
    headers=json.loads((fixtures/'deflate-header-reference.json').read_text())
    assert headers['reference']['source_pin']==PIN
    cases=[]
    for c in headers['header_cases']:
        for static in [False,True]:
            for carry in [False,True]:
                cases.append(dict(name=f"{c['name']}-{'static' if static else 'dynamic'}-{'carry' if carry else 'empty'}",
                    static=static,out=[99,17] if carry else [],bitbuf=5 if carry else 0,
                    bitcount=3 if carry else 0,final=0 if carry else 1,
                    litlen_lens=c['litlen_lens'],offset_lens=c['offset_lens'],
                    precode={k:v for k,v in c['expected'].items() if k!='cost'}))
    app='''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
import BlockOut
import DeflateTables
write = |out, bitbuf, bitcount, is_final, use_static, litlen_lens, offset_lens| {
    precode = BlockOut.precompute_huffman_header(litlen_lens, offset_lens)
    var $out = out
    var $bitbuf = bitbuf
    var $bitcount = bitcount
'''+emission+'''
    {out: $out, bitbuf: $bitbuf, bitcount: $bitcount}
}
main! = |mode| {
'''
    for i,c in enumerate(cases):
        app+=f'    if mode == {i} {{\n'
        for k in ['out','litlen_lens','offset_lens']:app+=literal(k,c[k],8)
        app+=f"        result = write(out, {c['bitbuf']}.U64, {c['bitcount']}.U64, {c['final']}.U64, {'True' if c['static'] else 'False'}, litlen_lens, offset_lens)\n"
        app+=print_list('out','result.out')
        for key in ['bitbuf','bitcount']:app+=f'        echo!("{key} ${{result.{key}.to_str()}}")\n'
        app+='    } else {}\n'
    app+='}\n'; work=args.work_dir.resolve();work.mkdir(parents=True,exist_ok=True)
    source=root/'package/MechBlockHeaderWriterOracle.roc';source.write_text(app);(work/source.name).write_text(app)
    binary=work/'block-header-writer-reference';command=[str(args.roc),'build','--opt=speed',f'--output={binary}',str(source)]
    subprocess.run(command,check=True)
    for i,c in enumerate(cases):
        raw=subprocess.check_output([str(binary),str(i)],text=True);(work/f'{i}.txt').write_text(raw)
        c['expected']={'out':[int(line.split()[1]) for line in raw.splitlines() if line.startswith('out ')]}
        for key in ['bitbuf','bitcount']:c['expected'][key]=int(next(line.split()[1] for line in raw.splitlines() if line.startswith(key+' ')))
    digest=lambda b:hashlib.sha256(b).hexdigest()
    args.out.write_text(pretty(dict(reference=dict(source_pin=PIN,compiler_pin=COMPILER_PIN,
        package_source_sha256={k:digest(v) for k,v in package.items()},emission_sha256=digest(emission.encode()),
        generated_source_sha256=digest(app.encode()),binary_sha256=digest(binary.read_bytes()),command=command),tables=headers['tables'],cases=cases))+'\n')
    print(json.dumps({'cases':len(cases)}))

if __name__=='__main__':main()

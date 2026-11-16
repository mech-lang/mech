#!/usr/bin/env python3
"""Execute the pinned stored-block loop and retain compact exact byte fixtures."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zlib
from generate_block_reference import pinned_package_sources, PIN, COMPILER_PIN
from generate_header_reference import literal, print_list
from generate_path_reference import pretty


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root',type=Path)
    parser.add_argument('--roc',type=Path,required=True)
    parser.add_argument('--work-dir',type=Path,required=True)
    parser.add_argument('--out',type=Path,required=True)
    args=parser.parse_args();root=args.reference_root.resolve();package=pinned_package_sources(root)
    original=package['package/BlockOut.roc'].decode()
    start=original.index('\t\t\tvar $storing = True',original.index('flush_block ='))
    end=original.index('\n\t\t\t{\n\t\t\t\tout:',start)
    loop=original[start:end]
    cases=[dict(name=f'final-{n}',length=n,begin=3,bitcount=0,bitbuf=0,final=1,out=[]) for n in [0,1,5,65535,65536]]
    cases += [dict(name=f'carry-{bc}',length=65536,begin=3,bitcount=bc,bitbuf=(1<<bc)-1,final=0,out=[99,17]) for bc in [3,5,6,7]]
    app='''app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }
import pf.Host exposing [echo!]
write = |out, bitbuf, bitcount, input, begin, length, is_final| {
    var $out = out
    var $bitbuf = bitbuf
    var $bitcount = bitcount
    var $in_next = begin
    in_end = begin + length
'''+loop+'''
    {out: $out, bitbuf: $bitbuf, bitcount: $bitcount}
}
main! = |mode| {
'''
    for i,c in enumerate(cases):
        app+=f'    if mode == {i} {{\n'+literal('out',c['out'],8)
        app+=f'''        var $input = List.with_capacity({c['begin']+c['length']+2})
        var $at = 0.U64
        while $at < {c['begin']+c['length']+2} {{
            $input = List.append($input, $at.to_u8_wrap())
            $at = $at + 1
        }}
        result = write(out, {c['bitbuf']}.U64, {c['bitcount']}.U64, $input, {c['begin']}.U64, {c['length']}.U64, {c['final']}.U64)
'''+print_list('out','result.out')
        for k in ['bitbuf','bitcount']:app+=f'        echo!("{k} ${{result.{k}.to_str()}}")\n'
        app+='    } else {}\n'
    app+='}\n';work=args.work_dir.resolve();work.mkdir(parents=True,exist_ok=True)
    source=root/'package/MechStoredWriterOracle.roc';source.write_text(app);(work/source.name).write_text(app)
    binary=work/'stored-writer-reference';command=[str(args.roc),'build','--opt=speed',f'--output={binary}',str(source)]
    subprocess.run(command,check=True)
    for i,c in enumerate(cases):
        raw=subprocess.check_output([str(binary),str(i)],text=True);(work/f'{i}.txt').write_text(raw)
        data=bytes(int(l.split()[1]) for l in raw.splitlines() if l.startswith('out '))
        (work/f'{i}.deflate').write_bytes(data)
        c['expected']={'length':len(data),'zlib_hex':zlib.compress(data,9).hex(),'sha256':hashlib.sha256(data).hexdigest()}
        for k in ['bitbuf','bitcount']:c['expected'][k]=int(next(l.split()[1] for l in raw.splitlines() if l.startswith(k+' ')))
        if c['final']==1 and c['bitcount']==0 and not c['out']:
            assert zlib.decompress(data,-15)==bytes(n%256 for n in range(c['begin'],c['begin']+c['length']))
            c['independent_decompression']=True
    digest=lambda b:hashlib.sha256(b).hexdigest()
    args.out.write_text(pretty(dict(reference=dict(source_pin=PIN,compiler_pin=COMPILER_PIN,package_source_sha256={k:digest(v) for k,v in package.items()},loop_sha256=digest(loop.encode()),generated_source_sha256=digest(app.encode()),binary_sha256=digest(binary.read_bytes()),command=command),input_pattern='byte at index i = i modulo256; input length=begin+length+2',fixture_encoding='zlib hex is only compact golden storage; decompress outside measured execution',cases=cases))+'\n')
    print(json.dumps({'cases':len(cases),'largest_payload':max(c['length'] for c in cases),'independently_decompressed':sum(c.get('independent_decompression',False) for c in cases)}))

if __name__=='__main__':main()

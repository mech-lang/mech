#!/usr/bin/env python3
"""Execute pinned compressor cost-model reference functions."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import generate_block_reference as pinned


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference_root', type=Path)
    parser.add_argument('--roc', required=True, type=Path)
    parser.add_argument('--work-dir', required=True, type=Path)
    parser.add_argument('--out', required=True, type=Path)
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parents[3]
    root = args.reference_root.resolve()
    compiler = args.roc.resolve()
    work = args.work_dir.resolve()
    work.mkdir(parents=True, exist_ok=True)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    package=pinned.pinned_package_sources(root)
    blocks=json.loads((workspace/'hosts/gpu/tests/fixtures/native/deflate-block-reference.json').read_text())
    cases=[dict(name='empty',input=[],begin=0,length=0,frequencies=[0]*259,depth=300)]
    for b in blocks['cases']:
     freqs=[0]*259
     for at,value in b['frequencies']:freqs[at]=value
     cases.append(dict(name=b['name'],input=b['input'],begin=b['scalars']['block-begin'],length=b['scalars']['block-end']-b['scalars']['block-begin'],frequencies=freqs,depth=300))
    for name,length,count in [('middle-table',64,3),('match-table',64,4)]:
     freqs=[0]*259;freqs[20]=count
     cases.append(dict(name=name,input=[i%4 for i in range(length)],begin=0,length=length,frequencies=freqs,depth=300))
    cases.append(dict(name='cutoff',input=[65]*4090+[1,2,3,4,5,6],begin=0,length=4096,frequencies=[0]*259,depth=300))
    cases.append(dict(name='full-alphabet',input=list(range(256)),begin=0,length=256,frequencies=[0]*259,depth=300))
    app='app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }\nimport pf.Host exposing [echo!]\nimport CompressOptimal\nmain! = |mode| {\n'
    for i,c in enumerate(cases):
     app+=f'    if mode == {i} {{\n        input = ['+', '.join(f'{v}.U8' for v in c['input'])+']\n        frequencies = ['+', '.join(f'{v}.U32' for v in c['frequencies'])+']\n'
     app+=f'        result = CompressOptimal.choose_default_litlen_costs(input, {c["begin"]}, {c["length"]}, frequencies, {c["depth"]})\n'
     app+='        echo!("${result.lit_cost.to_str()} ${result.len_sym_cost.to_str()}")\n    } else {}\n'
    app+='}\n';source=root/'package/MechDefaultCostsOracle.roc';source.write_text(app);(work/source.name).write_text(app);binary=work/'choose-costs-reference';command=[str(compiler),'build','--opt=speed',f'--output={binary}',str(source)];subprocess.run(command,check=True)
    for i,c in enumerate(cases):
     raw=subprocess.check_output([str(binary),str(i)],text=True);(work/(c['name']+'-choose.txt')).write_text(raw);c['expected']=list(map(int,raw.split()));assert len(c['expected'])==2
    source_text=package['package/CompressOptimal.roc'].decode();tables={key:[int(v) for v in re.findall(r'\d+',source_text.split(key+' = [',1)[1].split(']',1)[0])] for key in ['default_lit_cost_0','default_lit_cost_1','default_lit_cost_2','default_len_sym_costs']}
    assert [len(tables[k]) for k in tables]==[257,257,257,3]
    digest=lambda data:hashlib.sha256(data).hexdigest()
    result={'reference':{'source_pin':pinned.PIN,'compiler_pin':pinned.COMPILER_PIN,'package_source_sha256':{n:digest(d) for n,d in package.items()},'generated_source_sha256':digest(app.encode()),'binary_sha256':digest(binary.read_bytes()),'command':command},'tables':tables,'cases':cases};args.out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps([(c['name'],c['expected']) for c in cases]))


if __name__ == '__main__':
    main()

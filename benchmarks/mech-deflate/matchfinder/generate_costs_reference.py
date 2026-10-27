#!/usr/bin/env python3
"""Execute pinned compressor cost-model reference functions."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import generate_block_reference as module


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
    package=module.pinned_package_sources(root)
    tally=json.loads((workspace/'hosts/gpu/tests/fixtures/native/deflate-tally-reference.json').read_text())
    base={key:[((i*7919+17) ^ (4294967295 if i%7==0 else 0))&4294967295 for i in range(n)] for key,n in [('literal',256),('length',259),('offset_slot',32)]}
    cases=[]
    for lit,len_cost in [(6,109),(160,84)]:
     cases.append(dict(name=f'default-{lit}-{len_cost}',mode=0,lit_cost=lit,len_cost=len_cost,change=0,**base))
    for selected in [tally['cases'][1],tally['cases'][3]]:
     cases.append(dict(name=f'codes-{selected["name"]}',mode=1,lit_cost=0,len_cost=0,change=0,litlen_lens=selected['litlen_lens'],offset_lens=selected['offset_lens'],**base))
    cases.append(dict(name='codes-unused',mode=1,lit_cost=0,len_cost=0,change=0,litlen_lens=[0]*288,offset_lens=[0]*32,**base))
    for change in range(5):
     cases.append(dict(name=f'adjust-{change}',mode=2,lit_cost=144,len_cost=93,change=change,**base))
    app='app [main!] { pf: platform "../benchmark/echo-clock/main.roc" }\nimport pf.Host exposing [echo!]\nimport CompressOptimal\nmain! = |mode| {\n'
    for i,c in enumerate(cases):
     app+=f'    if mode == {i} {{\n'
     for key in ['literal','length','offset_slot']+(['litlen_lens','offset_lens'] if c['mode']==1 else []):
      width=8 if key.endswith('lens') else 32
      app+=f'        {key} = ['+', '.join(f'{v}.U{width}' for v in c[key])+']\n'
     if c['mode']==0: call=f'CompressOptimal.set_default_costs({c["lit_cost"]}, {c["len_cost"]}, literal, length, offset_slot)'
     elif c['mode']==1:call='CompressOptimal.set_costs_from_codes(litlen_lens, offset_lens, literal, length, offset_slot)'
     else:call=f'CompressOptimal.adjust_costs_impl({c["lit_cost"]}, {c["len_cost"]}, {c["change"]}, literal, length, offset_slot)'
     app+=f'        result = {call}\n'
     for key in ['literal','length','offset_slot']:
      app+=f'''        var $i_{key} = 0.U64
            while $i_{key} < result.{key}.len() {{
                echo!("{key} ${{(List.get(result.{key}, $i_{key}) ?? 0).to_str()}}")
                $i_{key} = $i_{key} + 1
            }}
    '''
     app+='    } else {}\n'
    app+='}\n'
    source=root/'package/MechCostsOracle.roc';source.write_text(app);(work/source.name).write_text(app)
    binary=work/'costs-reference';command=[str(compiler),'build','--opt=speed',f'--output={binary}',str(source)]
    subprocess.run(command,check=True)
    for i,c in enumerate(cases):
     raw=subprocess.check_output([str(binary),str(i)],text=True);(work/(c['name']+'.txt')).write_text(raw)
     c['expected']={key:[] for key in ['literal','length','offset_slot']}
     for line in raw.splitlines():
      key,value=line.split();c['expected'][key].append(int(value))
     assert [len(c['expected'][key]) for key in ['literal','length','offset_slot']]==[256,259,32]
    tables=package['package/DeflateTables.roc'].decode()
    arrays={key:[int(v) for v in re.findall(r'\d+',tables.split(key+' = [',1)[1].split(']',1)[0])] for key in ['length_slot_tab','extra_length_bits','extra_offset_bits']}
    digest=lambda data:hashlib.sha256(data).hexdigest()
    result={'reference':{'source_pin':module.PIN,'compiler_pin':module.COMPILER_PIN,'package_source_sha256':{n:digest(d) for n,d in package.items()},'generated_source_sha256':digest(app.encode()),'binary_sha256':digest(binary.read_bytes()),'command':command},'tables':arrays,'cases':cases}
    args.out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'cases':len(cases),'source_pin':module.PIN}))


if __name__ == '__main__':
    main()

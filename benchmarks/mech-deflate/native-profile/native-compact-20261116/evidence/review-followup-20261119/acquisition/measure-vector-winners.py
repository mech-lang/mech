"""Untimed conditional counters on the exact selected scorer's vector epilogue."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
import json,sys,inspect,struct,subprocess
N=Path(__file__).resolve().parent;Q=N.parent.parent
prefix=(N/'count-dp-forward.py').read_text().split('roles=sys.argv[1:]',1)[0]
ctx={'__file__':str(N/'count-dp-forward.py'),'__name__':'winner_counter_helpers'};exec(compile(prefix,str(N/'count-dp-forward.py'),'exec'),ctx)
h=run_path(str(Q/'count-matcher-instructions.py'),run_name='conditional_scorer_counters')
root=N/'word-current-complements-profiled/canonical-counter-input/bound';out=N/'vector-winner-frequency';out.mkdir(exist_ok=True)
symbol=ctx['driver'].SCORER;meta=json.loads((root/'metadata.json').read_text());phase=h['phase_metadata'](meta,symbol)[2]
base,sections,symbols=h['macho'].load(root/'program.o');start=h['macho'].text_symbols(sections,symbols)[symbol];size=phase['code_bytes'];atom=ctx['study'].rebase.parse('word-current-complements-profiled');pool=atom['pool'];end=start+(pool if pool is not None else size);asm=h['instructions'](root/'program.o',start,end,allow_data_references=pool is not None)
pcs=list(asm);sites=[]
for i,pc in enumerate(pcs):
 op,args=asm[pc]
 if op!='csel' or not args.endswith(', ne') or i<3:continue
 prior=[asm[x] for x in pcs[max(0,i-15):i]]
 if prior[-1][0]=='cmp' and any(op.startswith('uminv') for op,_ in prior):sites.append(pc)
assert len(sites)==1,sites
site=sites[0]
# Assemble the condition increment rather than guessing an instruction encoding.
(out/'predicate.s').write_text('.text\n.globl _predicate\n_predicate:\n cinc x17, x17, ne\n ret\n')
subprocess.run(['/usr/bin/clang','-arch','arm64','-c',str(out/'predicate.s'),'-o',str(out/'predicate.o')],check=True)
b,sec,_=h['macho'].load(out/'predicate.o');off=sec[h['text']][3];increment=struct.unpack_from('<I',b,off)[0]
assert increment==0x9a910631
code=inspect.getsource(h['instrument']);code=code.replace('    leaders = sorted(leaders)','    leaders.add(winner_site)\n    leaders = sorted(leaders)')
needle='        mapped[pc] = start + len(output) * 4'
extra='''        if pc == winner_site:
            at = start + len(output) * 4
            new_relocs.extend(((at + 4, 0x5D000000 | n), (at + 8, 0x6C000000 | n)))
            # The same IP-register save/restore as block probes. CINC and the
            # other instructions preserve NZCV for the original CSEL.
            index = 1023
            output.extend((0xa9bf47f0, 0x90000010, 0xf9400210, 0xf9400211 | (index << 10),
                           conditional_increment, 0xf9000211 | (index << 10), 0xa8c147f0))
'''+needle
assert code.count(needle)==1;code=code.replace(needle,extra);h['instrument'].__globals__.update(winner_site=site,conditional_increment=increment);exec(compile(code,str(out/'instrument-source.py'),'exec'),h['instrument'].__globals__);instrument=h['instrument'].__globals__['instrument'];(out/'instrument-source.py').write_text(code)
assembly,leaders,body_end=instrument(root/'program.o',size,out/'program.o',symbol,pool);assert len(leaders)<1023
worker=h['link_worker'](root,out,out/'program.o');total_slot=leaders.index(site)
report={};cases=['prefix-natural-1024','prefix-natural-4096','prefix-natural-8192','prefix-random-4096','prefix-random-8192','prefix-random-1048576','16384-distribution-change']
for case in cases:
 result=subprocess.run([str(worker),str(root/'metadata.json'),str(Q/'fixtures.json')],input=f"C\t{h['indices'][case]}\t0\nNNQ\n",text=True,capture_output=True,check=True);assert result.stderr.endswith('CLEANUP private_payload_bytes=0 live_owner_records=0\n');rows=[json.loads(x) for x in result.stdout.splitlines() if x.startswith('{')];assert len(rows)==2
 assert all(all(row[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for row in rows)
 (out/('observations-'+case+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
 a,b=[x['block_counts'] for x in rows];assert a==b;total=a[total_slot];wins=a[1023];assert 0<=wins<=total
 report[case]={'vector_reductions':total,'vector_winners':wins,'fraction':wins/total if total else None,'verified_calls':2};print(case,report[case],flush=True)
record={'source_object_sha256':sha256((root/'program.o').read_bytes()).hexdigest(),'shipping_worker_sha256':sha256((N/'word-current-complements-profiled/bound/mech-worker').read_bytes()).hexdigest(),'diagnostic_object_sha256':sha256((out/'program.o').read_bytes()).hexdigest(),'symbol':symbol,'leaders':[hex(pc) for pc in leaders],'site':hex(site),'original_instruction':asm[site],'predicate':'NE after vector min-index sentinel comparison; true means the vector reduction supplies a replacement winner. This is per reduction, not per SIMD chunk.','cases':report,'scope':'Diagnostic clone only; no latency values or instruction totals are inferred from conditional slots. All shipping code remains source emitted.'};(out/'result.json').write_text(json.dumps(record,indent=2)+'\n')

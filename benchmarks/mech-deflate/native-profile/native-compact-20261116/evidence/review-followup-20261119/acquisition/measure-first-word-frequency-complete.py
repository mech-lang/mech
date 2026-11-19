"""Untimed first-word failure counters on the source-emitted incremental probe."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
import json,inspect,subprocess,struct
N=Path(__file__).resolve().parent;Q=N.parent.parent
prefix=(N/'count-packet-matcher.py').read_text().split('source_shapes={}',1)[0]
ctx={'__file__':str(N/'count-packet-matcher.py'),'__name__':'first_word_counter_helpers'};exec(compile(prefix,str(N/'count-packet-matcher.py'),'exec'),ctx)
h=run_path(str(Q/'count-matcher-instructions.py'),run_name='conditional_first_word_counters')
role='first-word-complements';root=N/role/'canonical-counter-input/bound';out=N/'first-word-frequency-complete';out.mkdir(exist_ok=True)
symbol='_mech_compact_numeric_475_vector';meta=json.loads((root/'metadata.json').read_text());phase=h['phase_metadata'](meta,symbol)[2]
base,sections,symbols=h['macho'].load(root/'program.o');start=h['macho'].text_symbols(sections,symbols)[symbol];size=phase['code_bytes'];shape=ctx['shape'](role);pool=shape['pool'];end=start+(pool if pool is not None else size);asm=h['instructions'](root/'program.o',start,end,allow_data_references=True)
pcs=list(asm);sites={}
for i,pc in enumerate(pcs):
 op,args=asm[pc]
 if op!='cbnz' or i<5:continue
 prior=[asm[p] for p in pcs[i-5:i]]
 if prior[-1][0]=='eor' and prior[-1][1].startswith('x') and sum(op=='ldr' and a.startswith('x') for op,a in prior)==2:
  reg=args.split(',')[0].strip();assert reg not in ('x16','x17') and reg.startswith('x')
  sites[pc]={'register':reg,'slot':1022+len(sites)}
assert len(sites)==2,sites
probe_sites=list(sites)
for i,pc in enumerate(probe_sites):
 index=pcs.index(pc)+1
 following=pcs[index:index+12]
 guards=[x for j,x in enumerate(following) if asm[x][0]=='b.lo' and j and asm[following[j-1]][0]=='cmp' and asm[following[j-1]][1].endswith(', #0x10')]
 assert len(guards)==1,(hex(pc),[(hex(x),asm[x]) for x in following])
 guard=guards[0];assert not any(asm[x][0].startswith(('b','cb','tb')) for x in pcs[index:pcs.index(guard)])
 sites[pc]['length_guard']=guard
 sites[guard]={'slot':1020+i,'kind':'short_tail','probe':pc}
(out/'predicate.s').write_text('.text\n.globl _predicate\n_predicate:\n cinc x17, x17, lo\n ret\n')
subprocess.run(['/usr/bin/clang','-arch','arm64','-c',str(out/'predicate.s'),'-o',str(out/'predicate.o')],check=True)
b,sec,_=h['macho'].load(out/'predicate.o');lo_increment=struct.unpack_from('<I',b,sec[h['text']][3])[0]
leaf=inspect.getsource(h['leaf_code']).replace('allow_data_references=literal_start is not None','allow_data_references=True');exec(compile(leaf,str(out/'leaf-source.py'),'exec'),h['instrument'].__globals__);(out/'leaf-source.py').write_text(leaf)
code=inspect.getsource(h['instrument']);code=code.replace('    leaders = sorted(leaders)','    leaders.update(probe_sites)\n    leaders = sorted(leaders)')
needle='        mapped[pc] = start + len(output) * 4'
extra='''        if pc in probe_sites:
            site = probe_sites[pc]
            at = start + len(output) * 4
            new_relocs.extend(((at + 4, 0x5D000000 | n), (at + 8, 0x6C000000 | n)))
            index = site['slot']
            register = int(site.get('register','x0')[1:])
            # CBZ skips only the increment when the loaded XOR is zero.
            # This sequence preserves NZCV and the original branch operand.
            output.extend((0xa9bf47f0, 0x90000010, 0xf9400210, 0xf9400211 | (index << 10)))
            if site.get('kind') == 'short_tail':
                output.append(conditional_lo_increment)
            else:
                output.extend((0xb4000040 | register, 0x91000631))
            output.extend((0xf9000211 | (index << 10), 0xa8c147f0))
'''+needle
assert code.count(needle)==1;code=code.replace(needle,extra);h['instrument'].__globals__.update(probe_sites=sites,conditional_lo_increment=lo_increment);exec(compile(code,str(out/'instrument-source.py'),'exec'),h['instrument'].__globals__);instrument=h['instrument'].__globals__['instrument'];(out/'instrument-source.py').write_text(code)
assembly,leaders,body_end=instrument(root/'program.o',size,out/'program.o',symbol,pool);assert len(leaders)<1020
worker=h['link_worker'](root,out,out/'program.o');report={}
training=('prefix-natural-1024','prefix-natural-4096','prefix-natural-8192','prefix-random-4096','prefix-random-8192','prefix-random-1048576','16384-distribution-change')
for case in training:
 result=subprocess.run([str(worker),str(root/'metadata.json'),str(Q/'fixtures.json')],input=f"C\t{h['indices'][case]}\t0\nNNQ\n",text=True,capture_output=True,check=True);assert result.stderr.endswith('CLEANUP private_payload_bytes=0 live_owner_records=0\n');rows=[json.loads(x) for x in result.stdout.splitlines() if x.startswith('{')];assert len(rows)==2
 assert all(all(row[k] for k in ('exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged')) for row in rows)
 (out/('observations-'+case+'.json')).write_text(json.dumps(rows,indent=2)+'\n')
 a,b=[x['block_counts'] for x in rows];assert a==b;measured=[]
 for pc in probe_sites:
  site=sites[pc]
  total=a[leaders.index(pc)];fail=a[site['slot']];assert 0<=fail<=total
  guard=site['length_guard'];gtotal=a[leaders.index(guard)];short=a[sites[guard]['slot']];assert gtotal==total-fail and 0<=short<=gtotal
  measured.append({'short_scalar_tails':short,'vector_entries_after_first_word':gtotal-short,'site':hex(pc),'probes':total,'first_word_mismatches':fail,'equal_first_words':total-fail,'failure_fraction':fail/total if total else None})
 report[case]={'sites':measured,'probes':sum(x['probes'] for x in measured),'first_word_mismatches':sum(x['first_word_mismatches'] for x in measured),'verified_calls':2};print(case,report[case],flush=True)
record={'role':role,'source_object_sha256':sha256((root/'program.o').read_bytes()).hexdigest(),'shipping_worker_sha256':sha256((N/role/'bound/mech-worker').read_bytes()).hexdigest(),'diagnostic_object_sha256':sha256((out/'program.o').read_bytes()).hexdigest(),'symbol':symbol,'leaders':[hex(pc) for pc in leaders],'sites':{hex(k):v for k,v in sites.items()},'cases':report,'scope':'Conditional diagnostic counters only. The exact following remaining-length guard separates vector entries from short scalar tails. Counts are per call and per probe, rather than vector iterations. No latency or instruction totals inferred.'};(out/'result.json').write_text(json.dumps(record,indent=2)+'\n')

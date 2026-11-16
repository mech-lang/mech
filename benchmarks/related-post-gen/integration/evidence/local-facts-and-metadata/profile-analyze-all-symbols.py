from pathlib import Path
import bisect,collections,hashlib,importlib.util,json,re,struct,subprocess
P=Path(__file__).parent;R=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
spec=importlib.util.spec_from_file_location('audit',R/'audit-native-link.py');audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
read=lambda p:json.loads(Path(p).read_text())
out={}
for label in ['before','after']:
    d=P/label;stats=read(d/'output.json.pc.json');assert stats['post_json_only'] and stats['last_phase_epoch']==2000
    raw=list(struct.iter_unpack('<7Q',(d/'output.json.pc.bin').read_bytes()));assert len(raw)==stats['records']
    resolutions={}
    for line in (d/'output.json.pc.bin.symbols.tsv').read_text().splitlines():
        pc,image,address,path,name=line.split('\t');value=dict(image_base=int(image,16),symbol_address=int(address,16),image=path,symbol=name)
        key=int(pc,16);assert key not in resolutions or resolutions[key]==value;resolutions[key]=value
    _,sections,symbols=audit.macho(d/'runner-pc');section=next(s for s in sections if s['name']=='__text')
    own_symbols=sorted([s for s in symbols if s['section']and sections[s['section']-1]['name']=='__text'],key=lambda s:s['value']);starts=[s['value']for s in own_symbols]
    counts=collections.Counter();groups=collections.Counter();lr_counts=collections.Counter()
    phases=collections.Counter()
    for ns,thread,pc,lr,sp,fp,phase in raw:
        assert phase&1 and 1<=phase<=1999
        phases[phase]+=1
        normalized=pc-stats['image_base']+0x100000000
        if section['address']<=normalized<section['address']+section['size']:
            i=bisect.bisect_right(starts,normalized)-1;assert i>=0
            name=own_symbols[i]['name'];path=str(d/'runner-pc')
            if re.search(r'_numeric_\d+$',name):group='outlined_numeric_native'
            elif name=='_mech_canonical_program_entry_v1':group='root_native'
            elif name.startswith('_mech_canonical_program_entry_v1'):group='other_emitted_native'
            else:group='other_executable_code'
        else:
            resolved=resolutions[pc];path=resolved['image'];name=resolved['symbol'];assert resolved['symbol_address']<=pc
            if any(s in name for s in ['ulock_wait','psynch_cvwait','psynch_mutexwait','mach_msg','workq_kernreturn','__semwait']):group='system_wait_position'
            elif any(s in name for s in ['malloc','free','xzone','realloc','calloc','nanov2','xzm_']):group='system_allocator'
            elif any(s in name for s in ['pthread','thread_start','bsdthread','thread_selfid']):group='system_thread_management'
            elif name=='-':group='unresolved_external'
            else:group='other_system_code'
            address=lr-stats['image_base']+0x100000000
            if section['address']<=address<section['address']+section['size']:
                i=bisect.bisect_right(starts,address)-1
                if i>=0:lr_counts[own_symbols[i]['name']]+=1
        counts[path,name]+=1;groups[group]+=1
    assert sum(groups.values())==len(raw)
    names=[name[1:]if name.startswith('__R')else name for path,name in counts]
    demangled=subprocess.check_output(['/opt/homebrew/opt/llvm/bin/llvm-cxxfilt'],input='\n'.join(names)+'\n',text=True).splitlines();assert len(names)==len(demangled)
    mapping={key:name for key,name in zip(counts,demangled)}
    row=dict(records=len(raw),all_resolved_symbols_from_same_live_process=True,unresolved_external_records=groups.get("unresolved_external",0),observed_post_json_epochs=len(phases),phase_range=[min(phases),max(phases)],record_bytes=56,raw_sha256=hashlib.sha256((d/'output.json.pc.bin').read_bytes()).hexdigest(),observation_categories=dict(groups),top_symbols=[dict(image=path,symbol=name,demangled=mapping[path,name],observations=count)for (path,name),count in counts.most_common(70)],external_positions_with_own_executable_lr=lr_counts.most_common(30))
    out[label]=row
    print(label,'categories',dict(groups),flush=True)
    for x in row['top_symbols'][:20]:print(x['observations'],x['demangled'],flush=True)
report=dict(schema='mech.related-post-gen.post-json-all-symbol-observations.v1',compilers={label:read(P/label/'summary.json')['compiler']for label in out},all2000_invocations_exact=True,profiled_timings_excluded=True,rows=out,limitations=['These are statistical current-PC observations, not CPU-time shares or dynamic instruction counts. Waiting threads can retain a wait PC while consuming no CPU.','A single odd epoch must surround each accepted register observation; sampling across a phase boundary is discarded.','JSON decoding, output verification and teardown occur outside the gate. Restored program and original JSON text are reused.','System names are resolved after sampling in the same live process; executable-local names use the exact profiled Mach-O.'])
(P/'all-symbol-observations.json').write_text(json.dumps(report,indent=2)+'\n')

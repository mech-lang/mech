from pathlib import Path
from collections import defaultdict
import bisect, importlib.util, json, re, subprocess
O=Path(__file__).parent
ROOT=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
spec=importlib.util.spec_from_file_location('audit',ROOT/'audit-native-link.py');audit=importlib.util.module_from_spec(spec);spec.loader.exec_module(audit)
result={}
def rank(group,limit=30):
    return [dict(symbol=k,requests=v[0],requested_bytes=v[1])for k,v in sorted(group.items(),key=lambda x:(-x[1][0],-x[1][1],x[0]))[:limit]]
for name,w,requests,bytes_ in [('plain',4,2945,7919355),('both-rows',4,2945,8006235)]:
    p=O/name;data=[json.loads(x)for x in (p/'allocations.jsonl').read_text().splitlines()];header,events=data[0],data[1:]
    assert len(events)==header['recorded']==requests and sum(x['bytes']for x in events)==bytes_
    assert not header['buffer_dropped']and not header['reentrant_dropped']
    assert all(0<len(x['stack'])<header['max_frames']and all(x['stack'])for x in events)
    _,sections,symbols=audit.macho(p/'runner-profile');text=next(s for s in sections if s['name']=='__text')
    raw=subprocess.check_output(['/opt/homebrew/opt/llvm/bin/llvm-nm','--defined-only','--numeric-sort','--demangle',str(p/'runner-profile')],text=True)
    (p/'symbols.txt').write_text(raw);demangled={}
    for line in raw.splitlines():
        m=re.match(r'^([0-9a-fA-F]+)\s+[tT]\s+(.*)$',line)
        if m:demangled[int(m[1],16)]=m[2]
    symbols=sorted((s for s in symbols if s['section']and sections[s['section']-1]['name']=='__text'),key=lambda s:s['value']);starts=[s['value']for s in symbols]
    def symbol(address):
        # The backtrace stores return PCs; a boundary return belongs to the
        # calling instruction immediately before it, not the next symbol.
        address=address-header['image_load_address']+0x100000000-1
        if text['address']<=address<text['address']+text['size']:
            i=bisect.bisect_right(starts,address)-1
            if i>=0:return demangled.get(starts[i],symbols[i]['name'])
        return None
    leaf=defaultdict(lambda:[0,0]);inclusive=defaultdict(lambda:[0,0]);stacks=defaultdict(lambda:[0,0]);roles={k:dict(requests=0,requested_bytes=0)for k in ['root','workers']};threads={}
    role_inclusive={role:defaultdict(lambda:[0,0])for role in ['root','workers']}
    own=0;unattributed=0
    for event in events:
        frames=[s for a in event['stack']if (s:=symbol(a))is not None]
        frames=[s for s in frames if 'allocation_profile::'not in s and not s.startswith('__rustc')]
        role='root'if event['thread']==header['root_thread']else'workers'
        roles[role]['requests']+=1;roles[role]['requested_bytes']+=event['bytes']
        tid=str(event['thread']);threads.setdefault(tid,dict(role=role,requests=0,requested_bytes=0));threads[tid]['requests']+=1;threads[tid]['requested_bytes']+=event['bytes']
        if not frames:unattributed+=1;continue
        own+=1;leaf[frames[0]][0]+=1;leaf[frames[0]][1]+=event['bytes']
        key=' -> '.join(frames[:5]);stacks[key][0]+=1;stacks[key][1]+=event['bytes']
        for s in set(frames):
            inclusive[s][0]+=1;inclusive[s][1]+=event['bytes'];role_inclusive[role][s][0]+=1;role_inclusive[role][s][1]+=event['bytes']
    row=dict(header=header,exact_standard_rust_output=True,requests=requests,requested_bytes=bytes_,counts_and_bytes_equal_candidate_benchmark_resource_counters=True,roles=roles,threads=threads,own_executable_attributed_requests=own,unattributed_requests=unattributed,top_allocation_symbols=rank(leaf,45),top_inclusive_symbols=rank(inclusive,70),top_call_stacks=rank(stacks,45),all_inclusive_symbols=rank(inclusive,len(inclusive)),root_inclusive_symbols=rank(role_inclusive['root'],len(role_inclusive['root'])),worker_inclusive_symbols=rank(role_inclusive['workers'],len(role_inclusive['workers'])))
    (p/'summary.json').write_text(json.dumps(row,indent=2)+'\n');result[name]=row
    print(name,'roles',json.dumps(roles),'unattributed',unattributed)
    for x in row['top_allocation_symbols'][:12]:print(x['requests'],x['requested_bytes'],x['symbol'][:260])
    print('setup call stacks:')
    for x in row['all_inclusive_symbols']:
        if any(k in x['symbol']for k in ['new_with_budget','run_worker','clone','histogram','prepare_numeric','retain','call_cache','BufferFrames::new']):print(x['requests'],x['requested_bytes'],x['symbol'][:220])
(O/'summary.json').write_text(json.dumps(dict(schema='mech.related-post-gen.allocation-stack-profile.v1',compiler='ccfadbdf0af69f741237f5d1099a3c3eef79386a',identity='IR280/work116/ABI41/image11',adapter='Current production descriptor-restoring AOT adapter; only GlobalAlloc diagnostic instrumentation added',diagnostic_timings_excluded=True,method='Capture every successful Rust GlobalAlloc alloc/alloc_zeroed/realloc request inside the checked continuous post-JSON input-admission/native-execution/output-materialization window into a fixed bounded array. System backtrace supplies raw return PCs. No event/reentrant loss or stack truncation. Map only own-executable text with verified Mach-O symbols and ASLR slide.',limitations=['Counts cover continuous post-JSON requests and requested bytes, not CPU time or live/peak memory.','GlobalAlloc hooks cover Rust requests, matching existing resource counters; direct C allocator calls are outside both measurements.','Inclusive stack counts overlap and must not be added. Inlining may attribute an allocation to a containing function.','Backtrace overhead and the fixed diagnostic buffer change timings and memory; these runs are excluded from latency/resource comparisons.'],variants=result),indent=2)+'\n')

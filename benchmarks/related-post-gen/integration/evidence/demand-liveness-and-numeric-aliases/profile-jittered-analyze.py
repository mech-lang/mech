from pathlib import Path
import json,struct,bisect,collections,re
P=Path(__file__).parent
for label in ['selected-ir242','candidate-ir245']:
 p=P/label;stats=json.loads((p/'output.json.sampler.json').read_text());data=(p/'output.json.samples.bin').read_bytes();fmt=struct.Struct('<5Q2I64Q');assert fmt.size==stats['record_bytes'] and len(data)==fmt.size*stats['records']
 names=[]
 for line in (p/'nm.demangled.txt').read_text().splitlines():
  m=re.match(r'^([0-9a-fA-F]+)\s+([Tt])\s+(.*)$',line)
  if m:names.append((int(m.group(1),16),m.group(3)))
 names.sort();starts=[x[0] for x in names];syms={}
 for line in (p/'output.json.symbols.tsv').read_text().splitlines():
  x=line.split('\t',4)
  if len(x)==5:syms[int(x[0])]=(int(x[1]),int(x[2]),x[3],x[4])
 counts=collections.Counter();categories=collections.Counter();native=collections.Counter();unknown=0
 for offset in range(0,len(data),fmt.size):
  record=fmt.unpack_from(data,offset);pc=record[1];info=syms.get(pc)
  if not info:symbol='unresolved';unknown+=1
  elif info[3].endswith('/runner-sampler'):
   addr=pc-info[0]+0x100000000;idx=bisect.bisect_right(starts,addr)-1;symbol=names[idx][1] if idx>=0 else info[2]
  else:symbol=info[2] or info[3]
  counts[symbol]+=1
  if 'mech_canonical_program_entry_v1' in symbol:category='emitted_native_code';native[symbol]+=1
  elif any(x in symbol.lower() for x in ['ulock_wait','__psynch','semaphore_wait','mach_msg','swtch_pri','thread_switch']):category='waiting_or_kernel_message'
  elif any(x in symbol.lower() for x in ['malloc','free','alloc::alloc','_realloc','memcpy','memmove','bzero','madvise']):category='allocator_or_memory_copy'
  elif 'mech_' in symbol:category='mech_host_code'
  elif 'sam_' in symbol:category='sampling_boundary'
  else:category='other_host_or_system'
  categories[category]+=1
 summary=dict(scope='Main-thread wall-clock snapshots inside post-JSON only; workers omitted; blocking and disturbance retained. Not all-thread CPU-time shares.',records=stats['records'],jitter_seed=0x990245,interval_min_ns=250000,interval_max_ns=1500000,categories={k:dict(samples=v,percent=100*v/stats['records']) for k,v in categories.most_common()},top_leaf_symbols=[dict(symbol=k,samples=v,percent=100*v/stats['records']) for k,v in counts.most_common(30)],native_functions=[dict(symbol=k,samples=v,percent=100*v/stats['records']) for k,v in native.most_common()],unresolved=unknown,sampler_zero_failures=all(stats[k]==0 for k in ['suspend_failures','state_failures','resume_failures']),suspend_resume_balanced=stats['suspended']==stats['resumed'])
 (p/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(label,json.dumps({k:summary[k] for k in ['records','categories','sampler_zero_failures','suspend_resume_balanced']}));print('native',summary['native_functions'][:6])

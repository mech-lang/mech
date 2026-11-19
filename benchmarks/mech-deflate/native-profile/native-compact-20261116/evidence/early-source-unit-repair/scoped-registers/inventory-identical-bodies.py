from pathlib import Path
import sys,json,hashlib,collections
from struct import unpack_from
W=Path('/Users/coreymontella/.codex/worktrees/pr995-runtime-integration/mech-shared-cold-epilogues.2L6obP');sys.path.insert(0,str(W/'benchmarks/mech-deflate/native-profile/compact-size'));import mach_o as m
Q=Path('/private/tmp/mech-shared-cold-epilogues.2L6obP/.qualification/consolidation-995/controller-source-boundaries');folder=Q/'scoped-registers/early-owner-output-root-preserved';o=folder/'frontend/program.o';data,sections,symbols=m.load(o);starts=m.text_symbols(sections,symbols);_,text_pc,text_size,text_at,rel_at,nrel=sections[m.TEXT];relocs=[unpack_from('<II',data,rel_at+8*i) for i in range(nrel)];metadata=json.loads((o.parent/'metadata.json').read_text());rows=[]
for unit in [metadata,metadata['early_source_region']['helper']]:
 for field,kind in [('layout_checked_controllers','layout'),('checked_continuations','layout_continuation')]:
  for group in unit.get(field,[]):
   names=['_mech_compact_checked_'+kind+'_'+str(h) for h in group['headers']];found=[name for name in names if name in starts];assert len(found)==1,(names,found)
   name=found[0];start=starts[name]-text_pc;length=group['code_bytes'];assert 0<=start<start+length<=text_size;code=data[text_at+start:text_at+start+length];assert len(code)==length;ref=[]
   for at,bits in relocs:
    if start<=at<start+length:
     target=symbols[bits&0xffffff][0] if bits&(1<<27) else bits&0xffffff
     ref.append((at-start,bits>>24,'self' if target==name else target))
   rows.append({'unit':unit['symbol'],'name':name,'bytes':length,'stack':group['stack_bytes'],'code_sha256':hashlib.sha256(code).hexdigest(),'relocations':ref})
groups=collections.defaultdict(list)
for row in rows:groups[(row['code_sha256'],json.dumps(row['relocations']))].append(row)
duplicates=[g for g in groups.values() if len(g)>1];result={'object_sha256':hashlib.sha256(data).hexdigest(),'scope':'Read-only inventory of identical finalized checked helper bytes and target relocations; signatures/address-use eligibility still require compiler proofs. No bytes are edited or timed.','rows':rows,'duplicate_groups':duplicates,'potential_body_bytes':sum(sum(x['bytes'] for x in g[1:]) for g in duplicates)}
(Q/'scoped-registers/identical-checked-bodies.json').write_text(json.dumps(result,indent=2)+'\n');print('Identical code and relocation groups:',[(g[0]['bytes'],[x['name'] for x in g]) for g in duplicates]);print('potential body bytes',result['potential_body_bytes'])

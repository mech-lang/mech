"""Canonical section ordering for diagnostic counters only; preserve every code/data byte and relocation target."""
from pathlib import Path
from hashlib import sha256
from runpy import run_path
from struct import unpack_from,pack_into
import json,sys
ROOT=Path(__file__).resolve().parent;L=ROOT/'immutable-only';Q=ROOT.parent.parent;m=run_path(str(Q/'reachability-census.py'),run_name='counter_input_macho');read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest();text=m['TEXT'];const=(b'__TEXT',b'__const');align=lambda n,a:(n+a-1)&-a

def normalize(source,dest):
 data,sections,symbols=m['load'](source);assert set(sections)=={text,const};commands={};at=32
 for _ in range(unpack_from('<I',data,16)[0]):
  kind,size=unpack_from('<II',data,at);commands[kind]=at;at+=size
 assert set(commands)=={0x19,2,11};segment=commands[0x19];assert unpack_from('<I',data,segment+64)[0]==2
 headers={}
 for i in range(2):
  h=segment+72+80*i;name,seg=unpack_from('<16s16s',data,h);headers[seg.rstrip(b'\0'),name.rstrip(b'\0')]=h
 ti,ta,ts,to,tr,tn=sections[text];ci,ca,cs,co,cr,cn=sections[const];assert cn==0
 if ta==0:
  dest.write_bytes(data);return {'identity':True,'source_sha256':sha(source),'diagnostic_sha256':sha(dest)}
 assert ca==0 and ci==1 and ti==2
 syms,n,strings,stringlen=unpack_from('<4I',data,commands[2]+8);assert n==len(symbols) and syms+16*n==strings and strings+stringlen==len(data)
 assert not any(unpack_from('<12I',data,commands[11]+32))
 relocs=[unpack_from('<II',data,tr+8*i) for i in range(tn)]
 for site,flags in relocs:
  assert site+4<=ts
  assert flags>>28==10 or flags&(1<<27),'Only symbol-relative relocations and paired addends are admitted.'
 start=32+unpack_from('<I',data,20)[0];talign=1<<unpack_from('<I',data,headers[text]+52)[0];calign=1<<unpack_from('<I',data,headers[const]+52)[0];newto=align(start,talign);newca=align(ts,calign);newco=align(newto+ts,calign);assert newco-newto==newca
 newtr=align(newco+cs,8);newsym=newtr+8*tn;newstrings=newsym+16*n;out=bytearray(newstrings+stringlen);out[:start]=data[:start]
 for index,key,newaddr,newoff,newreloc in [(0,text,0,newto,newtr),(1,const,newca,newco,0)]:
  h=segment+72+80*index;out[h:h+80]=data[headers[key]:headers[key]+80];pack_into('<Q',out,h+32,newaddr);pack_into('<I',out,h+48,newoff);pack_into('<I',out,h+56,newreloc)
 pack_into('<Q',out,segment+24,0);pack_into('<Q',out,segment+32,newca+cs);pack_into('<Q',out,segment+40,newto);pack_into('<Q',out,segment+48,newco+cs-newto)
 out[newto:newto+ts]=data[to:to+ts];out[newco:newco+cs]=data[co:co+cs];out[newtr:newtr+8*tn]=data[tr:tr+8*tn];out[newsym:newsym+16*n]=data[syms:syms+16*n];out[newstrings:]=data[strings:];pack_into('<4I',out,commands[2]+8,newsym,n,newstrings,stringlen)
 for i,(_,_,section,pc) in enumerate(symbols):
  h=newsym+16*i
  if section==ti:out[h+5]=1;pack_into('<Q',out,h+8,pc-ta)
  elif section==ci:out[h+5]=2;pack_into('<Q',out,h+8,pc-ca+newca)
  else:assert section==0
 dest.write_bytes(out);actual,ss,sy=m['load'](dest);assert actual[ss[text][3]:ss[text][3]+ts]==data[to:to+ts];assert actual[ss[const][3]:ss[const][3]+cs]==data[co:co+cs]
 assert [x[:2] for x in symbols]==[x[:2] for x in sy]
 for old,new in zip(symbols,sy):
  if old[2]:
   key=text if old[2]==ti else const;assert old[3]-sections[key][1]==new[3]-ss[key][1]
 assert actual[ss[text][4]:ss[text][4]+8*tn]==data[tr:tr+8*tn]
 return {'identity':False,'source_sha256':sha(source),'diagnostic_sha256':sha(dest),'code_bytes_unchanged':ts,'constant_bytes_unchanged':cs,'relocations_unchanged':tn,'source_sections':{str(k):v for k,v in sections.items()},'diagnostic_sections':{str(k):v for k,v in ss.items()},'scope':'Only diagnostic Mach-O section ordering, virtual bases, file offsets and corresponding symbol section/value coordinates change. Every text/data byte and each symbol-relative relocation target/offset is preserved. Not a shipping artifact and never used for latency.'}
link=run_path(str(Q/'current-roadmap/checked-root-census/recheck-link.py'),run_name='canonical_counter_link')
for role in ['immutable-profiled']:
 root=L/role;front=root/'canonical-counter-input/frontend';front.mkdir(parents=True);receipt=normalize(root/'frontend/program.o',front/'program.o')
 for name in ['metadata.json','program.mecb']:(front/name).write_bytes((root/'frontend'/name).read_bytes())
 (front.parent/'canonicalization.json').write_text(json.dumps(receipt,indent=2)+'\n');bound=front.parent/'bound';link['link_worker'](root/'bound',front,bound);link['check_worker'](bound,'immutable-only-canonical-'+role);print('Canonical counter input verified:',role,flush=True)

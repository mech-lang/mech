"""Rebase the existing three profiles only after exact ordered machine-control-graph equality."""
from pathlib import Path
from hashlib import sha256
from struct import unpack_from
from runpy import run_path
import json,os,subprocess,sys
ROOT=Path(__file__).resolve().parent;LAB=ROOT.parent;OUT=ROOT/'retest-register-spills';Q=LAB.parent
read=lambda p:json.loads(p.read_text());sha=lambda p:sha256(Path(p).read_bytes()).hexdigest()
sys.path.insert(0,str(Q));sys.path.insert(0,str(Q/'current-roadmap/scorer-runtime'));import emit
r=run_path(str(Q/'current-roadmap/scorer-runtime/rebase-layout.py'),run_name='current_pair_graph_tools')
def info(meta,symbol):
 rows=[]
 for p in meta['numeric_phases']:
  for name in [None,'vector_variant','no_growth_variant','composed_variant','grouped_histogram']:
   row=p if name is None else p.get(name,{})
   if row.get('symbol')==symbol:rows.append(row)
 assert len(rows)==1,(symbol,len(rows));return rows[0]
def shape(role,symbol):
 folder=OUT/role;record=read(folder/'receipt.json');atom=record['layout']['symbols']['_'+symbol];meta=info(read(folder/'frontend/metadata.json'),symbol)
 image,sections,_=emit.macho.load(folder/'frontend/program.o');_,base,_,offset,*_=sections[emit.macho.TEXT];begin=offset+atom['start']-base;refs=[];words={}
 for at in range(0,meta['code_bytes'],4):
  if refs and at>=min(refs):break
  w=unpack_from('<I',image,begin+at)[0];words[at]=w
  if w&0x3b000000==0x18000000:
   dest=at+4*r['signed_field'](w,5,19);assert at<dest<meta['code_bytes'] and dest%16==0;refs.append(dest)
 pool=min(refs) if refs else None
 while words[max(words)]==0xd503201f:del words[max(words)]
 stop=max(words)+4;transfers={at:r['control'](w,at) for at,w in words.items()};heads={0}
 for at,(kind,target) in transfers.items():
  if target is not None:assert target in words;heads.add(target)
  if kind!='next' and at+4<stop:heads.add(at+4)
 heads=sorted(heads);ids={at:i for i,at in enumerate(heads)};blocks=[]
 for end in [*heads[1:],stop]:
  kind,target=transfers[end-4];dest=None if target is None else ids[target];fall=ids[end] if kind not in ['return','trap','jump'] else None;blocks.append((kind,dest,fall))
 return {'symbol':symbol,'object_sha256':record['object_sha256'],'atom':atom,'pool':pool,'heads':heads,'blocks':blocks}
base=read(LAB/'early-codebook-gated-default-source/rebuild.json');profiles=[];proof=[]
for item in base['native_flags']['MECH_COMPACT_PHASE_BLOCK_LAYOUT'].split(';'):
 symbol,path=item.split('@',1);old=read(Path(path));a=shape('unprofiled-control',symbol);b=shape('unprofiled-paired',symbol)
 assert a['blocks']==b['blocks'],symbol
 assert sorted(old['order'])==a['heads'] and old.get('literal_offset')==a['pool'],symbol
 table=dict(zip(a['heads'],b['heads'],strict=True));new=old|{'order':[table[k] for k in old['order']],'literal_offset':b['pool']}
 assert 'object_tail' not in new,'Tail extent needs separate repricing'
 dest=OUT/'profiles'/Path(path).name;dest.parent.mkdir(exist_ok=True);dest.write_text(json.dumps(new,indent=2)+'\n');profiles.append(symbol+'@'+str(dest));proof.append({'symbol':symbol,'old_profile':str(path),'old_profile_sha256':sha(path),'new_profile':str(dest),'new_profile_sha256':sha(dest),'control':a,'paired':b})
(OUT/'profile-rebase.json').write_text(json.dumps(proof,indent=2)+'\n')
compiler=Path(read(OUT/'unprofiled-paired/receipt.json')['command'][0]);flags=base['native_flags']|{'MECH_COMPACT_PAIR_REGISTER_SPILLS':'1','MECH_COMPACT_PHASE_BLOCK_LAYOUT':';'.join(profiles)};folder=OUT/'paired-profiled';folder.mkdir();front=folder/'frontend';command=[str(compiler),base['phases']['source_to_native']['command'][1],str(front),'compact']
with (folder/'compile.log').open('w') as log:subprocess.run(command,env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}|flags,stdout=log,stderr=log,check=True)
artifacts={n:sha(front/n) for n in ['program.o','metadata.json','program.mecb']};assert artifacts['program.mecb']==base['bytecode_sha256'];layout=emit.symbols(front/'program.o')
record={'source_commit':'388e1087c401cd3b9f87a31b10652372ab720096','source_pin':'388e1087c401cd3b9f87a31b10652372ab720096','compiler_sha256':sha(compiler),'command':command,'compile':command,'flags':flags,'object_sha256':artifacts['program.o'],'metadata_sha256':artifacts['metadata.json'],'bytecode_sha256':artifacts['program.mecb'],'text_bytes':layout['text_bytes'],'layout':layout,'control_receipt':str(LAB/'preserved-registers/preserved/receipt.json'),'runtime_qualification':'not performed','compile_time_measurement':'not performed; matched complete builds still required'}
(folder/'receipt.json').write_text(json.dumps(record,indent=2)+'\n');print('Paired with graph-verified profiles',layout['text_bytes'],artifacts['program.o'],flush=True)

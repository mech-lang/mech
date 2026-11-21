"""Verify raw evidence; no diagnostic campaign is promoted to a full win."""
from pathlib import Path
from hashlib import sha256
from statistics import mean
import gzip,json,math,struct,subprocess,tarfile
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
digest=lambda b:sha256(b).hexdigest()
def close(a,b):assert math.isclose(a,b,rel_tol=1e-12,abs_tol=1e-12),(a,b)
def text_size(raw):
 offset=32
 for _ in range(struct.unpack_from('<I',raw,16)[0]):
  kind,size=struct.unpack_from('<II',raw,offset)
  if kind==0x19:
   for i in range(struct.unpack_from('<I',raw,offset+64)[0]):
    section=offset+72+80*i;name,segment=struct.unpack_from('<16s16s',raw,section)
    if name.rstrip(b'\0')==b'__text' and segment.rstrip(b'\0')==b'__TEXT':return struct.unpack_from('<Q',raw,section+40)[0]
  offset+=size
 raise AssertionError('Missing native text')
def main():
 pins=read(HERE/'archive-files.json');archives={}
 for archive,entries in pins.items():
  with tarfile.open(HERE/archive) as tar:
   assert sorted(tar.getnames())==sorted(entries)
   files={}
   for name,pin in entries.items():
    raw=tar.extractfile(name).read();assert len(raw)==pin['bytes'] and digest(raw)==pin['sha256'],(archive,name);files[name]=raw
   archives[archive]=files
 source=archives['source-and-receipts.tar.gz'];timings=archives['raw-runtime.tar.gz'];study=read(HERE/'study.json');assert not study['experimental_promotions'];assert study['qualified_selection']['roc']==81 and study['qualified_selection']['libdeflate']==62
 bytecode=digest(source['canonical-program.mecb']);assert bytecode=='c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3'
 for role,row in study['cases'].items():
  receipt=json.loads(source[row['receipt']]);assert receipt['artifacts']==row['artifacts'] and receipt['text_bytes']==row['text_bytes']
  for name in ['program.o','metadata.json']:
   raw=source[f'{role}/frontend/{name}'];assert digest(raw)==row['artifacts'][name],(role,name)
   if name=='program.o':assert text_size(raw)==row['text_bytes'],role
  assert row['artifacts']['program.mecb']==bytecode
 identities=json.loads(source['compiler-build/source-identities.json']);repo=HERE.parents[5]
 for name,pin in identities.items():
  raw=source['compiler-build/source-inputs/'+name];assert digest(raw)==pin
  committed=subprocess.check_output(['git','show',study['live_source_commit']+':'+name],cwd=repo);assert committed==raw,name
 n=0
 for campaign in study['campaigns'].values():
  for run in campaign['runs']:
   folder=Path(run['run']).name;summary=timings[folder+'/summary.json'];assert digest(summary)==run['summary_sha256'];rows=json.loads(summary);assert rows==[run['result']];row=rows[0];assert row['case']=='prefix-natural-4096'
   blocks=json.loads(gzip.decompress(timings[folder+'/'+row['case']+'/observations.json.gz']));assert len(blocks)==96
   peers=json.loads(timings[folder+'/pins.json']);role=run['role'];runtime=json.loads(source[role+'/bound/runtime.json']);assert peers['actors']['mech-experiment']['binary_sha256']==runtime['worker_sha256']
   assert peers['actors']['mech-control']['binary_sha256']==study['qualified_selection']['worker_sha256']
   for actor in row['paired_x_libdeflate']:
    for b in blocks:
     observations=b['observations'][actor];close(mean(x['seconds'] for x in observations),b['means'][actor])
     for x in observations:
      for key in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']:
       if key in x:assert x[key],(folder,actor,key)
    for key,reference in [('paired_x_libdeflate','libdeflate'),('paired_x_roc','roc')]:close(math.exp(mean(math.log(b['means'][actor]/b['means'][reference]) for b in blocks)),row[key][actor])
   assert row['paired_x_libdeflate']['mech-experiment']>1,(folder,'unexpected new point win');n+=1
 print(f'Verified {len(study["cases"])} native emissions, {n} focused orders, source pin and archive hashes. Retained score remains 81/62.')
if __name__=='__main__':main()

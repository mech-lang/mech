"""Independently reconcile source-built artifacts and the recorded single-case observations."""
from pathlib import Path
from hashlib import sha256
import gzip,io,json,math,struct,tarfile
HERE=Path(__file__).resolve().parent
read=lambda p:json.loads(p.read_text())
blob=lambda p:gzip.decompress(p.read_bytes())
sha=lambda b:sha256(b).hexdigest()
for name,pin in read(HERE/'files.json').items():assert sha((HERE/name).read_bytes())==pin,name
recipe=read(HERE/'emission-recipes.json');summary=read(HERE/'summary.json')
assert not summary['selected'] and not summary['new_qualified_runtime_win']
assert summary['qualified_reference']['encoder_text_bytes']==63804
assert summary['qualified_reference']['roc_wins']==81 and summary['qualified_reference']['libdeflate_wins']==62
assert sha(blob(HERE/'source.patch.gz'))==summary['source_patch_sha256']==recipe['source_patch_sha256']
for name,pin in summary['source_files'].items():assert sha(blob(HERE/'source'/(Path(name).name+'.gz')))==pin
assert summary['pointer_validation']['jit_tests']['passed_per_configuration']==15
for mode in ['head-pointer','latch-pointer-and-lanes']:
 assert '15 passed; 0 failed' in (HERE/'validation'/(mode+'-tests.log')).read_text()
assert sha(blob(HERE/'program.mecb.gz'))=='c6529d934f020f07f8e59f13fab40927e066649e202269818ad3f009a942a8c3'
for role,case in recipe['cases'].items():
 for n in ['program.o','metadata.json']:assert sha(blob(HERE/role/(n+'.gz')))==case['artifacts'][n]
 for profile in case['profiles']:assert sha((HERE/profile['path']).read_bytes())==profile['sha256']
 if role in ['pointer-default-layout','pointer-lanes-default-layout']:
  proof=json.loads(blob(HERE/role/'layout-proof.json.gz'))
  assert proof['profile_sha256']==next(p['sha256'] for p in recipe['cases'][role.replace('-default-layout','-profiled')]['profiles'] if p['symbol']=='mech_compact_numeric_3009_no_growth_composed')
 data=blob(HERE/role/'program.o.gz');at=32;text=None
 for _ in range(struct.unpack_from('<I',data,16)[0]):
  kind,size=struct.unpack_from('<II',data,at)
  if kind==0x19:
   for i in range(struct.unpack_from('<I',data,at+64)[0]):
    s=at+72+80*i
    if data[s:s+16].rstrip(b'\0')==b'__text':text=struct.unpack_from('<Q',data,s+40)[0]
  at+=size
 assert text==case['encoder_text_bytes']
assert recipe['cases']['pointer-disabled-control']['artifacts']['program.o']=='c5aab01c69ff4ed325b5149d2e1402a59477e5858f52052939d51f7eff699ef8'
linked=read(HERE/'validation/result.json');assert linked['matches_success_observations']==162 and len(linked['checked_failure_cases'])==6
assert sha(blob(HERE/'validation/stack-probe.o.gz'))==linked['stack_probe']['sha256']
reference=None
for role in ['maintained','pointer-profiled','pointer-lanes-profiled']:
 rows=json.loads(blob(HERE/'validation'/role/'smokes.json.gz'));assert len(rows)==162
 assert all(all(r[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for r in rows)
 if reference is None:reference=rows
 else:assert rows==reference
 assert linked['peak_stack_bytes'][role]==4848<=5280
 if role!='maintained':
  runtime=json.loads(blob(HERE/role/'runtime.json.gz'));assert sha(blob(HERE/role/'worker.gz'))==runtime['worker_sha256']
  assert runtime['object_sha256']==recipe['cases'][role]['artifacts']['program.o']
  r=json.loads(blob(HERE/role/'recoveries/result.json.gz'));assert r['cases']==6 and r['exact_recoveries']==12
  rows=json.loads(blob(HERE/role/'recoveries/checked-recoveries.json.gz'));assert len(rows)==12
  assert all(all(x[k] for k in ['exact_reference','independent_inflation','input_alias_unchanged','borrowed_output_alias_unchanged']) for x in rows)
for row in summary['runs']:
 with tarfile.open(fileobj=io.BytesIO(blob(HERE/'runtime'/(row['run']+'.tar.gz')))) as archive:
  get=lambda name:archive.extractfile(name).read()
  data=get('summary.json');assert sha(data)==row['summary_sha256']
  raw=json.loads(data);assert len(raw)==1 and raw[0]['case']=='prefix-natural-4096'
  complete=json.loads(get('complete.json'));assert complete['cases']==1 and complete['balanced_blocks_per_case']==96
  blocks=json.loads(gzip.decompress(get('prefix-natural-4096/observations.json.gz')));assert len(blocks)==96
  for key,peer in [('control','mech-control'),('libdeflate','libdeflate')]:
   ratio=math.exp(math.fsum(math.log(b['means']['mech-experiment']/b['means'][peer]) for b in blocks)/96)
   assert math.isclose(ratio,row[key]['paired_time_ratio'],rel_tol=1e-12)
  if row['role'] in ['pointer-profiled','pointer-lanes-profiled']:
   pins=json.loads(get('pins.json'));r=json.loads(blob(HERE/row['role']/'runtime.json.gz'))
   assert pins['actors']['mech-experiment']['binary_sha256']==r['worker_sha256']
   assert pins['actors']['roc']['binary_sha256']=='ed1ab9175cd175d26d453afc26ba1fa7adb4827ec705aa1cf9bcd231ac84860c'
   assert pins['actors']['libdeflate']['binary_sha256']=='06e3b4b11fafb5a79ee4a44a0462682662e72f3520b94fd8dd34b6f5e9c2e1d2'
   assert pins['actors']['roc']['metadata_sha256']==sha(blob(HERE/'peers/compatible-roc-metadata.json.gz'))
roc=read(HERE/'peers/roc-qualification.json');assert roc['complete_smokes']==162 and roc['verified']
assert read(HERE/'runtime/pointer-isolation.json')['probe_exit_code']==0
print('Verified nine encoder variants, actual linked workers, 162 outputs each, six failures, 12 recoveries each, stack ceiling and 12 recorded single-case runs. No full runtime qualification or 63rd-win claim.')

"""Freeze each completed R/APL/Uiua preparation and unchanged Mech AOT controls."""
import argparse,copy,hashlib,importlib.util,json,os
from datetime import datetime,timezone
from pathlib import Path
G=Path(__file__).resolve().parent
C=G.parents[1]
BENCHES=['regexredux','revcomp','pidigits','spectralnorm','mandelbrot','fasta','knucleotide','binarytrees','nbody','fannkuchredux']
LANGUAGES=['r','apl','uiua']
spec=importlib.util.spec_from_file_location('arrays',C/'array-comparison/all-20261009/run.py')
arrays=importlib.util.module_from_spec(spec);spec.loader.exec_module(arrays)
compact=arrays.compact
WORK=arrays.WORK
REQUIRED=arrays.REQUIRED
LOCK=Path('/tmp/mech-clbg-timing.lock')
compact.COMPILERS.update(['cargo','ldc2','dmd','gdc','fpc','gprbuild','futhark','zig','odin','ghc','ocamlopt','ninja'])
INPUTS={'knucleotide':(compact.SMALL_INPUT,compact.FULL_INPUT),'revcomp':(compact.SMALL_INPUT,compact.FULL_INPUT),'regexredux':(C/'regexredux/inputs/regexredux-input-small.txt',C/'regexredux/.build/input5000000.txt')}

def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
 path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
 tmp=path.with_suffix(path.suffix+'.tmp');tmp.write_text(json.dumps(value,indent=2)+'\n');tmp.replace(path)

def support_pins(value):
 """Accept producer pin dictionaries without treating arbitrary strings as files."""
 result={}
 if isinstance(value,dict):
  if value.get('path') and isinstance(value.get('sha256'),str):result[value['path']]=value['sha256']
  for name,child in value.items():
   if name in ['pins','libraries','pin_files','support_pins'] and isinstance(child,dict):
    for filename,digest in child.items():
     if isinstance(digest,str) and len(digest)==64:
      assert Path(filename).is_file(),('Missing required support pin',filename)
      result[str(Path(filename).absolute())]=digest
      result[str(Path(filename).resolve())]=digest
   if isinstance(child,(dict,list)):result.update(support_pins(child))
 elif isinstance(value,list):
  for child in value:result.update(support_pins(child))
 return result

def execution_fingerprint(row):
 # Different worker flags, runtime modes or native binaries are never aliases.
 return json.dumps({'source_sha256':row['source_sha256'],'binary_sha256':row.get('binary_sha256'),'command':[x.replace(row['source'],'{source}') for x in row['command']],'cwd':row.get('cwd',str(compact.ROOT)),'environment':row.get('runtime_environment',{}),'support':support_pins(row)},sort_keys=True)

def resolve_aliases(rows):
 known={r['key']:r for r in rows};ids={r['id']:r['key'] for r in rows};seen={}
 for row in rows:
  declared=row.get('timing_alias_of')
  fingerprint=execution_fingerprint(row)
  if declared:
   target=known.get(ids.get(declared,declared));assert target is not None,(row['key'],declared)
   assert target['benchmark']==row['benchmark'] and target['language']==row['language']
   assert execution_fingerprint(target)==fingerprint,('Non-identical declared timing alias',row['key'],target['key'])
   row['timing_alias_of']=target['key']
  elif fingerprint in seen:row['timing_alias_of']=seen[fingerprint]
  else:seen[fingerprint]=row['key']
 for row in rows:
  visited={row['key']}
  while row.get('timing_alias_of'):
   target=known[row['timing_alias_of']]
   assert target['key'] not in visited,('Alias cycle',target['key'])
   visited.add(target['key']);row['timing_alias_of']=target.get('timing_alias_of') or target['key']
   if not target.get('timing_alias_of'):break
 return rows

def freeze(language):
 manifest_path=G/(language+'-manifest.json')
 manifest=json.loads(manifest_path.read_text())
 assert 'pending' not in manifest.get('status','').lower(),('Preparation still pending',language,manifest.get('status'))
 rows=[];pins=support_pins(manifest)
 blocked={r['source_sha256'] for r in manifest.get('blocked_source_hashes',[])}
 for original in manifest['programs']:
  row=copy.deepcopy(original)
  if row.get('excluded') or row.get('algorithm',{}).get('strict') is False:continue
  row.setdefault('language',language);assert row['language']==language
  row.setdefault('key',row['id'] if row['id'].startswith(row['benchmark']+':') else row['benchmark']+':'+row['id'])
  assert row['algorithm']['strict'] and row['algorithm'].get('native_benchmark_kernel') is False
  source=Path(row.get('source',row.get('source_path',''))).resolve();row['source']=str(source)
  assert source.is_file() and sha(source)==row['source_sha256'],source
  assert row['source_sha256'] not in blocked,('Explicitly rejected source hash',row['key'],row['source_sha256'])
  assert isinstance(row['loc'],(int,float)) and row['loc']>=1
  assert row.get('small_validation',{}).get('reference_match') is True,('Small canonical validation missing',row['key'])
  row['cwd']=str(Path(row.get('cwd',compact.ROOT)).resolve())
  if row.get('binary'):assert sha(row['binary'])==row['binary_sha256'];pins[row['binary']]=row['binary_sha256']
  runtime=row.get('runtime') or manifest.get('runtime')
  if runtime:
   assert runtime.get('path') and runtime.get('sha256'),row['key'];pins[runtime['path']]=runtime['sha256']
  pins[source.as_posix()]=row['source_sha256']
  pins.update(support_pins(row))
  row.update(eligible_for_chart=False,measurement_status='not-measured')
  rows.append(row)
 assert {r['benchmark'] for r in rows}==set(BENCHES),(language,'Incomplete prepared suite')
 assert len({r['key'] for r in rows})==len(rows)
 resolve_aliases(rows)
 control_file=C/'remaining-languages-comparison/20261011/source-audit-v2.json'
 controls=copy.deepcopy(json.loads(control_file.read_text())['controls'])
 nbody_control_file=C/'nbody/experiments/20261011-shared-products/replication-50m.json'
 current_nbody=next(r for r in json.loads(nbody_control_file.read_text())['programs'] if r['id']=='pairwise-seed8')
 old_nbody=next(r for r in controls if r['benchmark']=='nbody')
 for field in ['source','source_sha256','binary_sha256','command','loc']:old_nbody[field]=current_nbody[field]
 old_nbody['binary']=current_nbody['command'][0]
 old_nbody['current_control_provenance']=str(nbody_control_file)
 baseline_file=C/'d-comparison/20261011/evidence/site-before.json'
 baseline=json.loads(baseline_file.read_text())
 endpoints={}
 for row in controls:
  for field in ['source','binary']:
   assert sha(row[field])==row[field+'_sha256'];pins[row[field]]=row[field+'_sha256']
  row.update(id=row['id']+'-for-'+language,key=row['benchmark']+':mech-control-for-'+language,control_for=language,cwd=str(compact.ROOT))
  group=next(g for b in baseline['benchmarks'] if b['key']==row['benchmark'] for g in b['groups'] if g['language']=='mech')
  fast=baseline['programs'][group['fast']];small=baseline['programs'][group['small']]
  assert row['source_sha256'] in fast['files'] and row['loc']==fast['loc'],('Mech baseline mismatch',row['benchmark'])
  endpoints[row['benchmark']]={'fastest_loc':fast['loc'],'smallest_loc':small['loc'],'fastest_key':group['fast'],'smallest_key':group['small'],'control_key':row['key']}
  row['algorithm']['native_benchmark_kernel']=False
 assert {r['benchmark'] for r in controls}==set(BENCHES)
 for path in [manifest_path,control_file,nbody_control_file,baseline_file,G/'prepare.py',G/'run.py',G/'summarize.py',C/'compact/run.py',C/'nbody/run.py',C/'array-comparison/all-20261009/run.py']:
  pins[str(path)]=sha(path)
 for pair in INPUTS.values():
  for path in pair:pins[str(path)]=sha(path)
 for bench in ['regexredux','pidigits']:
  for name in [bench+'-output-small.txt','output-full.txt']:
   path=C/bench/'inputs'/name;pins[str(path)]=sha(path)
 for path,digest in pins.items():assert sha(path)==digest,path
 audit={'status':'reviewed','language':language,'frozen_at':datetime.now(timezone.utc).isoformat(),'manifest':str(manifest_path),'manifest_sha256':sha(manifest_path),'programs':rows,'controls':controls,'pins':pins,'workloads':WORK,'benchmark_order':BENCHES,'baseline_endpoints':endpoints,'control_audit':str(control_file),'baseline_snapshot':str(baseline_file),'shared_timing_lock':str(LOCK),'power_required':REQUIRED,'timing_boundary':'Three fresh-process full-workload samples; language/runtime startup, workers, input, every computation, complete output to regular files and process shutdown included. Compilation, validation and hashing excluded. Global file lock serializes all benchmark cohorts; source/runtime/input/output references pinned; compiler and power overlap invalidates a sample. At most eight language workers. No extrapolated or shortened workloads admitted.','loc_policy':'Producer-audited nonblank/noncomment application LOC, independent statement/binding expansion; long literal data and nested calls remain expressions. Standard general-purpose libraries excluded uniformly.'}
 path=G/('source-audit-'+language+'.json');save(path,audit)
 print('FROZEN',language,len(rows),'roles',sum(not r.get('timing_alias_of') for r in rows),'distinct executions',flush=True)
 return audit

def main():
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('--languages',default=','.join(LANGUAGES));args=p.parse_args()
 for language in args.languages.split(','):assert language in LANGUAGES;freeze(language)
if __name__=='__main__':main()

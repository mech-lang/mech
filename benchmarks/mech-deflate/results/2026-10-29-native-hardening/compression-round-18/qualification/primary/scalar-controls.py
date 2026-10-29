from pathlib import Path
import hashlib,json,statistics,subprocess
root=Path(__file__).resolve().parent
before=root/'frame-before'
after=root/'frame-after'
expected=json.loads((root/'frame-before.json').read_text())['sha256']
assert hashlib.sha256(before.read_bytes()).hexdigest()==expected
active=subprocess.check_output(['ps','-Ao','comm='],text=True).splitlines()
assert not [x for x in active if Path(x.strip()).name in ['component-v2-tests','selected-native-tests','selected-bits-tests','rustc','cargo','current','roc-profile','word-updates','artifact_review']]
results={'before_sha256':expected,'after_sha256':hashlib.sha256(after.read_bytes()).hexdigest(),'controls':[],'primary_compression_observations':0}
commands=[]
cases=[('scalar-bool',0,10000),('scalar-u64',0,10000),('scalar-i64',0,10000),('scalar-f64',0,10000),('scalar-loop',0,100),('scalar-loop',64,100)]
for i,(case,steps,repetitions) in enumerate(cases):
 for form in ['original','decoded']:
  pair={}
  order=[('before',before),('after',after)] if i%2==0 else [('after',after),('before',before)]
  for label,binary in order:
   stem=f'scalar-{case}-{steps}-{form}-{label}'
   assert not (root/(stem+'.stdout')).exists()
   command=[str(binary),case,form,str(steps),'5',str(repetitions)]
   result=subprocess.run(command,capture_output=True,text=True,timeout=120)
   (root/(stem+'.stdout')).write_text(result.stdout);(root/(stem+'.stderr')).write_text(result.stderr)
   commands.append({'command':command,'exit':result.returncode});(root/'scalar-commands.json').write_text(json.dumps(commands,indent=2)+'\n')
   result.check_returncode();rows=[json.loads(l) for l in result.stdout.splitlines() if l.startswith('{')]
   assert len(rows)==6 and sum(not r['warmup'] for r in rows)==5
   pair[label]={'rows':rows,'median_seconds':statistics.median(r['seconds'] for r in rows if not r['warmup'])}
  b=pair['before']['rows'][1];a=pair['after']['rows'][1]
  assert b['artifact_sha256']==a['artifact_sha256']
  ratio=pair['after']['median_seconds']/pair['before']['median_seconds']
  checks={'requests_equal':a['allocation_requests']==b['allocation_requests'],'bytes_equal':a['requested_bytes']==b['requested_bytes'],'timing_ratio_le_1_05':ratio<=1.05}
  if case!='scalar-loop':assert b['scalar_operations']==a['scalar_operations']
  results['controls'].append({'case':case,'form':form,'steps':steps,'after_over_before':ratio,'checks':checks,**pair})
  (root/'scalar-results.json').write_text(json.dumps(results,indent=2)+'\n')
  print(case,form,steps,ratio,checks,flush=True)
results['direct_controls_pass']=all(all(x['checks'].values()) for x in results['controls'] if x['case']!='scalar-loop')
for form in ['original','decoded']:
 rows=[x for x in results['controls'] if x['case']=='scalar-loop' and x['form']==form]
 assert len(rows)==2
 for label in ['before','after']:
  assert rows[0][label]['rows'][1]['allocation_requests']==rows[1][label]['rows'][1]['allocation_requests']
  assert rows[0][label]['rows'][1]['requested_bytes']==rows[1][label]['rows'][1]['requested_bytes']
results['scalar_loop_allocation_slope_zero']=True
(root/'scalar-results.json').write_text(json.dumps(results,indent=2)+'\n')
print('SCALAR_CONTROLS',results['direct_controls_pass'],flush=True)

from pathlib import Path
import hashlib,json,statistics,subprocess,time
r=Path(__file__).resolve().parent;p=r/'scalar-repeat';p.mkdir();bins={name:r/'primary'/('frame-'+name) for name in ['before','after']}
plan={'reason':'Initial original zero-step loop ratio1.052454987>1.05; decoded zero-step moved in opposite direction. Fixed balanced replication, no conditional continuation or discarded observation. Initial strict failure remains retained.','pairs_per_form':8,'forms':['original','decoded'],'source_case':'scalar-loop','steps':0,'measured_samples':5,'warmups':1,'invocations_per_sample':100,'aggregation':'median of eight paired after/before medians per form; also report ratio of all pooled medians and every individual pair.','binaries':{k:{'path':str(v),'sha256':hashlib.sha256(v.read_bytes()).hexdigest()} for k,v in bins.items()}}
(p/'PLAN.json').write_text(json.dumps(plan,indent=2)+'\n')
active=subprocess.check_output(['ps','-Ao','comm='],text=True).splitlines();assert not [x for x in active if Path(x.strip()).name in ['rustc','cargo','engine-tests','component-v2-tests','current','roc-profile','artifact_review','object-evidence']]
results=[]
for i in range(8):
 for form in ['original','decoded']:
  pair={};order=['before','after'] if (i+(form=='decoded'))%2==0 else ['after','before']
  for label in order:
   stem=f'{form}-{i}-{label}';c=[str(bins[label]),'scalar-loop',form,'0','5','100'];a=time.monotonic();s=subprocess.run(c,capture_output=True,text=True,timeout=30);(p/(stem+'.stdout')).write_text(s.stdout);(p/(stem+'.stderr')).write_text(s.stderr);assert s.returncode==0
   rows=[json.loads(x) for x in s.stdout.splitlines() if x.startswith('{')];assert len(rows)==6 and sum(not x['warmup'] for x in rows)==5
   pair[label]={'command':c,'exit':s.returncode,'process_seconds':time.monotonic()-a,'rows':rows,'median_seconds':statistics.median(x['seconds'] for x in rows if not x['warmup'])}
  for field in ['artifact_sha256','allocation_requests','requested_bytes','work_per_invocation','visits_per_invocation','private_frame_fields']:
   assert pair['before']['rows'][1][field]==pair['after']['rows'][1][field],field
  results.append({'form':form,'pair':i,'order':order,'after_over_before':pair['after']['median_seconds']/pair['before']['median_seconds'],**pair})
  (p/'RESULTS.json').write_text(json.dumps({'plan':plan,'pairs':results},indent=2)+'\n')
summary={}
for form in plan['forms']:
 rows=[x for x in results if x['form']==form];summary[form]={'paired_ratios':[x['after_over_before'] for x in rows],'median_paired_ratio':statistics.median(x['after_over_before'] for x in rows),'pooled_ratio':statistics.median(y['seconds'] for x in rows for y in x['after']['rows'] if not y['warmup'])/statistics.median(y['seconds'] for x in rows for y in x['before']['rows'] if not y['warmup'])}
(p/'SUMMARY.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary),flush=True)

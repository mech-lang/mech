from pathlib import Path
import json,math,random,statistics as st
P=Path(__file__).parent;D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results');rule=json.loads((P/'prospective-rule.json').read_text())
def read(p):return json.loads(Path(p).read_text())
def quant(a,q):
 x=(len(a)-1)*q;i=math.floor(x);j=math.ceil(x);return a[i]+(a[j]-a[i])*(x-i)
for label in ['first','confirmation']:
 Q=P/label;suffix='-confirmation'if label=='confirmation'else'';lat=read(D/('comparison-native-dictionary-seed-frames'+suffix+'-20261108-5000-m3.json'));res=read(D/('comparison-native-dictionary-seed-frames-resources'+suffix+'-20261108-5000-m3.json'))
 assert len(lat['samples'])==31*31 and len(res['samples'])==32*7
 assert all(x['exit']==0 and x['exact_rust_output']for x in lat['samples']+res['samples'])
 def paired(a,b,f):
  data={n:{x['sample']:x[f]for x in lat['samples']if x['variant']==n}for n in [a,b]};assert all(set(d)==set(range(31))for d in data.values());d=[data[a][i]/data[b][i]-1 for i in range(31)];rng=random.Random(rule['confirmation_bootstrap_seed']if label=='confirmation'else rule['bootstrap_seed']);boots=sorted(st.median(d[rng.randrange(31)]for _ in range(31))for _ in range(rule['bootstrap_resamples']));return dict(candidate=a,baseline=b,wins=sum(v<0 for v in d),pairs=31,median_paired_relative_change=st.median(d),two_sided_95_interval=[quant(boots,.025),quant(boots,.975)],all_pair_changes=d)
 med={}
 for v in lat['implementations']:
  if v['kind']!='mech':continue
  n=v['name'];rows=[x for x in lat['samples']if x['variant']==n];med[n]={f:st.median(x[f]for x in rows)for f in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']};med[n].update(times_fastest_processing_Rust=lat['summary'][n]['post_json_over_fastest_matched_native'],times_fastest_whole_Rust=lat['summary'][n]['whole_process_over_fastest'])
 resources={};config=read(Q/'resources-config.json');refs={config['resource_reference'],config['whole_process_resource_reference']}
 for v in config['implementations']:
  if v['kind']!='mech'and v['name']not in refs:continue
  rows=[x for x in res['samples']if x['variant']==v['name']];r={}
  for label2,f in [('restore','restore_measurement'),('post_JSON','measurement')]:
   if not all(f in x for x in rows):continue
   r[label2]={k:st.median(x[f][k]for x in rows)for k in ['allocation_requests','requested_allocation_bytes','peak_live_requested_heap_bytes','peak_additional_live_requested_heap_bytes']};r[label2].update({k:st.median(x[f]['hardware'][k]for x in rows)for k in ['instructions_retired','cycles']})
  r['process_resources']=res['summary'][v['name']]['process_resources_median'];resources[v['name']]=r
 pairs={}
 for a,b in [(rule['candidate'],rule['immediate_parent']),(rule['candidate'],rule['emission_control']),(rule['candidate'],rule['older_processing_control']),(rule['emission_control'],rule['immediate_parent'])]:
  pairs[a+' vs '+b]={f:paired(a,b,f)for f in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']}
 out=dict(independent_confirmation=label=='confirmation',all_first_and_adverse_samples_retained=True,latency_exact_outputs=len(lat['samples']),resource_exact_outputs=len(res['samples']),fastest_processing_Rust=lat['matched_native_reference'],fastest_whole_Rust=lat['whole_process_reference'],medians=med,resource_medians=resources,paired_comparisons=pairs,whole_process_gate=False,goal_complete=False)
 (Q/'paired-analysis.json').write_text(json.dumps(out,indent=2)+'\n')
 print(label, 'fastestRust',lat['matched_native_reference']['variant'],lat['matched_native_reference']['median_seconds'],flush=True)
 for n,m in med.items():print(n,m,flush=True)
 for k,v in pairs.items():print(k,{f:{a:b for a,b in x.items()if a!='all_pair_changes'}for f,x in v.items()},flush=True)
 print('postJSONresources',json.dumps({n:d.get('post_JSON')for n,d in resources.items()}),flush=True)

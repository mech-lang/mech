from pathlib import Path
import hashlib,json,statistics,random,math,subprocess
P=Path(__file__).parent;D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
rule=json.loads((P/'prospective-rule.json').read_text());lat=json.loads((D/'comparison-native-recovered-scalar-homes-20261108-5000-m3.json').read_text());res=json.loads((D/'comparison-native-recovered-scalar-homes-resources-20261108-5000-m3.json').read_text())
def quant(a,q):
 pos=(len(a)-1)*q;i=math.floor(pos);j=math.ceil(pos);return a[i]+(a[j]-a[i])*(pos-i)
def paired(candidate,reference,boundary):
 values={n:{x['sample']:x[boundary]for x in lat['samples']if x['variant']==n}for n in [candidate,reference]};assert all(set(v)==set(range(31))for v in values.values())
 changes=[values[candidate][i]/values[reference][i]-1 for i in range(31)];rng=random.Random(rule['bootstrap']['seed']);boots=sorted(statistics.median(changes[rng.randrange(31)]for _ in range(31))for _ in range(rule['bootstrap']['resamples']))
 return dict(wins=sum(x<0 for x in changes),pairs=31,median_paired_relative_change=statistics.median(changes),two_sided_95_interval=[quant(boots,.025),quant(boots,.975)],one_sided_95_upper=quant(boots,.95),all_pair_relative_changes=changes)
decisions={}
for candidate in rule['candidates']:
 comparisons={ref:{boundary:paired(candidate,ref,boundary)for boundary in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']}for ref in rule['primary_references'][candidate]}
 target=comparisons[rule['matched_parent']]['post_json_seconds']['two_sided_95_interval'][1]<0;other=True
 decisions[candidate]=dict(target_passed=target,other_boundary_passed=other,first_cohort_passed=target and other,selected=False,paired=comparisons,confirmation_run=False)
names=list(dict.fromkeys([rule['fixed_reference'],'mech-ir264-cranelift-runtime-control','mech-llvm-reuse-4','mech-ir276-matrix-length-4','mech-ir277-packed-row-4',rule['matched_parent'],*rule['candidates']]));medians={};resources={}
for n in names:
 rows=[x for x in lat['samples']if x['variant']==n];summary=lat['summary'][n];medians[n]={k:statistics.median(x[k]for x in rows)for k in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']};medians[n].update(times_fastest_processing_Rust=summary['post_json_over_fastest_matched_native'],times_fastest_whole_Rust=summary['whole_process_over_fastest'])
 rr=[x for x in res['samples']if x['variant']==n+'-instrumented'];metrics={}
 for label,field in [('restore','restore_measurement'),('post_JSON','measurement')]:
  metrics[label]={k:statistics.median(x[field][k]for x in rr)for k in ['allocation_requests','requested_allocation_bytes','peak_live_requested_heap_bytes','peak_additional_live_requested_heap_bytes']};metrics[label].update({k:statistics.median(x[field]['hardware'][k]for x in rr)for k in ['instructions_retired','cycles']})
 metrics['process_resources']=res['summary'][n+'-instrumented']['process_resources_median'];resources[n]=metrics
for candidate,d in decisions.items():
    before=resources[rule['matched_parent']]['post_JSON']['instructions_retired']
    after=resources[candidate]['post_JSON']['instructions_retired']
    d['processing_instruction_ratio']=after/before
    d['processing_instruction_change']=after-before
    d['processing_instruction_gain']=after<before
    d['processing_latency_superiority_established']=d['target_passed']
    d['processing_latency_regression_established']=d['paired'][rule['matched_parent']]['post_json_seconds']['two_sided_95_interval'][0]>0
    d['whole_process_required_gate']=False
    d['whole_process_reported']=True
    d['user_processing_first_policy_applied']=True
out=dict(rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),first_cohort=True,all_first_and_adverse_results_retained=True,latency_exact_outputs=len(lat['samples']),resource_exact_outputs=len(res['samples']),fastest_processing_Rust=lat['matched_native_reference'],fastest_whole_Rust=lat['whole_process_reference'],medians=medians,resource_medians=resources,decisions=decisions,selected=False,goal_complete=False)
assert all(x['exact_rust_output']and x['exit']==0 for x in lat['samples']+res['samples'])
(P/'paired-analysis.json').write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps({n:{k:v for k,v in d.items()if k!='paired'}for n,d in decisions.items()},indent=2))
print(json.dumps(medians,indent=2))

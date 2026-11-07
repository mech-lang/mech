from pathlib import Path
import hashlib,json,math,random,statistics,subprocess
P=Path(__file__).parent
D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
rule=json.loads((P/'prospective-rule.json').read_text())
latency=json.loads((D/'comparison-native-positional-metadata-20261107-5000-m3.json').read_text())
resource=json.loads((D/'comparison-native-positional-metadata-resources-20261107-5000-m3.json').read_text())
candidate=rule['candidate'];results={}
def percentile(a,q):
    p=(len(a)-1)*q;i=math.floor(p);j=math.ceil(p)
    return a[i]+(a[j]-a[i])*(p-i)
for reference in [rule['fixed_reference'],rule['matched_parent']]:
    result={}
    for boundary in ['post_json_seconds','wall_seconds']:
        by={name:{r['sample']:r[boundary]for r in latency['samples']if r['variant']==name}for name in [reference,candidate]}
        assert set(by[reference])==set(by[candidate])==set(range(31))
        a=[by[candidate][i]/by[reference][i]-1 for i in range(31)]
        delta=[by[candidate][i]-by[reference][i]for i in range(31)]
        rng=random.Random(rule['bootstrap']['seed'])
        boots=sorted(statistics.median([a[rng.randrange(31)]for _ in range(31)])for _ in range(rule['bootstrap']['resamples']))
        result[boundary]=dict(wins=sum(x<0 for x in a),median_paired_relative_change=statistics.median(a),median_paired_delta_seconds=statistics.median(delta),two_sided_95_interval=[percentile(boots,.025),percentile(boots,.975)],one_sided_95_upper=percentile(boots,.95))
    results[reference]=result
rs={name:[r for r in resource['samples']if r['variant']==name+'-instrumented']for name in [rule['matched_parent'],candidate]}
restore={name:dict(instructions=statistics.median(r['restore_measurement']['hardware']['instructions_retired']for r in rows),seconds=statistics.median(r['compile_or_restore_seconds']for r in rows),allocations=statistics.median(r['restore_measurement']['allocation_requests']for r in rows),requested_bytes=statistics.median(r['restore_measurement']['requested_allocation_bytes']for r in rows),processing_instructions=statistics.median(r['measurement']['hardware']['instructions_retired']for r in rows),processing_allocations=statistics.median(r['measurement']['allocation_requests']for r in rows))for name,rows in rs.items()}
parent=restore[rule['matched_parent']];c=restore[candidate]
practical=(c['instructions']<=parent['instructions']*.9 or parent['seconds']-c['seconds']>=.001)
whole=all(v['wall_seconds']['wins']>=21 and v['wall_seconds']['two_sided_95_interval'][1]<0 for v in results.values())
ni=results[rule['fixed_reference']]['post_json_seconds']['one_sided_95_upper']<=.01
medians={name:{key:statistics.median(r[key]for r in latency['samples']if r['variant']==name)for key in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']}for name in [rule['fixed_reference'],rule['matched_parent'],candidate]}
for name,v in medians.items():
 v['times_fastest_processing_Rust']=v['post_json_seconds']/latency['matched_native_reference']['median_seconds']
 v['times_fastest_whole_Rust']=v['wall_seconds']/latency['whole_process_reference']['median_seconds']
report=dict(schema='mech.related-post-gen.positional-metadata-performance-decision.v1',rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),source_commit=subprocess.check_output(['git','-c','gc.auto=0','rev-parse','HEAD'],cwd='/private/tmp/mech-related-positional-metadata-compiler-20261107',text=True).strip(),candidate=candidate,paired=results,medians=medians,resource_restore=restore,practical_restore_threshold_passed=practical,whole_superiority_passed=whole,processing_noninferiority_passed=ni,selected=False,decision='Reject checked positional metadata readers: the predeclared restoration practical threshold and whole-process superiority requirements fail. Keep IR242 selected. Optional LLVM remains separate; do not rerun this unchanged candidate for a favorable cohort.',all_first_and_adverse_samples_retained=True,latency_full_outputs_exact=sum(r['exact_rust_output']for r in latency['samples']),resources_full_outputs_exact=sum(r['exact_rust_output']for r in resource['samples']),fastest_processing_Rust=latency['matched_native_reference'],fastest_whole_Rust=latency['whole_process_reference'],whole_process_improvement_not_established=True)
assert not whole and not practical and report['latency_full_outputs_exact']==744 and report['resources_full_outputs_exact']==175
(P/'paired-analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))

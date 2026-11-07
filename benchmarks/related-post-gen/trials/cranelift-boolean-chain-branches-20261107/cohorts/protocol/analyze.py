from pathlib import Path
import hashlib,json,math,random,statistics
P=Path(__file__).parent
D=Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen/results')
rule=json.loads((P/'prospective-rule.json').read_text())
latency=json.loads((D/'comparison-native-boolean-chain-branches-20261107-5000-m3.json').read_text())
candidate=rule['candidate'];results={}
def percentile(a,q):
    p=(len(a)-1)*q;i=math.floor(p);j=math.ceil(p)
    return a[i]+(a[j]-a[i])*(p-i)
for reference in [rule['fixed_anchor'],rule['matched_frontend_control']]:
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
report=dict(schema='mech.related-post-gen.boolean-chain-performance-decision.v1',rule_sha256=hashlib.sha256((P/'prospective-rule.json').read_bytes()).hexdigest(),source_commit='8571f39921',candidate=candidate,paired=results,
    processing_superiority=False,processing_practical_threshold=False,selected=False,decision='Reject this Boolean-chain branch distribution experiment. Keep the fixed IR242 selected compiler; preserve optional LLVM as a separate comparison. Do not rerun unchanged to seek a favorable cohort.',
    all_first_and_adverse_samples_retained=True,latency_full_outputs_exact=744,resources_full_outputs_exact=175)
(P/'paired-analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))

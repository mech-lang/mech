"""Recompute the separate R56-to-R61 elapsed confirmation from raw receipts."""
from pathlib import Path
import json,math,statistics,sys
root=Path(sys.argv[1]);out=Path(sys.argv[2]);r=root/'final-confirmation';p=r/'linked-primary'
summary=json.loads((p/'summary.json').read_text());purpose=json.loads((r/'purpose.json').read_text())
bindings={label:json.loads((r/'bindings'/(label+'.json')).read_text()) for label in ['before','after']}
reference=[line.split('\t') for line in (p/'reference.stdout').read_text().splitlines() if line.startswith('rep\troc-deflate\t')];assert len(reference)==20000
ref=statistics.median(int(row[3]) for row in reference)/1e9;assert ref==summary['reference_seconds']
all_rows={label:[] for label in bindings};blocks=[];groups=[]
for i in range(4):
 d=json.loads((p/f'group{i}/results.json').read_text());assert len(d['blocks'])==16;blocks+=d['blocks'];medians={}
 assert d['source_revisions']=={label:binding['compiler_source'] for label,binding in bindings.items()}
 for label in bindings:
  raw=d['all_observations'][label];assert len(raw)==498
  assert all(v['exact_reference'] and v['independent_inflation'] and v['input_bytes']==8 and v['raw_bytes']==10 and v['capacity']==13 and v['work']==60260219686 for v in raw)
  values=[v for v in raw if not v['warmup'] and not v['profile']];assert len(values)==496;all_rows[label]+=values;medians[label]=statistics.median(v['seconds'] for v in values)
  for j,b in enumerate(d['blocks']):
   assert b['block']==j and b['medians'][label]==statistics.median(v['seconds'] for v in values[j*31:(j+1)*31])
   assert b['after_over_before']==b['medians']['after']/b['medians']['before']
  assert (p/f'group{i}/{label}.stderr').read_text().endswith('CLEANUP bytes=0 nodes=0\n')
 groups.append({'group':i,'medians':medians,'after_over_before':medians['after']/medians['before']})
assert groups==summary['groups'];assert summary['primary_observations']==3968 and summary['all_observations']==3984 and summary['observations_per_variant']==1984
for label,values in all_rows.items():
 median=statistics.median(v['seconds'] for v in values);assert len(values)==1984
 assert summary['medians'][label]=={'seconds':median,'multiple_of_reference':median/ref,'multiple_of_fixed_reference':median/.0003}
ratios=[b['after_over_before'] for b in blocks];assert len(ratios)==64
computed={'pooled_ratio':summary['medians']['after']['seconds']/summary['medians']['before']['seconds'],'improving_groups':sum(g['after_over_before']<1 for g in groups),'improving_blocks':sum(v<1 for v in ratios),'median_block_ratio':statistics.median(ratios),'geometric_block_ratio':math.exp(statistics.mean(math.log(v) for v in ratios))}
assert computed==summary['comparisons']['after_over_before']
assert bindings['before']['normalized_source_sha256']==bindings['after']['normalized_source_sha256']
assert bindings['before']['original_worker_sha256']==bindings['after']['original_worker_sha256']
result={'round':61,'comparison':'accepted R56 production versus final R61 production','purpose':purpose,'primary':summary,'change_percent':100*(computed['pooled_ratio']-1),'adverse_pairs':[g for g in groups if g['after_over_before']>=1],'bindings':{label:{key:b[key] for key in ['compiler_source','worker_sha256','normalized_source_sha256','original_worker_sha256','pins']} for label,b in bindings.items()},'verified':{'primary_rows':3968,'all_rows':3984,'blocks':64,'prepared_pairs':4,'reference_rows':20000,'all_block_medians_recomputed':True,'all_exact_output_and_cleanup_checks':True},'scope':'A fresh separate elapsed-only cohort; no new code change. It compares frozen R56 and R61 executables with the same source, bytecode, allocator and evaluate_into timing. CPU/heap counters and per-change results remain separate. The adverse pair is retained. No independent gains are multiplied, and no counter change is inferred from this elapsed-only confirmation.'}
out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps({'change_percent':result['change_percent'],'medians':summary['medians'],'adverse_pairs':result['adverse_pairs']},indent=2))

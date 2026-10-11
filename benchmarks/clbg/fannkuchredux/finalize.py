#!/usr/bin/env python3
"""Combine labeled measurement batches without pooling samples or losing originals."""
import copy, json
import run

batches=['apple-m3-12.json','indexing-pilot-12.json','final-paired-12.json']
reports=[json.loads((run.HERE/'results'/name).read_text()) for name in batches]
rows={}
for name,report in zip(batches,reports):
    for row in report['programs']:
        row=copy.deepcopy(row)
        row['measurement_batch']=name
        row['sample_count']=len(row['samples_seconds'])
        rows[row['id']]=row
data=copy.deepcopy(reports[-1])
data['programs']=list(rows.values())
data['measurement_batches']=[dict(report=name,sha256=run.sha(run.HERE/'results'/name),
    samples=report['protocol']['samples'],programs=[r['id'] for r in report['programs']],
    compiler_snapshot=report['compiler_snapshot']) for name,report in zip(batches,reports)]
data['protocol']['samples']='3–5'
data['protocol']['selection_skip_regex']=None
data['protocol']['batch_note']='Broad sweep: 3 samples; final matched controls: 5. Remaining indexing ablations: 3. Per-build medians stay within their original batch; samples are not pooled.'
data['protocol']['arm_port_note']='Rust #6 NEON translation is explicitly marked source_modified. Original x86 source remains excluded.'
data['endpoints']=run.common.endpoint_summary(data['programs'])
data['nonheroic_endpoints']=run.common.endpoint_summary([r for r in data['programs'] if not r['upstream_starred']])
output=run.HERE/'results/apple-m3-final-12.json'
data['compiler_snapshot']=run.snapshot(output)
output.write_text(json.dumps(data,indent=2)+'\n')
for language in sorted({r['language'] for r in data['programs']}):
    group=[r for r in data['programs'] if r['language']==language]
    fast=min(group,key=lambda r:r['median_seconds'])
    print(language,fast['id'],f'{fast["median_seconds"]:.6f}s',fast['source']['nonblank_noncomment_lines'],'LOC')

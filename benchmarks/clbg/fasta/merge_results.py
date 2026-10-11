#!/usr/bin/env python3
"""Preserve source batches and normalize each to its own C #2 LLVM control."""
import json
from pathlib import Path
import run
HERE=run.HERE
old_path=HERE/'results/apple-m3-25000000.json';new_path=HERE/'results/apple-m3-default-25000000.json'
old=json.loads(old_path.read_text());new=json.loads(new_path.read_text())
rows=[];new_ids={r['id'] for r in new['programs']}
for path,report in [(old_path,old),(new_path,new)]:
 control=next(r['median_seconds'] for r in report['programs'] if r['id']=='fasta-gcc-2-llvm')
 for row in report['programs']:
  if path==old_path and (row['language']=='mech' or row['id'] in new_ids):continue
  rows.append(dict(row,measurement_batch=path.name,comparison_control_seconds=control,comparison_time_ratio=row['median_seconds']/control))
result=dict(new,programs=rows,excluded=old['excluded'],measurement_batches=[dict(path=p.name,sha256=run.sha(p),compiler_snapshot=d['compiler_snapshot']) for p,d in [(old_path,old),(new_path,new)]])
result['protocol']=dict(result['protocol'],samples='3–5',normalization='Per-batch median of C #2 LLVM is the time control; raw elapsed seconds and original reports are preserved.',selection_skip_regex=None)
def endpoints(cohort):
 summary=run.common.endpoint_summary([dict(r,median_seconds=r['comparison_time_ratio']) for r in cohort])
 summary['time_baseline_ratio_to_control']=summary.pop('time_baseline_seconds')
 summary['normalization_cohort']='Successfully validated local programs, with each measurement batch normalized to its C #2 LLVM control.'
 return summary
result['endpoints']=endpoints(rows);result['strict_endpoints']=endpoints([r for r in rows if r['algorithm']['strict']]);result['nonheroic_endpoints']=endpoints([r for r in rows if not r['upstream_starred']])
(HERE/'results/apple-m3-final-25000000.json').write_text(json.dumps(result,indent=2)+'\n')
print('combined',len(rows),'builds;',sum(r['algorithm']['strict'] for r in rows),'strict')

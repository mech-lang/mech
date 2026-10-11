#!/usr/bin/env python3
"""Compare complete revised Mech programs with the preserved upstream cohort."""
import argparse,copy,hashlib,json
from pathlib import Path
import run as benchmark
from corner_chart import normalized_endpoints
p=argparse.ArgumentParser();p.add_argument('revision',type=Path);p.add_argument('--output',type=Path,required=True);args=p.parse_args()
paths=[benchmark.HERE/'results/apple-m3-16000.json',args.revision]
reports=[json.loads(path.read_text()) for path in paths]
selected={};historical=[];batches={}
for path,report in zip(paths,reports):
 control=next(r for r in report['programs'] if r['id']=='mandelbrot-rust-4')['median_seconds']
 batches[path.name]={'path':path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'samples':report['protocol']['samples'],'reference_id':'mandelbrot-rust-4','reference_median_seconds':control,'compiler_snapshot':report['compiler_snapshot']}
 assert report['protocol']['size']==16000 and report['protocol']['requested_threads']==8
 for original in report['programs']:
  row=copy.deepcopy(original)
  row.update(measurement_batch=path.name,reference_median_seconds=control,comparison_time_ratio=row['median_seconds']/control)
  if row['language']=='mech' and not row.get('source_program'):historical.append(row)
  else:selected[row['id']]=row
rows=list(selected.values());data=copy.deepcopy(reports[-1]);data.pop('compiler_snapshot')
data.update(programs=rows,historical_programs=historical,measurement_batches=batches,endpoints=normalized_endpoints(rows),nonheroic_endpoints=normalized_endpoints([r for r in rows if not r['upstream_starred']]),exact_image_endpoints=normalized_endpoints([r for r in rows if r['warmup_image']['exact_reference_match']]),excluded=reports[0]['excluded']+reports[-1]['excluded'])
data['evaluation_status']={'kind':'complete_source_program','compiler_status':'Mech-defined kernels use canonical ProgramArtifact compiler; new engine grid coordinator, standard-library catalog and opt-in mech build --backend cranelift pathway.','source_scope':'Complete Mech source including arguments, grid/map-until, the Mech-defined kernel and PBM output; general library scheduler/codec excluded.','previous_claim':'Older kernel-only source-size claims remain withdrawn; raw studies and historical_programs preserved.'}
data['protocol'].update(samples_label='3 baseline runs; 5 per revised build',chart_note='Preserved upstream measurements plus newly measured complete Mech source programs; each batch normalized to its own Rust #4 control.',status='complete upstream baseline with revised source-program Mech cohort')
data['protocol'].pop('sample_order',None)
args.output.write_text(json.dumps(data,indent=2)+'\n');print(args.output)

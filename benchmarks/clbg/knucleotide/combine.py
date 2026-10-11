#!/usr/bin/env python3
"""Combine controlled batches, retaining raw medians and reproducible provenance."""
import argparse,hashlib,json,statistics
from pathlib import Path
from datetime import datetime,timezone
HERE=Path(__file__).resolve().parent

def combine(paths, output):
 reports=[json.loads(p.read_text()) for p in paths]
 selected={};batches=[];groups=[];origins={}
 for path,report in zip(paths,reports):
  reference=next(r for r in report['programs'] if r['id']=='knucleotide-rust-7')
  control=reference['median_seconds'];by_executable={}
  for row in report['programs']:
   if row['language']=='mech':
    policy=row.get('runtime_environment',{'MECH_SEQUENCE_PARTITIONS':'1','MECH_SEQUENCE_NARROW':'0','MECH_SEQUENCE_PROFILE':'0'})
    key=(row['executable_sha256'],json.dumps(policy,sort_keys=True))
    by_executable.setdefault(key,[]).append(row)
  pooled={}
  for (digest,policy),members in by_executable.items():
   if len(members)<2:continue
   samples=[x for r in members for x in r['samples_seconds']]
   median=statistics.median(samples)
   group={'measurement_batch':path.name,'executable_sha256':digest,'runtime_environment':json.loads(policy),'members':[r['id'] for r in members],'samples_seconds':samples,'median_seconds':median}
   groups.append(group)
   for row in members:pooled[row['id']]=group
  batches.append({'path':path.name,'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'compiler_snapshot':report['compiler_snapshot'],'reference_id':reference['id'],'reference_median_seconds':control,'samples':report['protocol']['samples']})
  for original in report['programs']:
   row=original.copy()
   if row['language']=='mech':
    digest=row['executable_sha256'];origins.setdefault(digest,{'report':path.name,'compiler_snapshot':report['compiler_snapshot']})
    row['build_provenance']=row.get('build_provenance',origins[digest])
   group=pooled.get(row['id']);median=group['median_seconds'] if group else row['median_seconds']
   row.update(measurement_batch=path.name,comparison_median_seconds=median,comparison_time_ratio=median/control,reference_median_seconds=control)
   if group:row['equivalent_executable_group']={'sha256':group['executable_sha256'],'members':group['members'],'pooled_sample_count':len(group['samples_seconds'])}
   # A repeated configuration uses the latest completed measurement, rather
   # than selecting whichever batch happened to record a lower time.
   selected[row['id']]=row
 data=reports[0].copy();data.pop('compiler_snapshot');data.pop('endpoints');data.pop('nonheroic_endpoints');data.pop('strict_endpoints')
 # Compare one compiler revision per language. Earlier Mech compiler/runtime
 # revisions remain in the history and raw reports, including faster ones.
 hot_paths=['hosts/gpu/src/sequence/compiler.rs','hosts/gpu/src/sequence/runtime.rs','hosts/gpu/src/sequence/counts.rs','hosts/gpu/Cargo.toml','benchmarks/clbg/knucleotide/mech-driver/src/main.rs']
 if 'src/engine/src/native_sequence/mod.rs' in reports[-1]['compiler_snapshot']['source_sha256']:
  hot_paths += ['src/engine/src/native_sequence/mod.rs','src/stdlib/src/native_sequence.rs']
 current=reports[-1]['compiler_snapshot']['source_sha256']
 def is_current(row):
  if row['language']!='mech':return True
  hashes=row['build_provenance']['compiler_snapshot']['source_sha256']
  return all(hashes.get(p)==current.get(p) for p in hot_paths)
 cohort=[r for r in selected.values() if is_current(r)]
 history=[r for r in selected.values() if not is_current(r)]
 data.update(schema_version=2,created_at=datetime.now(timezone.utc).isoformat(),programs=cohort,historical_programs=history,measurement_batches=batches,equivalent_executable_groups=groups,current_mech_backend_sha256={p:current[p] for p in hot_paths})
 data['evaluation_status']={'kind':'api_composition_prototype','cross_language_implementation_size_claim':'withdrawn_pending_compiler_library_boundary','reason':'The experimental frontend admits a small call graph outside the general Mech compiler/catalog and dispatches substantial counting and output composition to its Rust pipeline. General counting and sequence standard-library operations are permitted; the boundary requires reusable compiler-visible operations and native lowering, with benchmark-specific composition in Mech source.','timing_scope':'Measured Cranelift AOT window traversal plus Rust library/runtime pipeline.'}
 if 'src/engine/src/native_sequence/mod.rs' in current:
  data['evaluation_status']={'kind':'complete_source_program','compiler_status':'New engine typed sequence IR, standard-library catalog and Cranelift build pathway; opt-in cranelift_build feature. Sequence IR remains distinct from reactive resident ProgramArtifact.','source_scope':'Complete Mech program including source-defined frequency, sorting, arithmetic and lookup composition. General library operations excluded.','previous_claim':'Original eight-line prototype source-size claim remains withdrawn; preserved as history.'}
  data['protocol']['source_scope']=reports[-1]['protocol']['source_scope']
 data['protocol']=data['protocol']|{'samples_label':'3 baseline runs; 5 per improvement/revision; 11 in previous confirmation','comparison_normalization':'Each time divided by the Rust #7 median in the same batch; raw samples/medians retained. Repeated configurations use the latest batch. Byte-identical Mech executables with identical runtime policy in one batch share the median of their pooled samples.','source_reports':[p.name for p in paths],'mech_cohort':'Only the current compiler/runtime revision appears in the corner chart. Every earlier Mech revision, whether faster or slower, remains in historical_programs and its original raw report.'}
 output.write_text(json.dumps(data,indent=2)+'\n');print(output)
if __name__=='__main__':
 p=argparse.ArgumentParser(description=__doc__);p.add_argument('reports',type=Path,nargs='+');p.add_argument('--output',type=Path,required=True);a=p.parse_args();combine(a.reports,a.output)

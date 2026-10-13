"""Seal individually qualified V studies; V remains report-only throughout."""
import argparse
import copy
import fcntl
import json
from pathlib import Path
import re
import full_study
import prepare

HERE=Path(__file__).resolve().parent
CONTRACTS={
 'regexredux':'Complete stdin cleanup, all nine nonoverlapping regex counts, and all five ordered replacements; eight count workers join before the remaining count.',
 'revcomp':'Complete multirecord input, actual per-base complement/reversal, prescribed headers and 60-column output including EOF and CRLF cases.',
 'spectralnorm':'Forty fresh implicit-matrix products; every coefficient evaluated again for each product, without a cached coefficient matrix.',
 'mandelbrot':'Every requested pixel, up to fifty actual recurrence/escape iterations, full PBM payload and correct partial-byte padding.',
 'fasta':'Every prescribed LCG draw with carried seed, direct binary64 division and sequential unscaled CDF comparisons, complete output.',
 'nbody':'Five bodies, every one of ten unordered pairs on every prescribed step, exact square root/division, energy before and after.',
 'fannkuchredux':'Every factoradic-order permutation, actual prefix reversals, alternating global checksum and maximum flips.',
 'pidigits':'Both independent arbitrary-precision quotient extractions on every tested candidate; full prescribed decimal output.',
 'knucleotide':'All seven growable maps, widths1/2/3/4/6/12/18, every window including the one/two-base maps, and exact THREE record selection.',
 'binarytrees':'Real distinct interior and leaf nodes, two independently constructed children, prescribed temporary iterations and retained long-lived graph.',
}

def selected_work_profiles(profiles):
 assert len(profiles)==2 and {row['variant'] for row in profiles}=={'compact','optimized'}
 pairs={(row['source_sha256'],row['variant']):row for row in profiles}
 assert len(pairs)==2 and len({row['id'] for row in profiles})==2
 return pairs

def required_work_cases(benchmark):
 if benchmark=='pidigits':return {'digits-97','zero-digits'}
 if benchmark=='binarytrees':return {'nodes-6','nodes-7','nodes-9'}
 assert benchmark=='knucleotide'
 names=[name for name,_ in prepare.input_cases('knucleotide')]+['lookalike-target','exact-token-and-cr']
 assert len(names)==len(set(names)),'Work-case inventory has a duplicate'
 return set(names)

def validate_work_coverage(work,profiles,benchmark,source_revision):
 """Both actual source forms need every independent work case exactly once."""
 selected=selected_work_profiles(profiles)
 cases=required_work_cases(benchmark)
 field='source_sha256' if benchmark=='binarytrees' else 'scored_source_sha256'
 expected={(source,variant,case) for source,variant in selected for case in cases}
 assert work['benchmark']==benchmark and work['source_revision']==source_revision
 assert work['status']==('passed-real-node-and-retention-traces' if benchmark=='binarytrees' else 'passed')
 assert work['pins_unchanged_at_end'] is True and work['active'] is None
 assert work['full_performance_runs']==0 and work['never_chart_eligible'] is True
 assert not work.get('error') and not work.get('error_type')
 assert work['measured_sources_unchanged' if benchmark=='binarytrees' else 'measured_source_bytes_unchanged'] is True
 attempts={attempt['attempt']:attempt for attempt in work['attempts']}
 assert len(attempts)==len(work['attempts'])==len(expected),'Actual work attempts are missing or duplicated'
 assert set(attempts)==set(range(1,len(expected)+1))
 observed=set();used_attempts=set()
 for review in work['work_reviews']:
  assert review['status']=='passed' and review['benchmark']==benchmark
  key=(review[field],review['variant'],review['case'])
  assert key in expected and key not in observed,'A work case is missing, duplicated, or belongs to another source'
  observed.add(key)
  attempt=attempts[review['attempt']]
  assert review['attempt'] not in used_attempts,'Two reviews point to the same actual native execution'
  used_attempts.add(review['attempt'])
  original=selected[key[:2]]
  suffix='-tree-work' if benchmark=='binarytrees' else '-work-trace'
  assert attempt['id']==original['id']+suffix and attempt['case']==review['case']
  assert attempt['benchmark']==benchmark and attempt['qualified'] is True and attempt['full_workload'] is False
  assert attempt['source_sha256']==review['trace_source_sha256']
  assert type(attempt['exit_status']) is int and type(attempt['sampler_wait_status']) is int
  assert attempt['exit_status']==attempt['sampler_wait_status']==0
  assert attempt['pins_unchanged_at_end'] is True and attempt['validation']['reference_match'] is True
  assert attempt['never_chart_eligible'] is True and attempt['full_performance_run'] is False
  # These actual V wrappers use the older shared supervisor. Its raw schema
  # has an explicit fatal list, not remaining_owner's20-check dictionary.
  assert 'qualification_checks' not in attempt and attempt['fatal_qualification_fields']==[]
  assert attempt['valid_environment'] is True and attempt['retryable_environment_only'] is False
  assert attempt['power_at_start']==attempt['power_at_end']==dict(source='AC Power',low_power_mode=0)
  assert all(attempt[name]==[] for name in ('remaining_group_members','monitor_errors',
    'resource_monitor_errors','supervisor_remaining_members','competing_compilers','power_changes'))
  assert attempt['maximum_rss_mib']==8192 and attempt['timeout_seconds']==300
  assert type(attempt['peak_process_group_rss_kib']) is int and 0<=attempt['peak_process_group_rss_kib']<=8192*1024
  assert not any(attempt.get(name) for name in ('error','resource_violation','cleanup_error',
    'sampler_failure','supervisor_failure','supervisor_pin_failure','source_work_error',
    'supervisor_cleanup','supervisor_registration_error','monitor_finish_error','supervisor_error'))
  fingerprint=json.loads(attempt['execution_fingerprint'])
  assert fingerprint['source_sha256']==review['trace_source_sha256']
  assert fingerprint['environment']==attempt['runtime_environment'] and fingerprint['cwd']==attempt['cwd']
  command=[part.replace('{n}',str(attempt['workload_n'])) for part in fingerprint['command']]
  assert command==attempt['command'] and work['pins'][fingerprint['command'][0]]==fingerprint['binary_sha256']
  native=attempt['supervised_native_group']
  assert native['type']=='native-group-started' and type(native['pid']) is int and native['pid']>0
  assert native['pid']==native['pgid'] and native['command']==command and native['inherited_common_mutex'] is True
  if benchmark=='binarytrees':
   assert review['n']==int(review['case'].split('-')[1]) and review['full_workload'] is False
   assert review['n']==attempt['workload_n']
   assert all(review[name] is True for name in ('distinct_nodes_and_leaves_per_root',
     'retained_root_and_all_edges_identical','temporary_graphs_disjoint_from_live_retained_graph',
     'complete_stdout_match','creator_check_and_node_declaration_unchanged'))
  elif benchmark=='pidigits':
   assert review['n']==(97 if review['case']=='digits-97' else 0)
   assert review['n']==attempt['workload_n']
   assert all(review[name] is True for name in ('both_actual_extracts_per_candidate',
     'distinct_endpoint_numerators','all_numerators_and_quotients_reconciled','stdout_unchanged'))
  else:
   assert all(review[name] is True for name in ('all_seven_maps_reconciled',
     'all_overlapping_windows','first_two_maps_included','stdout_unchanged'))
 assert observed==expected and used_attempts==set(attempts),'Incomplete two-form work coverage'

def validate_allocation_approval(review,profiles,source_revision,module):
 """Bind manual allocation review to both exact unchanged generated-C cohorts.

 Required review fields are explicit because this approval does not yet exist.
 Each creator/leaf_allocator proof references a pinned actual generated C file
 and its inspected C symbol. No generated C is executable benchmark source.
 """
 selected=selected_work_profiles(profiles)
 assert review['status']=='passed' and review['source_revision']==source_revision
 assert review['pins_unchanged_at_end'] is True
 assert review['both_recursive_children'] is True and review['interior_and_leaf_heap_allocations'] is True
 ref=review['lowering_evidence'];path=Path(ref['path']).resolve(strict=True)
 assert path.is_relative_to(HERE/'lowering-evidence') and path.suffix=='.json'
 assert module.sha(path)==ref['sha256']==review['pins'][str(path)]
 lowering=json.loads(path.read_text())
 assert lowering['status']=='generated-C-retained-awaiting-root-allocator-review'
 assert lowering['source_revision']==source_revision and lowering['pins_unchanged_at_end'] is True
 assert lowering['native_benchmark_runs']==lowering['full_performance_runs']==0
 assert lowering['never_chart_eligible'] is True
 lowered={(row['source_sha256'],row['variant']):row for row in lowering['profiles']}
 approvals={(row['source_sha256'],row['variant']):row for row in review['profiles']}
 assert len(lowering['profiles'])==len(lowered)==len(review['profiles'])==len(approvals)==2
 assert set(lowered)==set(approvals)==set(selected),'Allocation review lacks either selected source form'
 pins=prepare.merge_pins(review['pins'],lowering['pins'],{str(path):ref['sha256']})
 for key,original in selected.items():
  produced,approval=lowered[key],approvals[key]
  assert produced['id']==approval['id']==original['id'] and approval['status']=='passed'
  assert approval['both_recursive_children'] is True and approval['interior_and_leaf_heap_allocations'] is True
  source=Path(produced['source']).resolve(strict=True)
  assert source.is_relative_to(HERE/'lowering-evidence') and module.sha(source)==key[0]==pins[str(source)]
  assert produced['binary_executed'] is False
  command=produced['build_command'];flags=original['compiler_flags']
  assert command[0]==original['build_command'][0]==original['runtime']['path']
  assert pins[command[0]]==original['runtime']['sha256']
  assert command[1:1+len(flags)]==flags
  assert command[1+len(flags):]==['-keepc','-o',command[-2],str(source)]
  generated={str(Path(name).resolve(strict=True)) for name in produced['generated_c']}
  assert generated and all(Path(name).is_relative_to(source.parent/'compiler-temp') for name in generated)
  for name in generated:
   assert Path(name).suffix=='.c' and module.sha(name)==pins[name]
  for name in ('creator_proof','leaf_allocator_proof'):
   proof=approval[name];filename=str(Path(proof['path']).resolve(strict=True))
   assert filename in generated and proof['sha256']==pins[filename]
   symbol=proof['symbol'];assert isinstance(symbol,str) and re.fullmatch(r'[A-Za-z_][A-Za-z0-9_]*',symbol)
   assert re.search(r'\b'+re.escape(symbol)+r'\s*\(',Path(filename).read_text()),'Reviewed C symbol absent from actual generated file'
 return pins

def seal(args):
 module=full_study.configure(args.benchmark)
 print('WAIT common mutex: V full-study manifest',args.benchmark,flush=True)
 with module.LOCK.open('a+') as lock:
  fcntl.flock(lock,fcntl.LOCK_EX)
  assert not module.MANIFEST.exists() and not module.REPORT.exists(),'No overwrite or implicit continuation'
  qualification=HERE/'qualification'/args.source_revision/args.benchmark/'probes.json'
  qualified=json.loads(qualification.read_text())
  assert qualified['status']=='bounded-outputs-passed' and qualified['pins_unchanged_at_end'] is True
  assert qualified['benchmark']==args.benchmark and len(qualified['profiles'])==2
  fixture=HERE/'input-v2-fixtures.json'
  source_receipt=json.loads(fixture.read_text())
  assert source_receipt['status']=='passed' and source_receipt['tests_run']==8
  draft=HERE/'revisions/input-v2/profiles.json'
  sources=json.loads(draft.read_text())['profiles']
  harness=module.load_harness()
  pins=prepare.merge_pins(qualified['pins'],source_receipt['pins'])
  evidence=[qualification,fixture,draft,Path(__file__).resolve(),Path(full_study.__file__),full_study.HELPER]
  work=None
  if args.benchmark in ('pidigits','knucleotide','binarytrees'):
   assert args.work_report and args.work_report_sha256,'Actual extractions/map/node traces required'
   path=Path(args.work_report).resolve(strict=True)
   assert path.is_relative_to(HERE/'work-qualification') and module.sha(path)==args.work_report_sha256
   work=json.loads(path.read_text())
   validate_work_coverage(work,qualified['profiles'],args.benchmark,args.source_revision)
   pins=prepare.merge_pins(pins,work['pins']);evidence.append(path)
   if args.benchmark=='binarytrees':
    assert args.allocation_review and args.allocation_review_sha256,'Root review of actual production creator/allocator C required'
    path=Path(args.allocation_review).resolve(strict=True)
    assert path.is_relative_to(HERE) and module.sha(path)==args.allocation_review_sha256
    review=json.loads(path.read_text())
    pins=prepare.merge_pins(pins,validate_allocation_approval(review,qualified['profiles'],args.source_revision,module));evidence.append(path)
  profiles=[]
  for original in qualified['profiles']:
   row=copy.deepcopy(original)
   source=(HERE/next(item['source'] for item in sources if item['id']==row['id'])).resolve(strict=True)
   assert module.sha(source)==row['source_sha256']==source_receipt['pins'][str(source)]
   assert module.sha(row['source'])==row['source_sha256'] and module.sha(row['binary'])==row['binary_sha256']
   assert row['small_validation']['reference_match'] is True
   assert row['algorithm']['strict'] is True and row['algorithm']['native_benchmark_kernel'] is False
   assert row['maximum_workers']<=8
   row.update(loc=None,loc_status='pending immutable canonical LOC and semantic review',
       eligible_for_chart=False,never_chart_eligible=True,report_only=True,
       required_work_review=dict(status='passed',source_sha256=row['source_sha256'],
           method='Root review of both complete language sources plus actual independent bounded outputs and required trace evidence',
           contract=CONTRACTS[args.benchmark],qualification=str(qualification)))
   profiles.append(row)
  control_path=full_study.SUITE/'source-audit-r.json'
  audit=json.loads(control_path.read_text());assert audit['status']=='reviewed'
  control=copy.deepcopy(next(row for row in audit['controls'] if row['benchmark']==args.benchmark))
  evidence.append(control_path)
  if args.benchmark=='mandelbrot':
   path=full_study.CLBG/'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
   control=copy.deepcopy(next(row for row in json.loads(path.read_text())['profiles'] if row['id']=='row16-aot-control'))
   evidence.append(path)
  assert module.sha(control['source'])==control['source_sha256'] and module.sha(control['binary'])==control['binary_sha256']
  pins=prepare.merge_pins(pins,control.get('support',{}),
    {str(Path(control['source'])):control['source_sha256'],str(Path(control['binary'])):control['binary_sha256']},
    {str(path):module.sha(path) for path in evidence})
  control.update(id='mech-aot-control',key=args.benchmark+':v-input-v2-study:mech-control',
    producer_loc=control.get('loc'),loc=None,loc_status='pending canonical count overlay',maximum_workers=8,
    role='Fresh unchanged current Mech AOT fastest-source control')
  profiles.append(control)
  ids=[row['id'] for row in profiles]
  manifest=dict(status='prepared-awaiting-root-review',benchmark=args.benchmark,created_at=module.now(),
    samples=3,size=harness.WORK[args.benchmark][1],profiles=profiles,
    orders=[dict(round=index+1,profiles=ids[index:]+ids[:index]) for index in range(3)],
    pins=pins,shared_timing_lock=str(module.LOCK),required_power_profile=harness.REQUIRED,
    maximum_rss_mib=8192,timeout_seconds=14400,maximum_compute_workers=8,environment_retry_limit=3,
    qualification=str(qualification),source_revision=args.source_revision,report_only=True,never_chart_eligible=True,
    timing_boundary='Fresh process startup, all prescribed work, complete input/output and shutdown; build/qualification excluded.')
  harness.PinGuard(pins).verify()
  module.save(module.MANIFEST,manifest)
  print(json.dumps(dict(manifest=str(module.MANIFEST),sha256=module.sha(module.MANIFEST),profiles=ids,full_processes=9,report_only=True)),flush=True)

def main():
 parser=argparse.ArgumentParser()
 parser.add_argument('--benchmark',choices=full_study.BENCHMARKS,required=True)
 parser.add_argument('--source-revision',default='production-input-v2')
 parser.add_argument('--work-report');parser.add_argument('--work-report-sha256')
 parser.add_argument('--allocation-review');parser.add_argument('--allocation-review-sha256')
 seal(parser.parse_args())
if __name__=='__main__':main()

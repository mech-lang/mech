from pathlib import Path
from collections import Counter, defaultdict
import gzip, hashlib, json, math, random, statistics, struct

P=Path(__file__).parent
sha=lambda b:hashlib.sha256(b).hexdigest()
def blob(name):
    f=P/name
    if not f.exists():f=Path(str(f)+'.gz')
    b=f.read_bytes();return gzip.decompress(b) if f.suffix=='.gz' else b
def read(name):return json.loads(blob(name))
manifest=read('manifest.json')
for name,v in manifest['files'].items():
    b=(P/name).read_bytes();assert len(b)==v['bytes'] and sha(b)==v['sha256']
    raw=gzip.decompress(b) if v['gzip'] else b
    assert len(raw)==v['decoded_bytes'] and sha(raw)==v['decoded_sha256']
report=read('compiler-native-latest-deflate-integration-20261108.json')
assert report['selected'] and report['working_baseline_adoption']['adopted'] and not report['goal_complete']
assert report['production_backend']=='Cranelift' and not report['LLVM_mandatory']
oracle=read('outputs/oracle.json');checked=set();cohort_total=0
def exact_output(key):
    if key not in checked:
        b=blob('outputs/'+key+'.json');assert sha(b)==key and json.loads(b)==oracle;checked.add(key)
def quant(values,q):
    pos=(len(values)-1)*q;i=math.floor(pos);j=math.ceil(pos)
    return values[i]+(values[j]-values[i])*(pos-i)

for prefix,protocol,a,rule,nlat,nres in [
    ('native-latest-deflate-integration','protocol',report['first_analysis'],report['prospective_rule'],1023,238),
    ('native-specialized-rows-confirmation','confirmation',report['confirmation_analysis'],report['confirmation_rule'],868,203)]:
    assert sha(blob(protocol+'/prospective-rule.json'))==a['rule_sha256']
    lat=read('cohorts/comparison-'+prefix+'-20261108-5000-m3.json')
    resources=read('cohorts/comparison-'+prefix+'-resources-20261108-5000-m3.json')
    for cohort,count in [(lat,nlat),(resources,nres)]:
        assert len(cohort['samples'])==count
        for row in cohort['samples']:
            assert row['exit']==0 and row['exact_rust_output'];exact_output(row['output_sha256']);cohort_total+=1
    config=read(protocol+'/latency-config.json')
    rust=[v for v in config['implementations'] if v['kind']!='mech'];assert len(rust)==22
    eligible=[v['name'] for v in rust if v['calculation_boundary']=='post_json_including_setup']
    best_post=min(eligible,key=lambda name:statistics.median(x.get('post_json_seconds',x['seconds']) for x in lat['samples'] if x['variant']==name))
    best_whole=min([v['name'] for v in rust],key=lambda name:statistics.median(x['wall_seconds'] for x in lat['samples'] if x['variant']==name))
    assert a['fastest_processing_Rust']['variant']==best_post and a['fastest_whole_Rust']['variant']==best_whole
    post=statistics.median(x.get('post_json_seconds',x['seconds']) for x in lat['samples'] if x['variant']==best_post)
    whole=statistics.median(x['wall_seconds'] for x in lat['samples'] if x['variant']==best_whole)
    for name,v in a['medians'].items():
        rows=[x for x in lat['samples'] if x['variant']==name];assert len(rows)==31
        for field in ['post_json_seconds','wall_seconds','compile_or_restore_seconds']:
            assert v[field]==statistics.median(x[field] for x in rows)
        assert math.isclose(v['times_fastest_processing_Rust'],v['post_json_seconds']/post)
        assert math.isclose(v['times_fastest_whole_Rust'],v['wall_seconds']/whole)
    for name,expected in a['resource_medians'].items():
        rows=[x for x in resources['samples'] if x['variant']==name+'-instrumented'];assert len(rows)==7
        for label,field in [('restore','restore_measurement'),('post_JSON','measurement')]:
            for metric,value in expected[label].items():
                values=[x[field]['hardware'][metric] if metric in ['instructions_retired','cycles'] else x[field][metric] for x in rows]
                assert statistics.median(values)==value
        for metric,value in expected['process_resources'].items():
            assert statistics.median(x['process_resources'][metric] for x in rows)==value
    for candidate,d in a['decisions'].items():
        assert not d['whole_process_required_gate'] and not d['selected']
        for reference,comparisons in d['paired'].items():
            for boundary,v in comparisons.items():
                values={name:{x['sample']:x[boundary] for x in lat['samples'] if x['variant']==name} for name in [candidate,reference]}
                assert all(set(x)==set(range(31)) for x in values.values())
                changes=[values[candidate][i]/values[reference][i]-1 for i in range(31)]
                rng=random.Random(rule['bootstrap']['seed'])
                boots=sorted(statistics.median(changes[rng.randrange(31)] for _ in range(31)) for _ in range(rule['bootstrap']['resamples']))
                assert v['wins']==sum(x<0 for x in changes) and v['median_paired_relative_change']==statistics.median(changes)
                assert v['two_sided_95_interval']==[quant(boots,.025),quant(boots,.975)] and v['one_sided_95_upper']==quant(boots,.95)
        parent=rule['per_candidate_matched_parents'][candidate]
        target=d['paired'][parent]['post_json_seconds']['two_sided_95_interval'][1]<0
        assert d['target_passed']==d['first_cohort_passed']==target
        before=a['resource_medians'][parent]['post_JSON']['instructions_retired']
        after=a['resource_medians'][candidate]['post_JSON']['instructions_retired']
        assert d['processing_instruction_ratio']==after/before and d['processing_instruction_change']==after-before
    assert read(protocol+'/latency-quiet.json')['quiet'] and read(protocol+'/resources-quiet.json')['quiet']

assert cohort_total==2332
def text_bytes(b):
    assert struct.unpack_from('<I',b)[0]==0xfeedfacf;pos=32
    for _ in range(struct.unpack_from('<I',b,16)[0]):
        cmd,size=struct.unpack_from('<II',b,pos)
        if cmd==0x19:
            for i in range(struct.unpack_from('<I',b,pos+64)[0]):
                at=pos+72+80*i
                if b[at:at+16].split(b'\0')[0]==b'__text':
                    length=struct.unpack_from('<Q',b,at+40)[0];offset=struct.unpack_from('<I',b,at+48)[0]
                    return b[offset:offset+length]
        pos+=size
    raise AssertionError('No native Object text')
assert text_bytes(blob('Objects/plain/object.o'))==text_bytes(blob('Objects/IR279-plain/object.o'))
for label in ['plain','triple-rows','quartet-rows','both-rows']:
    m=read('Objects/'+label+'/metadata.json');audit=read('runners/'+label+'/native-link-proof.json')
    assert (m['typed_ir_version'],m['work_policy_version'],m['native_descriptor_version'],m['abi_version'])==(280,116,11,41)
    assert len(m['compiled_numeric_functions'])==51 and audit['status']=='passed'
    assert sha(blob('Objects/'+label+'/object.o'))==audit['object']['sha256']
    assert audit['normalized_text_sha256']==report['native_code'][label]['normalized_text_sha256']
    assert m['code_bytes']==report['native_code'][label]['code_bytes'] and m['stack_bytes']==report['native_code'][label]['stack_bytes']
assert sha(blob('Objects/both-rows/source.mec'))==report['source_hash']

refs=read('qualification/refs.json');assert len(refs)==140
for q in refs:
    inp=blob('inputs/qualification/'+q['input_sha256']+'.json')
    expected=blob('outputs/'+q['expected_sha256']+'.json');actual=blob('outputs/'+q['actual_sha256']+'.json')
    assert sha(inp)==q['input_sha256'] and sha(expected)==q['expected_sha256'] and sha(actual)==q['actual_sha256']
    assert json.loads(expected)==json.loads(actual)
    if 'expected_indices_sha256' in q:
        posts=json.loads(inp);indices=read('inputs/qualification-indices/'+q['expected_indices_sha256']+'.json')['related_indices']
        assert json.loads(expected)==[{'_id':p['_id'],'tags':p['tags'],'related':[posts[i] for i in ids]} for p,ids in zip(posts,indices)]
listed=read('tests/ci-list.json');executed=read('tests/ci-execute.json')
names=[x.removesuffix(': test') for x in listed['stdout'].splitlines() if x.endswith(': test')]
passed=[x.removeprefix('test ').removesuffix(' ... ok') for x in executed['stdout'].splitlines() if x.startswith('test ') and x.endswith(' ... ok')]
assert listed['exit']==executed['exit']==0 and sorted(names)==sorted(passed) and len(names)==415
previous=read('previous-native-CI-list.json');old=[x.removesuffix(': test') for x in previous['stdout'].splitlines() if x.endswith(': test')]
assert len(old)==408 and set(old)<=set(names)
for filename,count in [('newest-deflate-tests.json',7),('recovered-scalar-home-tests.json',3),('packed-row-and-route-tests.json',8),('image-and-caller-geometry-tests.json',27),('optional-llvm-regressions.json',4)]:
    d=read('tests/'+filename);assert d['exit']==0 and f'{count} passed' in d['stdout']
assert read('tests/test-source-inventory.json')['files']==read('compiler-build/source-inventory.json')['files']
for kind,count in [('scalar',128),('one-kib',5)]:
    def rows(d):return [{k:v for k,v in json.loads(line).items() if k not in ['seconds','seconds_per_evaluation']} for line in d['stdout'].splitlines() if line.startswith('{')]
    before=rows(read('IR240-controls/'+kind+'-run.json'));after=rows(read('scalar-DEFLATE/'+kind+'-run.json'))
    assert len(before)==len(after)==count and before==after
for filename in ['first-native-tests-harness-failure.json','first-upstream-geometry-tests.json','first-scalar-deflate-link-disk-failure.json']:
    assert read('protocol/'+filename)['exit']!=0

# Independently reconstruct disjoint intervals on each calling thread.
priority=['input_admission','input_views','result_construction','session_setup','worker_dispatch','runtime_callbacks','numeric_native','other_generated_entry','runtime_outside_entry','other_driver']
def tag(e):
    stage=e['stage']
    if stage=='post_views':return 'input_views'
    if stage in ['input_admission','result_construction','session_setup']:return stage
    if stage=='helper_ffi' and e['operation']==(1<<64)-21:return 'worker_dispatch'
    if stage in ['helper_ffi','admit']:return 'runtime_callbacks'
    if stage=='native_body':return 'numeric_native'
    if stage=='generated_entry':return 'other_generated_entry'
    if stage in ['evaluate','execute_prepared_after_setup','worker']:return 'runtime_outside_entry'
def span(e,axis):
    return (e['start_ns'],e['end_ns']) if axis=='elapsed_ns' else (e['start_counters'][axis],e['end_counters'][axis])
def partition(events,thread,root,axis):
    event=next(e for e in events if e['thread']==thread and e['stage']==root);lo,hi=span(event,axis)
    edges=defaultdict(Counter)
    for e in events:
        if e['thread']!=thread or not (category:=tag(e)):continue
        start,end=span(e,axis);start=max(start,lo);end=min(end,hi)
        if start<end:edges[start][category]+=1;edges[end][category]-=1
    active=Counter();parts=Counter();previous=lo
    for at in sorted({lo,hi,*edges}):
        category=next((p for p in priority if active[p]>0),'other_driver');parts[category]+=at-previous
        active.update(edges[at]);assert all(v>=0 for v in active.values());previous=at
    assert sum(parts.values())==hi-lo
    return dict(total=hi-lo,categories=dict(parts))
profiles=read('diagnostics/profiles.json');assert len(profiles)==48
analysis=read('diagnostics/interval-analysis.json');captures=0
for row in profiles:
    assert row['exit']==0 and row['exact_Rust_output'];exact_output(row['output_sha256'])
    if row['kind']!='profile':continue
    path=Path(row['trace']).relative_to('/private/tmp/mech-related-specialized-rows-profile-20261108')
    trace=read('diagnostics/'+str(path));capture=trace['capture'];events=capture['events']
    assert capture['dropped']==capture['unmatched_ends']==capture['counter_errors']==0
    assert all(e['closed'] and e['start_counters'] and e['end_counters'] for e in events)
    assert sum(e['stage']=='native_body' for e in events)==23 and len({e['thread'] for e in events})==5
    expected=next(x for x in analysis['runs'] if x['variant']==row['variant'] and x['pair']==row['pair'])
    for axis in ['elapsed_ns','instructions','cycles']:
        assert expected['main'][axis]==partition(events,trace['main_thread'],'post_JSON',axis)
        for thread,worker in expected['workers'].items():assert worker[axis]==partition(events,int(thread),'worker',axis)
    captures+=1
assert captures==32
print(json.dumps(dict(records=len(manifest['files']),cohort_outputs_exact=cohort_total,qualification_outputs_exact=len(refs),native_CI_names_executed=415,scalar_DEFLATE_rows_exact=133,bootstrap_and_fastest_Rust_rederived=True,native_Object_identity_checked=True,profile_outputs_exact=48,profile_captures_partitions_rederived=32,first_failures_retained=True,working_baseline_adopted=True,goal_complete=False),indent=2))

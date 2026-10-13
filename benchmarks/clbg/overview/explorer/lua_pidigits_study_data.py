"""Add matched GMP source pools for Lua interpreter/JIT after full qualification."""
from copy import deepcopy
import json
from pathlib import Path
from isolated_study_data import validate_full_records
STUDIES={
 'lua':dict(cohort='lua-pidigits-comparison/20261012',digest='aaf6af5fad6bb4d2f9008e8f91be7037025e117f2cb9532dfb14ae928f6f55dd',runtime_sha='83ea84efb4f7692a4452cfedf9325cc1f342edc6cf0d50b262add4eab1b75bea',runtime_policy='Lua 5.5.1 · bytecode interpreter · GMP 6.3',prefix='lua-listed-'),
 'luajit':dict(cohort='luajit-pidigits-comparison/20261012',digest='8d07430a48e9613460f1d927a92c4ae88ba65e1e48a5e846bdd4c83ddebeeb5a',runtime_sha='ffe74a185b665faae02da56c194493c82d921afae03f3421b3d0a270886dd4c3',runtime_policy='LuaJIT 2.1 · default JIT · GMP 6.3',prefix='luajit-listed-'),
}
CONTROL_HASHES={'mech-fast-control':'7079dab9ee52637f5397547adac3a37cf73eda17c290ae5d6593ef45f93eeb8e','mech-small-control':'db8b2e388e9fb395183a73c2390ad2ff6ef1842d69b28db5b5c1200a35bdd5c1'}
PRODUCER_LOCS={1:62,5:77,6:63,7:57}

def _full_validation(row,reference,variant):
    """Preserve producer metadata while requiring its complete canonical output."""
    validation=row['full_validation']
    assert isinstance(validation,dict)
    assert type(validation['variant']) is int and validation['variant']==variant
    assert validation['kind']=='normalized'
    assert type(validation['n']) is int and validation['n']==10000
    assert validation['full_output_validation'] is True
    assert validation['command']==[part.replace('{n}','10000') for part in row['command']]
    assert validation['bytes']==reference['bytes'] and validation['sha256']==reference['sha256']
    assert validation['reference_match'] is True
    if 'canonical_full_workload' in validation:assert validation['canonical_full_workload'] is True
    if 'never_chart_eligible' in validation:assert validation['never_chart_eligible'] is False

def add_lua_pidigits_studies(root,data,load,measured,add_file,sha):
    root=Path(root)
    # Both complete source pools are needed before making a family comparison.
    for spec in STUDIES.values():
        path=root/spec['cohort']/'results/full-study.json'
        if not spec['digest'] or not path.is_file():return
        if json.loads(path.read_text()).get('status')!='complete':return
    reference_path=root/'pidigits/inputs/output-full.txt'
    reference=dict(bytes=reference_path.stat().st_size,sha256=sha(reference_path))
    benchmark=next(b for b in data['benchmarks'] if b['key']=='pidigits')
    audit=dict(status='complete',studies={},previous_endpoints={},library_sources={},admission_rule='All four listed application sources and both current Mech controls need three actual canonical N10000 records in both complete interpreter/JIT studies. No prior library or source timing is relabelled.',primary_comparison_policy='Every speed multiplier names its current freshly measured fast38 Mech AOT source. Fastest/fewest source LOC comparisons retain their recorded38/35 sources; the compact35 control is independently measured and inspectable.')
    for language,spec in STUDIES.items():
        manifest_path=spec['cohort']+'/full-study/manifest.json';report_path=spec['cohort']+'/results/full-study.json'
        assert sha(root/manifest_path)==spec['digest'],'Reviewed Lua-family study manifest changed'
        manifest,report=load(manifest_path),load(report_path)
        assert Path(report['manifest']).resolve()==(root/manifest_path).resolve()
        locs={spec['prefix']+str(v):loc for v,loc in PRODUCER_LOCS.items()}|{'mech-fast-control':38,'mech-small-control':35}
        rows,discarded=validate_full_records(manifest,report,spec['digest'],benchmark='pidigits',size=10000,locs=locs,workers=1,reference=reference,environment_retries=True)
        assert manifest['sample_count']==3 and manifest['full_performance_runs']==0 and manifest['preparation_full_output_checks']==10
        validation_path=manifest['validation'];assert sha(validation_path)==manifest['validation_sha256']
        validation=load(Path(validation_path).relative_to(root).as_posix())
        assert validation['status']=='passed' and validation['scored_samples']==0 and validation['generic_primitive_tests'] is True
        assert validation['application_output_checks']==248 and validation['mech_output_checks']==18
        assert len(validation['division_checks'])==4 and {r['variant'] for r in validation['division_checks']}==set(PRODUCER_LOCS)
        for check in validation['division_checks']:
            assert check['n']==97 and check['accepted_digits']==97 and check['actual_divisions']==2*check['tested_candidates']
            assert check['both_exact_divisions_per_candidate'] is True and check['quotients_independently_verified'] is True
            assert sha(check['path'])==check['sha256']
        for path,digest in manifest['pins'].items():assert sha(path)==digest,('Lua study pin differs',path)
        fast_control,small_control=rows['mech-fast-control'],rows['mech-small-control']
        for control in (fast_control,small_control):
            assert control['language']=='mech' and control['source_sha256']==CONTROL_HASHES[control['id']]
            assert manifest['pins'][control['binary']]==control['binary_sha256']
        candidates=[rows[spec['prefix']+str(v)] for v in PRODUCER_LOCS]
        for variant,row in zip(PRODUCER_LOCS,candidates):
            assert row['language']==language and row['runtime']['sha256']==spec['runtime_sha']
            assert row['command'][0]==row['runtime']['path'] and manifest['pins'][row['runtime']['path']]==spec['runtime_sha']
            assert row['algorithm']['strict'] is True and row['algorithm']['native_benchmark_kernel'] is False
            _full_validation(row,reference,variant)
            assert row['source_sha256']==sha(row['source']) and row['upstream_sha256']==sha(row['upstream_source'])
            assert row['physical_loc']==row['loc'] and row['maximum_workers']==1
        fast=min(candidates,key=lambda row:(row['median_seconds'],row['loc'],row['key']))
        small=min(candidates,key=lambda row:(row['loc'],row['median_seconds'],row['key']))
        group=next((g for g in benchmark['groups'] if g['language']==language),None)
        if group:
            previous=[deepcopy(data['programs'][key]) for key in group['programs']]
            audit['previous_endpoints'][language]=dict(programs=previous,files={digest:deepcopy(data['files'][digest]) for row in previous for digest in row['files']})
        else:
            group=dict(language=language,label='Lua (interpreted)' if language=='lua' else 'LuaJIT',programs=[]);benchmark['groups'].append(group)
            audit['previous_endpoints'][language]=dict(programs=[],files={})
        for row in {r['key']:r for r in (fast,small)}.values():
            measured(row['key'],'pidigits',language,row['loc'],row['samples_seconds'],row['source'],row['source_sha256'],report_path,manifest_path,
              algorithm=deepcopy(row['algorithm']),required_work_validated=True,loc_metric='statement-normalized',physical_loc=row['physical_loc'],loc_expansions=row['loc_expansions'],original_physical_loc=row['original_physical_loc'],
              source_url=row['source_url'],local_adaptation=True,adaptations=row['adaptations'],runtime_policy=spec['runtime_policy'],runtime=deepcopy(row['runtime']),runtime_arguments=row['command'],runtime_environment=row['runtime_environment'],maximum_workers=1,
              full_output_validation=row['sample_validation'],sample_execution_metadata=row['sample_execution_metadata'],study_manifest_sha256=spec['digest'],study_report_sha256=sha(root/report_path),timing_boundary=manifest['timing_boundary'],
              mech_comparison=dict(seconds=fast_control['median_seconds'],control_key=fast_control['key'],source=fast_control['source'],source_sha256=fast_control['source_sha256'],samples_seconds=fast_control['samples_seconds'],report=report_path,source_audit=manifest_path,fastest_loc=38,smallest_loc=35,time_x_mech=row['median_seconds']/fast_control['median_seconds'],fastest_loc_x_mech=row['loc']/38,smallest_loc_x_mech=row['loc']/35,compact_source_control=dict(seconds=small_control['median_seconds'],loc=35,source=small_control['source'],source_sha256=small_control['source_sha256'],samples_seconds=small_control['samples_seconds'],time_x_mech=row['median_seconds']/small_control['median_seconds'])))
        group['programs']=list(dict.fromkeys([fast['key'],small['key']]))
        study_sources={digest:deepcopy(data['files'][digest]) for row in rows.values() for digest in add_file(row['source'],row['source_sha256'])}
        for filename in ('gmp.c','gmp-lua51.c','lua51_compat.h'):
            path=root/spec['cohort']/'support'/filename
            if path.is_file():audit['library_sources'][path.relative_to(root).as_posix()]=dict(sha256=sha(path),code=path.read_text(),counted_in_application_loc=False,scope='Ordinary general GMP arithmetic/Lua ABI only; no spigot, extraction or workload executor.')
        historical_path=spec['cohort']+'/results/historical-exclusions.json'
        historical=load(historical_path)
        audit['studies'][language]=dict(manifest=manifest_path,manifest_sha256=spec['digest'],report=report_path,report_sha256=sha(root/report_path),profiles=rows,selected=dict(fast=fast['key'],small=small['key']),study_sources=study_sources,discarded_attempts=discarded,historical_exclusions=historical,timing_boundary=manifest['timing_boundary'],library=deepcopy(manifest['library']))
    data['lua_pidigits_study_audit']=audit
    data['provenance']['lua_pidigits_study_data.py']=sha(Path(__file__))
    data['provenance']['isolated_study_data.py']=sha(Path(__file__).with_name('isolated_study_data.py'))
    data['protocol']['lua_pidigits_studies']='Lua5.5 interpreter and LuaJIT each independently measure the same complete four-source listed pi-digits pool with ordinary GMP arithmetic, three fresh N10000 processes per candidate and current Mech fast/compact source controls. Both true endpoint quotient tests, complete rows/tail/count output and process startup/shutdown are retained. Previous OpenSSL LuaJIT samples and missing-module exclusions remain historical, without source/library relabelling.'

"""Retain126 exact bounded C3 records; preserve the next failed native build."""
import copy
import json
import math
from pathlib import Path
import re

import bootstrap_reuse
import references

HERE=Path(__file__).resolve().parents[2]
PACKET=Path(__file__).resolve().parent
PRIOR=HERE/'qualification/private-v2'
BUILD=HERE/'.build/private-v2'
PRIOR_PROPOSAL=HERE/'proposals/private-v2.json'
PRIOR_SHA='aacefa87b453aafa39ae30b3cb8a78324b5ad7dceb28dad9a7c1f8ffa35990f1'
REUSED_IDS={f'c3-{bench}-{variant}' for bench in (
    'regexredux','pidigits','revcomp','fasta','knucleotide','mandelbrot','spectralnorm')
    for variant in ('compact','optimized')}|{'c3-binarytrees-compact'}
GATES={'sampler_exited_normally','authoritative_native_group_received','native_inherited_common_mutex',
    'native_exited_normally','no_execution_error','no_wrapper_integrity_error','no_resource_violation',
    'no_resource_monitor_errors','no_compiler_power_monitor_errors','no_monitor_finish_error',
    'no_execute_cleanup_error','no_remaining_execute_group','no_supervisor_cleanup','no_supervisor_remaining_group',
    'valid_environment','actual_full_workload','exact_command_and_group','complete_output_reference_match',
    'complete_output_bytes_and_hash','pins_unchanged'}
OBSERVATIONS=('exit_status','seconds','power_at_start','power_at_end','competing_compilers','power_changes',
    'timeout_seconds','maximum_rss_mib','peak_process_group_rss_kib','valid_environment','pins_unchanged_at_end')
FATAL=('error','resource_violation','monitor_errors','resource_monitor_errors','cleanup_error','supervisor_cleanup_error',
    'remaining_group_members','supervisor_remaining_members','supervisor_pin_failure','source_work_error','source_work_failure',
    'preparation_supervisor_failure','sampler_failure','sampler_error','registration_error','supervisor_error','monitor_finish_error',
    'supervisor_cleanup','execute_group_members_after_cleanup','monitor_finish')


def validate_sources():
    prior=json.loads((HERE/'revisions/private-v2/profiles.json').read_text())
    expected=copy.deepcopy(prior)
    changes={
        'sources/binarytrees/optimized.c3':[('* (worker + 1) / count','* (long)(worker + 1) / count')],
        'sources/fannkuchredux/optimized.c3':[('* (worker + 1) / count','* (long)(worker + 1) / count'),
            ('checksum += (rank & 1) == 0 ? flips : -flips;','checksum += (long)((rank & 1) == 0 ? flips : -flips);')],
        'sources/fannkuchredux/compact.c3':[('total *= i + 1;','total *= (long)(i + 1);'),
            ('checksum += (rank & 1) == 0 ? flips : -flips;','checksum += (long)((rank & 1) == 0 ? flips : -flips);')],
    }
    for row in expected['profiles']:
        if row['source'] in changes:
            old=row['source'];new='revisions/private-v3/'+old
            row['source']=new;row['compiled_source_bundle']=[new if path==old else path for path in row['compiled_source_bundle']]
    assert json.loads((PACKET/'profiles.json').read_text())==expected
    for name,edits in changes.items():
        before=(HERE/name).read_text()
        for old,new in edits:
            assert before.count(old)==1
            before=before.replace(old,new)
        assert (PACKET/name).read_text()==before
    assert sum(map(len,changes.values()))==5


def observations(attempt):
    assert type(attempt['exit_status']) is int and attempt['exit_status']==0
    assert type(attempt['seconds']) in (int,float) and math.isfinite(attempt['seconds']) and attempt['seconds']>0
    assert attempt['power_at_start']==attempt['power_at_end']==dict(source='AC Power',low_power_mode=0)
    assert attempt['competing_compilers']==attempt['power_changes']==[]
    assert attempt['timeout_seconds']==120 and attempt['maximum_rss_mib']==10240
    assert type(attempt['peak_process_group_rss_kib']) is int and 0<=attempt['peak_process_group_rss_kib']<=10240*1024
    assert attempt['valid_environment'] is True and attempt['pins_unchanged_at_end'] is True
    assert not any(attempt.get(name) for name in FATAL)


def validate_records(report,inventory):
    assert report['status']=='stopped-unqualified' and report['active'] is None and report['proposal_sha256']==PRIOR_SHA
    assert report['error_type']=='AssertionError' and report['error']=="('Owned command failed', 'build', 1)"
    assert report['full_performance_runs']==0 and report['chart_admission'] is False
    assert report['never_chart_eligible'] is True and report['no_automatic_retry'] is True
    assert report['reused_bounded_cases']==0 and len(report['attempts'])==126
    groups={}
    for number,attempt in enumerate(report['attempts'],1):
        observations(attempt)
        identifier=attempt['id'];assert identifier in REUSED_IDS and type(attempt['attempt']) is int and attempt['attempt']==number
        assert type(attempt['workload_n']) is int
        benchmark=attempt['benchmark'];assert attempt['key']==benchmark+':'+identifier and attempt['language']=='c3'
        assert attempt['case'] in inventory[benchmark]
        assert attempt['full_workload'] is False and attempt['full_performance_run'] is False
        assert attempt['preparation_only'] is True and attempt['never_chart_eligible'] is True
        checks=attempt['qualification_checks']
        assert type(checks) is dict and set(checks)==GATES and all(type(value) is bool and value is True for value in checks.values())
        assert attempt['qualified'] is True and type(attempt['sampler_wait_status']) is int and attempt['sampler_wait_status']==0
        native=attempt['supervised_native_group']
        assert native['type']=='native-group-started' and native['command']==attempt['command'] and native['inherited_common_mutex'] is True
        assert type(native['pid']) is int and native['pid']>0 and native['pid']==native['pgid']
        assert attempt['validation']['reference_match'] is True
        assert (attempt['validation']['bytes'],attempt['validation']['sha256'])==(attempt['stdout_bytes'],attempt['stdout_sha256'])
        fingerprint=json.loads(attempt['execution_fingerprint'])
        assert set(fingerprint)=={'source_sha256','binary_sha256','command','cwd','environment','support'}
        assert fingerprint['command']==[str(BUILD/'bin'/identifier),'{n}']
        assert fingerprint['source_sha256']==attempt['source_sha256'] and fingerprint['cwd']==attempt['cwd']==str(HERE)
        assert fingerprint['environment']==attempt['runtime_environment']
        assert attempt['command']==[fingerprint['command'][0],str(attempt['workload_n'])]
        if attempt['case'].startswith('n-'): assert attempt['workload_n']==int(attempt['case'][2:])
        group=groups.setdefault(identifier,dict(benchmark=benchmark,fingerprint=fingerprint,records=[]))
        assert group['benchmark']==benchmark and group['fingerprint']==fingerprint
        assert attempt['case'] not in {row['case'] for row in group['records']}
        group['records'].append(copy.deepcopy(attempt))
    assert set(groups)==REUSED_IDS and len(groups)==15
    for group in groups.values(): assert [row['case'] for row in group['records']]==inventory[group['benchmark']]
    return groups


def validate_native_receipts(sampled,enriched,attempt):
    observations(sampled);observations(enriched)
    assert type(enriched['sampler_wait_status']) is int and enriched['sampler_wait_status']==0
    assert type(enriched['attempt']) is int and enriched['attempt']==attempt['attempt']
    assert all(type(record['workload_n']) is int for record in (sampled,enriched,attempt))
    assert sampled['workload_n']==enriched['workload_n']==attempt['workload_n']
    for key in OBSERVATIONS+('command','cwd','runtime_environment','source_sha256','execution_fingerprint','stdout_bytes','stdout_sha256','validation'):
        assert sampled[key]==enriched[key]==attempt[key]
    for key in ('qualified','qualification_checks','supervised_native_group','sampler_wait_status'):
        assert enriched[key]==attempt[key]


def reconstruct(preparation,guard,harness):
    validate_sources()
    assert preparation.sha(PRIOR_PROPOSAL)==PRIOR_SHA
    proposal=json.loads(PRIOR_PROPOSAL.read_text());guard.verify(proposal['pins'])
    bootstrap_path=Path(proposal['prior_bootstrap_reuse'])
    assert preparation.sha(bootstrap_path)==proposal['prior_bootstrap_reuse_sha256']
    bootstrap=json.loads(bootstrap_path.read_text());bootstrap_reuse.validate_packet(bootstrap,preparation,guard)
    assert proposal['case_inventory']==preparation.preflight_inputs(references)
    report_path=PRIOR/'report.json'
    groups=validate_records(json.loads(report_path.read_text()),proposal['case_inventory'])
    pins=preparation.merge(proposal['pins'],bootstrap['pins'],
        {str(PRIOR_PROPOSAL):PRIOR_SHA,str(report_path):preparation.sha(report_path)},
        preparation.pin_tree(PRIOR),preparation.pin_tree(BUILD))
    binding=proposal['toolchain_binding'];assert binding==bootstrap['toolchain']
    compiler,stdlib,sdk=map(Path,(binding['compiler'],binding['stdlib'],binding['sdk']))
    if harness is None: harness=guard.load_harness()
    profiles={}
    for declared in proposal['profiles']:
        identifier=declared['id']
        if identifier not in groups: continue
        sources=[PRIOR/'source-snapshot'/name for name in declared['compiled_source_bundle']]
        bundle=[dict(path=str(path),sha256=preparation.sha(path)) for path in sources]
        assert all(item['sha256']==preparation.sha(HERE/name) for item,name in zip(bundle,declared['compiled_source_bundle']))
        binary=BUILD/'bin'/identifier;cache=BUILD/'cache'/identifier
        command=preparation.build_command(compiler,stdlib,sdk,sources,binary,cache,create_cache=False)
        directory=PRIOR/'builds'/identifier
        receipt=json.loads((directory/'build.json').read_text())
        assert receipt['command']==command and receipt['exit_status']==receipt['final_exit_status']==0 and receipt['inherited_common_mutex'] is True
        assert not any(receipt.get(name) for name in ('error','cleanup_error','cleanup_errors','output_drain_error','remaining_group_members'))
        llvm=list((cache/'llvm').rglob('*.ll'))
        assert llvm and any(re.search(r'target triple = "(?:arm64|aarch64)-apple-macos',path.read_text()) for path in llvm)
        image=directory/'images';assert (image/'architecture.stdout').read_text().split()==['arm64']
        for name,argv in (('architecture',['/usr/bin/lipo','-archs',str(binary)]),('linked-images',['/usr/bin/otool','-L',str(binary)])):
            value=json.loads((image/(name+'.json')).read_text())
            assert value['command']==argv and value['exit_status']==value['final_exit_status']==0 and value['inherited_common_mutex'] is True
            assert not any(value.get(key) for key in ('error','cleanup_error','cleanup_errors','output_drain_error'))
        review=json.loads((image/'dependency-review.json').read_text())
        assert review['binary']==str(binary) and review['architectures']==['arm64']
        linked=[line.strip().split(' (',1)[0] for line in (image/'linked-images.stdout').read_text().splitlines()[1:]]
        assert [value['install_name'] for value in review['dependencies']]==linked
        for value in review['dependencies']:
            if value['install_name'].startswith(('/usr/lib/','/System/Library/')):
                assert value==dict(install_name=value['install_name'],scope='OS-image-not-hashed')
            else:
                lookup=Path(value['lookup_path']);target=lookup.resolve(strict=True)
                assert str(target)==value['path'] and any(target.is_relative_to(root.resolve()) for root in (compiler.parent,preparation.GMP,preparation.PCRE,preparation.CLANG.parent.parent))
                assert preparation.sha(lookup)==preparation.sha(target)==value['sha256']
                pins=preparation.merge(pins,{str(lookup):value['sha256'],str(target):value['sha256']})
        fingerprint=groups[identifier]['fingerprint']
        row=dict(declared,key=declared['benchmark']+':'+identifier,language='c3',
            source=str(sources[0]),source_sha256=bundle[0]['sha256'],binary_sha256=preparation.sha(binary),
            source_bundle=bundle,support={item['path']:item['sha256'] for item in bundle[1:]},
            command=[str(binary),'{n}'],cwd=str(HERE),runtime_environment=fingerprint['environment'],
            compiler_version='0.8.5',bundled_llvm_version=binding['bundled_llvm_version'],explicit_backend='llvm',
            explicit_target='macos-aarch64',build_command=command,cache=str(cache),measured_architecture='arm64',
            qualification_origin='retained-private-v2-exact-source-binary-and-records',new_native_cases=0)
        assert json.loads(harness.execution_fingerprint(row))==fingerprint
        old_records=[]
        for attempt in groups[identifier]['records']:
            case=attempt['case'];sample=PRIOR/identifier/case
            data=None if declared['benchmark'] in preparation.NUMERIC else dict(preparation.inputs(declared['benchmark'],references))[case]
            expected=getattr(references,declared['benchmark'])(attempt['workload_n'] if data is None else data)
            assert (sample/'actual.stdout').read_bytes()==(sample/'expected.stdout').read_bytes()==expected
            assert preparation.sha(sample/'actual.stdout')==attempt['stdout_sha256']
            if data is not None: assert (sample/'stdin.fasta').read_bytes()==data
            raw=PRIOR/'attempts'/('%03d-%s.json'%(attempt['attempt'],identifier));supervisor=raw.with_suffix('.supervisor.json')
            assert attempt['result_path']==str(raw) and attempt['supervisor_result_path']==str(supervisor)
            sampled,enriched=json.loads(raw.read_text()),json.loads(supervisor.read_text())
            validate_native_receipts(sampled,enriched,attempt)
            old_records.append(dict(case=case,prior_attempt=attempt['attempt'],raw_result=str(raw),supervisor_result=str(supervisor),
                stdout_sha256=attempt['stdout_sha256'],origin='retained-private-v2'))
        row['prior_bounded_records']=old_records
        profiles[identifier]=row
    assert set(profiles)==REUSED_IDS
    declared=next(row for row in proposal['profiles'] if row['id']=='c3-binarytrees-optimized')
    failed=PRIOR/'builds'/declared['id'];receipt=json.loads((failed/'build.json').read_text())
    sources=[PRIOR/'source-snapshot'/name for name in declared['compiled_source_bundle']]
    command=preparation.build_command(compiler,stdlib,sdk,sources,BUILD/'bin'/declared['id'],BUILD/'cache'/declared['id'],create_cache=False)
    assert receipt['command']==command and receipt['exit_status']==receipt['final_exit_status']==1 and receipt['inherited_common_mutex'] is True
    assert receipt['error_type']=='AssertionError' and receipt['error']=="('Owned command failed', 'build', 1)"
    assert not any(receipt.get(key) for key in ('cleanup_error','cleanup_errors','output_drain_error','remaining_group_members'))
    stderr=(failed/'build.stderr').read_text()
    assert "explicit cast to 'long'" in stderr and 'iterations * (worker + 1) / count' in stderr
    assert sources[0].read_bytes()==(HERE/'sources/binarytrees/optimized.c3').read_bytes()
    guard.verify(pins)
    return dict(status='verified-fifteen-exact-C3-v2-profiles-126-cases',profiles=profiles,
        reused_bounded_cases=126,reused_profile_count=15,old_cases_relabelled_as_new_attempts=False,native_cases_reexecuted=0,
        prior_report=str(report_path),prior_report_sha256=pins[str(report_path)],prior_proposal_sha256=PRIOR_SHA,
        preserved_failed_build=dict(receipt=str(failed/'build.json'),stderr=str(failed/'build.stderr'),source=str(sources[0])),
        toolchain=binding,pins=pins,recorded_in_completed_preparation_pins=False,newly_captured_preserved_native_evidence=True,
        full_performance_runs=0,chart_admission=False)


def validate_packet(packet,preparation,guard,harness):
    guard.verify(packet['pins']);assert packet==reconstruct(preparation,guard,harness)
    return packet

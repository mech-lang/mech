"""Verify all discovery observations and artifact pins; no new score is inferred."""
from pathlib import Path
from hashlib import sha256
from statistics import median
from runpy import run_path
from math import isclose
import gzip, json, subprocess, sys, tarfile, tempfile

E = Path(__file__).resolve().parent
P = E.parents[2]
read = lambda p: json.loads(Path(p).read_text())
digest = lambda data: sha256(data).hexdigest()
sys.path.insert(0, str(P))
sys.path.insert(0,str(P.parent/'compact-size'))
import mach_o
import qualification_timings as timing
from qualification_records import Evidence, Findings, load_fixtures

def main():
    result = read(E / 'result.json')
    assert result['promoted'] is False and result['qualified_selection_unchanged']
    assert result['qualified']['roc_wins'] == 81 and result['qualified']['libdeflate_wins'] == 62
    assert digest((P / 'program.mec').read_bytes()) == result['source_sha256']
    size = run_path(str(P / 'evidence/vector-mismatch-20261119/verify.py'), run_name='word_text_size')['text_size']
    for role, pin in result['roles'].items():
        obj = gzip.decompress((E / (role + '-program.o.gz')).read_bytes())
        assert digest(obj) == pin['artifact_sha256']['program.o'] and size(obj) == pin['text_bytes']
        assert digest(gzip.decompress((E / (role + '-metadata.json.gz')).read_bytes())) == pin['artifact_sha256']['metadata.json']
        assert pin['artifact_sha256']['program.mecb'] == result['bytecode_sha256']
        for profile in pin['profiles']:
            assert digest((E / profile['path']).read_bytes()) == profile['sha256']
        for name, expected in pin['source_inputs'].items():
            assert digest(subprocess.check_output(['git', 'show', pin['source_basis'] + ':' + name], cwd=P)) == expected
    portable = read(E / 'portable-replays.json')
    assert {row['role'] for row in portable['runs']} == set(result['roles'])
    for row in portable['runs']:
        assert row['source_sha256'] == result['source_sha256']
        assert {run['mode'] for run in row['variants']} == {'normal','checked'}
        for run in row['variants']:
            assert run['artifacts'] == result['roles'][row['role']]['artifact_sha256']
            assert run['compiler_sha256'] == portable['compiler_sha256']
    findings = Findings()
    fixtures = load_fixtures(Evidence(P, findings), read(P / 'qualification-policy-v3.json'))
    with tempfile.TemporaryDirectory(prefix='mech-word-replay-') as temporary:
        root = Path(temporary)
        checks, raw = root / 'checks', root / 'raw'
        for archive, members in read(E / 'archive-files.json').items():
            with tarfile.open(E / archive) as tar:
                cached, seen = {}, set()
                for member in tar:
                    name = member.name
                    assert name in members and name not in seen
                    seen.add(name)
                    part = Path(name)
                    assert not part.is_absolute() and '..' not in part.parts
                    assert member.isfile() or member.islnk()
                    data = cached[member.linkname] if member.islnk() else tar.extractfile(member).read()
                    cached[name] = data
                    pin = members[name]
                    assert len(data) == pin['bytes'] and digest(data) == pin['sha256']
                    target = (raw if archive == 'raw-runtime.tar.gz' else checks) / part
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
                assert seen == set(members)
        for role, pin in result['roles'].items():
            receipt = read(checks / role / 'receipt.json')
            bound = read(checks / role / 'bound/runtime.json')
            assert receipt['artifacts'] == pin['artifact_sha256'] and bound['worker_sha256'] == pin['worker_sha256']
            assert digest((checks / role / 'bound/mech-worker').read_bytes()) == pin['worker_sha256']
            link = read(checks / role / 'linked-proof.json')
            parts = Path(link['result']).parts
            relative = Path(*parts[parts.index('natural4k-scorer-repair') + 1:])
            data = (checks / relative).read_bytes()
            assert digest(data) == link['sha256']
            proof = json.loads(data)
            assert proof['matches_success_observations'] == 162 and len(proof['checked_failure_cases']) == 6
            assert proof['peak_stack_bytes'][role] == link['peak_stack_bytes'] <= 5280
        for folder, count, source in [('scalar-pair-revisit',20,result['source_basis_commit'])]:
            tests = read(checks / (folder + '-repository-tests/result.json'))
            log = (checks / (folder + '-repository-tests/test.log')).read_bytes()
            assert tests['exit_code'] == 0 and digest(log) == tests['test_log_sha256']
            assert f'{count} passed; 0 failed'.encode() in log
            assert tests['new_packed_key_native_calls'] == 94208 and tests['wide_item_native_calls']==16384 and tests['paired_scalar_native_calls']==22528 and tests['prior_winner_native_calls'] == 16384
            build = read(checks / (folder + '-compiler/build.json'))
            assert build['exit_code'] == 0
            assert tests['source_sha256']==build['source_sha256']
            for name, expected in build['source_sha256'].items():
                assert digest((checks / (folder + '-compiler/source-inputs') / name).read_bytes()) == expected
                assert digest(subprocess.check_output(['git','show',source+':'+name],cwd=P)) == expected
            disabled = read(checks / (folder + '-disabled-replays/result.json'))
            assert disabled['all_three_exact'] and len(disabled['runs']) == 33
            for row in disabled['runs']:
                for name, expected in row['artifacts'].items():
                    assert digest((checks / (folder + '-disabled-replays') / row['role'] / 'frontend' / name).read_bytes()) == expected
        for role, pin in result['roles'].items():
            recovery = read(checks / role / 'promotion-recoveries/result.json')
            assert recovery['observations'] == recovery['exact_recoveries'] == 12
            assert recovery['benchmark_worker_sha256'] == pin['worker_sha256']
            assert read(checks / role / 'checked-emission/result.json')['artifacts'] == pin['artifact_sha256']
        enabled = read(checks / 'scalar-pair-revisit-enabled-tests/result.json')
        assert enabled['complete'] and len(enabled['runs']) == 3
        assert enabled['source_sha256']==build['source_sha256']
        for row in enabled['runs']:
            log = (checks / ('scalar-pair-revisit-enabled-tests/' + row['label'] + '.log')).read_bytes()
            assert row['exit_code'] == 0 and digest(log) == row['log_sha256']
            assert b'20 passed; 0 failed' in log
        for role in result['counter_roles']:
            counts = read(checks / role / 'instruction-counts/counts.json')
            receipt = read(checks / role / 'receipt.json')
            assert counts['object_sha256'] == receipt['object_sha256']
            for case in counts['cases'].values():
                if 'instructions' not in case['counts']:
                    assert all(block['visits'] == 0 for block in case['blocks'])
                assert sum(block['visits'] * len(block['instructions']) for block in case['blocks']) == case['counts'].get('instructions', 0)
                assert sum(block['visits'] * sum(op == 'b' or op.startswith('b.') or op in ['cbz', 'cbnz', 'tbz', 'tbnz'] for op, args in block['instructions']) for block in case['blocks']) == case['counts'].get('branches', 0)
        census=result['broad_loss_census']
        counts_file=checks/'packed-eight-head-probe/instruction-counts-loss-census/counts.json'
        counts_data=counts_file.read_bytes();assert digest(counts_data)==census['counts_sha256']
        counts=json.loads(counts_data)
        assert len(counts['cases'])==11 and set(counts['cases'])==set(census['cases'])
        preceding=E.parent/'packed-eight-entry-study';prior_pins=read(preceding/'archive-files.json')
        member='packed-eight-head-probe/receipt.json'
        with tarfile.open(preceding/'checks-and-builds.tar.gz') as archive:
            old_receipt=archive.extractfile(member).read()
        assert digest(old_receipt)==prior_pins['checks-and-builds.tar.gz'][member]['sha256']
        assert counts['object_sha256']==json.loads(old_receipt)['artifacts']['program.o']
        for order in ['forward','reverse']:
            member='packed-eight-entry-qualified96-packed-eight-head-probe-profiled-'+order+'/summary.json'
            with tarfile.open(preceding/'raw-runtime.tar.gz') as archive:
                data=archive.extractfile(member).read()
            assert digest(data)==prior_pins['raw-runtime.tar.gz'][member]['sha256']
            latency={row['case']:row for row in json.loads(data)}
            for name,row in census['cases'].items():
                assert row['paired_x_libdeflate'][order]==latency[name]['paired_x_libdeflate']['mech-experiment']
        for name,row in census['cases'].items():
            actual=counts['cases'][name]
            assert row['counts']=={key:actual['counts'].get(key,0) for key in row['counts']}
            assert row['counts']['instructions']==sum(block['visits']*len(block['instructions']) for block in actual['blocks'])
            assert row['top_blocks']==sorted(actual['blocks'],key=lambda block:block['visits']*len(block['instructions']),reverse=True)[:3]
        for comparison in result['historical_counter_comparisons']:
            preceding=E.parent/comparison['previous_study'];member=comparison['previous_counts_member']
            with tarfile.open(preceding/'checks-and-builds.tar.gz') as prior:
                old_data=prior.extractfile(member).read()
            assert digest(old_data)==comparison['previous_counts_sha256']==read(preceding/'archive-files.json')['checks-and-builds.tar.gz'][member]['sha256']
            new_data=(checks/comparison['current_role']/'instruction-counts/counts.json').read_bytes()
            assert digest(new_data)==comparison['current_counts_sha256']
            old=json.loads(old_data);new=json.loads(new_data)
            assert set(comparison['cases'])==set(new['cases'])==set(old['cases'])
            for name,row in comparison['cases'].items():
                assert row['before']=={key:old['cases'][name]['counts'].get(key,0) for key in row['before']}
                assert row['after']=={key:new['cases'][name]['counts'].get(key,0) for key in row['after']}
                assert row['delta']=={key:row['after'][key]-row['before'][key] for key in row['before']}
        prior=E.parent/'packed-eight-entry-study';prior_pins=read(prior/'archive-files.json')
        for row in result['register_equivalence']:
            left=read(checks/row['left']/'receipt.json')
            right_path=checks/row['right']/'receipt.json'
            if right_path.exists():right=read(right_path)
            else:right={'artifacts':read(prior/'result.json')['roles'][row['right']]['artifact_sha256']}
            assert row['all_artifacts_same']==(left['artifacts']==right['artifacts'])
            lraw=read(checks/row['left'].removesuffix('-profiled')/'receipt.json')
            member=row['right'].removesuffix('-profiled')+'/receipt.json'
            target=checks/member
            if target.exists():rraw=read(target)
            else:
                with tarfile.open(prior/'checks-and-builds.tar.gz') as archive:data=archive.extractfile(member).read()
                assert digest(data)==prior_pins['checks-and-builds.tar.gz'][member]['sha256'];rraw=json.loads(data)
            symbol='_mech_compact_numeric_3009_no_growth_composed'
            def body(path,receipt):
                data,sections,_=mach_o.load(path)
                assert digest(data)==receipt['artifacts']['program.o']
                _,address,_,offset,_,_=sections[mach_o.TEXT]
                atom=receipt['layout']['symbols'][symbol];start=offset+atom['start']-address
                chunk=data[start:start+atom['extent']]
                assert digest(chunk)==atom['sha256']
                return chunk
            lpath=checks/row['left'].removesuffix('-profiled')/'frontend/program.o'
            rpath=checks/row['right'].removesuffix('-profiled')/'frontend/program.o'
            if not rpath.exists():
                member=row['right'].removesuffix('-profiled')+'/frontend/program.o'
                with tarfile.open(prior/'checks-and-builds.tar.gz') as archive:data=archive.extractfile(member).read()
                assert digest(data)==prior_pins['checks-and-builds.tar.gz'][member]['sha256']
                rpath.parent.mkdir(parents=True,exist_ok=True);rpath.write_bytes(data)
            assert row['raw_scorer_body_same']==(body(lpath,lraw)==body(rpath,rraw))
        gates=read(checks/'scalar-pair-register-gates.json')
        assert gates['complete'] and len(gates['runs'])==8 and all(row['exit_code']==0 for row in gates['runs'])
        required = read(checks / 'word-complements-prior-wins.json')['required_previous_individual_wins']
        panels = 0
        for stage in result['stages'].values():
            assert stage['complete']
            for run in stage['runs']:
                folder = raw / Path(run['run']).name
                assert read(folder / 'complete.json')['balanced_blocks_per_case'] == 24
                assert len(run['results']) == 21
                pins=read(folder/'pins.json')['actors']
                assert pins['mech-experiment']['binary_sha256']==result['roles'][run['role']]['worker_sha256']
                assert pins['mech-experiment']['metadata_sha256']==result['roles'][run['role']]['artifact_sha256']['metadata.json']
                assert pins['mech-control']['binary_sha256']==stage['plan']['control_worker_sha256_by_role'][run['role']]
                for row in run['results']:
                    name = row['case']
                    observations = json.loads(gzip.decompress((folder / name / 'observations.json.gz').read_bytes()))
                    blocks = timing.case_blocks(observations, fixtures[name], 24)[0]
                    for actor in ['mech-control', 'mech-experiment']:
                        for peer, key in [('roc', 'paired_x_roc'), ('libdeflate', 'paired_x_libdeflate')]:
                            assert isclose(1 + timing.estimate(timing.paired(blocks, actor, peer)), row[key][actor], rel_tol=2e-12)
                            if actor == 'mech-experiment' and name in required[peer]:
                                audit = next(a for a in result['panel_audit'] if a['role'] == run['role'] and a['order'] == run['order'])
                                assert (row[key][actor] >= 1) == (name in audit['previous_wins_missing'][peer])
                panels += 1
        assert panels == 8
        for key, cohort, trials in [('builds', 'scalar-pair-revisit-matched-builds', 10)]:
            summary = result[key]
            comparison = read(checks / cohort / 'comparison.json')
            plan = read(checks / cohort / 'plan.json')
            assert digest((checks / cohort / 'plan.json').read_bytes()) == comparison['plan_sha256'] == summary['plan_sha256']
            observations = comparison['observations']
            assert len(observations) == summary['trials'] == trials
            for actor, job in plan['actors'].items():
                assert median(row[actor]['seconds']['source_through_link'] for row in observations) == summary['medians'][actor]['source_through_link']
                if actor.startswith('mech-'):
                    for row in observations:
                        assert row[actor]['generated_artifacts'] == job['artifact_sha256']
                        assert row[actor]['stripped_executable']['bytes'] == observations[0][actor]['stripped_executable']['bytes']
        # Full81-case screens and the adverse 96-block qualification remain
        # independently replayable, including the unchanged-prefix inventory.
        if result['full24'] is not None:
            full24 = result['full24']
            assert full24['complete'] and len(full24['runs']) == 2
            for run in full24['runs']:
                folder = raw / Path(run['folder']).name
                assert read(folder/'complete.json')['balanced_blocks_per_case'] == 24
                assert len(run['results']) == 81
                pin=read(folder/'pins.json')['actors']['mech-experiment']
                assert pin['binary_sha256']==result['roles']['scalar-pair-head-probe-profiled']['worker_sha256']
                assert pin['metadata_sha256']==result['roles']['scalar-pair-head-probe-profiled']['artifact_sha256']['metadata.json']
                for row in run['results']:
                    blocks = timing.case_blocks(json.loads(gzip.decompress((folder/row['case']/'observations.json.gz').read_bytes())),fixtures[row['case']],24)[0]
                    for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
                        assert isclose(1+timing.estimate(timing.paired(blocks,'mech-experiment',peer)),row[key]['mech-experiment'],rel_tol=2e-12)
                assert run['peers']['roc']['wins'] == 81 and run['peers']['libdeflate']['wins'] in (62,63)
                for peer,key in [('roc','paired_x_roc'),('libdeflate','paired_x_libdeflate')]:
                    assert run['peers'][peer]['wins']==sum(row[key]['mech-experiment']<1 for row in run['results'])
                assert all(not row['lost_previous_named_wins'] for row in run['peers'].values())
        if result['qualification96'] is not None:
            qname='scalar-pair-revisit-qualified96-scalar-pair-head-probe-profiled'
            qresult=read(checks/qname/'result.json');assert qresult == result['qualification96']
            assert qresult['complete']
            qplan=read(checks/qname/'plan.json');assert digest((checks/qname/'plan.json').read_bytes())==qresult['plan_sha256']
            policy=read(checks/qname/'policy.json');assert policy['peer_win_floors']=={'roc':81,'libdeflate':63}
            assert policy['bootstrap_coverage']==.9 and policy['bootstrap_resamples']==8000
            required=read(checks/'word-complements-prior-wins.json')['required_previous_individual_wins']
            assert set(policy['required_peer_wins']['roc'])==set(required['roc'])
            assert set(policy['required_peer_wins']['libdeflate'])==set(required['libdeflate'])|{'prefix-natural-4096'}
            expected_policy=read(P/'qualification-policy-v3.json')
            expected_policy['peer_win_floors']['libdeflate']=63
            expected_policy['required_peer_wins']=policy['required_peer_wins']
            assert policy==expected_policy
            candidate=result['roles']['scalar-pair-head-probe-profiled']
            assert qplan['identities']['mech-experiment']['worker']==candidate['worker_sha256']
            assert qplan['identities']['mech-experiment']['metadata']==candidate['artifact_sha256']['metadata.json']
            assert qplan['candidate_artifacts']==candidate['artifact_sha256']
            assert qplan['source_commit']==result['source_basis_commit']
            assert qplan['compiler_sha256']==candidate['compiler_sha256']
            original_inventory=read(checks/'as-run-fixtures.json')
            reconciled=read(checks/qname/'inventory-reconciliation.json')
            assert reconciled['as_run_inventory_sha256']==digest((checks/'as-run-fixtures.json').read_bytes())
            assert reconciled['canonical_inventory_sha256']==policy['fixture_inventory_sha256']
            assert [row['name'] for row in original_inventory]==list(fixtures)
            for row in original_inventory:
                fixture=fixtures[row['name']]
                for field in ['input_sha256','expected_sha256','input_bytes','prefix_bytes','raw_bytes','capacity']:
                    assert row[field]==fixture[field]
                assert reconciled['content'][row['name']]=={field:digest((P/fixture[field]).read_bytes()) for field in ['input','prefix','expected']}
            # Original reference paths are relative to the old qualification root.
            # Recreate only their recorded data layout beneath a new temporary root.
            import shutil
            vq=raw
            qfolder=vq/'consolidation-995/natural4k-scorer-repair'/qname
            shutil.copytree(checks/qname,qfolder)
            shutil.copyfile(checks/'as-run-fixtures.json',vq/'fixtures.json')
            qfind=Findings();qev=Evidence(vq,qfind);qruns=[]
            for entry,record in zip(qplan['runs'],qresult['receipts'],strict=True):
                original=read(vq/record['original_pins']['path']);canonical=read(vq/record['pins']['path'])
                assert canonical==original|{'input_inventory_sha256':policy['fixture_inventory_sha256']}
                actual=timing.runtime_run(record,fixtures,qev,qplan['identities'],policy,entry)
                assert actual is not None;qruns.append(actual)
            assert qfind.rejected==qfind.unresolved==[]
            replay=timing.decide_runtime(qruns,[],policy,fixtures,qfind)
            assert replay['runs']==qresult['runtime_report']['runs']
            assert qfind.rejected==qresult['rejected'] and qfind.unresolved==qresult['unresolved']
            for actual,audit in zip(qruns,qresult['named_win_audit'],strict=True):
                for peer,row in audit['peers'].items():
                    assert set(row['cases'])==set(policy['required_peer_wins'][peer])
                    unsupported=[]
                    for index,(name,ratios) in enumerate(actual['ratios'].items()):
                        if name not in row['cases']:continue
                        parts=timing.paired(actual['cases'][name],'mech-experiment',peer)
                        if max(map(max,parts))>=0:
                            ci=timing.interval(parts,index,policy)
                            for key in ['estimate','lower','upper']:
                                assert isclose(ci[key],row['cases'][name]['interval'][key],rel_tol=2e-12,abs_tol=1e-14)
                            supported=ratios[peer]['mech-experiment']<1 and ci['upper']<0
                        else:
                            supported=ratios[peer]['mech-experiment']<1
                        assert supported==row['cases'][name]['supported']
                        if not supported:unsupported.append(name)
                    assert unsupported==row['unsupported']
            assert qresult['passed']==(not qfind.rejected and not qfind.unresolved and all(not peer['unsupported'] for audit in qresult['named_win_audit'] for peer in audit['peers'].values()))
        assert findings.rejected == findings.unresolved == []
    print('Verified six source/artifact bindings, four configurations of 20 optimized tests, 33 exact older recipes, semantic/recovery checks, work counts, actual raw-body equality, the eleven-case loss census, eight discovery orders and 10 complete build trials per actor. No new qualified score is claimed.')

if __name__ == '__main__':
    main()

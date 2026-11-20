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
    findings = Findings()
    fixtures = load_fixtures(Evidence(P, findings), read(P / 'qualification-policy-v3.json'))
    with tempfile.TemporaryDirectory(prefix='mech-word-replay-') as temporary:
        root = Path(temporary)
        checks, raw = root / 'checks', root / 'raw'
        for archive, members in read(E / 'archive-files.json').items():
            with tarfile.open(E / archive) as tar:
                assert sorted(tar.getnames()) == sorted(members)
                for name, pin in members.items():
                    part = Path(name)
                    assert not part.is_absolute() and '..' not in part.parts
                    data = tar.extractfile(name).read()
                    assert len(data) == pin['bytes'] and digest(data) == pin['sha256']
                    target = (raw if archive == 'raw-runtime.tar.gz' else checks) / part
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(data)
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
        producer = read(checks / 'selected-packet-compiler/build.json')
        assert producer['exit_code'] == 0
        for name, expected in producer['source_sha256'].items():
            assert digest((checks / 'selected-packet-compiler/source-inputs' / name).read_bytes()) == expected
            assert digest(subprocess.check_output(['git', 'show', result['source_basis_commit'] + ':' + name], cwd=P)) == expected
            if name != 'hosts/gpu/src/native_artifact/program/compact/phase_function.rs':
                assert digest(subprocess.check_output(['git', 'show', result['proof_test_reference']['source'] + ':' + name], cwd=P)) == expected
        disabled = read(checks / 'selected-packet-disabled-replays/result.json')
        assert disabled == result['mode_replay'] and disabled['all_three_exact'] and len(disabled['runs']) == 6
        for row in disabled['runs']:
            for name, expected in row['artifacts'].items():
                assert digest((checks / 'selected-packet-disabled-replays' / row['role'] / 'frontend' / name).read_bytes()) == expected
        for role, pin in result['roles'].items():
            recovery = read(checks / role / 'promotion-recoveries/result.json')
            assert recovery['observations'] == recovery['exact_recoveries'] == 12
            assert recovery['benchmark_worker_sha256'] == pin['worker_sha256']
            assert read(checks / role / 'checked-emission/result.json')['artifacts'] == pin['artifact_sha256']
            if role in result['new_candidates']:
                for proof in read(checks / role / 'profile-rebase.json'):
                    control, candidate = proof['control'], proof['candidate']
                    assert control['code_blocks'] == candidate['code_blocks']
                    assert len(control['code_heads']) == len(candidate['code_heads'])
                    assert proof['new_profile_sha256'] in [p['sha256'] for p in pin['profiles']]
        assert result['proof_test_reference']['native_calls'] == 72752
        assert result['proof_test_reference']['relevant_tests'] == 25
        required = read(checks / 'word-complements-prior-wins.json')['required_previous_individual_wins']
        panels = 0
        for stage in result['stages'].values():
            assert stage['complete']
            for run in stage['runs']:
                folder = raw / Path(run['run']).name
                assert read(folder / 'complete.json')['balanced_blocks_per_case'] == 24
                assert len(run['results']) == 21
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
        assert panels == 14
        for key, cohort, trials in [('builds', 'selected-packet-matched-builds', 14)]:
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
                        assert row[actor]['stripped_executable']['bytes'] == 276072
        assert findings.rejected == findings.unresolved == []
    print('Verified six new candidates and one exact reference, unchanged proof-source bindings, six exact mode replays, output/error/stack gates, 12 recoveries per worker, layout graph mappings, all fourteen discovery orders and fourteen complete-build trials per actor. Qualified selection remains 81 Roc / 62 C.')

if __name__ == '__main__':
    main()

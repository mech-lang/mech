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
        tests = read(checks / 'admitted-input-window-clean-repository-tests/result.json')
        log = (checks / 'admitted-input-window-clean-repository-tests/test.log').read_bytes()
        assert tests['exit_code'] == 0 and digest(log) == tests['test_log_sha256']
        assert b'6 passed; 0 failed' in log and b'2 passed; 0 failed' in log
        support = read(checks / 'input-window-clean-supporting-tests/result.json')
        for row, label, count in zip(support['tests'], ['wide_equal', 'phase_storage'], [10, 5], strict=True):
            log = (checks / 'input-window-clean-supporting-tests' / (label + '.log')).read_bytes()
            assert row['exit_code'] == 0 and digest(log) == row['log_sha256']
            assert f'{count} passed; 0 failed'.encode() in log
        for folder, source in [('admitted-input-window-compiler', '2e5949784'), ('admitted-input-window-clean-compiler', result['source_basis_commit'])]:
            build = read(checks / folder / 'build.json')
            assert build['exit_code'] == 0
            for name, expected in build['source_sha256'].items():
                assert digest((checks / folder / 'source-inputs' / name).read_bytes()) == expected
                assert digest(subprocess.check_output(['git', 'show', source + ':' + name], cwd=P)) == expected
        disabled = read(checks / 'input-window-clean-disabled-replays/result.json')
        assert disabled['all_three_exact'] and len(disabled['runs']) == 4
        for role in result['roles']:
            recovery = read(checks / role / 'promotion-recoveries/result.json')
            assert recovery['observations'] == recovery['exact_recoveries'] == 12
            assert recovery['benchmark_worker_sha256'] == result['roles'][role]['worker_sha256']
            assert read(checks / role / 'checked-emission/result.json')['artifacts'] == result['roles'][role]['artifact_sha256']
        inspection = read(checks / 'window-clean-probe-optimized-inspection/result.json')
        assert inspection == result['inspection']
        assert digest((checks / 'window-clean-probe-optimized-inspection/mech_compact_numeric_475_vector.clif').read_bytes()) == inspection['ir_sha256']
        assert inspection['artifacts'] == result['roles']['window-clean-probe-profiled']['artifact_sha256']
        for role in ['window-word', 'window-probe', 'window-packet', 'window-clean-word', 'window-clean-probe', 'window-clean-packet']:
            counts = read(checks / role / 'instruction-counts/counts.json')
            receipt = read(checks / role / 'receipt.json')
            assert counts['object_sha256'] == receipt['object_sha256']
            for case in counts['cases'].values():
                assert sum(block['visits'] * len(block['instructions']) for block in case['blocks']) == case['counts']['instructions']
                assert sum(block['visits'] * sum(op == 'b' or op.startswith('b.') or op in ['cbz', 'cbnz', 'tbz', 'tbnz'] for op, args in block['instructions']) for block in case['blocks']) == case['counts']['branches']
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
                                assert row[key][actor] < 1
                panels += 1
        assert panels == 6
        summary = result['builds']
        comparison = read(checks / 'window-clean-matched-builds/comparison.json')
        plan = read(checks / 'window-clean-matched-builds/plan.json')
        assert digest((checks / 'window-clean-matched-builds/plan.json').read_bytes()) == comparison['plan_sha256'] == summary['plan_sha256']
        observations = comparison['observations']
        assert len(observations) == summary['trials'] == 12
        for actor, job in plan['actors'].items():
            assert median(row[actor]['seconds']['source_through_link'] for row in observations) == summary['medians'][actor]['source_through_link']
            if actor.startswith('mech-'):
                for row in observations:
                    assert row[actor]['generated_artifacts'] == job['artifact_sha256']
                    assert row[actor]['stripped_executable']['bytes'] == 276072
        assert findings.rejected == findings.unresolved == []
    print('Verified 3 artifact/source pins, 23 repository tests, output/error/stack gates, 12 recoveries per candidate, exact optimized inspection, before/after instruction and branch totals, all six discovery orders, and 12 complete-build trials per actor. Qualified selection remains 81 Roc / 62 C.')

if __name__ == '__main__':
    main()

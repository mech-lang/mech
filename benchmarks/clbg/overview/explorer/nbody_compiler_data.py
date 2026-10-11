"""Import the matched full-size n-body compiler audit without admitting pilots."""
import hashlib
from pathlib import Path

COHORT = 'nbody/experiments/20261011-shared-products'


def add_nbody_compiler_audit(root, data, load, measured, sha):
    summary = load(COHORT + '/summary.json')
    report = load(COHORT + '/replication-50m.json')
    assert summary['status'] == 'passed' and report['status'] == 'complete'
    assert report['steps'] == 50_000_000 and report['samples'] == 7
    assert not report['discarded_samples'] and summary['completed_samples'] == 49
    assert report['required_power'] == dict(source='AC Power', low_power_mode=0)
    assert all(check['bit_identical'] for check in report['state_checks'])
    assert not summary['source_changes'] and not summary['specialized_host_kernel']
    for name, digest in summary['reports'].items():
        assert sha(root / COHORT / name) == digest
    for name, digest in summary['compiler_pins'].items():
        assert sha(root.parents[1] / name) == digest
        assert sha(root / COHORT / 'compiler-final' / Path(name).name) == digest
    independent = load(COHORT + '/independent-state-audit.json')
    objects = load(COHORT + '/selected-object-verification.json')
    assert independent['status'] == objects['status'] == 'passed'
    assert sha(root / COHORT / 'independent-state-audit.json') == summary['independent_state_audit_sha256']
    assert sha(root / COHORT / 'selected-object-verification.json') == summary['selected_object_verification_sha256']
    assert independent['max_scaled_error'] <= 1e-10
    assert all(row['identical_to_measured_object'] for row in objects['programs'])
    rows = {row['id']: row for row in report['programs']}
    keys = [('compact-baseline', summary['selected']['compact'], 'mech'),
            ('pairwise-mass-tails', summary['selected']['fast'], 'mech'),
            ('nbody-gpp-6-llvm-native', 'nbody-gpp-6-llvm-native', 'cpp'),
            ('chapel-3', 'chapel-3', 'chapel'), ('rust-3', 'rust-3', 'rust'),
            ('nbody-zig-2-faithful', 'nbody-zig-2-faithful', 'zig')]
    updated = []
    for target, identity, lang in keys:
        key = 'nbody:' + target
        row, previous = rows[identity], data['programs'][key]
        source = row if lang == 'mech' else summary['control_sources'][identity]
        assert previous['language'] == lang and previous['loc'] == row['loc']
        assert previous['files'] == [source['source_sha256']]
        assert sha(source['source']) == source['source_sha256']
        assert sha(row['command'][0]) == row['binary_sha256']
        assert len(row['samples_seconds']) == 7
        extra = dict(binary_sha256=row['binary_sha256'], runtime_policy=previous.get('runtime_policy', 'AOT'),
                     previous_measurement={k: previous[k] for k in ('seconds', 'samples', 'timing_sd', 'report', 'binary_sha256', 'full_output_validation') if k in previous},
                     timing_summary='Seven matched full-size fresh-process samples; startup, initialization and output included. Compilation excluded. C++, Chapel, Rust and Zig controls were remeasured in the same shuffled rounds.')
        validations = []
        for stdout in row['sample_stdout']:
            values = [float(line) for line in stdout.splitlines()]
            assert len(values) == 2
            assert all(abs(a - b) <= 1e-8 for a, b in zip(values, report['expected_energy']))
            validations.append(dict(bytes=len(stdout.encode()), sha256=hashlib.sha256(stdout.encode()).hexdigest(),
                                    energies=values, tolerance=1e-8, reference_match=True))
        extra['full_output_validation'] = validations
        if lang == 'mech':
            assert not row['codegen']['loop_calls']
            extra.update(native_flags=row['generation_command'][6:],
                         native_object_sha256=row['object_sha256'],
                         source_provenance='Unchanged pure Mech source; full 50-million-step work audit. Compact retains general register-pressure scheduling. The fast form selects square-reduction seed simplification and eight-instruction scheduling regions, with a replicated 1.4% improvement. The experimental shared-product pass is not selected.',
                         algorithm=dict(strict=True, status='reviewed', native_benchmark_kernel=False,
                                        reason='All pair forces, velocity and position updates are authored in Mech. General scheduling preserves the arithmetic graph and complete state bits.',
                                        full_work=summary['workload'],
                                        evidence=[dict(path=COHORT + '/summary.json', sha256=sha(root / COHORT / 'summary.json'))]))
        measured(key, 'nbody', lang, row['loc'], row['samples_seconds'],
                 source['source'], source['source_sha256'], COHORT + '/replication-50m.json',
                 COHORT + '/summary.json', **extra)
        updated.append(key)
    site = Path(__file__).parent
    for source, destination in [('matched-mech.csv', 'nbody-compiler-mech.csv'),
                                ('README.md', 'nbody-compiler-audit.md')]:
        (site / 'dist' / destination).write_bytes((root / COHORT / source).read_bytes())
        data['provenance'][destination] = sha(site / 'dist' / destination)
    data['nbody_compiler_audit'] = dict(summary, updated_endpoints=updated,
                                      confirmation_report=COHORT + '/replication-50m.json')
    data['provenance']['nbody_compiler_data.py'] = sha(__file__)
    data['protocol']['nbody_compiler'] = ('Two separate seven-round full-size n-body cohorts confirm a 1.4% improvement for the unchanged 25-LOC Mech source. '
        'The chart uses the final matched replication, including C++, Chapel, Rust and Zig controls. '
        'The fast form selects general square-reduction simplification and eight-instruction scheduling regions; '
        'the experimental shared-product pass is not selected. The compact form retains its previous 20-LOC source and scheduling flags. '
        'Other sources and measurements remain in their existing cohorts.')

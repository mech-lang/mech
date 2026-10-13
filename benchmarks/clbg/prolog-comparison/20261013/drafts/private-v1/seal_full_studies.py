"""Unexecuted full-index candidate. Actual bounded/work receipts are prerequisites."""
import argparse
import copy
import fcntl
import hashlib
import json
from pathlib import Path
import sys

PACKET = Path(__file__).resolve().parent
HERE = PACKET.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualification', type=Path, required=True)
    parser.add_argument('--qualification-sha256', required=True)
    parser.add_argument('--work-qualification', type=Path, required=True)
    parser.add_argument('--work-qualification-sha256', required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    assert args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for c in args.revision)
    print('WAIT common mutex: Prolog exact full-index candidate', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        import guarded_prepare as preparation
        initial = {str(path.resolve()): preparation.sha(path) for path in preparation.static_paths()}
        contracts = preparation.module(PACKET / 'contracts.py', 'contracts', initial)
        full_study = preparation.module(PACKET / 'full_study.py', 'prolog_full_controller', initial)
        assert full_study.GATES == contracts.GATES
        qualified_path = args.qualification.resolve()
        work_path = args.work_qualification.resolve()
        assert preparation.sha(qualified_path) == args.qualification_sha256
        assert preparation.sha(work_path) == args.work_qualification_sha256
        qualified = json.loads(qualified_path.read_text())
        assert qualified['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
        assert len(qualified['profiles']) == 20 and len(qualified['attempts']) == 156
        assert qualified['total_bounded_cases'] == 156 and qualified['all20_bounded_outputs_qualified'] is True
        preparation.verify(qualified['pins'])
        proposal = json.loads(Path(qualified['proposal']).read_text())
        assert preparation.sha(qualified['proposal']) == qualified['proposal_sha256']
        service, references, guard, harness, reused = preparation.reused_service(proposal['service_proposal_sha256'])
        expected_ids = {f'prolog-{bench}-{form}' for bench in contracts.BENCHMARKS for form in ('compact','optimized')}
        rows = list(qualified['profiles'].values())
        assert {row['id'] for row in rows} == expected_ids
        cases = preparation.bounded_cases(service, references)
        observed = set()
        for attempt in qualified['attempts']:
            contracts.complete_attempt(attempt, False)
            preparation.native_raw_receipts(attempt)
            identity = (attempt['id'], attempt['case'])
            assert identity not in observed
            observed.add(identity)
        assert observed == {(row['id'], case) for row in rows for case, _, _ in cases[row['benchmark']]}
        work = contracts.full_work_receipt(json.loads(work_path.read_text()), rows, preparation.verify)
        audit_path = full_study.SUITE / 'source-audit-r.json'
        audit = json.loads(audit_path.read_text())
        assert audit['status'] == 'reviewed'
        common = preparation.merge(initial, qualified['pins'], work['pins'], audit['pins'], reused,
            {str(qualified_path): args.qualification_sha256, str(work_path): args.work_qualification_sha256,
             str(audit_path): preparation.sha(audit_path), str(full_study.GUARD): full_study.GUARD_SHA})
        preparation.verify(common)
        output = HERE / 'studies' / args.revision
        assert not output.exists(), 'No implicit resume or overwritten full study'
        output.mkdir(parents=True)
        index = dict(status='sealed-awaiting-root-review', benchmarks={}, samples=3, full_processes=90,
            qualification=str(qualified_path), qualification_sha256=args.qualification_sha256,
            required_work_qualification=str(work_path), required_work_qualification_sha256=args.work_qualification_sha256,
            chart_admission=False, canonical_LOC_status='pending separate exact whole-bundle canonical overlay',
            no_automatic_retry=True, shared_timing_lock=str(LOCK))
        try:
            for bench in contracts.BENCHMARKS:
                candidates = [copy.deepcopy(row) for row in rows if row['benchmark'] == bench]
                assert len(candidates) == 2 and {row['variant'] for row in candidates} == {'compact','optimized'}
                pins = dict(common)
                for row in candidates:
                    assert row['bounded_output_qualified'] is True
                    assert row['compiler_bound_input_inventory']['status'] == 'qualified-exact-compiled-inputs'
                    assert row['command'][0] == row['binary'] and row['maximum_workers'] == 1
                    pins = preparation.merge(pins, row['application_artifact_pins'], row['runtime_closure_pins'])
                    proof = work['profiles'][row['id']]
                    row.update(loc=None, loc_status='pending actual qualified original Prolog source-owner overlay',
                        eligible_for_chart=False, chart_admission=False,
                        required_work_review=dict(status='passed', source_sha256=row['source_sha256'],
                            qualification=str(work_path), proof=proof))
                control = copy.deepcopy(next(row for row in audit['controls'] if row['benchmark'] == bench))
                if bench == 'mandelbrot':
                    path = full_study.CLBG / 'mandelbrot/experiments/20261011-array-state/source-modes-study/full-study/manifest.json'
                    actual = json.loads(path.read_text())
                    control = copy.deepcopy(next(row for row in actual['profiles'] if row['id'] == 'row16-aot-control'))
                    pins = preparation.merge(pins, actual['pins'], {str(path): preparation.sha(path)})
                assert control['language'] == 'mech' and control['benchmark'] == bench
                assert preparation.sha(control['source']) == control['source_sha256']
                assert preparation.sha(control['binary']) == control['binary_sha256']
                pins = preparation.merge(pins, control.get('support', {}),
                    {control['source']: control['source_sha256'], control['binary']: control['binary_sha256']})
                control.update(id='mech-aot-control', key=bench + ':prolog-full-study:mech-control',
                    producer_loc=control.get('loc'), loc=None, maximum_workers=8,
                    role='Fresh unchanged current Mech AOT control', chart_admission=False)
                profiles = candidates + [control]
                reference_paths = []
                if bench == 'nbody':
                    reference = None
                elif bench in ('pidigits', 'regexredux'):
                    path = full_study.CLBG / bench / 'inputs/output-full.txt'
                    reference = dict(bytes=path.stat().st_size, sha256=preparation.sha(path))
                    reference_paths.append(path)
                elif bench in ('spectralnorm', 'fannkuchredux'):
                    data = {'spectralnorm': b'1.274224153\n',
                            'fannkuchredux': b'3968050\nPfannkuchen(12) = 65\n'}[bench]
                    reference = dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())
                else:
                    count, digest = harness.compact.HASHES[bench][1]
                    reference = dict(bytes=count, sha256=digest)
                if bench in harness.INPUTS:
                    reference_paths.append(harness.INPUTS[bench][1])
                pins = preparation.merge(pins, {str(path): preparation.sha(path) for path in reference_paths})
                identifiers = [row['id'] for row in profiles]
                manifest = dict(status='prepared-awaiting-root-review', benchmark=bench, created_at=guard.now(),
                    samples=3, size=harness.WORK[bench][1], canonical_full_workload=True, profiles=profiles,
                    orders=[dict(round=i+1, profiles=identifiers[i:]+identifiers[:i]) for i in range(3)],
                    timeout_seconds=21600, maximum_rss_mib=10240, pins=pins,
                    full_output_reference=reference, chart_admission=False, no_automatic_retry=True,
                    shared_timing_lock=str(LOCK), compilation_outside_timer=True,
                    runtime_and_QLF_loading_inside_timer=True)
                preparation.verify(pins)
                path = output / bench / 'manifest.json'
                preparation.save_new(path, manifest)
                index['benchmarks'][bench] = dict(manifest=str(path), sha256=preparation.sha(path))
            assert len(index['benchmarks']) == 10
            preparation.save_new(output / 'index.json', index)
            print(json.dumps(dict(index=str(output/'index.json'), sha256=preparation.sha(output/'index.json'))), flush=True)
        except BaseException as error:
            preparation.save_new(output / 'failed-index.json', dict(status='stopped-unqualified',
                error=str(error) or type(error).__name__, error_type=type(error).__name__,
                completed_manifests=index['benchmarks'], chart_admission=False, full_processes_authorized=0))
            raise


if __name__ == '__main__':
    main()

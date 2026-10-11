"""Archive and admit an explicit reviewed candidate plan at an exited-stage boundary.

The plan pins every additive manifest. Old measured evidence survives only for
execution-identical candidates. Source-reviewed retirements retain original
sources and actual attempts; they do not create fake full-workload failures.
"""
import argparse
import copy
import fcntl
import json
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from prepare import G, LOCK, freeze, save, sha
from resume_after_guard_repair import migrate
from run import PinGuard


def now():
    return datetime.now(timezone.utc).isoformat()


def annotate_apl(manifest):
    result = copy.deepcopy(manifest)
    corrections = {
        'native APL array primitives may use eight CPU threads.':
            'The --cc8 flag requests an eight-core ceiling; this pinned macOS build disables parallel numerical primitives and executes them serially.',
        'allow eight-thread array primitives.':
            'request an eight-core ceiling, while this pinned macOS build executes numerical primitives serially.',
    }
    for row in result['programs']:
        for old, new in corrections.items():
            row['adaptations'] = [text.replace(old, new) for text in row.get('adaptations', [])]
    runtime_note = dict(enabled=False, requested_cc_is_ceiling_only=True,
        reason='This pinned GNU APL2.0 macOS configuration leaves HAVE_AFFINITY_NP undefined; '
               'src/Parallel.hh therefore undefines PARALLEL_ENABLED. No runtime or source byte changes.',
        proof_source=str(G / '.build/toolchains/apl-2.0/src/Parallel.hh'),
        proof_source_sha256=sha(G / '.build/toolchains/apl-2.0/src/Parallel.hh'),
        proof_config=str(G / '.build/toolchains/apl-2.0/config.h'),
        proof_config_sha256=sha(G / '.build/toolchains/apl-2.0/config.h'))
    result['runtime']['numerical_primitive_parallelism'] = runtime_note
    for row in result['programs']:
        if row.get('runtime'):
            row['runtime']['numerical_primitive_parallelism'] = copy.deepcopy(runtime_note)
    result['runtime_annotation_correction'] = runtime_note
    return result


def preview(language, descriptor, plan):
    original_path = G / (language + '-manifest.json')
    result = json.loads(original_path.read_text())
    known = {row['key'] for row in result['programs']}
    for extra_descriptor in descriptor['additions']:
        path = Path(extra_descriptor['path']).resolve()
        assert sha(path) == extra_descriptor['sha256'], ('Reviewed manifest changed', path)
        addition = json.loads(path.read_text())
        assert addition['language'] == language
        assert 'pending' not in addition.get('status', '').lower(), path
        for original in addition['programs']:
            row = copy.deepcopy(original)
            assert row['key'] not in known, ('Duplicate additive key', row['key'])
            assert row['small_validation']['reference_match'] is True
            assert row['algorithm']['strict'] and row['algorithm'].get('native_benchmark_kernel') is False
            assert sha(row['source']) == row['source_sha256']
            row.pop('execution_fingerprint', None)
            result['programs'].append(row)
            known.add(row['key'])
        result.setdefault('additive_preparations', []).append(dict(
            path=str(path), sha256=sha(path), preparation=addition))
    for key, reason in descriptor.get('retire', {}).items():
        assert key in known, ('Unknown retirement', key)
        row = next(row for row in result['programs'] if row['key'] == key)
        row.update(excluded=True, shortlist_status='prepared-unselected-retained', exclusion_reason=reason)
    if language == 'apl':
        result = annotate_apl(result)
    result.setdefault('preparation_revisions', []).append(dict(
        revision=plan['revision'], reason=plan['reason'],
        retained_prior_manifest=str(original_path), retained_prior_manifest_sha256=sha(original_path),
        fresh_control_benchmarks=descriptor.get('fresh_controls', []),
        retirements=descriptor.get('retire', {})))
    result['status'] = 'prepared'
    return result


def revise_controls(language, old, archive, descriptor, plan, plan_path):
    path = G / ('source-audit-' + language + '.json')
    audit = json.loads(path.read_text())
    normal_path = archive / (path.stem + '.pre-revision-prepare.json')
    shutil.copy2(path, normal_path)
    prior = {row['benchmark']: row for row in old['controls']}
    fresh = descriptor.get('fresh_controls', [])
    assert set(fresh) <= set(prior)
    changes = []
    for row in audit['controls']:
        before = prior[row['benchmark']]
        stripped = lambda item: {key: value for key, value in item.items() if key not in ['key', 'id']}
        assert stripped(row) == stripped(before), ('Control execution changed', row['benchmark'])
        if row['benchmark'] in fresh:
            row['key'] = row['key'] + '-' + plan['revision']
            row['id'] = row['id'] + '-' + plan['revision']
            assert row['key'] != before['key']
        else:
            row['key'], row['id'] = before['key'], before['id']
        audit['baseline_endpoints'][row['benchmark']]['control_key'] = row['key']
        changes.append(dict(benchmark=row['benchmark'], prior_key=before['key'],
                            new_key=row['key'], fresh=row['benchmark'] in fresh))
    script = Path(__file__).resolve()
    audit['pins'][str(script)] = sha(script)
    audit['pins'][str(plan_path)] = sha(plan_path)
    if language == 'apl':
        annotation = json.loads((G / 'apl-manifest.json').read_text())['runtime_annotation_correction']
        audit['pins'][annotation['proof_source']] = annotation['proof_source_sha256']
        audit['pins'][annotation['proof_config']] = annotation['proof_config_sha256']
    audit['prepared_revision'] = dict(revision=plan['revision'], reason=plan['reason'], at=now(),
        revision_script=str(script), revision_script_sha256=sha(script),
        plan=str(plan_path), plan_sha256=sha(plan_path), changes=changes,
        prior_source_audit=str(archive / path.name), prior_source_audit_sha256=sha(archive / path.name),
        pre_revision_prepare_audit=str(normal_path), pre_revision_prepare_audit_sha256=sha(normal_path),
        prior_control_revision=copy.deepcopy(old.get('control_revision')),
        prior_additive_revision=copy.deepcopy(old.get('additive_revision')))
    save(path, audit)
    PinGuard(audit['pins']).verify()


def apply(plan_path, plan, archive, coordinator_pid):
    assert coordinator_pid > 0
    listing = subprocess.check_output(['ps', '-axo', 'pid='], text=True)
    assert coordinator_pid not in [int(value) for value in listing.split()], 'Owning coordinator must already have exited'
    with LOCK.open('a+') as timing_lock:
        fcntl.flock(timing_lock, fcntl.LOCK_EX)
        with (G / 'results/.summary.lock').open('a+') as summary_lock:
            fcntl.flock(summary_lock, fcntl.LOCK_EX)
            previews = {language: preview(language, descriptor, plan)
                        for language, descriptor in plan['languages'].items()}
            old_audits = {}
            files = [Path(__file__).resolve(), plan_path] + [G / name for name in
                ['prepare.py', 'run.py', 'summarize.py', 'resume_after_guard_repair.py']]
            for language, descriptor in plan['languages'].items():
                audit_path = G / ('source-audit-' + language + '.json')
                old_audits[language] = json.loads(audit_path.read_text())
                PinGuard(old_audits[language]['pins']).verify()
                report_path = G / 'results' / ('timings-' + language + '.json')
                report = json.loads(report_path.read_text())
                assert not report.get('active'), ('Active report', language)
                assert report['source_audit_sha256'] == sha(audit_path)
                for key in descriptor.get('must_have_saved_full_attempt', []):
                    attempts = [row for row in report['attempts']
                                if row['key'] == key and row.get('full_workload') and row.get('finished_at')]
                    assert attempts, ('Required real full trial not preserved', key)
                files += [G / (language + '-manifest.json'), audit_path, report_path]
            archive.mkdir(parents=True, exist_ok=False)
            for path in files:
                assert not (archive / path.name).exists(), ('Archive basename collision', path)
                shutil.copy2(path, archive / path.name)
            evidence = dict(revision=plan['revision'], reason=plan['reason'], created_at=now(),
                status='archived-before-mutation', coordinator_pid_confirmed_exited=coordinator_pid,
                files={str(archive / path.name): sha(archive / path.name) for path in files})
            save(archive / 'revision.json', evidence)
            for language, manifest in previews.items():
                save(G / (language + '-manifest.json'), manifest)
                freeze(language)
                revise_controls(language, old_audits[language], archive, plan['languages'][language], plan, plan_path)
                migrate(language, archive, plan['reason'])
            evidence.update(status='complete', finished_at=now(),
                current_audits={language: sha(G / ('source-audit-' + language + '.json'))
                                for language in previews})
            save(archive / 'revision.json', evidence)
            print('PREPARED REVISION COMPLETE', archive, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', required=True)
    parser.add_argument('--archive', required=True)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--coordinator-exited-pid', type=int)
    args = parser.parse_args()
    plan_path = Path(args.plan).resolve()
    plan = json.loads(plan_path.read_text())
    assert plan['approved_by_root'] is True
    assert plan['revision'] and plan['reason'] and plan['languages']
    assert set(plan['languages']) <= {'apl', 'uiua'}
    for language, descriptor in plan['languages'].items():
        manifest = preview(language, descriptor, plan)
        print('PREVIEW', language, len(manifest['programs']), 'prepared;',
              sum(not row.get('excluded') for row in manifest['programs']), 'selected', flush=True)
    if args.apply:
        assert args.coordinator_exited_pid
        apply(plan_path, plan, Path(args.archive).resolve(), args.coordinator_exited_pid)
    else:
        print('No frozen preparation or actual measurement evidence modified.', flush=True)


if __name__ == '__main__':
    main()

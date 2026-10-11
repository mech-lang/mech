"""Archive and freeze approved additive APL/Uiua candidates with fresh controls.

This is preparation metadata only: measured attempts are never rewritten to claim
newly recorded fields. The old audit/report, normal prepare outputs, and source
manifests are retained before changing the active shortlist or control keys.
"""
import argparse
import copy
import fcntl
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path

from prepare import G, LOCK, freeze, save, sha
from resume_after_guard_repair import migrate
from run import PinGuard

REVISION = 'additive-v3'
ADDITIONS = {
    'apl': ['apl-regex-v3/manifest.json', 'apl-revcomp-v2/manifest.json'],
    'uiua': ['uiua-spectral-v3/manifest.json'],
}
FRESH_CONTROLS = {'apl': ['regexredux', 'revcomp'], 'uiua': ['spectralnorm']}
RETIRE_UIUA = {
    'spectralnorm:spectralnorm-uiua-compact':
        'Retired from the active shortlist after one complete canonical trial. '
        'The approved native-array 9 LOC form has the same normalized LOC and '
        'performs the same fresh coefficient work. The scalar source and real '
        '2307.811771667 s trial remain in immutable evidence and historical attempts.',
    'spectralnorm:spectralnorm-uiua-optimized':
        'Retired scalar coefficient composition in favor of the approved '
        'general native-array 17 LOC blocked implementation. Original source '
        'and its preparation evidence remain archived; no full timing is claimed.',
    'spectralnorm:spectralnorm-uiua-vectorized-compact-v3':
        'Prepared and validated 13 LOC candidate retained but not selected for '
        'full measurement: the equivalent inlined 9 LOC form is the compact '
        'shortlist candidate. No timing superiority is asserted without measurements.',
}
REASON = (
    'Root-approved additive standard-library/native-array preparation revision. '
    'APL adds generic PCRE2 count and bounded streaming reverse complement; '
    'Uiua selects faithful native-array spectral forms and retires scalar '
    'coefficient composition. Older sources, successful trials, failures, and '
    'guard evidence are retained. Affected benchmarks receive new control keys '
    'for three freshly interleaved unchanged Mech executions. Existing candidate '
    'evidence is preserved only for identical source, command, runtime, and support.'
)


def now():
    return datetime.now(timezone.utc).isoformat()


def preview(language):
    manifest_path = G / (language + '-manifest.json')
    manifest = json.loads(manifest_path.read_text())
    result = copy.deepcopy(manifest)
    known = {r['key'] for r in result['programs']}
    preparations = []
    for relative in ADDITIONS[language]:
        path = G / relative
        addition = json.loads(path.read_text())
        assert addition['language'] == language
        assert all(r['small_validation']['reference_match'] is True
                   for r in addition['programs'])
        for row in addition['programs']:
            assert row['key'] not in known, ('Already merged', row['key'])
            assert sha(row['source']) == row['source_sha256']
            assert row['algorithm']['strict']
            assert row['algorithm']['native_benchmark_kernel'] is False
            result['programs'].append(copy.deepcopy(row))
            known.add(row['key'])
        # Preserve producer metadata and its support pins as well as row pins.
        preparations.append({'path': str(path), 'sha256': sha(path),
                             'preparation': addition})
    if language == 'uiua':
        for row in result['programs']:
            if row['key'] in RETIRE_UIUA:
                row.update(excluded=True, exclusion_reason=RETIRE_UIUA[row['key']],
                           shortlist_status='prepared-unselected-retained')
        assert set(RETIRE_UIUA) <= known
    result.setdefault('additive_preparations', []).extend(preparations)
    result.setdefault('preparation_revisions', []).append({
        'revision': REVISION, 'reason': REASON,
        'fresh_control_benchmarks': FRESH_CONTROLS[language],
        'retained_prior_manifest': str(manifest_path),
        'retained_prior_manifest_sha256': sha(manifest_path),
    })
    result['status'] = 'prepared'
    return result


def revise_controls(language, archive):
    path = G / ('source-audit-' + language + '.json')
    audit = json.loads(path.read_text())
    # Preserve exactly what the unchanged common preparer produced.
    shutil.copy2(path, archive / (path.stem + '.pre-revision-prepare.json'))
    changes = []
    for row in audit['controls']:
        if row['benchmark'] not in FRESH_CONTROLS[language]:
            continue
        before = copy.deepcopy(row)
        old_key = row['key']
        row['key'] = old_key + '-' + REVISION
        row['id'] += '-' + REVISION
        endpoint = audit['baseline_endpoints'][row['benchmark']]
        assert endpoint['control_key'] == old_key
        endpoint['control_key'] = row['key']
        assert {k: v for k, v in row.items() if k not in ['id', 'key']} == {
            k: v for k, v in before.items() if k not in ['id', 'key']}
        changes.append({'benchmark': row['benchmark'], 'prior_key': old_key,
                        'new_key': row['key'], 'unchanged_execution': before})
    assert len(changes) == len(FRESH_CONTROLS[language])
    helper = Path(__file__).resolve()
    audit['pins'][str(helper)] = sha(helper)
    audit['control_revision'] = {
        'revision': REVISION, 'at': now(), 'reason': REASON,
        'pre_revision_prepare_audit': str(
            archive / (path.stem + '.pre-revision-prepare.json')),
        'pre_revision_prepare_audit_sha256': sha(
            archive / (path.stem + '.pre-revision-prepare.json')),
        'revision_script': str(helper), 'revision_script_sha256': sha(helper),
        'changes': changes,
    }
    save(path, audit)
    PinGuard(audit['pins']).verify()


def apply(archive, previews):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        archive.mkdir(parents=True, exist_ok=False)
        files = [G / name for name in ['prepare.py', 'run.py', 'summarize.py',
                 'resume_after_guard_repair.py', 'refreeze_additive.py']]
        for language in previews:
            files += [G / (language + '-manifest.json'),
                      G / ('source-audit-' + language + '.json'),
                      G / 'results' / ('timings-' + language + '.json')]
            audit = json.loads((G / ('source-audit-' + language + '.json')).read_text())
            PinGuard(audit['pins']).verify()
            report = json.loads((G / 'results' / ('timings-' + language + '.json')).read_text())
            assert not report.get('active'), ('Active full run', language)
            assert report['source_audit_sha256'] == sha(G / ('source-audit-' + language + '.json'))
        for path in files:
            shutil.copy2(path, archive / path.name)
        evidence = {'revision': REVISION, 'reason': REASON,
                    'created_at': now(), 'status': 'archived-before-mutation',
                    'files': {str(archive / p.name): sha(archive / p.name) for p in files}}
        save(archive / 'revision.json', evidence)
        for language, manifest in previews.items():
            save(G / (language + '-manifest.json'), manifest)
            freeze(language)
            revise_controls(language, archive)
            migrate(language, archive, REASON)
        evidence.update(status='complete', finished_at=now(),
                        current_audits={language: sha(G / ('source-audit-' + language + '.json'))
                                        for language in previews})
        save(archive / 'revision.json', evidence)
        print('ADDITIVE REVISION COMPLETE', archive, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', default=str(G / 'evidence' / 'additive-v3-before'))
    parser.add_argument('--apply', action='store_true', help='Only after parent approval and QA completion')
    args = parser.parse_args()
    previews = {language: preview(language) for language in ADDITIONS}
    for language, manifest in previews.items():
        print('PREVIEW', language, len(manifest['programs']), 'prepared roles;',
              sum(not r.get('excluded') for r in manifest['programs']), 'selected roles;',
              'fresh controls', FRESH_CONTROLS[language], flush=True)
    if args.apply:
        apply(Path(args.archive).resolve(), previews)
    else:
        print('No manifest, audit, report, or timing data modified.', flush=True)


if __name__ == '__main__':
    main()

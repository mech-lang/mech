"""Admit approved APL/Uiua k-nucleotide candidates at the medium-stage boundary.

Archive prior v3 preparation and measured evidence. Retain previous fresh
control identities and all execution-identical trials; never invent failures
for source-reviewed candidates that were not run at the canonical size.
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

ADDITIONS = {'apl': 'apl-knucleotide-v2/manifest.json',
             'uiua': 'uiua-knucleotide-v2/manifest.json'}
REASON = (
    'Root-approved additive k-nucleotide preparation revision before medium '
    'full runs. All programs visit/count every input position and construct '
    'the seven full growing maps, with no generator-period shortcut. APL adds '
    'bounded streaming/batched application arrays and retires unmeasured '
    'whole-input boxed-array candidates on documented workspace risk; no full '
    'failure is invented. Uiua retains both prior source forms and adds its '
    'generic native-group optimized candidate for actual full measurements. '
    'Earlier control identities and execution-identical evidence remain intact.'
)


def now():
    return datetime.now(timezone.utc).isoformat()


def preview(language):
    path = G / (language + '-manifest.json')
    result = json.loads(path.read_text())
    extra_path = G / ADDITIONS[language]
    extra = json.loads(extra_path.read_text())
    assert extra['language'] == language
    known = {r['key'] for r in result['programs']}
    for row in extra['programs']:
        assert row['key'] not in known, ('Already merged', row['key'])
        assert row['benchmark'] == 'knucleotide'
        assert row['small_validation']['reference_match'] is True
        assert row['algorithm']['strict']
        assert row['algorithm']['native_benchmark_kernel'] is False
        assert sha(row['source']) == row['source_sha256']
    if language == 'apl':
        for row in result['programs']:
            if row['benchmark'] == 'knucleotide':
                row.update(excluded=True, shortlist_status='prepared-unselected-retained',
                           exclusion_reason='Source review: canonical 254166745-byte input '
                           'needs approximately4GiB just as GNU APL16-byte boxed '
                           'cells, beyond its macOS workspace admission estimate. '
                           'Retired before any full attempt in favor of approved '
                           'bounded streaming forms. Small validated originals '
                           'and provenance retained; no measured failure asserted.')
    result['programs'].extend(copy.deepcopy(extra['programs']))
    result.setdefault('additive_preparations', []).append({
        'path': str(extra_path), 'sha256': sha(extra_path), 'preparation': extra})
    result.setdefault('preparation_revisions', []).append({
        'revision': 'knucleotide-v2', 'reason': REASON,
        'retained_prior_manifest': str(path), 'retained_prior_manifest_sha256': sha(path)})
    result['status'] = 'prepared'
    return result


def retain_control_identities(language, old, archive):
    path = G / ('source-audit-' + language + '.json')
    audit = json.loads(path.read_text())
    saved = archive / (path.stem + '.pre-revision-prepare.json')
    shutil.copy2(path, saved)
    old_controls = {r['benchmark']: r for r in old['controls']}
    for row in audit['controls']:
        before = old_controls[row['benchmark']]
        # The previous preparation may have assigned a fresh additive control
        # identity. This revision preserves it, without changing execution.
        stripped = lambda r: {k: v for k, v in r.items() if k not in ['key', 'id']}
        assert stripped(row) == stripped(before), ('Control execution changed', row['benchmark'])
        row['key'], row['id'] = before['key'], before['id']
        audit['baseline_endpoints'][row['benchmark']]['control_key'] = row['key']
    helper = Path(__file__).resolve()
    audit['pins'][str(helper)] = sha(helper)
    audit['additive_revision'] = {
        'revision': 'knucleotide-v2', 'reason': REASON, 'at': now(),
        'revision_script': str(helper), 'revision_script_sha256': sha(helper),
        'pre_revision_prepare_audit': str(saved), 'pre_revision_prepare_audit_sha256': sha(saved),
        'prior_control_revision': copy.deepcopy(old.get('control_revision')),
        'prior_source_audit': str(archive / path.name),
        'prior_source_audit_sha256': sha(archive / path.name),
        'new_knucleotide_controls_have_no_prior_samples': True,
    }
    save(path, audit)
    PinGuard(audit['pins']).verify()


def apply(archive, previews):
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        archive.mkdir(parents=True, exist_ok=False)
        old_audits = {}
        paths = [G / name for name in ['prepare.py', 'run.py', 'summarize.py',
                 'resume_after_guard_repair.py', 'refreeze_additive.py', 'refreeze_knucleotide.py']]
        for language in previews:
            audit_path = G / ('source-audit-' + language + '.json')
            old_audits[language] = json.loads(audit_path.read_text())
            PinGuard(old_audits[language]['pins']).verify()
            report_path = G / 'results' / ('timings-' + language + '.json')
            report = json.loads(report_path.read_text())
            assert not report.get('active'), ('Active full run', language)
            assert report['source_audit_sha256'] == sha(audit_path)
            controls = [r for r in report['programs'].values()
                        if r['benchmark'] == 'knucleotide' and r['language'] == 'mech']
            assert len(controls) == 1 and not controls[0]['samples_seconds']
            assert not controls[0]['attempts'], 'New knucleotide control is not fresh'
            paths += [G / (language + '-manifest.json'), audit_path, report_path]
        for path in paths:
            shutil.copy2(path, archive / path.name)
        evidence = {'revision': 'knucleotide-v2', 'reason': REASON,
                    'created_at': now(), 'status': 'archived-before-mutation',
                    'files': {str(archive / p.name): sha(archive / p.name) for p in paths}}
        save(archive / 'revision.json', evidence)
        for language, manifest in previews.items():
            save(G / (language + '-manifest.json'), manifest)
            freeze(language)
            retain_control_identities(language, old_audits[language], archive)
            migrate(language, archive, REASON)
        evidence.update(status='complete', finished_at=now(),
                        current_audits={language: sha(G / ('source-audit-' + language + '.json'))
                                        for language in previews})
        save(archive / 'revision.json', evidence)
        print('K-NUCLEOTIDE REVISION COMPLETE', archive, flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', default=str(G / 'evidence/knucleotide-v2-before'))
    parser.add_argument('--apply', action='store_true', help='Only at the approved completed-stage boundary')
    args = parser.parse_args()
    previews = {language: preview(language) for language in ADDITIONS}
    for language, manifest in previews.items():
        print('PREVIEW', language, len(manifest['programs']), 'prepared;',
              sum(not r.get('excluded') for r in manifest['programs']), 'selected', flush=True)
    if args.apply:
        apply(Path(args.archive).resolve(), previews)
    else:
        print('No frozen preparation or timing evidence modified.', flush=True)


if __name__ == '__main__':
    main()

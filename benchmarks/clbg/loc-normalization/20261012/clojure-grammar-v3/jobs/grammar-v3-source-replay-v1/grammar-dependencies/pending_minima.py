"""Bounded new compact-candidate record reconciliation; no execution/publication.

The caller owns the common timing mutex, supplies its digest function and a
frozen independent full-record validator. Prepared/null-LOC rows stay unscored
until all full records reconcile. Timings and producer fields never mutate.
"""
from copy import deepcopy
import json
from pathlib import Path


STUDIES = {
    'mandelbrot': ('mandelbrot-lua-2','lua'),
    'spectralnorm': ('spectralnorm-node-7','javascript'),
    'binarytrees': ('binarytrees-ruby-2','ruby'),
    'fannkuchredux': ('fannkuchredux-php-8','php'),
}
COHORT='loc-normalization/20261012/pending-minima'


def merge_validated(pool, records, *, benchmark, manifest_path, report_path, discarded):
    """Use only the explicitly identified foreign profile; archive fresh control."""
    identity,language=STUDIES[benchmark]
    if set(records)!={identity,'mech-aot-control'}:
        raise ValueError('Pending-minimum study profile inventory differs from its exact proposal.')
    row=records[identity]
    key=f'{benchmark}:{identity}'
    if row['id']!=identity or row['key']!=key or row['benchmark']!=benchmark or row['language']!=language:
        raise ValueError('Pending-minimum profile identity differs from its listed source.')
    control=records['mech-aot-control']
    if control['benchmark']!=benchmark or control['language']!='mech':
        raise ValueError('Pending-minimum control is not its named exact Mech profile.')
    samples=row['samples_seconds']
    if len(samples)!=3 or row.get('loc') is not None:
        raise ValueError('New candidate requires three full samples and retained null producer LOC.')
    prior=next((p for p in pool['pending_candidates'] if p['key']==key),None)
    if prior is None:
        raise ValueError('Completed candidate was not in the reconstructed pending listed source pool.')
    expected={ref['sha256'] for ref in prior['source_files']}
    if expected!={row['source_sha256']}:
        raise ValueError('Completed candidate differs from its original pending source identity.')
    candidate=dict(deepcopy(row), original_loc=prior.get('original_loc'),
        producer_loc=None, original_source_pool_loc=prior.get('original_loc'),
        original_source_pool_report=prior.get('source_report',prior.get('report')),
        samples=list(samples),seconds=row['median_seconds'],
        report=report_path,source_report=manifest_path,
        source_files=[dict(path=row['source'],sha256=row['source_sha256'])],
        source_identities=[],aliases=list(prior.get('aliases',[])),current_keys=[],
        already_audited=False,authoritative_loc=None,
        qualification=dict(admitted=True,report_completed=True,full_workload=True,
            sample_count=3,declared_sample_count=3,sample_count_matches=True,
            full_output_validated=True,
            validation_basis='Independent complete guarded full-record reconciliation',
            discarded_environment_records=len(discarded)),
        fresh_paired_mech_control=deepcopy(control),
        measurement_protocol='Full program execution with fresh exact-source Mech control; preserved producer records.')
    pool['candidates']=[p for p in pool['candidates'] if p['key']!=key]+[candidate]
    pool['pending_candidates']=[p for p in pool['pending_candidates'] if p['key']!=key]
    for path in (manifest_path,report_path):
        if path not in pool['report_paths']:pool['report_paths'].append(path)
    pool['reconciliation'].append(dict(report=report_path,
        status='complete-pending-minimum-study-independently-reconciled',
        candidate_key=key,profiles=len(records),discarded_environment_records=len(discarded),
        raw_producer_loc=None,canonical_loc_admitted=False,
        control_source_sha256=control['source_sha256'],control_timing_relabels=0))
    pool['candidates'].sort(key=lambda p:p['key'])
    return candidate


def reconcile(root,pool,*,digest,validate_study):
    """Use a qualified independent protocol adapter; incomplete stays pending.

    Actual pending-minimum manifests use the V full-study protocol, not the
    older generic isolated-study reference schema. ``validate_study`` must be
    a frozen, fixture-qualified adapter for that exact protocol and benchmark
    full-work/output oracles. This module supplies no permissive fallback.
    """
    root=Path(root)
    for benchmark,(identity,language) in STUDIES.items():
        base=f'{COHORT}/{benchmark}/full-study'
        manifest_path=f'{base}/manifest.json';report_path=f'{base}/report.json'
        if not (root/report_path).is_file():continue
        report=json.loads((root/report_path).read_text())
        if report.get('status')!='complete':continue
        manifest=json.loads((root/manifest_path).read_text())
        if manifest['benchmark']!=benchmark:
            raise ValueError('Pending-minimum manifest benchmark differs from its path.')
        if {p['id'] for p in manifest['profiles']}!={identity,'mech-aot-control'}:
            raise ValueError('Unexpected pending-minimum profile inventory.')
        if any(p.get('loc') is not None for p in manifest['profiles']):
            raise ValueError('Pending-minimum producer LOC was changed before source review.')
        for path,pin in manifest['pins'].items():
            if digest(path)!=pin:
                raise ValueError('Pending-minimum frozen pin changed: '+path)
        records,discarded=validate_study(manifest,report,
            manifest_sha256=digest(root/manifest_path),benchmark=benchmark,digest=digest)
        merge_validated(pool,records,benchmark=benchmark,manifest_path=manifest_path,
                        report_path=report_path,discarded=discarded)

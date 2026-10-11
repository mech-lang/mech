#!/usr/bin/env python3
"""Freeze additive parser-fix evidence and pins without running workloads."""
from copy import deepcopy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
SUITE = HERE.parent
PY = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    base = json.loads((SUITE / 'uiua-manifest.json').read_text())
    native = json.loads((SUITE / 'uiua-knucleotide-v2/manifest.json').read_text())
    derivation = json.loads((HERE / 'derivation.json').read_text())
    validation_path = HERE / 'results/validation.json'
    validation = json.loads(validation_path.read_text())
    assert validation['status'] == 'passed'
    assert validation['passed'] == validation['work_checks_passed'] == 69
    assert validation['fixtures_per_profile'] == 23
    assert len(validation['original_counterexamples']) == 6
    assert all(case['confirmed_parser_defect'] for case in validation['original_counterexamples'])
    assert validation['reference_validator_sha256'] == sha(SUITE / 'uiua-knucleotide-v2/validate.py')
    assert validation['shared_timing_lock'] == '/tmp/mech-clbg-timing.lock'
    assert validation['validator_sha256'] == sha(HERE / 'validate.py')
    runtime = deepcopy(base['runtime'])
    assert validation['runtime_sha256'] == runtime['sha256'] == sha(runtime['path'])
    old_rows = {
        'compact': next(row for row in base['programs'] if row['benchmark'] == 'knucleotide' and row['variant'] == 'compact'),
        'bounded': next(row for row in base['programs'] if row['benchmark'] == 'knucleotide' and row['variant'] == 'optimized'),
        'native-group': native['programs'][0],
    }
    audit = dict(status='passed-source-audit', created_at=datetime.now(timezone.utc).isoformat(),
                 scope='Generic EOF-aware comment cleaning and line-leading record boundaries; counting and formatting are preserved.',
                 derivation=derivation, runtime_sha256=runtime['sha256'],
                 original_counterexamples=validation['original_counterexamples'],
                 validation_file=str(validation_path), validation_sha256=sha(validation_path),
                 complete_seven_maps=7, widths=[1,2,3,4,6,12,18],
                 full_sequence_length=125000000, full_total_windows=874999961,
                 full_windows_by_width={str(width): 125000000-width+1 for width in [1,2,3,4,6,12,18]},
                 native_runtime_proof=deepcopy(json.loads((SUITE / 'uiua-knucleotide-v2/results/source-audit.json').read_text())['native_runtime_proof']),
                 native_benchmark_kernel=False, host_helpers=False,
                 generic_parser_fix='Line 3 accepts newline or EOF after a comment; line 4 captures complete non-header lines and stops only before a line-leading >. CR/LF cleanup, case conversion, all counting loops and formatting stay byte-for-byte identical.',
                 instrumentation=validation['instrumentation'], full_workload_measured=False)
    for item in derivation:
        original = Path(item['original_source'])
        source = Path(item['source'])
        assert sha(original) == item['original_source_sha256'] == old_rows[item['profile']]['source_sha256']
        assert sha(source) == item['source_sha256']
        before, after = original.read_text().splitlines(), source.read_text().splitlines()
        assert len(before) == len(after)
        assert [i+1 for i,(a,b) in enumerate(zip(before,after)) if a != b] == [3,4]
    audit_path = HERE / 'results/source-audit.json'
    audit_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False) + '\n')
    common_pins = dict(runtime['pins'])
    for proof in audit['native_runtime_proof']:
        common_pins[proof['path']] = proof['sha256']
    for path in [HERE / 'prepare.py', HERE / 'validate.py', HERE / 'prepare_manifest.py',
                 HERE / 'README.md', HERE / 'derivation.json', validation_path, audit_path,
                 SUITE / 'uiua-knucleotide-v2/validate.py', SUITE / 'uiua-knucleotide-v2/manifest.json',
                 SUITE / 'uiua-knucleotide-v2/results/source-audit.json']:
        common_pins[str(path)] = sha(path)
    for case in validation['original_counterexamples'] + validation['cases'] + validation['instrumentation']:
        for field in ('source', 'input_file', 'stdout_file', 'stderr_file'):
            if field in case:
                common_pins[case[field]] = sha(case[field])
    programs = []
    for item in derivation:
        profile = item['profile']
        row = deepcopy(old_rows[profile])
        source = Path(item['source'])
        source_hash = sha(source)
        count = sum(bool(line.split('#',1)[0].strip()) for line in source.read_text().splitlines())
        assert count == item['physical_loc'] == {'compact':19,'bounded':27,'native-group':29}[profile]
        assert all(len(re.findall(r'[A-Za-z][A-Za-z0-9]*\s*←',line.split('#',1)[0])) <= 1 for line in source.read_text().splitlines())
        sid = {'compact':'knucleotide-uiua-compact-v3',
               'bounded':'knucleotide-uiua-bounded-optimized-v3',
               'native-group':'knucleotide-uiua-native-group-optimized-v3'}[profile]
        cases = [deepcopy(case) for case in validation['cases'] if case['profile'] == profile]
        small = next(case for case in cases if case['case'] == 'canonical-small')
        pins = dict(row.get('pins', {}))
        pins.update(common_pins)
        pins[str(source)] = source_hash
        pins[item['original_source']] = item['original_source_sha256']
        execution = dict(source_sha256=source_hash, runtime_sha256=runtime['sha256'],
                         command=[runtime['path'],'run','--no-format','<source>'],
                         cwd=str(SUITE), runtime_environment=row['runtime_environment'],
                         library_pins={}, input_mode='stdin')
        row.update(id=sid, key='knucleotide:'+sid, source_profile=profile,
                   source=str(source), source_sha256=source_hash,
                   command=[runtime['path'],'run','--no-format',str(source)],
                   source_url=None, provenance='Additive EOF-comment repair of pinned '+profile+' source; originals and frozen evidence preserved.',
                   adaptations=row['adaptations'] + ['Ignore semicolon comments terminated by newline or EOF; stop record extraction only at line-leading header markers, retaining embedded > in ignored comments. All counting and formatting source preserved.'],
                   pins=pins, small_validation=small,
                   adversarial_validation=[case for case in cases if case['case'] != 'canonical-small'],
                   validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
                                            output_checks=23, complete_seven_map_checks=23),
                   original_source=item['original_source'], original_source_sha256=item['original_source_sha256'],
                   supersedes_key=old_rows[profile]['key'], source_audit=str(audit_path),
                   source_audit_sha256=sha(audit_path), loc=count, physical_loc=count, loc_expansions=[],
                   execution_fingerprint_inputs=execution,
                   execution_fingerprint=hashlib.sha256(json.dumps(execution,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                   eligible_for_chart=False, measurement_status='not-measured')
        row['algorithm']['strict'] = True
        row['algorithm']['source_audit'] = str(audit_path)
        row['algorithm']['source_audit_sha256'] = sha(audit_path)
        row.pop('timing_alias_of',None)
        row.pop('timing_alias_reason',None)
        programs.append(row)
    for row in programs:
        for path,digest in row['pins'].items():
            assert sha(path) == digest, path
    manifest = dict(status='prepared', language='uiua', runtime=runtime, pins=common_pins,
                    scope='Three additive k-nucleotide EOF-comment and line-leading record-boundary parser repairs; no full timing admission.',
                    programs=programs, source_audit=str(audit_path), source_audit_sha256=sha(audit_path),
                    validation_evidence=dict(path=str(validation_path), sha256=sha(validation_path),
                                             passed=69, work_checks_passed=69, fixtures_per_profile=23),
                    installation_command=[PY,str(SUITE/'install_uiua_runtime.py')],
                    preparation_commands=[[PY,str(HERE/name)] for name in ['prepare.py','validate.py','prepare_manifest.py']],
                    additive_migration_policy='Preserve original sources/frozen pins and timing evidence. Select explicitly through a new frozen audit; fresh full candidate/control measurements required before chart admission.')
    destination = HERE / 'manifest.json'
    destination.write_text(json.dumps(manifest,indent=2,ensure_ascii=False)+'\n')
    print(destination,sha(destination))
    for row in programs:
        print(row['key'],row['loc'],'LOC',row['source_sha256'])


if __name__ == '__main__':
    main()

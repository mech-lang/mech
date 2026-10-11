"""Freeze corrected sources and retain only identical, fully validated measurements."""
import copy
import json
from datetime import datetime, timezone
from pathlib import Path
from bootstrap import D, sha
from loc import normalize, fixtures
from probe import save

TREE_REASON = 'Recursively allocate both children for every interior node; every leaf and interior is a distinct allocation of the same size with two child slots. Construct and traverse the complete stretch tree, release it before constructing a separate long-lived tree, construct and traverse every prescribed temporary tree, and check the retained long-lived root only after all temporary work. General arenas may reset after a complete construction/check; independent worker arenas and a separate long-lived arena preserve object identity and lifetime.'


def main():
    fixtures()
    old_audit = json.loads((D/'source-audit.json').read_text())
    original = json.loads((D/'results/timings.json').read_text())
    extras = json.loads((D/'results/final-extra-builds.json').read_text())['programs']
    small = json.loads((D/'results/final-extra-small-checks.json').read_text())
    assert small['status'] == 'passed'
    elm = json.loads((D/'results/elm-tree-lifetime-final.json').read_text())
    native = json.loads((D/'results/native-tree-allocation-final.json').read_text())
    assert elm['status'] == native['status'] == 'passed'
    for filename,digest in {**old_audit['pin_files'],**old_audit['libraries']}.items():
        assert sha(filename) == digest,filename
    superseded = {r['supersedes'] for r in extras if r.get('supersedes')}
    assert superseded == {'binarytrees:binarytrees-elm-idiomatic','pidigits:pidigits-elm-idiomatic'}
    rows = [copy.deepcopy(row) for row in old_audit['programs'] if row['key'] not in superseded]
    for row in extras:
        assert row['build_status'] == 'passed'
        check = small['programs'][row['key']]
        assert check['status'] == 'passed' and check['validation']['reference_match']
        assert check['source_sha256'] == row['source_sha256'] and check['binary_sha256'] == row['binary_sha256']
        metric = normalize(Path(row['source']).read_text(),row['language'])
        reason = TREE_REASON if row['benchmark'] == 'binarytrees' else 'Full unbounded LFT spigot with growing signed integer state; evaluate floor((3*q+r)/t) and floor((4*q+r)/t) for every tested candidate, emit exactly n digits with the prescribed grouping and counters. Ordinary pure-Elm arbitrary-precision library; large-by-small multiplication operand order is a general arithmetic optimization, not an input-specific shortcut.'
        rows.append(dict(row,loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],small_validation=check['validation'],eligible_for_chart=True,
            algorithm=dict(strict=True,status='required-work-reviewed',reason=reason,native_benchmark_kernel=False)))
    for row in rows:
        assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256']
        if row['benchmark'] == 'binarytrees': row['algorithm']['reason'] = TREE_REASON
    assert len(rows) == 56
    old_builds = json.loads((D/'results/builds.json').read_text())
    final_builds = dict(old_builds,programs=[r for r in old_builds['programs'] if r['key'] not in superseded]+extras)
    assert len(final_builds['programs']) == len(rows)
    save(D/'results/final-builds.json',final_builds)
    roc = copy.deepcopy(old_audit['allocation_evidence']['roc'])
    ir_path = D/'evidence/tree-roc-optimized.ll'
    ir = ir_path.read_text().splitlines()
    assert '%call5.i = tail call fastcc ptr' in ir[3163]
    assert '%call4.i.i.i = tail call fastcc ptr' in ir[3278]
    assert '(ptr %call5.i)' in ir[3475]
    roc.update(long_lived_created_line=3164,temporary_constructor_line=3279,long_lived_checked_line=3476,
        long_lived_retained_through_temporary_loop=True,lifetime_reason='The optimized main constructs the long-lived pointer before entering temporary loops, and passes that same pointer to its final check after both loops. The 32-byte leaf/interior allocations survive optimization in both constructor specializations.')
    elm.update(source_sha256=next(r['source_sha256'] for r in rows if r['id']=='binarytrees-elm-faithful'))
    evidence = dict(status='passed',elm=elm,roc=roc,native_arenas=native,
        superseded_evidence='The original manual Elm allocation probe proved node identity but did not exercise actual run ordering. Its lifetime assertion is superseded by the actual compiled-run instrumented checks and the corrected source below.')
    evidence_path = D/'results/allocation-evidence-final.json'
    save(evidence_path,evidence)
    audit = copy.deepcopy(old_audit)
    audit.update(programs=rows,allocation_evidence=evidence,final_builds='results/final-builds.json',
        final_corrections='Two unmeasured Elm candidates are superseded: the tree source checked the long-lived root too early after compiler let-binding reordering; the pi source placed the large operand second in asymmetric schoolbook multiplication. Corrected sources preserve the full workload. Four general-arena Zig/Odin candidates add compact and parallel alternatives without native benchmark executors.')
    for path in [evidence_path,D/'results/final-extra-small-checks.json',D/'results/elm-tree-lifetime-final.json',D/'results/native-tree-allocation-final.json',
                 D/'evidence/audit-elm-tree-lifetime.cjs',D/'evidence/tree-elm-order-issue.json',D/'evidence/elm-pidigits-operand-order.json']:
        audit['libraries'][str(path)] = sha(path)
    for row in native['programs']:
        for field in ('instrumentation_source','instrumentation_binary'):
            audit['libraries'][row[field]] = row[field+'_sha256']
    for path in [D/'freeze_final.py',D/'summarize.py',D/'audit_native_trees.py',D/'tree_extra.py',D/'results/final-builds.json']:
        audit['pin_files'][str(path)] = sha(path)
    audit_path = D/'final-source-audit.json'
    save(audit_path,audit)
    reviewed = {r['key']:r for r in rows+audit['controls']}
    programs = {}
    retained = 0
    for key,row in reviewed.items():
        current = copy.deepcopy(row)
        current.update(samples_seconds=[],sample_validation=[])
        before = original['programs'].get(key)
        if before:
            assert before['source_sha256'] == row['source_sha256'] and before['binary_sha256'] == row['binary_sha256']
            assert all(v['reference_match'] for v in before['sample_validation'])
            for field in ('samples_seconds','sample_validation','full_validation','median_seconds','sample_stdev_seconds'):
                if field in before: current[field] = copy.deepcopy(before[field])
            retained += len(current['samples_seconds'])
        programs[key] = current
    exclusions=[]
    for key in sorted(superseded):
        before=original['programs'][key]
        replacement=next(row for row in extras if row.get('supersedes') == key)
        exclusions.append(dict(key=key,replaced_by=replacement['key'],source=before['source'],source_sha256=before['source_sha256'],binary_sha256=before['binary_sha256'],samples_excluded=len(before['samples_seconds']),
            reason='Incorrect compiled long-lived-tree lifetime; no full sample admitted.' if before['benchmark']=='binarytrees' else 'Superseded asymmetric multiplication operand order; unfinished run was terminated for a general arithmetic performance correction, and no full timing admitted.'))
    assert retained == 75
    report = copy.deepcopy(original)
    report.update(status='running',programs=programs,orders={},completed_benchmarks=[],
        source_audit=str(audit_path),source_audit_sha256=sha(audit_path),runtimes=audit['runtimes'],
        corrected_at=datetime.now(timezone.utc).isoformat(),work_exclusions=exclusions,
        retained_source_report=dict(path='results/timings.json',sha256=sha(D/'results/timings.json'),samples=retained,
            reason='Only identical source/executable hashes with full reference validation retained. Superseded unmeasured Elm sources remain preserved for audit.'))
    report.pop('finished_at',None)
    save(D/'results/final-timings.json',report)
    print('FINAL AUDIT',len(rows),'candidates;',len(audit['controls']),'controls;',retained,'retained samples;',len(reviewed)*3,'required samples',flush=True)


if __name__=='__main__': main()

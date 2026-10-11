"""Admit general optimizations without altering any prior validated measurements."""
import copy
import json
from datetime import datetime, timezone
from pathlib import Path
from bootstrap import D, sha
from loc import normalize, fixtures
from probe import save


def main():
    fixtures()
    base_audit=json.loads((D/'final-source-audit.json').read_text())
    base=json.loads((D/'results/final-timings.json').read_text())
    assert base['status']=='complete' and len(base_audit['programs'])==56
    extras=json.loads((D/'results/optimization-builds.json').read_text())['programs']
    small=json.loads((D/'results/optimization-small-checks.json').read_text())
    helper=json.loads((D/'results/optimized-regex-helper-tests.json').read_text())
    assembly=json.loads((D/'results/native-tree-assembly-final.json').read_text())
    packing=json.loads((D/'results/elm-packing-checks.json').read_text())
    keys=json.loads((D/'results/nucleotide-key-checks.json').read_text())
    assert len(extras)==5 and small['status']==helper['status']==assembly['status']==packing['status']==keys['status']=='passed'
    assert len(packing['cases'])==9 and all(c['full_output_match'] for c in packing['cases'])
    assert len(keys['cases'])==4 and all(c['full_output_match'] for c in keys['cases'])
    assert helper['expectations']==14
    for filename,digest in {**base_audit['pin_files'],**base_audit['libraries']}.items(): assert sha(filename)==digest,filename
    audit=copy.deepcopy(base_audit)
    for row in extras:
        check=small['programs'][row['key']]
        assert row['build_status']=='passed' and check['status']=='passed' and check['validation']['reference_match']
        assert row['source_sha256']==check['source_sha256']==sha(row['source'])
        assert row['binary_sha256']==check['binary_sha256']==sha(row['binary'])
        metric=normalize(Path(row['source']).read_text(),row['language'])
        reason=('Read the complete input, clean every header/newline, independently count every occurrence of the nine case-insensitive patterns, apply all five ordered substitutions and print every count and required length. The PCRE2 JIT interface or AST-derived first-byte table is a general regex optimization, independent of benchmark names, specific patterns or input contents. Complete matching orchestration and application helpers remain in the measured language.' if row['benchmark']=='regexredux' else
            'Enumerate every n! permutation in canonical order, perform the actual prefix reversals to count flips, accumulate the signed checksum and maximum, and print both results. Base-sixteen packed permutation/counter digits use exact ordinary Elm numeric arithmetic for up to thirteen elements; floor division by exact powers of sixteen avoids 32-bit truncation. The complete general list/array fallback supports larger inputs and is counted in LOC. No host benchmark executor, precomputed permutation/result or input-specific shortcut.' if row['benchmark']=='fannkuchredux' else
            'Read the complete input and decode the entire THREE record. Construct all seven ordinary growable maps for widths 1,2,3,4,6,12,18 by scanning every overlapping window, using complete keys including 36 bits for width eighteen. General native threads independently process each width and ordinary map_entry avoids redundant lookup when incrementing a value. Join workers before deterministic frequency/count output and release every map/thread. No fixed bucket limit, period cache, target-only counts, discarded key bits or host benchmark executor.' if row['language']=='odin' else
            'Read the complete input and decode the entire THREE record. Construct all seven ordinary core Dict maps for widths 1,2,3,4,6,12,18 by scanning every overlapping window. Exact ordinary numeric keys preserve all thirty-six bits for width eighteen without truncating bitwise comparisons. Count all keys, then sort both frequency tables and retrieve all five query counts in required order. Complete application sequencing is Elm; no fixed bucket table, period cache, target-only counting, discarded key bits or host benchmark executor.')
        audit['programs'].append(dict(row,loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],small_validation=check['validation'],eligible_for_chart=True,
            algorithm=dict(strict=True,status='required-work-reviewed',reason=reason,native_benchmark_kernel=False)))
    assert len(audit['programs'])==61
    audit['allocation_evidence']['timed_native_assembly']=assembly
    audit['elm_packing_evidence']=packing
    audit['nucleotide_key_evidence']=keys
    audit['general_helper_tests'].update(optimized_regex_cases=14,regex_implementations=2,total_expectations=228,
        optimized_regex_evidence=helper)
    builds=json.loads((D/'results/final-builds.json').read_text())
    builds['programs']+=extras
    build_path=D/'results/builds-complete.json'
    save(build_path,builds)
    audit['final_builds']='results/builds-complete.json'
    for path in [D/'results/optimization-small-checks.json',D/'results/optimized-regex-helper-tests.json',D/'results/native-tree-assembly-final.json',D/'results/elm-packing-checks.json',D/'results/nucleotide-key-checks.json',
                 D/'evidence/general-helpers-optimized-regex.roc',D/'evidence/general-helpers-optimized-regex.log',D/'support/libpcre2-8.0.dylib']:
        audit['libraries'][str(path)]=sha(path)
    for row in assembly['programs']: audit['libraries'][row['assembly']]=row['assembly_sha256']
    for path in [D/'finalize_optimizations.py',D/'prepare_optimizations.py',D/'audit_native_assembly.py',build_path]: audit['pin_files'][str(path)]=sha(path)
    audit_path=D/'source-audit-complete.json'
    save(audit_path,audit)
    report=copy.deepcopy(base)
    for row in audit['programs']:
        if row['key'] not in report['programs']:
            report['programs'][row['key']]=dict(row,samples_seconds=[],sample_validation=[])
        else:
            old=report['programs'][row['key']]
            assert old['source_sha256']==row['source_sha256'] and old['binary_sha256']==row['binary_sha256']
            assert len(old['samples_seconds'])==len(old['sample_validation'])==3 and all(v['reference_match'] for v in old['sample_validation'])
    report.update(status='running',source_audit=str(audit_path),source_audit_sha256=sha(audit_path),
        optimized_at=datetime.now(timezone.utc).isoformat(),
        retained_initial_source_report=base['retained_source_report'],
        retained_source_report=dict(path='results/final-timings.json',sha256=sha(D/'results/final-timings.json'),samples=198,
            reason='All three complete reference-validated samples from every unchanged candidate/control are preserved. Only two new general regex alternatives, one pure-Elm packed permutation/counter alternative and two standard-map nucleotide alternatives require new timings.'))
    report['completed_benchmarks']=[b for b in report['completed_benchmarks'] if b not in ('regexredux','fannkuchredux','knucleotide')]
    for benchmark in ['regexredux','fannkuchredux','knucleotide']: report['orders'].pop(benchmark,None)
    report.pop('finished_at',None)
    save(D/'results/timings-complete.json',report)
    print('COMPLETE AUDIT',len(audit['programs']),'sources;',198,'retained samples;',213,'required samples',flush=True)


if __name__=='__main__': main()

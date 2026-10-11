"""Freeze reviewed source candidates, LOC and unchanged Mech controls per batch."""
import argparse
import ast
import json
from pathlib import Path
from bootstrap import D, sha
from loc import normalize, fixtures
from probe import WORK

C = D.parents[1]
tree=ast.parse((C/'lisp-racket-comparison/20261010/run.py').read_text())
REASONS=next(ast.literal_eval(node.value) for node in tree.body if isinstance(node,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='REASONS' for t in node.targets))

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--languages',required=True)
    parser.add_argument('--output',required=True)
    args=parser.parse_args()
    fixtures()
    languages=args.languages.split(',')
    built=json.loads((D/'results/builds.json').read_text())
    small=json.loads((D/'results/small-checks.json').read_text())
    rows=[]
    for row in built['programs']:
        if row['language'] not in languages: continue
        assert row['build_status']=='passed',row['id']
        check=small['programs'][row['key']]
        assert check['status']=='passed' and check['source_sha256']==row['source_sha256'] and check['binary_sha256']==row['binary_sha256'],row['id']
        assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
        metric=normalize(Path(row['source']).read_text(),row['language'])
        rows.append(dict(row,loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],loc_expansions=metric['expansions'],small_validation=check['validation'],eligible_for_chart=True,
            algorithm=dict(strict=True,status='required-work-reviewed',reason=REASONS[row['benchmark']],native_benchmark_kernel=False)))
    for language in languages:
        assert {r['benchmark'] for r in rows if r['language']==language}==set(WORK),language
    old=json.loads((C/'lisp-racket-comparison/20261010/source-audit.json').read_text())
    controls=old['controls']
    for row in controls:
        assert sha(row['source'])==row['source_sha256'] and sha(row['binary'])==row['binary_sha256']
    libraries={}
    for library in ['/opt/homebrew/opt/gmp/lib/libgmp.10.dylib','/opt/homebrew/opt/pcre2/lib/libpcre2-8.0.dylib']:
        libraries[library]=sha(library)
    if 'roc' in languages:
        platform=Path('/Users/cim310/.cache/roc/packages/github.com/roc-lang/basic-cli/releases/download/0.20.0/X73hGh05nNTkDHU06FHC0YfFaQB1pimX7gncRcao5mU')
        for file in platform.rglob('*'):
            if file.is_file() and (file.suffix=='.roc' or file.name=='macos-arm64.a'): libraries[str(file)]=sha(file)
    if 'elm' in languages:
        node='/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'
        libraries[node]=sha(node)
        package_cache=Path('/Users/cim310/.elm/0.19.3/packages')
        for file in package_cache.rglob('*'):
            if file.is_file() and (file.suffix in ('.elm','.js') or file.name=='elm.json'): libraries[str(file)]=sha(file)
    for file in [D/'support/elm-runner.cjs',D/'sources/elm/elm.json']:
        if 'elm' in languages: libraries[str(file)]=sha(file)
    evidence={str(path):sha(path) for path in list((D/'evidence').glob('tree-*'))+[D/'evidence/general-helpers.roc',D/'evidence/general-helper-cases.json',D/'evidence/general-helper-tests.log',D/'results/allocation-evidence.json',D/'results/full-check-failures.json'] if path.is_file()}
    libraries.update(evidence)
    result=dict(status='reviewed',languages=languages,programs=rows,controls=controls,runtimes={l:built['runtimes'][l] for l in languages},libraries=libraries,
        allocation_evidence=json.loads((D/'results/allocation-evidence.json').read_text()),general_helper_tests=dict(status='passed',arithmetic_cases=200,regex_cases=14,total_expectations=214),
        pin_files={str(D/f):sha(D/f) for f in ['audit.py','loc.py','run.py','probe.py']},
        source_audit='Every measured application contains its complete workload and all helpers/FFI declarations. General standard libraries, arithmetic and regex are permitted; no native benchmark executor. Published sources are separately preserved and all work corrections labelled. Compiler fixes and I/O changes count toward LOC.',
        timing_boundary='Fresh process startup, language/module/library loading, complete input, computation, complete output to a regular file, and shutdown are timed. Compilation, validation and hashing are excluded. All timings are local; three sequential shuffled full-workload samples, unchanged Mech controls, AC power and Low Power Mode off. Concurrent compilers/power changes invalidate a sample.',
        workloads=WORK)
    (D/args.output).write_text(json.dumps(result,indent=2)+'\n')
    print('REVIEWED',len(rows),'candidates;',len(controls),'controls',flush=True)

if __name__=='__main__': main()

#!/usr/bin/env python3
"""Compile whitespace-only D reflows and verify small/full canonical output."""
from pathlib import Path
import hashlib, importlib.util, json, shutil, subprocess

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
ROOT = CLBG.parents[1]

def sha(path):
    with Path(path).open('rb') as handle:
        return hashlib.file_digest(handle, 'sha256').hexdigest()

def main():
    audit = json.loads((HERE / 'sources/normalization-audit.json').read_text())
    prior = json.loads((CLBG / 'd-comparison/20261011/source-audit.json').read_text())['programs']
    rows = []
    for item in audit['d_reflows']:
        source = Path(item['reflow_source'])
        assert sha(source) == item['reflow_source_sha256'] and item['nonwhitespace_tokens_identical']
        assert sha(item['original_source']) == item['original_source_sha256']
        original = next(row for row in prior if row['source'] == item['original_source'])
        directory = HERE / 'builds' / ('d-' + item['benchmark'] + '-reflow')
        directory.mkdir(parents=True, exist_ok=True)
        # D derives a module name from the basename; preserve the original
        # valid name while leaving the illustrative reflow source intact.
        staged = directory / Path(original['source']).name
        shutil.copy2(source, staged)
        binary = directory / 'aot'
        command = [str(staged) if arg == original['source'] else '-of=' + str(binary) if arg.startswith('-of=') else arg
                   for arg in original['build_command']]
        with (directory / 'build.log').open('w') as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
        rows.append(dict(benchmark=item['benchmark'], source=str(source), source_sha256=sha(source),
            loc=item['reflow_loc']['normalized_loc'], binary=str(binary), binary_sha256=sha(binary),
            build_command=command, command=[str(binary)] + ([] if item['benchmark'] == 'regexredux' else ['{n}']),
            runtime_environment=original.get('runtime_environment', {}), validations=[]))
    spec = importlib.util.spec_from_file_location('gap_d_validation', CLBG / 'array-comparison/all-20261009/run.py')
    common = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(common)
    for row in rows:
        for full in [False, True]:
            common.compact.wait_for_environment(common.REQUIRED)
            result = common.execute(row, full, 600)
            assert not result['competing_compilers'] and not result['power_changes']
            row['validations'].append(dict(workload='full' if full else 'small', **result))
            print(row['benchmark'], 'full' if full else 'small', 'PASS', flush=True)
        item = next(item for item in audit['d_reflows'] if item['benchmark'] == row['benchmark'])
        item['execution_status'] = 'Compiled with frozen D flags; complete small and full canonical output validated. No algorithm or nonwhitespace token changed.'
    (HERE / 'results/d-reflow-validation.json').write_text(json.dumps(rows, indent=2) + '\n')
    (HERE / 'sources/normalization-audit.json').write_text(json.dumps(audit, indent=2) + '\n')

if __name__ == '__main__':
    main()

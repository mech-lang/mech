"""Validate complete outputs on small canonical inputs before full measurements."""
import argparse
import importlib.util
import json
from pathlib import Path
from bootstrap import D, sha

C = D.parents[1]
spec = importlib.util.spec_from_file_location('array_runner', C / 'array-comparison/all-20261009/run.py')
arrays = importlib.util.module_from_spec(spec)
spec.loader.exec_module(arrays)
compact = arrays.compact
compact.COMPILERS.update(['zig','odin','roc','elm'])
WORK, REQUIRED = arrays.WORK, arrays.REQUIRED

def execute(row, full, timeout=14400):
    if row['benchmark'] == 'nbody':
        return compact.execute(row, full, timeout, watch_compilers=True, expected_power=REQUIRED)
    return arrays.execute(row, full, timeout)

def save(path, value):
    temp = path.with_suffix('.json.tmp')
    temp.write_text(json.dumps(value,indent=2)+'\n')
    temp.replace(path)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--languages',default='zig,odin,roc,elm')
    parser.add_argument('--benchmarks',default=','.join(WORK))
    args = parser.parse_args()
    built = json.loads((D / 'results/builds.json').read_text())
    path = D / 'results/small-checks.json'
    report = json.loads(path.read_text()) if path.exists() else dict(status='running',programs={})
    for row in built['programs']:
        if row['build_status'] != 'passed' or row['language'] not in args.languages.split(',') or row['benchmark'] not in args.benchmarks.split(','): continue
        previous = report['programs'].get(row['key'])
        if previous and previous['status'] == 'passed' and previous['source_sha256'] == row['source_sha256'] and previous['binary_sha256'] == row['binary_sha256']:
            print('REUSE CHECK',row['id'],flush=True)
            continue
        compact.wait_for_environment(REQUIRED)
        try:
            assert sha(row['source']) == row['source_sha256'] and sha(row['binary']) == row['binary_sha256']
            result = execute(row,False,timeout=600)
            assert not result['competing_compilers'] and not result['power_changes']
            report['programs'][row['key']] = dict(status='passed',source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],**result)
            print('PASS',row['id'],f"{result['seconds']:.6f}s",flush=True)
        except Exception as error:
            report['programs'][row['key']] = dict(status='failed',source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],error=str(error))
            print('FAIL',row['id'],str(error)[:2500],flush=True)
        save(path,report)
    report['status'] = 'passed' if all(report['programs'].get(r['key'],{}).get('status') == 'passed' for r in built['programs'] if r['build_status'] == 'passed') else 'partial'
    save(path,report)

if __name__ == '__main__': main()

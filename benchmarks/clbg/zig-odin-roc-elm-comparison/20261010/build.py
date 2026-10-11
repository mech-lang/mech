"""Compile source candidates before running any full timing cohort."""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
from bootstrap import D, sha

NODE = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node'

def build(row):
    row = row.copy()
    runtime = json.loads((D / 'bootstrap.json').read_text())['runtimes'][row['language']]
    source = Path(row['source'])
    binary = D / 'binaries' / row['id']
    if row['language'] == 'zig':
        cmd = [runtime['path'], 'build-exe', str(source), '-O', 'ReleaseFast', '-mcpu=native', '-fllvm', '-lc', '-femit-bin=' + str(binary)]
        if row['benchmark'] == 'regexredux': cmd += ['-I/opt/homebrew/opt/pcre2/include', '-L/opt/homebrew/opt/pcre2/lib', '-lpcre2-8']
        if row.get('uses_gmp'): cmd += ['-I/opt/homebrew/opt/gmp/include', '-L/opt/homebrew/opt/gmp/lib', '-lgmp']
    elif row['language'] == 'odin':
        cmd = [runtime['path'], 'build', str(source), '-file', '-o:speed', '-microarch:native', '-no-bounds-check', '-out:' + str(binary), '-thread-count:2']
        if 'gmp' in row.get('libraries',[]): cmd += ['-collection:external=/opt/homebrew/opt/gmp/lib']
    elif row['language'] == 'roc':
        cmd = [runtime['path'], 'build', str(source), '--optimize', '--output', str(binary)]
    else:
        binary = binary.with_suffix('.js')
        cmd = [runtime['path'], 'make', str(source), '--optimize', '--output=' + str(binary)]
    digest = sha(source)
    previous_path = D / 'results/builds.json'
    previous = json.loads(previous_path.read_text()) if previous_path.exists() else {'programs': []}
    prior = next((p for p in previous['programs'] if p['key'] == row['key'] and p['source_sha256'] == digest and p['build_command'] == cmd and p['build_status'] == 'passed'), None)
    if prior and binary.exists() and sha(binary) == prior['binary_sha256']:
        print('REUSE BUILD', row['id'], flush=True)
        return prior
    env = dict(os.environ)
    env['PATH'] = '/opt/homebrew/bin:' + str(Path(NODE).parent) + ':' + env['PATH']
    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=D / 'sources' / 'elm' if row['language'] == 'elm' else D, env=env)
    log = D / 'results' / (row['id'] + '.build.log')
    log.write_text(result.stdout)
    row.update(source_sha256=digest, build_command=cmd, build_status='passed' if result.returncode == 0 else 'failed', build_log=str(log), compiler_sha256=runtime['sha256'])
    if result.returncode == 0:
        row.update(binary=str(binary), binary_sha256=sha(binary))
        args = [] if row['benchmark'] in ['knucleotide', 'revcomp', 'regexredux'] else ['{n}']
        row['command'] = ([NODE, str(D / 'support/elm-runner.cjs'), str(binary)] if row['language'] == 'elm' else [str(binary)]) + args
    print('BUILD', row['id'], row['build_status'], flush=True)
    if result.returncode: print(result.stdout[:2200], flush=True)
    return row

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--languages', default='zig,odin,roc,elm')
    parser.add_argument('--jobs', type=int, default=2)
    args = parser.parse_args()
    sources = []
    for path in [D / 'published.json', *sorted(D.glob('written*.json'))]:
        if path.exists(): sources += json.loads(path.read_text())['programs']
    prior = json.loads((D / 'results/builds.json').read_text()) if (D / 'results/builds.json').exists() else dict(programs=[])
    selected = [r for r in sources if r['language'] in args.languages.split(',')]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        rows = list(pool.map(build, selected))
    selected_keys = {r['key'] for r in rows}
    rows += [r for r in prior['programs'] if r['key'] not in selected_keys]
    runtime = json.loads((D / 'bootstrap.json').read_text())['runtimes']
    (D / 'results/builds.json').write_text(json.dumps(dict(status='built', runtimes=runtime, programs=rows), indent=2) + '\n')
    print('BUILT', sum(r['build_status'] == 'passed' for r in rows), 'of', len(rows), flush=True)

if __name__ == '__main__': main()

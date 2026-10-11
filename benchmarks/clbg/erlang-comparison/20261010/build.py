"""Compile all pinned Erlang candidates, audit LOC, and pin the local BEAM JIT."""
import concurrent.futures
import json
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from pygments import lex
from pygments.lexers import get_lexer_by_name
from pygments.token import Comment
from prepare import sha

D = Path(__file__).resolve().parent
ERL = Path('/opt/homebrew/opt/erlang/bin/erl').resolve()
ERLC = Path('/opt/homebrew/opt/erlang/bin/erlc').resolve()


def loc(source):
    text = source.read_text()
    clean = ''.join('\n' * value.count('\n') if token in Comment else value
        for token, value in lex(text, get_lexer_by_name('erlang', stripnl=False, ensurenl=False)))
    physical = sum(bool(line.strip()) for line in clean.splitlines())
    output = subprocess.check_output([str(ERL), '+S', '1:1', '-noshell', '-pa', str(D / 'support'),
        '-run', 'loc_probe', 'main', str(source)], text=True)
    packed = json.loads(output)
    expansions = [dict(line=r['line'], extra=r['extra'], source=text.splitlines()[r['line'] - 1],
        reason='Multiple independent expressions on the same physical line in an Erlang clause or block; structural tuple/list/argument separators excluded.') for r in packed]
    return dict(physical_loc=physical, normalized_loc=physical + sum(r['extra'] for r in expansions), expansions=expansions,
        metric='statement-normalized', policy='Nonblank/noncomment LOC plus packed independent expressions. Semantic module/export/compile/macro directives and all application helpers count; structural data, argument and guard separators do not. No compressed-size metric.')


def build(row):
    row = row.copy()
    source = Path(row['source'])
    module = re.search(r'-module\((\w+)\)', source.read_text())[1]
    directory = D / 'builds' / row['id']
    directory.mkdir(parents=True, exist_ok=True)
    copied = directory / (module + '.erl')
    copied.write_bytes(source.read_bytes())
    command = [str(ERLC), '-o', str(directory), str(copied)]
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=directory)
    log = D / 'results' / (row['id'] + '.build.log')
    log.write_text(result.stdout)
    row.update(module=module, build_command=command, build_status='passed' if result.returncode == 0 else 'failed',
        build_log=str(log), loc_audit=loc(source))
    if result.returncode == 0:
        beam = directory / (module + '.beam')
        row.update(binary=str(beam), binary_sha256=sha(beam),
            command=[str(ERL), '+S', '8:8', '-noshell'] + (['-noinput'] if row['benchmark'] == 'regexredux' else []) + ['-pa', str(directory), '-run', module, 'main',
                '0' if row['benchmark'] in ['knucleotide', 'regexredux', 'revcomp'] else '{n}'])
        assembly_command = [str(ERLC), '-S', '-o', str(directory), str(copied)]
        subprocess.run(assembly_command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=directory, check=True)
        assembly = directory / (module + '.S')
        row.update(assembly=str(assembly), assembly_sha256=sha(assembly))
    print('BUILD', row['id'], row['build_status'], row['loc_audit']['normalized_loc'], 'LOC', flush=True)
    return row


def main():
    subprocess.run([str(ERLC), '-o', str(D / 'support'), str(D / 'support/loc_probe.erl')], check=True)
    manifest = json.loads((D / 'manifest.json').read_text())
    corrections = json.loads((D / 'corrections.json').read_text())
    assert corrections['manifest_sha256'] == sha(D / 'manifest.json')
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        programs = list(pool.map(build, manifest['programs'] + corrections['programs']))
    root = Path('/opt/homebrew/opt/erlang/lib/erlang').resolve()
    engine = next(root.glob('erts-*/bin/beam.smp'))
    release = next(root.glob('releases/*/OTP_VERSION')).read_text().strip()
    expression = 'io:put_chars(json:encode(#{otp=>list_to_binary(erlang:system_info(otp_release)),erts=>list_to_binary(erlang:system_info(version)),jit=>erlang:system_info(emu_flavor),arch=>list_to_binary(erlang:system_info(system_architecture)),schedulers=>erlang:system_info(schedulers_online)})),halt().'
    details = json.loads(subprocess.check_output([str(ERL), '+S', '8:8', '-noshell', '-eval', expression], text=True))
    assert details['jit'] == 'jit' and details['schedulers'] == 8
    runtimes = {name: dict(path=str(path), sha256=sha(path)) for name, path in [('launcher', ERL), ('compiler', ERLC), ('engine', engine)]}
    report = dict(status='built', created_at=datetime.now(timezone.utc).isoformat(), programs=programs,
        manifest_sha256=sha(D / 'manifest.json'), corrections_sha256=sha(D / 'corrections.json'),
        runtimes=runtimes, runtime_details=dict(details, release=release),
        runtime_policy='Erlang/OTP ' + release + ' on native Apple Silicon, BEAM load-time JIT enabled; eight online schedulers, published worker partitioning retained, default garbage collection and heap settings.',
        timing_boundary='Fresh-process wall time includes BEAM initialization, module loading and load-time JIT, complete input, all computation, complete output and process shutdown. erlc bytecode compilation, setup and output validation are excluded.')
    (D / 'results/builds.json').write_text(json.dumps(report, indent=2) + '\n')
    print('BUILT', sum(r['build_status'] == 'passed' for r in programs), 'of', len(programs))


if __name__ == '__main__':
    main()

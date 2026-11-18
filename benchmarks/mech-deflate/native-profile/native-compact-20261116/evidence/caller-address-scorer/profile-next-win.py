"""Profile the two nearest lost wins with callback-instrumented native code."""
import contextlib
import os
import subprocess

from emit import HERE, Q, SCORER, read, save, sha, symbols

ROLE = 'caller-addresses-refined'
NAMES = ('prefix-natural-4096', 'prefix-natural-8192', 'prefix-natural-16384', '320-overlap')
MATCHER = '_mech_compact_numeric_475_vector'
REFERENCE = Q / 'profile-root-phase-calls.py'


def main():
    source = HERE / ROLE
    recorded = read(source / 'receipt.json')
    bound = read(source / 'bound/runtime.json')
    profile = source / 'phase-attribution'
    profile.mkdir()
    frontend = profile / 'frontend'
    compiler, input_file, _, mode = recorded['compile']
    assert sha(compiler) == recorded['compiler_sha256']
    assert sha(input_file) == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    flags = recorded['flags'] | {'MECH_COMPACT_PROFILE_PHASE_CALLS': '1'}
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')}
    command = [compiler, input_file, str(frontend), mode]
    with (profile / 'emit.log').open('w') as log:
        subprocess.run(command, check=True, stdout=log, stderr=subprocess.STDOUT, env=env | flags)
    assert sha(frontend / 'program.mecb') == recorded['bytecode_sha256']
    native = frontend / 'program.o'
    old = source / 'bound/program.o'
    assert sha(old) == bound['object_sha256']
    object_summary = symbols(native)
    comparisons = {}
    for name in (SCORER, MATCHER):
        prior = recorded['layout']['symbols'][name]
        changed = object_summary['symbols'][name]
        comparisons[name] = {'normal': prior, 'instrumented': changed,
                             'matches': all(prior[field] == changed[field]
                                            for field in ('sha256', 'relocations', 'extent'))}
        assert comparisons[name]['matches'], name
    link = []
    for argument in bound['link']:
        if argument == f'link-arg={old}':
            link.append(f'link-arg={native}')
        elif argument == str(source / 'bound/mech-worker'):
            link.append(str(profile / 'mech-worker'))
        else:
            link.append(argument)
    assert f'link-arg={native}' in link and str(profile / 'mech-worker') in link
    save(profile / 'runtime.json', {'link': link})
    (profile / 'metadata.json').write_bytes((frontend / 'metadata.json').read_bytes())

    recipe = REFERENCE.read_text()
    start = recipe.index('runtime = json.loads')
    start_cases = recipe.index('names = [')
    end_cases = recipe.index('samples = ', start_cases)
    code = recipe[:recipe.index('qualification = ')] + recipe[start:start_cases] + (
        f'names = {list(NAMES)!r}\n') + recipe[end_cases:]
    with (profile / 'phase-profile.log').open('w') as log, contextlib.redirect_stdout(log):
        exec(compile(code, str(REFERENCE), 'exec'), {'profile': profile, 'qualification': Q})
    result = read(profile / 'phase-timings.json')
    observations = []
    for row in result['cases']:
        preceding = {key: row[key] for key in ('case', 'median_instrumented_ns')}
        phases = row['phases']
        preceding.update(scorer=phases['3009'], matcher=phases.get('475'))
        observations.append(preceding)
    report = {'source_pin': recorded['source_pin'], 'instrumented_object_sha256': sha(native),
              'instrumented_metadata_sha256': sha(frontend / 'metadata.json'),
              'bound_original_object_sha256': sha(old), 'compiler_sha256': sha(compiler),
              'profiler_sha256': sha(REFERENCE), 'hot_atom_bindings': comparisons,
              'compile': command, 'flags': flags, 'as_run_fixture_sha256': sha(Q / 'fixtures.json'),
              'runtime_receipt_sha256': sha(profile / 'runtime.json'),
              'instrumented_worker_sha256': sha(profile / 'mech-worker'),
              'trace_sha256': sha(profile / 'phase-timings.json'), 'derived': observations}
    save(profile / 'result.json', report)
    print(observations, flush=True)


if __name__ == '__main__':
    main()

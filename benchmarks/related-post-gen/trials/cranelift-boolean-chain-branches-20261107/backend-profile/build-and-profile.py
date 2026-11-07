from pathlib import Path
import bisect, collections, hashlib, importlib.util, json, os, shutil, struct, subprocess, time

P = Path(__file__).parent
R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
OLD = Path('/private/tmp/mech-related-native-adaptive-histogram-schema-reuse-pc-profile-20261105-v2')
SAMPLER = R / 'results/native-u32-arithmetic-20261106/native-pc-profile/sampler.c'
BUNDLES = {
    'cranelift': Path('/private/tmp/mech-related-llvm-native-image-cranelift-control-20261107-v4'),
    'llvm': Path('/private/tmp/mech-related-llvm-native-image-smoke-20261107-v4/reuse-4'),
}
POSTS = Path('/private/tmp/mech-related-current-input-fixtures-20261107/posts.json')
ORACLE = Path('/private/tmp/mech-related-frozen-native-composition-latency-20261105-v1/related_posts_rust.json')
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def write(p, d): p.write_text(json.dumps(d, indent=2) + '\n')
def run(cmd, receipt, timeout=180):
    cmd = list(map(str, cmd)); start = time.monotonic()
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env={**os.environ, 'RUSTC_BOOTSTRAP':'1'})
    d = dict(command=cmd, exit=result.returncode, seconds=time.monotonic()-start,
             stdout=result.stdout, stderr=result.stderr, diagnostic_timings_excluded=True)
    write(receipt, d)
    assert result.returncode == 0, result.stdout[-3000:] + result.stderr[-6000:]
    print(receipt.name, 'passed', flush=True)
    return d

spec = importlib.util.spec_from_file_location('audit', R/'audit-native-link.py')
audit = importlib.util.module_from_spec(spec); spec.loader.exec_module(audit)
shutil.copy2(SAMPLER, P/'sampler.c')
run(['cc', '-O3', '-c', P/'sampler.c', '-o', P/'sampler.o'], P/'sampler-build.json')
source = (OLD/'runner.rs').read_text()
assert source.count('    unsafe { mpc_enable(1) };') == 1
source = source.replace('    unsafe { mpc_enable(1) };\n', '')
needle = '        let report = program.evaluate_into(&input, &mut output, mode.execution(), &memory).unwrap();'
assert source.count(needle) == 1
source = source.replace(needle, '        unsafe { mpc_enable(1) };\n' + needle + '\n        unsafe { mpc_enable(0) };')
for label, bundle in BUNDLES.items():
    p = P/label; p.mkdir(exist_ok=True)
    if not (p/'build').exists(): (p/'build').symlink_to(bundle/'build', target_is_directory=True)
    reference = (bundle/'runner.rs').read_text()
    assert source[:source.index('fn run()')] == reference[:reference.index('fn run()')]
    (p/'runner.rs').write_text(source)
    original = json.loads((bundle/'runner-aot.build.json').read_text())
    command = [str(p/'runner.rs') if x == str(bundle/'runner.rs') else str(p/'runner-pc') if x == str(bundle/'runner-aot') else x for x in original['command']]
    assert str(p/'runner.rs') in command and str(p/'runner-pc') in command
    command[1:1] = ['-C', 'link-arg='+str(P/'sampler.o')]
    run(command, p/'build.json')
    run(['python3', R/'audit-native-link.py', '--object', p/'build/object.o', '--executable', p/'runner-pc', '--output', p/'native-link-proof.json'], p/'link-audit.json')

expected = json.loads(ORACLE.read_text())
rows = []
for repeat, order in enumerate([['cranelift', 'llvm'], ['llvm', 'cranelift']]):
    for label in order:
        p = P/label; output = p/f'output-{repeat}.json'
        record = run([p/'runner-pc', p/'build/source.mec', POSTS, output, '1000'], p/f'run-{repeat}.json', timeout=90)
        result = json.loads(record['stdout'].strip())
        assert result['completed_iterations'] == 1000 and result['all_iterations_indices_exact']
        assert json.loads(output.read_text()) == expected
        stats = json.loads(Path(str(output)+'.pc.json').read_text())
        assert stats['suspended'] == stats['resumed']+stats['resume_terminated'] and stats['full'] == 0 and stats['records'] > 1000
        _, sections, symbols = audit.macho(p/'runner-pc')
        section = next(s for s in sections if s['name'] == '__text')
        symbols = sorted([s for s in symbols if s['section'] and sections[s['section']-1]['name'] == '__text'], key=lambda s:s['value'])
        starts = [s['value'] for s in symbols]; counts = collections.Counter(); pcs = collections.defaultdict(collections.Counter)
        for ns, thread, pc, lr, sp, fp in struct.iter_unpack('<6Q', Path(str(output)+'.pc.bin').read_bytes()):
            address = pc-stats['image_base']+0x100000000
            if section['address'] <= address < section['address']+section['size']:
                i = bisect.bisect_right(starts, address)-1
                if i >= 0:
                    symbol = symbols[i]; counts[symbol['name']] += 1
                    if symbol['name'].startswith('_mech_canonical_program_entry_v1'): pcs[symbol['name']][address-symbol['value']] += 1
        row = dict(variant=label, repeat=repeat, executable_sha256=sha(p/'runner-pc'), Object_sha256=sha(p/'build/object.o'), source_sha256=sha(p/'build/source.mec'),
                   all1000_index_arrays_and_final_rust_output_exact=True, sampler=stats,
                   top_symbol_observations=counts.most_common(80), native_function_observations={k:sum(v.values()) for k,v in pcs.items()},
                   native_pc_offsets={k:dict(v) for k,v in pcs.items()})
        write(p/f'summary-{repeat}.json', row); rows.append(row)
        print(label, repeat, stats['records'], 'register observations; 1000 exact invocations', flush=True)
write(P/'execution-profile.json', dict(schema='mech.related-post-gen.matched-backend-execution-profile.v1', rows=rows,
      source_sha256=sha(POSTS), all4000_invocations_exact=True,
      limitations=['Sampling is enabled only around evaluate_into, excluding restoration, JSON, input construction and materialization.',
                   'Own-process periodic register observations are not executed instruction counts or CPU-time shares.',
                   'No thread suspension and no stack walk. Scheduler and idle/join observations may occur.',
                   'All timings from these diagnostic binaries are excluded from benchmark comparisons.']))

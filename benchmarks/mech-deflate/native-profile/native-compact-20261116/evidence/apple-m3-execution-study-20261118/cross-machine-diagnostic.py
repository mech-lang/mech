"""Counter and stack diagnostic, serial workers; not a qualification timing run."""
from pathlib import Path
from datetime import datetime, timezone
from hashlib import sha256
import argparse, ctypes, json, os, statistics, subprocess, time

p = argparse.ArgumentParser()
p.add_argument('--fixtures', required=True)
p.add_argument('--mech', required=True)
p.add_argument('--metadata', required=True)
p.add_argument('--roc', required=True)
p.add_argument('--output', required=True)
p.add_argument('--counter-blocks', type=int, default=12)
a = p.parse_args()
out = Path(a.output)
out.mkdir()

names = ('user_time system_time pkg_idle_wkups interrupt_wkups pageins wired_size '
         'resident_size phys_footprint proc_start_abstime proc_exit_abstime '
         'child_user_time child_system_time child_pkg_idle_wkups child_interrupt_wkups '
         'child_pageins child_elapsed_abstime diskio_bytesread diskio_byteswritten '
         'cpu_time_qos_default cpu_time_qos_maintenance cpu_time_qos_background '
         'cpu_time_qos_utility cpu_time_qos_legacy cpu_time_qos_user_initiated '
         'cpu_time_qos_user_interactive billed_system_time serviced_system_time '
         'logical_writes lifetime_max_phys_footprint instructions cycles billed_energy '
         'serviced_energy interval_max_phys_footprint runnable_time').split()

class Usage(ctypes.Structure):
    _fields_ = [('uuid', ctypes.c_ubyte * 16)] + [(n, ctypes.c_uint64) for n in names]

class Timebase(ctypes.Structure):
    _fields_ = [('numer', ctypes.c_uint32), ('denom', ctypes.c_uint32)]

lib = ctypes.CDLL('/usr/lib/libproc.dylib', use_errno=True)
lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
lib.proc_pid_rusage.restype = ctypes.c_int
system = ctypes.CDLL('/usr/lib/libSystem.B.dylib')
tb = Timebase()
assert system.mach_timebase_info(ctypes.byref(tb)) == 0

def usage(pid):
    value = Usage()
    assert lib.proc_pid_rusage(pid, 4, ctypes.byref(value)) == 0, ctypes.get_errno()
    return {n: getattr(value, n) for n in names}

def run(command):
    return subprocess.check_output(command, text=True).strip()

def quiet():
    text = run(['/bin/ps', '-axo', 'pid,ppid,%cpu,etime,comm', '-r'])
    busy = []
    for line in text.splitlines()[1:]:
        fields = line.split(None, 4)
        if len(fields) == 5 and float(fields[2]) > 40 and int(fields[0]) != os.getpid():
            busy.append(line)
    return {'at': datetime.now(timezone.utc).isoformat(), 'processes': text, 'busy': busy}

preflight = []
for _ in range(2):
    preflight.append(quiet())
    time.sleep(0.25)
(out / 'preflight.json').write_text(json.dumps(preflight, indent=2) + '\n')
assert not any(x['busy'] for x in preflight), 'Unrelated CPU load; no measurement started'

fixtures = json.loads(Path(a.fixtures).read_text())
cases = ['prefix-natural-65536', 'prefix-natural-8192', 'random-1024']
commands = {'mech': [a.mech, a.metadata, a.fixtures],
            'roc': [a.roc, '/unused-metadata-argument', a.fixtures]}
plan = {'started_at': datetime.now(timezone.utc).isoformat(),
        'boundary': 'whole W command: repeated compressions plus independent output validation and loop/pipe overhead; not isolated native-call counts',
        'sample_boundary': 'W loop; compression and validation are separable by stack',
        'cpu': run(['/usr/sbin/sysctl', '-n', 'machdep.cpu.brand_string']),
        'os': run(['/usr/bin/sw_vers']), 'power': run(['/usr/bin/pmset', '-g', 'batt']),
        'thermal': run(['/usr/bin/pmset', '-g', 'therm']),
        'timebase': {'numer': tb.numer, 'denom': tb.denom},
        'cases': cases, 'counter_blocks': a.counter_blocks,
        'commands': commands,
        'sha256': {k: sha256(Path(v).read_bytes()).hexdigest() for k, v in
                   [('mech', a.mech), ('metadata', a.metadata), ('roc', a.roc), ('fixtures', a.fixtures)]}}
(out / 'plan.json').write_text(json.dumps(plan, indent=2) + '\n')
records = []
for case in cases:
    case_dir = out / case
    case_dir.mkdir()
    workers = {}
    errors = {}
    try:
        for actor in commands:
            errors[actor] = (case_dir / (actor + '.stderr')).open('w')
            w = subprocess.Popen(commands[actor], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=errors[actor], text=True,
                                 env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')})
            workers[actor] = w
            assert w.stdout.readline().strip() == 'SESSION_READY'
            index = next(i for i,v in enumerate(fixtures) if v['name'] == case)
            w.stdin.write('C\t' + str(index) + '\t0\n')
            w.stdin.flush()
            assert w.stdout.readline().strip() == 'CASE_READY\t' + case
            for _ in range(2):
                w.stdin.write('W'); w.stdin.flush()
                assert w.stdout.readline().startswith('WARMUP_DONE\t' + case)
        for block in range(a.counter_blocks):
            order = ['mech', 'roc'] if block % 2 == 0 else ['roc', 'mech']
            for actor in order:
                w = workers[actor]
                before = usage(w.pid)
                start = time.perf_counter_ns()
                w.stdin.write('W'); w.stdin.flush()
                response = w.stdout.readline().strip()
                elapsed = (time.perf_counter_ns() - start) * 1e-9
                after = usage(w.pid)
                fields = response.split('\t')
                assert fields[:2] == ['WARMUP_DONE', case]
                calls = int(fields[2])
                delta = {k: after[k] - before[k] for k in names}
                cpu_seconds = (delta['user_time'] + delta['system_time']) * tb.numer / tb.denom * 1e-9
                row = {'case': case, 'actor': actor, 'block': block, 'calls': calls,
                       'response': response, 'wall_seconds': elapsed,
                       'cpu_seconds': cpu_seconds,
                       'seconds_per_call': float(fields[3]) / calls,
                       'instructions_per_call': delta['instructions'] / calls,
                       'cycles_per_call': delta['cycles'] / calls,
                       'cycles_per_instruction': delta['cycles'] / delta['instructions'],
                       'effective_ghz': delta['cycles'] / cpu_seconds / 1e9,
                       'before': before, 'after': after, 'delta': delta}
                records.append(row)
                (out / 'counter-records.json').write_text(json.dumps(records, indent=2) + '\n')
        # Each sampler observes only the selected worker doing CPU work.
        for actor in ['mech', 'roc']:
            w = workers[actor]
            capture = case_dir / (actor + '-sample.txt')
            with (case_dir / (actor + '-sample.stdout')).open('w') as log:
                sampler = subprocess.Popen(['/usr/bin/sample', str(w.pid), '5', '1', '-file', str(capture)],
                                           stdout=log, stderr=subprocess.STDOUT)
                warmups = []
                started = time.monotonic()
                while time.monotonic() - started < 6:
                    w.stdin.write('W'); w.stdin.flush()
                    response = w.stdout.readline().strip()
                    assert response.startswith('WARMUP_DONE\t' + case)
                    warmups.append(response)
                assert sampler.wait() == 0
            (case_dir / (actor + '-profile-loop.json')).write_text(json.dumps(warmups, indent=2) + '\n')
            w.stdin.write('NN'); w.stdin.flush()
            checks = [json.loads(w.stdout.readline()) for _ in range(2)]
            assert all(v['exact_reference'] and v['independent_inflation'] and
                       v['input_alias_unchanged'] and v['borrowed_output_alias_unchanged'] for v in checks)
            assert w.stdout.readline().strip() == 'CASE_DONE\t' + case
            (case_dir / (actor + '-checks.json')).write_text(json.dumps(checks, indent=2) + '\n')
            w.stdin.write('Q\n'); w.stdin.flush()
            assert w.wait() == 0
        print('Completed', plan['cpu'], case, flush=True)
    finally:
        for w in workers.values():
            if w.poll() is None:
                w.terminate(); w.wait()
        for log in errors.values():
            log.close()

summary = []
for case in cases:
    for actor in commands:
        rows = [v for v in records if v['case'] == case and v['actor'] == actor]
        summary.append({'case': case, 'actor': actor,
                        **{key: statistics.median(v[key] for v in rows) for key in
                           ['seconds_per_call', 'instructions_per_call', 'cycles_per_call',
                            'cycles_per_instruction', 'effective_ghz', 'cpu_seconds']}})
(out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
(out / 'processes-after.txt').write_text(run(['/bin/ps', '-axo', 'pid,ppid,%cpu,etime,comm', '-r']) + '\n')
print(json.dumps(summary, indent=2), flush=True)

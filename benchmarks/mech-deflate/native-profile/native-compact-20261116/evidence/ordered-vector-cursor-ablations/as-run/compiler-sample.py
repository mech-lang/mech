"""Sample the exact published compiler separately from matched timing."""

from collections import Counter, defaultdict
import json
import os
from pathlib import Path
import re
from statistics import median
import subprocess
import sys

from build import CONTROL_PLAN, read, save, sha
from stage import BASE, HERE

OUT = HERE / 'compiler-sample'
ACTOR = 'mech-ascii-miss'
SAMPLER = '/usr/bin/sample'
FRAME = re.compile(r"^([ !:+|]+?)(\d+) (.+?)  \(in [^)]+\)")
THREAD = re.compile(r"^\s+\d+ Thread[_ ]\d+(?:: (\S+))?")
IDLE = ('pthread_join', 'Thread4join', 'ulock_wait', 'psynch_cvwait', 'semaphore_wait',
        'psynch_mutexwait', 'pthread_cond_wait')
BOUNDARIES = [
    ('tail-exit-liveness', ('phase_numeric5exits8liveness', 'phase_numeric5exits10live_query',
                           'phase_numeric::exits::liveness', 'phase_numeric::exits::live_query')),
    ('guard-closure', ('6guards5stateNtB5_5State5close', 'guard_state::State::close')),
    ('ssa-construction', ('cranelift_frontend3ssa', 'cranelift_frontend::ssa::', 'SSABuilder')),
    ('tail-provenance', ('phase_numeric', 'readonly_numeric', '7prepare4tail', 'prepare::tail::',
                         'numeric_tree', 'prepare13numeric_phase', 'prepare::numeric_phase::')),
    ('ownership-guards', ('guard_state', '7compact6guards', 'compact::guards::')),
    ('guarded-ownership-search', ('owner_flow', 'owner_retirement', 'private_chains')),
    ('constant-phi-removal', ('remove_constant_phis', 'remove_trivial_params')),
    ('verifier', ('8verifier', '9verify_if', 'verify_with_analyses')),
    ('register-allocation', ('regalloc2', 'regalloc')),
    ('cranelift-egraphs', ('egraph', '11aegraph')),
    ('native-cleanup', ('cfg_cleanup', 'unsigned_guard_facts')),
    ('native-emitter', ('emit_compact_closed_object', 'mech_gpu15native_artifact', 'mech_gpu::native_artifact')),
    ('parser', ('parse_canonical_document', 'mech_syntax8document', 'mech_syntax::document')),
    ('source-semantics', ('source_semantics', 'lower_native_program_artifact')),
    ('catalog', ('build_catalog', 'FunctionCatalog', 'catalog::', '7catalog')),
]


def category(path):
    for label, symbols in BOUNDARIES:
        if any(fragment in frame for fragment in symbols for frame in path):
            return label
    return 'other'


def graph(path):
    stack = []
    counts = Counter()
    leaves = defaultdict(Counter)
    threads = defaultdict(Counter)
    thread_name = 'unlabeled'

    def finish():
        names = [name for _, _, name, _ in stack]
        _, count, leaf, children = stack.pop()
        weight = count - children
        assert weight >= 0, (path, count, children, leaf)
        if weight == 0:
            return
        # The old stack captures can also contain the main thread waiting
        # for a worker. Keep waits distinct from compilation work.
        if any(fragment in name for name in names for fragment in IDLE):
            label = 'thread-wait'
        else:
            label = category(names)
        threads[thread_name][label] += weight
        if thread_name == 'compile-profile':
            counts[label] += weight
            leaves[label][leaf] += weight

    started = False
    for line in path.read_text().splitlines():
        if line == 'Call graph:':
            started = True
            continue
        if not started:
            continue
        if line.startswith(('Total number in stack', 'Binary Images:', 'Sort by top of stack')):
            break
        if match := THREAD.match(line):
            while stack:
                finish()
            thread_name = match[1] or 'unlabeled'
            continue
        match = FRAME.match(line)
        if match is None:
            continue
        depth, count, name = len(match[1]), int(match[2]), match[3]
        while stack and depth <= stack[-1][0]:
            finish()
        if stack:
            stack[-1][3] += count
        stack.append([depth, count, name, 0])
    while stack:
        finish()
    assert counts
    return counts, {label: value.most_common(12) for label, value in leaves.items()}, threads


def main():
    target = read(CONTROL_PLAN)['actors'][ACTOR]
    compiler, source, _, mode = target['compile']
    assert sha(Path(compiler)) == target['compiler_sha256']
    assert sha(Path(source)) == '653597f08f889908c48610e90aa65e94a77847fb34500d0cc3a046f83cd3d299'
    env = {key: value for key, value in os.environ.items() if not key.startswith('MECH_')} | target['flags']
    OUT.mkdir()
    plan = {'source_pin': BASE, 'compiler_sha256': target['compiler_sha256'],
            'compiler': compiler, 'source': source, 'source_sha256': sha(Path(source)),
            'flags': target['flags'], 'sampling_interval_ms': 1, 'sampling_duration_seconds': 3,
            'output_contract': target['artifact_sha256']}
    save(OUT / 'plan.json', plan)
    rows = []
    for iteration in range(4):
        folder = OUT / f'sample-{iteration + 1}'
        folder.mkdir()
        emitted = folder / 'frontend'
        raw = folder / 'sample.txt'
        call = [compiler, source, str(emitted), mode]
        with (folder / 'compiler.log').open('wb') as compiler_log:
            with subprocess.Popen(call, env=env, stdout=compiler_log, stderr=subprocess.STDOUT) as child:
                sample = [SAMPLER, str(child.pid), '3', '1', '-mayDie', '-file', str(raw)]
                with (folder / 'sample.log').open('wb') as sample_log:
                    subprocess.run(sample, stdout=sample_log, stderr=subprocess.STDOUT, check=True, timeout=12)
                assert child.wait(timeout=12) == 0
        artifacts = {'encoder': 'program.o', 'metadata': 'metadata.json', 'bytecode': 'program.mecb'}
        assert {label: sha(emitted / name) for label, name in artifacts.items()} == plan['output_contract']
        result = json.loads((emitted / 'compile.json').read_bytes())
        phases = {entry['phase']: entry['seconds'] for entry in result['phases']}
        counts, leaves, threads = graph(raw)
        rows.append({'raw_sha256': sha(raw), 'phase_seconds_instrumented': phases,
                     'stack_counts': counts, 'hot_leaves': leaves, 'threads': threads})
        print(iteration + 1, counts.most_common(12), flush=True)
    summarize(rows)


def summarize(rows):
    labels = sorted({key for row in rows for key in row['stack_counts']})
    totals = {key: median(row['stack_counts'].get(key, 0) for row in rows) for key in labels}
    totals = dict(sorted(totals.items(), key=lambda item: -item[1]))
    save(OUT / 'result.json', {'plan_sha256': sha(OUT / 'plan.json'), 'captures': rows,
                              'median_coordinator_samples': totals})
    print(totals, flush=True)


def reclassify():
    previous = read(OUT / 'result.json')
    assert previous['plan_sha256'] == sha(OUT / 'plan.json')
    captures = previous['captures']
    for iteration, row in enumerate(captures, 1):
        raw = OUT / f'sample-{iteration}/sample.txt'
        assert row['raw_sha256'] == sha(raw)
        sample_counts, leaves, threads = graph(raw)
        row.update(stack_counts=sample_counts, hot_leaves=leaves, threads=threads)
    summarize(captures)


if __name__ == '__main__':
    (reclassify if sys.argv[1:] == ['reclassify'] else main)()

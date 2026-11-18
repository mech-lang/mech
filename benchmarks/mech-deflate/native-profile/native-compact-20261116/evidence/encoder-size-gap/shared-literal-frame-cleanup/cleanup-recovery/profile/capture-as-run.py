"""Attribute the extra final-cleanup work with samples, separate from build timing."""

from collections import Counter
from hashlib import file_digest
import json
import os
from pathlib import Path
import re
import subprocess


HERE = Path(__file__).resolve().parent
RECORD = json.loads((HERE/'literal-frame-batched-schedule/batched-routes/receipt.json').read_text())
FRAME = re.compile(r'^([ !:+|]+?)(\d+) (.+?)  \(in [^)]+\)')
THREAD = re.compile(r'^\s+\d+ Thread[_ ]')


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def classify(sample):
    stack, leaves, groups = [], Counter(), Counter()

    def pop():
        _, total, children, name = stack[-1]
        count = total-children
        assert count >= 0
        path = [node[3] for node in stack]
        stack.pop()
        if not count or not any('optimize_final_batched_routes' in frame for frame in path):
            return
        groups['final_cleanup'] += count
        leaves[name] += count
        vocabulary = ' '.join(path).lower()
        for category, terms in [
            ('verification', ['verifier', 'verify_ir', 'verify_context']),
            ('constant_phis', ['remove_constant_phis']),
            ('local_phis', ['local_phis']),
            ('edge_rewrites', ['threaded_edges', 'identity_routes']),
            ('range_proofs', ['word_ranges', 'selected_ranges', 'known_widths']),
            ('liveness', ['dead_values']),
            ('graph_construction', ['dominator', 'flowgraph']),
        ]:
            if any(term in vocabulary for term in terms):
                groups[category] += count
                break
        else:
            groups['other'] += count

    def clear():
        while stack:
            pop()

    active = False
    for line in sample.read_text().splitlines():
        if line == 'Call graph:':
            active = True
            continue
        if not active:
            continue
        if line.startswith(('Total number in stack', 'Sort by top of stack')):
            break
        if THREAD.match(line):
            clear()
        if match := FRAME.match(line):
            indent, count, name = len(match[1]), int(match[2]), match[3]
            while stack and indent <= stack[-1][0]:
                pop()
            if stack:
                stack[-1][2] += count
            stack.append([indent, count, 0, name])
    clear()
    return groups, leaves


def main():
    output = HERE/'final-cleanup-full-window-samples'
    output.mkdir()
    binary, source, _, mode = RECORD['command']
    assert sha(binary) == RECORD['compiler_sha256']
    env = {k: v for k, v in os.environ.items() if not k.startswith('MECH_')} | RECORD['flags']
    combined, leaves, observations = Counter(), Counter(), []
    for number in range(3):
        work = output/str(number)
        work.mkdir()
        command = [binary, source, str(work/'frontend'), mode]
        capture = work/'sample.txt'
        with (work/'compiler.log').open('w') as log, (work/'sampler.log').open('w') as sampling:
            with subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT) as child:
                subprocess.run(['/usr/bin/sample', str(child.pid), '4', '1', '-file', capture],
                               stdout=sampling, stderr=sampling, check=True)
                assert child.wait(timeout=30) == 0
        for artifact, pin in [('program.o', 'object_sha256'), ('metadata.json', 'metadata_sha256'),
                              ('program.mecb', 'bytecode_sha256')]:
            assert sha(work/'frontend'/artifact) == RECORD[pin]
        groups, counts = classify(capture)
        combined.update(groups)
        leaves.update(counts)
        observations.append({'samples': dict(groups), 'sample_sha256': sha(capture)})
        print(number, dict(groups), flush=True)
    result = {'source_commit': RECORD['source_commit'], 'compiler_sha256': sha(binary),
              'observations': observations, 'samples': dict(combined),
              'top_leaves': leaves.most_common(25),
              'scope': 'Three instrumented source compilations. Samples attribute final cleanup only; they are not matched build timings.'}
    (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2), flush=True)


if __name__ == '__main__':
    main()

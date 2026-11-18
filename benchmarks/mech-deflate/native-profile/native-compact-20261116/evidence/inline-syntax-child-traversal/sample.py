"""Attribute stacks within the compiler's measured semantic frontend."""

from collections import Counter
from hashlib import file_digest
import json
import os
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
PLAN = HERE.parent / 'immutable-root-literals/matched/plan.json'
PIN = 'mech-literals-enabled'
OBSERVATIONS = HERE / 'source-stack-samples-profiled'
FRAME = re.compile(r"^([ !:+|]+?)(\d+) (.+?)  \(in [^)]+\)")


def sha(path):
    with Path(path).open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def attributed(name):
    if 'source_semantics' not in name:
        return False
    return any(marker in name for marker in ('compile_collected_document', 'compile_document_units',
                                              'compile_document_with_capture_options',
                                              'compile_document_with_catalog'))


def summarize(stack_path):
    stack = []
    semantic_leaves = Counter()
    invocation_frames = Counter()
    semantic = inside_lowering = 0
    active = False

    def close():
        nonlocal semantic, inside_lowering
        _, visits, name, inner = stack[-1]
        depth = visits - inner
        assert depth >= 0
        if depth and any(attributed(frame[2]) for frame in stack):
            semantic += depth
            semantic_leaves[name] += depth
            involved = [frame[2] for frame in stack]
            if any('compile_document_units' in x for x in involved):
                inside_lowering += depth
                key_frames = {n for n in involved if 'source_semantics' in n
                              and 'compile_document_units' not in n and not attributed(n)}
                invocation_frames.update({label: depth for label in key_frames})
        stack.pop()

    for line in stack_path.read_text(errors='replace').splitlines():
        if line == 'Call graph:':
            active = True
        if line.startswith(('Total number in stack', 'Sort by top of stack', 'Binary Images:')):
            break
        match = FRAME.match(line) if active else None
        if not match:
            continue
        depth, visits, name = len(match[1]), int(match[2]), match[3]
        while stack and stack[-1][0] >= depth:
            close()
        if stack:
            stack[-1][3] += visits
        stack.append([depth, visits, name, 0])
    while stack:
        close()
    return {'semantic_samples': semantic, 'document_lowering_samples': inside_lowering,
            'semantic_leaf_frames': semantic_leaves.most_common(36),
            'document_lowering_frames': invocation_frames.most_common(36)}


def main():
    pinned = json.loads(PLAN.read_bytes())['settings'][PIN]
    compiler, source = pinned['compile'][:2]
    assert sha(compiler) == pinned['compiler_sha256']
    OBSERVATIONS.mkdir(exist_ok=False)
    env = {key:value for key,value in os.environ.items() if not key.startswith('MECH_')}
    rows = []
    for step in range(1,9):
        place = OBSERVATIONS / f'sample-{step}'
        place.mkdir()
        raw = place / 'call-graph.txt'
        command = [compiler,source,str(place/'frontend'),'frontend']
        with (place/'compiler.log').open('wb') as observed, (place/'sampler.log').open('wb') as sampling:
            with subprocess.Popen(command, env=env | pinned['flags'], stdout=observed,stderr=subprocess.STDOUT) as child:
                args = ['/usr/bin/sample',str(child.pid),'2','1','-file',str(raw)]
                result = subprocess.run(args, stdout=sampling, stderr=subprocess.STDOUT)
                assert child.wait(timeout=10)==0
        assert sha(place/'frontend/program.mecb') == pinned['artifact_sha256']['bytecode']
        if not raw.exists():
            print(f'Sample {step} captured no log', result.returncode, flush=True)
            continue
        row = summarize(raw) | {'raw_sha256': sha(raw), 'compiler': command, 'sampler': args,
                             'sampler_exit': result.returncode}
        rows.append(row)
        print('Sample',step,'frontend',row['semantic_samples'],'lowering',row['document_lowering_samples'],flush=True)
    summary = {'compiler_sha256': sha(compiler),'source_sha256':sha(source),'observations':rows,
               'note':'Stack samples from the standalone source-to-bytecode phase. Durations exclude this instrumentation.'}
    (OBSERVATIONS/'summary.json').write_text(json.dumps(summary,indent=2)+'\n')


if __name__=='__main__':
    main()

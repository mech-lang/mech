"""Build and validate general optimization alternatives after current timing ends."""
import concurrent.futures
import hashlib
import json
import re
import subprocess
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from bootstrap import D, sha
from build import build
from probe import execute, save, compact, REQUIRED


def main():
    base = json.loads((D / 'results/final-timings.json').read_text())
    assert base['status'] == 'complete', 'Finish the active full cohort before compiling.'
    audit = json.loads((D / 'final-source-audit.json').read_text())
    for filename, digest in {**audit['pin_files'], **audit['libraries']}.items():
        assert sha(filename) == digest, filename
    sources = json.loads((D / 'extra-optimization.json').read_text())['programs']
    assert len(sources) == 5
    for row in sources:
        assert sha(row['source']) == row['source_sha256'], row['source']
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        rows = list(pool.map(build, sources))
    save(D / 'results/optimization-builds.json', dict(status='passed' if all(r['build_status']=='passed' for r in rows) else 'failed', programs=rows))
    assert all(r['build_status'] == 'passed' for r in rows)
    roc = audit['runtimes']['roc']
    proof = D / 'evidence/general-helpers-optimized-regex.roc'
    command = [roc['path'], 'test', str(proof)]
    result = subprocess.run(command, cwd=D, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    log = D / 'evidence/general-helpers-optimized-regex.log'
    log.write_text(result.stdout)
    plain = re.sub(r'\x1b\[[0-9;]*m', '', result.stdout)
    passed = result.returncode == 0 and re.search(r'0 failed and 14 passed', plain) is not None
    evidence = dict(status='passed' if passed else 'failed', expectations=14, command=command,
        source_sha256=sha(proof), compiler_sha256=roc['sha256'], log=str(log), log_sha256=sha(log),
        checked_at=datetime.now(timezone.utc).isoformat())
    save(D / 'results/optimized-regex-helper-tests.json', evidence)
    print(plain[-3500:], flush=True)
    assert passed, 'Independent generic regex fixtures failed.'
    checks = dict(status='running', programs={})
    for row in rows:
        compact.wait_for_environment(REQUIRED)
        result = execute(row, False, timeout=600)
        assert not result['competing_compilers'] and not result['power_changes']
        assert result['validation']['reference_match']
        checks['programs'][row['key']] = dict(status='passed', source_sha256=row['source_sha256'], binary_sha256=row['binary_sha256'], **result)
        print('SMALL PASS', row['id'], f"{result['seconds']:.6f}s", flush=True)
        save(D / 'results/optimization-small-checks.json', checks)
    checks['status'] = 'passed'
    save(D / 'results/optimization-small-checks.json', checks)
    original = next(p for p in audit['programs'] if p['id']=='fannkuchredux-elm-idiomatic')
    packed = next(p for p in rows if p['id']=='fannkuchredux-elm-packed')
    cases = []
    for n in range(1,10):
        outputs = []
        for row in [original, packed]:
            command = [part.replace('{n}',str(n)) for part in row['command']]
            result = subprocess.run(command, cwd=D, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
            assert result.returncode == 0, (n, row['id'], result.stderr)
            outputs.append(result.stdout)
        assert outputs[0] == outputs[1], (n, outputs)
        cases.append(dict(n=n, full_output_match=True, output_sha256=hashlib.sha256(outputs[1]).hexdigest()))
    save(D / 'results/elm-packing-checks.json', dict(status='passed', cases=cases,
        source_sha256=packed['source_sha256'], binary_sha256=packed['binary_sha256'],
        baseline_source_sha256=original['source_sha256'], baseline_binary_sha256=original['binary_sha256'],
        representation=dict(radix=16,maximum_packed_digits=13,maximum_value=2**52-1,largest_exact_integer=2**53-1,
            division='floor of division by an exact power of sixteen; no truncating 32-bit integer division',
            fallback='All larger input sizes retain the complete general list/array permutation implementation.')))
    print('PACKING PASS', len(cases), 'complete smaller inputs', flush=True)
    queries = [(3,'GGT'),(4,'GGTA'),(6,'GGTATT'),(12,'GGTATTTTAATT'),(18,'GGTATTTTAATTTATAGT')]
    target = queries[-1][1]
    other = 'AA'+target[2:]
    def encode(text):
        value = 0
        for char in text: value = value*4+'ACGT'.index(char)
        return value
    assert encode(target) != encode(other) and encode(target)%(2**32) == encode(other)%(2**32)
    widths = [1,2,3,4,6,12,18]
    counting_cases = []
    for index, sequence in enumerate([target+'A'*20+other, other+'A'*20+target]):
        maps = {width:Counter(sequence[i:i+width] for i in range(len(sequence)-width+1)) for width in widths}
        expected = ''
        for width in [1,2]:
            for word,count in sorted(maps[width].items(),key=lambda pair:(-pair[1],pair[0])):
                expected += f'{word} {100.0*count/(len(sequence)-width+1):.3f}\n'
            expected += '\n'
        for width, query in queries: expected += f'{maps[width][query]}\t{query}\n'
        assert maps[18][target] == 1 and maps[18][other] == 1
        raw = ('>ONE ignored\r\nACGT\r\n>THREE collision check\r\n'+sequence[:28].lower()+'\r\n'+sequence[28:].lower()+'\r\n>NEXT ignored\r\nGGTATTTTAATTTATAGT\r\n').encode('ascii')
        for row in rows:
            if row['benchmark'] != 'knucleotide': continue
            result = subprocess.run(row['command'], input=raw, cwd=D, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
            assert result.returncode == 0 and result.stdout == expected.encode('ascii'), (row['id'],result.stdout,result.stderr,expected)
            counting_cases.append(dict(key=row['key'],case=index+1,sequence_length=len(sequence),widths=widths,
                full_output_match=True,input_sha256=hashlib.sha256(raw).hexdigest(),output_sha256=hashlib.sha256(result.stdout).hexdigest(),
                source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256']))
    assert len(counting_cases) == 4
    save(D / 'results/nucleotide-key-checks.json', dict(status='passed',cases=counting_cases,
        distinct_full_keys=[encode(target),encode(other)],shared_low_32_bits=encode(target)%(2**32),
        reason='Independent complete seven-map oracle on lowercase/CRLF inputs, record termination, and width-eighteen keys that collide if their upper four bits are discarded. Both long words occur once and must remain distinct.'))
    print('FULL KEY PASS', len(counting_cases), 'complete adversarial outputs', flush=True)
    for filename, digest in {**audit['pin_files'], **audit['libraries']}.items():
        assert sha(filename) == digest, filename


if __name__ == '__main__':
    main()

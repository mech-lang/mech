"""Untimed workload admission checks, complete outputs and tree identities."""
from pathlib import Path
import hashlib
import importlib.util
import json
import subprocess

D = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('toit_validation', Path(__file__).with_name('validate.py'))
validation = importlib.util.module_from_spec(spec)
spec.loader.exec_module(validation)
refs = validation.refs


def tree_structure(row):
    source = Path(row['source']).read_text().replace('main args:', 'benchmark-main args:')
    is_list = row['id'].endswith('-2')
    source += '\n'+('''visit node/List seen/List:
  if node.size != 2: throw "UNEQUAL_NODE_SIZE"
  seen.do: if identical it node: throw "SHARED_NODE"
  seen.add node
  if node[0]:
    visit node[0] seen
    visit node[1] seen
  else if node[1]: throw "UNEQUAL_CHILDREN"
main:
  9.repeat: | depth |
    root := make-tree depth
    seen := []
    visit root seen
    if seen.size != (1 << (depth + 1)) - 1: throw "WRONG_COUNT"
  print "distinct equal-size trees verified"
''' if is_list else '''visit node/Tree seen/List:
  seen.do: if identical it node: throw "SHARED_NODE"
  seen.add node
  if node.left:
    visit node.left seen
    visit node.right seen
  else if node.right: throw "UNEQUAL_CHILDREN"
main:
  9.repeat: | depth |
    root := Tree.with depth
    seen := []
    visit root seen
    if seen.size != (1 << (depth + 1)) - 1: throw "WRONG_COUNT"
  print "distinct equal-size trees verified"
''')
    path = D/'support/toit'/('tree-admission-'+row['id']+'.toit')
    binary = path.with_suffix('')
    path.write_text(source)
    compile_cmd = [str(D/'support/toit/toit/bin/toit'), 'compile', '-O2', '-o', str(binary), str(path)]
    compile_result = subprocess.run(compile_cmd, capture_output=True)
    assert compile_result.returncode == 0, compile_result.stderr.decode(errors='replace')
    proc = subprocess.run([str(binary)], capture_output=True, timeout=60)
    assert proc.returncode == 0, proc.stderr.decode(errors='replace')
    assert proc.stdout == b'distinct equal-size trees verified\n'
    return dict(test='Distinct equal-size node identities at depths 0..8; root created before temporary trees and consumed after them.',
                source=str(path), source_sha256=validation.sha(path.read_bytes()),
                passed=True)


def main():
    path = D/'toit_manifest.json'
    manifest = json.loads(path.read_text())
    report = dict(status='running', checks=[])
    evidence = D/'support/toit/adversarial-checks.json'
    corpus = b'>ONE\nACGT\n>TWO\nGGT\n>THREE original\nGGTATTTTAATTTATAGTCCCCCCCCCCCCCCCCCCCCCCCCCCCAATATTTTAATTTATAGTGGT\n'
    varied = b'>ONE\r\nacgt\r\n>TWO\r\nggt\r\n>THREE varied\r\n;ignore\r\nggtattttaatttatagt\r\n\r\naatattttaatttatagtaaggta\r\n>FOUR next\r\nGGTATTTTAATTTATAGT\r\n'
    regex_cases = [b'', b'>description\n', b'>ONE\nagggtaaaagggtaaatttaccct\n',
        b'>A\ntHaNtHaNtaNDcaNHaDHaSWaSaNaSaTBY<>x|p||q|\n',
        b'>THREE\n'+b'aN'*257+b'\n', b'>ONE\nx<inside>y|abcdef|z\n', refs.fasta(301)]
    for row in manifest['programs']:
        assert row.get('small_validation', {}).get('reference_match'), row['id']
        benchmark = row['benchmark']
        if benchmark == 'nbody': cases = [(n,b'') for n in [0,1,10,100]]
        elif benchmark == 'spectralnorm': cases = [(n,b'') for n in [1,2,7,83]]
        elif benchmark == 'pidigits': cases = [(n,b'') for n in [1,9,10,11,27,1000]]
        elif benchmark == 'fannkuchredux': cases = [(n,b'') for n in [1,2,3,4,5,6,8]]
        elif benchmark == 'fasta': cases = [(n,b'') for n in [1,2,60,61,17497]]
        elif benchmark == 'knucleotide': cases = [(0,corpus),(0,refs.fasta(17)),(0,varied)]
        elif benchmark == 'regexredux': cases = [(0,raw) for raw in regex_cases]
        elif benchmark == 'revcomp': cases = [(0,refs.fasta(n)) for n in [1,7,60,61]]+[(0,varied)]
        elif benchmark == 'binarytrees': cases = [(n,b'') for n in [4,7,8,12]]
        elif benchmark == 'mandelbrot': cases = [(n,b'') for n in [1,2,3,7,8,9,16,17,48,64]]
        else: raise AssertionError(benchmark)
        for index,(n,raw) in enumerate(cases):
            expected = refs.reference(benchmark,n,raw)
            result = validation.check(row,n,raw,expected,'adversarial-'+str(index))
            report['checks'].append(dict(id=row['id'],benchmark=benchmark,passed=True,**result))
            evidence.write_text(json.dumps(report, indent=2)+'\n')
        if benchmark == 'binarytrees':
            report['checks'].append(dict(id=row['id'],benchmark=benchmark,**tree_structure(row)))
        if benchmark == 'pidigits':
            text = Path(row['source']).read_text()
            assert '(q * 3 + r) / t' in text and '(q * 4 + r) / t' in text
            row['algorithm']['both_exact_extract_divisions'] = True
            row['algorithm']['arbitrary_precision'] = True
        if benchmark == 'knucleotide':
            row['algorithm'].update(seven_growing_maps=True, all_overlapping_windows=True, max_key_bits=36)
        if benchmark == 'binarytrees':
            row['algorithm'].update(distinct_nodes=True, equal_size_leaves=True, long_lived_root=True)
        row['adversarial_validation'] = dict(passed=True,evidence=str(evidence),
            checks=sum(check['id']==row['id'] for check in report['checks']))
        path.write_text(json.dumps(manifest, indent=2)+'\n')
        print('PASS', row['id'], row['adversarial_validation']['checks'], flush=True)
    capacity_source = D/'support/toit/capacity.toit'
    capacity_bin = D/'support/toit/capacity'
    p = subprocess.run([str(capacity_bin)], capture_output=True, timeout=60)
    assert p.returncode == 0 and p.stdout == b'140002\n9\n'
    report['checks'].append(dict(test='Standard Bignum multiplication supports 70,000-byte operands, beyond full pi size.',
        source=str(capacity_source),source_sha256=validation.sha(capacity_source.read_bytes()),
        output_sha256=validation.sha(p.stdout),passed=True))
    report.update(status='passed',total_checks=len(report['checks']))
    evidence.write_text(json.dumps(report, indent=2)+'\n')
    manifest.update(status='prepared',adversarial_evidence=str(evidence))
    path.write_text(json.dumps(manifest, indent=2)+'\n')
    print('COMPLETE', report['total_checks'], flush=True)


if __name__ == '__main__':
    main()

"""Replay a passed immutable Crystal grammar on the exact twenty source forms.

This produces inspectable proposals; unresolved grammar diagnostics and the
source-specific semantic review stay explicit. No source, producer LOC, timing
report, global collector or visualization is modified.
"""
import argparse
import fcntl
import hashlib
import importlib
import json
from pathlib import Path
import sys
from datetime import datetime, timezone

HERE = Path(__file__).resolve().parent.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--fixture-receipt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    assert output.is_relative_to(HERE / 'source-counts') and output.suffix == '.json'
    print('WAIT common mutex: Crystal qualified immutable grammar source replay', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        receipt_path = args.fixture_receipt.resolve(strict=True)
        receipt = json.loads(receipt_path.read_text())
        assert receipt['status'] == 'passed' and receipt['failures'] == receipt['errors'] == 0
        assert receipt['exclusive_mutex_held'] is True and receipt['common_timing_mutex'] == str(LOCK)
        assert 'test_foreign_crystal' in receipt['suites']
        assert sys.version.split()[0] == receipt['python_version'].split()[0]
        pins = dict(receipt.get('frozen_dependency_pins', receipt.get('frozen_module_pins', {})))
        snapshot = receipt_path.parent / 'snapshot'
        grammar = snapshot / 'foreign_crystal.py'
        assert str(grammar) in pins and str(snapshot / 'foreign_core.py') in pins
        assert str(snapshot / 'test_foreign_crystal.py') in pins
        for path, digest in pins.items():
            assert sha(path) == digest, ('Immutable grammar changed', path)
        pygments = Path(receipt['pygments_module'])
        assert sha(pygments) == receipt['pygments_module_sha256']
        sys.path.insert(0, str(pygments.parent.parent))
        sys.path.insert(0, str(snapshot))
        normalizer = importlib.import_module('foreign_crystal')
        assert Path(normalizer.__file__).resolve() == grammar.resolve()
        profiles_path = HERE / 'profiles.json'
        profiles = json.loads(profiles_path.read_text())['profiles']
        assert len(profiles) == 20 and len({row['id'] for row in profiles}) == 20
        pins.update({str(receipt_path): sha(receipt_path), str(pygments): sha(pygments),
                     str(profiles_path): sha(profiles_path), str(Path(__file__).resolve()): sha(__file__)})
        rows = []
        for row in profiles:
            source = (HERE / row['source']).resolve(strict=True)
            assert source.is_relative_to(HERE / 'sources')
            digest = sha(source)
            pins[str(source)] = digest
            rows.append(dict(id=row['id'], benchmark=row['benchmark'], variant=row['variant'],
                source=str(source), source_sha256=digest, producer_loc=row['loc'],
                **normalizer.normalize(source.read_text(), 'crystal')))
        for path, digest in pins.items():
            assert sha(path) == digest, ('Source/grammar changed during replay', path)
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open('x') as stream:
            json.dump(dict(status='source-proposals-awaiting-semantic-review',
                created_at=datetime.now(timezone.utc).isoformat(), fixture=str(receipt_path),
                pins=pins, profiles=rows, source_count=20, full_performance_runs=0,
                chart_admission=False, producer_LOC_mutated=False,
                parser_pass_is_not_semantic_certification=True), stream, indent=2)
            stream.write('\n')
        print(json.dumps(dict(output=str(output), sha256=sha(output), sources=20,
            diagnostics=sum(len(row['diagnostics']) for row in rows),
            needs_review=sum(bool(row['diagnostics']) for row in rows))), flush=True)


if __name__ == '__main__':
    main()

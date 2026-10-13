"""Candidate owned otool audit of the exact already-qualified optimized tree."""
import fcntl
import importlib.util
import json
import os
from pathlib import Path
import re
import sys

HERE = Path(__file__).resolve().parents[2]
OUTPUT = HERE / 'allocation-inspections/optimized-v1'
LOCK = Path('/tmp/mech-clbg-timing.lock')


def main():
    print('WAIT common mutex: exact scored optimized Nim tree ARM audit', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        named, opened = LOCK.stat(), os.fstat(lock.fileno())
        assert (named.st_dev, named.st_ino) == (opened.st_dev, opened.st_ino)
        sys.path.insert(0, str(HERE))
        import guarded_prepare_v7 as preparation
        assert Path(preparation.__file__).resolve() == HERE / 'guarded_prepare_v7.py'
        guard = preparation.load_guard(); harness = guard.load_harness()
        harness.install_interrupt_handlers()
        report_path = HERE / 'qualification/private-v7/report.json'
        report = json.loads(report_path.read_text())
        assert report['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
        assert len(report['profiles']) == 20 and len(report['attempts']) == 20 and len(report['proofs']) == 24
        guard.verify(report['pins'])
        row = next(value for value in report['profiles'] if value['id'] == 'nim-binarytrees-optimized')
        binary = Path(row['command'][0]); assert preparation.sha(binary) == row['binary_sha256']
        generated = binary.parent.parent / 'cache/nim-binarytrees-optimized/@moptimized.nim.c'
        assert str(generated) in report['pins'] and preparation.sha(generated) == report['pins'][str(generated)]
        frozen_parser = HERE / 'revisions/work-full-v3/tree_work.py'
        spec = importlib.util.spec_from_file_location('nim_original_creator_reader', frozen_parser)
        module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
        name, creator = module.creator_c(generated)
        assert not OUTPUT.exists(); OUTPUT.mkdir(parents=True)
        guard.OUT, guard.REPORT = OUTPUT, OUTPUT / 'unused-native-report.json'
        source_fragment = OUTPUT / 'optimized-scored-creator.c'
        with source_fragment.open('x') as stream: stream.write(creator)
        receipt = preparation.owned_command(guard, harness, ['/usr/bin/otool', '-tvV', str(binary)],
            dict(os.environ, LC_ALL='C'), OUTPUT / 'scored-binary', lock.fileno(), 60, 'disassembly')
        text = receipt['stdout'].decode(); marker = '_' + name + ':'
        assert text.count(marker) == 1
        after = text.split(marker, 1)[1]
        boundary = re.search(r'(?m)^_[A-Za-z][A-Za-z0-9_$]*:\s*$', after)
        arm = marker + (after[:boundary.start()] if boundary else after)
        arm_path = OUTPUT / 'optimized-scored-creator.arm'
        with arm_path.open('x') as stream: stream.write(arm)
        pins = preparation.merge(report['pins'], preparation.pin_tree(OUTPUT),
            {str(report_path): preparation.sha(report_path), str(Path(__file__).resolve()): preparation.sha(__file__),
             str(frozen_parser): preparation.sha(frozen_parser), str(generated): preparation.sha(generated)})
        guard.verify(pins)
        preparation.new_json(OUTPUT / 'inspection.json', dict(status='actual-scored-optimized-evidence-awaiting-path-review',
            creator_symbol=name, source=str(generated), source_sha256=preparation.sha(generated),
            binary=str(binary), binary_sha256=row['binary_sha256'], creator=str(arm_path),
            scored_creator_source=str(source_fragment), pins=pins, exclusive_common_mutex_held=True,
            native_audit_commands=1, benchmark_replays=0, source_changes=0, allocation_gate_changed=False))
        print(json.dumps(dict(status='actual-scored-optimized-evidence-awaiting-path-review', output=str(OUTPUT))), flush=True)
        print(creator, flush=True); print(arm, flush=True)


if __name__ == '__main__': main()

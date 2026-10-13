"""Preserve fresh compiler-generated C from unchanged V tree sources for review.

No benchmark process runs here. The retained C is compiler evidence, never
counted as application source and never substituted for a V implementation.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import re

import full_study
import prepare

HERE = Path(__file__).resolve().parent


def preserve(args):
    module = full_study.configure('binarytrees')
    destination = HERE/'lowering-evidence'/args.revision
    print('WAIT common mutex: unchanged V tree allocation lowering',flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        assert not destination.exists(),'Never overwrite compiler evidence'
        qualified_path = HERE/'qualification'/args.source_revision/'binarytrees/probes.json'
        qualified = json.loads(qualified_path.read_text())
        assert qualified['status'] == 'bounded-outputs-passed'
        assert qualified['pins_unchanged_at_end'] is True
        bootstrap = json.loads((HERE/'bootstrap.json').read_text())
        assert bootstrap['status'] == 'prepared-smoke-passed'
        harness = module.load_harness()
        harness.install_interrupt_handlers()
        pins = prepare.merge_pins(qualified['pins'],
            {str(path):module.sha(path) for path in (Path(__file__).resolve(),qualified_path)})
        harness.PinGuard(pins).verify()
        destination.mkdir(parents=True)
        report_path=destination/'report.json'
        report=dict(status='preserving',created_at=module.now(),profiles=[],
            full_performance_runs=0,native_benchmark_runs=0,never_chart_eligible=True,
            source_revision=args.source_revision,revision=args.revision)
        module.save(report_path,report)
        try:
            for original in qualified['profiles']:
                directory=destination/original['id']
                directory.mkdir()
                source=directory/'unchanged.v'
                source.write_bytes(Path(original['source']).read_bytes())
                assert module.sha(source)==original['source_sha256']
                temporary=directory/'compiler-temp'
                temporary.mkdir()
                assert not list(temporary.iterdir())
                binary=directory/'unscored-allocation-evidence'
                environment=dict(os.environ,**bootstrap['runtime_environment'])|prepare.ENVIRONMENT
                environment['VTMP']=str(temporary)
                command=[bootstrap['compiler']['path']]+original['compiler_flags']+[
                    '-keepc','-o',str(binary),str(source)]
                prepare.command_build(module,harness,command,environment,directory,lock.fileno())
                retained_c=list(temporary.rglob('*.c'))
                assert retained_c,'Pinned compiler did not retain its generated C; stop for inspection'
                files=[source,binary]+[path for path in temporary.rglob('*') if path.is_file()]
                pins=prepare.merge_pins(pins,{str(path):module.sha(path) for path in files})
                report['profiles'].append(dict(id=original['id'],variant=original['variant'],
                    source=str(source),source_sha256=original['source_sha256'],
                    generated_c=[str(path) for path in retained_c],build_command=command,
                    isolated_empty_compiler_temp=str(temporary),binary_executed=False,
                    added_flag='-keepc retains temporary compiler output; code source and production flags unchanged',
                    allocation_review_status='pending root inspection of creator and allocator definitions'))
                module.save(report_path,report)
            harness.PinGuard(pins).verify()
            report.update(status='generated-C-retained-awaiting-root-allocator-review',pins=pins,
                pins_unchanged_at_end=True,finished_at=module.now())
        except BaseException as error:
            report.update(status='stopped-unqualified',error=str(error),
                error_type=type(error).__name__,finished_at=module.now(),no_automatic_retry=True)
            raise
        finally:
            module.save(report_path,report)


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--source-revision',required=True)
    parser.add_argument('--revision',required=True)
    args=parser.parse_args()
    for value in (args.source_revision,args.revision):
        assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}',value)
    preserve(args)


if __name__ == '__main__':
    main()

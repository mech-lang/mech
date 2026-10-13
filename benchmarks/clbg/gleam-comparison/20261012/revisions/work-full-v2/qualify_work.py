"""Unscored actual Gleam work proofs after exact156-output qualification (152 fresh plus four explicit retained cases)."""
import argparse
import copy
import fcntl
import json
from pathlib import Path
import shutil
import sys

import full_study
import fixture_runner
import work_traces
BASE = Path(__file__).resolve().parents[1] / 'private-v2'
sys.path.insert(0, str(BASE))
import guarded_prepare as preparation
import references

HERE = preparation.HERE
GATES = full_study.GATES


def complete_attempt(attempt):
    assert attempt['qualified'] is True and attempt['exit_status'] == attempt['sampler_wait_status'] == 0
    assert set(attempt['qualification_checks']) == GATES
    assert all(value is True for value in attempt['qualification_checks'].values())


def qualified_report(path, guard):
    report = json.loads(path.read_text())
    assert report['status'] == 'bounded-output-ownership-passed-awaiting-work-and-LOC-review'
    assert report['active'] is None and report['full_performance_runs'] == 0
    assert report['chart_admission'] is False and report['no_automatic_retry'] is True
    guard.verify(report['pins'])
    rows = {row['id']:row for row in report['profiles']}
    assert len(rows)==20 and len(report['attempts'])==152 and report['reused_bounded_cases']==4
    assert report['reused_cases_relabelled_as_attempts'] is False and report['fresh_profile_builds']==19
    reuse_path=Path(report['prior_bounded_proof_reuse'])
    assert preparation.sha(reuse_path)==report['prior_bounded_proof_reuse_sha256']
    import proof_reuse
    reuse=json.loads(reuse_path.read_text())
    proof_reuse.validate_packet(reuse,preparation,preparation.load_service(),guard)
    prior=rows['gleam-binarytrees-compact']
    assert prior['prior_bounded_records']==reuse['prior_records']
    assert prior['qualification_origin']=='exact-unchanged-private-v1-app-and-four-records'
    assert prior['source_bundle']==reuse['profile']['source_bundle'] and prior['compiled_artifacts']==reuse['profile']['compiled_artifacts']
    assert prior['command']==reuse['profile']['command'] and prior['runtime_environment']==reuse['profile']['runtime_environment']
    inventory = preparation.preflight_inputs(references)
    expected = {(row['id'], case) for row in rows.values() for case in inventory[row['benchmark']]}
    actual={(attempt['id'],attempt['case']) for attempt in report['attempts']}
    retained={(prior['id'],item['case']) for item in prior['prior_bounded_records']}
    assert not actual & retained and actual | retained == expected
    assert retained=={(prior['id'],'n-'+str(n)) for n in (0,4,7,9)}
    for attempt in report['attempts']:
        complete_attempt(attempt)
        row = rows[attempt['id']]
        assert attempt['source_sha256'] == row['source_sha256']
        assert row['language'] == 'gleam' and row['compiler_version'] == '1.19.0'
        assert row['runtime_version'] == '29.1.1' and row['measured_architecture'] == 'arm64'
        assert row['backend'] == 'BEAM load-time JIT'
        assert preparation.sha(row['source']) == row['source_sha256']
        assert preparation.sha(row['command'][0]) == row['binary_sha256']
        assert all(preparation.sha(item['path']) == item['sha256'] for item in row['source_bundle'])
        guard.verify(row['compiled_artifacts'])
        assert preparation.sha(row['compiled_input_inventory']) == row['compiled_input_inventory_sha256']
    guard.verify(prior['support']);guard.verify(prior['compiled_artifacts'])
    assert preparation.sha(prior['source'])==prior['source_sha256'] and preparation.sha(prior['command'][0])==prior['binary_sha256']
    return report


def compile_copy(service, guard, harness, qualification, row, source, label, output, lock_fd, pins, revision):
    directory = Path(qualification['build']) / 'source-work' / revision / label
    assert not directory.exists()
    package = directory / 'package'
    src = package / 'src'
    src.mkdir(parents=True)
    main = src / 'benchmark.gleam'
    main.write_text(source)
    sources = [main]
    for item in row['source_bundle'][1:]:
        old = Path(item['path'])
        target = src / old.name
        shutil.copyfile(old, target)
        assert preparation.sha(target) == item['sha256']
        sources.append(target)
    for name, text in [('clbg_trace.gleam',work_traces.TRACE_GLEAM),('clbg_trace_ffi.erl',work_traces.TRACE_ERLANG)]:
        target = src / name
        target.write_text(text)
        sources.append(target)
    (package / 'gleam.toml').write_text('name = "benchmark"\nversion = "1.0.0"\ntarget = "erlang"\n[dependencies]\ngleam_stdlib = "== 1.0.5"\n')
    application = directory / 'application'
    libraries = Path(qualification['libraries'])
    environment = dict(row['runtime_environment'],ERL_CRASH_DUMP=str(output/'builds'/label/'compiler-erl_crash.dump'))
    command = preparation.compile_command(Path(qualification['compiler']), package, application, libraries)
    service.owned_command(guard, harness, command, environment, output / 'builds' / label, lock_fd, 180, 'compile')
    assert (application / 'ebin/benchmark.beam').is_file()
    pins.update(service.merge(pins, service.pin_tree(directory)))
    artifact_pins = service.pin_tree(application)
    inventory = output / 'builds' / label / 'observer-compiler-input-inventory.json'
    preparation.new_json(inventory, dict(status='unscored-exact-authored-observer-compiler-inputs',
        source=str(main), source_sha256=preparation.sha(main),
        source_bundle=[dict(path=str(p),sha256=preparation.sha(p)) for p in sources],
        build_command=command, compiled_artifacts=artifact_pins,
        runtime_classpath=[str(application / 'ebin'), str(libraries / 'gleam_stdlib/ebin')]))
    pins[str(inventory)] = preparation.sha(inventory)
    observed = dict(row, id=row['id'] + '-work-trace', key=row['key'] + '-work-trace',
        source=str(main), source_sha256=preparation.sha(main),
        command=preparation.runtime_command(application, libraries),runtime_environment=environment,
        qualified_runtime_environment=dict(row['runtime_environment']),
        runtime_environment_rebinding='Fresh observer/compiler crash output; observer execution fingerprint is newly derived',
        support=service.merge(row['support'], service.pin_tree(application), {str(p):preparation.sha(p) for p in sources[1:]}),
        source_bundle=[dict(path=str(p),sha256=preparation.sha(p)) for p in sources],
        compiled_artifacts=artifact_pins, compiled_input_inventory=str(inventory),
        compiled_input_inventory_sha256=preparation.sha(inventory),
        runtime_classpath=[str(application / 'ebin'), str(libraries / 'gleam_stdlib/ebin')],
        preparation_only=True, never_chart_eligible=True)
    return observed


def probe_trace(service, guard, harness, row, n, data, expected, pins, output, lock_fd, report, case, reconcile):
    directory = output / row['id'] / case
    observed = copy.deepcopy(row)
    observed['runtime_environment']['CLBG_TRACE'] = str(directory / 'trace')
    observed['runtime_environment']['ERL_CRASH_DUMP'] = str(directory/'erl_crash.dump')
    attempt = service.probe(guard, harness, observed, n, data, expected, pins, output, lock_fd, report, case, trace=reconcile)
    complete_attempt(attempt)
    return attempt


def scored_allocation(service, guard, harness, row, output, lock_fd):
    applications = {Path(path).parent for path in row['compiled_artifacts'] if Path(path).name == 'benchmark.beam'}
    assert len(applications) == 1
    ebin = applications.pop()
    outputs = []
    for name in ('benchmark','clbg_reference'):
        beam = ebin / (name + '.beam')
        assert str(beam) in row['compiled_artifacts']
        expression = 'io:format("~tp.~n",[beam_disasm:file("' + str(beam) + '")]),halt().'
        result = service.owned_command(guard, harness,
            [str(preparation.ERL), *preparation.RUNTIME_FLAGS, '-noshell','-noinput','-eval',expression],
            dict(row['runtime_environment'],ERL_CRASH_DUMP=str(output/'scored-allocation'/row['id']/(name+'-erl_crash.dump'))), output / 'scored-allocation' / row['id'], lock_fd, 30, name)
        outputs.append(result['stdout'].decode())
    return work_traces.allocation_beam(*outputs)


INTEGER_PRIMITIVE = '''import clbg_io as io
fn power(n: Int, value: Int) -> Int { case n == 0 { True -> value False -> power(n - 1, value * 10) } }
pub fn main() -> Nil {
  let big = power(10000, 1)
  let assert True = big / power(9999, 1) == 10
  let assert True = 17 / 5 == 3
  let assert True = -17 / 5 == -3
  io.text("BEAM unbounded Int primitive passed\\n")
}
'''
REGEX_PRIMITIVE = '''import clbg_io as io
import clbg_regex as regex
pub fn main() -> Nil {
  let pattern = regex.compile("a+")
  let assert Ok(#(1,2)) = regex.search(pattern, <<"baa":utf8>>, 0)
  let assert Error(Nil) = regex.search(pattern, <<"bbb":utf8>>, 0)
  io.text("OTP re primitive passed\\n")
}
'''


def run(args):
    print('WAIT common mutex: Gleam actual BEAM source-work qualification', flush=True)
    with (HERE / 'source-work-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with preparation.LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            service = preparation.load_service()
            guard = service.load_guard()
            harness = guard.load_harness()
            harness.install_interrupt_handlers()
            prerequisite = HERE / 'qualification' / args.qualified_revision / 'report.json'
            qualification = qualified_report(prerequisite, guard)
            output = HERE / 'work-qualification' / args.revision
            assert not output.exists()
            output.mkdir(parents=True)
            guard.OUT, guard.REPORT = output, output / 'report.json'
            paths = [Path(__file__).resolve(), Path(work_traces.__file__).resolve(),
                Path(__file__).with_name('test_work_traces.py'), Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'),
                Path(__file__).with_name('fixture_runner.py'), Path(__file__).with_name('full_study.py'),
                Path(__file__).with_name('test_full_study.py'), Path(__file__).with_name('WORK-PROTOCOL.md')]
            pins = service.merge(qualification['pins'],{str(prerequisite):preparation.sha(prerequisite)},
                {str(p):preparation.sha(p) for p in paths})
            report = dict(status='preparing-source-work', prerequisite=str(prerequisite),
                prerequisite_sha256=preparation.sha(prerequisite), profiles=qualification['profiles'],
                qualification_revision=args.qualified_revision, work_revision=args.revision,
                attempts=[], active=None, full_performance_runs=0, chart_admission=False,
                no_automatic_retry=True, full_manifests_sealed=0, proofs=[], source_reviews=[])
            guard.save(guard.REPORT, report)
            try:
                receipt = output / 'fixtures/actual-inventory.json'
                service.owned_command(guard,harness,[sys.executable,'-B',str(Path(fixture_runner.__file__).resolve()),
                    '--set','work','--receipt',str(receipt)],
                    dict(qualification['profiles'][0]['runtime_environment'],PYTHONPATH=str(Path(__file__).parent),ERL_CRASH_DUMP=str(output/'fixtures/erl_crash.dump')),
                    output / 'fixtures',lock.fileno(),30,'source-work-fixtures')
                fixture_runner.validate_receipt(json.loads(receipt.read_text()),'work')
                pins.update(service.merge(pins,service.pin_tree(output / 'fixtures')))
                report['actual_mock_fixtures'] = dict(receipt=str(receipt),sha256=preparation.sha(receipt),methods=17,zero_skips=True)
                for row in qualification['profiles']:
                    source = Path(row['source']).read_text()
                    benchmark, optimized = row['benchmark'], row['variant'] == 'optimized'
                    report['source_reviews'].append(dict(profile=row['id'],status='passed',
                        scored_source_sha256=row['source_sha256'],scored_binary_sha256=row['binary_sha256'],
                        compiled_source_bundle=row['source_bundle'],compiled_artifacts=row['compiled_artifacts'],
                        worker_cap=row['maximum_compute_workers'],complete_bounded_cases=sum(a['id']==row['id'] for a in qualification['attempts'])+len(row.get('prior_bounded_records',[])),
                        review=str(Path(__file__).with_name('REQUIRED_SOURCE_REVIEW.md'))))
                    if benchmark not in ('binarytrees','pidigits','knucleotide'):
                        continue
                    allocation = scored_allocation(service,guard,harness,row,output,lock.fileno()) if benchmark=='binarytrees' else None
                    transform = {'binarytrees':lambda s:work_traces.tree_source(s,optimized),
                        'pidigits':work_traces.pi_source,'knucleotide':lambda s:work_traces.map_source(s,optimized)}[benchmark]
                    observed = compile_copy(service,guard,harness,qualification,row,transform(source),row['id'],output,lock.fileno(),pins,args.revision)
                    if benchmark == 'binarytrees':
                        cases = [('allocated-n'+str(n),n,None,references.binarytrees(n),lambda d,n=n:work_traces.tree_trace(d,n,optimized)) for n in (0,4,7,9)]
                        kind = 'tree'
                    elif benchmark == 'pidigits':
                        cases = [('real-divisions-n'+str(n),n,None,references.pidigits(n),lambda d,n=n:work_traces.pi_trace(d,n)) for n in (0,1,9,10,11,27,97)]
                        kind = 'pi'
                    else:
                        cases = [('complete-maps-'+name,harness.WORK[benchmark][0],data,references.knucleotide(data),
                            lambda d,data=data,name=name:work_traces.map_trace(d,data,optimized,references,require_growth=name=='arbitrary-widths'))
                            for name,data in preparation.inputs(benchmark,references)]
                        kind = 'maps'
                    for case,n,data,expected,reconcile in cases:
                        attempt = probe_trace(service,guard,harness,observed,n,data,expected,pins,output,lock.fileno(),report,case,reconcile)
                        proof = dict(kind=kind,profile=row['id'],case=case,status='passed',attempt=attempt['attempt'],
                            scored_source_sha256=row['source_sha256'],scored_binary_sha256=row['binary_sha256'],
                            compiled_artifacts=row['compiled_artifacts'],trace_source_sha256=observed['source_sha256'],reconciliation=attempt['source_work_proof'])
                        if allocation: proof['allocation'] = allocation
                        if kind=='maps': proof['reviewed_source_loop'] = dict(status='passed',source=row['source'],source_sha256=row['source_sha256'],
                            method='Actual compact nested every-window encoding' if not optimized else 'Actual rolling warmup/index loop; complete map/window-total proof without per-window event stream')
                        report['proofs'].append(proof)
                        guard.save(guard.REPORT,report)
                for benchmark,source,expected in [('pidigits',INTEGER_PRIMITIVE,b'BEAM unbounded Int primitive passed\n'),
                    ('regexredux',REGEX_PRIMITIVE,b'OTP re primitive passed\n')]:
                    base = next(row for row in qualification['profiles'] if row['benchmark']==benchmark and row['variant']=='compact')
                    observed = compile_copy(service,guard,harness,qualification,base,source,benchmark+'-primitive',output,lock.fileno(),pins,args.revision)
                    observed['runtime_environment']['ERL_CRASH_DUMP']=str(output/observed['id']/'generic-abi/erl_crash.dump')
                    attempt = service.probe(guard,harness,observed,0,None,expected,pins,output,lock.fileno(),report,'generic-abi')
                    complete_attempt(attempt)
                    report['proofs'].append(dict(kind='primitive',profile=base['id'],case='generic-abi',status='passed',attempt=attempt['attempt'],
                        scored_source_sha256=base['source_sha256'],scored_binary_sha256=base['binary_sha256'],
                        compiled_artifacts=base['compiled_artifacts'],trace_source_sha256=observed['source_sha256']))
                assert len(report['proofs']) == len(report['attempts']) == 48 and len(report['source_reviews']) == 20
                pins = service.merge(pins,{p:d for p,d in service.pin_tree(output).items() if Path(p)!=guard.REPORT})
                guard.verify(pins)
                report.update(status='source-work-allocation-passed-awaiting-full-review-and-LOC',finished_at=guard.now(),pins=pins,pins_unchanged_at_end=True)
                guard.save(guard.REPORT,report)
            except BaseException as error:
                report.update(status='stopped-unqualified',finished_at=guard.now(),active=None,error=str(error) or type(error).__name__,error_type=type(error).__name__)
                guard.save(guard.REPORT,report)
                raise


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--qualified-revision',required=True)
    parser.add_argument('--revision',required=True)
    args = parser.parse_args()
    assert args.qualified_revision and args.revision and all(c in 'abcdefghijklmnopqrstuvwxyz0123456789-' for value in (args.qualified_revision,args.revision) for c in value)
    run(args)


if __name__ == '__main__':
    main()

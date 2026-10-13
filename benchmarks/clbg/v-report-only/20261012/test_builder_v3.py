"""Six focused metadata/schema checks; no compiler, benchmark or signals."""
import ast
import copy
import fcntl
import io
import json
import os
from pathlib import Path
from types import SimpleNamespace
import unittest

import prepare_builder_v3 as worker
import retained_builder_v3 as retained
import full_study

FD = None


def declarations():
    return json.loads(worker.DRAFT_PATH.read_text())


def fingerprint(row):
    return json.dumps(dict(id=row['id'], source=row['source_sha256'], binary=row['binary_sha256']), sort_keys=True)


def retained_fixture(benchmark):
    selected = worker.select_profiles(declarations(), benchmark)
    pins, profiles, raw = {}, [], {}
    report = dict(status='bounded-outputs-passed', active=None, benchmark=benchmark,
        revision='production-input-v2', full_performance_runs=0, never_chart_eligible=True,
        pins_unchanged_at_end=True, profiles=profiles, attempts=[], pins=pins)
    cases = ['canonical-small'] + [name for name, _ in worker.input_cases(benchmark)]
    for ordinal, original in enumerate(selected):
        root = retained.PRIOR / benchmark / original['id']
        source, binary = str(root / 'program.v'), str(root / 'program')
        origin = str((worker.HERE / original['source']).resolve())
        source_sha, binary_sha = ('a' if ordinal == 0 else 'b') * 64, ('c' if ordinal == 0 else 'd') * 64
        pins.update({source:source_sha, origin:source_sha, binary:binary_sha})
        compiler = dict(path='/mock/compiler/v', sha256='f'*64)
        pins[compiler['path']] = compiler['sha256']
        command = [compiler['path']] + original['compiler_flags'] + ['-o', binary, source]
        row = dict(original, original_source=origin, source=source, binary=binary,
            source_sha256=source_sha, binary_sha256=binary_sha, command=[binary],
            runtime=compiler, build_command=command,
            small_validation=dict(reference_match=True), build_status='passed',
            qualification_status='bounded-outputs-passed', never_chart_eligible=True)
        profiles.append(row)
        raw[str(root/'build.json')] = dict(command=command, inherited_common_mutex=True,
            returncode=0, pid=900+ordinal)
        for case in cases:
            number = len(report['attempts']) + 1
            result_path = str(retained.PRIOR / benchmark / 'attempts' / f'{number:03d}-{row["id"]}.json')
            attempt = dict(id=row['id'], case=case, attempt=number, round=0, benchmark=benchmark,
                qualified=True, fatal_qualification_fields=[], retryable_environment_only=False,
                full_workload=False, preparation_only=True, never_chart_eligible=True,
                full_performance_run=False, exit_status=0, sampler_wait_status=0, seconds=0.25,
                valid_environment=True, pins_unchanged_at_end=True, power_at_start=retained.POWER,
                power_at_end=retained.POWER, timeout_seconds=300, maximum_rss_mib=8192,
                peak_process_group_rss_kib=4096, remaining_group_members=[], supervisor_remaining_members=[],
                monitor_errors=[], resource_monitor_errors=[], competing_compilers=[], power_changes=[],
                validation=dict(reference_match=True, bytes=3, sha256='e'*64), stdout_bytes=3,
                stdout_sha256='e'*64, command=[binary], source_sha256=source_sha,
                execution_fingerprint=fingerprint(row), supervised_native_group=dict(type='native-group-started',
                    command=[binary], pid=1000+number, pgid=1000+number, inherited_common_mutex=True),
                result_path=result_path)
            report['attempts'].append(attempt)
            raw[result_path] = copy.deepcopy(attempt)
    return report, selected, pins, raw


def check(report, selected, pins, raw, benchmark):
    return retained.validate_records(report, selected, benchmark,
        SimpleNamespace(execution_fingerprint=fingerprint), lambda filename:pins[str(filename)],
        lambda filename:raw[str(filename)])


class BuilderChecks(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(FD, 'Use the common-mutex entry point')

    def test_exact_one_source_and_declaration_repair(self):
        rows = worker.select_profiles(declarations())
        parent = json.loads((worker.HERE / 'revisions/input-v2/profiles.json').read_text())['profiles']
        self.assertEqual(len(rows), 20)
        self.assertEqual(sum(a != b for a,b in zip(rows,parent)), 1)
        self.assertEqual(len(worker.select_profiles(declarations(),'pidigits')), 2)
        self.assertEqual({row['id'] for row in rows if row['benchmark']=='pidigits'},
                         {'v-pidigits-compact-input-v2','v-pidigits-optimized-builder-v3'})

    def test_unrelated_flags_source_and_algorithm_changes_are_rejected(self):
        for mutation in (lambda rows:rows[0]['compiler_flags'].append('-fast'),
                         lambda rows:rows[0].update(source='sources/nbody/compact.v'),
                         lambda rows:rows[0]['algorithm'].update(native_benchmark_kernel=True),
                         lambda rows:rows.pop()):
            changed = declarations()
            mutation(changed['profiles'])
            with self.assertRaises(AssertionError):
                worker.select_profiles(changed)

    def test_complete_two_retained_benchmarks_keep_distinct_records(self):
        for benchmark in retained.RETAINED:
            report, selected, pins, raw = retained_fixture(benchmark)
            before = copy.deepcopy(report)
            proof = check(report, selected, pins, raw, benchmark)
            self.assertEqual(proof['bounded_records'], len(report['attempts']))
            self.assertEqual(proof['native_cases_reexecuted'], 0)
            self.assertFalse(proof['old_records_relabelled'])
            self.assertEqual(report, before)

    def test_missing_duplicate_or_unqualified_retained_records_fail(self):
        for mutation in (lambda value:value['attempts'].pop(),
                         lambda value:value['attempts'][-1].update(case=value['attempts'][-2]['case']),
                         lambda value:value['attempts'][0].update(qualified=False),
                         lambda value:value.update(status='stopped')):
            report, selected, pins, raw = retained_fixture('revcomp')
            mutation(report)
            with self.assertRaises(AssertionError):
                check(report, selected, pins, raw, 'revcomp')

    def test_raw_source_binary_power_or_native_contradictions_fail(self):
        for field, value in (('seconds',0.125), ('power_at_end',dict(source='Battery Power',low_power_mode=0)),
                             ('peak_process_group_rss_kib',90000000)):
            report, selected, pins, raw = retained_fixture('regexredux')
            raw[report['attempts'][0]['result_path']][field] = value
            with self.assertRaises(AssertionError):
                check(report, selected, pins, raw, 'regexredux')
        for mutation in (lambda report,pins:report['attempts'][0]['supervised_native_group'].update(inherited_common_mutex=False),
                         lambda report,pins:pins.update({report['profiles'][0]['binary']:'f'*64}),
                         lambda report,pins:report['profiles'][0].update(source_sha256='f'*64)):
            report, selected, pins, raw = retained_fixture('regexredux')
            mutation(report,pins)
            with self.assertRaises(AssertionError):
                check(report, selected, pins, raw, 'regexredux')
        report, selected, pins, raw = retained_fixture('regexredux')
        build = str(retained.PRIOR/'regexredux'/report['profiles'][0]['id']/'build.json')
        raw[build]['command'] = list(raw[build]['command']) + ['-fast']
        with self.assertRaises(AssertionError):
            check(report, selected, pins, raw, 'regexredux')

    def test_proven_probe_oracle_and_owned_controller_functions_are_unchanged(self):
        def functions(path):
            return {node.name:ast.dump(node,include_attributes=False)
                    for node in ast.parse(path.read_text()).body if isinstance(node,ast.FunctionDef)}
        original = functions(worker.HERE/'prepare_input_v2.py')
        changed = functions(worker.HERE/'prepare_builder_v3.py')
        self.assertEqual(set(original),set(changed))
        for name in ('read_additive_prerequisites','exact_oracle','probe','input_cases','main'):
            self.assertEqual(original[name],changed[name])
        source = (worker.HERE/'prepare_builder_v3_cohort.py').read_text()
        self.assertIn('owned_preparation = base.owned_preparation',source)
        self.assertIn("if item not in ('regexredux','revcomp')",source)
        self.assertNotIn('subprocess.Popen',source)


def main():
    global FD
    module = full_study.configure('pidigits')
    print('WAIT common mutex: six V builder-continuation metadata fixtures',flush=True)
    with module.LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        FD = lock.fileno()
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output,verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(BuilderChecks))
        text = output.getvalue()
        print(text,end='',flush=True)
        log = worker.HERE/'builder-v3-fixtures.log'
        with log.open('x') as stream:stream.write(text)
        receipt = worker.HERE/'builder-v3-fixtures.json'
        assert not receipt.exists()
        paths=[Path(__file__).resolve(),worker.HERE/'prepare_builder_v3.py',
            worker.HERE/'prepare_builder_v3_cohort.py',worker.HERE/'retained_builder_v3.py',
            worker.HERE/'prepare_input_v2.py',worker.HERE/'prepare_input_v2_cohort.py',
            worker.HERE/'prepare_cohort.py',worker.DRAFT_PATH,worker.PLAN_PATH,
            worker.HERE/'revisions/pidigits-builder-v3/sources/pidigits/optimized.v',log]
        module.save(receipt,dict(status='passed' if result.wasSuccessful() else 'failed',
            tests_run=result.testsRun,failures=len(result.failures),errors=len(result.errors),
            native_probes=0,actual_signals_sent=0,full_performance_runs=0,never_chart_eligible=True,
            source_revision='builder-continuation-v3',
            pins={str(path):module.sha(path) for path in paths}))
        if not result.wasSuccessful() or result.testsRun!=6:raise SystemExit(1)


if __name__=='__main__':main()

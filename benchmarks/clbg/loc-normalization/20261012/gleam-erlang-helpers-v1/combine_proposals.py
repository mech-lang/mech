"""Bind complete mixed authored-source proposals; semantic admission is separate."""
import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import sys
import traceback

LOCK = Path('/tmp/mech-clbg-timing.lock')
EXPECTED_HELPER_IDS = ['named-header-and-value','inline-first-callback','expanded-first-callback','independent-callback-effect',
    'two-real-callbacks','inline-receive-arm-body-close','expanded-receive-arm-body-close','nested-receive-two-outer-arms',
    'packed-case-pattern-and-data','independent-case-arm-effect','runner-try-catch-two-effects','literal-keywords-and-separators',
    'wrapped-data-retains-physical-row','independent-chained-matches','additional-anonymous-clause','unknown-authored-include',
    'actual-parser-error','unknown-comprehension-owner','earlier-effect-before-callback','intervening-effect-between-callbacks',
    'written-selector-matches-remain-gated']


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def verify(pins):
    for path,digest in pins.items(): assert sha(path) == digest, ('Changed count evidence',path)


def merge(pins,group):
    for path,digest in group.items():
        assert path not in pins or pins[path] == digest, ('Conflicting count evidence',path)
        pins[path] = digest


def save_new(path,value):
    with Path(path).open('x') as stream: json.dump(value,stream,indent=2); stream.write('\n')


def combined(args,pins):
    assert sha(args.gleam) == args.gleam_sha256 and sha(args.erlang) == args.erlang_sha256
    gleam,erlang = json.loads(args.gleam.read_text()),json.loads(args.erlang.read_text())
    assert gleam['status'] == 'actual20-source-proposals-awaiting-Erlang-count-and-semantic-review'
    assert erlang['status'] == 'original-authored-Erlang-helper-count-proposals-awaiting-semantic-review'
    assert gleam['qualification'] == erlang['qualification']
    assert gleam['actual_common_mutex_held'] is erlang['actual_common_mutex_held'] is True
    for doc in (gleam,erlang):
        assert doc['canonical_LOC_admitted'] is doc['chart_admission'] is False
        merge(pins,doc['pins']); verify(doc['pins'])
    merge(pins,{str(args.gleam):args.gleam_sha256,str(args.erlang):args.erlang_sha256})
    for report,expected_status in ((args.gleam,'completed-source-proposals'),(args.erlang,'completed-helper-count-proposals')):
        terminal_path = report.with_name('terminal-pins.json')
        terminal = json.loads(terminal_path.read_text())
        assert terminal['status'] == expected_status and terminal['actual_common_mutex_held'] is True
        assert terminal['pins'][str(report)] == sha(report)
        verify(terminal['pins']); verify(terminal['prerequisite_pins'])
        merge(pins,terminal['pins']); merge(pins,terminal['prerequisite_pins'])
        merge(pins,{str(terminal_path):sha(terminal_path)})
    fixture = erlang['fixture_receipt']; path = Path(fixture['path'])
    assert sha(path) == fixture['sha256'] == pins[str(path)]
    actual = json.loads(path.read_text())
    assert actual['status'] == 'passed-original-source-owner-fixtures' and type(actual['actual_cases']) is int and actual['actual_cases'] == 21
    assert actual['expected_ids'] == actual['started_ids'] == EXPECTED_HELPER_IDS
    assert [item['id'] for item in actual['raw_case_receipts']] == EXPECTED_HELPER_IDS
    assert all(type(actual[name]) is int and actual[name] == 0 for name in ('failures','errors','skips'))
    assert actual['legacy_sequence_metric_preserved_separately'] is True
    legacy = actual['legacy_fixture_receipt']; legacy_path = Path(legacy['path'])
    assert sha(legacy_path) == legacy['sha256'] == pins[str(legacy_path)]
    legacy_doc = json.loads(legacy_path.read_text())
    assert legacy_doc['status'] == 'passed' and type(legacy_doc['actual_cases']) is int and legacy_doc['actual_cases'] == 5
    assert legacy_doc['expected_ids'] == legacy_doc['started_ids'] == ['independent-bindings','tuple-data','call-arguments','list-data-and-comment','nested-block']
    assert legacy_doc['metric'] == 'preserved-legacy-expression-sequence-metric-not-relabelled-as-new-owner-metric'
    assert all(type(legacy_doc[name]) is int and legacy_doc[name] == 0 for name in ('failures','errors','skips'))
    counts = dict(gleam['source_counts'])
    pending = {path for path,value in counts.items() if value['source_language'] == 'erl'}
    assert pending == set(erlang['source_counts'])
    for path,value in erlang['source_counts'].items():
        prior = counts[path]
        assert value['source_sha256'] == prior['source_sha256'] == pins[path]
        assert value['origin'] == prior['origin'] and value['role'] == prior['role'] == 'application-helper'
        assert value['metric']['generated_source_substitute'] is False
        assert value['metric']['original_source'] == value['origin']
        assert value['metric']['original_source_sha256'] == prior['source_sha256'] == pins[value['origin']]
        assert type(value['metric']['normalized_loc']) is int and value['metric']['normalized_loc'] > 0
        counts[path] = dict(prior,metric=value['metric'],count_receipt=dict(path=str(args.erlang),sha256=args.erlang_sha256))
    profiles = {}
    assert type(gleam['profile_count']) is int and gleam['profile_count'] == len(gleam['profiles']) == 20
    assert set(gleam['compiler_bound_input_inventory']) == set(gleam['profiles'])
    for identifier,row in gleam['profiles'].items():
        assert row['id'] == identifier
        source_bundle = []
        inventory = gleam['compiler_bound_input_inventory'][identifier]
        assert {(item['path'],item['sha256'],item['role']) for item in row['source_bundle']} == {
            (item['path'],item['sha256'],item['role']) for item in inventory['source_bundle']}
        for item in row['source_bundle']:
            value = counts[item['path']]
            assert item['sha256'] == value['source_sha256'] == pins[item['path']]
            assert item['role'] == value['role'] and item['origin'] == value['origin']
            metric = value['metric']
            assert type(metric['normalized_loc']) is int and metric['normalized_loc'] > 0
            assert type(metric['physical_loc']) is int and metric['physical_loc'] > 0
            assert not metric.get('diagnostics')
            source_bundle.append(dict(item,normalized_loc=metric['normalized_loc'],physical_loc=metric['physical_loc'],diagnostics=[]))
        profiles[identifier] = dict(row,source_bundle=source_bundle,canonical_loc=None,
            proposed_complete_bundle_units=sum(item['normalized_loc'] for item in source_bundle),source_language_counts_complete=True,
            independent_ownership_review_passed=False,independent_semantic_review_passed=False)
    verify(pins)
    return dict(status='complete20-authored-source-count-proposals-awaiting-independent-semantic-review',
        qualification=gleam['qualification'],profiles=profiles,profile_count=20,source_counts=counts,
        compiler_bound_input_inventory=gleam['compiler_bound_input_inventory'],
        count_receipts=dict(gleam=dict(path=str(args.gleam),sha256=args.gleam_sha256),erlang=dict(path=str(args.erlang),sha256=args.erlang_sha256)),
        pins=dict(pins),actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False,
        full_performance_runs=0,source_or_timing_mutated=False,count_override=False)


def combine(args):
    print('WAIT common mutex: complete original Gleam/Erlang count proposals',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        descriptor,named = os.fstat(lock.fileno()),LOCK.stat()
        assert (descriptor.st_dev,descriptor.st_ino) == (named.st_dev,named.st_ino)
        assert not args.output.exists()
        evidence = args.output.with_suffix('.evidence')
        assert not evidence.exists(); evidence.mkdir(parents=True)
        pins = {}; status = 'failed-stop'
        try:
            merge(pins,{str(path):sha(path) for path in (Path(__file__).resolve(),Path(__file__).with_name('REVIEW.md'),Path(sys.executable).resolve())})
            result = combined(args,pins)
            save_new(args.output,result); status = 'completed-complete-bundle-proposals'
        except BaseException as error:
            with (evidence/'exception.log').open('x') as stream: stream.write(traceback.format_exc())
            save_new(evidence/'failure.json',dict(status='failed-stop',error=str(error) or type(error).__name__,error_type=type(error).__name__,
                actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))
            raise
        finally:
            captured = {}
            try:
                verify(pins)
                paths = sorted(path for path in evidence.rglob('*') if path.is_file())
                if args.output.is_file(): paths.append(args.output)
                for path in paths: captured[str(path)] = sha(path)
            except BaseException as error:
                save_new(evidence/'terminal-pins.json',dict(status='failed-terminal-integrity',preceding_status=status,
                    error=str(error) or type(error).__name__,pins=captured,actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))
                raise
            save_new(evidence/'terminal-pins.json',dict(status=status,pins=captured,prerequisite_pins=pins,
                actual_common_mutex_held=True,canonical_LOC_admitted=False,chart_admission=False))
        print(json.dumps(dict(path=str(args.output),sha256=sha(args.output),profiles=20,canonical_LOC_admitted=False)),flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--gleam',type=Path,required=True); parser.add_argument('--gleam-sha256',required=True)
    parser.add_argument('--erlang',type=Path,required=True); parser.add_argument('--erlang-sha256',required=True)
    parser.add_argument('--output',type=Path,required=True)
    args = parser.parse_args()
    args.gleam,args.erlang,args.output = (path.resolve() for path in (args.gleam,args.erlang,args.output))
    combine(args)


if __name__ == '__main__': main()

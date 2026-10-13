"""Metadata only: bind Crystal display claims to actual immutable receipts."""
import argparse
from copy import deepcopy
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
COHORT = CLBG / 'crystal-comparison/20261012'
SUITE = COHORT / 'reconciliation/full-v1-suite-v2.json'
COUNTS = COHORT / 'source-counts/corrected-qualified-overlay-v2/qualified-loc-overlay.json'
COUNTS_SHA = 'e22140622a5154a1e3b9512580eb469c8c078beda4a2ae9812bf429e51e089e2'
INDEX = COHORT / 'studies/full-v1/index.json'
INDEX_SHA = 'f8b3f9f20265245461020f112bf0c7b2bd880c702f726c246ccf68a480147476'
IMPORTER = CLBG / 'overview/explorer/native_languages_data_v1.py'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def merge(pins, values):
    for path, value in values.items():
        assert path not in pins or pins[path] == value, ('Conflicting evidence', path)
        pins[path] = value


def verify(pins):
    for path, value in pins.items():
        assert sha(path) == value, ('Actual evidence changed', path)


def receipt(path, command, pins):
    assert str(path) in pins and sha(path) == pins[str(path)]
    value = json.loads(path.read_text())
    assert value['command'] == command and value['inherited_common_mutex'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(name) for name in ('error','cleanup_error','cleanup_errors','remaining_group_members'))
    return value


def qualify(args, output, pins):
    descriptor_path = HERE / 'descriptor.json'
    assert sha(descriptor_path) == args.descriptor_sha256 and sha(SUITE) == args.suite_sha256
    assert sha(COUNTS) == COUNTS_SHA and sha(INDEX) == INDEX_SHA
    descriptor = json.loads(descriptor_path.read_text())
    assert descriptor['status'] == 'source-reviewed-descriptor-awaiting-guarded-actual-evidence-qualification'
    assert descriptor['language'] == 'crystal' and descriptor['execution'] == 'AOT'
    assert all(descriptor[name] is False for name in ('actual_compiler_version_verified',
        'actual_native_architecture_verified','actual_memory_policy_verified'))
    suite, counts, index = (json.loads(path.read_text()) for path in (SUITE,COUNTS,INDEX))
    assert suite['language'] == 'crystal' and counts['language'] == 'crystal'
    merge(pins, suite['provenance']); merge(pins, counts['pins'])
    merge(pins, {str(descriptor_path):args.descriptor_sha256, str(SUITE):args.suite_sha256,
        str(COUNTS):COUNTS_SHA, str(INDEX):INDEX_SHA, str(IMPORTER):args.importer_sha256})
    verify(pins)
    preparation_path = Path(index['qualified_preparation'])
    assert sha(preparation_path) == index['qualified_preparation_sha256']
    preparation = json.loads(preparation_path.read_text())
    assert preparation['pins_unchanged_at_end'] is True and preparation['active'] is None
    merge(pins, preparation['pins']); merge(pins,{str(preparation_path):index['qualified_preparation_sha256']})
    expected = descriptor['expected_evidence']
    version = CLBG / expected['compiler_version_stdout']
    version_receipt = CLBG / expected['compiler_version_receipt']
    command = json.loads(version_receipt.read_text())['command']
    assert len(command) == 2 and command[1] == '--version'
    receipt(version_receipt,command,pins)
    assert version.read_text() == expected['actual_version_text'] and sha(version) == pins[str(version)]
    compiler = command[0]
    assert sha(compiler) == pins[compiler]
    release = Path(compiler).parents[2]
    gc = CLBG / expected['gc_selection_source']
    assert gc == release / 'src/gc.cr'
    assert sha(gc) == pins[str(gc)]
    assert gc.read_text().rstrip().endswith('{% if flag?(:gc_none) || flag?(:wasm32) %}\n  require "gc/none"\n{% else %}\n  require "gc/boehm"\n{% end %}')
    boehm, archive, libtool = release / 'src/gc/boehm.cr', release / 'embedded/lib/libgc.a', release / 'embedded/lib/libgc.la'
    for path in (boehm,archive,libtool):
        assert str(path) in pins and sha(path) == pins[str(path)]
    assert "dlname=''" in libtool.read_text() and "old_library='libgc.a'" in libtool.read_text()
    proof_rows = {}
    for benchmark, entry in suite['benchmarks'].items():
        manifest_path, report_path = Path(entry['manifest']), Path(entry['report'])
        assert sha(manifest_path) == entry['manifest_sha256'] == index['benchmarks'][benchmark]['sha256']
        assert sha(report_path) == entry['report_sha256']
        manifest, report = json.loads(manifest_path.read_text()), json.loads(report_path.read_text())
        assert report['status'] == 'complete' and report['pins_unchanged_at_end'] is True
        # Original frozen timing reports have no top-level pins member. Their
        # exact bytes and raw/supervisor records are independently suite-pinned.
        merge(pins,manifest['pins'])
        assert pins[str(report_path)] == entry['report_sha256']
        for attempt in report['attempts']:
            for field in ('result_path','supervisor_result_path'):
                path = Path(attempt[field]); assert path.is_file()
                assert str(path) in suite['provenance'] and sha(path) == suite['provenance'][str(path)]
        for identity,row in entry['profiles'].items():
            if row['language'] != 'crystal': continue
            assert identity == row['id'] and identity not in proof_rows
            invref = counts['compiler_bound_input_inventory'][identity]
            assert invref['source_sha256'] == row['source_sha256'] and invref['binary_sha256'] == row['binary_sha256']
            evidence_path = Path(invref['evidence_path'])
            assert sha(evidence_path) == invref['evidence_sha256'] == pins[str(evidence_path)]
            evidence = json.loads(evidence_path.read_text())
            build_path = Path(evidence['actual_build_receipt']['path']); build_command = evidence['actual_build_command']
            assert evidence['actual_build_receipt']['sha256'] == pins[str(build_path)]
            receipt(build_path,build_command,pins)
            assert build_command[0] == compiler and build_command[1] == 'build'
            assert build_command[2:12] == ['--release','--no-debug','--no-color','--error-trace','--mcpu','native','--threads','1','--link-flags',build_command[11]]
            assert build_command[-3:] == ['-o',row['binary'],row['source']]
            assert not re.search(r'gc_none|wasm32|GC_DISABLE|DYLD_INSERT', ' '.join(build_command))
            assert not any(re.search(r'gc_disable|gc_none|gc_dont_gc|dyld_insert', key, re.I) for key in row['runtime_environment'])
            assert 'GC.disable' not in Path(row['source']).read_text()
            images = build_path.parent / 'images'
            receipt(images/'architecture.json',['/usr/bin/lipo','-archs',row['binary']],pins)
            assert (images/'architecture.stdout').read_text().split() == ['arm64']
            receipt(images/'linked-images.json',['/usr/bin/otool','-L',row['binary']],pins)
            image = json.loads((images/'dependency-review.json').read_text())
            assert image['binary'] == row['binary'] and image['architectures'] == ['arm64']
            assert [item['install_name'] for item in image['dependencies']] == [line.strip().split(' (',1)[0]
                for line in (images/'linked-images.stdout').read_text().splitlines()[1:]]
            if benchmark == 'regexredux':
                assert any(item.get('path','').startswith('/opt/homebrew/Cellar/pcre2/10.49/') for item in image['dependencies'])
            if benchmark == 'pidigits':
                assert any(item.get('path','').startswith('/opt/homebrew/Cellar/gmp/6.3.0/') for item in image['dependencies'])
            proof_rows[identity] = dict(source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],
                compiler_input_inventory=invref,actual_build_receipt=evidence['actual_build_receipt'],
                architecture_receipt=str(images/'architecture.json'),image_receipt=str(images/'linked-images.json'))
    assert len(proof_rows) == 20 and set(proof_rows) == set(counts['profiles'])
    verify(pins)
    runtime_path = output / 'toolchain-runtime-receipt.json'
    save(runtime_path,dict(status='verified-actual-Crystal-compiler-runtime-and-memory',
        actual_compiler_version_stdout=version.read_text(),actual_version_receipt=dict(path=str(version_receipt),sha256=pins[str(version_receipt)]),
        actual_version_stdout=dict(path=str(version),sha256=pins[str(version)]),profiles=proof_rows,
        actual_memory_selection='default gc/boehm with pinned static libgc.a; all20 no-disable build/source/environment checks',
        memory_strategies=['gc'],native_architecture='arm64',pins=pins))
    merge(pins,{str(runtime_path):sha(runtime_path)})
    metadata = deepcopy(descriptor)
    metadata.update(status='reviewed-actual-native-language-metadata',actual_compiler_version_verified=True,
        actual_native_architecture_verified=True,actual_memory_policy_verified=True,memory_strategies=['gc'],pins=pins)
    metadata['measured_backend_template'].update(verified=True)
    metadata['measured_backend_template']['runtime_receipt'].update(verified=True,report=str(runtime_path),report_sha256=pins[str(runtime_path)])
    assert sha(IMPORTER) == args.importer_sha256
    sys.dont_write_bytecode = True
    name = 'crystal_publication_exact_cohort_validator'; assert name not in sys.modules
    spec = importlib.util.spec_from_file_location(name,IMPORTER); validator=importlib.util.module_from_spec(spec)
    sys.modules[name]=validator; spec.loader.exec_module(validator)
    assert Path(validator.__file__).resolve() == IMPORTER and sha(IMPORTER) == args.importer_sha256
    assert len(validator.validate_cohort('crystal',suite,counts,metadata)) == 20
    verify(pins)
    save(output/'metadata.json',metadata)


def main():
    parser=argparse.ArgumentParser()
    for name in ('descriptor-sha256','suite-sha256','importer-sha256','revision'): parser.add_argument('--'+name,required=True)
    args=parser.parse_args()
    assert re.fullmatch(r'[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch(r'[0-9a-f]{64}',getattr(args,name)) for name in ('descriptor_sha256','suite_sha256','importer_sha256'))
    print('WAIT common mutex: Crystal publication metadata only',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        held,named=os.fstat(lock.fileno()),LOCK.stat(); assert (held.st_dev,held.st_ino)==(named.st_dev,named.st_ino)
        output=HERE/'qualified'/args.revision; assert not output.exists(); output.mkdir(parents=True)
        pins={}; outcome=dict(status='failed-metadata',chart_admission=False,native_subjects=0)
        try:
            pins[str(Path(__file__).resolve())]=sha(__file__)
            qualify(args,output,pins); outcome['status']='metadata-qualified-awaiting-root-whole-six-review'
        except BaseException as error:
            outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            (output/'actual-error.log').write_text(traceback.format_exc()); raise
        finally:
            available={}
            try:
                for path in output.rglob('*'):
                    if path.is_file(): available[str(path)]=sha(path)
            except BaseException as error:
                outcome.update(status='failed-terminal-integrity',error=str(error),error_type=type(error).__name__,available_bytes_not_fully_pinned=True)
                outcome['available_evidence_pins']=available; save(output/'outcome.json',outcome); raise
            else:
                outcome['available_evidence_pins']=available; save(output/'outcome.json',outcome)
        print(json.dumps(dict(status=outcome['status'],metadata=str(output/'metadata.json'),sha256=sha(output/'metadata.json'))),flush=True)


if __name__=='__main__': main()

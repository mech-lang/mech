"""Metadata only: bind Nim C/Clang/ORC claims to exact saved receipts."""
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
COHORT = CLBG / 'nim-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
IMPORTER = CLBG / 'overview/explorer/native_languages_data_v1.py'
BENCHMARKS = {'regexredux','revcomp','pidigits','spectralnorm','mandelbrot',
              'fasta','knucleotide','binarytrees','nbody','fannkuchredux'}


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as stream:
        json.dump(value,stream,indent=2); stream.write('\n')


def merge(pins,values):
    for path,digest in values.items():
        assert type(path) is str and Path(path).is_absolute() and re.fullmatch('[0-9a-f]{64}',digest)
        assert path not in pins or pins[path] == digest, ('Conflicting actual evidence',path)
        pins[path] = digest


def verify(pins):
    for path,digest in pins.items():
        assert sha(path) == digest, ('Actual evidence changed',path)


def pinned_json(path,pins):
    path = Path(path)
    assert str(path) in pins and sha(path) == pins[str(path)], ('Unbound actual receipt',str(path))
    return json.loads(path.read_text())


def pinned_text(path,pins):
    path = Path(path)
    assert str(path) in pins and sha(path) == pins[str(path)], ('Unbound actual output',str(path))
    return path.read_text()


def owned_receipt(path,command,pins):
    value = pinned_json(path,pins)
    assert value['command'] == command
    assert value['inherited_common_mutex'] is value['preparation_only'] is value['never_chart_eligible'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(key) for key in ('error','cleanup_error','cleanup_errors',
        'remaining_group_members','output_drain_error'))
    return value


def version_proof(compiler,root,expected,pins):
    receipt_path,stdout,stderr = (root/'bootstrap'/('nim-version'+suffix) for suffix in ('.json','.stdout','.stderr'))
    owned_receipt(receipt_path,[compiler,'--version'],pins)
    assert str(stdout) in pins and sha(stdout) == pins[str(stdout)] and stdout.read_text() == expected
    assert str(stderr) in pins and sha(stderr) == pins[str(stderr)] and stderr.read_bytes() == b''
    assert sha(compiler) == pins[compiler]
    return dict(compiler=compiler,compiler_sha256=pins[compiler],receipt=str(receipt_path),
        receipt_sha256=pins[str(receipt_path)],stdout=str(stdout),stdout_sha256=pins[str(stdout)])


def memory_proof(command,evidence,row,pins):
    toolchain = Path(command[0]).parent.parent
    assert command.count('--mm:orc') == 1
    assert [value for value in command if value.startswith(('--mm:','--gc:'))] == ['--mm:orc']
    assert '--lib:'+str(toolchain/'lib') in command and '--threads:on' in command
    assert all(flag in command for flag in ('--skipUserCfg:on','--skipParentCfg:on','--skipProjCfg:on'))
    assert not any(re.search(r'(?i)(?:gc:none|mm:none|gc:arc|mm:arc|gcnone|nogc|gcdisable)',value) for value in command)
    inputs = evidence['processing_evidence']
    assert inputs['complete_import_and_include_events'] is True and inputs['unknown_roots'] == []
    assert inputs['generated_source_substitute'] is False
    actual_files = {item['resolved_path']:item for item in inputs['events']}
    paths = [toolchain/'lib/system.nim',toolchain/'lib/system/arc.nim',toolchain/'lib/system/orc.nim']
    for path in paths:
        assert str(path.resolve()) in actual_files and str(path) in pins and sha(path) == pins[str(path)]
    system,arc,orc = (path.read_text() for path in paths)
    assert 'include system/arc' in system and 'when defined(nimV2):' in system
    assert 'when defined(gcOrc):' in arc and 'include orc' in arc
    assert 'proc nimNewObj(' in arc and 'proc nimIncRefCyclic(' in orc
    assert 'Cycle collector based on' in orc and 'nimDecRefIsLastCyclic' in orc
    if 'runtime_environment' in evidence:
        assert row['runtime_environment'] == evidence['runtime_environment']
    assert not any(re.search(r'(?i)gc_disable|gc_none|dyld_insert',key) for key in row['runtime_environment'])
    for item in evidence['source_bundle']:
        assert sha(item['path']) == item['sha256'] == pins[item['path']]
        assert not re.search(r'\b(?:GC_disable|GC_disableMarkAndSweep)\b',Path(item['path']).read_text())
    return dict(selection='--mm:orc',strategies=['rc','gc'],loaded_memory_sources=[
        dict(path=str(path),sha256=pins[str(path)]) for path in paths],
        original_processing_event_binding=True,original_build_selection=True)


def qualify(args,output,pins):
    descriptor_path = HERE/'descriptor.json'
    assert sha(__file__) == args.qualifier_sha256 and sha(descriptor_path) == args.descriptor_sha256
    descriptor = json.loads(descriptor_path.read_text()); expected = descriptor['expected_evidence']
    assert descriptor['language'] == 'nim' and descriptor['execution'] == 'AOT'
    assert descriptor['status'] == 'source-reviewed-descriptor-awaiting-guarded-actual-evidence-qualification'
    assert all(descriptor[field] is False for field in ('actual_compiler_version_verified',
        'actual_native_architecture_verified','actual_memory_policy_verified'))
    paths = {name:Path(getattr(args,name)).resolve() for name in ('suite','canonical_loc','full_index')}
    assert all(path.is_relative_to(COHORT) for path in paths.values())
    for name,path in paths.items():
        digest = getattr(args,name+'_sha256'); assert sha(path) == digest
        merge(pins,{str(path):digest})
    suite,counts,index = (json.loads(paths[name].read_text()) for name in ('suite','canonical_loc','full_index'))
    assert suite['language'] == counts['language'] == 'nim'
    assert set(suite['benchmarks']) == set(index['benchmarks']) == BENCHMARKS
    assert type(index['samples']) is int and index['samples'] == 3
    assert type(index['full_processes']) is int and index['full_processes'] == 90 and index['chart_admission'] is False
    merge(pins,suite['provenance']); merge(pins,counts['pins'])
    merge(pins,{str(descriptor_path):args.descriptor_sha256,str(Path(__file__).resolve()):args.qualifier_sha256,
        str(HERE/'REVIEW.md'):sha(HERE/'REVIEW.md'),str(IMPORTER):args.importer_sha256,
        str(Path(sys.executable).resolve()):sha(Path(sys.executable).resolve())})
    work_path = Path(index['work_qualification']); assert sha(work_path) == index['work_qualification_sha256']
    work = json.loads(work_path.read_text())
    assert work['status'] == 'source-work-allocation-passed-awaiting-full-review-and-LOC'
    assert work['active'] is None and work['pins_unchanged_at_end'] is True and work['chart_admission'] is False
    merge(pins,work['pins']); merge(pins,{str(work_path):index['work_qualification_sha256']})
    native_path = CLBG/expected['native_qualification']
    assert sha(native_path) == expected['native_qualification_sha256'] == work['prerequisite_sha256']
    assert Path(work['prerequisite']) == native_path
    native = json.loads(native_path.read_text())
    assert native['proposal_sha256'] == expected['native_proposal_sha256']
    assert native['active'] is None and len(native['profiles']) == 20 and len(native['proofs']) == 24
    assert len(native['attempts']) == 20 and native['reused_bounded_cases'] == 154
    assert native['chart_admission'] is False and native['full_performance_runs'] == 0
    merge(pins,native['pins']); merge(pins,{str(native_path):expected['native_qualification_sha256']})
    native_rows = {row['id']:row for row in native['profiles']}; assert len(native_rows) == 20
    verify(pins)
    # This same executable's version command was already owned and saved by the
    # Crystal preparation. Capture that evidence now; do not relabel its history.
    clang_stdout,clang_receipt = (CLBG/expected[field] for field in ('clang_version_stdout','clang_version_receipt'))
    clang_stderr = clang_stdout.with_suffix('.stderr')
    clang_paths = (clang_stdout,clang_receipt,clang_stderr)
    newly_captured = {str(path):sha(path) for path in clang_paths}
    merge(pins,newly_captured)
    owned_receipt(clang_receipt,[expected['clang'],'--version'],pins)
    assert clang_stdout.read_text() == expected['actual_clang_version_text'] and clang_stderr.read_bytes() == b''
    assert sha(expected['clang']) == pins[expected['clang']]
    profiles = {}; compiler_proofs = {}
    for benchmark,entry in suite['benchmarks'].items():
        manifest_path,report_path = Path(entry['manifest']),Path(entry['report'])
        assert sha(manifest_path) == entry['manifest_sha256'] == index['benchmarks'][benchmark]['sha256']
        assert Path(index['benchmarks'][benchmark]['manifest']) == manifest_path
        assert sha(report_path) == entry['report_sha256'] == pins[str(report_path)]
        manifest,report = json.loads(manifest_path.read_text()),json.loads(report_path.read_text())
        assert report['status'] == 'complete' and report['pins_unchanged_at_end'] is True
        merge(pins,manifest['pins'])
        for attempt in report['attempts']:
            for key in ('result_path','supervisor_result_path'):
                path = Path(attempt[key]); assert str(path) in suite['provenance']
                assert sha(path) == suite['provenance'][str(path)]
        for identifier,row in entry['profiles'].items():
            if row['language'] != 'nim': continue
            assert row['id'] == identifier and identifier not in profiles
            original = native_rows[identifier]
            assert all(row[key] == original[key] for key in ('source','source_sha256','binary_sha256','runtime_environment'))
            assert row['binary'] == original['command'][0] and row['command'] == original['command']
            assert sha(row['source']) == row['source_sha256'] == pins[row['source']]
            assert sha(row['binary']) == row['binary_sha256'] == pins[row['binary']]
            invref = counts['compiler_bound_input_inventory'][identifier]
            assert invref['source_sha256'] == row['source_sha256'] and invref['binary_sha256'] == row['binary_sha256']
            evidence_path = Path(invref['evidence_path'])
            assert sha(evidence_path) == invref['evidence_sha256'] == pins[str(evidence_path)]
            evidence = json.loads(evidence_path.read_text())
            assert evidence['id'] == identifier and evidence['source'] == row['source'] and evidence['binary'] == row['binary']
            assert evidence['original_semantic_flags_preserved'] is True and evidence['original_binary_rebuilt'] is False
            assert evidence['compiler_frontend_only'] is True and evidence['actual_common_mutex_held'] is True
            command = evidence['original_build_command']; build = evidence['original_build_receipt']
            assert command[1] == 'c' and command[-1] == row['source'] and '--out:'+row['binary'] in command
            build_path = Path(build['path']); assert build['sha256'] == pins[str(build_path)]
            owned_receipt(build_path,command,pins)
            for suffix in ('.stdout','.stderr'):
                pinned_text(build_path.with_suffix(suffix),pins)
            for flag in ('--cc:clang','--clang.exe:'+expected['clang'],'--clang.linkerexe:'+expected['clang'],'--passC:-ffp-contract=off'):
                assert flag in command
            assert not any('fast-math' in value for value in command)
            root = Path(row['source']).parents[3]
            assert build_path == root/'builds'/identifier/'build.json'
            compiler_proofs[command[0]] = version_proof(command[0],root,expected['actual_nim_version_text'],pins)
            selected_memory = memory_proof(command,evidence,row,pins)
            images = build_path.parent/'images'
            owned_receipt(images/'architecture.json',['/usr/bin/lipo','-archs',row['binary']],pins)
            assert pinned_text(images/'architecture.stdout',pins).split() == ['arm64']
            owned_receipt(images/'linked-images.json',['/usr/bin/otool','-L',row['binary']],pins)
            image = pinned_json(images/'dependency-review.json',pins)
            assert image['binary'] == row['binary'] and image['architectures'] == ['arm64']
            assert [item['install_name'] for item in image['dependencies']] == [line.strip().split(' (',1)[0]
                for line in pinned_text(images/'linked-images.stdout',pins).splitlines()[1:]]
            for dependency in image['dependencies']:
                if 'path' in dependency:
                    assert dependency['sha256'] == pins[dependency['path']] == sha(dependency['path'])
            assert any(item.get('path','').startswith('/opt/homebrew/Cellar/pcre/8.45/') for item in image['dependencies'])
            assert any(item.get('path','').startswith('/opt/homebrew/Cellar/gmp/6.3.0/') for item in image['dependencies'])
            assert {(item['path'],item['sha256']) for item in evidence['source_bundle']} == {(item['path'],item['sha256']) for item in invref['files']}
            profiles[identifier] = dict(source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],
                runtime_environment=row['runtime_environment'],compiler_input_inventory=invref,
                original_build_receipt=build,actual_memory_selection=selected_memory,
                architecture_receipt=str(images/'architecture.json'),image_receipt=str(images/'linked-images.json'))
    assert len(profiles) == 20 and set(profiles) == set(counts['profiles']) == set(native_rows)
    verify(pins)
    runtime_path = output/'toolchain-runtime-receipt.json'
    save(runtime_path,dict(status='verified-actual-Nim-C-Clang-ORC-compiler-runtime-and-memory',
        actual_nim_version_stdout=expected['actual_nim_version_text'],actual_clang_version_stdout=clang_stdout.read_text(),
        original_compiler_proofs=compiler_proofs,clang=dict(executable=expected['clang'],sha256=pins[expected['clang']],
            saved_receipt=str(clang_receipt),saved_stdout=str(clang_stdout),newly_captured_saved_receipt_pins=newly_captured,
            recorded_in_original_Nim_preparation_pins=False),profiles=profiles,memory_strategies=['rc','gc'],
        native_architecture='arm64',pins=dict(pins)))
    merge(pins,{str(runtime_path):sha(runtime_path)})
    metadata = deepcopy(descriptor)
    metadata.update(status='reviewed-actual-native-language-metadata',actual_compiler_version_verified=True,
        actual_native_architecture_verified=True,actual_memory_policy_verified=True,memory_strategies=['rc','gc'],pins=dict(pins))
    metadata['measured_backend_template']['verified'] = True
    metadata['measured_backend_template']['runtime_receipt'].update(verified=True,report=str(runtime_path),report_sha256=pins[str(runtime_path)])
    assert sha(IMPORTER) == args.importer_sha256
    name = 'nim_publication_exact_cohort_validator'; assert name not in sys.modules
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location(name,IMPORTER); validator = importlib.util.module_from_spec(spec)
    sys.modules[name] = validator; spec.loader.exec_module(validator)
    assert Path(validator.__file__).resolve() == IMPORTER and sha(IMPORTER) == args.importer_sha256
    assert len(validator.validate_cohort('nim',suite,counts,metadata)) == 20
    verify(pins); save(output/'metadata.json',metadata)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','qualifier-sha256','descriptor-sha256','suite','suite-sha256','canonical-loc',
                 'canonical-loc-sha256','full-index','full-index-sha256','importer-sha256'):
        parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch('[0-9a-f]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    print('WAIT common mutex: Nim actual publication metadata only',flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        held,named = os.fstat(lock.fileno()),LOCK.stat(); assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        output = HERE/'qualified'/args.revision; assert not output.exists(); output.mkdir(parents=True)
        pins = {}; outcome = dict(status='failed-metadata',chart_admission=False,native_subjects=0)
        try:
            qualify(args,output,pins); outcome['status'] = 'metadata-qualified-awaiting-root-whole-six-review'
        except BaseException as error:
            outcome.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
            with (output/'actual-error.log').open('x') as stream: stream.write(traceback.format_exc())
            raise
        finally:
            available = {}
            try:
                for path in output.rglob('*'):
                    if path.is_file(): available[str(path)] = sha(path)
                verify(pins)
            except BaseException as error:
                outcome.update(status='failed-terminal-integrity',terminal_error=str(error),terminal_error_type=type(error).__name__,
                    available_bytes_not_fully_pinned=True)
                outcome['available_evidence_pins'] = available; outcome['prerequisite_pins'] = pins
                save(output/'outcome.json',outcome); raise
            else:
                outcome['available_evidence_pins'] = available; outcome['prerequisite_pins'] = pins
                save(output/'outcome.json',outcome)
        print(json.dumps(dict(status=outcome['status'],metadata=str(output/'metadata.json'),sha256=sha(output/'metadata.json'))),flush=True)


if __name__ == '__main__': main()

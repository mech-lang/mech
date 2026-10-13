"""Metadata only: reconcile original C3 LLVM/ARM/manual-memory evidence."""
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
COHORT = CLBG/'c3-comparison/20261012'
LOCK = Path('/tmp/mech-clbg-timing.lock')
IMPORTER = CLBG/'overview/explorer/native_languages_data_v1.py'
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
        assert sha(path) == digest, ('Changed actual evidence',path)


def pinned_text(path,pins):
    path = Path(path)
    assert str(path) in pins and sha(path) == pins[str(path)], ('Unbound actual output',str(path))
    return path.read_text()


def pinned_json(path,pins):
    return json.loads(pinned_text(path,pins))


def owned_receipt(path,command,pins):
    value = pinned_json(path,pins)
    assert value['command'] == command
    assert value['inherited_common_mutex'] is value['preparation_only'] is value['never_chart_eligible'] is True
    assert type(value['exit_status']) is type(value['final_exit_status']) is int
    assert value['exit_status'] == value['final_exit_status'] == 0
    assert not any(value.get(key) for key in ('error','cleanup_error','cleanup_errors','remaining_group_members','output_drain_error'))
    return value


def option(command,key):
    assert command.count(key) == 1
    return command[command.index(key)+1]


def memory_proof(row,bound,work,stdlib,pins):
    source = pinned_text(row['source'],pins)
    reviewed = next(item for item in work['source_reviews'] if item['profile'] == row['id'])
    assert reviewed['status'] == 'passed' and reviewed['scored_source_sha256'] == row['source_sha256']
    assert not any(re.search(r'(?i)boehm|libgc|gc_disable|dyld_insert',key+'='+str(value))
        for key,value in row['runtime_environment'].items())
    selected = dict(strategy='owned',source_review=reviewed,explicit_memory_lifetime_review=True)
    if row['benchmark'] == 'binarytrees':
        proofs = [proof for proof in work['proofs'] if proof['kind'] == 'tree' and proof['profile'] == row['id']]
        assert len(proofs) == 4
        assert 'libc::free(node)' in source and 'destroy(long_lived)' in source
        for proof in proofs:
            assert proof['status'] == proof['allocation']['status'] == proof['reconciliation']['status'] == 'passed'
            assert proof['scored_source_sha256'] == row['source_sha256'] and proof['scored_binary_sha256'] == row['binary_sha256']
            assert all(proof['allocation'][key] is True for key in ('real_allocation_call','both_actual_child_calls',
                'allocation_before_leaf_path','scored_native_arm64'))
        selected['actual_scored_allocation_proofs'] = proofs
    elif row['benchmark'] == 'knucleotide':
        assert '.init(&allocators::LIBC_ALLOCATOR)' in source and '.free()' in source
        allocator = stdlib/'std/core/allocators/libc_allocator.c3'
        text = pinned_text(allocator,pins)
        assert 'const LibcAllocator LIBC_ALLOCATOR = {};' in text
        assert 'fn void LibcAllocator.release' in text and 'libc::free(old_ptr);' in text
        assert 'libc::calloc(1, (usz)bytes)' in text and 'libc::malloc((usz)bytes)' in text
        assert any(item['path'] == str(allocator) and item['role'] == 'vendor-standard-library'
            for item in bound['actual_printed_compiler_inputs'])
        proofs = [proof for proof in work['proofs'] if proof['kind'] == 'maps' and proof['profile'] == row['id']]
        assert proofs and all(proof['status'] == proof['reconciliation']['status'] == 'passed' for proof in proofs)
        selected.update(actual_complete_map_proofs=proofs,selected_allocator_source=dict(path=str(allocator),sha256=pins[str(allocator)]))
    for item in bound['source_bundle']:
        text = pinned_text(item['path'],pins)
        assert sha(item['path']) == item['sha256']
        assert not re.search(r'\bGC_(?:malloc|free|collect)|\bimport\s+.*\bgc\b',text)
    return selected


def qualify(args,output,pins):
    descriptor_path = HERE/'descriptor.json'
    assert sha(__file__) == args.qualifier_sha256 and sha(descriptor_path) == args.descriptor_sha256
    descriptor = json.loads(descriptor_path.read_text()); expected = descriptor['expected_evidence']
    assert descriptor['language'] == 'c3' and descriptor['execution'] == 'AOT'
    assert descriptor['status'] == 'source-reviewed-descriptor-awaiting-guarded-actual-evidence-qualification'
    assert all(descriptor[key] is False for key in ('actual_compiler_version_verified','actual_native_architecture_verified','actual_memory_policy_verified'))
    paths = {name:Path(getattr(args,name)).resolve() for name in ('suite','canonical_loc','full_index')}
    assert all(path.is_relative_to(COHORT) for path in paths.values())
    for name,path in paths.items():
        digest = getattr(args,name+'_sha256'); assert sha(path) == digest; merge(pins,{str(path):digest})
    suite,counts,index = (json.loads(paths[name].read_text()) for name in ('suite','canonical_loc','full_index'))
    assert suite['language'] == counts['language'] == 'c3' and set(suite['benchmarks']) == set(index['benchmarks']) == BENCHMARKS
    assert type(index['samples']) is int and index['samples'] == 3
    assert type(index['full_processes']) is int and index['full_processes'] == 90 and index['chart_admission'] is False
    merge(pins,suite['provenance']); merge(pins,counts['pins'])
    merge(pins,{str(descriptor_path):args.descriptor_sha256,str(Path(__file__).resolve()):args.qualifier_sha256,
        str(HERE/'REVIEW.md'):sha(HERE/'REVIEW.md'),str(IMPORTER):args.importer_sha256,
        str(Path(sys.executable).resolve()):sha(Path(sys.executable).resolve())})
    work_path = Path(index['work_qualification']); assert sha(work_path) == index['work_qualification_sha256']
    work = json.loads(work_path.read_text())
    assert work['status'] == 'source-work-allocation-passed-awaiting-full-review-and-LOC' and work['active'] is None
    assert work['pins_unchanged_at_end'] is True and work['chart_admission'] is False
    assert len(work['profiles']) == 20 and len(work['proofs']) == 48 and len(work['attempts']) == 10
    assert work['reused_work_proofs'] == 38 and work['reused_cases_relabelled_as_attempts'] is False
    merge(pins,work['pins']); merge(pins,{str(work_path):index['work_qualification_sha256']})
    native_path = CLBG/expected['native_qualification']
    assert sha(native_path) == expected['native_qualification_sha256'] == work['prerequisite_sha256']
    assert Path(work['prerequisite']) == native_path
    native = json.loads(native_path.read_text())
    assert native['proposal_sha256'] == expected['native_proposal_sha256']
    assert native['active'] is None and len(native['profiles']) == 20
    assert len(native['attempts']) == 28 and native['reused_bounded_cases'] == 126 and native['fresh_profile_builds'] == 5
    assert native['reused_cases_relabelled_as_attempts'] is False and native['chart_admission'] is False
    merge(pins,native['pins']); merge(pins,{str(native_path):expected['native_qualification_sha256']})
    native_rows = {row['id']:row for row in native['profiles']}; assert len(native_rows) == 20
    verify(pins)
    toolchain = pinned_json(CLBG/expected['actual_toolchain'],pins)
    compiler,stdlib = native['compiler'],Path(native['stdlib'])
    assert toolchain['compiler'] == compiler and sha(compiler) == toolchain['compiler_sha256'] == pins[compiler]
    assert toolchain['compiler_version'] == expected['actual_version_text'] and toolchain['bundled_llvm_version'] == '23.1.2'
    assert toolchain['explicit_backend'] == 'llvm' and toolchain['explicit_target'] == 'macos-aarch64'
    version_stdout,version_receipt = (CLBG/expected[key] for key in ('compiler_version_stdout','compiler_version_receipt'))
    owned_receipt(version_receipt,[compiler,'--version'],pins)
    assert pinned_text(version_stdout,pins) == expected['actual_version_text']
    assert pinned_text(version_stdout.with_suffix('.stderr'),pins) == ''
    clang_stdout,clang_receipt = (CLBG/expected[key] for key in ('clang_version_stdout','clang_version_receipt'))
    newly_captured = {str(path):sha(path) for path in (clang_stdout,clang_receipt,clang_stdout.with_suffix('.stderr'))}
    merge(pins,newly_captured); owned_receipt(clang_receipt,[expected['clang'],'--version'],pins)
    assert pinned_text(clang_stdout,pins) == expected['actual_clang_version_text']
    assert pinned_text(clang_stdout.with_suffix('.stderr'),pins) == ''
    assert toolchain['linker'] == expected['clang'] and sha(expected['clang']) == pins[expected['clang']]
    profiles = {}; authors = set(); copies = set()
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
                path = Path(attempt[key]); assert str(path) in suite['provenance'] and sha(path) == suite['provenance'][str(path)]
        for identifier,row in entry['profiles'].items():
            if row['language'] != 'c3': continue
            assert row['id'] == identifier and identifier not in profiles
            original = native_rows[identifier]
            for key in ('source','source_sha256','binary_sha256','runtime_environment','source_bundle'):
                assert row[key] == original[key]
            assert row['binary'] == original['command'][0] and row['command'] == original['command']
            assert sha(row['binary']) == row['binary_sha256'] == pins[row['binary']]
            invref = counts['compiler_bound_input_inventory'][identifier]
            assert invref['source_sha256'] == row['source_sha256'] and invref['binary_sha256'] == row['binary_sha256']
            evidence_path = Path(invref['evidence_path']); assert sha(evidence_path) == invref['evidence_sha256'] == pins[str(evidence_path)]
            evidence = json.loads(evidence_path.read_text())
            assert evidence['profile'] == identifier and evidence['inventory_derived_from_original_actual_compile_receipt'] is True
            assert evidence['authored_application_helpers_complete'] is True
            bound = evidence['independently_retained_compiler_input_evidence']
            assert bound['id'] == identifier and bound['source'] == row['source'] and bound['binary'] == row['binary']
            assert bound['source_bundle'] == row['source_bundle']
            assert {(item['path'],item['sha256']) for item in bound['source_bundle']} == {(item['path'],item['sha256']) for item in invref['files']}
            command = bound['actual_original_compile_command']; build_path = Path(bound['production_build_receipt'])
            assert command == original['build_command'] and command[:2] == [compiler,'compile']
            assert command[2:2+len(row['source_bundle'])] == [item['path'] for item in row['source_bundle']]
            assert option(command,'-o') == row['binary'] and option(command,'--stdlib') == str(stdlib)
            assert '--backend=llvm' in command and option(command,'--target') == 'macos-aarch64'
            assert '--fp-math=strict' in command and option(command,'--cc') == expected['clang'] and '--linker=cc' in command
            assert option(command,'--macos-sdk') == toolchain['sdk'] and option(command,'--threads') == '1'
            assert bound['production_build_receipt_sha256'] == pins[str(build_path)]
            owned_receipt(build_path,command,pins)
            assert pinned_text(Path(bound['compiler_input_stdout']),pins) == pinned_text(build_path.with_suffix('.stdout'),pins)
            assert pinned_text(Path(bound['compiler_input_stderr']),pins) == pinned_text(build_path.with_suffix('.stderr'),pins)
            assert bound['compiler_input_stdout_sha256'] == pins[bound['compiler_input_stdout']]
            assert bound['compiler_input_stderr_sha256'] == pins[bound['compiler_input_stderr']]
            for item in bound['files']:
                author,copy = item['path'],item['compiler_input_copy']; authors.add(author); copies.add(copy)
                assert pinned_text(author,pins) == pinned_text(copy,pins)
                assert sha(author) == sha(copy) == item['sha256'] == item['compiler_input_sha256']
            selected_memory = memory_proof(row,bound,work,stdlib,pins)
            images = build_path.parent/'images'
            owned_receipt(images/'architecture.json',['/usr/bin/lipo','-archs',row['binary']],pins)
            assert pinned_text(images/'architecture.stdout',pins).split() == ['arm64']
            owned_receipt(images/'mach-header.json',['/usr/bin/otool','-hv',row['binary']],pins)
            assert 'ARM64' in pinned_text(images/'mach-header.stdout',pins)
            owned_receipt(images/'linked-images.json',['/usr/bin/otool','-L',row['binary']],pins)
            image = pinned_json(images/'dependency-review.json',pins)
            assert image['binary'] == row['binary'] and image['architectures'] == ['arm64']
            assert [item['install_name'] for item in image['dependencies']] == [line.strip().split(' (',1)[0]
                for line in pinned_text(images/'linked-images.stdout',pins).splitlines()[1:]]
            for item in image['dependencies']:
                if 'path' in item: assert item['sha256'] == pins[item['path']] == sha(item['path'])
            assert not any(re.search(r'(?i)libgc|boehm',item['install_name']) for item in image['dependencies'])
            if benchmark == 'regexredux': assert any(item.get('path','').startswith('/opt/homebrew/Cellar/pcre2/10.49/') for item in image['dependencies'])
            if benchmark == 'pidigits': assert any(item.get('path','').startswith('/opt/homebrew/Cellar/gmp/6.3.0/') for item in image['dependencies'])
            profiles[identifier] = dict(source_sha256=row['source_sha256'],binary_sha256=row['binary_sha256'],
                runtime_environment=row['runtime_environment'],compiler_input_inventory=invref,
                original_build_receipt=dict(path=str(build_path),sha256=pins[str(build_path)]),
                actual_memory_selection=selected_memory,architecture_receipt=str(images/'architecture.json'),image_receipt=str(images/'linked-images.json'))
    assert len(profiles) == 20 and set(profiles) == set(counts['profiles']) == set(native_rows)
    assert len(authors) == 24 and len(copies) == 25
    verify(pins)
    runtime_path = output/'toolchain-runtime-receipt.json'
    save(runtime_path,dict(status='verified-actual-C3-LLVM-native-and-owned-memory',
        actual_c3_version_stdout=version_stdout.read_text(),actual_clang_version_stdout=clang_stdout.read_text(),
        original_toolchain=toolchain,original_version_receipt=str(version_receipt),original_version_stdout=str(version_stdout),
        newly_captured_saved_linker_receipt_pins=newly_captured,recorded_in_original_C3_preparation_pins=False,
        profiles=profiles,authored_source_count=24,measured_source_copy_count=25,memory_strategies=['owned'],
        native_architecture='arm64',pins=dict(pins)))
    merge(pins,{str(runtime_path):sha(runtime_path)})
    metadata = deepcopy(descriptor)
    metadata.update(status='reviewed-actual-native-language-metadata',actual_compiler_version_verified=True,
        actual_native_architecture_verified=True,actual_memory_policy_verified=True,memory_strategies=['owned'],pins=dict(pins))
    metadata['measured_backend_template']['verified'] = True
    metadata['measured_backend_template']['runtime_receipt'].update(verified=True,report=str(runtime_path),report_sha256=pins[str(runtime_path)])
    assert sha(IMPORTER) == args.importer_sha256
    name = 'c3_publication_exact_cohort_validator'; assert name not in sys.modules
    sys.dont_write_bytecode = True
    spec = importlib.util.spec_from_file_location(name,IMPORTER); validator = importlib.util.module_from_spec(spec)
    sys.modules[name] = validator; spec.loader.exec_module(validator)
    assert Path(validator.__file__).resolve() == IMPORTER and sha(IMPORTER) == args.importer_sha256
    assert len(validator.validate_cohort('c3',suite,counts,metadata)) == 20
    verify(pins); save(output/'metadata.json',metadata)


def main():
    parser = argparse.ArgumentParser()
    for name in ('revision','qualifier-sha256','descriptor-sha256','suite','suite-sha256','canonical-loc',
        'canonical-loc-sha256','full-index','full-index-sha256','importer-sha256'):
        parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert re.fullmatch('[a-z0-9][a-z0-9-]{0,63}',args.revision)
    assert all(re.fullmatch('[0-9a-f]{64}',value) for key,value in vars(args).items() if key.endswith('_sha256'))
    print('WAIT common mutex: C3 actual publication metadata only',flush=True)
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
                outcome.update(status='failed-terminal-integrity',terminal_error=str(error),terminal_error_type=type(error).__name__,available_bytes_not_fully_pinned=True)
                outcome['available_evidence_pins'] = available; outcome['prerequisite_pins'] = pins
                save(output/'outcome.json',outcome); raise
            else:
                outcome['available_evidence_pins'] = available; outcome['prerequisite_pins'] = pins; save(output/'outcome.json',outcome)
        print(json.dumps(dict(status=outcome['status'],metadata=str(output/'metadata.json'),sha256=sha(output/'metadata.json'))),flush=True)


if __name__ == '__main__': main()

"""Intake official Builder once, then separately review/run its spec-only route."""
import argparse
from contextlib import ExitStack
from datetime import datetime, timezone
import fcntl
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import sys
import tarfile
import traceback
import unicodedata

P = Path(__file__).resolve().parent
H = P.parents[1]
C = H.parents[1] / 'continuation'
LOCK = Path('/tmp/mech-clbg-timing.lock')
SOURCE = H / '.build/saved-runtime-source-intake-v1/source-original'
SAVED = H / 'runtime-qualification/saved-runtime-source-intake-v1'
SAVED_OWNER = C / 'owned-jobs/rebol-saved-runtime-source-intake-v1'
SAVED_ROOT = C / 'jobs/rebol-saved-source-closed-review-v1/receipt.json'
SAVED_ROOT_SHA = '3f7b4fdd2eeaacc8c2a86e87b33276a76eea0f1d45a8046f47a9114a3fe45b86'
SAVED_REPORT_SHA = '0b467ff8c3fc42ba66e8a83b24d1ad412b37bc5eae43c4a990ee93f4c8d9fe42'
SAVED_BODY_SHA = '21059d5c19ee7d26b8f50d9475e105a7249781c1b690adac776ab7f4444ab373'
SAVED_DURABLE_SHA = '56e4ff4edc603ab063e85934baae0cc70d2bfec94eb92df7cd39637567a51da7'
SAVED_WORKER = H / 'revisions/saved-runtime-source-intake-v1/resume.py'
SAVED_WORKER_SHA = '382a87ddba21477325c3f54978df5d2369878395a7d0c3742d2d845a1aed7aa9'
SAVED_NOTE_SHA = 'b76d26ed7b9e2156f3b71ea762adf49d9a36567ce98fa88c3a15d0f22eb51591'
FAILED_REVIEW_SHA = '7fa46c85521038aabe1be7431f4a8bd2217a32e15d261248162ad611e1fcf32a'
SOURCE_ARCHIVE_SHA = '48df5f2c4cc62d7b4085c908427dd6a698540c32ec4c2a7893a2e5c0d639f671'
NEST_SHA = 'b10fa4bb1301db8d7292be358acdfc143b7d5dd98c488c4cc77e65be3a1483b5'
PREMAKE_SHA = 'fab18619da3c7c0d8689df29e7b17f1f46a3e2c106e57c7f535d81ef0d93a92b'
BUILDER_COMMIT = '5dd9ec60cb96b1ae3cda0ba56ba033d2cf86042e'
BUILDER_URL = 'https://github.com/Siskin-framework/Builder/releases/download/0.21.15/siskin-macos-aarch64.gz'
BUILDER_GZIP_SHA = 'a252cfc599ddd591651d626a85c82a36d20b5c41d9fc8dd3b4b8b4d878c9fcb8'
BUILDER_SOURCE_URL = 'https://codeload.github.com/Siskin-framework/Builder/tar.gz/' + BUILDER_COMMIT
PYTHON = '/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12'
INTAKE = H / 'sdk-qualification/builder-intake-v1'
BUILD = H / '.build/sdk-builder-spec-v1'
SPEC = H / 'sdk-qualification/builder-spec-only-v1'
FILES = ('controller.py','launch.py','REVIEW.md')
PLAN = H / 'revisions/actual-source-sdk-plan-v1/REVIEW.md'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with Path(path).open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def bind(path, digest, pins):
    path = Path(path).absolute()
    assert re.fullmatch('[a-f0-9]{64}', digest) and sha(path) == digest
    assert str(path) not in pins or pins[str(path)] == digest
    pins[str(path)] = digest
    resolved = path.resolve(strict=True)
    if resolved != path:
        assert sha(resolved) == digest and (str(resolved) not in pins or pins[str(resolved)] == digest)
        pins[str(resolved)] = digest
    return path


def read(path, digest, pins):
    return json.loads(bind(path, digest, pins).read_bytes())


def cold(path, name, pins):
    assert name not in sys.modules
    path = Path(path).resolve(strict=True)
    raw = bind(path, pins[str(path)], pins).read_bytes()
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    exec(compile(raw, str(path), 'exec'), module.__dict__)
    assert Path(module.__file__).resolve() == path and sys.modules[name] is module
    return module


def source_packet(args, pins):
    path = Path(args.packet).resolve(strict=True)
    assert not path.stat().st_mode & 0o222
    value = read(path, args.packet_sha256, pins)
    expected = {str(P / name) for name in FILES} | {str(PLAN), str(Path(sys.executable).resolve())}
    assert set(value) == {'schema','pins'} and value['schema'] == 'frozen-rebol-builder-spec-only-v1'
    assert set(value['pins']) == expected
    for filename, digest in value['pins'].items():
        if filename != str(Path(sys.executable).resolve()):
            assert not Path(filename).stat().st_mode & 0o222
        bind(filename, digest, pins)


def closed(owner, digest, command, pid, parent, pins):
    done = read(owner / 'terminal.json', digest, pins)
    records = [read(owner / name, sha(owner / name), pins)
        for name in ('request.json','command-started.json','worker-started.json')]
    request, start, worker = records
    assert request['command'] == start['command'] == worker['command'] == command
    assert request['cwd'] == start['cwd'] == worker['cwd'] == str(H.parents[3])
    assert request['common_mutex_owned_by_target_command'] is True
    assert request['qualification_granted'] is request['automatic_retry'] is False
    assert done['status'] == 'command-exited-zero-awaiting-report-review'
    assert type(done['exit_code']) is int and done['exit_code'] == 0 and done['stop_signals'] == []
    assert done['actual_command_started'] is True and done['command_identity_observation_missing'] is False
    assert done['qualification_granted'] is done['automatic_retry'] is False
    assert not any(done.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
    assert start['command_identity'] == done['command_identity'] and start['command_pid'] == done['command_pid']
    assert start['command_identity_observation_missing'] is False and worker['worker_identity'] == done['worker_identity']
    actual_pid, actual_parent = done['command_pid'], done['worker_identity']['pid']
    assert type(actual_pid) is type(actual_parent) is int and actual_pid > 0 and actual_parent > 0
    assert pid is None or actual_pid == pid
    assert parent is None or actual_parent == parent
    listing = done['command_identity']['listing'].split(None, 7)
    assert listing[:2] == [str(actual_pid), str(actual_parent)] and listing[7].split() == command


def retained(pins):
    root = read(SAVED_ROOT, SAVED_ROOT_SHA, pins)
    assert root['status'] == 'closed-complete-Rebol-saved-runtime-and-full-commit-source-intake'
    assert root['verified_pins'] == 1064 and root['source_files'] == 898
    assert root['report_sha256'] == SAVED_REPORT_SHA and root['body_terminal_sha256'] == SAVED_BODY_SHA
    assert root['durable_terminal_sha256'] == SAVED_DURABLE_SHA and root['source_archive_sha256'] == SOURCE_ARCHIVE_SHA
    assert root['successful_native_commands_reused'] == 5 and root['native_commands'] == root['runtime_downloads'] == 0
    assert root['source_downloads'] == 1 and root['fixture_reexecutions'] == root['benchmark_runs'] == 0
    assert root['source_tree_commit_independently_verified'] is root['generated_headers_verified'] is root['runtime_configuration_admitted'] is False
    command = [PYTHON,'-I','-S','-B','-u',str(SAVED_WORKER),'--worker-sha256',SAVED_WORKER_SHA,
        '--review-sha256',SAVED_NOTE_SHA,'--closed-review-sha256',FAILED_REVIEW_SHA]
    closed(SAVED_OWNER, SAVED_DURABLE_SHA, command, 48933, 48929, pins)
    report = read(SAVED / 'report.json', SAVED_REPORT_SHA, pins)
    body = read(SAVED / 'terminal-pin.json', SAVED_BODY_SHA, pins)
    assert body['status'] == report['status'] == 'runtime-discovered-awaiting-source-header-and-primitive-review'
    assert report['terminal_evidence_complete'] is True and not report.get('terminal_artifact_errors')
    assert body['report'] == str(SAVED / 'report.json') and body['report_sha256'] == SAVED_REPORT_SHA
    assert body['partial_evidence_pins'] == report['pins'] and body['inherited_common_mutex'] is True
    assert not any(report.get(key) for key in ('error','error_type'))
    for path, digest in report['pins'].items():
        bind(path, digest, pins)
    bind(SAVED_WORKER, SAVED_WORKER_SHA, pins)
    bind(SAVED_WORKER.with_name('REVIEW.md'), SAVED_NOTE_SHA, pins)
    resume = cold(SAVED_WORKER, 'rebol_sdk_saved_source_original_helpers', pins)
    # This pure retained path checks the original failed receipt/five native0
    # commands and old6/11 fixture closure. Its main/download/parse paths are
    # never invoked and no saved stage is repeated.
    base, contracts, runtime, _, _ = resume.retained(argparse.Namespace(closed_review_sha256=FAILED_REVIEW_SHA), pins)
    inventory = read(SAVED / 'source-header-closure.json', pins[str(SAVED / 'source-header-closure.json')], pins)
    assert inventory['source_files'] == 898 and inventory['original_full_commit_source'] == str(SOURCE)
    assert inventory['source_archive_sha256'] == SOURCE_ARCHIVE_SHA and inventory['generated_headers_present'] == []
    bind(inventory['source_archive'], SOURCE_ARCHIVE_SHA, pins)
    paths = sorted(path for path in SOURCE.rglob('*') if path.is_file())
    assert len(paths) == 898 and not any(path.is_symlink() for path in SOURCE.rglob('*'))
    for path in paths:
        assert str(path) in pins
        bind(path, pins[str(path)], pins)
    bind(SOURCE / 'make/rebol3.nest', NEST_SHA, pins)
    bind(SOURCE / 'make/pre-make.r3', PREMAKE_SHA, pins)
    base.verify(pins)
    return base, contracts, runtime, paths


def unpack_builder(archive_path, root):
    root.mkdir(exist_ok=False)
    seen, records, total = set(), [], 0
    with tarfile.open(archive_path, 'r:gz') as archive:
        members = archive.getmembers()
        assert 0 < len(members) <= 8192
        for member in members:
            path = PurePosixPath(member.name)
            assert path.parts and path.parts[0] == 'Builder-' + BUILDER_COMMIT
            assert not path.is_absolute() and '\\' not in member.name
            assert all(part not in ('','.','..') for part in member.name.rstrip('/').split('/'))
            relative = Path(*path.parts[1:])
            key = unicodedata.normalize('NFD', str(relative)).casefold()
            assert key not in seen and (member.isdir() or member.isfile())
            seen.add(key)
            target = root / relative
            assert target.resolve().is_relative_to(root.resolve())
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                assert member.size <= 32 * 1024**2
                total += member.size
                assert total <= 128 * 1024**2
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as source, target.open('xb') as output:
                    shutil.copyfileobj(source, output)
                assert target.stat().st_size == member.size
                target.chmod(0o444 | (member.mode & 0o111))
            records.append(dict(name=member.name, relative=str(relative), bytes=member.size,
                type='directory' if member.isdir() else 'regular', mode=member.mode))
    assert (root / 'tree/rebol/Siskin/siskin.reb').is_file()
    return records


def launch(base, service, guard, harness, builder, cwd, phase, output, pins, fd, environment):
    command = [str(builder),'--version'] if phase == 'version' else [
        str(builder),str(cwd / 'make/rebol3.nest'),'--test','%rebol3-bulk-macos-arm64']
    journal = output / (phase + '-startup.json')
    plan = output / (phase + '-launch-plan.json')
    launch_pins = {str(builder): pins[str(builder)], str(P / 'launch.py'): pins[str(P / 'launch.py')]}
    if phase == 'spec-only':
        launch_pins.update({str(path): pins[str(path)] for path in cwd.rglob('*') if path.is_file()})
    save(plan, dict(schema='rebol-builder-single-owned-exec-v1', phase=phase, builder=str(builder), cwd=str(cwd),
        common_fd=fd, command=command, startup_receipt=str(journal), pins=launch_pins))
    bind(plan, sha(plan), pins)
    actual_command = [PYTHON,'-I','-S','-B','-u',str(P / 'launch.py'),'--plan',str(plan),'--plan-sha256',pins[str(plan)]]
    result = service.owned_command(guard,harness,actual_command,environment,output,fd,120,phase)
    receipt = result['detail']
    assert receipt['command'] == actual_command and receipt['inherited_common_mutex'] is True
    assert type(receipt['exit_status']) is type(receipt['final_exit_status']) is int
    assert receipt['exit_status'] == receipt['final_exit_status'] == 0 and not receipt.get('error') and not receipt.get('cleanup_errors')
    started = json.loads(journal.read_bytes())
    assert started['actual_cwd'] == str(cwd) and started['command'] == command and started['pid'] == receipt['pid']
    assert started['inherited_common_mutex'] is True and started['common_mutex_inode'] == 79859134
    base.verify(pins)
    return result


def intake(base, contracts, paths, service, guard, harness, output, pins, fd):
    assert BUILD.resolve().is_relative_to(H.resolve())
    ancestor = BUILD
    while ancestor != H:
        assert not ancestor.is_symlink()
        ancestor = ancestor.parent
    BUILD.mkdir(parents=True, exist_ok=False)
    archives, tools, copy = BUILD / 'archives', BUILD / 'tools', BUILD / 'source-copy'
    archives.mkdir(); tools.mkdir(); copy.mkdir()
    for name in ('home','data','install','temp','version-cwd'):
        (BUILD / name).mkdir()
    environment = dict(PATH='/usr/bin:/bin:/usr/sbin:/sbin',HOME=str(BUILD / 'home'), REBOL_HOME=str(BUILD / 'data'),
        SISKIN_INSTALL=str(BUILD / 'install'),SISKIN_TEMP=str(BUILD / 'temp'),CI='true',LANG='C',LC_ALL='C')
    gzip_path = archives / 'siskin-macos-aarch64.gz'
    record = base.download(BUILDER_URL,gzip_path,32 * 1024**2)
    assert record['sha256'] == BUILDER_GZIP_SHA
    bind(gzip_path,BUILDER_GZIP_SHA,pins)
    builder = tools / 'siskin'
    with gzip.open(gzip_path,'rb') as source, builder.open('xb') as target:
        total = 0
        while chunk := source.read(65536):
            total += len(chunk)
            assert total <= 64 * 1024**2
            target.write(chunk)
    builder.chmod(0o500)
    bind(builder,sha(builder),pins)
    archive = archives / ('Builder-' + BUILDER_COMMIT + '.tar.gz')
    source_record = base.download(BUILDER_SOURCE_URL,archive,32 * 1024**2)
    bind(archive,source_record['sha256'],pins)
    builder_source = BUILD / 'builder-source-original'
    members = unpack_builder(archive,builder_source)
    for path in builder_source.rglob('*'):
        if path.is_file():
            bind(path,sha(path),pins)
    copies = []
    for original in paths:
        target = copy / original.relative_to(SOURCE)
        target.parent.mkdir(parents=True,exist_ok=True)
        with target.open('xb') as stream:
            stream.write(original.read_bytes())
        assert sha(target) == pins[str(original)]
        target.chmod(0o444 | (original.stat().st_mode & 0o111))
        bind(target,pins[str(original)],pins)
        copies.append(dict(original=str(original),copy=str(target),sha256=pins[str(original)]))
    assert len(copies) == 898
    base.verify(pins)
    # The generic image service's scored=True requests an exact ARM64 slice;
    # this is an unscored Builder tool, explicitly disclosed in its receipt.
    extra = service.native_images(guard,harness,builder,environment,output / 'images',fd,[tools],scored=True)
    assert extra == {}, 'New non-OS Builder image requires separate root review'
    image = json.loads((output / 'images/dependency-review.json').read_bytes())
    assert image['binary'] == str(builder) and image['architectures'] == ['arm64']
    for dependency in image['dependencies']:
        contracts.os_image(dependency['install_name'])
    for path in (output / 'images').rglob('*'):
        if path.is_file():
            bind(path,sha(path),pins)
    version = launch(base,service,guard,harness,builder,BUILD / 'version-cwd','version',output,pins,fd,environment)
    version_text = re.sub(r'\x1b\[[0-9;]*[A-Za-z]','',(version['stdout'] + version['stderr']).decode('utf-8'))
    assert 'SISKIN-Framework Builder' in version_text and 'Rebol' in version_text
    save(output / 'intake.json',dict(status='official-Builder-and-source-copy-ready-awaiting-spec-route-review',
        builder=str(builder),builder_sha256=pins[str(builder)],official_gzip=dict(record,publisher_sha256=BUILDER_GZIP_SHA),
        builder_source=str(builder_source),builder_source_commit=BUILDER_COMMIT,builder_source_archive=source_record,
        builder_source_members=members,source_tree_commit_independently_verified=False,
        builder_source_binary_build_equivalence_proved=False,actual_version_stdout_stderr=version_text,
        source_copy=str(copy),source_copy_files=copies,source_copy_count=898,environment=environment,
        image_review=str(output / 'images/dependency-review.json'),unscored_native_tool_commands=4,
        runtime_native_successes_reused=5,original_runtime_commands_reexecuted=0,old_fixture_reexecutions=0,
        header_generation_executed=False,extension_build_executed=False,spec_executed=False,chart_admission=False))


def spec_only(args, base, service, guard, harness, output, pins, fd, dependencies):
    review_path = Path(args.intake_review).resolve(strict=True)
    assert not review_path.stat().st_mode & 0o222
    review = read(review_path,args.intake_review_sha256,pins)
    assert review['status'] == 'reviewed-actual-builder-intake-and-exact-Rebol-spec-only-route'
    assert review['spec_only_execution_authorized'] is True and review['generation_execution_authorized'] is False
    assert review['unresolved'] == [] and review['linked_repositories'] == []
    assert review['selected_target'] == '%rebol3-bulk-macos-arm64'
    assert review['no_force_eval_on_selected_route'] is review['all_selected_actions_are_suppressed_by_test'] is True
    owner = Path(review['durable_owner']).resolve(strict=True)
    assert owner.parent == C / 'owned-jobs'
    lease = dependencies.enter_context((owner / 'owner.lock').open('rb'))
    fcntl.flock(lease,fcntl.LOCK_SH | fcntl.LOCK_NB)
    held,named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
    assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
    expected = [PYTHON,'-I','-S','-B','-u',str(P / 'controller.py'),'--intake',
        '--packet',str(Path(args.packet).resolve()),'--packet-sha256',args.packet_sha256]
    closed(owner,review['durable_terminal_sha256'],expected,None,None,pins)
    old = read(INTAKE / 'report.json',review['report_sha256'],pins)
    body = read(INTAKE / 'terminal-pin.json',review['body_terminal_sha256'],pins)
    assert old['status'] == body['status'] == 'official-Builder-and-source-copy-ready-awaiting-spec-route-review'
    assert old['terminal_evidence_complete'] is True and not any(old.get(key) for key in ('error','error_type','terminal_artifact_errors'))
    assert body['report'] == str(INTAKE / 'report.json') and body['report_sha256'] == review['report_sha256']
    assert body['partial_evidence_pins'] == old['pins'] and body['inherited_common_mutex'] is True
    for path,digest in old['pins'].items():
        bind(path,digest,pins)
    actual = read(INTAKE / 'intake.json',review['intake_sha256'],pins)
    assert actual['status'] == 'official-Builder-and-source-copy-ready-awaiting-spec-route-review'
    assert actual['builder_source_commit'] == BUILDER_COMMIT and actual['source_copy_count'] == 898
    assert actual['builder_source_binary_build_equivalence_proved'] is actual['source_tree_commit_independently_verified'] is False
    copy,builder = Path(actual['source_copy']),Path(actual['builder'])
    assert copy == BUILD / 'source-copy' and builder == BUILD / 'tools/siskin'
    assert Path(actual['builder_source']) == BUILD / 'builder-source-original'
    assert review['builder_sha256'] == actual['builder_sha256'] == pins[str(builder)]
    assert review['reviewed_source_pins'] == {str(path):pins[str(path)] for path in Path(actual['builder_source']).rglob('*') if path.is_file()}
    assert review['selected_recipe_pins'] == {str(copy / 'make/rebol3.nest'):NEST_SHA,str(copy / 'make/pre-make.r3'):PREMAKE_SHA}
    assert len(actual['source_copy_files']) == 898
    for item in actual['source_copy_files']:
        assert Path(item['original']).is_relative_to(SOURCE) and Path(item['copy']).is_relative_to(copy)
        assert Path(item['copy']).relative_to(copy) == Path(item['original']).relative_to(SOURCE)
        bind(item['original'],item['sha256'],pins); bind(item['copy'],item['sha256'],pins)
    assert {str(path) for path in copy.rglob('*') if path.is_file()} == {item['copy'] for item in actual['source_copy_files']}
    base.verify(pins)
    result = launch(base,service,guard,harness,builder,copy,'spec-only',output,pins,fd,actual['environment'])
    assert result['stderr'] == b''
    original_paths = {item['copy'] for item in actual['source_copy_files']}
    added = sorted(path for path in copy.rglob('*') if path.is_file() and str(path) not in original_paths)
    specs = [path for path in added if path.name == 'spec.reb']
    assert len(specs) == 1 and specs[0].stat().st_size > 0, 'One actual resolved spec required'
    assert all(path.name in {'spec.reb','objects.txt'} and path.parent == specs[0].parent for path in added), 'Unexpected non-spec output'
    assert not any(path.is_symlink() for path in copy.rglob('*'))
    assert not any(path.is_file() for root in (BUILD / 'install',BUILD / 'temp') for path in root.rglob('*'))
    assert not (copy / 'src/include/reb-lib.h').exists(), 'No header generation in spec phase'
    for path in added:
        assert path.resolve().is_relative_to(copy.resolve()) and not path.is_symlink()
        bind(path,sha(path),pins)
    save(output / 'spec-result.json',dict(status='actual-resolved-spec-captured-awaiting-full-root-spec-review',
        resolved_spec=str(specs[0]),resolved_spec_sha256=pins[str(specs[0])],new_files=[dict(path=str(path),sha256=pins[str(path)]) for path in added],
        immutable_source_files_preserved=898,original_runtime_commands_reexecuted=0,old_fixture_reexecutions=0,
        builder_intake_reexecutions=0,owned_spec_commands=1,selected_target=review['selected_target'],
        source_policy_review=dict(path=str(review_path),sha256=args.intake_review_sha256),
        native_child_count_independently_observed=False,header_generation_executed=False,extension_build_executed=False,
        resolved_spec_semantics_qualified=False,chart_admission=False))


def main():
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--intake',action='store_true'); mode.add_argument('--spec-only',action='store_true')
    for name in ('packet','packet-sha256'):
        parser.add_argument('--' + name,required=True)
    parser.add_argument('--intake-review'); parser.add_argument('--intake-review-sha256')
    args = parser.parse_args()
    assert sys.dont_write_bytecode and Path(sys.executable).resolve() == Path(PYTHON).resolve()
    assert bool(args.intake_review) is bool(args.intake_review_sha256) is args.spec_only
    assert re.fullmatch('[a-f0-9]{64}',args.packet_sha256)
    def interrupted(signum,frame):
        raise KeyboardInterrupt('Rebol SDK gate interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT,signal.SIGTERM,signal.SIGHUP,signal.SIGALRM):
        signal.signal(signum,interrupted)
    print('WAIT common mutex: Rebol Builder intake or separately reviewed spec-only',flush=True)
    with LOCK.open('r+') as common, ExitStack() as dependencies:
        fcntl.flock(common,fcntl.LOCK_EX)
        held,named = os.fstat(common.fileno()),LOCK.stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino) and held.st_ino == 79859134
        for owner in (SAVED_OWNER,C / 'owned-jobs/rebol-retained-runtime-intake-v1',C / 'owned-jobs/rebol-runtime6-fixture-seal-v1'):
            lease = dependencies.enter_context((owner / 'owner.lock').open('rb'))
            fcntl.flock(lease,fcntl.LOCK_SH | fcntl.LOCK_NB)
            opened,named = os.fstat(lease.fileno()), (owner / 'owner.lock').stat()
            assert (opened.st_dev,opened.st_ino) == (named.st_dev,named.st_ino)
        output = INTAKE if args.intake else SPEC
        assert output.resolve().is_relative_to(H.resolve())
        ancestor = output
        while ancestor != H:
            assert not ancestor.is_symlink()
            ancestor = ancestor.parent
        output.mkdir(parents=True,exist_ok=False)
        pins,base,failure = {},None,None
        report = dict(status='running-Rebol-SDK-gate',inherited_common_mutex=True,
            chart_admission=False,runtime_configuration_admitted=False,source_work_qualified=False,
            header_generation_executed=False,extension_build_executed=False,benchmark_processes_authorized=0,
            full_processes_authorized=0,actual_fixture_methods_reexecuted=0,original_runtime_commands_reexecuted=0)
        signal.alarm(600)
        try:
            source_packet(args,pins)
            base,contracts,runtime,paths = retained(pins)
            service,guard,harness = base.service(pins)
            if args.intake:
                intake(base,contracts,paths,service,guard,harness,output,pins,common.fileno())
                report['status'] = 'official-Builder-and-source-copy-ready-awaiting-spec-route-review'
            else:
                spec_only(args,base,service,guard,harness,output,pins,common.fileno(),dependencies)
                report['status'] = 'actual-resolved-spec-captured-awaiting-full-root-spec-review'
        except BaseException as error:
            failure = error
            (output / 'error.log').write_text(traceback.format_exc())
            raise
        finally:
            signal.alarm(0)
            # Capture partial new evidence even when a native/parse gate stops.
            capture_errors = []
            for root in (output,BUILD / 'archives',BUILD / 'tools',BUILD / 'builder-source-original',BUILD / 'source-copy'):
                if root.exists():
                    for path in root.rglob('*'):
                        if path.is_file() and path.name not in ('report.json','terminal-pin.json'):
                            try:
                                bind(path,pins.get(str(path),sha(path)),pins)
                            except BaseException as error:
                                capture_errors.append(dict(path=str(path),error=str(error)))
            if capture_errors:
                report['private_root_pin_errors'] = capture_errors
                if failure is None:
                    failure = RuntimeError('New SDK evidence could not be fully pinned')
            if base is None:
                path = H / 'revisions/runtime-v1/bootstrap_runtime.py'
                bind(path,'7e2d3cf696273b8d96f9a31b752e7170da69ea0240b577de6c9dbe118f8ec117',pins)
                base = cold(path,'rebol_sdk_terminal_only_original_helper',pins)
            base.terminal(output,report,pins,failure)
            if failure is not None:
                raise failure


if __name__ == '__main__':
    main()

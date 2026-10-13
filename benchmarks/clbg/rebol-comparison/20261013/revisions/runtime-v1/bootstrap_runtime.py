"""Unexecuted private runtime intake. First seal never downloads or runs Rebol.

All actual hashes/imports/fixtures/downloads/native queries/cleanup occur with
the same named common mutex held. No compiler/header/extension/benchmark route.
"""
import argparse
import fcntl
import gzip
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import sys
import tarfile
import time
import urllib.request

PACKET=Path(__file__).resolve().parent
HERE=PACKET.parents[1]
CLBG=HERE.parents[1]
LOCK=Path('/tmp/mech-clbg-timing.lock')
SERVICE=CLBG/'c3-comparison/20261012/revisions/private-v1/guarded_prepare.py'
SERVICE_PROPOSAL=CLBG/'c3-comparison/20261012/proposals/private-v1.json'
SERVICE_SHA='43438603d675472393050d05928e488a20f16040493935aca676007fc0ada050'
OWNERSHIP_IDS=[
    'test_guarded_prepare.OwnershipChecks.test_popen_failure_has_no_foreign_cleanup',
    'test_guarded_prepare.OwnershipChecks.test_identity_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_journal_failure_cleans_known_child',
    'test_guarded_prepare.OwnershipChecks.test_timeout_preserves_complete_drained_output',
    'test_guarded_prepare.OwnershipChecks.test_orphan_compiler_child_is_fatal',
    'test_guarded_prepare.OwnershipChecks.test_cleanup_failure_is_fatal_and_waits',
    'test_guarded_prepare.OwnershipChecks.test_live_child_cleanup_failure_is_bounded_and_only_signals_owned_group',
    'test_guarded_prepare.ProbeRestorationChecks.test_setup_failure_restores_present_state',
    'test_guarded_prepare.ProbeRestorationChecks.test_absent_input_key_is_removed_on_failure',
    'test_guarded_prepare.ProbeRestorationChecks.test_parent_failure_archives_actual_sampler_bytes',
    'test_guarded_prepare.ProbeRestorationChecks.test_trace_failure_archives_and_stops',
]


def sha(path):
    with Path(path).open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def save(path,value):
    path=Path(path);path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as stream:json.dump(value,stream,indent=2);stream.write('\n')


def merge(*groups):
    result={}
    for group in groups:
        for path,digest in group.items():
            assert path not in result or result[path]==digest,('Conflicting pin',path)
            result[path]=digest
    return result


def verify(pins):
    for path,digest in pins.items():assert sha(path)==digest,('Changed pinned bytes',path)


def fresh(path,name,pins):
    path=Path(path).resolve(strict=True)
    assert name not in sys.modules and sha(path)==pins[str(path)]
    source=path.read_bytes();assert hashlib.sha256(source).hexdigest()==pins[str(path)]
    spec=importlib.util.spec_from_file_location(name,path)
    loaded=importlib.util.module_from_spec(spec);sys.modules[name]=loaded
    exec(compile(source,str(path),'exec'),loaded.__dict__)
    assert sys.modules[name] is loaded and Path(loaded.__file__).resolve()==path
    assert sha(path)==pins[str(path)]
    return loaded


def service(pins):
    assert sha(SERVICE_PROPOSAL)==SERVICE_SHA
    original=json.loads(SERVICE_PROPOSAL.read_bytes())
    assert original['status']=='sealed-awaiting-root-approval'
    receipt=Path(original['actual_mock_fixture_receipt'])
    assert original['actual_mock_fixture_receipt_sha256']==original['pins'][str(receipt)]
    saved={path:digest for path,digest in original['pins'].items() if Path(path).suffix=='.py'}
    saved.update({path:digest for path,digest in original['pins'].items()
        if Path(path).parent==receipt.parent and Path(path).name.startswith('ownership-fixtures')})
    saved[str(SERVICE_PROPOSAL)]=SERVICE_SHA
    pins.update(merge(pins,saved));verify(pins)
    actual=json.loads(receipt.read_bytes())
    assert actual['status']=='passed' and actual['inventory']=='ownership' and actual['successful'] is True
    assert actual['expected_test_ids']==OWNERSHIP_IDS
    for key in ('tests_run','discovered_tests'):assert type(actual[key]) is int and actual[key]==11
    assert len(actual['actual_test_ids'])==len(set(actual['actual_test_ids']))==11
    assert set(actual['actual_test_ids'])==set(OWNERSHIP_IDS)
    assert all(actual[key]==[] for key in ('skipped','failures','errors','expected_failures','unexpected_successes'))
    for loaded in list(sys.modules.values()):
        filename=getattr(loaded,'__file__',None)
        if filename:
            path=Path(filename).resolve()
            if path.is_relative_to(CLBG):assert str(path) in pins and sha(path)==pins[str(path)],('Foreign cached source',path)
    retained=fresh(SERVICE,'rebol_reviewed_generic_service',pins)
    guard=retained.load_guard();harness=guard.load_harness()
    for loaded in (guard,harness,harness.compact,harness.compact.common):
        path=str(Path(loaded.__file__).resolve());assert path in pins and sha(path)==pins[path]
    verify(pins)
    harness.install_interrupt_handlers()
    return retained,guard,harness


def static_paths():
    return sorted(set(PACKET.glob('*.py'))|set(PACKET.glob('*.reb'))|set(PACKET.glob('*.md'))|
        {HERE/'RESEARCH_PLAN.md',HERE/'CAPABILITIES.md'})


def terminal(output,report,pins,error):
    partial=dict(pins);failures=list(report.get('private_root_pin_errors',[]))
    for path in sorted(output.rglob('*')):
        if path.is_file() and path.name not in ('report.json','terminal-pin.json'):
            try:partial=merge(partial,{str(path.resolve()):sha(path)})
            except BaseException as failure:failures.append(dict(path=str(path),error=str(failure) or type(failure).__name__))
    if error is not None:report.update(status='stopped-unqualified',error=str(error) or type(error).__name__,error_type=type(error).__name__)
    for path,digest in partial.items():
        try:assert sha(path)==digest,('Changed pinned bytes',path)
        except BaseException as failure:failures.append(dict(path=path,error=str(failure) or type(failure).__name__))
    if failures:report.update(status='failed-terminal-integrity',terminal_artifact_errors=failures)
    report.update(pins=partial,terminal_evidence_complete=not failures)
    save(output/'report.json',report)
    save(output/'terminal-pin.json',dict(status=report['status'],report=str(output/'report.json'),
        report_sha256=sha(output/'report.json'),partial_evidence_pins=partial,inherited_common_mutex=True,
        chart_admission=False,full_processes_authorized=0))
    if failures:raise RuntimeError('Runtime terminal integrity failed; readable partial pins retained')


def metadata_seal(output,pins):
    c=fresh(PACKET/'contracts.py','rebol_runtime_contracts',pins)
    assert c.authorize('seal')['downloads'] is False
    service(pins) # Verify retained eleven-fixture ownership source; no native command.
    runner=fresh(PACKET/'fixture_runner.py','rebol_runtime_fixture_runner',pins)
    receipt=runner.run(output/'fixtures',pins,sha,verify,save,c.IDS)
    for path in sorted((output/'fixtures').rglob('*')):
        if path.is_file():pins[str(path.resolve())]=sha(path)
    proposal=dict(status='sealed-awaiting-root-review',tag=c.TAG,source_commit=c.COMMIT,
        runtime_url=c.RUNTIME_URL,publisher_runtime_sha256=c.RUNTIME_SHA,source_url=c.SOURCE_URL,
        source_archive_prior_sha256=None,fixture_receipt=receipt,service_proposal_sha256=SERVICE_SHA,
        runtime_configuration_admitted=False,source_work_qualified=False,full_processes_authorized=0,
        header_generation_authorized=False,benchmark_processes_authorized=0,pins=pins)
    c.authorize('run',proposal);verify(pins)
    filename=HERE/'proposals/runtime-v1.json';save(filename,proposal)
    return dict(status='sealed-awaiting-root-review',proposal=str(filename),proposal_sha256=sha(filename),
        actual_fixture_count=len(c.IDS),downloads_started=0,native_commands_started=0)


def download(url,destination,max_bytes):
    started=time.monotonic();total=0
    request=urllib.request.Request(url,headers={'User-Agent':'mech-private-rebol-runtime-intake'})
    with urllib.request.urlopen(request,timeout=30) as response,destination.open('xb') as stream:
        while chunk:=response.read(1024*1024):
            total+=len(chunk)
            assert total<=max_bytes and time.monotonic()-started<=300,'Bounded publisher download exceeded limit'
            stream.write(chunk)
    assert total>0
    return dict(url=url,actual_bytes=total,sha256=sha(destination),elapsed_seconds=time.monotonic()-started)


def unpack_runtime(archive,destination):
    total=0
    with gzip.open(archive,'rb') as source,destination.open('xb') as target:
        while chunk:=source.read(1024*1024):
            total+=len(chunk);assert total<=32*1024*1024,'Runtime gzip expands beyond private intake limit'
            target.write(chunk)
    assert total>0
    destination.chmod(0o500)


def unpack_source(c,archive,destination):
    destination.mkdir();seen=set();total=0
    with tarfile.open(archive,'r:gz') as bundle:
        for member in bundle:
            kind='directory' if member.isdir() else 'regular' if member.isfile() else 'unsupported'
            relative=c.source_member(member.name,kind,seen);target=destination/relative
            if member.isdir():target.mkdir(parents=True,exist_ok=True);continue
            assert member.size<=64*1024*1024
            total+=member.size;assert total<=256*1024*1024,'Source archive expands beyond private limit'
            target.parent.mkdir(parents=True,exist_ok=True)
            with bundle.extractfile(member) as source,target.open('xb') as stream:
                shutil.copyfileobj(source,stream,1024*1024)
            assert target.stat().st_size==member.size
            target.chmod(0o500 if member.mode&0o111 else 0o400)
    for name in c.SOURCE_REQUIRED:assert (destination/name).is_file(),('Source/header closure missing',name)
    assert (destination/'.version').read_text().strip()==c.TAG,('Tagged source version differs',destination)
    return dict(original_full_commit_source=str(destination),source_files=sum(path.is_file() for path in destination.rglob('*')),
        source_unpacked_bytes=total,source_archive_digest_newly_captured=True,
        source_archive_prior_publisher_sha_available=False,source_generation_executed=False)


def parse_probe(raw):
    expected={'version','platform','function_is_interpreted_value','function_is_native_value','dynamic_rebinding',
        'fresh_block_identity','automatic_gc_before','automatic_gc_after'}
    result={}
    for line in raw.decode('utf-8').splitlines():
        key,value=line.split('=',1);assert key in expected and key not in result
        if key.startswith('automatic_gc_'):assert re.fullmatch('[0-9]+',value);value=int(value)
        elif key not in {'version','platform'}:assert value in {'true','false'};value=value=='true'
        result[key]=value
    assert set(result)==expected
    return result


def bootstrap(args,output,pins,lock_fd,report):
    proposal=HERE/'proposals/runtime-v1.json'
    assert args.proposal_sha256 and sha(proposal)==args.proposal_sha256
    sealed=json.loads(proposal.read_bytes());pins.update(merge(pins,sealed['pins'],{str(proposal):args.proposal_sha256}))
    verify(pins)
    c=fresh(PACKET/'contracts.py','rebol_runtime_contracts',pins);c.authorize('run',sealed)
    assert sealed['service_proposal_sha256']==SERVICE_SHA
    owned,guard,harness=service(pins)
    build=c.private_target(HERE,HERE/'.build/runtime-v1')
    build.mkdir(parents=True)
    assert build.resolve().is_relative_to(HERE.resolve()) and not build.is_symlink()
    # Terminal captures this exclusive private root on both success and failure.
    report['private_root']=str(build)
    archives=build/'archives';archives.mkdir()
    runtime_archive=archives/'rebol3-bulk-macos-arm64.gz'
    record=download(c.RUNTIME_URL,runtime_archive,16*1024*1024)
    save(output/'publisher-runtime-download.json',record)
    assert record['sha256']==c.RUNTIME_SHA,'Publisher runtime gzip SHA differs'
    pins[str(runtime_archive)]=c.RUNTIME_SHA
    originals=build/'originals';originals.mkdir();original=originals/'rebol3'
    unpack_runtime(runtime_archive,original);pins[str(original)]=sha(original)
    runtime_dir=build/'runtime';runtime_dir.mkdir();runtime=runtime_dir/'rebol3'
    shutil.copyfile(original,runtime);runtime.chmod(0o500);assert sha(runtime)==pins[str(original)]
    pins[str(runtime)]=pins[str(original)]
    data=build/'private-data';data.mkdir()
    environment=dict(PATH='/usr/bin:/bin:/usr/sbin:/sbin',LANG='C',LC_ALL='C',REBOL_HOME=str(data))
    before=c.startup_inventory(runtime_dir,data);save(output/'startup-before.json',before)
    save(output/'environment.json',dict(environment=environment,cwd=str(owned.HERE),
        private_child_security_option='-s',global_environment_or_security_mutations=0))
    images=output/'images'
    extra=owned.native_images(guard,harness,runtime,environment,images,lock_fd,[runtime_dir],scored=True)
    assert extra=={},'Unexpected private dependency requires separate image closure'
    reviewed=json.loads((images/'dependency-review.json').read_bytes())
    assert reviewed['architectures']==['arm64']
    for dependency in reviewed['dependencies']:c.os_image(dependency['install_name'])
    save(output/'os-image-review.json',reviewed)
    version=owned.owned_command(guard,harness,[str(runtime),'-v'],environment,output/'queries',lock_fd,30,'version')
    assert '3.22.1' in version['stdout'].decode() and 'Rebol/Bulk' in version['stdout'].decode()
    bytefile=output/'regular-file-all-bytes.bin'
    command=[str(runtime),'-q','-s',str(PACKET/'runtime_probe.reb'),str(bytefile)]
    probe=owned.owned_command(guard,harness,command,environment,output/'queries',lock_fd,30,'features')
    fields=parse_probe(probe['stdout']);observed=c.observations(fields,bytefile.read_bytes())
    after=c.startup_inventory(runtime_dir,data);assert before==after
    save(output/'startup-after.json',after)
    save(output/'observed-runtime-features.json',dict(fields=fields,command=command,observations=observed,
        runtime_binary=str(runtime),runtime_binary_sha256=pins[str(runtime)],regular_byte_output=str(bytefile),
        regular_byte_output_sha256=sha(bytefile),inherited_common_mutex=True))
    source_archive=archives/('Rebol3-'+c.COMMIT+'.tar.gz')
    # The source archive has no prior publisher SHA in reviewed primary evidence.
    # Capture its immutable full-commit URL bytes; never execute them in this phase.
    source_record=download(c.SOURCE_URL,source_archive,64*1024*1024)
    save(output/'source-download.json',dict(source_record,source_commit=c.COMMIT,
        actual_sha_newly_captured=True,prior_publisher_sha_available=False))
    pins[str(source_archive)]=source_record['sha256']
    source_root=build/'source-original';source_inventory=unpack_source(c,source_archive,source_root)
    for path in sorted(source_root.rglob('*')):
        if path.is_file():pins[str(path.resolve())]=sha(path)
    save(output/'source-header-closure.json',dict(source_inventory,required_paths=c.SOURCE_REQUIRED,
        source_archive=str(source_archive),source_archive_sha256=source_record['sha256'],
        generated_headers_present=[str(path) for path in source_root.rglob('reb-lib.h')],
        source_archive_root_digest_review_required_before_generation=True))
    report.update(status='runtime-discovered-awaiting-source-header-and-primitive-review',
        observed_runtime_features=observed,runtime_binary=str(runtime),runtime_binary_sha256=pins[str(runtime)],
        source_archive_sha256=source_record['sha256'],source_archive_newly_captured=True,
        source_header_generation_executed=False,native_command_count=5,
        source_tree_commit_independently_verified=False,
        selected_architecture='arm64',emulation_used=False,runtime_configuration_admitted=False)


def main():
    parser=argparse.ArgumentParser();phase=parser.add_mutually_exclusive_group(required=True)
    phase.add_argument('--seal',action='store_true');phase.add_argument('--run',action='store_true')
    parser.add_argument('--proposal-sha256');args=parser.parse_args()
    with LOCK.open('a+') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX)
        actual=os.fstat(lock.fileno());named=LOCK.stat();assert (actual.st_dev,actual.st_ino)==(named.st_dev,named.st_ino)
        inherited=fcntl.fcntl(lock.fileno(),fcntl.F_DUPFD,64);os.set_inheritable(inherited,True)
        output=HERE/('proposal-seals/runtime-v1' if args.seal else 'runtime-qualification/runtime-v1')
        output.mkdir(parents=True)
        pins={};report=dict(status='started',inherited_common_mutex=True,chart_admission=False,
            runtime_configuration_admitted=False,source_work_qualified=False,full_processes_authorized=0,
            benchmark_processes_authorized=0);error=None
        try:
            for path in static_paths():pins[str(path.resolve())]=sha(path)
            if args.seal:report.update(metadata_seal(output,pins))
            else:bootstrap(args,output,pins,inherited,report)
        except BaseException as failure:
            error=failure
            try:
                with (output/'exception.log').open('x') as stream:
                    import traceback
                    traceback.print_exc(file=stream)
            except BaseException as journal_error:
                report['exception_journal_error']=str(journal_error) or type(journal_error).__name__
            raise
        finally:
            try:
                if 'private_root' in report:
                    for path in sorted(Path(report['private_root']).rglob('*')):
                        if path.is_file():
                            try:pins=merge(pins,{str(path.resolve()):sha(path)})
                            except BaseException as failure:
                                report.setdefault('private_root_pin_errors',[]).append(dict(path=str(path),error=str(failure)))
                                if error is None:error=failure
                terminal(output,report,pins,error)
            finally:os.close(inherited)


if __name__=='__main__':main()

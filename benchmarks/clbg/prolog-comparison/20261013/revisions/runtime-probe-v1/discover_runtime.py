"""Only missing proper probe and image closure; preserve successful version."""
import argparse,fcntl,hashlib,importlib.util,json,os,sys
from pathlib import Path
P=Path(__file__).resolve().parent;H=P.parents[1];C=H.parents[1]/'continuation'
LOCK=Path('/tmp/mech-clbg-timing.lock');PRIOR=H/'.build/swi-runtime-postslice-v1'
JOB=C/'owned-jobs/prolog-runtime-postslice-discovery-v1'
POST=H/'revisions/runtime-postslice-v1/discover_runtime.py'
POST_SHA='54dbac39a7fbeab8f24e7a78c385f2fe0442652844b0c0af8b3a4f3c01a9a387'
AUDIT=C/'jobs/prolog-probe-collision-review-v2/review.json'
AUDIT_SHA='ee049861eb70c41a6040abc3cc5943ededdca46ba5b1e189812c2b7ce1f9b20b'
AUDIT_TERMINAL_SHA='a42b7bc12e9d8308df3d81ead955baffae287c3b3f352d9633689eaaa8f12c23'
FAILED_SHA='e2929aa1ed44bad3815e61bf1eb34a83eb532897ae17f35442dc28e265a7b9f1'
FAILED_TERMINAL_SHA='b4123f2b59bb217ac4b7640c1ab2cb087756ed41655c8bee1dd579a88fbdddd6'
VERSION_SHA='512ba02872fbd607e4168bc30a9340861215686808b1371deb928f125f9fba0f'
RAW_SHA='e792b7602560f20cd5be2b4ceca83a2d2d8bc7a81545879f1febbec604de2346'

def sha(path):
    with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
    path.parent.mkdir(parents=True,exist_ok=True)
    with path.open('x') as f:json.dump(value,f,indent=2);f.write('\n')

def merge(*groups):
    result={}
    for group in groups:
        for path,digest in group.items():assert path not in result or result[path]==digest;result[path]=digest
    return result

def verify(pins):
    for path,digest in pins.items():assert sha(path)==digest,('Changed pinned evidence',path)

def fresh(path,name,pins):
    assert name not in sys.modules and sha(path)==pins[str(path)]
    raw=path.read_bytes();assert hashlib.sha256(raw).hexdigest()==pins[str(path)]
    module=importlib.util.module_from_spec(importlib.util.spec_from_file_location(name,path));sys.modules[name]=module
    exec(compile(raw,str(path),'exec'),module.__dict__)
    assert module is sys.modules[name] and Path(module.__file__).resolve()==path;verify(pins);return module

def main():
    parser=argparse.ArgumentParser()
    for key in ('proposal-sha256','runtime-proposal-sha256','worker-sha256','failed-report-sha256','failed-terminal-sha256','layout-review-sha256','layout-terminal-sha256'):
        parser.add_argument('--'+key,required=True)
    args=parser.parse_args()
    for value in vars(args).values():assert len(value)==64 and all(char in '0123456789abcdef' for char in value)
    print('WAIT common mutex: corrected private runtime probe and missing image closure only',flush=True)
    with LOCK.open('r+') as common:
        fcntl.flock(common,fcntl.LOCK_EX);a,b=os.fstat(common.fileno()),LOCK.stat()
        assert(a.st_dev,a.st_ino)==(b.st_dev,b.st_ino) and a.st_ino==79859134
        with (JOB/'owner.lock').open('r+') as prior_owner:
            fcntl.flock(prior_owner,fcntl.LOCK_EX|fcntl.LOCK_NB)
            output=H/'.build/swi-runtime-probe-v1';output.mkdir()
            pins={};legacy=None;error=None
            report=dict(status='running',chart_admission=False,full_processes_authorized=0,inherited_common_mutex=True,
                global_installation_mutated=False,runtime_configuration_admitted=False,source_work_qualified=False,
                source_library_review_pending=True,automatic_retry=False,actual_benchmark_runs=0,
                downloads=0,extractions=0,mounts=0,app_copies=0,slices=0,fixture_reexecutions=0,version_reexecutions=0,
                proposal=str(H/'proposals/private-v1.json'),proposal_sha256=args.proposal_sha256,runtime_proposal_sha256=args.runtime_proposal_sha256)
            def read(path,digest=None):
                value=sha(path);assert digest is None or value==digest
                assert str(path) not in pins or pins[str(path)]==value;pins[str(path)]=value
                return json.loads(path.read_text())
            try:
                assert sys.dont_write_bytecode
                for path in P.iterdir():
                    if path.is_file():assert not path.stat().st_mode&0o222;pins[str(path.resolve())]=sha(path)
                assert pins[str(Path(__file__).resolve())]==args.worker_sha256
                audit=read(AUDIT,AUDIT_SHA);at=read(AUDIT.with_name('terminal.json'),AUDIT_TERMINAL_SHA)
                assert audit['status']==at['status']=='closed-version-passed-probe-result-retained-owned-receipt-collision'
                assert at['body_error'] is None and at['integrity_errors']==[] and at['review']==str(AUDIT) and at['review_sha256']==AUDIT_SHA
                assert audit['actual_native_commands']==audit['fixture_reexecutions']==0 and audit['private_lease_closed'] is True
                pins.update(merge(pins,audit['pins']));verify(pins)
                failed=read(PRIOR/'report.json',FAILED_SHA);terminal=read(PRIOR/'terminal-pin.json',FAILED_TERMINAL_SHA)
                assert failed['status']==terminal['status']=='stopped-unqualified' and failed['error_type']=='FileExistsError'
                assert failed['terminal_evidence_complete'] is True and failed['pins']==terminal['partial_evidence_pins']
                assert terminal['report']==str(PRIOR/'report.json') and terminal['report_sha256']==FAILED_SHA
                assert failed['runtime_configuration_admitted'] is failed['source_work_qualified'] is False and failed['full_processes_authorized']==0
                pins.update(merge(pins,failed['pins']));verify(pins)
                request,worker,started,ended=(read(JOB/name) for name in ('request.json','worker-started.json','command-started.json','terminal.json'))
                original=['/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/python/bin/python3.12','-I','-S','-B','-u',str(POST),
                    '--proposal-sha256',args.proposal_sha256,'--runtime-proposal-sha256',args.runtime_proposal_sha256,'--worker-sha256',POST_SHA,
                    '--failed-report-sha256',args.failed_report_sha256,'--failed-terminal-sha256',args.failed_terminal_sha256,
                    '--layout-review-sha256',args.layout_review_sha256,'--layout-terminal-sha256',args.layout_terminal_sha256]
                assert request['command']==worker['command']==started['command']==original and request['cwd']==worker['cwd']==started['cwd']==str(C.parent.parent.parent)
                assert ended['status']=='command-failed' and type(ended['exit_code']) is int and ended['exit_code']==1 and ended['actual_command_started'] is True
                assert ended['stop_signals']==[] and not any(ended.get(k) for k in ('error','error_type','exception_cleanup','error_log_write_failure'))
                assert worker['worker_identity']==ended['worker_identity'] and started['command_identity']==ended['command_identity'] and started['command_pid']==ended['command_pid']
                assert started['command_identity_observation_missing'] is ended['command_identity_observation_missing'] is False
                assert started['command_identity']['listing'].split()[7:]==original
                assert pins[str(POST)]==sha(POST)==POST_SHA
                post=fresh(POST,'prolog_probe_prior_postslice_source',pins)
                pins,binary,fat,home,native,original_app,closed=post.original_inputs(args,pins)
                legacy=fresh(post.OLD_WORKER,'prolog_probe_original_runtime_helpers',pins)
                version=read(PRIOR/'runtime/version.json',VERSION_SHA)
                assert version['command']==[str(binary),'--version'] and version['inherited_common_mutex'] is True
                assert type(version['exit_status']) is type(version['final_exit_status']) is int and version['exit_status']==version['final_exit_status']==0
                assert not any(version.get(k) for k in ('error','error_type','cleanup_error','cleanup_errors'))
                assert '10.0.2' in (PRIOR/'runtime/version.stdout').read_text()
                raw=read(PRIOR/'runtime/feature-probe.json',RAW_SHA);active=read(PRIOR/'runtime/feature-probe.active.json')
                assert 'command' not in raw and active['command']==audit['actual_probe_command']
                assert (PRIOR/'runtime/feature-probe.stderr').read_bytes()==b'' and not (PRIOR/'images').exists()
                probe_original=legacy.ORIGINAL_PACKET/'tools/runtime_probe.pl';support_original=legacy.ORIGINAL_PACKET/'tools/prepare_qlf.pl'
                before=probe_original.read_bytes();support=support_original.read_bytes()
                assert before.count(b':- use_module(prepare_qlf, []).')==support.count(b':- initialization(main, main).')==1
                assert (P/'runtime_probe.pl').read_bytes()==before.replace(b':- use_module(prepare_qlf, []).',b':- use_module(prepare_without_main, []).',1)
                assert (P/'prepare_without_main.pl').read_bytes()==support.replace(b':- initialization(main, main).',b'% This runtime-only import retains helpers without registering the compiler CLI main.',1)
                assert b'    halt(0).' in (P/'runtime_probe.pl').read_bytes()
                runtime=read(H/'proposals/runtime-v2.json',args.runtime_proposal_sha256)
                legacy.runtime_fixtures(runtime['local_fixture_receipt']);pins.update(merge(pins,runtime['pins']))
                preparation,sealed,pins=legacy.load_preparation(args,pins)
                service,references,guard,harness,pins=legacy.reused_service(preparation,sealed,pins)
                images=legacy.fresh_load(legacy.PACKET/'image_closure_v2.py','prolog_probe_runtime_image_validator',pins)
                guard.OUT,guard.REPORT=output,output/'report.json';harness.install_interrupt_handlers();harness.compact.wait_for_environment(harness.REQUIRED)
                environment=dict(harness.compact.ENV);environment['SWI_HOME_DIR']=str(home);verify(pins)
                result_path=output/'runtime/feature-result.json';receipt_path=output/'runtime/feature-probe.json'
                assert result_path!=receipt_path and not result_path.exists() and not receipt_path.exists()
                command=preparation.prefix(binary)+['-l',str(P/'runtime_probe.pl'),'-g','clbg_runtime_probe:main','--',str(result_path)]
                probe=service.owned_command(guard,harness,command,environment,output/'runtime',common.fileno(),120,'feature-probe')
                assert probe['detail']['exit_status']==probe['detail']['final_exit_status']==0
                actual=json.loads(result_path.read_text());contracts=preparation.module(legacy.ORIGINAL_PACKET/'contracts.py','contracts',pins)
                contracts.runtime_flags(actual['runtime'],home);large=(1<<220)+12345
                assert type(actual['large']) is int and actual['large']==large and type(actual['quotient']) is int and actual['quotient']==large//97
                assert type(actual['remainder']) is int and actual['remainder']==large%97
                assert actual['negative_floor']==-3 and actual['negative_modulo']==2 and actual['square_root']==2.0
                assert (actual['hash_value'],actual['hash_size'])==(5,4) and actual['hash_capacity']>=8
                assert actual['regex_matches']==2 and actual['regex_replacement']=='xbx' and actual['regex_jit_requested'] is False and type(actual['pcre_version']) is str
                foreign=[]
                for row in actual['foreign_libraries']:
                    path=Path(row['resolved_path']).resolve(strict=True);assert path.is_relative_to(native.resolve());foreign.append(path)
                image_pins,visited=images.inspect_closure(preparation,service,guard,harness,binary,foreign,environment,output/'images',common.fileno(),native)
                pins=merge(pins,image_pins,service.pin_tree(native),service.pin_tree(original_app),service.pin_tree(post.PRIOR/'official-source'))
                allowed_helpers={P/'runtime_probe.pl',P/'prepare_without_main.pl'}
                for name in actual['loaded_sources']:
                    path=Path(name).resolve(strict=True);assert path.is_relative_to(native.resolve()) or path in allowed_helpers
                    pins=merge(pins,{str(path):sha(path)})
                layout=json.loads((post.REVIEW.parent/'native-layout.json').read_text())
                assert {str(path.relative_to(native)) for path in native.rglob('*')}=={row['relative'] for row in layout}
                report.update(status='runtime-discovered-awaiting-root-source-library-review',binary=str(binary),binary_sha256=sha(binary),home=str(home),architecture='arm64',
                    version_components=[10,0,2],runtime_backend='swi-native-bytecode-vm',allowed_library_roots=[str(native)],environment={'SWI_HOME_DIR':str(home)},
                    version_receipt=legacy.actual_receipt(preparation,PRIOR/'runtime/version.json',[str(binary),'--version']),
                    reused_closed_postslice=dict(failed_report=str(PRIOR/'report.json'),failed_report_sha256=FAILED_SHA,failed_terminal_sha256=FAILED_TERMINAL_SHA,
                        raw_failed_probe_result=str(PRIOR/'runtime/feature-probe.json'),raw_failed_probe_result_sha256=RAW_SHA,raw_probe_relabelled_as_success=False,original_version_reused=True),
                    archive_or_existing_installation_receipt=dict(status='passed',archive=str(post.PRIOR/'swipl-10.0.2-1.fat.dmg'),sha256=legacy.DMG_SHA,
                        official_source_archive=str(post.PRIOR/'swipl-10.0.2.tar.gz'),official_source_archive_sha256=legacy.SOURCE_SHA,original_fat_binary=str(fat),arm64_slice_receipt=str(post.PRIOR/'slice/arm64.json')),
                    actual_feature_probe=str(result_path),actual_feature_probe_sha256=sha(result_path),feature_probe_owned_receipt=legacy.actual_receipt(preparation,receipt_path,command),
                    loaded_native_images=visited,default_gc_observed=actual['runtime']['gc'],bounded_observed=actual['runtime']['bounded'],pcre_version=actual['pcre_version'])
            except BaseException as failure:error=failure
            finally:
                if legacy is not None:legacy.terminal(output,report,pins,error)
                else:
                    failures=[]
                    for path,digest in pins.items():
                        try:assert sha(path)==digest
                        except BaseException as failure:failures.append(dict(path=path,error=str(failure) or type(failure).__name__))
                    report.update(status='failed-terminal-integrity' if failures else 'stopped-unqualified',pins=pins,terminal_evidence_complete=not failures,terminal_artifact_errors=failures)
                    if error:report.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                    save(output/'report.json',report);save(output/'terminal-pin.json',dict(status=report['status'],report=str(output/'report.json'),report_sha256=sha(output/'report.json'),partial_evidence_pins=pins,inherited_common_mutex=True,chart_admission=False,full_processes_authorized=0))
                    if failures:raise RuntimeError('Early probe terminal integrity failed')
            if error is not None:raise error
            print(json.dumps(dict(status=report['status'],report=str(output/'report.json'),report_sha256=sha(output/'report.json'),configuration_admitted=False)),flush=True)

if __name__=='__main__':main()

"""Stage reviewed C3 data after source opening; retain common through publication.

No benchmark, preparation or fixture runs. Source candidate only: root/peer
full review, frozen actual arguments and supported source opening precede any
live data write. The supported Sites helper reads its own credential input;
this coordinator never reads, captures or writes credential JSON or files.
"""
import argparse
import fcntl
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
LOCK = Path('/tmp/mech-clbg-timing.lock')
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
NODE = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')
HELPER = Path('/Users/cim310/.codex/plugins/cache/openai-curated-remote/sites/1.0.1/scripts/site-workflow.mjs')
PROJECT = 'appgprj_6ac8f49664688191961eac14d8a9465b'
BASELINE = HERE / 'jobs/current-chart-session-v2/candidate-data.json'
BASELINE_SHA = '27b1c792b253d3d465ca6f853e7f85c9d089cb87e3b161a634feb7b5c1f69c8e'
CANDIDATE = HERE / 'jobs/c3-current-candidate-v1/candidate-data.json'
CANDIDATE_SHA = '4fe402c21360fbbbe9d650c15d1fd4c62422c17cfad8cafd24721fe7f0bd2947'
OUTCOME = HERE / 'jobs/c3-current-candidate-v1/outcome.json'
REVIEW = HERE / 'jobs/c3-current-candidate-actual-review-v1/receipt.json'
OPENING = HERE / 'jobs/c3-source-opening-v1/receipt.json'
DEPENDENCY = HERE / 'owned-jobs/c3-current-candidate-v1'
PRESERVATION = HERE / 'qualify_c3_current_candidate_v1.py'
PRESERVATION_SHA = '302638172799a25dfd13b0808ea3da9451fbb53bd03a68f1d7f3360487772cc7'
JOURNAL = HERE / 'jobs/c3-chart-session-v1'


def run(args):
    with (HERE / 'c3-chart-session-v1-owner.lock').open('a+') as owner:
        fcntl.flock(owner,fcntl.LOCK_EX | fcntl.LOCK_NB)
        held,named = os.fstat(owner.fileno()), (HERE / 'c3-chart-session-v1-owner.lock').stat()
        assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
        print('WAIT closed C3 candidate lease',flush=True)
        with (DEPENDENCY / 'owner.lock').open('a+') as dependency:
            fcntl.flock(dependency,fcntl.LOCK_EX)
            held,named = os.fstat(dependency.fileno()), (DEPENDENCY / 'owner.lock').stat()
            assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
            print('WAIT common mutex: reviewed C3 stage, QA, packaging and publication',flush=True)
            with LOCK.open('a+') as common:
                fcntl.flock(common,fcntl.LOCK_EX)
                held,named = os.fstat(common.fileno()), LOCK.stat()
                assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
                import hashlib
                import http.server
                import json
                import signal
                import stat
                import subprocess
                import threading
                import traceback
                import types

                def sha(path):
                    with Path(path).open('rb') as stream:
                        return hashlib.file_digest(stream,'sha256').hexdigest()
                def save(path,value):
                    with path.open('x') as stream:
                        json.dump(value,stream,indent=2);stream.write('\n')
                pins = {}
                def bind(path,expected=None):
                    path = Path(path).resolve(strict=True);digest = sha(path)
                    assert expected is None or digest == expected, 'Changed reviewed input: ' + str(path)
                    assert str(path) not in pins or pins[str(path)] == digest, 'Conflicting input pin'
                    pins[str(path)] = digest;return digest
                def read(path,expected=None):
                    bind(path,expected);return json.loads(Path(path).read_text())
                def verify():
                    for path,digest in list(pins.items()):bind(path,digest)
                def alarm(_number,_frame):raise TimeoutError('C3 pre-helper chart session exceeded 1200 seconds')
                server,thread,helper = None,None,None
                previous_handler = signal.getsignal(signal.SIGALRM)
                created = False
                retained = None
                try:
                    retained = fcntl.fcntl(common.fileno(),fcntl.F_DUPFD,64)
                    os.set_inheritable(retained,True)
                    assert not JOURNAL.exists(), 'Exclusive new session; no retry or overwrite'
                    JOURNAL.mkdir(parents=True);created = True
                    signal.signal(signal.SIGALRM,alarm);signal.alarm(1200)
                    for path in (Path(__file__).resolve(),HERE / 'C3_CHART_SESSION_V1_REVIEW.md',CANDIDATE,OUTCOME,REVIEW,OPENING,PRESERVATION):
                        assert path.is_file() and not path.stat().st_mode & 0o222, 'Reviewed frozen source and actual data required'
                    bind(__file__,args.source_sha256)
                    bind(HERE / 'C3_CHART_SESSION_V1_REVIEW.md',args.review_sha256)
                    review = read(REVIEW,args.closure_review_sha256)
                    opening = read(OPENING,args.source_opening_sha256)
                    assert opening['status'] == 'supported-source-opening-completed' and opening['project_id'] == PROJECT and opening['checkout_path'] == str(SITE)
                    assert opening['commit_sha'] == 'c23339fe74db3623db8409d92826dc0d64b3b853'
                    assert type(opening['source_helper_exit_code']) is int and opening['source_helper_exit_code'] == 0
                    assert opening['shared_timing_mutex_held_by_parent_through_helper_exit'] is opening['owner_private_audience_unchanged'] is True
                    assert opening['external_visitors'] == opening['site_data_mutations'] == opening['benchmark_runs'] == 0
                    assert opening['credentials_kept_only_in_session_memory_and_hidden_stdin'] is True
                    assert review['status'] == 'actual-normal-C3-current-candidate-closure-reviewed'
                    assert review['actual_exit_code'] == 0 and review['actual_C3_source_forms'] == 20 and review['actual_full_processes_reused'] == 90
                    assert review['fixture_reexecutions'] == review['live_dataset_mutations'] == 0 and review['chart_admission'] is False
                    assert review['old_records_and_fixed_baselines_unchanged'] is True and review['common_mutex_held'] is True
                    assert review['candidate'] == dict(path=str(CANDIDATE),sha256=CANDIDATE_SHA)
                    for path,digest in review['pins'].items():bind(path,digest)
                    outcome = read(OUTCOME,review['original_outcome']['sha256'])
                    assert outcome['status'] == 'qualified-current-C3-candidate-awaiting-root-stage-review' and outcome['chart_admission'] is False
                    assert outcome['native_commands'] == outcome['fixture_reexecutions'] == outcome['live_dataset_mutations'] == 0
                    assert not any(outcome.get(key) for key in ('error','error_type','error_log_write_failure','terminal_pin_errors','available_bytes_not_fully_pinned'))
                    for values in (outcome['pins'],outcome['available_evidence_pins']):
                        for path,digest in values.items():bind(path,digest)
                    terminal = read(DEPENDENCY / 'terminal.json',review['durable_terminal']['sha256'])
                    request,started = read(DEPENDENCY / 'request.json'),read(DEPENDENCY / 'command-started.json')
                    assert terminal['status'] == 'command-exited-zero-awaiting-report-review' and type(terminal['exit_code']) is int and terminal['exit_code'] == 0
                    assert terminal['actual_command_started'] is True and terminal['command_identity_observation_missing'] is False and terminal['stop_signals'] == []
                    assert not any(terminal.get(key) for key in ('error','error_type','exception_cleanup','error_log_write_failure'))
                    assert request['command'] == started['command'] and request['cwd'] == started['cwd'] == str(CLBG.parent.parent)
                    assert started['command_pid'] == terminal['command_pid'] and started['command_identity'] == terminal['command_identity']
                    baseline = read(BASELINE,BASELINE_SHA);candidate = read(CANDIDATE,CANDIDATE_SHA)
                    bind(PRESERVATION,PRESERVATION_SHA)
                    raw = PRESERVATION.read_bytes();assert hashlib.sha256(raw).hexdigest() == PRESERVATION_SHA
                    name = 'c3_stage_preservation_service';assert name not in sys.modules
                    module = types.ModuleType(name);module.__file__ = str(PRESERVATION);sys.modules[name] = module
                    sys.dont_write_bytecode = True
                    exec(compile(raw,str(PRESERVATION),'exec'),module.__dict__)
                    assert Path(module.__file__).resolve() == PRESERVATION.resolve()
                    module.preserve(baseline,candidate)
                    inventory = {key:row for key,row in candidate['program_inventory'].items() if row['language'] == 'c3'}
                    assert len(inventory) == len({row['id'] for row in inventory.values()}) == 20
                    assert sum(row['language'] == 'c3' for row in candidate['programs'].values()) == review['admitted_endpoint_programs'] == 19
                    for key,row in inventory.items():
                        benchmark = next(item for item in baseline['benchmarks'] if item['key'] == row['benchmark'])
                        assert len(row['samples']) == 3 and row['x'] == row['loc']/benchmark['baseline_loc'] and row['y'] == row['seconds']/benchmark['baseline_seconds']
                        assert row['files'] and all(identity in candidate['files'] for identity in row['files'])
                        if key in candidate['programs']:assert candidate['programs'][key] == row
                    root_data,site_data = CLBG / 'overview/explorer/dist/data.json',SITE / 'dist/data.json'
                    assert str(root_data.resolve()) not in pins and str(site_data.resolve()) not in pins, 'Live observations cannot invalidate frozen inputs after staging'
                    assert root_data.read_bytes() == site_data.read_bytes() == BASELINE.read_bytes(), 'Current exact published baseline differs'
                    immutable = JOURNAL / 'input-data.json'
                    with immutable.open('xb') as stream:stream.write(BASELINE.read_bytes())
                    immutable.chmod(0o444);bind(immutable,BASELINE_SHA)
                    bind(NODE.resolve());bind(HELPER.resolve())
                    save(JOURNAL / 'started.json',dict(status='qualified-inputs-awaiting-supported-source-opening',
                        candidate_sha256=CANDIDATE_SHA,published_baseline_sha256=BASELINE_SHA,
                        common_mutex_fd=retained,common_mutex_inode=named.st_ino,pins=dict(pins),
                        native_benchmarks_started=0,fixture_reexecutions=0,live_dataset_mutations=0,credentials_received=False,automatic_retry=False))
                    print('INPUTS AND ACTUAL SUPPORTED SOURCE OPENING QUALIFIED. Send opened-source to stage the accepted data. No data has been staged.',flush=True)
                    assert sys.stdin.readline().strip() == 'opened-source', 'Expected nonsecret supported-source-opening completion'
                    verify()
                    assert root_data.read_bytes() == site_data.read_bytes() == BASELINE.read_bytes(), 'Opened Site source baseline differs'
                    save(JOURNAL / 'source-opening-complete.json',dict(status='root-attested-supported-source-opening-complete',
                        project_id=PROJECT,baseline_sha256=BASELINE_SHA,actual_receipt=str(OPENING),
                        actual_receipt_sha256=args.source_opening_sha256,credentials_read_by_coordinator=False))
                    candidate_bytes = CANDIDATE.read_bytes();assert hashlib.sha256(candidate_bytes).hexdigest() == CANDIDATE_SHA
                    staged = []
                    for target in (root_data,site_data):
                        temporary = target.with_name('.data.c3-chart-session-v1.tmp')
                        mode = stat.S_IMODE(target.stat().st_mode)
                        with temporary.open('xb') as stream:
                            stream.write(candidate_bytes);stream.flush();os.fsync(stream.fileno())
                        temporary.chmod(mode);os.replace(temporary,target)
                        assert sha(target) == CANDIDATE_SHA;staged.append(str(target))
                        save(JOURNAL / ('stage-'+str(len(staged))+'.json'),dict(path=str(target),sha256=CANDIDATE_SHA))
                    save(JOURNAL / 'qualified-stage.json',dict(status='qualified-C3-staged-awaiting-browser-QA',candidate=str(CANDIDATE),
                        candidate_sha256=CANDIDATE_SHA,staged_paths=staged,C3_source_inventory=20,C3_endpoint_programs=19,
                        old_records_and_fixed_baselines_unchanged=True,native_benchmarks_started=0,fixture_reexecutions=0,audience_changes=0))
                    print('QUALIFIED C3 DATA STAGED. After root copies reviewed UI, send preview.',flush=True)
                    assert sys.stdin.readline().strip() == 'preview', 'Expected nonsecret reviewed UI handoff'
                    assert sha(root_data) == sha(site_data) == CANDIDATE_SHA
                    assets = {str(path.resolve()):sha(path) for path in sorted((SITE / 'dist').rglob('*')) if path.is_file()}
                    class Handler(http.server.SimpleHTTPRequestHandler):
                        def __init__(self,*values,**options):super().__init__(*values,directory=str(SITE / 'dist'),**options)
                        def log_message(self,_format,*_values):pass
                    server = http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
                    thread = threading.Thread(target=server.serve_forever,daemon=True);thread.start()
                    url = 'http://127.0.0.1:'+str(server.server_address[1])+'/'
                    save(JOURNAL / 'preview.json',dict(url=url,source_asset_pins=assets,common_mutex_fd=retained))
                    print('READY '+url,flush=True)
                    print('After browser QA, send package. Use refresh-ui after root-reviewed UI changes. Credentials go only to the supported helper.',flush=True)
                    while True:
                        command = sys.stdin.readline().strip()
                        if command == 'package':break
                        assert command == 'refresh-ui', 'Expected nonsecret package or refresh-ui'
                        assert sha(root_data) == sha(site_data) == CANDIDATE_SHA
                        assets = {str(path.resolve()):sha(path) for path in sorted((SITE / 'dist').rglob('*')) if path.is_file()}
                        number = len(list(JOURNAL.glob('ui-refresh-*.json'))) + 1
                        save(JOURNAL / ('ui-refresh-'+str(number)+'.json'),dict(status='root-reviewed-UI-refreshed-awaiting-repeat-browser-QA',
                            source_asset_pins=assets,candidate_sha256=CANDIDATE_SHA))
                        print('UI REFRESHED. Repeat affected browser/syntax checks before package.',flush=True)
                    verify()
                    for path,digest in assets.items():assert sha(path) == digest, 'Reviewed asset changed'
                    assert sha(root_data) == sha(site_data) == CANDIDATE_SHA
                    server.shutdown();server.server_close();server = None
                    thread.join(timeout=10);assert not thread.is_alive();thread = None
                    held,named = os.fstat(retained),LOCK.stat();assert (held.st_dev,held.st_ino) == (named.st_dev,named.st_ino)
                    fcntl.flock(retained,fcntl.LOCK_EX | fcntl.LOCK_NB)
                    environment = dict(os.environ)
                    environment['PATH'] = str(NODE.parent)+os.pathsep+environment.get('PATH','/usr/bin:/bin:/usr/sbin:/sbin')
                    save(JOURNAL / 'packaging-handoff.json',dict(status='entering-supported-Sites-helper',
                        node=str(NODE),helper=str(HELPER),project_id=PROJECT,common_mutex_fd=retained,
                        bundled_node_PATH_prefix=str(NODE.parent),candidate_sha256=CANDIDATE_SHA,
                        credentials_received=False,audience_changes=0))
                    signal.alarm(0);signal.signal(signal.SIGALRM,previous_handler)
                    # Direct inherited terminal: no credential reads or capture.
                    helper = subprocess.Popen([str(NODE),str(HELPER),'--project-id',PROJECT],cwd=str(SITE),
                        env=environment,close_fds=True,pass_fds=(retained,))
                    save(JOURNAL / 'helper-started.json',dict(pid=helper.pid,common_mutex_fd=retained,
                        credentials_read_by_coordinator=False,supported_helper_owns_site_git=True))
                    helper_exit = helper.wait()
                    save(JOURNAL / 'helper-completed.json',dict(exit_code=helper_exit,common_mutex_retained=True,credentials_read_by_coordinator=False))
                    assert type(helper_exit) is int and helper_exit == 0, 'Supported Sites helper failed'
                    print('SITE WORKFLOW COMPLETE. Publish and save the authorized checkpoint, then send release.',flush=True)
                    assert sys.stdin.readline().strip() == 'release', 'Expected nonsecret release after publication/checkpoint'
                    verify();assert sha(root_data) == sha(site_data) == CANDIDATE_SHA
                    save(JOURNAL / 'released.json',dict(status='root-authorized-C3-publication-and-checkpoint-complete',
                        candidate_sha256=CANDIDATE_SHA,helper_exit_code=helper_exit,native_benchmarks_started=0,fixture_reexecutions=0,
                        audience_changes=0,credentials_read_by_coordinator=False))
                except BaseException as error:
                    if created:
                        failure = dict(status='failed-with-preserved-C3-session-evidence',error=str(error) or type(error).__name__,
                            error_type=type(error).__name__,native_benchmarks_started=0,fixture_reexecutions=0,automatic_retry=False)
                        if helper is not None and helper.poll() is None:
                            try:
                                helper.terminate()
                                try:helper.wait(timeout=10)
                                except subprocess.TimeoutExpired:helper.kill();helper.wait(timeout=10)
                                failure['owned_helper_stopped_and_reaped'] = True
                            except BaseException as cleanup_error:
                                failure['owned_helper_cleanup_error'] = str(cleanup_error) or type(cleanup_error).__name__
                        try:save(JOURNAL / 'failure.json',failure)
                        except BaseException:pass
                    raise
                finally:
                    try:
                        signal.alarm(0);signal.signal(signal.SIGALRM,previous_handler)
                        if server is not None:
                            try:
                                if thread is not None and thread.is_alive():server.shutdown()
                            finally:server.server_close()
                        if thread is not None and thread.ident is not None:thread.join(timeout=10)
                    finally:
                        if retained is not None:os.close(retained)


def main():
    parser = argparse.ArgumentParser()
    for name in ('source-sha256','review-sha256','closure-review-sha256','source-opening-sha256'):parser.add_argument('--'+name,required=True)
    args = parser.parse_args()
    assert all(len(value) == 64 and all(char in '0123456789abcdef' for char in value) for value in vars(args).values())
    run(args)


if __name__ == '__main__':main()

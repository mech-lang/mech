"""Package the unchanged, already verified chart with bundled Node on PATH."""
import fcntl
import os
from pathlib import Path
import sys

HERE = Path(__file__).resolve().parent
CLBG = HERE.parent
SITE = Path('/Users/cim310/.codex/visualizations/2026/10/08/01a11c4b-7bf0-75b0-a854-cf4f0e614617/benchmark-explorer-site')
NODE = Path('/Users/cim310/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node')
HELPER = '/Users/cim310/.codex/plugins/cache/openai-curated-remote/sites/1.0.1/scripts/site-workflow.mjs'
PROJECT = 'appgprj_6ac8f49664688191961eac14d8a9465b'
LOCK = Path('/tmp/mech-clbg-timing.lock')

print('WAIT common mutex: package verified current chart; no repeated checks or benchmarks', flush=True)
with LOCK.open('a+') as common:
    fcntl.flock(common, fcntl.LOCK_EX)
    held, named = os.fstat(common.fileno()), LOCK.stat()
    assert (held.st_dev, held.st_ino) == (named.st_dev, named.st_ino)
    import hashlib
    import json
    import subprocess

    def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
    def read(path): return json.loads(Path(path).read_bytes())
    def save(path, value):
        with path.open('x') as stream: json.dump(value, stream, indent=2); stream.write('\n')

    prior = HERE/'jobs/current-chart-session-v2'
    output = HERE/'jobs/current-chart-package-v4'
    assert not output.exists(); output.mkdir()
    stage = read(prior/'qualified-stage.json')
    assert stage['status'] == 'qualified-and-staged-awaiting-browser-QA'
    assert stage['retained_twenty_source_inventory'] is True
    assert stage['old_records_and_fixed_baselines_unchanged'] is True
    assert sha(stage['candidate']) == stage['candidate_sha256']
    assert all(sha(path) == stage['candidate_sha256'] for path in stage['staged_paths'])
    refresh = read(prior/'ui-refresh-2.json')
    assets = refresh['source_asset_pins']
    assert set(assets) == {str(path.resolve()) for path in (SITE/'dist').rglob('*') if path.is_file()}
    assert all(sha(path) == digest for path, digest in assets.items())
    assert read(prior/'helper-completed.json')['exit_code'] == 127
    assert 'Supported Sites helper failed' in read(prior/'failure.json')['error']
    assert read(SITE/'.openai/hosting.json')['project_id'] == PROJECT
    assert (HERE/'language-overview-ui-v1/BROWSER_QA.md').is_file()
    environment = dict(os.environ)
    environment['PATH'] = str(NODE.parent) + os.pathsep + environment.get('PATH', '')
    fd = fcntl.fcntl(common.fileno(), fcntl.F_DUPFD, 64)
    os.set_inheritable(fd, True)
    try:
        save(output/'verified-inputs.json', dict(status='unchanged-verified-chart-packaging-recovery',
            candidate_sha256=stage['candidate_sha256'], source_asset_pins=assets,
            browser_qa_sha256=sha(HERE/'language-overview-ui-v1/BROWSER_QA.md'),
            source_sha256=sha(__file__), bundled_node=str(NODE), common_mutex_inode=named.st_ino,
            fixture_reexecutions=0, native_benchmarks_started=0, audience_changes=0))
        helper = subprocess.Popen([str(NODE), HELPER, '--project-id', PROJECT], cwd=str(SITE),
            env=environment, pass_fds=(fd,))
        code = helper.wait()
        save(output/'helper-completed.json', dict(exit_code=code, credentials_read_by_wrapper=False))
        assert type(code) is int and code == 0, 'Supported Sites helper failed'
        print('SITE WORKFLOW COMPLETE. Publish and save the authorized checkpoint, then send release.', flush=True)
        assert sys.stdin.readline().strip() == 'release'
        assert all(sha(path) == digest for path, digest in assets.items())
        assert all(sha(path) == stage['candidate_sha256'] for path in stage['staged_paths'])
        save(output/'released.json', dict(status='packaging-recovery-completed-root-release',
            helper_exit_code=code, native_benchmarks_started=0, fixture_reexecutions=0))
    except BaseException as error:
        save(output/'failure.json', dict(status='failed-preserved-no-automatic-retry',
            error=str(error), error_type=type(error).__name__, native_benchmarks_started=0))
        raise
    finally:
        os.close(fd)

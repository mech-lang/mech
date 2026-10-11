"""Compile pinned SBCL cores and Racket CS modules outside measured runs."""
import concurrent.futures
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from prepare import sha
from loc import normalize

D = Path(__file__).resolve().parent
SBCL = Path('/opt/homebrew/bin/sbcl').resolve()
RACKET = (D / 'support/racket/bin/racket').resolve()
RACO = D / 'support/racket/bin/raco'


def build(row):
    row = row.copy()
    source = Path(row['source'])
    directory = D / 'builds' / row['id']
    directory.mkdir(parents=True, exist_ok=True)
    log = D / 'results' / (row['id'] + '.build.log')
    row['loc_audit'] = normalize(source.read_text(), row['language'])
    if row['language'] == 'lisp':
        binary = D / 'binaries' / (row['id'] + '.core')
        script = directory / 'compile.lisp'
        extra = ''
        if row['benchmark'] == 'regexredux':
            extra = '(require :asdf)\n(push #p"' + str(D / 'support/cl-ppcre') + '/" asdf:*central-registry*)\n(asdf:load-system :cl-ppcre)\n'
        script.write_text(extra + '(require :sb-concurrency)\n' +
            '(handler-bind ((sb-ext:defconstant-uneql (lambda (condition) (declare (ignore condition)) (invoke-restart \'continue))))\n' +
            '  (multiple-value-bind (fasl warnings failure) (compile-file ' + json.dumps(str(source)) + ' :output-file ' + json.dumps(str(directory / 'program.fasl')) + ')\n' +
            '    (declare (ignore warnings))\n' +
            '    (when failure (error "Benchmark compilation failed"))\n' +
            '    (load fasl)))\n' +
            '(in-package :cl-user)\n' +
            '(sb-ext:save-lisp-and-die ' + json.dumps(str(binary)) + ' :purify t)\n')
        command = [str(SBCL), '--noinform', '--no-sysinit', '--no-userinit', '--non-interactive', '--load', str(script)]
        runtime = [str(SBCL), '--noinform', '--core', str(binary), '--no-sysinit', '--no-userinit', '--non-interactive', '--eval', '(cl-user::main)', '--end-toplevel-options', '{n}']
    else:
        command = [str(RACO), 'make', str(source)]
        binary = source.parent / 'compiled' / (source.stem + '_rkt.zo')
        runtime = [str(RACKET), str(source), '{n}']
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, cwd=directory)
    log.write_text(result.stdout)
    row.update(build_command=command, build_status='passed' if result.returncode == 0 else 'failed', build_log=str(log))
    if result.returncode == 0:
        assert binary.exists(), binary
        row.update(binary=str(binary), binary_sha256=sha(binary), command=runtime)
    print('BUILD', row['id'], row['build_status'], row['loc_audit']['normalized_loc'], 'LOC', flush=True)
    return row


def main():
    manifest = json.loads((D / 'manifest.json').read_text())
    corrections = json.loads((D / 'corrections.json').read_text())
    assert corrections['manifest_sha256'] == sha(D / 'manifest.json')
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        programs = list(pool.map(build, manifest['programs'] + corrections['programs']))
    runtime_paths = {'sbcl': SBCL, 'racket': RACKET, 'gmp': Path('/opt/homebrew/opt/gmp/lib/libgmp.10.dylib').resolve()}
    for path in (D / 'support/racket/lib').glob('*.dylib'):
        runtime_paths['racket-' + path.name] = path.resolve()
    runtimes = {key: dict(path=str(path), sha256=sha(path)) for key, path in runtime_paths.items()}
    details = dict(sbcl=subprocess.check_output([str(SBCL), '--version'], text=True).strip(),
                   racket=subprocess.check_output([str(RACKET), '--version'], text=True).strip(),
                   cl_ppcre_commit=subprocess.check_output(['git', '-C', str(D / 'support/cl-ppcre'), 'rev-parse', 'HEAD'], text=True).strip(),
                   racket_installer_sha256=sha(D / 'support/racket-9.3-aarch64-macosx.dmg'))
    report = dict(status='built', created_at=datetime.now(timezone.utc).isoformat(), programs=programs,
                  manifest_sha256=sha(D / 'manifest.json'), corrections_sha256=sha(D / 'corrections.json'), runtimes=runtimes,
                  runtime_details=details,
                  runtime_policy='Native Apple Silicon: SBCL 2.6.9 AOT compiled saved cores and Racket 9.3 CS with precompiled modules; original source optimization declarations and worker partitions retained. Default heap and garbage collection. GMP is a general integer library and CL-PPCRE a general regex library. No native benchmark executors.',
                  timing_boundary='Fresh-process wall time includes runtime/core/module loading, input, all computation, complete output to a regular file, runtime compilation/JIT performed during execution, and shutdown. SBCL compile-file/save-core and raco make occur before timing; validation is excluded.')
    (D / 'results/builds.json').write_text(json.dumps(report, indent=2) + '\n')
    print('BUILT', sum(r['build_status'] == 'passed' for r in programs), 'of', len(programs))


if __name__ == '__main__':
    main()

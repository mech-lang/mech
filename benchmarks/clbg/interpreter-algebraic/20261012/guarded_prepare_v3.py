"""Additive compile-input snapshot repair; fresh three-copy bounded preparation."""
import argparse
import fcntl
import importlib.util
import json
import os
import re
from pathlib import Path
import shutil
import signal
import sys

HERE = Path(__file__).resolve().parent
REVISION = HERE / 'revisions/compile-input-v3'
ROOT = HERE.parents[3]
LOCK = Path('/tmp/mech-clbg-timing.lock')
MODE = ROOT / 'benchmarks/clbg/mandelbrot/experiments/20261011-array-state/source-modes-study/compiler-manifest.json'
MODE_SHA = 'ac5758cd755a4e2d964ac89c67d3bb95a3bf99618bbb667100a1fef745163698'
HELPER = ROOT / 'benchmarks/clbg/nim-comparison/20261012/guarded_prepare_v2.py'
HELPER_PROPOSAL = HELPER.parent / 'proposals/private-v2.json'
HELPER_PROPOSAL_SHA = '76e37273079d8a9e8c8541cbe67a36b96a64753d337670a03bf63cd01c65da52'
IMAGE_HELPER = ROOT / 'benchmarks/clbg/pony-comparison/20261012/guarded_prepare.py'
IMAGE_PROPOSAL = IMAGE_HELPER.parent / 'proposals/private-v1.json'
IMAGE_PROPOSAL_SHA = '6a93655535bd56218bb8a95d26ed10e9638617b5da5addf37210ea8bc120a541'
DRIVER = Path('benchmarks/clbg/interpreter-integration/20261011/driver/Cargo.toml')
RESIDENT = Path('hosts/gpu/src/sequence/resident.rs')
MECH_SOURCE = ROOT / 'benchmarks/clbg/work-audit/20261009/sources/spectralnorm-four-functions.mec'
VARIANTS = ('strict', 'reduction', 'broad')
SIZES = (1, 2, 3, 7, 8, 9, 31, 32, 33, 65, 129)
RUNTIME_ENV = dict(MECH_NUMERIC_UNROLL='4', MECH_NUMERIC_ROWS='4', MECH_NUMERIC_INDUCTION='0',
    MECH_ARRAY_GRAIN='8', MECH_SEQUENCE_PARTITIONS='8', RAYON_NUM_THREADS='8', LC_ALL='C', OMP_NUM_THREADS='1',
    OPENBLAS_NUM_THREADS='1', VECLIB_MAXIMUM_THREADS='1')


def module(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    loaded = importlib.util.module_from_spec(spec)
    sys.modules[name] = loaded
    spec.loader.exec_module(loaded)
    return loaded


def helpers():
    # Read immutable reviewed function implementations; change no module globals.
    temporary = module(HELPER, 'fp_reviewed_ownership_helper')
    assert temporary.sha(HELPER_PROPOSAL) == HELPER_PROPOSAL_SHA
    prior = json.loads(HELPER_PROPOSAL.read_text())
    assert temporary.sha(HELPER) == prior['pins'][str(HELPER)]
    assert temporary.sha(IMAGE_PROPOSAL) == IMAGE_PROPOSAL_SHA
    images = json.loads(IMAGE_PROPOSAL.read_text())
    assert temporary.sha(IMAGE_HELPER) == images['pins'][str(IMAGE_HELPER)]
    image_helper = module(IMAGE_HELPER, 'fp_reviewed_image_helper')
    guard = temporary.load_guard()
    harness = guard.load_harness()
    harness.install_interrupt_handlers()
    return temporary, image_helper, guard, harness


def seal():
    print('WAIT LOCK: algebraic interpreter metadata', flush=True)
    with LOCK.open('a+') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        helper, _, guard, harness = helpers()
        REVISION.mkdir(parents=True, exist_ok=True)
        proposal = REVISION / 'proposal.json'
        assert not proposal.exists(), 'Do not overwrite a reviewed or failed proposal'
        assert helper.sha(MODE) == MODE_SHA
        old_proposal = HERE / 'proposal.json'
        assert helper.sha(old_proposal) == 'a3354b41609710a22aeacb24ba3d5d4c66803b0acae92cfa065472d1f5912317'
        old_report = json.loads((HERE / 'preparation/report.json').read_text())
        assert old_report['status'] == 'stopped-unqualified' and old_report['attempts'] == []
        assert old_report['error_type'] == 'AssertionError' and old_report['full_performance_runs'] == 0
        old_build = json.loads((HERE / 'preparation/strict/build/compiler.json').read_text())
        assert old_build['exit_status'] == old_build['final_exit_status'] == 101
        old_metadata = HERE / 'revisions/compile-input-v2/metadata'
        old_fixture = json.loads((old_metadata / 'compile-input-fixtures.json').read_text())
        assert old_fixture['exit_status'] == old_fixture['final_exit_status'] == 1
        assert not (old_metadata.parent / 'proposal.json').exists() and not (old_metadata.parent / 'preparation').exists()
        compiler = json.loads(MODE.read_text())
        inclusion = compile_inputs(helper, compiler['source_pins'])
        ownership_pins = json.loads(HELPER_PROPOSAL.read_text())['pins']
        image_pins = json.loads(IMAGE_PROPOSAL.read_text())['pins']
        pins = helper.merge(compiler['source_pins'], compiler['toolchain_library_pins'],
            ownership_pins, image_pins, inclusion['pins'], helper.pin_tree(HERE / 'preparation'),
            helper.pin_tree(old_metadata), {str(old_proposal): helper.sha(old_proposal)},
            {str(MODE): MODE_SHA, str(HELPER_PROPOSAL): HELPER_PROPOSAL_SHA,
             str(IMAGE_PROPOSAL): IMAGE_PROPOSAL_SHA},
            {str(Path(compiler[key]['path'])): compiler[key]['sha256'] for key in ('cargo', 'rustc')},
            {str(path): helper.sha(path) for path in HERE.iterdir() if path.suffix in ('.rs', '.py', '.md', '.toml')},
            {str(HELPER): helper.sha(HELPER), str(IMAGE_HELPER): helper.sha(IMAGE_HELPER),
             str(MECH_SOURCE): helper.sha(MECH_SOURCE), str(helper.GUARD): helper.GUARD_SHA})
        toolchain = Path(compiler['rustc']['path']).parent.parent
        actual_libraries = {str(path.resolve()): helper.sha(path) for path in (toolchain / 'lib').rglob('*')
                            if path.is_file() and path.suffix in ('.dylib', '.rlib', '.rmeta', '.a', '.so', '.o')}
        pins = helper.merge(pins, actual_libraries)
        for name in ('llvm-dis', 'llvm-objdump'):
            path = helper.CLANG.parent / name
            pins[str(path)] = helper.sha(path)
        core = toolchain / 'lib/rustlib/src/rust/library/core/src'
        for path in (core / 'num/f64.rs', core / 'iter/traits/accum.rs'):
            pins[str(path)] = helper.sha(path)
        sdk = Path(json.loads(HELPER_PROPOSAL.read_text())['sdk'])
        pins = helper.merge(pins, helper.pin_tree(sdk / 'usr/include'), helper.pin_tree(sdk / 'usr/lib'))
        guard.verify(pins)
        fixture_directory = REVISION / 'metadata'
        helper.owned_command(guard, harness, [sys.executable, '-B', str(HERE / 'test_compile_inputs_v3.py')],
            dict(os.environ, MECH_TIMING_LOCK_FD=str(lock.fileno())), fixture_directory, lock.fileno(), 30, 'compile-input-fixtures')
        pins = helper.merge(pins, helper.pin_tree(fixture_directory))
        assert (HERE / 'resident_strict.rs').read_bytes() == (ROOT / RESIDENT).read_bytes()
        assert 'feature = "float_algebraic"' in (core / 'num/f64.rs').read_text()
        helper.new_json(proposal, dict(status='sealed-awaiting-root-approval', created_at=guard.now(), pins=pins,
            source_revision='compile-input-v3', preserved_proposal=str(HERE / 'proposal.json'),
            preserved_failure=str(HERE / 'preparation/report.json'), compile_input_pins=inclusion['pins'],
            compile_input_sites=inclusion['sites'], compile_input_source_roots=inclusion['source_roots'],
            preserved_metadata_failure=str(HERE / 'revisions/compile-input-v2/metadata'),
            metadata_fixture_methods=5,
            variants=VARIANTS, sizes=SIZES, compiler_manifest=str(MODE), sdk=str(sdk),
            cargo=compiler['cargo']['path'], rustc=compiler['rustc']['path'], source=str(MECH_SOURCE),
            actual_compiler_stdlib_library_pins=actual_libraries,
            llvm_dis=str(helper.CLANG.parent / 'llvm-dis'),
            llvm_objdump=str(helper.CLANG.parent / 'llvm-objdump'),
            shared_timing_lock=str(LOCK), maximum_compute_workers=8,
            full_processes_authorized=0, preparation_only=True, never_chart_eligible=True,
            pending_gates=['actual codegen review and meaningful gain', 'actual Mech work/output qualification',
                           'independent full-study manifest review']))
        print(json.dumps(dict(proposal=str(proposal), sha256=helper.sha(proposal))), flush=True)



INCLUDE_SOURCE_ROOTS = tuple(Path(path) for path in (
    'hosts/gpu/src', 'src/abi/src', 'src/bytecode/src', 'src/compute/src',
    'src/core/src', 'src/engine/src', 'src/runtime/src', 'src/stdlib/src',
    'src/syntax/src', 'machines'))


def compile_inputs(helper, source_pins, root=ROOT, selected=INCLUDE_SOURCE_ROOTS):
    """Retain literal compile-time inclusions of the copied dependency sources.

    The only accepted nonliteral form is the existing package-relative
    CARGO_MANIFEST_DIR concat. Unknown forms fail metadata before any compiler.
    Test-only inclusions in these source files are retained too; no test is run.
    """
    inputs, sites = {}, {}
    start = re.compile(r'include_(?:str|bytes)!\s*\(')
    direct = re.compile(r'include_(?:str|bytes)!\s*\(\s*"([^"\n]+)"\s*\)')
    concatenated = re.compile(r'include_(?:str|bytes)!\s*\(\s*concat!\s*\(\s*'
        r'env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\n]+)"\s*,?\s*\)\s*\)')
    for filename in sorted(source_pins):
        source = Path(filename)
        if source.suffix != '.rs' or not source.is_relative_to(root):
            continue
        relative = source.relative_to(root)
        if not any(relative.is_relative_to(prefix) for prefix in selected):
            continue
        text = source.read_text()
        occurrences = []
        for match in direct.finditer(text):
            occurrences.append((match.start(), (source.parent / match[1]).resolve()))
        for match in concatenated.finditer(text):
            package = next((parent for parent in source.parents
                            if parent.is_relative_to(root) and (parent / 'Cargo.toml').is_file()), None)
            assert package is not None, ('Package-relative inclusion lacks a manifest', str(source))
            occurrences.append((match.start(), Path(str(package) + match[1]).resolve()))
        assert {offset for offset, _ in occurrences} == {match.start() for match in start.finditer(text)}, (
            'Unreviewed compile-time inclusion form', str(source))
        for _, target in sorted(occurrences):
            assert target.is_relative_to(root) and target.is_file(), ('Missing/escaping compile-time input', str(target))
            inputs[str(target)] = helper.sha(target)
            sites.setdefault(str(source), []).append(str(target))
    assert inputs, 'Dependency snapshot has no recognized compile-time inputs'
    return dict(pins=inputs, sites=sites, source_roots=[str(path) for path in selected])


def snapshot(helper, pins, destination):
    assert not destination.exists()
    destination.mkdir(parents=True)
    for filename, digest in pins.items():
        path = Path(filename)
        if path.is_relative_to(ROOT) and not path.is_relative_to(HERE):
            copied = destination / path.relative_to(ROOT)
            copied.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, copied)
            assert helper.sha(copied) == digest


def prepare(digest):
    print('WAIT LOCK: three-copy algebraic interpreter preparation', flush=True)
    REVISION.mkdir(parents=True, exist_ok=True)
    with (REVISION / 'owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            helper, image_helper, guard, harness = helpers()
            proposal_path = REVISION / 'proposal.json'
            assert helper.sha(proposal_path) == digest, 'Exact reviewed proposal required'
            proposal = json.loads(proposal_path.read_text())
            guard.verify(proposal['pins'])
            output = REVISION / 'preparation'
            assert not output.exists(), 'No implicit resume or overwrite of a failed revision'
            output.mkdir()
            guard.OUT = output
            guard.REPORT = output / 'report.json'
            report = dict(status='preparing', variants=[], attempts=[], active=None,
                          proposal_sha256=digest, full_performance_runs=0, preparation_only=True,
                          never_chart_eligible=True, no_automatic_retry=True)
            guard.save(guard.REPORT, report)
            pins = helper.merge(proposal['pins'], {str(proposal_path): digest})
            try:
                references = module(HELPER.parent / 'references.py', 'fp_independent_spectral_reference')
                for variant in proposal['variants']:
                    detail = dict(variant=variant, status='building', started_at=guard.now())
                    report['variants'].append(detail)
                    guard.save(guard.REPORT, report)
                    project = REVISION / '.build' / variant / 'workspace'
                    snapshot(helper, helper.merge(json.loads(MODE.read_text())['source_pins'], proposal['compile_input_pins']), project)
                    (project / RESIDENT).write_bytes((HERE / ('resident_' + variant + '.rs')).read_bytes())
                    lib = project / 'hosts/gpu/src/lib.rs'
                    if variant != 'strict':
                        lib.write_text('#![feature(float_algebraic)]\n' + lib.read_text())
                    # An unscored, opt-in binary reaches the real private tape
                    # functions without compiling unrelated library test suites.
                    manifest = project / DRIVER
                    manifest.write_text(manifest.read_text() + '\n[[bin]]\nname = "mech-algebraic-conformance"\npath = "../../../../../hosts/gpu/src/bin/mech-algebraic-conformance.rs"\n')
                    gpu_manifest = project / 'hosts/gpu/Cargo.toml'
                    gpu_manifest.write_text(gpu_manifest.read_text().replace('[features]\n',
                        '[features]\nalgebraic-conformance = []\n', 1))
                    fixture = (HERE / 'conformance.rs').read_text().replace('#[test]\n', '').replace(
                        'fn algebraic_experiment_', 'pub(super) fn algebraic_experiment_')
                    resident = project / RESIDENT
                    resident.write_text(resident.read_text() + '\n#[cfg(feature = "algebraic-conformance")]\nmod fp_conformance {\n' +
                        f'const MODE: &str = "{variant}";\n' + fixture + '\n}\n' +
                        '#[cfg(feature = "algebraic-conformance")]\npub(super) fn run_algebraic_conformance() {\n' +
                        'fp_conformance::algebraic_experiment_conformance();\n' +
                        'fp_conformance::algebraic_experiment_axes_tails_and_integer_errors();\n}\n')
                    sequence = project / 'hosts/gpu/src/sequence/mod.rs'
                    sequence.write_text(sequence.read_text() + '\n#[cfg(feature = "algebraic-conformance")]\n' +
                        'pub fn run_algebraic_conformance() { resident::run_algebraic_conformance(); }\n')
                    (project / 'hosts/gpu/src/bin/mech-algebraic-conformance.rs').write_text(
                        'fn main() { mech_gpu::sequence::run_algebraic_conformance(); }\n')
                    pins = helper.merge(pins, helper.pin_tree(project))
                    target = project.parent / 'target'
                    environment = dict(os.environ, RUSTC=proposal['rustc'], RUSTC_BOOTSTRAP='1',
                        CARGO_TARGET_DIR=str(target), CARGO_BUILD_JOBS='2', SDKROOT=proposal['sdk'],
                        MACOSX_DEPLOYMENT_TARGET='14.4', C_INCLUDE_PATH=str(helper.GMP / 'include'),
                        LIBRARY_PATH=str(helper.GMP / 'lib'), DYLD_LIBRARY_PATH=str(helper.GMP / 'lib'),
                        RUSTFLAGS='-D warnings -C target-cpu=native -L native=' + str(helper.GMP / 'lib') +
                                  ' -C save-temps')
                    for key in ('CARGO_ENCODED_RUSTFLAGS', 'CARGO_HOME', 'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER',
                                'DYLD_INSERT_LIBRARIES', 'DYLD_FRAMEWORK_PATH', 'DYLD_FALLBACK_LIBRARY_PATH'):
                        environment.pop(key, None)
                    environment['PATH'] = str(Path(proposal['rustc']).parent) + ':/usr/bin:/bin'
                    base = [proposal['cargo'], 'build', '--release', '--offline', '--locked', '--manifest-path', str(manifest)]
                    command = base + ['--bin', 'mech-interpreter-integrated']
                    helper.owned_command(guard, harness, command, environment, output / variant / 'build',
                                         lock.fileno(), 1800, 'compiler')
                    binary = target / 'release/mech-interpreter-integrated'
                    pins = helper.merge(pins, {str(binary): helper.sha(binary)}, image_helper.native_images(
                        guard, harness, binary, environment, output / variant / 'images', lock.fileno(),
                        [Path(proposal['rustc']).parent.parent, helper.GMP, helper.CLANG.parent.parent]))
                    bitcode = [path for path in sorted((target / 'release/deps').glob('mech_gpu*.bc'))
                               if '.no-opt.' not in path.name]
                    assert bitcode, 'Actual GPU optimized LLVM bitcode was not retained'
                    codegen = []
                    for index, path in enumerate(bitcode):
                        llvm_ir = path.with_suffix('.ll')
                        helper.owned_command(guard, harness, [proposal['llvm_dis'], str(path), '-o', str(llvm_ir)],
                            environment, output / variant / 'codegen', lock.fileno(), 120, f'ir-{index:03d}')
                        codegen.extend((path, llvm_ir))
                    helper.owned_command(guard, harness, [proposal['llvm_objdump'], '--disassemble', '--demangle',
                        '--no-show-raw-insn', str(binary)], environment, output / variant / 'codegen',
                        lock.fileno(), 120, 'arm-disassembly')
                    codegen.append(output / variant / 'codegen/arm-disassembly.stdout')
                    pins = helper.merge(pins, {str(path): helper.sha(path) for path in codegen})
                    detail.update(binary=str(binary), binary_sha256=pins[str(binary)], source_copy=str(resident),
                                  codegen_files=[str(path) for path in codegen], build_command=command,
                                  runtime_environment=RUNTIME_ENV)
                    helper.owned_command(guard, harness, base + ['--features', 'mech-gpu/algebraic-conformance',
                        '--bin', 'mech-algebraic-conformance'], environment, output / variant / 'conformance-build',
                        lock.fileno(), 1800, 'compiler')
                    # Recheck the first exact production binary and source pins
                    # after the separate feature build; never replace its proof.
                    guard.verify(pins)
                    conformance_binary = target / 'release/mech-algebraic-conformance'
                    pins = helper.merge(pins, {str(conformance_binary): helper.sha(conformance_binary)},
                        image_helper.native_images(guard, harness, conformance_binary, environment,
                            output / variant / 'conformance-images', lock.fileno(),
                            [Path(proposal['rustc']).parent.parent, helper.GMP, helper.CLANG.parent.parent]))
                    conformance_command = [str(conformance_binary)]
                    detail.update(conformance_binary=str(conformance_binary),
                                  conformance_binary_sha256=pins[str(conformance_binary)],
                                  conformance_command=conformance_command)
                    guard.verify(pins)
                    helper.owned_command(guard, harness, conformance_command,
                        dict(environment, **RUNTIME_ENV), output / variant / 'conformance', lock.fileno(), 120, 'native')
                    pins = helper.merge(pins, helper.pin_tree(output / variant / 'conformance'),
                        helper.pin_tree(output / variant / 'conformance-build'),
                        helper.pin_tree(output / variant / 'conformance-images'))
                    detail.update(conformance_receipt=str(output / variant / 'conformance/native.json'),
                                  conformance_stdout=str(output / variant / 'conformance/native.stdout'),
                                  conformance_stderr=str(output / variant / 'conformance/native.stderr'))
                    guard.verify(pins)
                    row = dict(id='mech-algebraic-' + variant, key='spectralnorm:mech-algebraic-' + variant,
                        benchmark='spectralnorm', language='mech', source=proposal['source'],
                        source_sha256=pins[proposal['source']], binary_sha256=pins[str(binary)],
                        command=[str(binary), proposal['source'], 'resident', '{n}'], cwd=str(ROOT),
                        runtime_environment=dict(RUNTIME_ENV))
                    for n in proposal['sizes']:
                        helper.probe(guard, harness, row, n, None, references.spectralnorm(n), pins,
                                     output, lock.fileno(), report, 'n-' + str(n))
                    detail.update(status='bounded-output-passed-pending-codegen-work-and-performance-review', finished_at=guard.now())
                    guard.verify(pins)
                    guard.save(guard.REPORT, report)
                report.update(status='bounded-three-copy-preparation-complete-pending-independent-review',
                              finished_at=guard.now(), pins=pins, full_performance_runs=0)
                guard.save(guard.REPORT, report)
            except BaseException as error:
                report.update(status='stopped-unqualified', error=str(error) or type(error).__name__,
                              error_type=type(error).__name__, finished_at=guard.now())
                guard.save(guard.REPORT, report)
                raise


def main():
    def interrupted(signum, frame):
        raise KeyboardInterrupt('FP preparation interrupted by signal ' + str(signum))
    for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
        signal.signal(signum, interrupted)
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--seal', action='store_true')
    mode.add_argument('--prepare', action='store_true')
    parser.add_argument('--proposal-sha256')
    args = parser.parse_args()
    if args.seal: seal()
    else:
        assert args.proposal_sha256
        prepare(args.proposal_sha256)


if __name__ == '__main__':
    main()

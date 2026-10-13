"""Build the actual derived n-body library; no benchmark/AOT admission here."""
from datetime import datetime, timezone
import fcntl
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import sys
import traceback

HERE = Path(__file__).resolve().parent
CLBG = HERE.parents[1]
LOCK = Path('/tmp/mech-clbg-timing.lock')
BOOTSTRAP = HERE / 'jobs/packagecompiler-bootstrap-v2/report.json'
PREPARATION = CLBG / 'c3-comparison/20261012/revisions/private-v3/guarded_prepare.py'
ORIGINAL = CLBG / 'nbody/upstream/nbody-julia-5.jl'
ORIGINAL_SHA = '2def3e7e9910b46cd522b81041e04bbcede2481f69684b683bea605bef598a54'
ADAPTER = HERE / 'revisions/proposal-v2/source_adapters_candidate.py'
ROLES = HERE / 'revisions/proposal-v1/existing-source-roles.json'
OUTPUT = HERE / 'jobs/nbody-library-pilot-v1'
BUILD = HERE / '.build/nbody-library-pilot-v1'


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def save(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, indent=2)
        stream.write('\n')


def verify(pins):
    for path, value in pins.items():
        assert sha(path) == value, ('Reviewed build evidence changed', path)


def main():
    print('WAIT common mutex: actual Julia n-body library pilot build', flush=True)
    with (HERE / 'nbody-pilot-v1-owner.lock').open('a+') as owner:
        fcntl.flock(owner, fcntl.LOCK_EX | fcntl.LOCK_NB)
        with LOCK.open('a+') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            a, b = os.fstat(lock.fileno()), LOCK.stat()
            assert (a.st_dev, a.st_ino) == (b.st_dev, b.st_ino)
            assert not OUTPUT.exists() and not BUILD.exists()
            OUTPUT.mkdir(parents=True)
            pins = {str(Path(__file__).resolve()):sha(__file__)}
            receipt = dict(status='failed',exclusive_common_timing_mutex_held=True,common_timing_mutex=str(LOCK),
                actual_benchmark_runs=0,actual_workload_executions=0,runtime_aot_proven=False,chart_admission=False,
                automatic_retry=False,started_at=datetime.now(timezone.utc).isoformat())
            try:
                bootstrap = json.loads(BOOTSTRAP.read_text())
                assert bootstrap['status'] == 'exact-packagecompiler-bootstrap-complete-awaiting-library-pilot'
                assert bootstrap['pins_unchanged_at_end'] is True and bootstrap['packagecompiler']['version'] == '2.4.3'
                pins.update(bootstrap['pins']);pins[str(BOOTSTRAP)] = sha(BOOTSTRAP)
                assert str(PREPARATION) in pins
                assert sha(ORIGINAL) == ORIGINAL_SHA
                pins[str(ORIGINAL)] = ORIGINAL_SHA;pins[str(ROLES)] = sha(ROLES)
                roles = json.loads(ROLES.read_text())
                assert roles['expected_workload_roles'] == len(roles['roles']) == 20
                assert roles['expected_distinct_sources'] == 17
                selected = [row for row in roles['roles'] if row['benchmark'] == 'nbody']
                assert len(selected) == 2 and {row['variant'] for row in selected} == {'compact','optimized'}
                for row in selected:
                    assert row['sources'] == [dict(path='nbody/upstream/nbody-julia-5.jl',sha256=ORIGINAL_SHA)]
                    report = CLBG / row['report']
                    assert sha(report) == row['expected_report_sha256']
                    pins[str(report)] = row['expected_report_sha256']
                snapshot = OUTPUT / 'source-snapshot';snapshot.mkdir()
                for source in (ADAPTER,HERE / 'revisions/proposal-v1/entry_template.jl',
                               HERE / 'revisions/proposal-v1/direct_driver.c',
                               HERE / 'revisions/proposal-v2/STATIC_MODULE_REVIEW.md'):
                    target = snapshot / source.name
                    shutil.copyfile(source,target)
                    assert sha(source) == sha(target)
                    pins[str(source)] = pins[str(target)] = sha(source);target.chmod(0o444)
                verify(pins)
                module_name = 'julia_nbody_pilot_exact_source_editor'
                assert module_name not in sys.modules
                spec = importlib.util.spec_from_file_location(module_name,snapshot / ADAPTER.name)
                adapter = importlib.util.module_from_spec(spec);sys.modules[module_name] = adapter
                spec.loader.exec_module(adapter)
                assert Path(adapter.__file__).resolve() == snapshot / ADAPTER.name
                verify(pins)
                original = ORIGINAL.read_text()
                derived = adapter.derive(original,'nbody/upstream/nbody-julia-5.jl')
                assert len(derived['edits']) == 2 and derived['matched_derived_jit_required'] is True
                assert derived['runtime_aot_proven'] is derived['chart_admission'] is False
                assert len(derived['runtime_initializers']) == 1 and not derived['appended_declarations']
                assert derived['edits'][0]['replacement'] == 'global bodies::Vector{Body}\n'
                untouched = []
                point = 0
                for edit in derived['edits']:
                    assert original[edit['start']:edit['end']] == edit['original']
                    untouched.append(dict(start=point,end=edit['start'],text=original[point:edit['start']]))
                    point = edit['end']
                untouched.append(dict(start=point,end=len(original),text=original[point:]))
                spec = importlib.util.spec_from_file_location('julia_nbody_pilot_owned_commands',PREPARATION)
                preparation = importlib.util.module_from_spec(spec);spec.loader.exec_module(preparation)
                guard = preparation.load_guard();harness = guard.load_harness()
                environment = dict(bootstrap['runtime_environment'])
                julia = (Path(bootstrap['runtime_restoration']['runtime']) / 'bin/julia').resolve(strict=True)
                project = BUILD / 'ClbgNbodyPilot';(project / 'src').mkdir(parents=True)
                depot = BUILD / 'depot'
                shutil.copytree(bootstrap['depot'],depot,symlinks=True)
                environment['JULIA_DEPOT_PATH'] = str(depot)
                (project / 'Project.toml').write_text('name = "ClbgNbodyPilot"\nuuid = "91cc2cb8-cf4a-4c37-9b3a-bf932ade7016"\nversion = "0.1.0"\n')
                workload = project / 'src/workload.jl';workload.write_text(derived['source'])
                entry = project / 'src/entry.jl';shutil.copyfile(snapshot / 'entry_template.jl',entry)
                package = project / 'src/ClbgNbodyPilot.jl'
                package.write_text('module ClbgNbodyPilot\ninclude("workload.jl")\ninclude("entry.jl")\nend\n')
                driver = BUILD / 'direct_driver.c';shutil.copyfile(snapshot / 'direct_driver.c',driver)
                jit = BUILD / 'matched_jit_driver.jl'
                jit.write_text('include(joinpath(@__DIR__, "ClbgNbodyPilot", "src", "ClbgNbodyPilot.jl"))\nClbgNbodyPilot.initialize_runtime_state()\nClbgNbodyPilot.run_original_driver()\n')
                precompile = BUILD / 'precompile.jl'
                precompile.write_text('using ClbgNbodyPilot\n'
                    '@assert !isdefined(ClbgNbodyPilot, :bodies)\n'
                    'precompile(ClbgNbodyPilot.initialize_runtime_state, ())\n'
                    'precompile(ClbgNbodyPilot.init_sun, (Vector{ClbgNbodyPilot.Body},))\n'
                    'precompile(ClbgNbodyPilot.advance!, (Vector{ClbgNbodyPilot.Body},))\n'
                    'precompile(ClbgNbodyPilot.energy, (Vector{ClbgNbodyPilot.Body},))\n'
                    'precompile(ClbgNbodyPilot.nbody!, (Vector{ClbgNbodyPilot.Body}, Int))\n'
                    'precompile(ClbgNbodyPilot.run_original_driver, ())\n'
                    'precompile(ClbgNbodyPilot.clbg_entry, (Cint, Ptr{Ptr{Cchar}}))\n'
                    '@assert !isdefined(ClbgNbodyPilot, :bodies)\n')
                bitcode = BUILD / 'final-image-candidate.bc'
                build_script = BUILD / 'build.jl'
                build_script.write_text('using PackageCompiler\n'
                    'PackageCompiler.create_library(ARGS[1], ARGS[2]; lib_name="clbgnbodypilot", incremental=false, filter_stdlibs=false, '
                    'precompile_execution_file=ARGS[3], sysimage_build_args=Cmd(["-O3", "--output-bc=" * ARGS[4]]), force=false)\n')
                for path in (project / 'Project.toml',workload,entry,package,driver,jit,precompile,build_script):
                    pins[str(path)] = sha(path)
                save(OUTPUT / 'source-adaptation.json',dict(original_source=str(ORIGINAL),original_sha256=ORIGINAL_SHA,
                    adapter=derived,untouched_original_spans=untouched,derived_source=str(workload),derived_sha256=pins[str(workload)],
                    counted_compiled_app_files=[str(path) for path in (package,workload,entry,driver)],
                    separately_counted_matched_jit_loader=str(jit),original_jit_roles=selected,
                    source_transformation_only=True,chart_admission=False))
                bundle = BUILD / 'bundle'
                preparation.owned_command(guard,harness,[str(julia),'--startup-file=no','--history-file=no','--threads=8,0',
                    '--project=' + bootstrap['project'],str(build_script),str(project),str(bundle),str(precompile),str(bitcode)],
                    environment,OUTPUT / 'build',lock.fileno(),1800,'create-library')
                assert bitcode.is_file() and bitcode.stat().st_size > 0
                qualification_path = CLBG / 'c3-comparison/20261012/qualification/private-v3/report.json'
                qualification = json.loads(qualification_path.read_text())
                clang = preparation.CLANG
                assert sha(clang) == qualification['pins'][str(clang)]
                pins[str(clang)] = sha(clang);pins[str(qualification_path)] = sha(qualification_path)
                sdk = Path(qualification['sdk'])
                for path,value in qualification['pins'].items():
                    if Path(path).is_relative_to(sdk):
                        assert sha(path) == value;pins[path] = value
                executable = BUILD / 'nbody-aot'
                preparation.owned_command(guard,harness,[str(clang),'-O3','-arch','arm64','-isysroot',str(sdk),str(driver),
                    '-L' + str(bundle / 'lib'),'-lclbgnbodypilot','-Wl,-rpath,' + str(bundle / 'lib'),'-o',str(executable)],
                    environment,OUTPUT / 'link',lock.fileno(),120,'direct-c-launcher')
                pins.update(preparation.native_images(guard,harness,executable,environment,OUTPUT / 'images',lock.fileno(),
                    [bundle,Path(bootstrap['runtime_restoration']['runtime']),clang.parent.parent]))
                pins.update(preparation.pin_tree(BUILD))
                pins.update(preparation.pin_tree(OUTPUT));verify(pins)
                receipt.update(status='nbody-library-built-awaiting-state-native-coverage-and-runtime-qualification',
                    library_bundle=str(bundle),direct_executable=str(executable),final_image_bitcode_candidate=str(bitcode),
                    matched_jit_driver=str(jit),packagecompiler_version='2.4.3',julia_version=bootstrap['julia_version'],
                    pins_unchanged_at_end=True,bootstrap=dict(path=str(BOOTSTRAP),sha256=pins[str(BOOTSTRAP)]),
                    original_source_sha256=ORIGINAL_SHA,compact_optimized_share_original_source=True)
            except BaseException as error:
                receipt.update(error=str(error) or type(error).__name__,error_type=type(error).__name__)
                with (OUTPUT / 'failure.log').open('x') as stream:stream.write(traceback.format_exc())
                partial = {}
                try:
                    for directory in (OUTPUT,BUILD):
                        if directory.exists():
                            for path in sorted(directory.rglob('*')):
                                if path.is_file():
                                    value = sha(path)
                                    partial[str(path)] = value
                                    assert str(path) not in pins or pins[str(path)] == value
                                    pins[str(path)] = value
                    verify(pins)
                    receipt.update(terminal_evidence_preserved=True)
                except BaseException as integrity:
                    receipt.update(status='failed-terminal-integrity',preceding_build_failure=dict(
                        error=str(error) or type(error).__name__,error_type=type(error).__name__),
                        terminal_error=str(integrity) or type(integrity).__name__,terminal_error_type=type(integrity).__name__,
                        newly_captured_partial_terminal_pins=partial,terminal_evidence_preserved=False)
                    raise
                raise
            finally:
                receipt.update(pins=pins,completed_at=datetime.now(timezone.utc).isoformat())
                save(OUTPUT / 'report.json',receipt)
                terminal = {}
                try:
                    for path in sorted(OUTPUT.rglob('*')):
                        if path.is_file():terminal[str(path)] = sha(path)
                except BaseException as error:
                    save(OUTPUT / 'terminal-pins.json',dict(terminal_status='failed-terminal-integrity',preceding_status=receipt['status'],
                        pins=terminal,error=str(error) or type(error).__name__,error_type=type(error).__name__,
                        available_bytes_not_fully_pinned=True,exclusive_common_timing_mutex_held=True,
                        runtime_aot_proven=False,chart_admission=False,automatic_retry=False))
                    raise
                save(OUTPUT / 'terminal-pins.json',dict(terminal_status=receipt['status'],pins=terminal,
                    exclusive_common_timing_mutex_held=True,runtime_aot_proven=False,chart_admission=False,automatic_retry=False))
            print(json.dumps(dict(status=receipt['status'],report=str(OUTPUT / 'report.json'))),flush=True)


if __name__ == '__main__':
    main()

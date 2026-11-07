#!/usr/bin/env python3
"""Build diagnostic resource adapters without changing compiler or algorithms."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import tomllib


ROOT = Path(__file__).resolve().parent
RUST = Path('/Users/cim310/.rustup/toolchains/nightly-2026-03-03-aarch64-apple-darwin/bin')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def run(command, cwd, receipt, environment=None):
    started = time.monotonic()
    result = subprocess.run(list(map(str, command)), cwd=cwd, env=environment, capture_output=True, text=True)
    receipt.write_text(json.dumps(dict(command=list(map(str, command)), cwd=str(cwd), exit=result.returncode,
                                      seconds=time.monotonic()-started, stdout=result.stdout, stderr=result.stderr,
                                      rustflags=(environment or {}).get('RUSTFLAGS')), indent=2)+'\n')
    if result.returncode:
        raise SystemExit(result.stdout+result.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--variant', action='append', required=True)
    args = parser.parse_args()
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=True)
    base = json.loads(args.config.read_text())
    variants = {v['name']: v for v in base['implementations']}
    config = dict(machine=base['machine'], reference=base['reference'],
                  latency_qualification=False,
                  authoritative_latency_comparison=base.get('authoritative_latency_comparison', 'comparison-numeric-invariant-arguments-final-20261104-5000-m3.json'),
                  timing_boundaries=base.get('timing_boundaries'),
                  resource_reference=base.get('resource_reference', 'rust-concurrent-instrumented'),
                  whole_process_resource_reference=base.get('whole_process_resource_reference'),
                  measurement_source=str(ROOT/'measurement.rs'), measurement_source_sha256=sha(ROOT/'measurement.rs'),
                  implementations=[variants[base['reference']]])
    pins = []
    module = f'#[path = "{ROOT / "measurement.rs"}"] mod measurement;\n'
    system = '#[global_allocator]\nstatic METRICS_ALLOCATOR: measurement::Tracked<std::alloc::System> = measurement::Tracked(std::alloc::System);\n'
    for name in args.variant:
        original = variants[name]
        out = args.out/name
        out.mkdir(exist_ok=True)
        changes = []
        if original['kind'] == 'mech':
            build = json.loads(Path(original['receipt']).read_text())
            source = Path(build['source_path'])
            bundle = Path(build['bundle']['source.mec']['path']).parent
            target = out/'build'
            if not target.exists():
                target.symlink_to(bundle, target_is_directory=True)
            assert target.resolve() == bundle.resolve()
            text = source.read_text()
            changes.append(('', module+system))
            matched_post_json = original.get('calculation_boundary') == 'post_json_including_setup'
            replacements = [
                ('let compiled = Instant::now();', 'let restore_window = measurement::Window::begin();\n    let compiled = Instant::now();'),
                ('let compile_seconds = compiled.elapsed().as_secs_f64();', 'let compile_seconds = compiled.elapsed().as_secs_f64();\n    let restore_measurement = restore_window.finish();'),
                ('let prepare = Instant::now();', 'let json_window = measurement::Window::begin();\n    let prepare = Instant::now();'),
                ('let json_seconds = prepare.elapsed().as_secs_f64();', 'let json_seconds = prepare.elapsed().as_secs_f64();\n    let json_measurement = json_window.finish();'),
            ]
            if matched_post_json:
                # One counter window spans admission, intervening setup and
                # execution, matching the continuous authoritative timer.
                replacements.append(('let post_json_start = Instant::now();', 'let mut metrics_window = Some(measurement::Window::begin());\n    let post_json_start = Instant::now();'))
            else:
                replacements.extend([
                ('let admit = Instant::now();', 'let admission_window = measurement::Window::begin();\n    let admit = Instant::now();'),
                ('let admission_seconds = admit.elapsed().as_secs_f64();', 'let admission_seconds = admit.elapsed().as_secs_f64();\n    let admission_measurement = admission_window.finish();'),
                ('let start = Instant::now();', 'let metrics_window = measurement::Window::begin();\n        let start = Instant::now();'),
                ])
            admission_field = '' if matched_post_json else '"admission_measurement":&admission_measurement,'
            finish_window = ('metrics_window.take().unwrap().finish()'
                             if matched_post_json else 'metrics_window.finish()')
            replacements.extend([
                ('println!("{}", serde_json::json!', 'let resource_measurement = '+finish_window+';\n        println!("{}", serde_json::json!'),
                ('"sample":sample,', '"sample":sample,"measurement":resource_measurement,"restore_measurement":&restore_measurement,"json_measurement":&json_measurement,'+admission_field+'"transport_bytes":report.transport_bytes,"comparison_work":report.comparison_work,"visits":report.visits,"worker_credit_claims":report.worker_credit_claims,"worker_credit_batches":report.worker_credit_batches,"worker_credit_reclaims":report.worker_credit_reclaims,"retained_nodes":memory.retained_snapshot_nodes(),'),
            ])
            for old, new in replacements:
                assert text.count(old) == 1, (name, old)
                text = text.replace(old, new)
                changes.append((old, new))
            offset = 0
            for line in text.splitlines(keepends=True):
                if not line.startswith('//!'):
                    break
                offset += len(line)
            text = text[:offset]+module+system+text[offset:]
            destination = out/'runner.rs'
            destination.write_text(text)
            run(['python3', ROOT/'build-host.py', '--compiler-build', build['compiler_build'], '--source', destination,
                 '--out', out/'runner-metrics', '--bundle', bundle, '--linked'], ROOT, out/'host-build-run.json')
            run(['python3', ROOT/'audit-native-link.py', '--object', bundle/'object.o', '--executable', out/'runner-metrics',
                 '--output', out/'native-link-audit.json'], ROOT, out/'audit-run.json')
            variant = dict(name=name+'-instrumented', kind='mech', category=original['category']+'_instrumented',
                           executable=str(out/'runner-metrics'), source=original['source'], receipt=str(out/'runner-metrics.build.json'))
        else:
            if name == base['reference']:
                source = Path(original['executable']).parents[2]/'src/main.rs'
            else:
                source = Path(original['source'])
            # The same-algorithm reference pins its canonical source separately
            # from the temporary Cargo package used for the executable.
            package = Path(original['executable']).parents[2]
            shutil.copytree(package/'src', out/'src', dirs_exist_ok=True)
            for relative in ['Cargo.toml', 'Cargo.lock']:
                shutil.copy2(package/relative, out/relative)
            if (package/'.cargo').exists():
                shutil.copytree(package/'.cargo', out/'.cargo', dirs_exist_ok=True)
            text = source.read_text()
            assert text == (package/'src/main.rs').read_text()
            offset = 0
            for line in text.splitlines(keepends=True):
                if line.startswith('//!'):
                    offset += len(line)
                else:
                    break
            prefix = module
            old_allocator = 'static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;'
            if old_allocator in text:
                new_allocator = 'static GLOBAL: measurement::Tracked<mimalloc::MiMalloc> = measurement::Tracked(mimalloc::MiMalloc);'
                text = text.replace(old_allocator, new_allocator)
                changes.append((old_allocator, new_allocator))
            else:
                assert '#[global_allocator]' not in text
                prefix += system
            control_main = 'fn main()->io::Result<()> {' in text
            if control_main:
                begin = ('let start=Instant::now();\n    let mut offsets'
                         if original.get('calculation_boundary') == 'post_json_including_setup'
                         else 'let start=Instant::now();let (indices,phases)')
                end = 'let elapsed=start.elapsed();'
            else:
                begin = ('let start = hint::black_box(Instant::now());'
                         if 'let start = hint::black_box(Instant::now());' in text
                         else 'let start = Instant::now();')
                end = 'let end = hint::black_box(Instant::now());'
            new_begin = 'let preparation_measurement = preparation_window.finish();\n    let metrics_window = measurement::Window::begin();\n    '+begin
            new_end = end+'\n    let resource_measurement = metrics_window.finish();\n    println!("PROCESS_METRICS {}", serde_json::json!({"processing":resource_measurement,"preparation":preparation_measurement}));'
            opening = 'fn main()->io::Result<()> {' if control_main else 'fn main() -> io::Result<()> {'
            for old, new in [(opening, opening+'\n    let preparation_window = measurement::Window::begin();'), (begin, new_begin), (end, new_end)]:
                assert text.count(old) == 1, (name, old)
                text = text.replace(old, new)
                changes.append((old, new))
            text = text[:offset]+prefix+text[offset:]
            changes.append(('', prefix))
            destination = out/'src/main.rs'
            destination.write_text(text)
            env = {**os.environ, 'RUSTC':str(RUST/'rustc'), 'CARGO_TARGET_DIR':str(args.out/'target')}
            if 'rustflags' in original:
                env['RUSTFLAGS'] = original['rustflags']
            run([RUST/'cargo', 'build', '--release', '--locked', '--offline'], out, out/'build.json', env)
            binary = tomllib.loads((out/'Cargo.toml').read_text())['package']['name']
            shutil.copy2(args.out/'target/release'/binary, out/'runner-metrics')
            variant = dict(name=('rust-standard' if name == base['reference'] else name)+'-instrumented', kind='upstream',
                           category=original['category']+'_instrumented', executable=str(out/'runner-metrics'),
                           source=str(destination), receipt=str(out/'build.json'), output=original['output'])
            if 'rustflags' in original:
                variant['rustflags'] = original['rustflags']
        restored = text
        # The injected module prefix must be removed before reversing inserted
        # allocator/timer code. No algorithm statement is altered.
        injected = module+system if original['kind']=='mech' else prefix
        assert restored.count(injected) == 1
        restored = restored.replace(injected, '', 1)
        for old, new in reversed(changes):
            if old:
                assert restored.count(new) == 1, (name, new)
                restored = restored.replace(new, old, 1)
        assert restored == source.read_text(), name
        if 'arguments' in original:
            variant['arguments'] = list(original['arguments'])
        if 'processing_boundary' in original:
            variant['processing_boundary'] = original['processing_boundary']
        if 'calculation_boundary' in original:
            variant['calculation_boundary'] = original['calculation_boundary']
        variant['measurement_boundary'] = ('post_json_including_setup'
                                           if original.get('calculation_boundary') == 'post_json_including_setup'
                                           else 'processing')
        pins.append(dict(variant=name, original_source=str(source), original_source_sha256=sha(source),
                         instrumented_source=str(destination), instrumented_source_sha256=sha(destination),
                         reverse_patch_exact=True, original_executable_sha256=sha(original['executable']),
                         diagnostic_executable_sha256=sha(variant['executable'])))
        config['implementations'].append(variant)
        print(json.dumps(dict(variant=name, status='built', reverse_patch_exact=True)), flush=True)
    (args.out/'source-pins.json').write_text(json.dumps(pins, indent=2)+'\n')
    (args.out/'metrics-config.json').write_text(json.dumps(config, indent=2)+'\n')


if __name__ == '__main__':
    main()

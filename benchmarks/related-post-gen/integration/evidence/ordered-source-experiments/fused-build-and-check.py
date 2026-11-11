from pathlib import Path
import copy, json, subprocess, time

U = Path(__file__).parent
R = Path('/Users/cim310/mech-lang/mech/benchmarks/related-post-gen')
O = Path('/private/tmp/mech-related-deflate-exhaustion-proof-integration-opt-20261106-v1')
P = Path('/private/tmp/mech-related-ordered-subsets-source-20261106-v1')

def run(command, receipt):
    command = list(map(str, command))
    start = time.monotonic()
    result = subprocess.run(command, capture_output=True, text=True)
    receipt.write_text(json.dumps(dict(command=command, exit=result.returncode,
        seconds=time.monotonic()-start, stdout=result.stdout, stderr=result.stderr), indent=2)+'\n')
    assert result.returncode == 0, result.stdout+result.stderr
    print(receipt.name, 'passed', flush=True)

run([O/'compiler', U/'source.mec', U/'build'], U/'compile-run.json')
run(['python3', R/'build-host.py', '--compiler-build', O/'build.json',
    '--source', U/'runner.rs', '--out', U/'runner-aot', '--bundle', U/'build', '--linked'], U/'host-build-run.json')
run(['python3', R/'audit-native-link.py', '--object', U/'build/object.o',
    '--executable', U/'runner-aot', '--output', U/'native-link-audit.json'], U/'audit-run.json')
run(['python3', R/'experiments/check_subset3.py', '--runner', U/'runner-aot',
    '--rust', '/private/tmp/mech-related-post-rust-local-20261102/rust/target/release/rust',
    '--source', U/'source.mec', '--out', U/'edges'], U/'edge-run.json')

config = json.loads((P/'comparison-config.json').read_text())
variant = copy.deepcopy(next(v for v in config['implementations'] if v['name']=='mech-ordered-4'))
variant.update(name='mech-fused-4', category='IR234_fused_ordering_uniqueness_source',
    executable=str(U/'runner-aot'), source=str(U/'source.mec'), receipt=str(U/'runner-aot.build.json'))
config['implementations'].append(variant)
config['authoritative_latency_comparison']='comparison-ordered-subsets-fused-source-randomized-20261106-5000-m3.json'
(U/'comparison-config.json').write_text(json.dumps(config, indent=2)+'\n')
run(['python3', R/'build-metrics.py', '--config', U/'comparison-config.json', '--out', U/'metrics',
     '--variant', 'mech-fused-4'], U/'metrics-build-run.json')
resource = json.loads((P/'resource-config.json').read_text())
instrumented = json.loads((U/'metrics/metrics-config.json').read_text())
resource['implementations'].extend(v for v in instrumented['implementations'] if v['name']!='rust')
resource['authoritative_latency_comparison']=config['authoritative_latency_comparison']
(U/'resource-config.json').write_text(json.dumps(resource, indent=2)+'\n')
metadata=json.loads((U/'build/metadata.json').read_text())
print(json.dumps({key:metadata.get(key) for key in ['typed_ir_version','code_bytes','stack_bytes']}), flush=True)

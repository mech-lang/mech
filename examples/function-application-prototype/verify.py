#!/usr/bin/env python3
"""Small AOT conformance checks; not a performance benchmark."""
import argparse, json, os, pathlib, subprocess, tempfile
HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[1]
TARGET_DIR = pathlib.Path(os.environ.get('MECH_TARGET_DIR', os.environ.get('CARGO_TARGET_DIR', ROOT/'target'))).expanduser()
if not TARGET_DIR.is_absolute(): TARGET_DIR = ROOT/TARGET_DIR
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--cli', type=pathlib.Path, default=os.environ.get('MECH_CLI', TARGET_DIR/'release'/('mech.exe' if os.name == 'nt' else 'mech')))
parser.add_argument('--report', type=pathlib.Path, default=HERE/'verification-current.json')
options = parser.parse_args()
CLI = options.cli.expanduser()
env = os.environ | {
    'CARGO_TARGET_DIR': str(TARGET_DIR),
    'RUSTFLAGS': '-D warnings',
    'MECH_NUMERIC_UNROLL': '4', 'MECH_NUMERIC_ROWS': '4',
}
def build(source, binary, succeeds=True):
    p = subprocess.run([str(CLI), 'build', str(source), '--backend', 'cranelift', '--no-config', '--offline', '-o', str(binary)], cwd=ROOT, env=env, capture_output=True)
    if succeeds and p.returncode:
        raise RuntimeError(p.stderr.decode()+p.stdout.decode())
    if not succeeds and not p.returncode:
        raise AssertionError('invalid source accepted')
    return p

def run(binary, *args):
    return subprocess.check_output([str(binary), *map(str,args)], env=env)

with tempfile.TemporaryDirectory(prefix='mech-application-proof-') as t:
    t = pathlib.Path(t)
    build(HERE/'spectralnorm.mec', t/'prototype')
    build(HERE/'spectralnorm-baseline.mec', t/'baseline')
    sizes = [1,2,3,5,7,8,9,15,16,17,31,100,5500]
    for n in sizes:
        actual, expected = run(t/'prototype',n), run(t/'baseline',n)
        assert actual == expected, (n,actual,expected)
    build(HERE/'generic.mec', t/'generic')
    expected = b'1.189207115\n15.375000000\n90.000000000,135.000000000,180.000000000\n2.500\n'
    assert run(t/'generic') == expected
    invalid = {
        'type': 'f(x<f64>) => <f64> := x |> math/sqrt\ny := 2 |> f\n',
        'forward-capture': 'f(x<u64>) => <u64> := x + offset\ny := 2 |> f\noffset := 3\n',
        'effect': 'f(x<f64>) => <f64> := number/to-f64(io/argument-u64(1))\ny := sequence/iterate(2.0, 1, f)\n',
    }
    for name, source in invalid.items():
        p=t/(name+'.mec'); p.write_text(source)
        build(p,t/name,False)
report={'spectralnorm_source_loc':7,'baseline_source_loc':13,'identical_output_sizes':sizes,'generic_scalar_vector_rectangular_and_kind_change':True,'invalid_types_forward_captures_and_effects_rejected':True,'all_passed':True}
options.report.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))

"""Build this checkout's compiler and regenerate the native encoder without LLVM tools."""
from pathlib import Path
from hashlib import sha256
import argparse, json, os, platform, struct, subprocess, time

from rebuild_checks import check_faults, check_successes


M1_SOURCE_RECIPES = {
    'm1-checked-regions': 'indexed-checked-regions',
    'm1-compact-match-equality': 'compact-match-equality',
    'm1-shared-release-setup': 'shared-release-setup',
    'm1-shared-heap-setup': 'shared-heap-call-screen',
    'm1-shared-heap-layout': 'shared-heap-layout-recovery',
    'm1-literal-frame-cleanup': 'shared-literal-frame-cleanup',
    'm1-preserved-root-registers': 'preserved-root-registers',
    'm1-checked-codebook-phase': 'checked-codebook-phase',
    'm1-sparse-guard-rounds': 'sparse-guard-rounds',
    'm1-register-spill-pairs': 'current-register-spill-pairs',
    'm1-matcher-guard-facts': 'current-matcher-guard-facts',
}


def source_recipe(directory, name):
    manifest = directory / 'evidence/encoder-size-gap' / M1_SOURCE_RECIPES[name] / 'candidate.json'
    pinned = json.loads(manifest.read_text())
    profiles = []
    for profile in pinned['profiles']:
        path = manifest.parent / profile['path']
        if sha256(path.read_bytes()).hexdigest() != profile['sha256']:
            raise SystemExit(f'M1 layout profile changed: {path}')
        profiles.append(f'{profile["symbol"]}@{path}')
    return manifest, pinned, ';'.join(profiles)


def load_recorded_runtime(checkout, output, pin):
    """Restore the three original source files from the benchmark's first commit."""
    hashes = pin['source_sha256']
    if set(hashes) != {'mod.rs', 'terminal_appends.rs', 'zero_pages.rs'}:
        raise SystemExit("The recorded runtime recipe must pin all three source files.")
    payload = {}
    for name, expected in hashes.items():
        revision = f"{pin['commit']}:{pin['directory']}/{name}"
        try:
            data = subprocess.check_output(
                ['git', '--no-replace-objects', '-C', checkout, 'cat-file', 'blob', revision],
                stderr=subprocess.PIPE,
                env=os.environ | {"GIT_NO_LAZY_FETCH": "1"},
            )
        except (FileNotFoundError, subprocess.CalledProcessError) as exc:
            raise SystemExit(
                f"Linking the recorded worker needs a Git checkout with PR #993 history. "
                f"Could not read {revision}. Fetch the pinned commit before retrying."
            ) from exc
        if sha256(data).hexdigest() != expected:
            raise SystemExit(f"The recorded runtime source hash differs for {revision}.")
        payload[name] = data
    output.mkdir()
    for name, data in payload.items():
        (output / name).write_bytes(data)
    return output / 'mod.rs'


here=Path(__file__).resolve().parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('output',type=Path)
parser.add_argument(
    '--variant',
    choices=[
        'baseline', 'cold-tails', 'owner-ranges', 'fresh-owner-counts',
        'fresh-owner-counts-cleanup', 'fresh-owner-counts-cold-tails',
        'fresh-owner-counts-cold-tails-paired-spills',
        'fresh-owner-counts-cold-tails-zero-branches',
        'frame-copies',
        *M1_SOURCE_RECIPES,
    ],
    default='baseline',
)
parser.add_argument('--mimalloc-object',type=Path,help='optional fixed comparator allocator; also link and check the worker')
parser.add_argument('--target-dir',type=Path)
parser.add_argument(
    '--direct-source', action='store_true',
    help='run the benchmark frontend and diagnostic native emitter in one compiler process',
)
parser.add_argument(
    '--compiler-opt-level', type=int, choices=(1, 3), default=1,
    help='Rust optimization level for the compiler binaries (default: 1)',
)
parser.add_argument(
    '--reuse-equivalent-vector-ir', action='store_true',
    help='reuse machine code for vector alternatives with identical prepared IR',
)
parser.add_argument(
    '--preselect-root-inlining', action='store_true',
    help='choose single-call checked-root helpers before native code generation',
)
parser.add_argument(
    '--skip-refused-general-ssa', action='store_true',
    help='skip general SSA for eligible checked private vector phases already omitted from the object',
)
args=parser.parse_args()
named = args.variant in M1_SOURCE_RECIPES
base_variant = 'frame-copies' if named else args.variant
if named:
    if (platform.system(), platform.machine()) != ('Darwin', 'arm64'):
        parser.error('The archived M1 encoder identities require Apple AArch64.')
    manifest, pinned, pinned_layout = source_recipe(here, args.variant)
    if sha256((here / 'program.mec').read_bytes()).hexdigest() != pinned['source_sha256']:
        raise SystemExit('The source differs from the named recipe.')
    args.direct_source = True
    args.compiler_opt_level = 3
    args.preselect_root_inlining = True
    args.skip_refused_general_ssa = True
out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
target=(args.target_dir or out/'compiler').resolve()
recipe=json.loads((here/'recipe.json').read_text())
if args.mimalloc_object:
    allocator = args.mimalloc_object.resolve(strict=True)
    recorded_runtime = load_recorded_runtime(here, out / 'recorded-runtime', recipe['recorded_runtime'])
def sha(p):return sha256(Path(p).read_bytes()).hexdigest()
def run(command,log,env):
    command=list(map(str,command));start=time.perf_counter()
    with log.open('w') as stream:subprocess.run(command,check=True,env=env,stdout=stream,stderr=subprocess.STDOUT)
    return {'command':command,'seconds':time.perf_counter()-start}
env={k:v for k,v in os.environ.items() if not k.startswith('MECH_')}
profile = {
    'CARGO_BUILD_JOBS': '1',
    'CARGO_PROFILE_RELEASE_OPT_LEVEL': str(args.compiler_opt_level),
    'CARGO_PROFILE_RELEASE_DEBUG': 'false',
    'CARGO_PROFILE_RELEASE_LTO': 'false',
    'CARGO_PROFILE_RELEASE_CODEGEN_UNITS': '16',
}
env.update(profile)
# The allocator's C build takes OPT_LEVEL from Cargo's package profile.
allocator_profile = 'profile.release.package.libmimalloc-sys.opt-level=1'
cargo = [
    'cargo', '+nightly-2026-03-03', 'build', '--locked', '--release',
    '--config', allocator_profile, '--manifest-path', here.parent / 'Cargo.toml',
    '--target-dir', target, '--message-format=json',
]
if args.direct_source:
    cargo.extend(('--bin', 'compile_profile'))
else:
    cargo.extend(('--bin', 'native_compact_frontend', '--bin', 'emit_compact_size'))
build=run(cargo,out/'compiler-build.jsonl',env)
build['profile'] = profile
build['package_profile'] = allocator_profile
libraries={}
for line in (out/'compiler-build.jsonl').read_text().splitlines():
    try:row=json.loads(line)
    except ValueError:continue
    if row.get('reason')=='compiler-artifact':
        for path in row['filenames']:
            if path.endswith('.rlib'):libraries[row['target']['name']]=path
assert sha(here/'program.mec')==recipe['source_sha256']
flags=dict(recipe['flags'])
variant_key=base_variant.replace('-','_')
paired_spills=base_variant=='fresh-owner-counts-cold-tails-paired-spills'
frame_copies = base_variant == 'frame-copies'
zero_branches = base_variant == 'fresh-owner-counts-cold-tails-zero-branches' or frame_copies
fresh_owner_cold_tails=base_variant=='fresh-owner-counts-cold-tails' or paired_spills or zero_branches
profiles=recipe.get(variant_key+'_profiles',recipe['profiles'])
flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT']=';'.join(p['symbol']+'@'+str(here/p['path']) for p in profiles)
if args.preselect_root_inlining:
    flags['MECH_COMPACT_INLINE_SINGLE_CALLS'] = 'preselected'
if args.skip_refused_general_ssa:
    flags['MECH_COMPACT_SKIP_REFUSED_GENERAL_SSA'] = '1'
if args.reuse_equivalent_vector_ir:
    flags['MECH_COMPACT_REUSE_EQUIVALENT_VECTOR_IR'] = '1'
if base_variant=='cold-tails' or fresh_owner_cold_tails:
    flags['MECH_COMPACT_SHARE_COLD_TAILS'] = '1'
if base_variant=='owner-ranges':flags['MECH_COMPACT_PROVE_BACKING_OWNERS']='ranges'
if base_variant in ('fresh-owner-counts', 'fresh-owner-counts-cleanup') or fresh_owner_cold_tails:
    flags['MECH_COMPACT_FOLD_FRESH_OWNER_COUNTS'] = '1'
if base_variant=='fresh-owner-counts-cleanup':flags.update(MECH_COMPACT_PROVE_BACKING_OWNERS='all',MECH_COMPACT_FINISH_CFG_CLEANUP='1')
if paired_spills:
    flags['MECH_COMPACT_PAIR_REGISTER_SPILLS'] = '1'
if zero_branches:
    flags['MECH_COMPACT_BRANCH_ON_ZERO'] = '1'
    tail_profile = 'frame_copies_tail_profile' if frame_copies else 'zero_branches_tail_profile'
    flags['MECH_COMPACT_ROOT_TAIL_PROFILE'] = str(here / recipe[tail_profile])
if frame_copies:
    flags['MECH_COMPACT_SHARE_FRAME_COPIES'] = '1'
    flags['MECH_COMPACT_DIRECT_ENTRY_LITERALS'] = '1'
    if args.direct_source:
        flags['MECH_COMPILE_OVERLAP_SOURCE_CATALOG'] = '1'
if named:
    for name in pinned['remove_flags']:
        del flags[name]
    flags.update(pinned['override_flags'])
    flags['MECH_COMPACT_PHASE_BLOCK_LAYOUT'] = pinned_layout
assert not any('LLVM' in key for key in flags)
source = here / 'program.mec'
frontend = out / 'frontend'
native = out / 'native'
bytecode = frontend / 'program.mecb'
native_env = env | flags | {'PATH': '/usr/bin:/bin'}


def verify_bytecode():
    expected = pinned['artifact_sha256']['program.mecb'] if named else recipe['bytecode_sha256']
    assert sha(bytecode) == expected, (
        'changed canonical bytecode; inspect before comparing native output'
    )


def build_runtime(output, runtime_entry, runtime_pin, environment):
    runtime_source = output / 'runtime.rs'
    runtime_source.write_text(
        'pub mod native_artifact { #[path = ' + json.dumps(str(runtime_entry)) +
        '] pub mod compact_runtime; }\n'
    )
    runtime = output / 'libmech_gpu_runtime.rlib'
    runtime_build = run(
        ['rustc', '+nightly-2026-03-03', '--edition=2024', '--crate-name', 'mech_gpu',
         '--crate-type=rlib', '-C', 'opt-level=3', runtime_source, '-o', runtime],
        output / 'runtime-build.log', environment,
    )
    return runtime, runtime_pin | {'build': runtime_build, 'rlib_sha256': sha(runtime)}


if named and args.mimalloc_object:
    runtime, runtime_build = build_runtime(out, recorded_runtime, recipe['recorded_runtime'], env)

phases = {}
start = time.perf_counter()
if args.direct_source:
    compiler = target / 'release/compile_profile'
    phases['source_to_native'] = run(
        [compiler, source, frontend, 'compact'], out / 'direct-source.log', native_env,
    )
    verify_bytecode()
    native.mkdir()
    for artifact in ('program.o', 'metadata.json'):
        (frontend / artifact).rename(native / artifact)
else:
    compiler = target / 'release/native_compact_frontend'
    phases['frontend'] = run(
        [compiler, source, frontend, 'frontend'], out / 'frontend.log', env,
    )
    verify_bytecode()
    emitter = target / 'release/emit_compact_size'
    phases['native'] = run([emitter, bytecode, native], out / 'native.log', native_env)
object_file=out/'native/program.o';data=object_file.read_bytes();assert struct.unpack_from('<I',data)[0]==0xfeedfacf
text_bytes=None;offset=32
for _ in range(struct.unpack_from('<I',data,16)[0]):
    command,size=struct.unpack_from('<II',data,offset)
    if command==0x19:
        for section in range(struct.unpack_from('<I',data,offset+64)[0]):
            at=offset+72+section*80
            if data[at:at+16].rstrip(b'\0')==b'__text':text_bytes=struct.unpack_from('<Q',data,at+40)[0]
    offset+=size
expected=recipe[variant_key+'_object_sha256'] if not named else pinned['artifact_sha256']['program.o']
expected_metadata=recipe[variant_key+'_metadata_sha256'] if not named else pinned['artifact_sha256']['metadata.json']
actual_metadata=sha(out/'native/metadata.json')
record={'variant':args.variant,'compiler_build':build,'phases':phases,'source_to_object_seconds':time.perf_counter()-start,'object_sha256':sha(object_file),'expected_object_sha256':expected,'metadata_sha256':actual_metadata,'expected_metadata_sha256':expected_metadata,'object_text_bytes':text_bytes,'object_text_x_roc':text_bytes/57696,'object_text_x_libdeflate':text_bytes/37080,'full_integrated_source_identity':sha(object_file)==expected and actual_metadata==expected_metadata,'runtime_wins':'not measured by this command'}
record['native_flags'] = flags
record['compiler_pipeline'] = 'direct-source' if args.direct_source else 'split'
if named:
    record['source_recipe'] = {
        'manifest': str(manifest.relative_to(here)),
        'manifest_sha256': sha(manifest),
        'profiles': pinned['profiles'],
        'expected_worker_sha256': pinned['worker_sha256'],
    }
    record['source_commit'] = subprocess.check_output(
        ['git', '-C', here, 'rev-parse', 'HEAD'], text=True,
    ).strip()
    record['source_sha256'] = sha(source)
    record['bytecode_sha256'] = sha(bytecode)
    record['compiler_sha256'] = sha(compiler)
    record['compiler_toolchain'] = subprocess.check_output(
        ['rustc', '+nightly-2026-03-03', '--version', '--verbose'], text=True,
    ).strip()
    record['full_integrated_source_identity'] &= text_bytes == pinned['encoder_text_bytes']
(out/'rebuild.json').write_text(json.dumps(record,indent=2)+'\n')
assert record['full_integrated_source_identity'],'integrated compiler changed the object; report this result, do not reuse the historical win count'
if args.mimalloc_object:
    if not named:
        runtime, runtime_build = build_runtime(out, recorded_runtime, recipe['recorded_runtime'], env)
    record['recorded_runtime'] = runtime_build
    command=['rustc','+nightly-2026-03-03','--edition=2024','-C','opt-level=3','-D','warnings','-L','dependency='+str(target/'release/deps'),here/'worker.rs','-o',out/'mech-worker','--extern','mech_gpu='+str(runtime),'-C','link-arg='+str(object_file),'-C','link-arg='+str(allocator)]
    for name in ['mech_engine','serde','serde_json','miniz_oxide']:command+=['--extern',name+'='+libraries[name]]
    record['worker_link']=run(command,out/'worker-link.log',env)
    if named:
        record['source_through_final_link_seconds'] = time.perf_counter() - start
    fixtures=json.loads((here/'fixtures.json').read_text())
    for case in fixtures:
        for key in ['input','prefix','expected']:case[key]=str(here/case[key])
    fixture_file=out/'fixtures.json';fixture_file.write_text(json.dumps(fixtures))
    metadata = out / 'native/metadata.json'
    calls = check_successes(out / 'mech-worker', metadata, fixture_file, fixtures, out / 'worker-smoke.log', env)
    (out/'correctness.json').write_text(json.dumps(calls,indent=2)+'\n')
    record['verified_exact_calls']=len(calls);record['worker_sha256']=sha(out/'mech-worker')
    if named:
        record['worker_matches_archive'] = record['worker_sha256'] == pinned['worker_sha256']
        record['allocator_object_sha256'] = sha(allocator)
        record['checked_failure_probe'] = check_faults(
            here, out, command, metadata, fixture_file, fixtures, env, run,
        )
    (out/'rebuild.json').write_text(json.dumps(record,indent=2)+'\n')
print(json.dumps(record,indent=2))

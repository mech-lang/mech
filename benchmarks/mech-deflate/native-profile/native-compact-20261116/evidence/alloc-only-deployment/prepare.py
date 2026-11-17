"""Copy the pinned sources, apply the narrow alloc-only diff, and record argv.

This preparation only reads the published inputs and existing link receipt.
It invokes neither the compiler nor a deployment executable.
"""
from argparse import ArgumentParser
from difflib import unified_diff
import gzip
from hashlib import file_digest, sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
DEPLOY = HERE.parent / 'minimal-deployment'
REQUIRED = {
    '_mech_compact_allocate_backing',
    '_mech_compact_finish_byte_appends',
    '_mech_compact_free',
    '_mech_compact_release_four',
    '_mech_compact_release_nullable',
    '_mech_compact_release_owner',
    '_mech_compact_release_three',
    '_mech_compact_release_two',
    '_mech_compact_writable',
    '_mech_compact_writable_scalar_backing',
    '_memmove',
}
TARGET = 'aarch64-apple-darwin'
TOOLCHAIN = '+nightly-2026-03-03'


def sha(path):
    with path.open('rb') as file:
        return file_digest(file, 'sha256').hexdigest()


def load(path):
    return json.loads(path.read_bytes())


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def replace(text, old, new):
    assert text.count(old) == 1, old
    return text.replace(old, new)


def runtime_main(text):
    text = replace(text, """use std::{
    alloc::{Layout, alloc, alloc_zeroed, dealloc},
    marker::PhantomData,
    mem::MaybeUninit,
    ptr::{self, NonNull},
};
""", """use alloc::{
    alloc::{Layout, alloc, alloc_zeroed, dealloc},
    boxed::Box,
    vec::Vec,
};
use core::{marker::PhantomData, mem::MaybeUninit, ptr::{self, NonNull}};
""")
    for module in ('slice', 'mem', 'iter'):
        text = text.replace(f'std::{module}::', f'core::{module}::')
    for marker in (
        'struct Completed {', 'unsafe impl Send for Completed {}',
        "impl Heap<'_> {\n    fn adopt",
        '#[unsafe(no_mangle)]\npub unsafe extern "C" fn mech_compact_parallel(',
    ):
        text = replace(text, marker, '#[cfg(feature = "native-threads")]\n'+marker)
    return replace(text, '#[cfg(test)]\nmod parallel_tests {',
                   '#[cfg(all(test, feature = "native-threads"))]\nmod parallel_tests {')


def zero_pages(text):
    text = replace(text, 'use std::{alloc::Layout, ptr::NonNull};', 'use core::{alloc::Layout, ptr::NonNull};')
    text = replace(text, '    use std::{ffi::c_void, sync::OnceLock};',
                   '''    use core::{
        ffi::c_void,
        sync::atomic::{AtomicUsize, Ordering},
    };''')
    text = replace(text, '    thread_local! {', '    std::thread_local! {')
    text = replace(text, 'mmap(std::ptr::null_mut(),','mmap(core::ptr::null_mut(),')
    return replace(text, """        static PAGE_BYTES: OnceLock<usize> = OnceLock::new();
        let page_bytes = *PAGE_BYTES.get_or_init(|| {
            // SAFETY: the system query has no pointer arguments.
            usize::try_from(unsafe { getpagesize() }).unwrap()
        });""", """        static PAGE_BYTES: AtomicUsize = AtomicUsize::new(0);
        let page_bytes = match PAGE_BYTES.load(Ordering::Relaxed) {
            0 => {
                // SAFETY: the system query has no pointer arguments. All
                // callers obtain the same nonzero process page size.
                let value = usize::try_from(unsafe { getpagesize() }).unwrap();
                PAGE_BYTES.store(value, Ordering::Relaxed);
                value
            }
            value => value,
        };""")


def bridge(text):
    text = replace(text, "//! Narrow C bridge for the retained diagnostic raw DEFLATE artifact.\n",
                   "//! Narrow C bridge for the retained diagnostic raw DEFLATE artifact.\n#![no_std]\n\nextern crate alloc;\n")
    text = replace(text, """use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage, View};
use std::ffi::{c_int, c_void};
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};
""", """use alloc::vec::Vec;
use core::ffi::{c_int, c_void};
use core::mem::{self, ManuallyDrop};
use core::ptr::{self, NonNull};
use mech_gpu::native_artifact::compact_runtime::{
    Counters, Heap, NativeStatus, Owner, OwnerStorage, View,
};
""")
    text = replace(text, 'unsafe extern "C" {\n    fn mech_compact_closed_v2(',
                   '''// The retained object reads the reference count and reuse request counter
// directly. Pin the layout consumed by the compiled artifact.
const _: () = {
    assert!(mem::size_of::<DeployBuffer>() == 24);
    assert!(mem::size_of::<NativeStatus>() == 16);
    assert!(mem::offset_of!(NativeStatus, origin) == 8);
    assert!(mem::size_of::<Owner>() == 48);
    assert!(mem::offset_of!(Owner, references) == 0);
    assert!(
        mem::offset_of!(Heap<'static>, counters)
            + mem::offset_of!(Counters, payload_requests) == 0x60
    );
};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    // The original deployment built the bridge with panic=abort.
    unsafe { abort() }
}

unsafe extern "C" {
    fn abort() -> !;
    fn mech_compact_closed_v2(''')
    return replace(text, 'std::mem::replace(output,', 'mem::replace(output,')


def runtime_root(text):
    assert text.startswith('pub mod native_artifact')
    return """#![no_std]

extern crate alloc;
#[cfg(any(test, feature = "native-threads"))]
extern crate std;

pub mod native_artifact {
    pub mod compact_runtime;
}
"""


def base_manifest(paths, deployment, manifest_path, baseline):
    manifest = load(manifest_path)
    published_pins = load(deployment / 'input-pins.json')
    for name in ('mech_encoder', 'mech_metadata', 'mech_runtime', 'mimalloc', 'pinned_allocator'):
        row = manifest[name]
        path = (manifest_path.parent / row['path']).resolve()
        assert sha(path) == row['sha256'] == published_pins[name]['sha256'], name
        paths[name] = path
    before = load(baseline / 'build.json')
    for name in ('bridge.rs', 'native_abi.rs', 'main.c', 'deploy.h', 'build.py'):
        path = deployment / 'source' / name
        assert sha(path) == before['sources'][name], name
    options = subprocess.check_output(['/usr/bin/nm', '-u', paths['mech_encoder']], text=True)
    assert set(options.split()) == REQUIRED
    link_map = (baseline / 'mech.map').read_bytes()
    live, separator, _ = link_map.partition(b'# Dead Stripped Symbols:')
    assert separator and b'mech_compact_parallel' not in live
    return before


def cargo_manifests(tree):
    (tree / 'Cargo.toml').write_text("""[package]
name = "mech_deploy"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
crate-type = ["staticlib"]
path = "source/bridge.rs"

[dependencies]
mech_gpu = { path = "runtime" }

[profile.release]
opt-level = 3
panic = "abort"
debug = false
codegen-units = 1
""")
    (tree / 'runtime/Cargo.toml').write_text("""[package]
name = "mech_gpu"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "lib.rs"

[features]
native-threads = []
""")


def prepare(tree, deployment, manifest, baseline):
    paths = {}
    before = base_manifest(paths, deployment, manifest, baseline)
    pinned_sources = load(HERE / 'measurements/plan.json')['sources']
    runtime_sources = deployment / 'fixed-runtime-source'
    tree.mkdir()
    edited = {}
    file_sources = {
        'source/bridge.rs': (deployment / 'source/bridge.rs', bridge),
        'source/native_abi.rs': (deployment / 'source/native_abi.rs', None),
        'source/main.c': (deployment / 'source/main.c', None),
        'source/deploy.h': (deployment / 'source/deploy.h', None),
        'source/pinned_allocator.rs': (paths['pinned_allocator'], lambda t: replace(t, 'use std::alloc::', 'use core::alloc::')),
        'runtime/lib.rs': (runtime_sources / 'runtime-lib-as-run.rs.gz', runtime_root),
    }
    rdir = 'runtime/native_artifact/compact_runtime/'
    file_sources.update({
        rdir+'mod.rs': (runtime_sources / 'compact_runtime/mod.rs.gz', runtime_main),
        rdir+'zero_pages.rs': (runtime_sources / 'compact_runtime/zero_pages.rs.gz', zero_pages),
        rdir+'terminal_appends.rs': (runtime_sources / 'compact_runtime/terminal_appends.rs.gz', lambda t: t.replace('std::', 'core::')),
    })
    diff = []
    for name, (original, change) in file_sources.items():
        target = tree / name
        target.parent.mkdir(parents=True, exist_ok=True)
        data = original.read_bytes()
        if original.suffix == '.gz':
            data = gzip.decompress(data)
        before_digest = sha256(data).hexdigest()
        assert before_digest == pinned_sources[name]['before_sha256'], original
        old = data.decode()
        new = old if change is None else change(old)
        target.write_text(new)
        if change is not None:
            diff.extend(unified_diff(old.splitlines(keepends=True), new.splitlines(keepends=True),
                                     fromfile=f'a/{name}', tofile=f'b/{name}'))
        after_digest = sha(target)
        assert after_digest == pinned_sources[name]['experiment_sha256'], target
        edited[name] = {'original': str(original), 'before_sha256': before_digest, 'experiment_sha256': after_digest}
    patch = ''.join(diff)
    assert patch == gzip.decompress((HERE / 'alloc-only.patch.gz').read_bytes()).decode()
    (tree / 'alloc-only.patch').write_text(patch)
    cargo_manifests(tree)
    paths['main'] = baseline / 'main.o'
    runtime = tree / 'libmech_gpu_alloc_only.rlib'
    archive = tree / 'libmech_deploy.a'
    base_compile = before['commands']['compile-mech']
    original_bridge_source, = (arg for arg in base_compile if arg.endswith('/bridge.rs'))
    original_archive = before['commands']['compile-mech'][-1]
    bridge_args = [str(tree / 'source/bridge.rs') if arg == original_bridge_source
                   else str(archive) if arg == original_archive
                   else f'mech_gpu={runtime}' if arg.startswith('mech_gpu=') else arg
                   for arg in base_compile]
    runtime_args = [
        'rustc', TOOLCHAIN, '--target='+TARGET, '--edition=2024', '--crate-name=mech_gpu',
        '--crate-type=rlib', '-D', 'warnings', '-C', 'opt-level=3', '-C', 'debuginfo=0',
        '-C', 'codegen-units=1', '-C', 'panic=abort', str(tree / 'runtime/lib.rs'), '-o', str(runtime),
    ]
    link = before['commands']['link-mech']
    old = {f'-Wl,-map,{baseline / "mech.map"}': f'-Wl,-map,{tree / "mech.map"}',
           original_archive: str(archive), link[-1]: str(tree / 'mech')}
    stock_libraries = ('-lSystem', '-lc', '-lm')
    assert all(arg in link for arg in stock_libraries)
    link_args = [old.get(arg,arg) for arg in link if arg not in stock_libraries]
    fallback = [
        'cargo', TOOLCHAIN, 'rustc', '--offline', '--release', '--target', TARGET,
        '-Z', 'build-std=core,alloc,compiler_builtins', '--manifest-path', str(tree / 'Cargo.toml'),
        '--target-dir', str(tree/'cargo-target'), '--', '-D', 'warnings', '--print', 'native-static-libs',
    ]
    write(tree / 'plan.json', {
        'scope': 'Same frozen diagnostic native encoder and macOS 15 C deployment CLI',
        'sources': edited,
        'fixed_input_sha256': {str(path): sha(path) for path in paths.values()},
        'required_encoder_imports': sorted(REQUIRED),
        'runtime_rust_crate': runtime_args,
        'bridge_rust_staticlib': bridge_args,
        'clang_link_without_rust_native_libraries': link_args,
        'link_note': 'Append native-static-libs from the proposed bridge build. Clang supplies Darwin libSystem by default.',
        'rust_src_fallback': fallback,
        'fallback_bridge_archive': str(tree / 'cargo-target' / TARGET / 'release/libmech_deploy.a'),
        'stripping': ['/usr/bin/strip', '-S', '-x', str(tree/'mech-stripped')],
        'comparisons': {'baseline': str(baseline/'build.json'),
                        'fixture_verifier': str(deployment/'source/verify.py')},
    })
    print(tree / 'plan.json')


if __name__ == '__main__':
    cli = ArgumentParser(description=__doc__)
    cli.add_argument('output', type=Path, help='Fresh output directory')
    cli.add_argument('--deployment', type=Path, default=DEPLOY)
    cli.add_argument('--manifest', type=Path, required=True)
    cli.add_argument('--baseline', type=Path, required=True)
    args = cli.parse_args()
    prepare(args.output.resolve(), args.deployment.resolve(),
            args.manifest.resolve(), args.baseline.resolve())

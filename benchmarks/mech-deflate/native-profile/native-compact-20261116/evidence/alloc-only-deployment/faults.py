"""Replay the six established prepared-capacity allocation failures.

The diagnostic probe links the same native object and abort-core runtime archive
as the deployment candidate. Its only allocator difference is the established
one-shot refusal. The probe's output includes the native status and origin.
"""
from argparse import ArgumentParser
from ctypes import CDLL, Structure, addressof, byref, c_int, c_size_t, c_uint64, c_void_p, create_string_buffer
from hashlib import file_digest, sha256
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent

ROOT = '''
#![no_std]
use core::{ffi::c_void, ptr};
use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage, View};
#[path = "../source/native_abi.rs"]
mod native_abi;
use native_abi::{INPUT_OFFSET, INPUT_WORDS, OUTPUT_INPUT_OFFSET, OUTPUT_WORDS};
mod pinned_allocator;
#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    unsafe { abort() }
}
unsafe extern "C" {
    fn abort() -> !;
    fn mech_compact_closed_v2(heap: *mut c_void, input: *const u64, output: *mut u64) -> NativeStatus;
}

#[repr(C)]
pub struct Outcome {
    status: u64,
    origin: u64,
    requests: u64,
    bytes: u64,
    live_at_failure: u64,
}

fn view(data: *const u8, len: usize, cap: usize) -> View {
    [data as u64, len as u64, cap as u64, 1, len as u64, 0]
}

// Contract follows the existing private fault harness. All pointers are
// borrowed for this call. The test process calls sequentially.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn checked_failure_probe(
    input: *const u8, len: usize, prefix: *const u8, prefix_len: usize, cap: usize, result: *mut Outcome,
) -> i32 {
    let mut args = [0; INPUT_WORDS];
    args[INPUT_OFFSET .. INPUT_OFFSET + 6].copy_from_slice(&view(input, len, len));
    args[OUTPUT_INPUT_OFFSET .. OUTPUT_INPUT_OFFSET + 6].copy_from_slice(&view(prefix, prefix_len, cap));
    let mut output = [0; OUTPUT_WORDS];
    let mut space = OwnerStorage::default();
    let mut heap = Heap::with_owner_storage(&mut space);
    pinned_allocator::FAIL_CAPACITY.store(cap, core::sync::atomic::Ordering::Relaxed);
    let status = unsafe {
        mech_compact_closed_v2((&mut heap as *mut Heap).cast(), args.as_ptr(), output.as_mut_ptr())
    };
    let remaining = pinned_allocator::FAIL_CAPACITY.swap(0, core::sync::atomic::Ordering::Relaxed);
    let observations = Outcome {
        status: status.status,
        origin: status.origin,
        requests: heap.counters.payload_requests,
        bytes: heap.counters.requested_payload_bytes,
        live_at_failure: heap.counters.live_payload_bytes,
    };
    // The original destructor asserts that every private payload is retired.
    drop(heap);
    unsafe { ptr::write(result, observations) };
    i32::from(remaining != 0 || status.status == 0 || output != [0; OUTPUT_WORDS])
}
'''


def sha(path):
    with path.open('rb') as stream:
        return file_digest(stream, 'sha256').hexdigest()


def read(path):
    return json.loads(path.read_bytes())


def run(label, command):
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    (WORK / (label+'.log')).write_bytes(result.stdout)
    result.check_returncode()
    return result.stdout.decode(errors='replace')


def render_probe():
    WORK.mkdir()
    (WORK / 'lib.rs').write_text(ROOT)
    (WORK / 'Cargo.toml').write_text("""[package]
name = "mech_fault_probe"
version = "0.0.0"
edition = "2024"
publish = false

[lib]
path = "lib.rs"
crate-type = ["staticlib"]

[dependencies]
mech_gpu = { path = "../runtime" }

[profile.release]
opt-level = 3
panic = "abort"
debug = false
codegen-units = 1
""")
    allocator = (WORK.parent / 'source/pinned_allocator.rs').read_text()
    anchor = '    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {\n'
    fault = '''        if layout.align() == 1 && FAIL_CAPACITY.compare_exchange(
            layout.size(), 0, Ordering::Relaxed, Ordering::Relaxed,
        ).is_ok() {
            return core::ptr::null_mut();
        }
'''
    assert allocator.count(anchor) == 1
    (WORK / 'pinned_allocator.rs').write_text(
        allocator.replace(anchor, anchor+fault) + '''
use core::sync::atomic::{AtomicUsize, Ordering};
pub static FAIL_CAPACITY: AtomicUsize = AtomicUsize::new(0);
''',
    )


def main():
    global WORK
    parser = ArgumentParser(description=__doc__)
    parser.add_argument('trial', type=Path)
    parser.add_argument('--manifest', type=Path, required=True)
    parser.add_argument('--inventory', type=Path, required=True)
    parser.add_argument('--previous', type=Path, required=True)
    args = parser.parse_args()
    trial = args.trial.resolve()
    WORK = trial / 'fault-exact-provider'
    render_probe()
    inventory = args.inventory.resolve()
    previous_cases = args.previous.resolve()
    fixed = read(args.manifest)
    receipt = read(trial / 'receipt.json')
    deploy_archive = Path(next(
        arg for arg in receipt['commands']['link'] if Path(arg).name == 'libmech_deploy.a'
    ))
    members = subprocess.check_output(['/usr/bin/ar', '-t', str(deploy_archive)], text=True)
    runtime_member, = [name for name in members.splitlines() if name.startswith('mech_gpu-')]
    deployment_member = subprocess.check_output(['/usr/bin/ar', '-p', str(deploy_archive), runtime_member])
    deps = trial / 'cargo-target/aarch64-apple-darwin/release/deps'
    runtime_archive = deps / ('lib' + runtime_member.split('.', 1)[0] + '.rlib')
    runtime_digest = sha(runtime_archive)
    sysroot = WORK / 'sysroot'
    sysroot_libs = sysroot / 'lib/rustlib/aarch64-apple-darwin/lib'
    sysroot_libs.mkdir(parents=True)
    for crate in ('alloc', 'core', 'compiler_builtins'):
        basename, = [name for name in members.splitlines() if name.startswith(crate + '-')]
        library = deps / ('lib' + basename.split('.', 1)[0] + '.rlib')
        (sysroot_libs / library.name).symlink_to(library)
    archive = WORK / 'libmech_fault_probe.a'
    compile_cmd = ['rustc', '+nightly-2026-03-03', '--target=aarch64-apple-darwin',
                   '--edition=2024', '--crate-name=mech_fault_probe', '--crate-type=staticlib',
                   '-C', 'opt-level=3', '-C', 'panic=abort', '-C', 'codegen-units=1',
                   '--sysroot', str(sysroot), '-L', f'dependency={deps}',
                   '--extern', f'mech_gpu={runtime_archive}', str(WORK / 'lib.rs'),
                   '-o', str(archive)]
    run('compile', compile_cmd)
    assert sha(runtime_archive) == runtime_digest
    probe_member = subprocess.check_output(['/usr/bin/ar', '-p', str(archive), runtime_member])
    assert probe_member == deployment_member
    dylib = WORK / 'libfault.dylib'
    link = [
        '/usr/bin/clang', '-arch', 'arm64', '-mmacosx-version-min=15.0', '-dynamiclib',
        '-Wl,-dead_strip', '-Wl,-u,_checked_failure_probe', str(archive),
        fixed['mech_encoder']['path'], fixed['mimalloc']['path'], '-o', str(dylib),
    ]
    run('link', link)
    assert sha(inventory) == 'b6de359f4e8e5fa47c9ebed9474954f6e7b809e08d4f3b36193412492c4d9490'
    fixtures = {item['name']: item for item in read(inventory)}
    original = read(HERE / 'measurements/checked-failures.json')
    assert sha(previous_cases) == original['original_six_case_sha256']
    comparison = read(previous_cases)['cases']
    assert len(comparison) == 6

    class Outcome(Structure):
        _fields_ = [(field, c_uint64) for field in ('status','origin','requests','bytes','live_at_failure')]

    probe = CDLL(str(dylib)).checked_failure_probe
    probe.argtypes = [c_void_p, c_size_t, c_void_p, c_size_t, c_size_t, c_void_p]
    probe.restype = c_int
    done = {}
    for name, previous in comparison.items():
        fixture = fixtures[name]
        path = inventory.parent / fixture['input']
        assert sha(path) == fixture['input_sha256']
        data = path.read_bytes()
        prefix_path = inventory.parent / fixture['prefix']
        assert sha(prefix_path) == prefix_path.name
        prefix = prefix_path.read_bytes()
        assert len(data) == fixture['input_bytes'] and len(prefix) == fixture['prefix_bytes']
        cap = fixture['capacity']
        input_buf = create_string_buffer(data, max(len(data),1))
        prefix_buf = create_string_buffer(prefix, cap)
        original_input, original_prefix = input_buf.raw, prefix_buf.raw
        result = Outcome()
        for _ in range(2):
            status = probe(addressof(input_buf),len(data),addressof(prefix_buf),len(prefix),cap,byref(result))
            outcome = {'case':name, **{field:getattr(result,field) for field,_ in result._fields_}}
            assert status == 0 and outcome == previous['outcome'], (name,status,outcome,previous['outcome'])
            assert input_buf.raw == original_input and prefix_buf.raw == original_prefix, name
        done[name] = outcome
        print(name,result.status,result.origin,flush=True)
    (WORK/'proof.json').write_text(json.dumps({
        'original_six_case_sha256':sha(previous_cases),
        'runtime_archive_sha256':runtime_digest,
        'runtime_archive':str(runtime_archive),
        'runtime_object_member':runtime_member,
        'runtime_object_member_sha256':sha256(probe_member).hexdigest(),
        'dylib_sha256':sha(dylib),
        'compile':compile_cmd,
        'link':link,
        'verified':done,
        'scope':'Prepared-capacity native entry with a one-shot allocator failure. Same alloc-only runtime; separate from deployment CLI.',
    },indent=2)+'\n')


if __name__ == '__main__':
    main()

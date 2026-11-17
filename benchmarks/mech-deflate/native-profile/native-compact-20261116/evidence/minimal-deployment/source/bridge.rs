//! Narrow C bridge for the retained diagnostic raw DEFLATE artifact.

use mech_gpu::native_artifact::compact_runtime::{Heap, NativeStatus, OwnerStorage, View};
use std::ffi::{c_int, c_void};
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};

mod pinned_allocator;
#[global_allocator]
static ALLOCATOR: pinned_allocator::PinnedMimalloc = pinned_allocator::PinnedMimalloc;

mod native_abi;
use native_abi::{INPUT_OFFSET, INPUT_WORDS, OUTPUT_INPUT_OFFSET, OUTPUT_WORDS};

#[repr(C)]
pub struct DeployBuffer {
    data: *mut u8,
    len: usize,
    cap: usize,
}

impl DeployBuffer {
    const EMPTY: Self = Self { data: ptr::null_mut(), len: 0, cap: 0 };
}

unsafe extern "C" {
    fn mech_compact_closed_v2(
        heap: *mut c_void,
        input: *const u64,
        output: *mut u64,
    ) -> NativeStatus;
}

fn borrowed_view(data: *const u8, len: usize) -> View {
    let address = if data.is_null() { NonNull::<u8>::dangling().as_ptr().cast_const() } else { data };
    [address as u64, len as u64, len as u64, 1, len as u64, 0]
}

fn error_code(status: u64) -> c_int {
    c_int::try_from(status).unwrap_or(-1)
}

/// Encode one raw level-12 stream using the fixed diagnostic artifact.
///
/// # Safety
/// Output points to a writable, zeroed DeployBuffer disjoint from the input.
/// A nonempty input points to that many initialized bytes, borrowed through
/// return. The caller releases a successful output only with deploy_release.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn deploy_encode(
    input: *const u8,
    input_len: usize,
    output: *mut DeployBuffer,
) -> c_int {
    if input_len > isize::MAX as usize || (input_len != 0 && input.is_null()) {
        return -1;
    }
    let Some(output) = (unsafe { output.as_mut() }) else {
        return -1;
    };
    if !output.data.is_null() || output.len != 0 || output.cap != 0 {
        return -1;
    }

    let mut arguments = [0; INPUT_WORDS];
    arguments[INPUT_OFFSET..INPUT_OFFSET + 6].copy_from_slice(&borrowed_view(input, input_len));
    arguments[OUTPUT_INPUT_OFFSET..OUTPUT_INPUT_OFFSET + 6]
        .copy_from_slice(&borrowed_view(ptr::null(), 0));
    let mut native_output: [u64; OUTPUT_WORDS] = [0; OUTPUT_WORDS];
    let mut storage = OwnerStorage::default();
    let mut heap = Heap::with_owner_storage(&mut storage);
    // The exact object's metadata proves the packed U8 packet layout.
    let status = unsafe {
        mech_compact_closed_v2(
            (&mut heap as *mut Heap).cast(),
            arguments.as_ptr(),
            native_output.as_mut_ptr(),
        )
    };
    if status.status != 0 {
        return error_code(status.status);
    }
    let bytes = match unsafe { heap.take_u8(native_output) } {
        Ok(bytes) => bytes,
        Err(status) => return error_code(status),
    };
    drop(heap);
    let mut bytes = ManuallyDrop::new(bytes);
    *output = DeployBuffer {
        data: bytes.as_mut_ptr(),
        len: bytes.len(),
        cap: bytes.capacity(),
    };
    0
}

/// Release a result and reset the descriptor.
///
/// # Safety
/// Output is null, points to a zeroed descriptor, or owns the unmodified
/// allocation returned there by deploy_encode. The writable descriptor is
/// exclusive for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn deploy_release(output: *mut DeployBuffer) {
    let Some(output) = (unsafe { output.as_mut() }) else {
        return;
    };
    let previous = std::mem::replace(output, DeployBuffer::EMPTY);
    if !previous.data.is_null() {
        // The same pinned mimalloc backs the original Vec and this drop.
        unsafe { drop(Vec::from_raw_parts(previous.data, previous.len, previous.cap)) };
    }
}

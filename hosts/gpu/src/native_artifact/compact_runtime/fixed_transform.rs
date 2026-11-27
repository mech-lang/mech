//! Invocation-local replay of an admitted, allocation-free fixed U32 transform.
use super::Heap;
use std::{
    alloc::{Layout, alloc_zeroed},
    ptr::{self, NonNull},
    slice,
};

const MIN_WORDS: usize = 64;
const MAX_WORDS: usize = 512;
const SCALARS: usize = 4;

#[derive(Clone, Copy, Eq, PartialEq)]
struct Key {
    phase: usize,
    scalar_count: usize,
    rows: u64,
    columns: u64,
    scalars: [u64; SCALARS],
}

struct Entry {
    key: Key,
    input: [u32; MAX_WORDS],
    output: [u32; MAX_WORDS],
}

pub(super) struct FixedTransforms {
    entries: [Entry; 2],
}

impl FixedTransforms {
    fn try_allocate() -> Option<Box<Self>> {
        // All fields are integers. Zero initializes unused keys and keeps
        // the fixed payload arrays off the native call stack.
        let cache = NonNull::new(unsafe { alloc_zeroed(Layout::new::<Self>()) })?;
        Some(unsafe { Box::from_raw(cache.cast().as_ptr()) })
    }
}

// Compact numeric functions return the two native words in x0 and x1.
#[repr(C)]
pub struct PhaseResult {
    status: u64,
    site: u64,
}

type Phase = unsafe extern "C" fn(*mut Heap, *const u64, *mut u64) -> PhaseResult;

/// The compiler emits this call only after all existing owner and geometry
/// admissions. Its witnessed phase modifies just this one fixed initialized
/// U32 view, consumes all scalar input words and has no observable success
/// outputs besides the current view. The operator, inputs and geometry are
/// compared exactly, so no result can be used for another transform.
///
/// # Safety
/// The phase, packet and exclusive view are valid for the synchronous call.
/// The phase cannot allocate, transfer owners, export a continuation, or call
/// the replay helper recursively. The caller ignores output packet words on
/// success. The two packets may overlap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mech_compact_fixed_transform(
    context: *mut Heap,
    input: *const u64,
    output: *mut u64,
    phase: Phase,
    scalar_count: usize,
) -> PhaseResult {
    debug_assert!(scalar_count <= SCALARS);
    // SAFETY: the witnessed packet is [scalars, data, len, rows, cols,
    // current owner, capacity]. Finish reading before either packet is used
    // by the callee.
    let packet = unsafe { slice::from_raw_parts(input, scalar_count + 6) };
    let [data, length, rows, columns, _, capacity] = packet[scalar_count..] else {
        unreachable!()
    };
    let length = length as usize;
    if !(MIN_WORDS..=MAX_WORDS).contains(&length)
        || capacity as usize != length
        || !((rows == 1 && columns as usize == length) || (columns == 1 && rows as usize == length))
    {
        return unsafe { phase(context, input, output) };
    }
    let mut scalars = [0; SCALARS];
    scalars[..scalar_count].copy_from_slice(&packet[..scalar_count]);
    let key = Key {
        phase: phase as usize,
        scalar_count,
        rows,
        columns,
        scalars,
    };
    let data = ptr::with_exposed_provenance_mut::<u32>(data as usize);
    let entry = {
        // SAFETY: the caller's invocation owns this heap exclusively during
        // the lookup. No heap reference is retained across the phase call.
        let heap = unsafe { &mut *context };
        let Some(registry) = heap.registry.as_mut() else {
            return unsafe { phase(context, input, output) };
        };
        if registry.fixed_transforms.is_none() {
            registry.fixed_transforms = FixedTransforms::try_allocate();
        }
        let Some(cache) = registry.fixed_transforms.as_mut() else {
            return unsafe { phase(context, input, output) };
        };
        if let Some(previous) = cache
            .entries
            .iter()
            .find(|entry| entry.key.phase == key.phase)
        {
            let current = unsafe { slice::from_raw_parts(data, length) };
            if previous.key == key && &previous.input[..length] == current {
                // SAFETY: the saved words are private and disjoint from this
                // valid current writable view. Only initialized words copy.
                unsafe { ptr::copy_nonoverlapping(previous.output.as_ptr(), data, length) };
                return PhaseResult { status: 0, site: 0 };
            }
            None
        } else {
            cache
                .entries
                .iter_mut()
                .find(|entry| entry.key.phase == 0)
                .map(|entry| {
                    // Record the first completed visit. Differing input on later
                    // visits runs normally and leaves this snapshot available.
                    unsafe { ptr::copy_nonoverlapping(data, entry.input.as_mut_ptr(), length) };
                    NonNull::from(entry)
                })
        }
    };
    let result = unsafe { phase(context, input, output) };
    if result.status == 0
        && let Some(mut entry) = entry
    {
        // SAFETY: the fixed transform cannot replace this invocation's
        // registry, the current view or the private cache.
        let entry = unsafe { entry.as_mut() };
        unsafe { ptr::copy_nonoverlapping(data, entry.output.as_mut_ptr(), length) };
        entry.key = key;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // The packet writes distinguish actual calls from a restored result.
    // Make the first operation visible even on failure to model a partially
    // executed checked phase without accidentally saving its output.
    unsafe extern "C" fn increment(
        _: *mut Heap,
        input: *const u64,
        output: *mut u64,
    ) -> PhaseResult {
        let [step, data, count, _, _, _, _] = unsafe { *input.cast::<[u64; 7]>() };
        let data = ptr::with_exposed_provenance_mut::<u32>(data as usize);
        let values = unsafe { slice::from_raw_parts_mut(data, count as usize) };
        unsafe { output.write(17) };
        if step == 0 {
            values[0] = 7;
            return PhaseResult {
                status: 5,
                site: 123,
            };
        }
        for value in values {
            *value = value.wrapping_add(step as u32);
        }
        PhaseResult { status: 0, site: 0 }
    }

    #[test]
    fn restores_only_exact_successful_inputs_within_the_current_invocation() {
        let replay = |heap: &mut Heap<'_>, values: &mut [u32], step, rows| {
            let length = values.len() as u64;
            let packet = [
                step,
                values.as_mut_ptr().expose_provenance() as u64,
                length,
                rows,
                length / rows,
                1,
                length,
            ];
            let mut output = 0;
            // SAFETY: the test supplies a complete fixed view and a witnessed
            // equivalent phase stub. Output words are checked only as a
            // diagnostic signal for entry into that stub.
            let result = unsafe {
                mech_compact_fixed_transform(heap, packet.as_ptr(), &mut output, increment, 1)
            };
            (result.status, result.site, output)
        };

        let mut heap = Heap::new();
        heap.registry();
        let mut words = [u32::MAX; 128];
        for _ in 0..2 {
            assert_eq!(replay(&mut heap, &mut words, 0, 1), (5, 123, 17));
            assert_eq!(words[0], 7);
            words[0] = u32::MAX;
        }
        assert_eq!(replay(&mut heap, &mut words, 3, 1), (0, 0, 17));
        assert_eq!(words, [2; 128]);

        let mut other = [u32::MAX; 128];
        assert_eq!(replay(&mut heap, &mut other, 3, 128), (0, 0, 17));
        other.fill(u32::MAX);
        other[127] = 9;
        assert_eq!(replay(&mut heap, &mut other, 3, 1), (0, 0, 17));
        assert_eq!(other[127], 12);
        other.fill(u32::MAX);
        assert_eq!(replay(&mut heap, &mut other, 7, 1), (0, 0, 17));
        assert_eq!(other, [6; 128]);
        other.fill(u32::MAX);
        assert_eq!(replay(&mut heap, &mut other, 3, 1), (0, 0, 0));
        assert_eq!(other, [2; 128]);

        let mut next = Heap::new();
        next.registry();
        other.fill(u32::MAX);
        assert_eq!(replay(&mut next, &mut other, 3, 1), (0, 0, 17));
        assert_eq!(other, [2; 128]);
    }
}

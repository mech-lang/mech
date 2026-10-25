//! Execute both production candidate-address paths against concentrated keys.
use super::*;
use crate::native_artifact::program_packed_map::NarrowSlot;

#[test]
fn aligned_probe_generated_addresses_cover_both_strides_and_both_word_keys() {
    let mut alphabet = [0u64; 2];
    for &byte in b"ACGT" { alphabet[(byte / 64) as usize] |= 1 << (byte % 64); }
    let compact = Domain::from_alphabet(alphabet).unwrap();
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut functions = Vec::new();
    for domain in [Domain::ascii(), compact] {
        for peeled in [false, true] {
            let mut context = module.make_context();
            context.func.signature.params.push(AbiParam::new(types::I64));
            context.func.signature.returns.push(AbiParam::new(types::I64));
            let id = module.declare_function(&format!("aligned_probe_{}_{}", domain.bits(), peeled),
                Linkage::Local, &context.func.signature).unwrap();
            let mut fb = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
                let entry = b.create_block(); let absent = b.create_block(); let bad = b.create_block();
                b.append_block_params_for_function_params(entry); b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0];
                let frame = FrameState::memory(pointer, domain).with_specialized_width(Some(18));
                let result = probe_fresh_core_to(&mut b, &frame, true, bad, peeled.then_some(absent));
                b.ins().return_(&[result]);
                if peeled {
                    b.switch_to_block(absent);
                    let zero = b.ins().iconst(types::I64, 0); b.ins().return_(&[zero]);
                }
                b.switch_to_block(bad);
                let rejected = b.ins().iconst(types::I64, -1); b.ins().return_(&[rejected]);
                b.seal_all_blocks(); b.finalize();
            }
            module.define_function(id, &mut context).unwrap();
            functions.push((domain, id));
        }
    }
    module.finalize_definitions().unwrap();
    const CAPACITY: usize = 64;
    type Entry = unsafe extern "C" fn(*mut Frame) -> u64;
    for (domain, id) in functions {
        let narrow = narrow(domain);
        let keys = (0..2_000_000u64).filter_map(|low| {
            let high = if narrow { 0 } else { low.rotate_left(37) & ((1 << 62) - 1) };
            let mixed = crate::native_artifact::program_packed_map::hash(low, high);
            (mixed as usize & (CAPACITY - GROUP) == CAPACITY - GROUP && mixed >> 57 == 7)
                .then_some((low, high))
        }).take(CAPACITY + 1).collect::<Vec<_>>();
        assert_eq!(keys.len(), CAPACITY + 1);
        let mut wide = vec![Slot::default(); CAPACITY];
        let mut small = vec![NarrowSlot::default(); CAPACITY];
        assert_eq!((size_of::<NarrowSlot>(), size_of::<Slot>()), (16, 24));
        let base = if narrow { small.as_mut_ptr() as u64 } else { wide.as_mut_ptr() as u64 };
        let mut controls = vec![0x80u8; CAPACITY + GROUP - 1];
        let mut expected = std::collections::BTreeMap::new();
        // Fill all but one slot through independent aligned linear placement.
        // Hashes share both their first (last) group and their fingerprint.
        for (ordinal, &(low, high)) in keys[..CAPACITY - 1].iter().enumerate() {
            let index = (CAPACITY - GROUP + ordinal) % CAPACITY;
            let value = if ordinal % 2 == 0 { 0 } else { u64::MAX };
            if narrow { small[index] = NarrowSlot { low, value }; }
            else { wide[index] = Slot { low, high, value }; }
            controls[index] = 7;
            if index < GROUP - 1 { controls[CAPACITY + index] = 7; }
            expected.insert((low, high), base + (index * slot_bytes(domain)) as u64);
        }
        // SAFETY: the retained module defines this exact signature. All frame,
        // control and concrete payload backing stays live for synchronous calls.
        let entry = unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        for &(low, high) in &keys {
            // SAFETY: Frame contains only u64 words. Zero is a valid initial
            // bit pattern; every field used by this probe is assigned below.
            let mut frame: Frame = unsafe { core::mem::zeroed() };
            frame.slots = base; frame.controls = controls.as_ptr() as u64;
            frame.capacity = CAPACITY as u64; frame.len = (CAPACITY - 1) as u64;
            frame.key_low = low; frame.key_high = high; frame.width = 18;
            let wanted = expected.get(&(low, high)).copied().unwrap_or(0);
            assert_eq!(unsafe { entry(&mut frame) }, wanted);
            assert_eq!((frame.entry_pointer, frame.entry_known), (wanted, 1));
        }
    }
}

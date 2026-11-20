//! Widened index decoding must preserve the complete old item on strict ties.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn packed_wide_item_preserves_strict_choices_and_full_width_total_offsets() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for wide in [false, true] {
        for delta in [0u64, 1, 3, u64::MAX] {
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 3];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let mut builder = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let [input, positions, output] = b.block_params(entry)[..] else {
                unreachable!()
            };
            let costs = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
            let best = b.ins().load(types::I32, MemFlags::new(), input, 16);
            let old = b.ins().load(types::I64, MemFlags::new(), input, 24);
            let high = b.ins().load(types::I64, MemFlags::new(), input, 32);
            let positions = b.ins().load(types::I16X8, MemFlags::new(), positions, 0);
            let initial = super::packed_key::initial(&mut b, best);
            let key = super::packed_key::key(&mut b, costs, positions);
            let keys = b.ins().umin(key, initial);
            let (minimum, winner) = if wide {
                super::packed_key::finish_wide_item(&mut b, keys)
            } else {
                super::packed_key::finish(&mut b, keys)
            };
            let changed = b.ins().icmp_imm(IntCC::NotEqual, winner, -1);
            let index = if wide {
                winner
            } else {
                b.ins().uextend(types::I64, winner)
            };
            // This is the same pure total extension used by source item
            // reconstruction. Its value is discarded for an unchanged key.
            let index = b.ins().iadd_imm(index, delta as i64);
            let high = b.ins().band_imm(high, -4294967296);
            let item = b.ins().bor(index, high);
            let item = b.ins().select(changed, item, old);
            let minimum = b.ins().uextend(types::I64, minimum);
            b.ins().store(MemFlags::new(), minimum, output, 0);
            b.ins().store(MemFlags::new(), item, output, 8);
            b.ins().return_(&[item]);
            b.seal_all_blocks();
            b.finalize();
            let id = module
                .declare_function(
                    &format!("packed_item_{wide}_{delta}"),
                    Linkage::Local,
                    &context.func.signature,
                )
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            functions.push((id, delta));
        }
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u64, *const u16, *mut u64) -> u64;
    let calls = functions
        .into_iter()
        .map(|(id, delta)| {
            // SAFETY: the live finalized JIT owns this exact three-pointer C
            // signature; storage is provided and bounded below for every call.
            let call = unsafe {
                std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id))
            };
            (call, delta)
        })
        .collect::<Vec<_>>();
    let mut seed = 0xf1a7_32cb_806d_951eu64;
    for sample in 0..2048 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let best = [0, 1, 7, 65534, 65535, next() as u32 & 65535][sample % 6];
        let mut costs = std::array::from_fn::<_, 4, _>(|_| next() as u32);
        let mut positions = [0u16; 8];
        for position in &mut positions[..4] {
            *position = (next() % 65535 + 1) as u16;
        }
        match sample % 5 {
            0 => costs = [best; 4],
            1 => costs = [best.saturating_sub(1); 4],
            2 => {
                costs = [0, u32::MAX, 0, 0];
                positions[..4].copy_from_slice(&[65535, 1, 32768, 2]);
            }
            _ => {}
        }
        let old = next();
        let high = next();
        let input = [
            u64::from(costs[0]) | (u64::from(costs[1]) << 32),
            u64::from(costs[2]) | (u64::from(costs[3]) << 32),
            u64::from(best),
            old,
            high,
        ];
        let choice = costs
            .into_iter()
            .zip(positions)
            .filter(|&(cost, _)| cost < best)
            .min_by_key(|&(cost, ordinal)| (cost, ordinal));
        let original = (input, positions);
        for &(call, delta) in &calls {
            let minimum = choice.map_or(best, |(cost, _)| cost);
            let expected = choice.map_or(old, |(_, ordinal)| {
                (u64::from(ordinal) - 1).wrapping_add(delta) | (high & !0xffff_ffff)
            });
            let guard = 0xa5a5_a5a5_a5a5_a5a5;
            let mut output = [guard; 5];
            // SAFETY: all immutable input/ordinal lanes and the separate
            // guarded output cover every load/store and remain live and fixed
            // throughout the synchronous call. Best and ordinals fit U16.
            let actual = unsafe {
                call(
                    input.as_ptr(),
                    positions.as_ptr(),
                    output.as_mut_ptr().add(1),
                )
            };
            assert_eq!(actual, expected, "sample {sample}, delta {delta}");
            assert_eq!(output, [guard, u64::from(minimum), expected, guard, guard]);
            assert_eq!((input, positions), original);
        }
    }
}

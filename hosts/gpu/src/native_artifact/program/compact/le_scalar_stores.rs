//! Store an admitted prefix of a little-endian scalar with no excess writes.

use cranelift_codegen::ir::{
    Block, Endianness, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;

#[derive(Clone, Copy)]
pub(super) enum Packing {
    Widths,
    Ends,
}

/// The caller has checked count <= 8 and acquired writable backing for all
/// count bytes. Each selected store consumes its exact part of that prefix.
pub(super) fn store(
    b: &mut FunctionBuilder<'_>,
    mut address: Value,
    mut word: Value,
    count: Value,
    packing: Packing,
) {
    // Keep block identities for the original unrolled store region. Unused
    // blocks are never inserted into the layout.
    let blocks: [(Block, Block); 8] = std::array::from_fn(|_| (b.create_block(), b.create_block()));
    let (wide, shorter) = blocks[0];
    let finish = blocks[7].1;
    let little = MemFlags::new().with_endianness(Endianness::Little);
    let eight = b.ins().icmp_imm(IntCC::Equal, count, 8);
    b.ins().brif(eight, wide, &[], shorter, &[]);
    b.switch_to_block(wide);
    b.ins().store(little, word, address, 0);
    b.ins().jump(finish, &[]);

    b.switch_to_block(shorter);
    for ((put, next), (width, ty)) in blocks[1..3]
        .iter()
        .copied()
        .zip([(4, types::I32), (2, types::I16)])
    {
        match packing {
            Packing::Widths => {
                b.append_block_param(next, types::I64);
                b.append_block_param(next, types::I64);
                let selected = b.ins().band_imm(count, width);
                b.ins()
                    .brif(selected, put, &[], next, &[address.into(), word.into()]);
                b.switch_to_block(put);
                let value = b.ins().ireduce(ty, word);
                b.ins().store(little, value, address, 0);
                let advanced = b.ins().iadd_imm(address, width);
                let remaining = b.ins().ushr_imm(word, width * 8);
                b.ins().jump(next, &[advanced.into(), remaining.into()]);
                b.switch_to_block(next);
                address = b.block_params(next)[0];
                word = b.block_params(next)[1];
            }
            Packing::Ends => {
                let enough = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, count, width);
                b.ins().brif(enough, put, &[], next, &[]);
                b.switch_to_block(put);
                // This range is [width, 2 * width). Both writes fit and
                // overlapping bytes receive identical bits.
                let offset = b.ins().iadd_imm(count, -width);
                let tail = b.ins().iadd(address, offset);
                let shift = b.ins().ishl_imm(offset, 3);
                let high = b.ins().ushr(word, shift);
                let first = b.ins().ireduce(ty, word);
                let last = b.ins().ireduce(ty, high);
                b.ins().store(little, first, address, 0);
                b.ins().store(little, last, tail, 0);
                b.ins().jump(finish, &[]);
                b.switch_to_block(next);
            }
        }
    }

    let last = blocks[3].0;
    let selected = b.ins().band_imm(count, 1);
    b.ins().brif(selected, last, &[], finish, &[]);
    b.switch_to_block(last);
    let byte = b.ins().ireduce(types::I8, word);
    b.ins().store(little, byte, address, 0);
    b.ins().jump(finish, &[]);
    b.switch_to_block(finish);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module, default_libcall_names};

    #[test]
    fn exact_little_endian_prefix_preserves_adjacent_bytes() {
        for packing in [Packing::Widths, Packing::Ends] {
            verify_prefix(packing);
        }
    }

    fn verify_prefix(packing: Packing) {
        let mut module = JITModule::new(
            JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names()).unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let &[address, value, count] = b.block_params(entry) else {
            unreachable!("three admitted store words")
        };
        store(&mut b, address, value, count, packing);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function("store_prefix", Linkage::Local, &context.func.signature)
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this is the live JIT module's three-word store signature.
        let run: unsafe extern "C" fn(*mut u8, u64, u64) =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for value in [0u64, 0xF001A5C7682B9D4E, u64::MAX] {
            for offset in 1..=16 {
                for count in 0..=8 {
                    let mut data = [0x79; 32];
                    let mut expected = data;
                    expected[offset..offset + count].copy_from_slice(&value.to_le_bytes()[..count]);
                    unsafe { run(data.as_mut_ptr().add(offset), value, count as u64) };
                    assert_eq!(data, expected, "offset {offset} count {count}");
                }
            }
        }
    }
}

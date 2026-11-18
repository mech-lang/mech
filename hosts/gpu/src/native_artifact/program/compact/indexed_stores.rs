//! Share backing acquisition and the checked changed-value store.

use super::{
    array_growth::provider,
    layout::{Layout, LayoutKind, Type},
    packed,
    recipes::Recipe,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{
    Block, FuncRef, InstBuilder, MemFlags, Value, condcodes::IntCC, types,
};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::{FuncOrDataId, Module};
use mech_core::{IntegerWidth, SchemaBody};
use mech_engine::native_artifact::NativeCollectionPrimitive;

#[derive(Clone, Copy)]
pub(super) struct IndexedStore {
    pub(super) helper: FuncRef,
    pub(super) offset: Value,
    pub(super) value: Value,
    pub(super) finish: Block,
    pub(super) outline_reuse: bool,
}

pub(super) struct IndexedStores {
    helpers: [Option<FuncRef>; 4],
    outline_reuse: bool,
}

fn scalar_width(element: &Type) -> Option<usize> {
    let LayoutKind::Scalar {
        bytes,
        float: false,
        ..
    } = element.kind
    else {
        return None;
    };
    let width = usize::from(bytes);
    (element.words == 1 && element.packed_align == width && matches!(width, 1 | 2 | 4 | 8))
        .then_some(width)
}

impl IndexedStores {
    pub(super) fn declare<M: Module>(
        module: &mut M,
        b: &mut FunctionBuilder<'_>,
        layout: &Layout,
        actions: &[Recipe],
        consuming: &[bool],
        outline_reuse: bool,
    ) -> Result<Self, NativeCompileError> {
        let mut uses = [0; 4];
        for (action, &consume) in actions.iter().zip(consuming) {
            if consume
                && let Recipe::Buffer {
                    operation: NativeCollectionPrimitive::MatrixIndexedReplace,
                    output,
                    ..
                } = action
                && let Some(width) = scalar_width(layout.ty(output.get()).element()?)
            {
                uses[width.trailing_zeros() as usize] += 1;
            }
        }
        let mut helpers = [None; 4];
        let widths = [
            IntegerWidth::W8,
            IntegerWidth::W16,
            IntegerWidth::W32,
            IntegerWidth::W64,
        ];
        for (index, (&uses, integer)) in uses.iter().zip(widths).enumerate() {
            if uses < 2 {
                continue;
            }
            let width = 1usize << index;
            let suffix = if outline_reuse { "checked" } else { "backing" };
            let name = format!("mech_compact_replace_u{}_{suffix}", 8 * width);
            let element = Type::derive(&SchemaBody::UnsignedInteger(integer))?;
            let Some(FuncOrDataId::Func(writable)) =
                module.get_name("mech_compact_writable_scalar_backing")
            else {
                return Err(backend(
                    "indexed replacement requires registered scalar backing",
                ));
            };
            helpers[index] = Some(provider(
                module,
                b,
                &name,
                7,
                2,
                false,
                |module, helper| {
                    let inputs = helper.block_params(helper.current_block().unwrap());
                    let &[heap, data, length, capacity, owner, offset, value] = inputs else {
                        unreachable!("seven private indexed write words")
                    };
                    let writable = module.declare_func_in_func(writable, helper.func);
                    let changed = helper.create_block();
                    let done = helper.create_block();
                    for block in [changed, done] {
                        helper.append_block_param(block, types::I64);
                        helper.append_block_param(block, types::I64);
                    }
                    if outline_reuse {
                        let inspect = helper.create_block();
                        let acquire = helper.create_block();
                        helper.ins().brif(owner, inspect, &[], acquire, &[]);
                        helper.switch_to_block(inspect);
                        let refs = helper.ins().load(types::I64, MemFlags::new(), owner, 0);
                        // The requested capacity is the current capacity, so
                        // the original in-place gate reduces to uniqueness.
                        let unique = helper.ins().icmp_imm(IntCC::Equal, refs, 1);
                        helper.ins().brif(
                            unique,
                            changed,
                            &[data.into(), owner.into()],
                            acquire,
                            &[],
                        );
                        helper.switch_to_block(acquire);
                    }
                    let layout = helper.ins().iconst(types::I64, 2 * width as i64 + 1);
                    let call = helper.ins().call(
                        writable,
                        &[heap, data, length, capacity, owner, capacity, layout],
                    );
                    let &[data, owner_or_status] = helper.inst_results(call) else {
                        unreachable!("two backing result words")
                    };
                    let backing = [data.into(), owner_or_status.into()];
                    helper.ins().brif(data, changed, &backing, done, &backing);
                    helper.switch_to_block(changed);
                    let &[data, owner] = helper.block_params(changed) else {
                        unreachable!("two successful backing words")
                    };
                    let address = helper.ins().iadd(data, offset);
                    packed::store(helper, address, &element, &[value])?;
                    helper.ins().jump(done, &[data.into(), owner.into()]);
                    helper.switch_to_block(done);
                    let &[data, owner_or_status] = helper.block_params(done) else {
                        unreachable!("two backing result words")
                    };
                    helper.ins().return_(&[data, owner_or_status]);
                    Ok(())
                },
            )?);
        }
        Ok(Self {
            helpers,
            outline_reuse,
        })
    }

    pub(super) fn store(
        &self,
        element: &Type,
        offset: Value,
        values: &[Value],
        finish: Block,
    ) -> Option<IndexedStore> {
        let width = scalar_width(element)?;
        let &[value] = values else {
            return None;
        };
        let helper = self.helpers[width.trailing_zeros() as usize]?;
        Some(IndexedStore {
            helper,
            offset,
            value,
            finish,
            outline_reuse: self.outline_reuse,
        })
    }
}

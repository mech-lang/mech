//! Data-dependent safe addresses for immutable, still-owned capture cells.
//!
//! This applies only inside an outlined numeric callee. Its caller has already
//! admitted the views. A direct immutable ancestor slot owns each selected
//! backing before the call and until the root resumes after its return. Refuse
//! slots that any selected write can retire, or any exit can publish into.
//! Other owners must copy on write while this ancestor still owns the backing.
//!
//! Clamping makes the physical read safe independently of source control flow.
//! Empty captures use the runtime's immutable zero cell. The original selector
//! guard and rejecting source operation are emitted unchanged; their errors,
//! charges and publication order do not depend on this speculative read.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::plan::CodePlan;
use crate::native_artifact::program::prepare::readonly_numeric::{ArrayStorage, Operation, Proof};
use crate::native_artifact::program::prepare::tail::Region;
use cranelift_codegen::ir::{InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_core::{DimensionExpr, SchemaBody};
use mech_engine::native_artifact::NativeIterationAction as Action;
use std::collections::BTreeSet;

fn vector(schema: &SchemaBody) -> bool {
    matches!(schema, SchemaBody::Matrix { dimensions, .. }
        if dimensions.len() == 2 && dimensions.iter().any(|dimension|
            matches!(dimension, DimensionExpr::Constant(1))))
}

pub(super) fn stable_arrays(code: CodePlan<'_>, region: &Region, proof: &Proof) -> BTreeSet<usize> {
    let CodePlan::Program(program) = code else {
        return BTreeSet::new();
    };
    let metadata = program.metadata();
    let mut replaced = BTreeSet::from([region.tail.carrier().get(), region.tail.returned().get()]);
    replaced.extend(
        proof
            .cold_aliases
            .iter()
            .flat_map(|(_, aliases)| aliases.iter().map(|a| a.value)),
    );
    for &(site, ref operation) in &proof.actions {
        if matches!(
            operation,
            Operation::Write { .. } | Operation::Append { .. } | Operation::AppendLeBytes { .. }
        ) {
            let Some(call) = metadata
                .unsigned_buffer_calls
                .get(site)
                .and_then(Option::as_ref)
            else {
                return BTreeSet::new();
            };
            replaced.extend(call.owned_source_slots().iter().map(|value| value.get()));
        }
    }
    let facts = metadata.plan.facts();
    let sites = proof.sites.iter().copied().collect::<BTreeSet<_>>();
    proof
        .arrays
        .iter()
        .enumerate()
        .filter_map(|(at, array)| {
            let ArrayStorage::Capture { value } = array.storage else {
                return None;
            };
            if !array.immutable_capture() || replaced.contains(&value) {
                return None;
            }
            let definitions = facts.definition_sites(value).ok()?;
            if definitions.iter().any(|definition| {
                definition
                    .site()
                    .is_some_and(|site| sites.contains(&(site as usize)))
            }) {
                return None;
            }
            let Action::LibraryCall { template, .. } = code.blocks()[array.witness].action() else {
                return None;
            };
            vector(template.inputs()[0].body()).then_some(at)
        })
        .collect()
}

pub(super) fn flags() -> MemFlags {
    MemFlags::new()
        .with_readonly()
        .with_notrap()
        .with_can_move()
}

pub(super) fn address(
    b: &mut FunctionBuilder<'_>,
    ordinal: Value,
    length: Value,
    base: Value,
    width: u8,
) -> Result<(Value, cranelift_codegen::ir::Type), NativeCompileError> {
    // A validated Rust slice has at most isize::MAX initialized bytes. Its
    // clamped byte offset therefore fits, including an arbitrary raw selector.
    // Empty captures lend one immutable zero cell instead of a dangling slice.
    let nonempty = b.ins().icmp_imm(IntCC::NotEqual, length, 0);
    let zero = b.ins().iconst(types::I64, 0);
    let last = b.ins().iadd_imm(length, -1);
    let last = b.ins().select(nonempty, last, zero);
    let before = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, last);
    let safe = b.ins().select(before, ordinal, last);
    let bytes = b.ins().imul_imm(safe, i64::from(width));
    let address = b.ins().iadd(base, bytes);
    let ty = match width {
        1 => types::I8,
        2 => types::I16,
        4 => types::I32,
        8 => types::I64,
        _ => {
            return Err(crate::native_artifact::compiler::backend(
                "movable capture width differs",
            ));
        }
    };
    Ok((address, ty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module};

    #[test]
    fn movable_capture_reads_clamp_every_unsigned_width_without_control_guards() {
        let memory = flags();
        assert!(memory.readonly() && memory.notrap() && memory.can_move());
        assert!(!memory.aligned());
        assert!(memory.alias_region().is_none());
        for width in [1u8, 2, 4, 8] {
            let mut module =
                JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 3];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let function = module
                .declare_function("read", Linkage::Export, &context.func.signature)
                .unwrap();
            let mut frontend = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
                let block = b.create_block();
                b.append_block_params_for_function_params(block);
                b.switch_to_block(block);
                let args = b.block_params(block).to_vec();
                let (address, ty) = address(&mut b, args[2], args[1], args[0], width).unwrap();
                // Deliberately no selector guard: even an invalid raw ordinal
                // must access an initialized cell after arbitrary code motion.
                let loaded = b.ins().load(ty, memory, address, 0);
                let loaded = if width < 8 {
                    b.ins().uextend(types::I64, loaded)
                } else {
                    loaded
                };
                b.ins().return_(&[loaded]);
                b.seal_all_blocks();
                b.finalize();
            }
            module.define_function(function, &mut context).unwrap();
            module.finalize_definitions().unwrap();
            // SAFETY: the generated signature has three U64 parameters and
            // one U64 result, and the module outlives every invocation below.
            let read: unsafe extern "C" fn(u64, u64, u64) -> u64 =
                unsafe { core::mem::transmute(module.get_finalized_function(function)) };
            let bytes = core::array::from_fn::<_, 64, _>(|i| (i * 37 + 11) as u8);
            static ZERO: u64 = 0;
            for length in 0..=8u64 {
                let base = if length == 0 {
                    (&ZERO as *const u64) as u64
                } else {
                    bytes.as_ptr() as u64
                };
                for ordinal in (0..=length + 2).chain([1 << 63, u64::MAX]) {
                    let at = ordinal.min(length.saturating_sub(1)) as usize * usize::from(width);
                    let expected = if length == 0 {
                        0
                    } else {
                        match width {
                            1 => u64::from(bytes[at]),
                            2 => {
                                u64::from(u16::from_ne_bytes(bytes[at..at + 2].try_into().unwrap()))
                            }
                            4 => {
                                u64::from(u32::from_ne_bytes(bytes[at..at + 4].try_into().unwrap()))
                            }
                            8 => u64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap()),
                            _ => unreachable!(),
                        }
                    };
                    // SAFETY: nonempty cases lend length initialized cells;
                    // empty cases lend the static initialized zero cell.
                    let actual = unsafe { read(base, length, ordinal) };
                    assert_eq!(
                        actual, expected,
                        "width={width}, length={length}, ordinal={ordinal}"
                    );
                }
            }
        }
    }
}

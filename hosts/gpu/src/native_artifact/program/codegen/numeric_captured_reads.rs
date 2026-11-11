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
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData, MemFlags, Opcode, Value, ValueDef, types,
};
use cranelift_frontend::FunctionBuilder;
use mech_core::SchemaBody;
use std::collections::BTreeSet;

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
            Some(at)
        })
        .collect()
}

pub(super) fn flags() -> MemFlags {
    MemFlags::new()
        .with_readonly()
        .with_notrap()
        .with_can_move()
}

pub(super) struct Clamp {
    pub ordinal: Value,
    pub rows: Value,
}

pub(super) struct Candidate {
    pub site: usize,
    pub array: usize,
    pub load: Inst,
    pub ordinal: Value,
    pub rows: Value,
    pub clamp: Clamp,
}

fn invariant(
    function: &Function,
    dom: &cranelift_codegen::dominator_tree::DominatorTree,
    header: Block,
    value: Value,
    depth: usize,
    steps: &mut usize,
) -> bool {
    if depth == 0 || *steps == 0 {
        return false;
    }
    *steps -= 1;
    let value = function.dfg.resolve_aliases(value);
    match function.dfg.value_def(value) {
        ValueDef::Param(owner, _) => owner != header && dom.block_dominates(owner, header),
        ValueDef::Result(inst, _) => {
            if dom.dominates(inst, header, &function.layout) {
                return true;
            }
            if !function.dfg.value_type(value).is_int()
                || !matches!(
                    function.dfg.insts[inst].opcode(),
                    Opcode::Iconst
                        | Opcode::Iadd
                        | Opcode::IaddImm
                        | Opcode::Isub
                        | Opcode::Imul
                        | Opcode::ImulImm
                        | Opcode::Band
                        | Opcode::BandImm
                        | Opcode::Bor
                        | Opcode::BorImm
                        | Opcode::Bxor
                        | Opcode::BxorImm
                        | Opcode::Bnot
                        | Opcode::Ishl
                        | Opcode::IshlImm
                        | Opcode::Ushr
                        | Opcode::UshrImm
                        | Opcode::Sshr
                        | Opcode::SshrImm
                        | Opcode::Ireduce
                        | Opcode::Uextend
                        | Opcode::Sextend
                        | Opcode::Icmp
                        | Opcode::IcmpImm
                        | Opcode::Select
                        | Opcode::Umin
                        | Opcode::Umax
                )
            {
                return false;
            }
            function
                .dfg
                .inst_args(inst)
                .iter()
                .all(|&arg| invariant(function, dom, header, arg, depth - 1, steps))
        }
        _ => false,
    }
}

fn ordinary_operand(function: &mut Function, clamped: Value, original: Value) {
    let ValueDef::Result(inst, 0) = function.dfg.value_def(clamped) else {
        return;
    };
    if function.layout.inst_block(inst).is_none()
        || !matches!(
            function.dfg.insts[inst].opcode(),
            Opcode::Umin | Opcode::Umax
        )
        || function.dfg.inst_results(inst) != [clamped]
    {
        return;
    }
    function.dfg.clear_results(inst);
    function.dfg.change_to_alias(clamped, original);
    function.layout.remove_inst(inst);
    // Preserve aliases: the following unsigned-extent pass still names them.
}

/// Profit selection runs after local-phi cleanup. Safety still comes entirely
/// from admitted ownership and the clamped address, never from this heuristic.
/// Advancing or single-use reads retain ordinary source-guarded addressing.
pub(super) fn select(
    context: &mut cranelift_codegen::Context,
    candidates: &[Candidate],
    ordinary_flags: MemFlags,
) -> (Vec<(usize, usize)>, Vec<(usize, usize)>) {
    context.compute_cfg();
    context.compute_domtree();
    let mut loops = cranelift_codegen::loop_analysis::LoopAnalysis::new();
    loops.compute(&context.func, &context.cfg, &context.domtree);
    let selected = candidates
        .iter()
        .map(|candidate| {
            let Some(block) = context.func.layout.inst_block(candidate.load) else {
                return false;
            };
            let Some(lp) = loops.innermost_loop(block) else {
                return false;
            };
            let mut steps = 256;
            invariant(
                &context.func,
                &context.domtree,
                loops.loop_header(lp),
                candidate.ordinal,
                24,
                &mut steps,
            )
        })
        .collect::<Vec<_>>();
    let mut kept = Vec::new();
    let mut declined = Vec::new();
    for (candidate, reusable) in candidates.iter().zip(selected) {
        if context.func.layout.inst_block(candidate.load).is_none() {
            continue;
        }
        if reusable {
            kept.push((candidate.site, candidate.array));
            continue;
        }
        ordinary_operand(
            &mut context.func,
            candidate.clamp.ordinal,
            candidate.ordinal,
        );
        ordinary_operand(&mut context.func, candidate.clamp.rows, candidate.rows);
        let InstructionData::Load { flags, .. } = &mut context.func.dfg.insts[candidate.load]
        else {
            unreachable!("capture candidate lost its physical load");
        };
        *flags = ordinary_flags;
        declined.push((candidate.site, candidate.array));
    }
    (kept, declined)
}

pub(super) fn address(
    b: &mut FunctionBuilder<'_>,
    ordinal: Value,
    length: Value,
    base: Value,
    rows: Value,
    columns: Value,
    width: u8,
    schema: &SchemaBody,
) -> Result<(Value, cranelift_codegen::ir::Type, Clamp), NativeCompileError> {
    // A validated Rust slice has at most isize::MAX initialized bytes. Its
    // clamped byte offset therefore fits, including an arbitrary raw selector.
    // Empty captures lend one immutable zero cell instead of a dangling slice.
    let one = b.ins().iconst(types::I64, 1);
    let nonempty_length = b.ins().umax(length, one);
    let last = b.ins().iadd_imm(nonempty_length, -1);
    let safe = b.ins().umin(ordinal, last);
    // A nonempty validated view has nonzero rows. Empty matrices can have any
    // other dimension, so give their zero ordinal a nonzero divisor as well.
    // Both generic and unit-axis geometry are safe without a control guard.
    let rows = b.ins().umax(rows, one);
    let (address, ty) =
        super::numeric_words::linear_address(b, safe, rows, columns, width, schema, |_| base)?;
    Ok((
        address,
        ty,
        Clamp {
            ordinal: safe,
            rows,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::AbiParam;
    use cranelift_frontend::FunctionBuilderContext;
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::{Linkage, Module};
    use mech_core::{DimensionExpr as Dim, DimensionParameterId, IntegerWidth};

    #[test]
    fn movable_capture_reads_select_invariant_ordinals_and_remove_advancing_clamps() {
        use cranelift_codegen::ir::condcodes::IntCC;
        let schema = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: Box::new([
                Dim::Parameter(DimensionParameterId::new(0)),
                Dim::Constant(1),
            ]),
        };
        let mut module = JITModule::new(
            JITBuilder::with_flags(
                &[("opt_level", "speed")],
                cranelift_module::default_libcall_names(),
            )
            .unwrap(),
        );
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 4];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let id = module
            .declare_function(
                "select_capture_reads",
                Linkage::Export,
                &context.func.signature,
            )
            .unwrap();
        let mut frontend = FunctionBuilderContext::new();
        let mut candidates = Vec::new();
        let mut advancing_clamps = Vec::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
            let entry = b.create_block();
            let header = b.create_block();
            let body = b.create_block();
            let exit = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(header, types::I64);
            b.append_block_param(header, types::I64);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let zero = b.ins().iconst(types::I64, 0);
            let one = b.ins().iconst(types::I64, 1);
            b.ins().jump(header, &[zero.into(), zero.into()]);
            b.switch_to_block(header);
            let at = b.block_params(header)[0];
            let sum = b.block_params(header)[1];
            let requested = b.ins().icmp(IntCC::UnsignedLessThan, at, args[3]);
            let in_bounds = b.ins().icmp(IntCC::UnsignedLessThan, at, args[1]);
            let more = b.ins().band(requested, in_bounds);
            b.ins().brif(more, body, &[], exit, &[]);
            b.switch_to_block(body);
            let mut loaded = Vec::new();
            for (site, ordinal) in [(1, args[2]), (2, at)] {
                let (pointer, ty, clamp) =
                    address(&mut b, ordinal, args[1], args[0], args[1], one, 8, &schema).unwrap();
                let value = b.ins().load(ty, flags(), pointer, 0);
                let ValueDef::Result(load, 0) = b.func.dfg.value_def(value) else {
                    unreachable!()
                };
                if site == 2 {
                    advancing_clamps.extend([clamp.ordinal, clamp.rows]);
                }
                candidates.push(Candidate {
                    site,
                    array: 0,
                    load,
                    ordinal,
                    rows: args[1],
                    clamp,
                });
                loaded.push(value);
            }
            let combined = b.ins().iadd(loaded[0], loaded[1]);
            let next_sum = b.ins().iadd(sum, combined);
            let next = b.ins().iadd_imm(at, 1);
            b.ins().jump(header, &[next.into(), next_sum.into()]);
            b.switch_to_block(exit);
            b.ins().return_(&[sum]);
            b.seal_all_blocks();
            b.finalize();
        }
        let ordinary = MemFlags::new().with_notrap();
        let (kept, declined) = select(&mut context, &candidates, ordinary);
        assert_eq!(kept, [(1, 0)]);
        assert_eq!(declined, [(2, 0)]);
        assert_eq!(
            context.func.dfg.insts[candidates[0].load].memflags(),
            Some(flags())
        );
        assert_eq!(
            context.func.dfg.insts[candidates[1].load].memflags(),
            Some(ordinary)
        );
        for value in advancing_clamps {
            assert_ne!(context.func.dfg.resolve_aliases(value), value);
        }
        context.verify(module.isa()).unwrap();
        module.define_function(id, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: this owned JIT function has precisely the four-word ABI.
        let run: unsafe extern "C" fn(*const u64, u64, u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let values = [3u64, 7, 11, 19, 23, 31, 47, 53];
        for length in 1..=values.len() {
            for ordinal in [0, 1, 7, 1 << 63, u64::MAX] {
                for count in [0, 1, 2, 7, 8, 9, 99] {
                    let selected = values[ordinal.min(length as u64 - 1) as usize];
                    let expected = values[..length.min(count as usize)]
                        .iter()
                        .map(|v| v + selected)
                        .sum::<u64>();
                    // SAFETY: the advancing source guard admits only initialized cells;
                    // the invariant load retains its independently safe clamp.
                    assert_eq!(
                        unsafe { run(values.as_ptr(), length as u64, ordinal, count) },
                        expected
                    );
                }
            }
        }
    }

    #[test]
    fn movable_capture_reads_clamp_every_unsigned_width_without_control_guards() {
        let memory = flags();
        assert!(memory.readonly() && memory.notrap() && memory.can_move());
        assert!(!memory.aligned());
        assert!(memory.alias_region().is_none());
        for width in [1u8, 2, 4, 8] {
            let schema = SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(match width {
                    1 => IntegerWidth::W8,
                    2 => IntegerWidth::W16,
                    4 => IntegerWidth::W32,
                    8 => IntegerWidth::W64,
                    _ => unreachable!(),
                })),
                dimensions: Box::new([
                    Dim::Parameter(DimensionParameterId::new(0)),
                    Dim::Parameter(DimensionParameterId::new(1)),
                ]),
            };
            let mut module = JITModule::new(
                JITBuilder::with_flags(
                    &[("opt_level", "speed")],
                    cranelift_module::default_libcall_names(),
                )
                .unwrap(),
            );
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 5];
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
                let (address, ty, _) = address(
                    &mut b, args[4], args[1], args[0], args[2], args[3], width, &schema,
                )
                .unwrap();
                // Deliberately no selector guard: an invalid raw ordinal must
                // still access initialized storage after arbitrary code motion.
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
            // SAFETY: the signature has five U64 parameters and one U64 result.
            // The module outlives every invocation below.
            let read: unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64 =
                unsafe { core::mem::transmute(module.get_finalized_function(function)) };
            let bytes = core::array::from_fn::<_, 64, _>(|i| (i * 37 + 11) as u8);
            static ZERO: u64 = 0;
            for (rows, columns) in [
                (0u64, 0u64),
                (0, u64::MAX),
                (u64::MAX, 0),
                (1, 0),
                (0, 1),
                (1, 1),
                (1, 8),
                (8, 1),
                (2, 3),
                (3, 2),
                (2, 4),
                (4, 2),
            ] {
                let length = rows * columns;
                let base = if length == 0 {
                    (&ZERO as *const u64) as u64
                } else {
                    bytes.as_ptr() as u64
                };
                for ordinal in (0..=length + 2).chain([1 << 63, u64::MAX]) {
                    let logical = ordinal.min(length.saturating_sub(1));
                    let physical = if length == 0 {
                        0
                    } else {
                        (logical % rows) * columns + logical / rows
                    };
                    let at = physical as usize * usize::from(width);
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
                    let actual = unsafe { read(base, length, rows, columns, ordinal) };
                    assert_eq!(
                        actual, expected,
                        "width={width}, rows={rows}, columns={columns}, ordinal={ordinal}"
                    );
                }
            }
        }
    }
}

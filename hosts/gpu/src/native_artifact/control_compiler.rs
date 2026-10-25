//! Canonical scalar CFG definition shared by JIT and Object emission.
use crate::native_artifact::abi::{NO_NODE, NativeEntryFrame as Frame, STATUS_ABI, STATUS_OK};
use crate::native_artifact::compiler::{
    admit, backend, compiled_stack_bytes, frame_load, guard, host_supported, limit, offset,
    primitive, set_origin,
};
use crate::native_artifact::control_abi::{
    ENTRY, Entry, NativeMatchDescriptor, STATUS_REJECT, VERSION,
};
use crate::native_artifact::control_runtime::{
    NativeMatchExecutionError, NativeMatchExecutionReport,
};
use crate::native_artifact::{NativeCompileError, NativeCompileLimits, NativeExecutionLimits};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, types};
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{
    NativeMatchAction, NativeMatchArtifactPlan, NativeMatchTerminator, NativeMatchValue,
    NativeScalarKind, NativeScalarValue,
};
use std::collections::VecDeque;

pub struct JitMatchArtifact {
    _module: cranelift_jit::JITModule,
    entry: Entry,
    descriptor: NativeMatchDescriptor,
}
impl JitMatchArtifact {
    pub fn compile(plan: &NativeMatchArtifactPlan) -> Result<Self, NativeCompileError> {
        Self::compile_with_limits(plan, NativeCompileLimits::default())
    }
    pub fn compile_with_limits(
        plan: &NativeMatchArtifactPlan,
        limits: NativeCompileLimits,
    ) -> Result<Self, NativeCompileError> {
        host_supported()?;
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[
                ("opt_level", "none"),
                ("is_pic", "true"),
                ("preserve_frame_pointers", "true"),
                ("unwind_info", "true"),
            ],
            default_libcall_names(),
        )
        .map_err(backend)?;
        let mut module = cranelift_jit::JITModule::new(builder);
        let (function, descriptor) = define(plan, &mut module, limits)?;
        module.finalize_definitions().map_err(backend)?;
        // SAFETY: the one checked definition emits the private repr(C) ABI;
        // this owner retains its module for every synchronous invocation.
        let entry = unsafe {
            std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(function))
        };
        Ok(Self {
            _module: module,
            entry,
            descriptor,
        })
    }
    pub fn descriptor(&self) -> &NativeMatchDescriptor {
        &self.descriptor
    }
    #[cfg(test)]
    pub(super) fn test_checked_entry(&self) -> Entry { self.entry }
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeMatchExecutionReport, NativeMatchExecutionError> {
        crate::native_artifact::control_runtime::execute(
            &self.descriptor,
            |frame| {
                // SAFETY: this owner created and retains the exact checked entry.
                unsafe { (self.entry)(frame) }
            },
            inputs,
            outputs,
            limits,
            budget,
        )
    }
}
pub struct NativeMatchObject {
    bytes: Vec<u8>,
    descriptor: NativeMatchDescriptor,
    identity: crate::native_artifact::control_identity::NativeMatchObjectIdentity,
}
impl NativeMatchObject {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn descriptor(&self) -> &NativeMatchDescriptor {
        &self.descriptor
    }
    pub fn identity_bytes(&self) -> &[u8; 168] {
        self.identity.bytes()
    }
    pub fn identity_symbol(&self) -> &str {
        self.identity.symbol()
    }
    pub fn identity(&self) -> &crate::native_artifact::control_identity::NativeMatchObjectIdentity {
        &self.identity
    }
    pub fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "schema":"mech.canonical-scalar-match-object.v1","control_abi_version":VERSION,
            "typed_ir_sha256":crate::native_artifact::identity::hex(self.descriptor.typed_ir_sha256()),
            "identity_symbol":self.identity.symbol(),"stack_bytes":self.descriptor.stack_bytes(),"code_bytes":self.descriptor.code_bytes(),
            "linked":false,"executed":false,"timing_samples":0,
        })
    }
}
pub fn emit_match_object(
    plan: &NativeMatchArtifactPlan,
) -> Result<NativeMatchObject, NativeCompileError> {
    emit_match_object_with_limits(plan, NativeCompileLimits::default())
}
pub fn emit_match_object_with_limits(
    plan: &NativeMatchArtifactPlan,
    limits: NativeCompileLimits,
) -> Result<NativeMatchObject, NativeCompileError> {
    host_supported()?;
    let mut flags = settings::builder();
    for (key, value) in [
        ("opt_level", "none"),
        ("is_pic", "true"),
        ("preserve_frame_pointers", "true"),
        ("unwind_info", "true"),
    ] {
        flags.set(key, value).map_err(backend)?;
    }
    let isa = cranelift_native::builder()
        .map_err(backend)?
        .finish(settings::Flags::new(flags))
        .map_err(backend)?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "mech_canonical_scalar_match", default_libcall_names())
            .map_err(backend)?,
    );
    let (_, descriptor) = define(plan, &mut module, limits)?;
    let identity =
        crate::native_artifact::control_identity::NativeMatchObjectIdentity::from_descriptor(
            &descriptor,
        )
        .map_err(backend)?;
    let id = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(identity.bytes().to_vec().into_boxed_slice());
    data.set_align(8);
    module.define_data(id, &data).map_err(backend)?;
    let bytes = module.finish().emit().map_err(backend)?;
    Ok(NativeMatchObject {
        bytes,
        descriptor,
        identity,
    })
}

fn successors(term: NativeMatchTerminator) -> [Option<u32>; 2] {
    match term {
        NativeMatchTerminator::Jump(next) => [Some(next), None],
        NativeMatchTerminator::Branch { yes, no, .. }
        | NativeMatchTerminator::LiteralBranch { yes, no, .. } => [Some(yes), Some(no)],
        _ => [None, None],
    }
}
fn action_reads(action: &NativeMatchAction) -> Vec<NativeMatchValue> {
    let reads = match action {
        NativeMatchAction::Primitive { operands, .. } => operands.to_vec(),
        NativeMatchAction::Copy { source, .. } => vec![*source],
        NativeMatchAction::None => Vec::new(),
    };
    reads
}
fn output(action: &NativeMatchAction) -> Option<NativeMatchValue> {
    match action {
        NativeMatchAction::Primitive { output, .. } | NativeMatchAction::Copy { output, .. } => {
            Some(*output)
        }
        NativeMatchAction::None => None,
    }
}

/// Check dominance and all exact types without constructing a callable plan.
/// Metadata is private-constructor, but the emitter still refuses malformed CFG.
fn validate(plan: &NativeMatchArtifactPlan) -> Result<(), NativeCompileError> {
    let bad = || NativeCompileError::InvalidPlan {
        node: None,
        reason: "invalid Match CFG or scalar dominance",
    };
    let n = plan.blocks().len();
    let count = plan.values().len();
    if plan.entry() as usize >= n {
        return Err(bad());
    }
    let ty = |value: NativeMatchValue| plan.values().get(value.get() as usize).ok_or_else(bad);
    let mut predecessors = vec![Vec::new(); n];
    let mut degrees = vec![0usize; n];
    for (index, block) in plan.blocks().iter().enumerate() {
        for next in successors(block.terminator()).into_iter().flatten() {
            let next = next as usize;
            if next >= n {
                return Err(bad());
            }
            predecessors[next].push(index);
            degrees[next] = degrees[next].checked_add(1).ok_or_else(bad)?;
        }
        match block.action() {
            NativeMatchAction::Primitive {
                op,
                operands,
                output,
                ..
            } => {
                let (args, result) = op.signature();
                if operands.len() != args.len() || ty(*output)?.kind() != result {
                    return Err(bad());
                }
                for (&value, &kind) in operands.iter().zip(args) {
                    if ty(value)?.kind() != kind {
                        return Err(bad());
                    }
                }
            }
            NativeMatchAction::Copy { source, output } => {
                if ty(*source)? != ty(*output)? {
                    return Err(bad());
                }
            }
            NativeMatchAction::None => {}
        }
        match block.terminator() {
            NativeMatchTerminator::Branch { condition, .. } => {
                if ty(condition)?.kind() != NativeScalarKind::Bool {
                    return Err(bad());
                }
            }
            NativeMatchTerminator::LiteralBranch { left, right, .. } => {
                if ty(left)? != ty(right)? {
                    return Err(bad());
                }
            }
            _ => {}
        }
    }
    let mut initial = vec![false; count];
    for input in plan.inputs() {
        let id = input.value().get() as usize;
        if id >= count || ty(input.value())? != input.declaration().ty() {
            return Err(bad());
        }
        initial[id] = true;
    }
    for constant in plan.constants() {
        let id = constant.value().get() as usize;
        if id >= count {
            return Err(bad());
        }
        let kind = match constant.scalar() {
            NativeScalarValue::Bool(_) => NativeScalarKind::Bool,
            NativeScalarValue::U8(_) => NativeScalarKind::U8,
            NativeScalarValue::U16(_) => NativeScalarKind::U16,
            NativeScalarValue::U32(_) => NativeScalarKind::U32,

            NativeScalarValue::U64(_) => NativeScalarKind::U64,
            NativeScalarValue::I64(_) => NativeScalarKind::I64,
            NativeScalarValue::F64(_) => NativeScalarKind::F64,
        };
        if ty(constant.value())?.kind() != kind {
            return Err(bad());
        }
        initial[id] = true;
    }
    let mut ends = vec![Vec::<bool>::new(); n];
    let mut ready = (0..n)
        .filter(|&id| degrees[id] == 0)
        .collect::<VecDeque<_>>();
    let mut seen = 0;
    while let Some(id) = ready.pop_front() {
        seen += 1;
        let block = &plan.blocks()[id];
        let mut known = if let Some(&first) = predecessors[id].first() {
            ends[first].clone()
        } else {
            initial.clone()
        };
        for &parent in predecessors[id].iter().skip(1) {
            for (bit, &other) in known.iter_mut().zip(&ends[parent]) {
                *bit &= other;
            }
        }
        for value in action_reads(block.action()) {
            if !known.get(value.get() as usize).copied().unwrap_or(false) {
                return Err(bad());
            }
        }
        if let Some(value) = output(block.action()) {
            known[value.get() as usize] = true;
        }
        let conditions = match block.terminator() {
            NativeMatchTerminator::Branch { condition, .. } => vec![condition],
            NativeMatchTerminator::LiteralBranch { left, right, .. } => vec![left, right],
            _ => Vec::new(),
        };
        for value in conditions {
            if !known.get(value.get() as usize).copied().unwrap_or(false) {
                return Err(bad());
            }
        }
        if block.terminator() == NativeMatchTerminator::Complete {
            for output in plan.outputs() {
                if !known
                    .get(output.value().get() as usize)
                    .copied()
                    .unwrap_or(false)
                    || ty(output.value())? != output.ty()
                {
                    return Err(bad());
                }
            }
        }
        ends[id] = known;
        for next in successors(block.terminator()).into_iter().flatten() {
            let next = next as usize;
            degrees[next] -= 1;
            if degrees[next] == 0 {
                ready.push_back(next);
            }
        }
    }
    if seen != n {
        return Err(bad());
    }
    Ok(())
}

pub(in crate::native_artifact) fn load(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    value: NativeMatchValue,
) -> Result<Value, NativeCompileError> {
    Ok(b.ins().load(
        types::I64,
        MemFlags::new(),
        locals,
        offset(value.get() as usize)?,
    ))
}
fn define<M: Module>(
    plan: &NativeMatchArtifactPlan,
    module: &mut M,
    limits: NativeCompileLimits,
) -> Result<(FuncId, NativeMatchDescriptor), NativeCompileError> {
    host_supported()?;
    if module.target_config().pointer_type() != types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    limit("Match blocks", plan.blocks().len() as u64, limits.max_nodes)?;
    limit(
        "Match inputs",
        plan.inputs().len() as u64,
        limits.max_inputs,
    )?;
    limit(
        "Match outputs",
        plan.outputs().len() as u64,
        limits.max_outputs,
    )?;
    limit(
        "Match scalar values",
        plan.values().len() as u64,
        limits.max_nodes.saturating_mul(8),
    )?;
    offset(plan.values().len())?;
    offset(plan.inputs().len())?;
    offset(plan.outputs().len())?;
    validate(plan)?;
    let mut signature_entry = module.make_signature();
    signature_entry.params.push(AbiParam::new(types::I64));
    signature_entry.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(ENTRY, Linkage::Export, &signature_entry)
        .map_err(backend)?;
    let mut callback_signature = module.make_signature();
    callback_signature.params = vec![AbiParam::new(types::I64); 2];
    callback_signature.returns.push(AbiParam::new(types::I64));
    let mut context = module.make_context();
    context.func.signature = signature_entry;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let frame = b.block_params(entry)[0];
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        b.set_cold_block(error);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, frame, 0);
        guard(&mut b, nonnull, error, STATUS_ABI);
        set_origin(&mut b, frame, NO_NODE);
        for (field, value) in [
            (core::mem::offset_of!(Frame, version), VERSION),
            (
                core::mem::offset_of!(Frame, input_count),
                plan.inputs().len() as u64,
            ),
            (
                core::mem::offset_of!(Frame, local_count),
                plan.values().len() as u64,
            ),
            (
                core::mem::offset_of!(Frame, output_count),
                plan.outputs().len() as u64,
            ),
        ] {
            let actual = frame_load(&mut b, frame, field);
            let valid = b.ins().icmp_imm(IntCC::Equal, actual, value as i64);
            guard(&mut b, valid, error, STATUS_ABI);
        }
        let inputs = frame_load(&mut b, frame, core::mem::offset_of!(Frame, inputs));
        let locals = frame_load(&mut b, frame, core::mem::offset_of!(Frame, locals));
        let outputs = frame_load(&mut b, frame, core::mem::offset_of!(Frame, outputs));
        let user_context = frame_load(&mut b, frame, core::mem::offset_of!(Frame, context));
        let callback = frame_load(&mut b, frame, core::mem::offset_of!(Frame, admit_next));
        for (pointer, count) in [
            (inputs, plan.inputs().len()),
            (locals, plan.values().len()),
            (outputs, plan.outputs().len()),
            (user_context, 1),
            (callback, 1),
        ] {
            if count > 0 {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(&mut b, valid, error, STATUS_ABI);
            }
        }
        for (index, input) in plan.inputs().iter().enumerate() {
            let word = b
                .ins()
                .load(types::I64, MemFlags::new(), inputs, offset(index)?);
            if let Some(maximum) = super::compiler::word_upper_bound(input.declaration().ty().kind()) {
                let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, word, maximum as i64);
                guard(&mut b, valid, error, STATUS_ABI);
            }
            b.ins().store(
                MemFlags::new(),
                word,
                locals,
                offset(input.value().get() as usize)?,
            );
        }
        for constant in plan.constants() {
            let word = match constant.scalar() {
                NativeScalarValue::Bool(value) => u64::from(value),
                NativeScalarValue::U8(value) => u64::from(value),
                NativeScalarValue::U16(value) => u64::from(value),
                NativeScalarValue::U32(value) => u64::from(value),

                NativeScalarValue::U64(value) => value,
                NativeScalarValue::I64(value) => value as u64,
                NativeScalarValue::F64(value) => value.bits(),
            };
            let value = b.ins().iconst(types::I64, word as i64);
            b.ins().store(
                MemFlags::new(),
                value,
                locals,
                offset(constant.value().get() as usize)?,
            );
        }
        let callback_signature = b.import_signature(callback_signature);
        let mut reachable = vec![false; plan.blocks().len()];
        let mut pending = vec![plan.entry() as usize];
        while let Some(id) = pending.pop() {
            if reachable[id] {
                continue;
            }
            reachable[id] = true;
            pending.extend(
                successors(plan.blocks()[id].terminator())
                    .into_iter()
                    .flatten()
                    .map(|next| next as usize),
            );
        }
        let blocks = reachable
            .iter()
            .map(|&used| used.then(|| b.create_block()))
            .collect::<Vec<_>>();
        b.ins()
            .jump(blocks[plan.entry() as usize].expect("reachable entry"), &[]);
        for (index, block) in plan.blocks().iter().enumerate() {
            if !reachable[index] {
                continue;
            }
            b.switch_to_block(blocks[index].expect("reachable block"));
            let before = b.func.dfg.num_insts();
            set_origin(&mut b, frame, index as u64);
            admit(
                &mut b,
                callback_signature,
                callback,
                user_context,
                index,
                error,
            );
            emit_scalar_action(&mut b, locals, block.action(), error)?;
            emit_scalar_terminator(
                &mut b,
                frame,
                locals,
                outputs,
                block.terminator(),
                &blocks,
                plan.outputs(),
                plan.values(),
            )?;
            let emitted = (b.func.dfg.num_insts() - before) as u64;
            let bound = match block.action() {
                NativeMatchAction::Primitive { op, .. } => {
                    crate::native_artifact::abi::primitive_work(*op)
                }
                _ => {
                    if block.terminator() == NativeMatchTerminator::Complete {
                        (plan.outputs().len() as u64)
                            .checked_mul(8)
                            .and_then(|v| v.checked_add(128))
                            .ok_or_else(|| backend("Match completion overflow"))?
                    } else {
                        128
                    }
                }
            };
            if emitted.checked_add(64).is_none_or(|total| total > bound) {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(block.origin().node()),
                    reason: "Match visit exceeds conservative work template",
                });
            }
        }
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or(NativeCompileError::StackMetadata {
            reason: "Match compiled code missing",
        })?;
    let stack = compiled_stack_bytes(&compiled.buffer)?;
    let code = compiled.code_info().total_size as u64;
    limit("Match stack", stack, limits.max_stack_bytes)?;
    limit("Match code", code, limits.max_code_bytes)?;
    Ok((
        function,
        NativeMatchDescriptor::from_plan(plan, stack, code).map_err(backend)?,
    ))
}

/// One maintained scalar semantic emitter, shared by acyclic and loop CFGs.
pub(in crate::native_artifact) fn emit_scalar_action(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    action: &NativeMatchAction,
    error: cranelift_codegen::ir::Block,
) -> Result<(), NativeCompileError> {
    match action {
        NativeMatchAction::Primitive {
            op,
            operands,
            output,
            ..
        } => {
            let args = operands
                .iter()
                .map(|&value| load(b, locals, value))
                .collect::<Result<Vec<_>, _>>()?;
            let value = primitive(b, *op, &args, error);
            b.ins().store(
                MemFlags::new(),
                value,
                locals,
                offset(output.get() as usize)?,
            );
        }
        NativeMatchAction::Copy { source, output } => {
            let value = load(b, locals, *source)?;
            b.ins().store(
                MemFlags::new(),
                value,
                locals,
                offset(output.get() as usize)?,
            );
        }
        NativeMatchAction::None => {}
    }

    Ok(())
}

/// Literal branches preserve language equality for all exact scalar kinds.
pub(in crate::native_artifact) fn emit_scalar_terminator(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    locals: Value,
    outputs: Value,
    terminator: NativeMatchTerminator,
    blocks: &[Option<cranelift_codegen::ir::Block>],
    output_values: &[mech_engine::native_artifact::NativeMatchOutput],
    value_types: &[mech_engine::native_artifact::NativeScalarType],
) -> Result<(), NativeCompileError> {
    if terminator == NativeMatchTerminator::Complete {
        set_origin(b, frame, NO_NODE);
        for (index, output) in output_values.iter().enumerate() {
            let value = load(b, locals, output.value())?;
            b.ins()
                .store(MemFlags::new(), value, outputs, offset(index)?);
        }
        let status = b.ins().iconst(types::I64, STATUS_OK as i64);
        b.ins().return_(&[status]);
        return Ok(());
    }
    emit_branch_terminator(b, locals, terminator, blocks, |value| {
        value_types.get(value.get() as usize).map(|ty| ty.kind())
    })
}

/// The same native branches serve scalar and mixed-value CFGs. A mixed value
/// never acquires a fictitious scalar type merely to reuse this emitter.
pub(in crate::native_artifact) fn emit_branch_terminator(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    terminator: NativeMatchTerminator,
    blocks: &[Option<cranelift_codegen::ir::Block>],
    kind: impl Fn(NativeMatchValue) -> Option<NativeScalarKind>,
) -> Result<(), NativeCompileError> {
    match terminator {
        NativeMatchTerminator::Jump(next) => {
            b.ins()
                .jump(blocks[next as usize].expect("reachable successor"), &[]);
        }
        NativeMatchTerminator::Branch { condition, yes, no } => {
            let value = load(b, locals, condition)?;
            let condition = b.ins().icmp_imm(IntCC::NotEqual, value, 0);
            b.ins().brif(
                condition,
                blocks[yes as usize].expect("reachable successor"),
                &[],
                blocks[no as usize].expect("reachable successor"),
                &[],
            );
        }
        NativeMatchTerminator::LiteralBranch {
            left,
            right,
            yes,
            no,
        } => {
            let kind = kind(left).ok_or(NativeCompileError::InvalidPlan {
                node: None,
                reason: "literal branch needs its exact scalar kind",
            })?;
            let left = load(b, locals, left)?;
            let right = load(b, locals, right)?;
            // Match literal equality is language equality, not raw
            // transport-bit identity: signed zeros match, NaNs do not.
            let condition = if kind == NativeScalarKind::F64 {
                let left = b.ins().bitcast(types::F64, MemFlags::new(), left);
                let right = b.ins().bitcast(types::F64, MemFlags::new(), right);
                b.ins().fcmp(FloatCC::Equal, left, right)
            } else {
                b.ins().icmp(IntCC::Equal, left, right)
            };
            b.ins().brif(
                condition,
                blocks[yes as usize].expect("reachable successor"),
                &[],
                blocks[no as usize].expect("reachable successor"),
                &[],
            );
        }
        NativeMatchTerminator::Reject => {
            let status = b.ins().iconst(types::I64, STATUS_REJECT as i64);
            b.ins().return_(&[status]);
        }
        NativeMatchTerminator::IntegrityFailure => {
            let status = b.ins().iconst(types::I64, super::abi::STATUS_INTEGRITY as i64);
            b.ins().return_(&[status]);
        }
        NativeMatchTerminator::Complete => {
            return Err(NativeCompileError::InvalidPlan {
                node: None,
                reason: "completion must use its typed publication boundary",
            });
        }
    }

    Ok(())
}

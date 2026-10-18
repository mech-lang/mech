//! Canonical scalar CFG definition shared by JIT and Object emission.
use super::abi::{NO_NODE, NativeEntryFrame as Frame, STATUS_ABI};
use super::compiler::{
    admit, backend, compiled_stack_bytes, frame_load, guard, host_supported, limit, offset,
    set_origin, signature,
};
use super::control_compiler::{emit_scalar_action, emit_scalar_terminator};
use super::iteration_abi::{ENTRY, Entry, NativeIterationDescriptor, VERSION};
use super::iteration_runtime::{NativeIterationExecutionError, NativeIterationExecutionReport};
use super::{NativeCompileError, NativeCompileLimits, NativeExecutionLimits};
use cranelift_codegen::{
    ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types},
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationArtifactPlan, NativeIterationTerminator,
    NativeMatchAction, NativeMatchTerminator, NativeMatchValue, NativeScalarKind,
    NativeScalarValue,
};

pub struct JitIterationArtifact {
    _module: cranelift_jit::JITModule,
    entry: Entry,
    descriptor: NativeIterationDescriptor,
}
impl JitIterationArtifact {
    pub fn compile(plan: &NativeIterationArtifactPlan) -> Result<Self, NativeCompileError> {
        Self::compile_with_limits(plan, NativeCompileLimits::default())
    }
    pub fn compile_with_limits(
        plan: &NativeIterationArtifactPlan,
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
    pub fn descriptor(&self) -> &NativeIterationDescriptor {
        &self.descriptor
    }
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeIterationExecutionReport, NativeIterationExecutionError> {
        super::iteration_runtime::execute(
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
pub struct NativeIterationObject {
    bytes: Vec<u8>,
    descriptor: NativeIterationDescriptor,
    identity: super::iteration_identity::NativeIterationObjectIdentity,
}
impl NativeIterationObject {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn descriptor(&self) -> &NativeIterationDescriptor {
        &self.descriptor
    }
    pub fn identity_bytes(&self) -> &[u8; 176] {
        self.identity.bytes()
    }
    pub fn identity_symbol(&self) -> &str {
        self.identity.symbol()
    }
    pub fn identity(&self) -> &super::iteration_identity::NativeIterationObjectIdentity {
        &self.identity
    }
    pub fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "schema":"mech.canonical-scalar-iteration-object.v1","iteration_abi_version":VERSION,
            "typed_ir_sha256":super::identity::hex(self.descriptor.typed_ir_sha256()),
            "identity_symbol":self.identity.symbol(),"stack_bytes":self.descriptor.stack_bytes(),"code_bytes":self.descriptor.code_bytes(),
            "linked":false,"executed":false,"timing_samples":0,
        })
    }
}
pub fn emit_iteration_object(
    plan: &NativeIterationArtifactPlan,
) -> Result<NativeIterationObject, NativeCompileError> {
    emit_iteration_object_with_limits(plan, NativeCompileLimits::default())
}
pub fn emit_iteration_object_with_limits(
    plan: &NativeIterationArtifactPlan,
    limits: NativeCompileLimits,
) -> Result<NativeIterationObject, NativeCompileError> {
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
        ObjectBuilder::new(
            isa,
            "mech_canonical_scalar_iteration",
            default_libcall_names(),
        )
        .map_err(backend)?,
    );
    let (_, descriptor) = define(plan, &mut module, limits)?;
    let identity =
        super::iteration_identity::NativeIterationObjectIdentity::from_descriptor(&descriptor)
            .map_err(backend)?;
    let id = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(identity.bytes().to_vec().into_boxed_slice());
    data.set_align(8);
    module.define_data(id, &data).map_err(backend)?;
    let bytes = module.finish().emit().map_err(backend)?;
    Ok(NativeIterationObject {
        bytes,
        descriptor,
        identity,
    })
}

fn successors(term: NativeIterationTerminator) -> [Option<u32>; 2] {
    match term {
        NativeIterationTerminator::Scalar(NativeMatchTerminator::Jump(next)) => [Some(next), None],
        NativeIterationTerminator::Scalar(
            NativeMatchTerminator::Branch { yes, no, .. }
            | NativeMatchTerminator::LiteralBranch { yes, no, .. },
        ) => [Some(yes), Some(no)],
        NativeIterationTerminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        _ => [None, None],
    }
}
pub(super) fn validate(plan: &NativeIterationArtifactPlan) -> Result<(), NativeCompileError> {
    let bad = || NativeCompileError::InvalidPlan {
        node: None,
        reason: "invalid iteration scalar signature",
    };
    plan.validate_current_visits()
        .map_err(|reason| NativeCompileError::InvalidPlan { node: None, reason })?;
    let ty = |value: NativeMatchValue| plan.values().get(value.get() as usize).ok_or_else(bad);
    for block in plan.blocks() {
        if matches!(block.action(), NativeIterationAction::LibraryCall { .. }) {
            return Err(NativeCompileError::InvalidPlan {
                node: Some(block.origin().node()),
                reason: "managed call requires native managed transport",
            });
        }
        if let NativeIterationAction::Scalar(NativeMatchAction::Primitive {
            op,
            operands,
            output,
            ..
        }) = block.action()
        {
            let (args, result) = signature(*op);
            if operands.len() != args.len() || ty(*output)?.kind() != result {
                return Err(bad());
            }
            for (&value, &kind) in operands.iter().zip(args) {
                if ty(value)?.kind() != kind {
                    return Err(bad());
                }
            }
        }
    }
    Ok(())
}
fn load(
    b: &mut FunctionBuilder<'_>,
    locals: Value,
    value: NativeMatchValue,
) -> Result<Value, NativeCompileError> {
    super::control_compiler::load(b, locals, value)
}

fn define<M: Module>(
    plan: &NativeIterationArtifactPlan,
    module: &mut M,
    limits: NativeCompileLimits,
) -> Result<(FuncId, NativeIterationDescriptor), NativeCompileError> {
    let (function, stack, code) = define_cfg(
        super::iteration_code_plan::CodePlan::Scalar(plan),
        ENTRY,
        VERSION,
        module,
        limits,
    )?;
    Ok((
        function,
        NativeIterationDescriptor::from_plan(plan, stack, code).map_err(backend)?,
    ))
}

pub(super) fn define_cfg<M: Module>(
    plan: super::iteration_code_plan::CodePlan<'_>,
    entry_symbol: &str,
    version: u64,
    module: &mut M,
    limits: NativeCompileLimits,
) -> Result<(FuncId, u64, u64), NativeCompileError> {
    host_supported()?;
    if module.target_config().pointer_type() != types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    limit(
        "Iteration blocks",
        plan.blocks().len() as u64,
        limits.max_nodes,
    )?;
    limit(
        "Iteration inputs",
        plan.input_count() as u64,
        limits.max_inputs,
    )?;
    limit(
        "Iteration outputs",
        plan.output_count() as u64,
        limits.max_outputs,
    )?;
    limit(
        "Iteration scalar values",
        plan.value_count() as u64,
        limits.max_nodes.saturating_mul(8),
    )?;
    let local_count = plan
        .value_count()
        .checked_add(plan.iterations().len())
        .ok_or_else(|| backend("iteration local count overflow"))?;
    offset(local_count)?;
    offset(plan.input_count())?;
    offset(plan.output_count())?;
    plan.validate()?;
    let mut signature_entry = module.make_signature();
    signature_entry.params.push(AbiParam::new(types::I64));
    signature_entry.returns.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(entry_symbol, Linkage::Export, &signature_entry)
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
            (core::mem::offset_of!(Frame, version), version),
            (
                core::mem::offset_of!(Frame, input_count),
                plan.input_count() as u64,
            ),
            (
                core::mem::offset_of!(Frame, local_count),
                local_count as u64,
            ),
            (
                core::mem::offset_of!(Frame, output_count),
                plan.output_count() as u64,
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
            (inputs, plan.input_count()),
            (locals, local_count),
            (outputs, plan.output_count()),
            (user_context, 1),
            (callback, 1),
        ] {
            if count > 0 {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(&mut b, valid, error, STATUS_ABI);
            }
        }
        let hooks = super::program_hooks::Hooks::load(
            plan.is_program(),
            &mut b,
            module,
            frame,
            user_context,
            error,
        )?;
        for index in 0..plan.input_count() {
            let value = plan.input(index);
            if plan.kind(value).is_none() {
                continue;
            }
            let word = b
                .ins()
                .load(types::I64, MemFlags::new(), inputs, offset(index)?);
            if plan.kind(value) == Some(NativeScalarKind::Bool) {
                let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, word, 1);
                guard(&mut b, valid, error, STATUS_ABI);
            }
            b.ins()
                .store(MemFlags::new(), word, locals, offset(value.get() as usize)?);
        }
        for index in 0..plan.constant_count() {
            let (slot, word) = plan.constant(index)?;
            if let Some(word) = word {
                let value = b.ins().iconst(types::I64, word as i64);
                b.ins()
                    .store(MemFlags::new(), value, locals, offset(slot.get() as usize)?);
            }
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
            match block.action() {
                NativeIterationAction::LibraryCall {
                    operands, output, ..
                } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing mixed call hooks"))?;
                    let mut args = [b.ins().iconst(types::I64, 0); 2];
                    if operands.len() > 2 {
                        return Err(backend("unsupported library arity"));
                    }
                    for (index, &operand) in operands.iter().enumerate() {
                        if plan.kind(operand).is_some() {
                            args[index] = load(&mut b, locals, operand)?;
                        }
                    }
                    hooks.call(&mut b, index, 0, args, error);
                    if plan.kind(*output).is_some() {
                        let value = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(output.get() as usize)?,
                        );
                    }
                }
                NativeIterationAction::Scalar(action) => {
                    if let NativeMatchAction::Copy { output, .. } = action
                        && plan.kind(*output).is_none()
                    {
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("missing managed move hook"))?;
                        let zero = b.ins().iconst(types::I64, 0);
                        hooks.call(&mut b, index, 1, [zero; 2], error);
                    } else {
                        emit_scalar_action(&mut b, locals, action, error)?;
                    }
                }
                NativeIterationAction::Initialize { iteration } => {
                    let iteration = &plan.iterations()[*iteration as usize];
                    if plan.kind(iteration.carrier()).is_some() {
                        let value = load(&mut b, locals, iteration.seed())?;
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(iteration.carrier().get() as usize)?,
                        );
                    } else {
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("missing managed carrier hook"))?;
                        let zero = b.ins().iconst(types::I64, 0);
                        hooks.call(&mut b, index, 1, [zero; 2], error);
                    }
                    let count = b.ins().iconst(types::I64, iteration.count() as i64);
                    let slot = plan.value_count()
                        + match block.action() {
                            NativeIterationAction::Initialize { iteration } => *iteration as usize,
                            _ => unreachable!(),
                        };
                    b.ins().store(MemFlags::new(), count, locals, offset(slot)?);
                }
                NativeIterationAction::Latch { iteration } => {
                    let id = *iteration as usize;
                    let iteration = &plan.iterations()[id];
                    let counter = b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        locals,
                        offset(plan.value_count() + id)?,
                    );
                    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, counter, 0);
                    guard(&mut b, nonzero, error, STATUS_ABI);
                    if plan.kind(iteration.carrier()).is_some() {
                        let next = load(&mut b, locals, iteration.next())?;
                        b.ins().store(
                            MemFlags::new(),
                            next,
                            locals,
                            offset(iteration.carrier().get() as usize)?,
                        );
                    } else {
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("missing managed latch hook"))?;
                        let zero = b.ins().iconst(types::I64, 0);
                        hooks.call(&mut b, index, 1, [zero; 2], error);
                    }
                    let remaining = b.ins().iadd_imm(counter, -1);
                    b.ins().store(
                        MemFlags::new(),
                        remaining,
                        locals,
                        offset(plan.value_count() + id)?,
                    );
                }
            }
            match block.terminator() {
                NativeIterationTerminator::Scalar(terminator) => match plan {
                    super::iteration_code_plan::CodePlan::Scalar(p) => emit_scalar_terminator(
                        &mut b,
                        frame,
                        locals,
                        outputs,
                        terminator,
                        &blocks,
                        p.outputs(),
                        p.values(),
                    )?,
                    #[cfg(feature = "canonical-native-program")]
                    super::iteration_code_plan::CodePlan::Program(_) => {
                        if terminator == NativeMatchTerminator::Complete {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing typed completion hook"))?;
                            for out in 0..plan.output_count() {
                                let value = plan.output(out);
                                let first = if plan.kind(value).is_some() {
                                    load(&mut b, locals, value)?
                                } else {
                                    b.ins().iconst(types::I64, 0)
                                };
                                let zero = b.ins().iconst(types::I64, 0);
                                hooks.call(&mut b, index, 2 + out as u64, [first, zero], error);
                            }
                            set_origin(&mut b, frame, NO_NODE);
                            let status = b.ins().iconst(types::I64, super::abi::STATUS_OK as i64);
                            b.ins().return_(&[status]);
                        } else {
                            super::control_compiler::emit_branch_terminator(
                                &mut b,
                                locals,
                                terminator,
                                &blocks,
                                |v| plan.kind(v),
                            )?;
                        }
                    }
                },
                NativeIterationTerminator::Header {
                    iteration,
                    body,
                    exit,
                } => {
                    let remaining = b.ins().load(
                        types::I64,
                        MemFlags::new(),
                        locals,
                        offset(plan.value_count() + iteration as usize)?,
                    );
                    let nonzero = b.ins().icmp_imm(IntCC::NotEqual, remaining, 0);
                    b.ins().brif(
                        nonzero,
                        blocks[body as usize].expect("reachable body"),
                        &[],
                        blocks[exit as usize].expect("reachable exit"),
                        &[],
                    );
                }
            }
            let emitted = (b.func.dfg.num_insts() - before) as u64;
            let bound = match block.action() {
                NativeIterationAction::Scalar(NativeMatchAction::Primitive { op, .. }) => {
                    super::abi::primitive_work(*op)
                        .checked_add(128)
                        .ok_or_else(|| backend("iteration work overflow"))?
                }
                _ => {
                    if block.terminator()
                        == NativeIterationTerminator::Scalar(NativeMatchTerminator::Complete)
                    {
                        (plan.output_count() as u64)
                            .checked_mul(if plan.is_program() { 64 } else { 8 })
                            .and_then(|v| {
                                v.checked_add((plan.iterations().len() as u64).checked_mul(8)?)
                            })
                            .and_then(|v| v.checked_add(256))
                            .ok_or_else(|| backend("iteration completion overflow"))?
                    } else {
                        256
                    }
                }
            };
            if emitted.checked_add(128).is_none_or(|total| total > bound) {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(block.origin().node()),
                    reason: "Iteration visit exceeds conservative work template",
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
            reason: "Iteration compiled code missing",
        })?;
    let stack = compiled_stack_bytes(&compiled.buffer)?;
    let code = compiled.code_info().total_size as u64;
    limit("Iteration stack", stack, limits.max_stack_bytes)?;
    limit("Iteration code", code, limits.max_code_bytes)?;
    Ok((function, stack, code))
}

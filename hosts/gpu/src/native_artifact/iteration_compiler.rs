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
        NativeIterationTerminator::TailHeader { body, .. } => [Some(body), None],
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
        if matches!(
            block.action(),
            NativeIterationAction::LibraryCall { .. }
                | NativeIterationAction::Structural { .. }
                | NativeIterationAction::PrepareU64Range { .. }
                | NativeIterationAction::PrepareStringSpanSelector { .. }
                | NativeIterationAction::ProjectStringSpan { .. }
                | NativeIterationAction::ProjectStringMapEntry { .. }
                | NativeIterationAction::TextJoinFinish { .. }
                | NativeIterationAction::HistogramQueryStep { .. }
                | NativeIterationAction::HistogramFinish { .. }
        ) {
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
    #[cfg(feature = "canonical-native-program")]
    let aggregates = match plan {
        super::iteration_code_plan::CodePlan::Program(program) => program.scalar_aggregates(),
        _ => Default::default(),
    };
    #[cfg(feature = "canonical-native-program")]
    let aggregate_words = aggregates.words();
    #[cfg(not(feature = "canonical-native-program"))]
    let aggregate_words = 0;
    #[cfg(feature = "canonical-native-program")]
    let borrowed = match plan {
        super::iteration_code_plan::CodePlan::Program(program) => {
            super::program_borrowed_layout::Layout::derive(program).map_err(backend)?
        }
        _ => Default::default(),
    };
    #[cfg(feature = "canonical-native-program")]
    let borrowed_words = borrowed.words;
    #[cfg(not(feature = "canonical-native-program"))]
    let borrowed_words = 0;
    #[cfg(feature = "canonical-native-program")]
    let (closed, prefix) = match plan {
        super::iteration_code_plan::CodePlan::Program(program) => {
            // No completion block belongs to a closed Tail. Its completion
            // envelope is irrelevant to the body bound, not a second work policy.
            let mut visits = super::iteration_abi::control_visits(
                program.blocks(),
                program.iterations().iter().map(Into::into),
                0,
            )
            .map_err(backend)?;
            super::program_selected_join_layout::Layout::derive(program)
                .map_err(backend)?
                .fund_visit_checks(&mut visits)
                .map_err(backend)?;
            let prefix = super::program_prefix_layout::derive(program, &borrowed, &visits)
                .map_err(backend)?;
            for (site, visit) in visits.iter_mut().enumerate() {
                visit.work = visit
                    .work
                    .checked_add(borrowed.extra_work(site).map_err(backend)?)
                    .and_then(|work| {
                        work.checked_add(super::program_prefix_layout::extra_work(
                            prefix.as_ref(),
                            site,
                        ))
                    })
                    .ok_or_else(|| backend("Tail visit cost overflow"))?;
            }
            let owner_prices = program.map_owner_words().map_err(backend)?;
            for (site, visit) in visits.iter_mut().enumerate() {
                visit.work = visit.work.checked_add(super::program_map_owner_layout::extra_work(&owner_prices, site).map_err(backend)?)
                    .ok_or_else(|| backend("map owner visit work overflow"))?;
            }
            (
                super::program_tail_layout::Layout::derive(program, &borrowed, &visits)
                    .map_err(backend)?,
                prefix,
            )
        }
        _ => (Default::default(), None),
    };
    #[cfg(feature = "canonical-native-program")]
    let closed_words = closed.words;
    #[cfg(not(feature = "canonical-native-program"))]
    let closed_words = 0;
    let canonical_local_count = plan
        .value_count()
        .checked_add(plan.iteration_count())
        .and_then(|n| n.checked_add(aggregate_words))
        .and_then(|n| n.checked_add(borrowed_words))
        .and_then(|n| n.checked_add(closed_words))
        .ok_or_else(|| backend("iteration local count overflow"))?;
    #[cfg(feature = "canonical-native-program")]
    let owner = if let super::iteration_code_plan::CodePlan::Program(program) = plan {
        let layout = program.map_owner_words().map_err(backend)?
            .after_frame_words(canonical_local_count).map_err(backend)?;
        (!layout.regions().is_empty()).then_some(layout)
    } else { None };
    #[cfg(feature = "canonical-native-program")]
    let local_count = owner.as_ref().map_or(Ok(canonical_local_count),
        super::program_map_owner_layout::local_words).map_err(backend)?;
    #[cfg(not(feature = "canonical-native-program"))]
    let local_count = canonical_local_count;
    offset(local_count)?;
    offset(plan.input_count())?;
    offset(plan.output_count())?;
    plan.validate()?;
    #[cfg(feature = "canonical-native-program")]
    let window_functions = if let super::iteration_code_plan::CodePlan::Program(program) = plan {
        program
            .window_regions()
            .into_iter()
            .map(|region| {
                let (function, _, stack, code) =
                    super::program_compiler::window_codegen::define(module, &region)?;
                Ok((region, function, stack, code))
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?
    } else {
        Vec::new()
    };
    #[cfg(feature = "canonical-native-program")]
    let map_functions = if let super::iteration_code_plan::CodePlan::Program(program) = plan {
        program
            .string_map_regions()
            .into_iter()
            .map(|region| {
                let (function, stack, code) =
                    super::program_compiler::string_map_codegen::define(module, &region)?;
                Ok((region, function, stack, code))
            })
            .collect::<Result<Vec<_>, NativeCompileError>>()?
    } else {
        Vec::new()
    };
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
        #[cfg(feature = "canonical-native-program")]
        let program_phase = if plan.is_program() {
            let phase = frame_load(
                &mut b,
                frame,
                core::mem::offset_of!(super::program_abi::Frame, phase),
            );
            let valid = b.ins().icmp_imm(
                IntCC::UnsignedLessThanOrEqual,
                phase,
                if prefix.is_some() {
                    super::program_abi::PHASE_CAPTURE_TAIL
                } else {
                    super::program_abi::PHASE_TURN
                } as i64,
            );
            guard(&mut b, valid, error, STATUS_ABI);
            Some(phase)
        } else {
            None
        };
        #[cfg(not(feature = "canonical-native-program"))]
        let program_phase: Option<cranelift_codegen::ir::Value> = None;
        #[cfg(feature = "canonical-native-program")]
        let prefix_facts = prefix.as_ref().map(|_| {
            super::program_compiler::prefix_codegen::EntryFacts::load(
                &mut b,
                frame,
                program_phase.expect("managed prefix phase"),
            )
        });
        let resume_block = if let Some(phase) = program_phase {
            let initialize = b.create_block();
            let resume = b.create_block();
            let is_resume = b.ins().icmp_imm(IntCC::NotEqual, phase, 0);
            b.ins().brif(is_resume, resume, &[], initialize, &[]);
            b.switch_to_block(initialize);
            Some(resume)
        } else {
            None
        };
        for index in 0..plan.input_count() {
            // Managed sessions import all fresh inputs directly into their
            // certified scalar lanes AFTER activation/capture. ABI3 retains
            // its original initialization below.
            if plan.is_program() {
                continue;
            }
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
        #[cfg(feature = "canonical-native-program")]
        super::program_compiler::scalar_aggregates::constants(&mut b, locals, &aggregates)?;
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
        if let Some(resume) = resume_block {
            b.switch_to_block(resume);
            let boundary = plan
                .activation_boundary()
                .ok_or_else(|| backend("managed plan has no activation checkpoint"))?;
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix {
                let snapshot = b.create_block();
                let dispatch = b.create_block();
                let phase = program_phase.expect("managed resume phase");
                let capture_tail = b.create_block();
                let scan_or_snapshot = b.create_block();
                let is_capture_tail = b.ins().icmp_imm(
                    IntCC::Equal,
                    phase,
                    super::program_abi::PHASE_CAPTURE_TAIL as i64,
                );
                b.ins()
                    .brif(is_capture_tail, capture_tail, &[], scan_or_snapshot, &[]);
                b.switch_to_block(capture_tail);
                let target = frame_load(
                    &mut b,
                    frame,
                    core::mem::offset_of!(super::program_abi::Frame, resume_site),
                );
                let at_exit = b
                    .ins()
                    .icmp_imm(IntCC::Equal, target, i64::from(region.tail.exit()));
                guard(&mut b, at_exit, error, STATUS_ABI);
                let facts = prefix_facts.expect("managed prefix entry facts");
                guard(&mut b, facts.final_input, error, STATUS_ABI);
                b.ins().jump(
                    blocks[region.tail.exit() as usize].expect("reachable scan exit"),
                    &[],
                );
                b.switch_to_block(scan_or_snapshot);
                b.ins().brif(facts.is_prefix, dispatch, &[], snapshot, &[]);
                b.switch_to_block(dispatch);
                let target = frame_load(
                    &mut b,
                    frame,
                    core::mem::offset_of!(super::program_abi::Frame, resume_site),
                );
                let at_boundary = b.ins().icmp_imm(IntCC::Equal, target, i64::from(boundary));
                let at_header =
                    b.ins()
                        .icmp_imm(IntCC::Equal, target, i64::from(region.tail.header()));
                let valid = b.ins().bor(at_boundary, at_header);
                guard(&mut b, valid, error, STATUS_ABI);
                let reopen = b.create_block();
                b.ins().brif(at_header, reopen, &[], snapshot, &[]);
                b.switch_to_block(reopen);
                super::program_compiler::prefix_codegen::resume(
                    &mut b,
                    locals,
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
                b.ins().jump(
                    blocks[region.tail.header() as usize].expect("reachable prefix header"),
                    &[],
                );
                b.switch_to_block(snapshot);
            }
            b.ins().jump(
                blocks[boundary as usize].expect("reachable checkpoint"),
                &[],
            );
        }
        let body_error = b.create_block();
        b.append_block_param(body_error, types::I64);
        b.set_cold_block(body_error);
        for (index, block) in plan.blocks().iter().enumerate() {
            let error = body_error;
            if !reachable[index] {
                continue;
            }
            b.switch_to_block(blocks[index].expect("reachable block"));
            let before = b.func.dfg.num_insts();
            set_origin(&mut b, frame, index as u64);
            if plan.activation_boundary() == Some(index as u32) {
                let phase =
                    program_phase.ok_or_else(|| backend("checkpoint without managed phase"))?;
                let checkpoint = b.create_block();
                let turn = b.create_block();
                let fresh = b.ins().icmp_imm(IntCC::Equal, phase, 0);
                b.ins().brif(fresh, checkpoint, &[], turn, &[]);
                b.switch_to_block(checkpoint);
                let status = b.ins().iconst(types::I64, 5);
                b.ins().return_(&[status]);
                b.switch_to_block(turn);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix
                && region.tail.header() as usize == index
            {
                super::program_compiler::prefix_codegen::gate(
                    &mut b,
                    locals,
                    prefix_facts.expect("managed prefix entry facts"),
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
            }
            #[cfg(feature = "canonical-native-program")]
            let admission_join = if let Some(hooks) = &hooks {
                super::program_compiler::tail_codegen::admission(
                    &mut b, locals, &closed, index, hooks, error,
                )?
            } else {
                None
            };
            #[cfg(feature = "canonical-native-program")]
            if let Some(region) = &prefix
                && region.tail.exit() as usize == index
            {
                super::program_compiler::prefix_codegen::scanned(
                    &mut b,
                    locals,
                    prefix_facts.expect("managed prefix entry facts"),
                    region,
                    &borrowed,
                    &closed,
                    hooks.as_ref().expect("managed hooks"),
                    error,
                )?;
            }
            admit(
                &mut b,
                callback_signature,
                callback,
                user_context,
                index,
                error,
            );
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = admission_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let NativeIterationTerminator::Header {
                iteration, exit, ..
            } = block.terminator()
                && let Some((region, function, _, _)) = window_functions
                    .iter()
                    .find(|(region, ..)| region.iteration() == iteration)
            {
                let function = module.declare_func_in_func(*function, b.func);
                hooks
                    .as_ref()
                    .ok_or_else(|| backend("compiled window has no owner hooks"))?
                    .window_region(
                        &mut b,
                        index,
                        region,
                        function,
                        locals,
                        plan.value_count() + iteration as usize,
                        blocks[exit as usize].expect("reachable window exit"),
                        error,
                    )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let NativeIterationTerminator::Header {
                iteration, exit, ..
            } = block.terminator()
                && let Some((region, function, _, _)) = map_functions
                    .iter()
                    .find(|(region, ..)| region.iteration() == iteration)
            {
                let function = module.declare_func_in_func(*function, b.func);
                hooks
                    .as_ref()
                    .ok_or_else(|| backend("compiled String map has no owner hooks"))?
                    .string_map_region(
                        &mut b,
                        index,
                        region,
                        function,
                        locals,
                        plan.value_count() + iteration as usize,
                        blocks[exit as usize].expect("reachable map exit"),
                        error,
                    )?;
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(hooks) = &hooks {
                super::program_compiler::borrowed_codegen::prepare(
                    &mut b, locals, &borrowed, index, hooks, error,
                )?;
            }
            #[cfg(feature = "canonical-native-program")]
            let borrowed_join = super::program_compiler::borrowed_codegen::emit(
                &mut b,
                locals,
                &borrowed,
                &closed,
                index,
                hooks.as_ref(),
                error,
            )?;
            #[cfg(feature = "canonical-native-program")]
            let (owner_join, owner_handled) = super::program_compiler::map_owner_codegen::emit(
                &mut b, locals, owner.as_ref(), index, hooks.as_ref(), plan, error,
            )?;
            #[cfg(not(feature = "canonical-native-program"))]
            let owner_handled = false;
            #[cfg(feature = "canonical-native-program")]
            let aggregate = super::program_compiler::scalar_aggregates::emit(
                &mut b,
                locals,
                &aggregates,
                index,
                error,
            )?;
            #[cfg(not(feature = "canonical-native-program"))]
            let aggregate = false;
            match block.action() {
                _ if owner_handled => {}
                _ if aggregate
                    && !matches!(
                        block.action(),
                        NativeIterationAction::Initialize { .. }
                            | NativeIterationAction::Latch { .. }
                    ) => {}
                NativeIterationAction::PrepareStringSpanSelector { collection } => {
                    #[cfg(feature = "canonical-native-program")]
                    {
                        let super::iteration_code_plan::CodePlan::Program(program) = plan else {
                            return Err(backend("String span requires managed program transport"));
                        };
                        let projection = program.collections()[*collection as usize]
                            .string_span_projection()
                            .ok_or_else(|| backend("String span lacks selected proof"))?;
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("String span lacks managed hooks"))?;
                        let first = load(&mut b, locals, projection.operands()[0])?;
                        let second = load(&mut b, locals, projection.operands()[1])?;
                        hooks.collection(&mut b, index, [first, second], error)?;
                    }
                    #[cfg(not(feature = "canonical-native-program"))]
                    {
                        let _ = collection;
                        return Err(backend("String span requires managed program transport"));
                    }
                }
                NativeIterationAction::PrepareU64Range { iteration } => {
                    let mech_engine::native_artifact::NativeControlLoopRef::Collection(generator) =
                        plan.iteration(*iteration as usize)
                    else {
                        return Err(backend("range preparation outside collection loop"));
                    };
                    let mech_engine::native_artifact::NativeCollectionSource::U64Range(cursor) =
                        generator.source()
                    else {
                        return Err(backend("range preparation lacks selected cursor"));
                    };
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing range preparation hooks"))?;
                    let first = load(&mut b, locals, cursor.operands()[0])?;
                    let second = load(&mut b, locals, cursor.operands()[1])?;
                    hooks.collection(&mut b, index, [first, second], error)?;
                    // No Matrix result word/handle is written or fabricated.
                }
                NativeIterationAction::Structural { output, .. } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("structural observation without managed hooks"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    hooks.call(&mut b, index, 0xffff_ffff_ffff_fff0, [zero; 3], error);
                    if plan.kind(*output).is_some() {
                        let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            word,
                            locals,
                            offset(output.get() as usize)?,
                        );
                    }
                }
                NativeIterationAction::LibraryCall {
                    operands, output, ..
                } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing mixed call hooks"))?;
                    hooks.library(
                        &mut b,
                        index,
                        operands,
                        locals,
                        |operand| plan.kind(operand).is_some(),
                        error,
                    )?;
                    if plan.kind(*output).is_some() {
                        #[cfg(feature = "canonical-native-program")]
                        let count_join = prefix_facts.and_then(|facts| {
                            super::program_compiler::prefix_codegen::count_store(
                                &mut b,
                                facts,
                                prefix.as_ref(),
                                index,
                            )
                        });
                        let value = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(output.get() as usize)?,
                        );
                        #[cfg(feature = "canonical-native-program")]
                        if let Some(joined) = count_join {
                            b.ins().jump(joined, &[]);
                            b.switch_to_block(joined);
                        }
                    }
                }
                NativeIterationAction::Scalar(action) => {
                    #[cfg(feature = "canonical-native-program")]
                    let count_action = if let Some(facts) = prefix_facts {
                        super::program_compiler::prefix_codegen::count_action(
                            &mut b,
                            locals,
                            facts,
                            prefix.as_ref(),
                            index,
                            action,
                            error,
                        )?
                    } else {
                        false
                    };
                    #[cfg(not(feature = "canonical-native-program"))]
                    let count_action = false;
                    if count_action {
                        // The selected EOF-dependent action was emitted above.
                    } else if let NativeMatchAction::Copy { output, .. } = action
                        && plan.kind(*output).is_none()
                    {
                        let hooks = hooks
                            .as_ref()
                            .ok_or_else(|| backend("missing managed move hook"))?;
                        let zero = b.ins().iconst(types::I64, 0);
                        hooks.call(&mut b, index, 1, [zero; 3], error);
                    } else {
                        emit_scalar_action(&mut b, locals, action, error)?;
                    }
                }
                NativeIterationAction::Initialize { iteration } => {
                    let count = match plan.iteration(*iteration as usize) {
                        mech_engine::native_artifact::NativeControlLoopRef::Iteration(
                            iteration,
                        ) => {
                            if aggregate {
                                // The fixed-width carrier words were copied above.
                            } else if plan.kind(iteration.carrier()).is_some() {
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
                                hooks.call(&mut b, index, 1, [zero; 3], error);
                            }
                            b.ins().iconst(types::I64, iteration.count() as i64)
                        }
                        mech_engine::native_artifact::NativeControlLoopRef::Collection(_) => {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing collection initializer hook"))?;
                            let zero = b.ins().iconst(types::I64, 0);
                            hooks.collection(&mut b, index, [zero; 2], error)?;
                            b.ins().load(types::I64, MemFlags::new(), hooks.result, 0)
                        }
                        mech_engine::native_artifact::NativeControlLoopRef::Tail(_) => {
                            let zero = b.ins().iconst(types::I64, 0);
                            if !aggregate {
                                hooks
                                    .as_ref()
                                    .ok_or_else(|| backend("missing tail initializer hook"))?
                                    .call(&mut b, index, 1, [zero; 3], error);
                            }
                            zero
                        }
                    };
                    let slot = plan.value_count() + *iteration as usize;
                    b.ins().store(MemFlags::new(), count, locals, offset(slot)?);
                }
                NativeIterationAction::Latch { iteration } => {
                    if matches!(
                        plan.iteration(*iteration as usize),
                        mech_engine::native_artifact::NativeControlLoopRef::Tail(_)
                    ) {
                        if !aggregate {
                            let hooks = hooks
                                .as_ref()
                                .ok_or_else(|| backend("missing tail latch hook"))?;
                            let zero = b.ins().iconst(types::I64, 0);
                            hooks.call(&mut b, index, 1, [zero; 3], error);
                        }
                    } else {
                        let id = *iteration as usize;
                        let counter = b.ins().load(
                            types::I64,
                            MemFlags::new(),
                            locals,
                            offset(plan.value_count() + id)?,
                        );
                        let nonzero = b.ins().icmp_imm(IntCC::NotEqual, counter, 0);
                        guard(&mut b, nonzero, error, STATUS_ABI);
                        if let mech_engine::native_artifact::NativeControlLoopRef::Iteration(
                            iteration,
                        ) = plan.iteration(id)
                        {
                            if aggregate {
                                // Copy was emitted above; counted-loop progress is unchanged.
                            } else if plan.kind(iteration.carrier()).is_some() {
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
                                hooks.call(&mut b, index, 1, [zero; 3], error);
                            }
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
                NativeIterationAction::CollectionElement { iteration } => {
                    let mech_engine::native_artifact::NativeControlLoopRef::Collection(data) =
                        plan.iteration(*iteration as usize)
                    else {
                        return Err(backend("element hook outside collection loop"));
                    };
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing collection element hook"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    hooks.collection(&mut b, index, [zero; 2], error)?;
                    if let Some(output) = data.element().filter(|&value| plan.kind(value).is_some())
                    {
                        let value = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                        b.ins().store(
                            MemFlags::new(),
                            value,
                            locals,
                            offset(output.get() as usize)?,
                        );
                    }
                }
                NativeIterationAction::CollectionBegin { .. }
                | NativeIterationAction::ProjectStringSpan { .. }
                | NativeIterationAction::ProjectStringMapEntry { .. }
                | NativeIterationAction::TextJoinFinish { .. }
                | NativeIterationAction::HistogramQueryStep { .. }
                | NativeIterationAction::HistogramFinish { .. }
                | NativeIterationAction::CollectionFinish { .. }
                | NativeIterationAction::CollectionPush { .. } => {
                    let hooks = hooks
                        .as_ref()
                        .ok_or_else(|| backend("missing collection builder hook"))?;
                    let zero = b.ins().iconst(types::I64, 0);
                    let value = match block.action() {
                        NativeIterationAction::CollectionPush { value, .. }
                            if plan.kind(*value).is_some() =>
                        {
                            load(&mut b, locals, *value)?
                        }
                        _ => zero,
                    };
                    hooks.collection(&mut b, index, [value, zero], error)?;
                    #[cfg(feature = "canonical-native-program")]
                    if let NativeIterationAction::HistogramQueryStep { collection, step } =
                        block.action()
                    {
                        let super::iteration_code_plan::CodePlan::Program(program) = plan else {
                            return Err(backend(
                                "histogram query requires managed program transport",
                            ));
                        };
                        let mech_engine::native_artifact::NativeCollectionResult::GuardedHistogramQuery(sink) =
                            program.collections()[*collection as usize].result() else {
                            return Err(backend("histogram query compiler lacks selected proof"));
                        };
                        if let Some(output) = sink.steps()[*step as usize].output()
                            && program.values()[output.get() as usize]
                                .scalar_kind()
                                .is_some()
                        {
                            let word = b.ins().load(types::I64, MemFlags::new(), hooks.result, 0);
                            b.ins().store(
                                MemFlags::new(),
                                word,
                                locals,
                                offset(output.get() as usize)?,
                            );
                        }
                    }
                }
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = owner_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(joined) = borrowed_join {
                b.ins().jump(joined, &[]);
                b.switch_to_block(joined);
            }
            #[cfg(feature = "canonical-native-program")]
            if let Some(hooks) = &hooks {
                super::program_compiler::tail_codegen::prepare(
                    &mut b, locals, &closed, index, hooks, error,
                )?;
            }
            match block.terminator() {
                NativeIterationTerminator::TailHeader { body, .. } => {
                    b.ins()
                        .jump(blocks[body as usize].expect("reachable tail body"), &[]);
                }
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
                                hooks.output(&mut b, index, out, first, error)?;
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
                                v.checked_add((plan.iteration_count() as u64).checked_mul(8)?)
                            })
                            .and_then(|v| v.checked_add(256))
                            .ok_or_else(|| backend("iteration completion overflow"))?
                    } else {
                        256
                    }
                }
            };
            let bound = bound
                .checked_add(
                    super::iteration_abi::helper_operand_work(block.action()).map_err(backend)?,
                )
                .ok_or_else(|| backend("iteration operand work overflow"))?;
            #[cfg(feature = "canonical-native-program")]
            let bound = bound
                .checked_add(borrowed.extra_work(index).map_err(backend)?)
                .and_then(|n| n.checked_add(closed.extra_work(index)))
                .and_then(|n| {
                    n.checked_add(super::program_prefix_layout::extra_work(
                        prefix.as_ref(),
                        index,
                    ))
                })
                .ok_or_else(|| backend("borrowed String instruction work overflow"))?;
            if emitted.checked_add(128).is_none_or(|total| total > bound) {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(block.origin().node()),
                    reason: "Iteration visit exceeds conservative work template",
                });
            }
        }
        b.switch_to_block(body_error);
        let status = b.block_params(body_error)[0];
        #[cfg(feature = "canonical-native-program")]
        if let Some(hooks) = &hooks {
            let site = frame_load(&mut b, frame, core::mem::offset_of!(Frame, error_node));
            super::program_compiler::tail_codegen::abort(
                &mut b, locals, &closed, site, hooks, error,
            )?;
        }
        b.ins().jump(error, &[status.into()]);
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
    #[cfg(feature = "canonical-native-program")]
    let (stack, code) = (
        stack
            .checked_add(
                window_functions
                    .iter()
                    .map(|(_, _, stack, _)| *stack)
                    .chain(map_functions.iter().map(|(_, _, stack, _)| *stack))
                    .max()
                    .unwrap_or(0),
            )
            .ok_or_else(|| backend("compiled region stack overflow"))?,
        window_functions
            .iter()
            .map(|(_, _, _, code)| *code)
            .chain(map_functions.iter().map(|(_, _, _, code)| *code))
            .try_fold(code, |total, code| total.checked_add(code))
            .ok_or_else(|| backend("compiled region code overflow"))?,
    );
    limit("Iteration stack", stack, limits.max_stack_bytes)?;
    limit("Iteration code", code, limits.max_code_bytes)?;
    Ok((function, stack, code))
}

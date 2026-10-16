use super::abi::{
    ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NO_NODE, NativeAbiDescriptor, NativeEntryFrame,
    RawEntry, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::runtime::{NativeExecutionError, NativeExecutionLimits, NativeExecutionReport};
use cranelift_codegen::{
    ir::{AbiParam, Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types},
    isa::unwind::UnwindInst,
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::{ConstantId, InputId, ManagedMemoryBudget, NodeId};
use mech_engine::native_artifact::{
    NativeArtifactPlan, NativeScalarKind, NativeScalarOp, NativeScalarValue, NativeValueRef,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub struct NativeCompileLimits {
    pub max_nodes: u64,
    pub max_inputs: u64,
    pub max_outputs: u64,
    pub max_code_bytes: u64,
    pub max_stack_bytes: u64,
}
impl Default for NativeCompileLimits {
    fn default() -> Self {
        Self {
            max_nodes: 4096,
            max_inputs: 4096,
            max_outputs: 4096,
            max_code_bytes: 16 * 1024 * 1024,
            max_stack_bytes: 64 * 1024,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeCompileError {
    UnsupportedHost,
    Limit {
        dimension: &'static str,
        required: u64,
        limit: u64,
    },
    InvalidPlan {
        node: Option<NodeId>,
        reason: &'static str,
    },
    StackMetadata {
        reason: &'static str,
    },
    Backend(String),
}
impl std::fmt::Display for NativeCompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native compilation: {self:?}")
    }
}
impl std::error::Error for NativeCompileError {}
fn backend(error: impl std::fmt::Display) -> NativeCompileError {
    NativeCompileError::Backend(error.to_string())
}
fn limit(dimension: &'static str, required: u64, maximum: u64) -> Result<(), NativeCompileError> {
    if required > maximum {
        Err(NativeCompileError::Limit {
            dimension,
            required,
            limit: maximum,
        })
    } else {
        Ok(())
    }
}
fn host_supported() -> Result<(), NativeCompileError> {
    if !cfg!(all(
        target_pointer_width = "64",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )) {
        return Err(NativeCompileError::UnsupportedHost);
    }
    Ok(())
}

/// Owns the private entry pointer and its code lifetime. Construction requires
/// the opaque, witness-derived canonical plan; no source-text API exists.
pub struct JitArtifact {
    _module: cranelift_jit::JITModule,
    entry: RawEntry,
    descriptor: NativeAbiDescriptor,
}
impl JitArtifact {
    pub fn compile(plan: &NativeArtifactPlan) -> Result<Self, NativeCompileError> {
        Self::compile_with_limits(plan, NativeCompileLimits::default())
    }
    pub fn compile_with_limits(
        plan: &NativeArtifactPlan,
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
        // SAFETY: define emits this one private repr(C) ABI; module owns its
        // code and remains alive through every safe wrapper invocation.
        let entry = unsafe {
            std::mem::transmute::<*const u8, RawEntry>(module.get_finalized_function(function))
        };
        Ok(Self {
            _module: module,
            entry,
            descriptor,
        })
    }
    pub fn descriptor(&self) -> &NativeAbiDescriptor {
        &self.descriptor
    }
    /// Computes into private admitted transport storage. Caller outputs change
    /// only after all checked operations, admissions and output validation pass.
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeExecutionReport, NativeExecutionError> {
        super::runtime::execute(
            &self.descriptor,
            self.entry,
            inputs,
            outputs,
            limits,
            budget,
        )
    }
}

/// Object bytes and same-definition metadata only. This does not assert that
/// an application was linked or executed, and exposes no raw entry pointer.
pub struct NativeObject {
    bytes: Vec<u8>,
    descriptor: NativeAbiDescriptor,
}
impl NativeObject {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn descriptor(&self) -> &NativeAbiDescriptor {
        &self.descriptor
    }
    pub fn metadata(&self) -> serde_json::Value {
        let ty = |ty: &mech_engine::native_artifact::NativeScalarType| {
            serde_json::json!({
                "kind": format!("{:?}", ty.kind()),
                "schema_key": ty.schema_key().as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>(),
            })
        };
        serde_json::json!({
            "schema": "mech.canonical-native-object.v1",
            "abi_version": ABI_VERSION, "entry_symbol": NATIVE_ARTIFACT_ENTRY_SYMBOL,
            "program_revision": self.descriptor.revision().as_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>(),
            "inputs": self.descriptor.inputs().iter().map(ty).collect::<Vec<_>>(),
            "outputs": self.descriptor.outputs().iter().map(ty).collect::<Vec<_>>(),
            "ordered_nodes": self.descriptor.lets().iter().map(|(node, op)| serde_json::json!({"node":node.get(), "op":format!("{op:?}")})).collect::<Vec<_>>(),
            "generated_entry_stack_bytes": self.descriptor.stack_bytes(),
            "code_bytes": self.descriptor.code_bytes(), "object_bytes": self.bytes.len(),
            "linked": false, "executed": false, "timing_samples": 0,
        })
    }
}
pub fn emit_object(plan: &NativeArtifactPlan) -> Result<NativeObject, NativeCompileError> {
    emit_object_with_limits(plan, NativeCompileLimits::default())
}
pub fn emit_object_with_limits(
    plan: &NativeArtifactPlan,
    limits: NativeCompileLimits,
) -> Result<NativeObject, NativeCompileError> {
    host_supported()?;
    let mut flags = settings::builder();
    for (name, value) in [
        ("opt_level", "none"),
        ("is_pic", "true"),
        ("preserve_frame_pointers", "true"),
        ("unwind_info", "true"),
    ] {
        flags.set(name, value).map_err(backend)?;
    }
    let isa = cranelift_native::builder()
        .map_err(backend)?
        .finish(settings::Flags::new(flags))
        .map_err(backend)?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "mech_canonical_scalar", default_libcall_names())
            .map_err(backend)?,
    );
    let (_, descriptor) = define(plan, &mut module, limits)?;
    let bytes = module.finish().emit().map_err(backend)?;
    Ok(NativeObject { bytes, descriptor })
}

struct Sources {
    inputs: BTreeMap<InputId, (usize, NativeScalarKind)>,
    constants: BTreeMap<ConstantId, NativeScalarValue>,
    locals: BTreeMap<NodeId, (usize, NativeScalarKind)>,
}
impl Sources {
    fn kind(&self, value: NativeValueRef) -> Option<NativeScalarKind> {
        match value {
            NativeValueRef::Input(id) => self.inputs.get(&id).map(|v| v.1),
            NativeValueRef::Local(id) => self.locals.get(&id).map(|v| v.1),
            NativeValueRef::Constant(id) => self.constants.get(&id).map(|v| match v {
                NativeScalarValue::Bool(_) => NativeScalarKind::Bool,
                NativeScalarValue::U64(_) => NativeScalarKind::U64,
            }),
        }
    }
    fn load(
        &self,
        b: &mut FunctionBuilder<'_>,
        value: NativeValueRef,
        inputs: Value,
        locals: Value,
    ) -> Result<Value, NativeCompileError> {
        let (base, index) = match value {
            NativeValueRef::Input(id) => (
                inputs,
                self.inputs
                    .get(&id)
                    .ok_or(NativeCompileError::InvalidPlan {
                        node: None,
                        reason: "unknown input",
                    })?
                    .0,
            ),
            NativeValueRef::Local(id) => (
                locals,
                self.locals
                    .get(&id)
                    .ok_or(NativeCompileError::InvalidPlan {
                        node: Some(id),
                        reason: "local used before definition",
                    })?
                    .0,
            ),
            NativeValueRef::Constant(id) => {
                let word = match self
                    .constants
                    .get(&id)
                    .ok_or(NativeCompileError::InvalidPlan {
                        node: None,
                        reason: "unknown constant",
                    })? {
                    NativeScalarValue::Bool(value) => u64::from(*value),
                    NativeScalarValue::U64(value) => *value,
                };
                return Ok(b.ins().iconst(types::I64, word as i64));
            }
        };
        Ok(b.ins()
            .load(types::I64, MemFlags::new(), base, offset(index)?))
    }
}
fn offset(index: usize) -> Result<i32, NativeCompileError> {
    index
        .checked_mul(8)
        .and_then(|v| i32::try_from(v).ok())
        .ok_or(NativeCompileError::InvalidPlan {
            node: None,
            reason: "transport offset exceeds ABI",
        })
}
fn guard(b: &mut FunctionBuilder<'_>, valid: Value, error: Block, status: u64) {
    let good = b.create_block();
    let status = b.ins().iconst(types::I64, status as i64);
    b.ins().brif(valid, good, &[], error, &[status.into()]);
    b.switch_to_block(good);
}
fn frame_load(b: &mut FunctionBuilder<'_>, frame: Value, offset: usize) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), frame, offset as i32)
}
fn set_origin(b: &mut FunctionBuilder<'_>, frame: Value, node: u64) {
    let word = b.ins().iconst(types::I64, node as i64);
    b.ins().store(
        MemFlags::new(),
        word,
        frame,
        core::mem::offset_of!(NativeEntryFrame, error_node) as i32,
    );
}
fn admit(
    b: &mut FunctionBuilder<'_>,
    sig: cranelift_codegen::ir::SigRef,
    callback: Value,
    context: Value,
    ordinal: usize,
    error: Block,
) {
    let ordinal = b.ins().iconst(types::I64, ordinal as i64);
    let call = b.ins().call_indirect(sig, callback, &[context, ordinal]);
    let status = b.inst_results(call)[0];
    let valid = b.ins().icmp_imm(IntCC::Equal, status, STATUS_OK as i64);
    guard(b, valid, error, STATUS_ADMISSION);
}
fn signature(op: NativeScalarOp) -> (&'static [NativeScalarKind], NativeScalarKind) {
    use NativeScalarKind::{Bool, U64};
    use NativeScalarOp::*;
    match op {
        U64Add | U64Sub | U64Mul | U64Div | U64Rem => (&[U64, U64], U64),
        U64Eq | U64Ne | U64Lt | U64Le | U64Gt | U64Ge => (&[U64, U64], Bool),
        BoolNot => (&[Bool], Bool),
        BoolAnd | BoolOr | BoolXor | BoolEq | BoolNe => (&[Bool, Bool], Bool),
    }
}
fn primitive(
    b: &mut FunctionBuilder<'_>,
    op: NativeScalarOp,
    args: &[Value],
    error: Block,
) -> Value {
    use NativeScalarOp::*;
    let a = args[0];
    if op == BoolNot {
        return b.ins().bxor_imm(a, 1);
    }
    let c = args[1];
    match op {
        U64Add | U64Sub | U64Mul => {
            let (value, overflow) = match op {
                U64Add => b.ins().uadd_overflow(a, c),
                U64Sub => b.ins().usub_overflow(a, c),
                _ => b.ins().umul_overflow(a, c),
            };
            let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(b, valid, error, STATUS_ARITHMETIC);
            value
        }
        U64Div | U64Rem => {
            let nonzero = b.ins().icmp_imm(IntCC::NotEqual, c, 0);
            guard(b, nonzero, error, STATUS_ARITHMETIC);
            if op == U64Div {
                b.ins().udiv(a, c)
            } else {
                b.ins().urem(a, c)
            }
        }
        BoolAnd => b.ins().band(a, c),
        BoolOr => b.ins().bor(a, c),
        BoolXor => b.ins().bxor(a, c),
        U64Eq | U64Ne | U64Lt | U64Le | U64Gt | U64Ge | BoolEq | BoolNe => {
            let cc = match op {
                U64Eq | BoolEq => IntCC::Equal,
                U64Ne | BoolNe => IntCC::NotEqual,
                U64Lt => IntCC::UnsignedLessThan,
                U64Le => IntCC::UnsignedLessThanOrEqual,
                U64Gt => IntCC::UnsignedGreaterThan,
                _ => IntCC::UnsignedGreaterThanOrEqual,
            };
            let value = b.ins().icmp(cc, a, c);
            b.ins().uextend(types::I64, value)
        }
        BoolNot => unreachable!(),
    }
}

/// One definition pass for both Module implementations. It emits no resident
/// executor calls, name-based primitives, expression memoization or value fold.
fn define<M: Module>(
    plan: &NativeArtifactPlan,
    module: &mut M,
    limits: NativeCompileLimits,
) -> Result<(FuncId, NativeAbiDescriptor), NativeCompileError> {
    host_supported()?;
    if module.target_config().pointer_type() != types::I64 {
        return Err(NativeCompileError::UnsupportedHost);
    }
    limit("nodes", plan.lets().len() as u64, limits.max_nodes)?;
    limit("inputs", plan.inputs().len() as u64, limits.max_inputs)?;
    limit("outputs", plan.outputs().len() as u64, limits.max_outputs)?;
    offset(plan.lets().len())?;
    offset(plan.inputs().len())?;
    offset(plan.outputs().len())?;
    let mut sources = Sources {
        inputs: BTreeMap::new(),
        constants: BTreeMap::new(),
        locals: BTreeMap::new(),
    };
    for (index, input) in plan.inputs().iter().enumerate() {
        sources
            .inputs
            .insert(input.input(), (index, input.ty().kind()));
    }
    for value in plan.constants() {
        sources.constants.insert(value.constant(), value.value());
    }
    let mut sig = module.make_signature();
    sig.params.push(AbiParam::new(types::I64));
    sig.returns.push(AbiParam::new(types::I64));
    let entry_id = module
        .declare_function(NATIVE_ARTIFACT_ENTRY_SYMBOL, Linkage::Export, &sig)
        .map_err(backend)?;
    let mut callback_sig = module.make_signature();
    callback_sig.params = vec![AbiParam::new(types::I64); 2];
    callback_sig.returns.push(AbiParam::new(types::I64));
    let mut ctx = module.make_context();
    ctx.func.signature = sig;
    let mut fbctx = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
        let first = b.create_block();
        b.append_block_params_for_function_params(first);
        b.switch_to_block(first);
        let frame = b.block_params(first)[0];
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        b.set_cold_block(error);
        let nonnull = b.ins().icmp_imm(IntCC::NotEqual, frame, 0);
        guard(&mut b, nonnull, error, STATUS_ABI);
        set_origin(&mut b, frame, NO_NODE);
        for (field, expected) in [
            (
                core::mem::offset_of!(NativeEntryFrame, version),
                ABI_VERSION,
            ),
            (
                core::mem::offset_of!(NativeEntryFrame, input_count),
                plan.inputs().len() as u64,
            ),
            (
                core::mem::offset_of!(NativeEntryFrame, local_count),
                plan.lets().len() as u64,
            ),
            (
                core::mem::offset_of!(NativeEntryFrame, output_count),
                plan.outputs().len() as u64,
            ),
        ] {
            let actual = frame_load(&mut b, frame, field);
            let valid = b.ins().icmp_imm(IntCC::Equal, actual, expected as i64);
            guard(&mut b, valid, error, STATUS_ABI);
        }
        let inputs = frame_load(
            &mut b,
            frame,
            core::mem::offset_of!(NativeEntryFrame, inputs),
        );
        let locals = frame_load(
            &mut b,
            frame,
            core::mem::offset_of!(NativeEntryFrame, locals),
        );
        let outputs = frame_load(
            &mut b,
            frame,
            core::mem::offset_of!(NativeEntryFrame, outputs),
        );
        let context = frame_load(
            &mut b,
            frame,
            core::mem::offset_of!(NativeEntryFrame, context),
        );
        let callback = frame_load(
            &mut b,
            frame,
            core::mem::offset_of!(NativeEntryFrame, admit_next),
        );
        for (pointer, count) in [
            (inputs, plan.inputs().len()),
            (locals, plan.lets().len()),
            (outputs, plan.outputs().len()),
            (context, 1),
            (callback, 1),
        ] {
            if count != 0 {
                let valid = b.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
                guard(&mut b, valid, error, STATUS_ABI);
            }
        }
        for (index, input) in plan.inputs().iter().enumerate() {
            if input.ty().kind() == NativeScalarKind::Bool {
                let word = b
                    .ins()
                    .load(types::I64, MemFlags::new(), inputs, offset(index)?);
                let valid = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, word, 1);
                guard(&mut b, valid, error, STATUS_ABI);
            }
        }
        let callback_sig = b.import_signature(callback_sig);
        for (index, node) in plan.lets().iter().enumerate() {
            let instructions_before = b.func.dfg.num_insts();
            let (kinds, kind) = signature(node.op());
            if node.operands().len() != kinds.len()
                || kind != node.ty().kind()
                || node
                    .operands()
                    .iter()
                    .zip(kinds)
                    .any(|(&v, &kind)| sources.kind(v) != Some(kind))
            {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(node.node()),
                    reason: "typed operation signature mismatch",
                });
            }
            set_origin(&mut b, frame, u64::from(node.node().get()));
            admit(&mut b, callback_sig, callback, context, index, error);
            let args = node
                .operands()
                .iter()
                .map(|v| sources.load(&mut b, *v, inputs, locals))
                .collect::<Result<Vec<_>, _>>()?;
            let value = primitive(&mut b, node.op(), &args, error);
            b.ins()
                .store(MemFlags::new(), value, locals, offset(index)?);
            // This is a bound on explicit lowered logical actions, never
            // hardware instructions/cycles. Reserve64 additional actions for
            // the fixed, allocation-free admission callback and reject a
            // template that would exceed its separately declared work cost.
            let emitted = b
                .func
                .dfg
                .num_insts()
                .checked_sub(instructions_before)
                .ok_or(NativeCompileError::InvalidPlan {
                    node: Some(node.node()),
                    reason: "instruction inventory decreased during checked definition",
                })?;
            if (emitted as u64)
                .checked_add(64)
                .is_none_or(|bound| bound > super::abi::primitive_work(node.op()))
            {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(node.node()),
                    reason: "checked primitive exceeds its conservative work template",
                });
            }

            if sources
                .locals
                .insert(node.node(), (index, node.ty().kind()))
                .is_some()
            {
                return Err(NativeCompileError::InvalidPlan {
                    node: Some(node.node()),
                    reason: "duplicate local identity",
                });
            }
        }
        set_origin(&mut b, frame, NO_NODE);
        admit(
            &mut b,
            callback_sig,
            callback,
            context,
            plan.lets().len(),
            error,
        );
        // These are private staging stores. No fallible generated operation
        // follows them; the safe wrapper validates all bits before publication.
        for (index, output) in plan.outputs().iter().enumerate() {
            if sources.kind(output.value()) != Some(output.ty().kind()) {
                return Err(NativeCompileError::InvalidPlan {
                    node: None,
                    reason: "typed output mismatch",
                });
            }
            let value = sources.load(&mut b, output.value(), inputs, locals)?;
            b.ins()
                .store(MemFlags::new(), value, outputs, offset(index)?);
        }
        let success = b.ins().iconst(types::I64, STATUS_OK as i64);
        b.ins().return_(&[success]);
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(entry_id, &mut ctx)
        .map_err(backend)?;
    let compiled = ctx
        .compiled_code()
        .ok_or(NativeCompileError::StackMetadata {
            reason: "compiled code missing",
        })?;
    let stack_bytes = compiled_stack_bytes(&compiled.buffer)?;
    let code_bytes = u64::from(compiled.code_info().total_size);
    limit(
        "compiled entry stack bytes",
        stack_bytes,
        limits.max_stack_bytes,
    )?;
    limit("code bytes", code_bytes, limits.max_code_bytes)?;
    let descriptor = NativeAbiDescriptor::from_compiled(plan, stack_bytes, code_bytes);
    Ok((entry_id, descriptor))
}

fn compiled_stack_bytes(
    buffer: &cranelift_codegen::MachBufferFinalized<cranelift_codegen::Final>,
) -> Result<u64, NativeCompileError> {
    let layout = buffer
        .frame_layout()
        .ok_or(NativeCompileError::StackMetadata {
            reason: "fixed frame layout missing",
        })?;
    let mut pushes = 0;
    let mut frames = 0;
    let mut clobbers = None;
    let mut fixed_allocation = None;
    for (_, inst) in &buffer.unwind_info {
        match inst {
            UnwindInst::PushFrameRegs {
                offset_upward_to_caller_sp: 16,
            } if pushes == 0 && frames == 0 => pushes += 1,
            UnwindInst::DefineNewFrame {
                offset_upward_to_caller_sp: 16,
                offset_downward_to_clobbers,
            } if pushes == 1 && frames == 0 => {
                frames += 1;
                clobbers = Some(*offset_downward_to_clobbers);
            }
            // Pinned AArch64 reports its ordinary fixed area explicitly; the
            // field is already included in frame_to_fp_offset. No tail calls
            // or dynamic stack allocations are emitted by this definition.
            UnwindInst::StackAlloc { size }
                if cfg!(target_arch = "aarch64") && frames == 1 && fixed_allocation.is_none() =>
            {
                fixed_allocation = Some(*size);
            }
            UnwindInst::PushFrameRegs { .. }
            | UnwindInst::DefineNewFrame { .. }
            | UnwindInst::StackAlloc { .. } => {
                return Err(NativeCompileError::StackMetadata {
                    reason: "unsupported or repeated frame setup",
                });
            }
            UnwindInst::SaveReg { .. } if frames == 1 => {}
            UnwindInst::Aarch64SetPointerAuth { .. } if cfg!(target_arch = "aarch64") => {}
            _ => {
                return Err(NativeCompileError::StackMetadata {
                    reason: "unsupported unwind action",
                });
            }
        }
    }
    if pushes != 1 || frames != 1 {
        return Err(NativeCompileError::StackMetadata {
            reason: "exact preserved native frame setup unavailable",
        });
    }
    let clobbers = clobbers.ok_or(NativeCompileError::StackMetadata {
        reason: "clobber layout missing",
    })?;
    if cfg!(target_arch = "aarch64") {
        if clobbers.checked_add(fixed_allocation.unwrap_or(0)) != Some(layout.frame_to_fp_offset) {
            return Err(NativeCompileError::StackMetadata {
                reason: "fixed AArch64 allocation does not match complete frame",
            });
        }
    } else if fixed_allocation.is_some() || clobbers > layout.frame_to_fp_offset {
        return Err(NativeCompileError::StackMetadata {
            reason: "unsupported x86_64 fixed-frame metadata",
        });
    }
    // Pinned Cranelift frame_to_fp_offset includes clobbers, fixed/spill slots
    // and outgoing arguments (machinst/abi.rs::sp_to_fp_offset). The supported
    // preserved-FP prologue contributes exactly the separately verified16.
    Ok(u64::from(layout.frame_to_fp_offset) + 16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_void;
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    fn bool_jit() -> JitArtifact {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x434e554e), Revision(1), "!flag<bool>\n").unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
            .unwrap()
            .compile_artifact()
            .unwrap();
        let plan = mech_engine::native_artifact::lower_pure_native_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        JitArtifact::compile(&plan).unwrap()
    }
    unsafe extern "C" fn count_admission(context: *mut c_void, _: u64) -> u64 {
        // SAFETY: this private test retains the exact AtomicU64 through call.
        unsafe { &*context.cast::<AtomicU64>() }.fetch_add(1, Ordering::SeqCst);
        STATUS_OK
    }
    #[test]
    fn raw_private_abi_rejects_noncanonical_bool_and_bad_header_before_callbacks_or_stores() {
        let jit = bool_jit();
        let calls = AtomicU64::new(0);
        let mut locals = vec![0; jit.descriptor().lets().len()];
        let mut outputs = vec![0xbeef; jit.descriptor().outputs().len()];
        for word in [2, u64::MAX] {
            let input = [word];
            let mut frame = NativeEntryFrame {
                version: ABI_VERSION,
                input_count: 1,
                local_count: locals.len() as u64,
                output_count: outputs.len() as u64,
                inputs: input.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: outputs.as_mut_ptr(),
                context: (&calls as *const AtomicU64).cast_mut().cast(),
                admit_next: count_admission,
                error_node: NO_NODE,
            };
            // SAFETY: private test supplies exact buffers/counts; invalid Bool
            // bits are intentionally rejected by the generated ABI guard.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(outputs.iter().all(|v| *v == 0xbeef));
        }
        let input = [1];
        for bad_version in [true, false] {
            let mut frame = NativeEntryFrame {
                version: if bad_version {
                    ABI_VERSION + 1
                } else {
                    ABI_VERSION
                },
                input_count: if bad_version { 1 } else { 0 },
                local_count: locals.len() as u64,
                output_count: outputs.len() as u64,
                inputs: input.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: outputs.as_mut_ptr(),
                context: (&calls as *const AtomicU64).cast_mut().cast(),
                admit_next: count_admission,
                error_node: NO_NODE,
            };
            // SAFETY: allocations still cover the true signature exactly.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(outputs.iter().all(|v| *v == 0xbeef));
        }
    }
    static ENTRY_CALLS: AtomicU64 = AtomicU64::new(0);
    unsafe extern "C" fn never_admitted_entry(_: *mut NativeEntryFrame) -> u64 {
        ENTRY_CALLS.fetch_add(1, Ordering::SeqCst);
        STATUS_OK
    }
    #[test]
    fn denied_setup_never_invokes_an_entry_or_changes_outputs() {
        let jit = bool_jit();
        let mut output = vec![NativeScalarValue::U64(99); jit.descriptor().outputs().len()];
        let before = output.clone();
        ENTRY_CALLS.store(0, Ordering::SeqCst);
        // The real source-derived descriptor is retained. Only an internal
        // entry-counter fixture is substituted to prove setup denies first.
        let error = super::super::runtime::execute(
            jit.descriptor(),
            never_admitted_entry,
            &[NativeScalarValue::Bool(true)],
            &mut output,
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            NativeExecutionError::WorkLimit {
                phase: super::super::runtime::NativeExecutionPhase::Setup,
                ..
            }
        ));
        assert_eq!(ENTRY_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(output, before);
    }
    struct CompletionDenial {
        calls: u64,
        stop: u64,
    }
    unsafe extern "C" fn deny_completion(context: *mut c_void, ordinal: u64) -> u64 {
        // SAFETY: this private test retains the exact fixed context through call.
        let context = unsafe { &mut *context.cast::<CompletionDenial>() };
        context.calls += 1;
        if ordinal == context.stop {
            STATUS_ADMISSION
        } else {
            STATUS_OK
        }
    }
    #[test]
    fn generated_completion_admission_precedes_every_private_output_store() {
        let jit = bool_jit();
        let input = [1_u64];
        let mut locals = vec![77; jit.descriptor().lets().len()];
        let mut outputs = vec![0xbeef; jit.descriptor().outputs().len()];
        let mut denial = CompletionDenial {
            calls: 0,
            stop: jit.descriptor().lets().len() as u64,
        };
        let mut frame = NativeEntryFrame {
            version: ABI_VERSION,
            input_count: 1,
            local_count: locals.len() as u64,
            output_count: outputs.len() as u64,
            inputs: input.as_ptr(),
            locals: locals.as_mut_ptr(),
            outputs: outputs.as_mut_ptr(),
            context: (&mut denial as *mut CompletionDenial).cast(),
            admit_next: deny_completion,
            error_node: NO_NODE,
        };
        // SAFETY: this test supplies exact private buffers and finite callback;
        // only its completion phase intentionally denies admission.
        assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ADMISSION);
        assert_eq!(denial.calls, jit.descriptor().lets().len() as u64 + 1);
        assert_eq!(locals, [0]);
        assert!(outputs.iter().all(|word| *word == 0xbeef));
    }
}

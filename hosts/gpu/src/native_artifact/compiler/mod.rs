use crate::native_artifact::abi::{
    ABI_VERSION, NATIVE_ARTIFACT_ENTRY_SYMBOL, NO_NODE, NativeAbiDescriptor, NativeEntryFrame,
    RawEntry, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use crate::native_artifact::runtime::{
    NativeExecutionError, NativeExecutionLimits, NativeExecutionReport,
};
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlags, Value, types};
use cranelift_codegen::isa::unwind::UnwindInst;
use cranelift_codegen::settings;
use cranelift_codegen::settings::Configurable;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};
use mech_core::{ConstantId, InputId, ManagedMemoryBudget, NodeId};
use mech_engine::native_artifact::{
    NativeArtifactPlan, NativeScalarKind, NativeScalarOp, NativeScalarValue, NativeValueRef,
};
use sha2::Digest;
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
pub(in crate::native_artifact) fn backend(error: impl std::fmt::Display) -> NativeCompileError {
    NativeCompileError::Backend(error.to_string())
}
pub(in crate::native_artifact) fn limit(
    dimension: &'static str,
    required: u64,
    maximum: u64,
) -> Result<(), NativeCompileError> {
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
pub(in crate::native_artifact) fn host_supported() -> Result<(), NativeCompileError> {
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
                ("enable_nan_canonicalization", "false"),
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
        crate::native_artifact::runtime::execute(
            &self.descriptor,
            |frame| {
                // SAFETY: this module owns the code and its exact private ABI.
                unsafe { (self.entry)(frame) }
            },
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
    identity: crate::native_artifact::identity::NativeObjectIdentity,
}
impl NativeObject {
    pub fn identity(&self) -> &crate::native_artifact::identity::NativeObjectIdentity {
        &self.identity
    }
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
            "typed_ir_sha256": crate::native_artifact::identity::hex(self.descriptor.typed_ir_sha256()),
            "identity_symbol": self.identity.symbol(),
            "identity_sha256": crate::native_artifact::identity::hex(&sha2::Sha256::digest(self.identity.bytes())),
            "work_policy_version": crate::native_artifact::identity::NATIVE_WORK_POLICY_VERSION,
            "target_tag": crate::native_artifact::identity::host_target_tag(),
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
        ("enable_nan_canonicalization", "false"),
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
    let identity =
        crate::native_artifact::identity::NativeObjectIdentity::from_descriptor(&descriptor)
            .ok_or(NativeCompileError::UnsupportedHost)?;
    let data = module
        .declare_data(identity.symbol(), Linkage::Export, false, false)
        .map_err(backend)?;
    let mut description = DataDescription::new();
    description.define(identity.bytes().to_vec().into_boxed_slice());
    description.set_align(8);
    module.define_data(data, &description).map_err(backend)?;
    let bytes = module.finish().emit().map_err(backend)?;
    Ok(NativeObject {
        bytes,
        descriptor,
        identity,
    })
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
                NativeScalarValue::I64(_) => NativeScalarKind::I64,
                NativeScalarValue::F64(_) => NativeScalarKind::F64,
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
                    NativeScalarValue::I64(value) => *value as u64,
                    NativeScalarValue::F64(value) => value.bits(),
                };
                return Ok(b.ins().iconst(types::I64, word as i64));
            }
        };
        Ok(b.ins()
            .load(types::I64, MemFlags::new(), base, offset(index)?))
    }
}
pub(in crate::native_artifact) fn offset(index: usize) -> Result<i32, NativeCompileError> {
    index
        .checked_mul(8)
        .and_then(|v| i32::try_from(v).ok())
        .ok_or(NativeCompileError::InvalidPlan {
            node: None,
            reason: "transport offset exceeds ABI",
        })
}
pub(in crate::native_artifact) fn guard(
    b: &mut FunctionBuilder<'_>,
    valid: Value,
    error: Block,
    status: u64,
) {
    let good = b.create_block();
    let status = b.ins().iconst(types::I64, status as i64);
    b.ins().brif(valid, good, &[], error, &[status.into()]);
    b.switch_to_block(good);
}
pub(in crate::native_artifact) fn frame_load(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    offset: usize,
) -> Value {
    b.ins()
        .load(types::I64, MemFlags::new(), frame, offset as i32)
}
pub(in crate::native_artifact) fn set_origin(b: &mut FunctionBuilder<'_>, frame: Value, node: u64) {
    set_origin_with_flags(b, frame, node, MemFlags::new());
}
pub(in crate::native_artifact) fn set_origin_with_flags(
    b: &mut FunctionBuilder<'_>,
    frame: Value,
    node: u64,
    flags: MemFlags,
) {
    let word = b.ins().iconst(types::I64, node as i64);
    b.ins().store(
        flags,
        word,
        frame,
        core::mem::offset_of!(NativeEntryFrame, error_node) as i32,
    );
}
pub(in crate::native_artifact) fn admit(
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
pub(in crate::native_artifact) fn primitive(
    b: &mut FunctionBuilder<'_>,
    op: NativeScalarOp,
    args: &[Value],
    error: Block,
) -> Value {
    use mech_engine::native_artifact::NativeScalarOp::*;
    // Only genuine selected conversion witnesses reach these typed arms.
    // Guards precede machine conversions and result stores, and the caller
    // admits this complete checked template before loading its operand.
    match op {
        U64ToI64 => {
            let valid = b
                .ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, args[0], i64::MAX);
            guard(b, valid, error, STATUS_ARITHMETIC);
            return args[0];
        }
        I64ToU64 => {
            let valid = b
                .ins()
                .icmp_imm(IntCC::SignedGreaterThanOrEqual, args[0], 0);
            guard(b, valid, error, STATUS_ARITHMETIC);
            return args[0];
        }
        U64ToF64 | I64ToF64 => {
            let value = if op == U64ToF64 {
                b.ins().fcvt_from_uint(types::F64, args[0])
            } else {
                b.ins().fcvt_from_sint(types::F64, args[0])
            };
            return b.ins().bitcast(types::I64, MemFlags::new(), value);
        }
        F64ToU64 | F64ToI64 => {
            // The actual core executor rejects nonfinite before truncation.
            // Inspect raw exponent bits first, so even signaling NaN reaches
            // the checked error without executing a floating instruction.
            let exponent = b.ins().band_imm(args[0], 0x7ff0_0000_0000_0000_i64);
            let finite = b
                .ins()
                .icmp_imm(IntCC::NotEqual, exponent, 0x7ff0_0000_0000_0000_i64);
            guard(b, finite, error, STATUS_ARITHMETIC);
            let value = b.ins().bitcast(types::F64, MemFlags::new(), args[0]);
            let truncated = b.ins().trunc(value);
            let signed = op == F64ToI64;
            let lower = b
                .ins()
                .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(
                    if signed { 0xc3e0_0000_0000_0000 } else { 0 },
                ));
            let upper = b
                .ins()
                .f64const(cranelift_codegen::ir::immediates::Ieee64::with_bits(
                    if signed {
                        0x43e0_0000_0000_0000
                    } else {
                        0x43f0_0000_0000_0000
                    },
                ));
            // Range checks follow truncation: (-1,0) is valid unsigned zero.
            let above = b.ins().fcmp(FloatCC::GreaterThanOrEqual, truncated, lower);
            let below = b.ins().fcmp(FloatCC::LessThan, truncated, upper);
            let valid = b.ins().band(above, below);
            guard(b, valid, error, STATUS_ARITHMETIC);
            return if signed {
                b.ins().fcvt_to_sint(types::I64, truncated)
            } else {
                b.ins().fcvt_to_uint(types::I64, truncated)
            };
        }
        _ => {}
    }
    let a = args[0];
    if matches!(
        op,
        F64Add
            | F64Sub
            | F64Mul
            | F64Div
            | F64Neg
            | F64Sqrt
            | F64Eq
            | F64Ne
            | F64Lt
            | F64Le
            | F64Gt
            | F64Ge
    ) {
        // ABI transport remains raw words. Only the exact typed primitive
        // interprets them as F64; no arithmetic cast or NaN canonicalization.
        let left = b.ins().bitcast(types::F64, MemFlags::new(), a);
        let value = match op {
            F64Neg => b.ins().fneg(left),
            F64Sqrt => b.ins().sqrt(left),
            _ => {
                let right = b.ins().bitcast(types::F64, MemFlags::new(), args[1]);
                match op {
                    F64Add => b.ins().fadd(left, right),
                    F64Sub => b.ins().fsub(left, right),
                    F64Mul => b.ins().fmul(left, right),
                    F64Div => b.ins().fdiv(left, right),
                    _ => {
                        let cc = match op {
                            F64Eq => FloatCC::Equal,
                            // NotEqual includes unordered, matching Rust !=.
                            F64Ne => FloatCC::NotEqual,
                            F64Lt => FloatCC::LessThan,
                            F64Le => FloatCC::LessThanOrEqual,
                            F64Gt => FloatCC::GreaterThan,
                            F64Ge => FloatCC::GreaterThanOrEqual,
                            _ => unreachable!("typed floating primitive"),
                        };
                        let value = b.ins().fcmp(cc, left, right);
                        return b.ins().uextend(types::I64, value);
                    }
                }
            }
        };
        return b.ins().bitcast(types::I64, MemFlags::new(), value);
    }
    if op == BoolNot {
        return b.ins().bxor_imm(a, 1);
    }
    if op == I64Neg {
        let valid = b.ins().icmp_imm(IntCC::NotEqual, a, i64::MIN);
        guard(b, valid, error, STATUS_ARITHMETIC);
        return b.ins().ineg(a);
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
        I64Add | I64Sub | I64Mul => {
            let (value, overflow) = match op {
                I64Add => b.ins().sadd_overflow(a, c),
                I64Sub => b.ins().ssub_overflow(a, c),
                _ => b.ins().smul_overflow(a, c),
            };
            let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(b, valid, error, STATUS_ARITHMETIC);
            value
        }
        I64Div | I64Rem => {
            let nonzero = b.ins().icmp_imm(IntCC::NotEqual, c, 0);
            guard(b, nonzero, error, STATUS_ARITHMETIC);
            // Rust checked_div AND checked_rem reject MIN/-1. Guard before
            // either machine operation so the native entry cannot trap.
            let minimum = b.ins().icmp_imm(IntCC::Equal, a, i64::MIN);
            let negative_one = b.ins().icmp_imm(IntCC::Equal, c, -1);
            let overflow = b.ins().band(minimum, negative_one);
            let valid = b.ins().icmp_imm(IntCC::Equal, overflow, 0);
            guard(b, valid, error, STATUS_ARITHMETIC);
            if op == I64Div {
                b.ins().sdiv(a, c)
            } else {
                b.ins().srem(a, c)
            }
        }
        I64Eq | I64Ne | I64Lt | I64Le | I64Gt | I64Ge => {
            let cc = match op {
                I64Eq => IntCC::Equal,
                I64Ne => IntCC::NotEqual,
                I64Lt => IntCC::SignedLessThan,
                I64Le => IntCC::SignedLessThanOrEqual,
                I64Gt => IntCC::SignedGreaterThan,
                _ => IntCC::SignedGreaterThanOrEqual,
            };
            let value = b.ins().icmp(cc, a, c);
            b.ins().uextend(types::I64, value)
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
        U64ToI64 | I64ToU64 | U64ToF64 | I64ToF64 | F64ToU64 | F64ToI64 | BoolNot | I64Neg
        | F64Add | F64Sub | F64Mul | F64Div | F64Neg | F64Sqrt | F64Eq | F64Ne | F64Lt | F64Le
        | F64Gt | F64Ge => unreachable!(),
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
            let (kinds, kind) = node.op().signature();
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
                .is_none_or(|bound| bound > crate::native_artifact::abi::primitive_work(node.op()))
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

pub(in crate::native_artifact) fn compiled_stack_bytes(
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
#[path = "tests.rs"]
pub(in crate::native_artifact::compiler) mod tests;

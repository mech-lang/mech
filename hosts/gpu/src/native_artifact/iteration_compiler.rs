//! Canonical scalar CFG definition shared by JIT and Object emission.
use crate::native_artifact::codegen::backend as codegen_backend;
use crate::native_artifact::compiler::{backend, host_supported};
use crate::native_artifact::iteration_abi::{ENTRY, Entry, NativeIterationDescriptor, VERSION};
use crate::native_artifact::iteration_runtime::{
    NativeIterationExecutionError, NativeIterationExecutionReport,
};
use crate::native_artifact::{NativeCompileError, NativeCompileLimits, NativeExecutionLimits};
use cranelift_module::{DataDescription, FuncId, Linkage, Module};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{
    NativeIterationAction, NativeIterationArtifactPlan, NativeMatchAction, NativeMatchValue,
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
        let mut module = codegen_backend::jit(codegen_backend::CONTROL_FLAGS)?;
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
    #[cfg(test)]
    pub(super) fn test_checked_entry(&self) -> Entry { self.entry }
    pub fn evaluate_into(
        &self,
        inputs: &[NativeScalarValue],
        outputs: &mut [NativeScalarValue],
        limits: NativeExecutionLimits,
        budget: Option<&ManagedMemoryBudget>,
    ) -> Result<NativeIterationExecutionReport, NativeIterationExecutionError> {
        crate::native_artifact::iteration_runtime::execute(
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
    identity: crate::native_artifact::iteration_identity::NativeIterationObjectIdentity,
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
    pub fn identity(
        &self,
    ) -> &crate::native_artifact::iteration_identity::NativeIterationObjectIdentity {
        &self.identity
    }
    pub fn metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "schema":"mech.canonical-scalar-iteration-object.v1","iteration_abi_version":VERSION,
            "typed_ir_sha256":crate::native_artifact::identity::hex(self.descriptor.typed_ir_sha256()),
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
    let mut module = codegen_backend::object(
        "mech_canonical_scalar_iteration",
        codegen_backend::CONTROL_FLAGS,
    )?;
    let (_, descriptor) = define(plan, &mut module, limits)?;
    let identity =
        crate::native_artifact::iteration_identity::NativeIterationObjectIdentity::from_descriptor(
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
    Ok(NativeIterationObject {
        bytes,
        descriptor,
        identity,
    })
}

pub(in crate::native_artifact) fn validate(
    plan: &NativeIterationArtifactPlan,
) -> Result<(), NativeCompileError> {
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
    }
    Ok(())
}
fn define<M: Module>(
    plan: &NativeIterationArtifactPlan,
    module: &mut M,
    limits: NativeCompileLimits,
) -> Result<(FuncId, NativeIterationDescriptor), NativeCompileError> {
    let (function, stack, code, _) = crate::native_artifact::codegen::control::define_cfg(
        crate::native_artifact::codegen::control::plan::CodePlan::Scalar(plan),
        ENTRY,
        VERSION,
        module,
        limits,
        &crate::native_artifact::codegen::options::OptimizationOptions::default(),
    )?;
    Ok((
        function,
        NativeIterationDescriptor::from_plan(plan, stack, code).map_err(backend)?,
    ))
}

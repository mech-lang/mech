#[cfg(feature = "compiler")]
pub mod bytecode_compiler;
pub mod catalog;

use crate::{
    ExecutionTarget, FunctionInvocation, ImplementationMemoryClass, MResult, MechFunction,
    MechFunctionFactory, OperationContractDeclaration, ResolvedOperationDescriptor,
    RuntimeFunctionId, SpecializedFunction,
};

/// Builds a unit-test kernel through the same managed instance boundary used
/// by catalog specialization. Tests must not execute raw implementations now
/// that physical access is exclusively frame-scoped.
pub(crate) fn managed_factory_instance<F>(
    invocation: FunctionInvocation,
    operation: &'static str,
) -> MResult<SpecializedFunction>
where
    F: MechFunctionFactory,
{
    let implementation = F::new_invocation(invocation.clone())?;
    let contract = F::declared_operation_contract()
        .or_else(|| implementation.semantic_operation_contract())
        .expect("managed factory test requires a declared operation contract");
    managed_implementation_instance(
        implementation,
        invocation,
        operation,
        contract.clone(),
        F::implementation_memory_class(),
    )
}

pub(crate) fn managed_implementation_instance(
    implementation: Box<dyn MechFunction>,
    invocation: FunctionInvocation,
    operation: &'static str,
    contract: OperationContractDeclaration,
    memory: ImplementationMemoryClass,
) -> MResult<SpecializedFunction> {
    SpecializedFunction::syntax_directed(
        (implementation, invocation),
        ResolvedOperationDescriptor::from_name(operation, contract)?,
        RuntimeFunctionId::from_name(operation),
        ExecutionTarget::DirectRuntime,
        memory,
    )
}

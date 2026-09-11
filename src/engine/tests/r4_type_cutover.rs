#![cfg(feature = "full_compiler")]

use mech_core::{
    AccessMode, AliasPolicy, BoundCall, BoundCallOrigin, BoundImplementationId,
    BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode, ExternalInteraction, InputPortLayout,
    InputPortPolicy, OperationContractDeclaration, OutputConstruction, OutputPortPolicy,
    ResidentKernelError, ResidentKernelInputs, ResidentOperationKey, ResidentValueRef,
    ResolvedOperationDescriptor, ShapeRule, ValueCell,
};

struct Inputs([f64; 1]);

impl ResidentKernelInputs for Inputs {
    fn len(&self) -> usize {
        1
    }

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
        (index == 0).then_some(ResidentValueRef::F64(&self.0))
    }
}

fn copy_first(
    _kernel: &BoundResidentKernel,
    first: &[f64],
    output: &mut [f64],
) -> Result<bool, ResidentKernelError> {
    output.copy_from_slice(first);
    Ok(true)
}

#[test]
fn resident_kernel_retains_the_immutable_semantic_binding() {
    let descriptor = ValueCell::from_exact(1.0_f64)
        .unwrap()
        .resolved_descriptor()
        .unwrap();
    let operation = ResolvedOperationDescriptor::from_name(
        "test/resident-copy",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::SameAsInput { input: 0 },
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let resident_operation = ResidentOperationKey::new(
        vec!["test".to_string()].into_boxed_slice(),
        "resident-copy".to_string(),
    )
    .unwrap();
    let binding = BoundCall::artifact_operation(
        operation,
        vec![descriptor.clone()].into_boxed_slice(),
        vec![descriptor].into_boxed_slice(),
        resident_operation.clone(),
    )
    .unwrap();
    let kernel = BoundResidentKernel::new_f64_output_1(copy_first, Box::new([]))
        .with_bound_call(binding.clone());

    assert_eq!(kernel.bound_call(), Some(&binding));
    assert_eq!(binding.origin(), &BoundCallOrigin::ArtifactOperation);
    assert_eq!(
        binding.implementation(),
        &BoundImplementationId::Resident(resident_operation)
    );
    assert_eq!(binding.runtime_function(), None);
    let mut output = [0.0];
    kernel
        .execute_f64_output(&Inputs([1.0]), &mut output)
        .unwrap();
    assert_eq!(output, [1.0]);
}

use mech_core::{
    AccessMode, AliasPolicy, BoundResidentKernel, ChangeDetectionPolicy, DeliveryMode,
    ExternalInteraction, FunctionCatalogBuilder, MResult, OutputConstruction,
    ResidentKernelBindError, ResidentKernelBindRequest, ResidentKernelError, ResidentKernelInputs,
    ResidentShape, ResidentValueKind, ResidentValueMut, ResidentValueRef,
    ResolvedOperationContract, ShapeRule,
};

pub(crate) fn install(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    builder.insert_resident_factory(["runtime"], "ConcatSS<string>", bind_concat)?;
    Ok(())
}

fn bind_concat(
    request: &ResidentKernelBindRequest<'_>,
) -> Result<BoundResidentKernel, ResidentKernelBindError> {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return Err(ResidentKernelBindError::UnsupportedContract);
    };
    if contract.interaction != ExternalInteraction::Pure
        || contract.inputs.len() != 2
        || request.inputs.len() != 2
        || contract.outputs.len() != 1
        || contract
            .inputs
            .iter()
            .zip(request.inputs)
            .any(|(port, layout)| {
                port.schema != layout.schema_id
                    || port.access != AccessMode::Read
                    || port.delivery != DeliveryMode::Signal
                    || layout.kind != ResidentValueKind::String
                    || layout.shape != ResidentShape::SCALAR
            })
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    let output = &contract.outputs[0];
    if output.schema != request.output.schema_id
        || output.access != AccessMode::Write
        || output.delivery != DeliveryMode::Signal
        || output.construction
            != (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        || output.alias != AliasPolicy::NoAlias
        || output.change_detection != ChangeDetectionPolicy::ExactScalar
        || request.output.kind != ResidentValueKind::String
        || request.output.shape != ResidentShape::SCALAR
    {
        return Err(ResidentKernelBindError::UnsupportedContract);
    }
    Ok(BoundResidentKernel::new(concat, Box::new([])))
}

fn concat(
    _kernel: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let Some(ResidentValueRef::String([left])) = inputs.get(0) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let Some(ResidentValueRef::String([right])) = inputs.get(1) else {
        return Err(ResidentKernelError::InvalidInput);
    };
    let ResidentValueMut::String([target]) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut next = String::with_capacity(left.len() + right.len());
    next.push_str(left);
    next.push_str(right);
    let changed = *target != next;
    *target = next;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Inputs([String; 2]);

    impl ResidentKernelInputs for Inputs {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn get(&self, index: usize) -> Option<ResidentValueRef<'_>> {
            self.0
                .get(index)
                .map(core::slice::from_ref)
                .map(ResidentValueRef::String)
        }
    }

    #[test]
    fn scalar_concat_writes_the_normal_resident_output() {
        let kernel = BoundResidentKernel::new(concat, Box::new([]));
        let inputs = Inputs(["Hello, ".to_string(), "Ada".to_string()]);
        let mut output = [String::new()];

        assert!(
            kernel
                .execute(&inputs, ResidentValueMut::String(&mut output))
                .unwrap()
        );
        assert_eq!(output[0], "Hello, Ada");
    }
}

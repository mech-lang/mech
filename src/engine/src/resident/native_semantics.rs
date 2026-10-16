//! Private proof of a selected maintained scalar implementation.
//!
//! Public operation names, cost declarations, and generic core attachments do
//! not grant this authority. Only finished, exact maintained binder branches
//! attach CandidateWitness. The shared binder validates and replaces it with
//! a seal over its exact final BoundCall. Neither witness type is exported.
use std::sync::Arc;

use mech_core::{
    AccessMode, AliasPolicy, BoundCall, BoundCallOrigin, BoundResidentKernel,
    ChangeDetectionPolicy, DeliveryMode, ExecutionTarget, ExternalInteraction, IntegerWidth,
    OutputConstruction, ResidentKernelBindRequest, ResidentPortLayout, ResidentShape,
    ResidentValueKind, ResolvedOperationContract, SchemaBody, ShapeRule,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeScalarKind {
    Bool,
    U64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NativeScalarOp {
    U64Add,
    U64Sub,
    U64Mul,
    U64Div,
    U64Rem,
    U64Eq,
    U64Ne,
    U64Lt,
    U64Le,
    U64Gt,
    U64Ge,
    BoolAnd,
    BoolOr,
    BoolXor,
    BoolNot,
    BoolEq,
    BoolNe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct NativeScalarSemantics {
    op: NativeScalarOp,
}
impl NativeScalarSemantics {
    // The optional canonical-native adapter consumes this read-only view.
    #[cfg_attr(not(feature = "native-artifact"), allow(dead_code))]
    pub(crate) fn op(self) -> NativeScalarOp {
        self.op
    }
    pub(crate) fn inputs(&self) -> &'static [NativeScalarKind] {
        use NativeScalarKind::{Bool, U64};
        use NativeScalarOp::*;
        match self.op {
            BoolNot => &[Bool],
            BoolAnd | BoolOr | BoolXor | BoolEq | BoolNe => &[Bool, Bool],
            _ => &[U64, U64],
        }
    }
    pub(crate) fn output(self) -> NativeScalarKind {
        use NativeScalarOp::*;
        match self.op {
            U64Add | U64Sub | U64Mul | U64Div | U64Rem => NativeScalarKind::U64,
            _ => NativeScalarKind::Bool,
        }
    }
}

#[derive(Clone)]
struct CandidateWitness {
    semantics: NativeScalarSemantics,
    contract: ResolvedOperationContract,
    inputs: Box<[ResidentPortLayout]>,
    output: ResidentPortLayout,
    parameters: Box<[u64]>,
}

struct SealedWitness {
    semantics: NativeScalarSemantics,
    bound_call: BoundCall,
}

fn scalar_port(
    request: &ResidentKernelBindRequest<'_>,
    port: &ResidentPortLayout,
    kind: NativeScalarKind,
) -> bool {
    let Some(entry) = request.schemas.entry(port.schema_id) else {
        return false;
    };
    let expected = match kind {
        NativeScalarKind::Bool => SchemaBody::Bool,
        NativeScalarKind::U64 => SchemaBody::UnsignedInteger(IntegerWidth::W64),
    };
    entry.key() == port.schema_key
        && entry.schema().body() == &expected
        && entry.schema().dimension_parameters().is_empty()
        && port.shape == ResidentShape::SCALAR
        && port.shape_instance.parameter_values().is_empty()
        && port.resolved_selector.is_none()
        && port.kind
            == match kind {
                NativeScalarKind::Bool => ResidentValueKind::Bool,
                NativeScalarKind::U64 => ResidentValueKind::Snapshot,
            }
}

fn scalar_request(
    request: &ResidentKernelBindRequest<'_>,
    semantics: NativeScalarSemantics,
) -> bool {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return false;
    };
    let [output] = contract.outputs.as_ref() else {
        return false;
    };
    request.inputs.len() == semantics.inputs().len()
        && contract.inputs.len() == request.inputs.len()
        && contract.interaction == ExternalInteraction::Pure
        && contract
            .inputs
            .iter()
            .zip(request.inputs)
            .all(|(policy, layout)| {
                policy.schema == layout.schema_id
                    && policy.access == AccessMode::Read
                    && policy.delivery == DeliveryMode::Signal
            })
        && output.schema == request.output.schema_id
        && output.access == AccessMode::Write
        && output.delivery == DeliveryMode::Signal
        && output.construction
            == (OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            })
        && output.alias == AliasPolicy::NoAlias
        && output.change_detection == ChangeDetectionPolicy::ExactScalar
        && request
            .inputs
            .iter()
            .zip(semantics.inputs())
            .all(|(port, kind)| scalar_port(request, port, *kind))
        && scalar_port(request, &request.output, semantics.output())
}

/// Used only by exact successful maintained scalar binder branches, after
/// every execution metadata/cost builder is finished. No public engine entry
/// accepts an attachment or supplies semantic certification by name.
pub(super) fn certify(
    request: &ResidentKernelBindRequest<'_>,
    kernel: BoundResidentKernel,
    op: NativeScalarOp,
) -> BoundResidentKernel {
    let semantics = NativeScalarSemantics { op };
    if !scalar_request(request, semantics) {
        return kernel;
    }
    let candidate = CandidateWitness {
        semantics,
        contract: request.contract.clone(),
        inputs: request.inputs.into(),
        output: request.output.clone(),
        parameters: kernel.parameters().into(),
    };
    kernel.with_lowering_witness(Arc::new(candidate))
}

fn bound_call_matches(request: &ResidentKernelBindRequest<'_>, bound_call: &BoundCall) -> bool {
    let ResolvedOperationContract::Declared(_) = request.contract else {
        return false;
    };
    let Ok(expected) = mech_core::ResolvedOperationDescriptor::from_resolved_contract(
        bound_call.operation_descriptor().canonical_name.clone(),
        request.contract,
    ) else {
        return false;
    };
    bound_call.origin() == &BoundCallOrigin::ArtifactOperation
        && matches!(
            bound_call.target(),
            ExecutionTarget::ResidentCpu | ExecutionTarget::Native
        )
        && bound_call.operation_descriptor() == &expected
        && bound_call.inputs().len() == request.inputs.len()
        && bound_call.outputs().len() == 1
        && bound_call
            .inputs()
            .iter()
            .zip(request.inputs)
            .all(|(descriptor, port)| {
                descriptor.schema().key() == port.schema_key
                    && descriptor.shape() == &port.shape_instance
            })
        && bound_call.outputs()[0].schema().key() == request.output.schema_key
        && bound_call.outputs()[0].shape() == &request.output.shape_instance
}

/// The shared resident/native binder owns this final metadata attachment. A
/// custom or changed delegated kernel remains usable by the resident but
/// cannot obtain our private semantic seal.
pub(super) fn bind_call(
    kernel: BoundResidentKernel,
    request: &ResidentKernelBindRequest<'_>,
    bound_call: BoundCall,
) -> BoundResidentKernel {
    let candidate = kernel
        .lowering_witness::<CandidateWitness>()
        .filter(|candidate| {
            candidate.contract == *request.contract
                && candidate.inputs.as_ref() == request.inputs
                && candidate.output == request.output
                && candidate.parameters.as_ref() == kernel.parameters()
                && scalar_request(request, candidate.semantics)
                && bound_call_matches(request, &bound_call)
        })
        .cloned();
    let accounting = kernel.work_accounting();
    let kernel = kernel
        .with_bound_call(bound_call.clone())
        .with_work_accounting(accounting);
    if let Some(candidate) = candidate {
        kernel.with_lowering_witness(Arc::new(SealedWitness {
            semantics: candidate.semantics,
            bound_call,
        }))
    } else {
        kernel
    }
}

/// Returns only typed semantics after auditing the actual selected kernel's
/// private seal. It never returns a witness or executes a kernel/input read.
#[cfg_attr(not(feature = "native-artifact"), allow(dead_code))]
pub(crate) fn validated_scalar(kernel: &BoundResidentKernel) -> Option<NativeScalarSemantics> {
    let seal = kernel.lowering_witness::<SealedWitness>()?;
    (kernel.bound_call()? == &seal.bound_call).then_some(seal.semantics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DeclaredOperationContract, FunctionCatalog, FunctionCatalogBuilder,
        ImplementationMemoryClass, ResidentKernelError, ResidentKernelInputs,
        ResidentKernelWorkAccounting, ResidentOperationKey, ResidentSnapshotOutput,
        ResidentValueMut, ResolvedInputPort, ResolvedOperationDescriptor, ResolvedOutputPort,
        ResolvedValueDescriptor, SchemaDraft, SchemaId, SchemaTable, SchemaTableBuilder,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn catalog() -> FunctionCatalog {
        let mut builder = FunctionCatalogBuilder::new();
        crate::resident::numeric::install(&mut builder).unwrap();
        builder.build().unwrap()
    }

    struct Binding {
        schemas: SchemaTable,
        contract: ResolvedOperationContract,
        inputs: Box<[ResidentPortLayout]>,
        output: ResidentPortLayout,
    }
    impl Binding {
        fn scalar(op: NativeScalarOp) -> Self {
            let mut builder = SchemaTableBuilder::new();
            let handles = [
                SchemaBody::Bool,
                SchemaBody::UnsignedInteger(IntegerWidth::W64),
            ]
            .into_iter()
            .map(|body| {
                builder
                    .insert(
                        SchemaDraft {
                            dimension_parameters: Box::new([]),
                            body,
                        }
                        .finalize()
                        .unwrap(),
                    )
                    .unwrap()
            })
            .collect::<Vec<_>>();
            let build = builder.finish().unwrap();
            let ids = handles
                .into_iter()
                .map(|handle| build.resolve(handle).unwrap())
                .collect::<Vec<_>>();
            let schemas = build.table;
            let semantics = NativeScalarSemantics { op };
            let layout = |kind| {
                let schema_id = match kind {
                    NativeScalarKind::Bool => ids[0],
                    NativeScalarKind::U64 => ids[1],
                };
                ResidentPortLayout {
                    schema_id,
                    schema_key: schemas.entry(schema_id).unwrap().key(),
                    kind: match kind {
                        NativeScalarKind::Bool => ResidentValueKind::Bool,
                        NativeScalarKind::U64 => ResidentValueKind::Snapshot,
                    },
                    shape: ResidentShape::SCALAR,
                    shape_instance: schemas
                        .get(schema_id)
                        .unwrap()
                        .instantiate_shape(Box::new([]))
                        .unwrap(),
                    activation_fixed_shape: true,
                    resolved_selector: None,
                }
            };
            let inputs = semantics
                .inputs()
                .iter()
                .copied()
                .map(layout)
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let output = layout(semantics.output());
            let contract = ResolvedOperationContract::Declared(DeclaredOperationContract {
                inputs: inputs
                    .iter()
                    .map(|port| ResolvedInputPort {
                        schema: port.schema_id,
                        access: AccessMode::Read,
                        delivery: DeliveryMode::Signal,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                outputs: Box::new([ResolvedOutputPort {
                    schema: output.schema_id,
                    access: AccessMode::Write,
                    delivery: DeliveryMode::Signal,
                    construction: OutputConstruction::FullWrite {
                        shape: ShapeRule::Declared,
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::ExactScalar,
                }]),
                interaction: ExternalInteraction::Pure,
            });
            Self {
                schemas,
                contract,
                inputs,
                output,
            }
        }
        fn request(&self) -> ResidentKernelBindRequest<'_> {
            ResidentKernelBindRequest {
                contract: &self.contract,
                schemas: &self.schemas,
                inputs: &self.inputs,
                output: self.output.clone(),
            }
        }
        fn call(&self, module: &str, operation: &str) -> BoundCall {
            let descriptor = |port: &ResidentPortLayout| {
                ResolvedValueDescriptor::from_schema(
                    self.schemas.get(port.schema_id).unwrap().clone(),
                    port.shape_instance.clone(),
                )
                .unwrap()
            };
            BoundCall::artifact_operation(
                ResolvedOperationDescriptor::from_resolved_contract(
                    format!("{module}/{operation}"),
                    &self.contract,
                )
                .unwrap(),
                self.inputs
                    .iter()
                    .map(descriptor)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                Box::new([descriptor(&self.output)]),
                ResidentOperationKey::new(Box::new([module.into()]), operation.into()).unwrap(),
            )
            .unwrap()
        }
        fn bind(&self, catalog: &FunctionCatalog, module: &str, name: &str) -> BoundResidentKernel {
            let entry = catalog.resident_factory(&[module.into()], name).unwrap();
            (entry.factory)(&self.request()).unwrap()
        }
        fn snapshot_output(&self) -> ResidentSnapshotOutput {
            ResidentSnapshotOutput {
                schema: self.output.schema_id,
                schema_key: self.output.schema_key,
                shape: self.output.shape_instance.clone(),
                exact_cardinality: None,
                maximum_cardinality: None,
            }
        }
    }

    #[test]
    fn maintained_scalar_branches_require_final_binding_before_exposing_semantics() {
        use NativeScalarOp::*;
        let catalog = catalog();
        for (module, name, op) in [
            ("math", "add", U64Add),
            ("math", "sub", U64Sub),
            ("math", "mul", U64Mul),
            ("math", "div", U64Div),
            ("math", "mod", U64Rem),
            ("compare", "eq", U64Eq),
            ("compare", "neq", U64Ne),
            ("compare", "lt", U64Lt),
            ("compare", "lte", U64Le),
            ("compare", "gt", U64Gt),
            ("compare", "gte", U64Ge),
            ("logic", "and", BoolAnd),
            ("logic", "or", BoolOr),
            ("logic", "xor", BoolXor),
            ("logic", "not", BoolNot),
            ("compare", "eq", BoolEq),
            ("compare", "neq", BoolNe),
        ] {
            let binding = Binding::scalar(op);
            let kernel = binding.bind(&catalog, module, name);
            assert!(
                validated_scalar(&kernel).is_none(),
                "unsealed {module}/{name}"
            );
            let sealed = bind_call(
                kernel.clone(),
                &binding.request(),
                binding.call(module, name),
            );
            assert_eq!(validated_scalar(&sealed).map(|s| s.op()), Some(op));
            assert_eq!(validated_scalar(&sealed.clone()).map(|s| s.op()), Some(op));
            assert_eq!(sealed.parameters(), kernel.parameters());
        }
    }

    #[test]
    fn every_delegated_semantic_or_cost_change_loses_private_authority() {
        let binding = Binding::scalar(NativeScalarOp::U64Add);
        let kernel = binding.bind(&catalog(), "math", "add");
        let call = binding.call("math", "add");
        let changed = [
            kernel
                .clone()
                .with_work_accounting(kernel.work_accounting()),
            kernel
                .clone()
                .with_snapshot_output(binding.snapshot_output()),
            kernel
                .clone()
                .with_snapshot_schemas(binding.schemas.clone()),
            kernel.clone().with_bound_call(call.clone()),
            kernel
                .clone()
                .with_retained_state(Arc::new(AtomicUsize::new(0))),
        ];
        for changed in changed {
            assert!(
                validated_scalar(&bind_call(changed, &binding.request(), call.clone())).is_none()
            );
        }
        let sealed = bind_call(kernel, &binding.request(), call);
        assert_eq!(
            validated_scalar(&sealed).map(|s| s.op()),
            Some(NativeScalarOp::U64Add)
        );
        assert!(
            validated_scalar(
                &sealed
                    .clone()
                    .with_work_accounting(sealed.work_accounting())
            )
            .is_none()
        );
        assert!(validated_scalar(&sealed.clone().with_retained_state(Arc::new(0u64))).is_none());
        assert!(
            validated_scalar(
                &sealed
                    .clone()
                    .with_snapshot_output(binding.snapshot_output())
            )
            .is_none()
        );
        assert!(
            validated_scalar(
                &sealed
                    .clone()
                    .with_snapshot_schemas(binding.schemas.clone())
            )
            .is_none()
        );
        assert!(
            validated_scalar(&sealed.clone().with_bound_call(binding.call("math", "add")))
                .is_none()
        );
    }

    static EXECUTIONS: AtomicUsize = AtomicUsize::new(0);
    fn custom_executor(
        _: &BoundResidentKernel,
        _: &dyn ResidentKernelInputs,
        _: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        EXECUTIONS.fetch_add(1, Ordering::SeqCst);
        Err(ResidentKernelError::Arithmetic)
    }
    fn custom_factory(
        _: &ResidentKernelBindRequest<'_>,
    ) -> Result<BoundResidentKernel, mech_core::ResidentKernelBindError> {
        Ok(BoundResidentKernel::new(custom_executor, Box::new([]))
            .with_work_accounting(ResidentKernelWorkAccounting::RuntimeMetered)
            // Even the public typed semantic description is not a private seal.
            .with_lowering_witness(Arc::new(NativeScalarSemantics {
                op: NativeScalarOp::U64Add,
            })))
    }
    #[test]
    fn same_name_cost_claim_and_public_attachment_cannot_certify_replacement() {
        let binding = Binding::scalar(NativeScalarOp::U64Add);
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_resident_factory(
                ["math"],
                "add",
                ImplementationMemoryClass::NoAdditionalScratch,
                custom_factory,
            )
            .unwrap();
        let replacement = binding.bind(&builder.build().unwrap(), "math", "add");
        assert!(
            validated_scalar(&bind_call(
                replacement,
                &binding.request(),
                binding.call("math", "add")
            ))
            .is_none()
        );
        assert_eq!(EXECUTIONS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn genuine_candidate_is_bound_to_its_exact_request_not_just_schema_name() {
        let mut binding = Binding::scalar(NativeScalarOp::U64Add);
        let kernel = binding.bind(&catalog(), "math", "add");
        binding.inputs[0].activation_fixed_shape = false;
        let sealed = bind_call(kernel, &binding.request(), binding.call("math", "add"));
        assert!(validated_scalar(&sealed).is_none());
        let binding = Binding::scalar(NativeScalarOp::BoolNot);
        let kernel = binding.bind(&catalog(), "logic", "not");
        let wrong = Binding::scalar(NativeScalarOp::U64Add).call("math", "add");
        assert!(validated_scalar(&bind_call(kernel, &binding.request(), wrong)).is_none());
    }

    #[test]
    fn uncertified_numeric_shapes_and_operations_remain_uncertified() {
        let binding = Binding::scalar(NativeScalarOp::U64Add);
        let kernel = binding.bind(&catalog(), "math", "pow");
        assert!(
            validated_scalar(&bind_call(
                kernel,
                &binding.request(),
                binding.call("math", "pow")
            ))
            .is_none()
        );
        // Shape-one matrix schemas are not scalar semantic kinds.
        let matrix = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::Bool),
                dimensions: Box::new([
                    mech_core::DimensionExpr::Constant(1),
                    mech_core::DimensionExpr::Constant(1),
                ]),
            },
        }
        .finalize()
        .unwrap();
        let mut builder = SchemaTableBuilder::new();
        let handle = builder.insert(matrix).unwrap();
        let build = builder.finish().unwrap();
        let id: SchemaId = build.resolve(handle).unwrap();
        let mut binding = Binding::scalar(NativeScalarOp::BoolNot);
        binding.schemas = build.table;
        binding.inputs[0].schema_id = id;
        binding.inputs[0].schema_key = binding.schemas.entry(id).unwrap().key();
        binding.inputs[0].shape_instance = binding
            .schemas
            .get(id)
            .unwrap()
            .instantiate_shape(Box::new([]))
            .unwrap();
        binding.output = binding.inputs[0].clone();
        let request = binding.request();
        assert!(!scalar_request(
            &request,
            NativeScalarSemantics {
                op: NativeScalarOp::BoolNot
            }
        ));
    }
}

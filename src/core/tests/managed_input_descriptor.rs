#![cfg(all(
    feature = "functions",
    feature = "u64",
    feature = "f64",
    feature = "matrixd"
))]

use std::{cell::RefCell, rc::Rc};

use mech_core::{
    AccessMode, AliasPolicy, AllocationRole, ChangeDetectionPolicy, DeliveryMode, ExecutionTarget,
    ExternalInteraction, FunctionInvocation, ImplementationMemoryClass, InputPortLayout,
    InputPortPolicy, KernelMemoryFrame, MResult, ManagedPort, MechError, MechExecutionServices,
    MechFunctionImpl, MemoryDomain, OperationContractDeclaration, OutputConstruction,
    OutputPortPolicy, ReactiveSolveStatus, ResolvedOperationDescriptor, RuntimeFunctionId,
    SchemaBody, ShapeRule, SpecializedFunction, ValueCell, ValueData,
};
#[cfg(feature = "semantic-compiler")]
use mech_core::{BytecodeCompilerContext, MechFunctionCompiler, Register};

#[path = "support/allocation_probe.rs"]
mod allocation_probe;

#[global_allocator]
static ALLOCATOR: allocation_probe::ProbeAllocator = allocation_probe::ProbeAllocator;

struct DescriptorCount {
    source: ValueCell,
    output: ManagedPort<u64>,
    observed: Rc<RefCell<Vec<(u64, u64)>>>,
}

impl MechFunctionImpl for DescriptorCount {
    fn solve_managed(
        &self,
        frame: &mut KernelMemoryFrame<'_>,
        _: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        // Measure only the frame-owned descriptor read and borrowed dimension
        // resolution. Neither snapshots nor backing descriptors enter here.
        let (extents, allocations) = allocation_probe::measured(|| -> MResult<_> {
            let descriptor = frame.input_cell_descriptor(&self.source, 0)?;
            let SchemaBody::Matrix { dimensions, .. } = descriptor.schema().body() else {
                return Ok(None);
            };
            let [rows, columns] = dimensions.as_ref() else {
                return Ok(None);
            };
            Ok(Some((
                descriptor
                    .shape()
                    .resolve_dimension(rows)
                    .map_err(MechError::from)?,
                descriptor
                    .shape()
                    .resolve_dimension(columns)
                    .map_err(MechError::from)?,
            )))
        });
        assert_eq!(
            allocations, 0,
            "descriptor inspection allocated payload or metadata"
        );
        assert!(frame.input_cell_descriptor(&self.source, 1).is_err());
        assert!(
            frame
                .with_input_cell_value_view::<u64, _>(&self.source, 0, |_| Ok(()))
                .is_err(),
            "a typed view accepted a different numeric slot kind"
        );
        let (rows, columns) = extents?.expect("rank-two matrix input descriptor");
        self.observed.borrow_mut().push((rows, columns));
        let count = rows
            .checked_mul(columns)
            .expect("test matrix count fits u64");
        frame.with_port_init_writer(&self.output, |writer| writer.write_next(count))?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn to_string(&self) -> String {
        "descriptor-only managed matrix count".into()
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for DescriptorCount {
    fn compile(&self, _: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn input_descriptor_tracks_active_respecialized_shape_without_reconstructing_payload() {
    let domain = MemoryDomain::new().unwrap();
    let source =
        ValueCell::from_exact_in(&domain, nalgebra::DMatrix::from_element(2, 3, 11.0_f64)).unwrap();
    let source_identity = source.reactive_cell_id();
    let output = ValueCell::from_exact_in(&domain, 0_u64).unwrap();
    let observed = Rc::new(RefCell::new(Vec::with_capacity(6)));
    let operation = ResolvedOperationDescriptor::from_name(
        "test/input-cell-descriptor",
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(Box::new([InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }])),
            outputs: Box::new([OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::ExactScalar,
            }]),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap();
    let invocation = FunctionInvocation::unary(output.clone(), source.clone());
    let output_port = invocation.output().try_managed::<u64>().unwrap();
    let function = SpecializedFunction::syntax_directed(
        (
            Box::new(DescriptorCount {
                source: source.clone(),
                output: output_port,
                observed: observed.clone(),
            }),
            invocation,
        ),
        operation,
        RuntimeFunctionId::from_name("test/input-cell-descriptor"),
        ExecutionTarget::DirectRuntime,
        ImplementationMemoryClass::NoAdditionalScratch,
    )
    .unwrap();
    assert!(
        !function
            .instance()
            .memory_plan()
            .allocations
            .iter()
            .any(|allocation| allocation.role == AllocationRole::ConstructionWorkspace)
    );
    let mut revision = function.instance().managed_plan_revision();
    let shapes = [(2, 3), (3, 2), (128, 64), (0, 7), (5, 0), (1, 2)];
    for (turn, (rows, columns)) in shapes.into_iter().enumerate() {
        if turn > 0 {
            let candidate =
                ValueCell::from_exact(nalgebra::DMatrix::from_element(rows, columns, 37.0_f64))
                    .unwrap()
                    .snapshot()
                    .unwrap();
            source.replace(&candidate).unwrap();
        }
        function.instance().solve_result().unwrap();
        assert_eq!(observed.borrow()[turn], (rows as u64, columns as u64));
        assert!(
            matches!(output.snapshot().unwrap().data(), ValueData::U64(count)
            if *count == (rows * columns) as u64)
        );
        assert_eq!(source.reactive_cell_id(), source_identity);
        if turn > 0 {
            let current = function.instance().managed_plan_revision();
            assert_ne!(
                current, revision,
                "shape replacement did not respecialize the managed call"
            );
            revision = current;
        }
        // The immutable bind-time template deliberately remains at its old
        // shape; the kernel must read the current realized call instead.
        assert_eq!(
            function.instance().memory_plan().inputs[0]
                .descriptor
                .current_extents()
                .unwrap()
                .as_ref(),
            &[2, 3]
        );
    }
}

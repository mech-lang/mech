//! Immutable shape checks for flat private frames. No previous-visit extents.
//! Unsupported relations retain the ordinary composite constructor's path.
use super::InvokeError;
use mech_core::{
    DimensionExpr, DimensionLifetime, SchemaBody, SchemaId, SchemaTable, ShapeInstance,
};

#[derive(Debug)]
pub(super) struct FrameShapeRecipe {
    output: SchemaId,
    inputs: Box<[SchemaId]>,
    parameters: usize,
    axes: Box<[Axis]>,
    bindings: Box<[usize]>,
    complete_axes: bool,
}
#[derive(Debug)]
struct Axis {
    input: usize,
    expected: DimensionExpr,
    actual: DimensionExpr,
}

impl FrameShapeRecipe {
    pub(super) fn derive(
        output: SchemaId,
        inputs: &[SchemaId],
        schemas: &SchemaTable,
    ) -> Option<Self> {
        let fields = match schemas.get(output)?.body() {
            SchemaBody::Tuple(fields) => fields.iter().collect::<Vec<_>>(),
            SchemaBody::Record(fields) => fields.iter().map(|field| &field.schema).collect(),
            _ => return None,
        };
        Self::derive_leaves(output, inputs, &fields, schemas)
    }

    /// The same dimension proof applied to exact structural leaf paths. The
    /// composition owner has checked every intervening tuple/record identity.
    pub(super) fn derive_leaves(
        output: SchemaId,
        inputs: &[SchemaId],
        fields: &[&SchemaBody],
        schemas: &SchemaTable,
    ) -> Option<Self> {
        let definition = schemas.get(output)?;
        // This is this consumer's eligibility policy, not a new type relation.
        // Bounded/persistent/computed output axes use the general constructor.
        if !definition.dimension_parameters().iter().all(|p| {
            p.lifetime() == DimensionLifetime::Turn
                && p.lower_bound() == &DimensionExpr::Constant(0)
                && p.upper_bound().is_none()
        }) {
            return None;
        }
        if fields.len() != inputs.len() {
            return None;
        }
        let mut axes = Vec::new();
        let mut complete_axes = true;
        let mut bound = vec![false; definition.dimension_parameters().len()];
        for (input, (expected, actual)) in fields.iter().zip(inputs).enumerate() {
            let actual = schemas.get(*actual)?.body();
            match (expected, actual) {
                (
                    SchemaBody::Matrix {
                        element: expected,
                        dimensions: ed,
                    },
                    SchemaBody::Matrix {
                        element: actual,
                        dimensions: ad,
                    },
                ) if expected == actual && ed.len() == ad.len() => {
                    // Nested elements must be closed; no parameter relationship
                    // inside a record/table element is silently generalized.
                    if !(mech_core::SchemaDraft {
                        body: expected.as_ref().clone(),
                        dimension_parameters: Box::new([]),
                    })
                    .finalize()
                    .is_ok()
                    {
                        return None;
                    }
                    for (expected, actual) in ed.iter().zip(ad) {
                        match expected {
                            DimensionExpr::Constant(_) => {}
                            DimensionExpr::Parameter(id) => {
                                *bound.get_mut(id.get() as usize)? = true
                            }
                            _ => return None,
                        }
                        axes.push(Axis {
                            input,
                            expected: expected.clone(),
                            actual: actual.clone(),
                        });
                    }
                }
                _ if *expected == actual => {
                    // Equality of symbolic bodies does not relate their separate
                    // parameter arenas. Keep the existing validator when an
                    // opaque child can hide an output extent or overflow.
                    complete_axes &= (mech_core::SchemaDraft {
                        body: actual.clone(),
                        dimension_parameters: Box::new([]),
                    })
                    .finalize()
                    .is_ok();
                }
                _ => return None,
            }
        }
        if bound.iter().any(|value| !value) {
            return None;
        }
        let mut bindings = vec![0; bound.len()];
        for (index, axis) in axes.iter().enumerate() {
            if let DimensionExpr::Parameter(id) = axis.expected {
                *bindings.get_mut(id.get() as usize)? = index;
            }
        }
        Some(Self {
            output,
            inputs: inputs.into(),
            parameters: bound.len(),
            axes: axes.into(),
            bindings: bindings.into(),
            complete_axes,
        })
    }

    /// The caller uses this only after prepared or ordinary measurement has
    /// checked every current typed child. Each output matrix then has exactly
    /// its validated child's extents and checked element-count product. Output
    /// parameters are unconstrained Turn parameters, proven above. A miss
    /// retains the constructor and its original error instead of rejecting here.
    pub(super) fn matches_current<'a>(
        &self,
        current: impl Fn(usize) -> Option<&'a ShapeInstance>,
    ) -> bool {
        self.complete_axes
            && self.axes.iter().all(|axis| {
                let Some(shape) = current(axis.input) else {
                    return false;
                };
                let Ok(actual) = shape.resolve_dimension(&axis.actual) else {
                    return false;
                };
                let expected = match axis.expected {
                    DimensionExpr::Constant(value) => Some(value),
                    DimensionExpr::Parameter(id) => self
                        .bindings
                        .get(id.get() as usize)
                        .and_then(|index| self.axes.get(*index))
                        .and_then(|binding| {
                            current(binding.input)
                                .and_then(|shape| shape.resolve_dimension(&binding.actual).ok())
                        }),
                    _ => None,
                };
                expected == Some(actual)
            })
    }

    /// Run only after the caller admits its existing normalization envelope.
    /// Every input's current shape is checked in its own parameter arena.
    /// The single output-parameter allocation replaces child descriptions and
    /// repeated closed-schema construction/binding, not any live shape check.
    pub(super) fn validate<'a>(
        &self,
        schemas: &SchemaTable,
        current: impl Fn(usize) -> Option<&'a ShapeInstance>,
    ) -> Result<(), InvokeError> {
        for (index, schema) in self.inputs.iter().enumerate() {
            let schema = schemas.get(*schema).ok_or(InvokeError::InvalidInput)?;
            let parameters = current(index).map_or(&[][..], ShapeInstance::parameter_values);
            schema
                .instantiate_shape(parameters.to_vec().into_boxed_slice())
                .map_err(|_| InvokeError::InvalidInput)?;
        }
        let mut values = vec![0; self.parameters];
        for axis in &self.axes {
            let shape = current(axis.input).ok_or(InvokeError::InvalidInput)?;
            let actual = shape
                .resolve_dimension(&axis.actual)
                .map_err(|_| InvokeError::InvalidInput)?;
            if let DimensionExpr::Parameter(id) = axis.expected {
                values[id.get() as usize] = actual;
            }
        }
        let shape = schemas
            .get(self.output)
            .ok_or(InvokeError::InvalidInput)?
            .instantiate_shape(values.into_boxed_slice())
            .map_err(|_| InvokeError::InvalidInput)?;
        // Recheck all axes, including constants and repeated parameters; a later
        // child cannot replace an earlier child's current extent silently.
        for axis in &self.axes {
            let actual = current(axis.input)
                .ok_or(InvokeError::InvalidInput)?
                .resolve_dimension(&axis.actual)
                .map_err(|_| InvokeError::InvalidInput)?;
            if shape
                .resolve_dimension(&axis.expected)
                .map_err(|_| InvokeError::InvalidInput)?
                != actual
            {
                return Err(InvokeError::InvalidInput);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        DimensionParameterDeclaration, DimensionParameterId as Id, DimensionParameterOrigin,
        IntegerWidth, SchemaDraft, SchemaTableBuilder,
    };

    fn parameters(count: u32) -> Box<[DimensionParameterDeclaration]> {
        (0..count)
            .map(|n| DimensionParameterDeclaration {
                id: Id::new(n),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            })
            .collect()
    }
    fn matrix(dimensions: Vec<DimensionExpr>) -> SchemaBody {
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: dimensions.into(),
        }
    }
    fn output() -> SchemaDraft {
        let buffer = matrix(vec![
            DimensionExpr::Constant(1),
            DimensionExpr::Parameter(Id::new(0)),
        ]);
        SchemaDraft {
            body: SchemaBody::Tuple(vec![buffer.clone(), buffer].into()),
            dimension_parameters: parameters(1),
        }
    }

    #[test]
    fn frame_recipe_agrees_with_constructor_for_fresh_fixed_and_repeated_axes() {
        let mut builder = SchemaTableBuilder::new();
        let input = builder
            .insert(
                SchemaDraft {
                    body: matrix(vec![
                        DimensionExpr::Parameter(Id::new(0)),
                        DimensionExpr::Parameter(Id::new(1)),
                    ]),
                    dimension_parameters: parameters(2),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let result = builder.insert(output().finalize().unwrap()).unwrap();
        let built = builder.finish().unwrap();
        let input = built.resolve(input).unwrap();
        let result = built.resolve(result).unwrap();
        let recipe = FrameShapeRecipe::derive(result, &[input, input], &built.table).unwrap();
        for (a, b, accepted) in [
            (vec![1, 0], vec![1, 0], true),
            (vec![1, 5], vec![1, 5], true),
            (vec![1, 3], vec![1, 3], true),
            (vec![1, 3], vec![1, 5], false),
            (vec![2, 5], vec![1, 5], false),
            (vec![1, 5], vec![2, 5], false),
            (vec![1, 0], vec![1, 0], true),
        ] {
            let shapes = [
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(a.into())
                    .unwrap(),
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(b.into())
                    .unwrap(),
            ];
            let children = [(input, shapes[0].clone()), (input, shapes[1].clone())];
            let ordinary = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                result,
                &children,
                &built.table,
            )
            .and_then(|shape| {
                mech_core::snapshot::CompositeSnapshotConstructor::bind(
                    result,
                    shape,
                    &children,
                    built.table.clone().into(),
                )
            });
            assert_eq!(ordinary.is_ok(), accepted, "{children:?}");
            assert_eq!(
                recipe.validate(&built.table, |i| Some(&shapes[i])).is_ok(),
                ordinary.is_ok()
            );
            assert_eq!(
                recipe.matches_current(|i| Some(&shapes[i])),
                ordinary.is_ok()
            );
        }
    }

    #[test]
    fn frame_recipe_direct_relations_agree_for_fresh_rectangles_and_zero_products() {
        let body = matrix(vec![
            DimensionExpr::Parameter(Id::new(0)),
            DimensionExpr::Parameter(Id::new(1)),
        ]);
        let mut builder = SchemaTableBuilder::new();
        let input = builder
            .insert(
                SchemaDraft {
                    body: body.clone(),
                    dimension_parameters: parameters(2),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![body.clone(), body].into()),
                    dimension_parameters: parameters(2),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let input = built.resolve(input).unwrap();
        let output = built.resolve(output).unwrap();
        let recipe = FrameShapeRecipe::derive(output, &[input, input], &built.table).unwrap();
        for (left, right, accepted) in [
            ([0, 7], [0, 7], true),
            ([2, 3], [2, 3], true),
            ([3, 2], [3, 2], true),
            ([2, 3], [3, 2], false),
            ([0, 7], [0, 8], false),
            ([0, 0], [0, 0], true),
            ([u64::MAX, 0], [u64::MAX, 0], true),
        ] {
            let shapes = [left, right].map(|values| {
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(values.into())
                    .unwrap()
            });
            let children = [(input, shapes[0].clone()), (input, shapes[1].clone())];
            let ordinary = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                output,
                &children,
                &built.table,
            )
            .and_then(|shape| {
                mech_core::snapshot::CompositeSnapshotConstructor::bind(
                    output,
                    shape,
                    &children,
                    built.table.clone().into(),
                )
            });
            assert_eq!(ordinary.is_ok(), accepted, "{children:?}");
            assert_eq!(
                recipe.validate(&built.table, |i| Some(&shapes[i])).is_ok(),
                accepted
            );
            assert_eq!(recipe.matches_current(|i| Some(&shapes[i])), accepted);
        }
    }

    #[test]
    fn frame_shape_guard_preserves_nested_output_overflow_after_fallback() {
        // Both inputs are valid and empty. The second input uses its own shape
        // arena; replacing that arena with the first child's extent overflows.
        let direct = matrix(vec![
            DimensionExpr::Constant(0),
            DimensionExpr::Parameter(Id::new(0)),
        ]);
        let nested = SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: "nested".into(),
                schema: matrix(vec![
                    DimensionExpr::Constant(0),
                    DimensionExpr::Add(
                        vec![
                            DimensionExpr::Parameter(Id::new(0)),
                            DimensionExpr::Constant(1),
                        ]
                        .into(),
                    ),
                ]),
            }]
            .into(),
        );
        let mut builder = SchemaTableBuilder::new();
        let mut add = |body| {
            builder
                .insert(
                    SchemaDraft {
                        body,
                        dimension_parameters: parameters(1),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap()
        };
        let direct_id = add(direct.clone());
        let nested_id = add(nested.clone());
        let output_id = add(SchemaBody::Tuple(vec![direct, nested].into()));
        let built = builder.finish().unwrap();
        let inputs = [direct_id, nested_id].map(|handle| built.resolve(handle).unwrap());
        let output = built.resolve(output_id).unwrap();
        let values = [
            (
                inputs[0],
                vec![u64::MAX],
                mech_core::ValueDataDraft::Matrix(Box::new([])),
            ),
            (
                inputs[1],
                vec![0],
                mech_core::ValueDataDraft::Record(
                    vec![mech_core::snapshot::NamedValueDraft {
                        name: "nested".into(),
                        value: mech_core::ValueDataDraft::Matrix(Box::new([])),
                    }]
                    .into(),
                ),
            ),
        ]
        .map(|(schema, shape_values, data)| {
            mech_core::ValueDraft {
                schema,
                shape_values: shape_values.into(),
                data,
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                &built.table,
            ))
            .unwrap()
        });
        assert!(
            super::super::frame_admission::FrameAdmissionRecipe::derive(&inputs, &built.table)
                .is_none()
        );
        let recipe = FrameShapeRecipe::derive(output, &inputs, &built.table).unwrap();
        assert!(!recipe.is_identity_forwarding());
        assert!(
            recipe
                .validate(&built.table, |i| Some(values[i].shape()))
                .is_err()
        );
        let children = inputs.map(|schema| {
            let value = values.iter().find(|v| v.schema() == schema).unwrap();
            (schema, value.shape().clone())
        });
        assert!(
            mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                output,
                &children,
                &built.table,
            )
            .is_err()
        );
        assert!(
            !recipe.matches_current(|i| Some(values[i].shape())),
            "a current-shape shortcut must retain the ordinary output overflow check",
        );
    }

    #[test]
    fn opaque_record_matrices_keep_fresh_shape_relations_without_footprint_recipe() {
        let mut input = matrix(vec![
            DimensionExpr::Parameter(Id::new(0)),
            DimensionExpr::Parameter(Id::new(1)),
        ]);
        let SchemaBody::Matrix { element, .. } = &mut input else {
            unreachable!()
        };
        *element = Box::new(SchemaBody::Record(
            vec![mech_core::SchemaField {
                name: "x".into(),
                schema: SchemaBody::UnsignedInteger(IntegerWidth::W32),
            }]
            .into(),
        ));
        let mut field = input.clone();
        let SchemaBody::Matrix { dimensions, .. } = &mut field else {
            unreachable!()
        };
        *dimensions = vec![
            DimensionExpr::Constant(1),
            DimensionExpr::Parameter(Id::new(0)),
        ]
        .into();
        let mut builder = SchemaTableBuilder::new();
        let input = builder
            .insert(
                SchemaDraft {
                    body: input,
                    dimension_parameters: parameters(2),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let output = builder
            .insert(
                SchemaDraft {
                    body: SchemaBody::Tuple(vec![field.clone(), field].into()),
                    dimension_parameters: parameters(1),
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let built = builder.finish().unwrap();
        let input = built.resolve(input).unwrap();
        let output = built.resolve(output).unwrap();
        assert!(
            super::super::frame_admission::FrameAdmissionRecipe::derive(
                &[input, input],
                &built.table
            )
            .is_none()
        );
        let recipe = FrameShapeRecipe::derive(output, &[input, input], &built.table).unwrap();
        for (left, right, accepts) in [
            ([1, 0], [1, 0], true),
            ([1, 5], [1, 5], true),
            ([1, 3], [1, 5], false),
            ([2, 3], [1, 3], false),
            ([1, 3], [1, 3], true),
            ([1, 0], [1, 0], true),
        ] {
            let shapes = [left, right].map(|shape| {
                built
                    .table
                    .get(input)
                    .unwrap()
                    .instantiate_shape(shape.into())
                    .unwrap()
            });
            let children = [(input, shapes[0].clone()), (input, shapes[1].clone())];
            let ordinary = mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                output,
                &children,
                &built.table,
            )
            .and_then(|shape| {
                mech_core::snapshot::CompositeSnapshotConstructor::bind(
                    output,
                    shape,
                    &children,
                    built.table.clone().into(),
                )
            });
            assert_eq!(ordinary.is_ok(), accepts, "{left:?}, {right:?}");
            assert_eq!(
                recipe.validate(&built.table, |i| Some(&shapes[i])).is_ok(),
                accepts
            );
            assert_eq!(recipe.matches_current(|i| Some(&shapes[i])), accepts);
        }
    }

    #[cfg(feature = "source")]
    mod fallback_caller {
        use super::*;
        use crate::native_artifact::{
            NativeCollectionPrimitive, NativeIterationAction, NativeLibraryWorkBudget,
            lower_native_program_artifact, prepare_native_program_library_call,
        };
        use mech_core::DimensionExpr as D;
        use mech_core::snapshot::SnapshotValidationContext;
        use mech_core::{
            ManagedMemoryBudget, OperationContractTableBuilder, SchemaField,
            ValueDataDraft as Data, ValueDraft,
        };
        fn parameter() -> Box<[DimensionParameterDeclaration]> {
            vec![DimensionParameterDeclaration {
                id: Id::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: D::Constant(0),
                upper_bound: None,
            }]
            .into()
        }

        fn overflow_artifact() -> crate::ProgramArtifact {
            // Reuse the maintained pack operation, binding identities and contract.
            // Only its three canonical schemas are changed; finalize validates them.
            use mech_syntax::document::{
                AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
                parse_canonical_document,
            };
            let parsed = parse_canonical_document(
                TextSnapshot::new(
                    DocumentId(0x53485045),
                    Revision(0),
                    "(left<[u8]>, right<[u16]>)\n",
                )
                .unwrap(),
                ParseConfig::default(),
            );
            assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
            let base = crate::CanonicalSourceFrontend
                .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
                .unwrap()
                .compile_artifact()
                .unwrap();
            assert_eq!(base.nodes().len(), 1);
            assert_eq!(base.inputs().len(), 2);
            assert_eq!(base.outputs().len(), 1);
            assert!(base.constants().is_empty());
            assert!(base.constraints().is_empty());
            assert!(base.compute_regions().is_empty());
            let matrix = |axis| SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                dimensions: vec![D::Constant(0), axis].into(),
            };
            let direct = matrix(D::Parameter(Id::new(0)));
            let nested = SchemaBody::Record(
                vec![SchemaField {
                    name: "nested".into(),
                    schema: matrix(D::Add(
                        vec![D::Parameter(Id::new(0)), D::Constant(1)].into(),
                    )),
                }]
                .into(),
            );
            let mut builder = SchemaTableBuilder::new();
            let handles = [
                direct.clone(),
                nested.clone(),
                SchemaBody::Tuple(vec![direct, nested].into()),
            ]
            .map(|body| {
                builder
                    .insert(
                        SchemaDraft {
                            body,
                            dimension_parameters: parameter(),
                        }
                        .finalize()
                        .unwrap(),
                    )
                    .unwrap()
            });
            let built = builder.finish().unwrap();
            let ids = handles.map(|handle| built.resolve(handle).unwrap());
            // Artifact schemas must retain canonical sorted IDs. An append-only
            // table extension is valid for runtime owners, not encoded artifacts.
            let schemas = built.table;
            let input = |name: &str| {
                base.inputs()
                    .iter()
                    .find(|input| crate::decode_source_input_name(&input.name).unwrap() == name)
                    .unwrap()
                    .schema
            };
            let prior = [input("left"), input("right"), base.outputs()[0].schema];
            let remap = |schema| {
                ids[prior
                    .iter()
                    .position(|old| *old == schema)
                    .expect("the fixture has only its two input and one output schemas")]
            };
            let mut inputs = base.inputs().to_vec();
            for input in &mut inputs {
                input.schema = remap(input.schema);
            }
            let mut slots = base.slots().to_vec();
            for slot in &mut slots {
                slot.schema = remap(slot.schema);
            }
            let mut outputs = base.outputs().to_vec();
            for output in &mut outputs {
                output.schema = remap(output.schema);
            }
            let mut nodes = base.nodes().to_vec();
            let crate::ExecutableNodeBody::Operation(operation) = &mut nodes[0].body else {
                panic!("one maintained pack");
            };
            let mut contract = base.contracts().get(operation.contract).unwrap().clone();
            let mech_core::ResolvedOperationContract::Declared(declared) = &mut contract else {
                panic!("declared pack contract");
            };
            for input in &mut declared.inputs {
                input.schema = remap(input.schema);
            }
            for output in &mut declared.outputs {
                output.schema = remap(output.schema);
            }
            let mut contracts = OperationContractTableBuilder::new();
            let handle = contracts.insert(contract).unwrap();
            let contracts = contracts.finish().unwrap();
            operation.contract = contracts.resolve(handle).unwrap();
            crate::ProgramArtifactDraft {
                schemas,
                constants: base.constants().clone(),
                contracts: contracts.table,
                requirements: base.requirements().clone(),
                inputs: inputs.into(),
                slots: slots.into(),
                nodes: nodes.into(),
                bindings: base.bindings().into(),
                outputs: outputs.into(),
                constraints: base.constraints().into(),
                compute_regions: base.compute_regions().into(),
            }
            .finalize()
            .unwrap()
        }

        #[test]
        fn ordinary_frame_fallback_preserves_nested_output_overflow_and_retry() {
            let original = overflow_artifact();
            let decoded = crate::decode_program_artifact_bytecode_v1(
                &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
            )
            .unwrap();
            let mut expected = None;
            for artifact in [original, decoded] {
                let mut catalog = mech_core::FunctionCatalogBuilder::new();
                crate::install_intrinsic_resident(&mut catalog).unwrap();
                let catalog = catalog.build().unwrap();
                let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
                let template = plan
                    .blocks()
                    .iter()
                    .find_map(|block| match block.action() {
                        NativeIterationAction::LibraryCall { template, .. }
                            if template.primitive()
                                == NativeCollectionPrimitive::CompositePack(2) =>
                        {
                            Some(template)
                        }
                        _ => None,
                    })
                    .unwrap();
                let call =
                    prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
                let mech_core::ResolvedOperationContract::Declared(contract) = template.contract()
                else {
                    panic!()
                };
                let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                let make = |index: usize, extent: u64| {
                    let data = if index == 0 {
                        Data::Matrix(Box::new([]))
                    } else {
                        Data::Record(
                            vec![mech_core::snapshot::NamedValueDraft {
                                name: "nested".into(),
                                value: Data::Matrix(Box::new([])),
                            }]
                            .into(),
                        )
                    };
                    let raw = ValueDraft {
                        schema: contract.inputs[index].schema,
                        shape_values: vec![extent].into(),
                        data,
                    }
                    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                    .unwrap();
                    let mut reserve = memory
                        .reserve_capacity(raw.memory_budget_admission_bytes(&memory).unwrap())
                        .unwrap();
                    raw.into_memory_budget(&mut reserve).unwrap()
                };
                let right = make(1, 0);
                let mut receipts = Vec::new();
                for (name, extent, accepts) in [
                    ("valid", 0, true),
                    ("overflow", u64::MAX, false),
                    ("retry", 0, true),
                ] {
                    let values = [Some(make(0, extent)), Some(right.clone())];
                    let aliases = values.clone();
                    // Independently run the maintained constructor on the same
                    // actual children before exercising private admission.
                    let children = values.each_ref().map(|value| {
                        let value = value.as_ref().unwrap();
                        (value.schema(), value.shape().clone())
                    });
                    let ordinary =
                        mech_core::snapshot::CompositeSnapshotConstructor::shape_for_children(
                            contract.outputs[0].schema,
                            &children,
                            artifact.schemas(),
                        )
                        .and_then(|shape| {
                            mech_core::snapshot::CompositeSnapshotConstructor::bind(
                                contract.outputs[0].schema,
                                shape,
                                &children,
                                artifact.schemas().clone().into(),
                            )
                        })
                        .and_then(|constructor| {
                            constructor.construct(
                                values
                                    .iter()
                                    .map(|value| value.as_ref().unwrap().clone())
                                    .collect(),
                                None,
                            )
                        })
                        .map(|value| {
                            (
                                value.shape().parameter_values().to_vec(),
                                value.canonical_data_draft().unwrap(),
                            )
                        })
                        .map_err(|error| format!("{error:?}"));
                    assert_eq!(ordinary.is_ok(), accepts, "{name}: {ordinary:?}");
                    if let Ok((shape, data)) = &ordinary {
                        assert_eq!(shape, &[0]);
                        assert_eq!(
                            data,
                            &Data::Tuple(
                                vec![
                                    Data::Matrix(Box::new([])),
                                    Data::Record(
                                        vec![mech_core::snapshot::NamedValueDraft {
                                            name: "nested".into(),
                                            value: Data::Matrix(Box::new([])),
                                        }]
                                        .into()
                                    ),
                                ]
                                .into()
                            )
                        );
                    }
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let run = |allowance| {
                        let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
                            .unwrap()
                            .limits;
                        limits.max_output_bytes = Some(128 * 1024 * 1024);
                        limits.max_temporary_bytes = Some(128 * 1024 * 1024);
                        limits.max_output_elements = Some(u64::MAX);
                        limits.max_compute_work = Some(allowance);
                        let work =
                            NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
                        let result = call.admit_buffer_frame_words(
                            &values,
                            &[0, 0],
                            limits,
                            &work,
                            Default::default(),
                        );
                        let receipt = (format!("{result:?}"), work.used());
                        drop(work);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                        assert!(values.iter().zip(&aliases).all(|(value, alias)| {
                            value
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(alias.as_ref().unwrap())
                        }));
                        (result, receipt)
                    };
                    let (result, full) = run(u64::MAX);
                    assert_eq!(result.is_ok(), accepts, "{name}: {result:?}");
                    if !accepts {
                        assert_eq!(full.0, "Err(InvalidInput)");
                    }
                    assert!(full.1.0 > 0);
                    let (denied, prefix) = run(full.1.0 - 1);
                    assert!(
                        format!("{denied:?}").contains("ComputeWork"),
                        "{name}: {denied:?}"
                    );
                    assert!(prefix.1.0 < full.1.0);
                    assert_eq!(run(u64::MAX).1, full);
                    receipts.push((name, ordinary, full, prefix));
                }
                println!("shape-fallback-caller: {receipts:?}");
                assert_eq!(&receipts, expected.get_or_insert_with(|| receipts.clone()));
                drop(right);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }

    #[test]
    fn frame_recipe_declines_bounded_persistent_and_computed_output_relations() {
        for policy in 0..3 {
            let mut draft = output();
            match policy {
                0 => draft.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(5)),
                1 => draft.dimension_parameters[0].lifetime = DimensionLifetime::Activation,
                _ => {
                    let SchemaBody::Tuple(fields) = &mut draft.body else {
                        unreachable!()
                    };
                    for field in fields {
                        let SchemaBody::Matrix { dimensions, .. } = field else {
                            unreachable!()
                        };
                        dimensions[1] = DimensionExpr::Add(
                            vec![
                                DimensionExpr::Parameter(Id::new(0)),
                                DimensionExpr::Constant(1),
                            ]
                            .into(),
                        );
                    }
                }
            }
            let mut builder = SchemaTableBuilder::new();
            let input = builder
                .insert(
                    SchemaDraft {
                        body: matrix(vec![
                            DimensionExpr::Constant(1),
                            DimensionExpr::Parameter(Id::new(0)),
                        ]),
                        dimension_parameters: parameters(1),
                    }
                    .finalize()
                    .unwrap(),
                )
                .unwrap();
            let result = builder.insert(draft.finalize().unwrap()).unwrap();
            let built = builder.finish().unwrap();
            let result = built.resolve(result).unwrap();
            let input = built.resolve(input).unwrap();
            assert!(FrameShapeRecipe::derive(result, &[input, input], &built.table).is_none());
        }
    }
}

impl FrameShapeRecipe {
    /// Every dynamic output axis is a distinct unconstrained Turn parameter;
    /// constant axes already agree statically. No current-value relation can
    /// fail when an invariant, previously validated child is forwarded.
    pub(super) fn is_identity_forwarding(&self) -> bool {
        if !self.complete_axes {
            return false;
        }
        let mut seen = vec![false; self.parameters];
        for axis in &self.axes {
            match axis.expected {
                DimensionExpr::Constant(_) if axis.expected == axis.actual => {}
                DimensionExpr::Parameter(id) => {
                    let Some(slot) = seen.get_mut(id.get() as usize) else {
                        return false;
                    };
                    if core::mem::replace(slot, true) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        seen.iter().all(|seen| *seen)
    }
}

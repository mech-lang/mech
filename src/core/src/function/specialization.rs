#[cfg(feature = "no_std")]
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

use crate::{
    DimensionExpr, FunctionCatalog, FunctionInstance, FunctionInvocation, FunctionPortBacking,
    FunctionValueRepresentation, MResult, MechError, MechErrorKind, MechFunctionFactory, Ref,
    RuntimeFunctionInputs, Schema, SchemaBody, SchemaKey, SchemaTable, SchemaTableBuilder,
    ShapeInstance, Value, ValueCell,
};

#[cfg(feature = "matrix")]
use crate::{FunctionArgumentRole, matrix::Matrix};

/// One source-level function input after expression lowering.
///
/// Absence and matrix all-selection are source controls; neither is a
/// canonical runtime value. Keeping them explicit prevents either control
/// from being confused with canonical unit or option absence.
#[derive(Clone, Debug)]
pub enum SpecializationInput {
    Cell(ValueCell),
    Absent,
    MatrixAllSelection,
}

impl SpecializationInput {
    pub const fn is_absent(&self) -> bool {
        matches!(self, Self::Absent)
    }

    pub fn require_matrix_all_selection(&self) -> MResult<()> {
        match self {
            Self::MatrixAllSelection => Ok(()),
            Self::Cell(_) => Err(control_input_error("value", "matrix all-selection")),
            Self::Absent => Err(control_input_error(
                "source absence",
                "matrix all-selection",
            )),
        }
    }

    pub fn cell(&self) -> MResult<&ValueCell> {
        match self {
            Self::Cell(cell) => Ok(cell),
            Self::Absent => Err(control_input_error("source absence", "cell")),
            Self::MatrixAllSelection => Err(control_input_error("matrix all-selection", "cell")),
        }
    }

    pub fn try_ref<T: FunctionPortBacking>(&self) -> MResult<Ref<T>> {
        self.cell()?.try_ref::<T>()
    }

    #[cfg(feature = "matrix")]
    pub fn try_matrix<T>(&self, input_index: usize) -> MResult<Matrix<T>>
    where
        T: FunctionPortBacking + Clone,
    {
        crate::function::argument::matrix_from_cell(
            self.cell()?,
            FunctionArgumentRole::Input(input_index),
        )
    }

    pub fn snapshot(&self) -> MResult<Value> {
        self.cell()?.snapshot()
    }

    pub fn schema_key(&self) -> Option<SchemaKey> {
        match self {
            Self::Cell(cell) => Some(cell.schema_key()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    pub fn closed_schema_body(&self) -> MResult<Option<SchemaBody>> {
        match self {
            Self::Cell(cell) => cell.closed_schema_body().map(Some),
            Self::Absent | Self::MatrixAllSelection => Ok(None),
        }
    }

    pub fn shape(&self) -> Option<ShapeInstance> {
        match self {
            Self::Cell(cell) => Some(cell.shape().clone()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    pub fn representation(&self) -> Option<FunctionValueRepresentation> {
        match self {
            Self::Cell(cell) => Some(cell.representation()),
            Self::Absent | Self::MatrixAllSelection => None,
        }
    }

    #[cfg(feature = "matrix")]
    pub fn matrix_descriptor(&self) -> MResult<Option<crate::FunctionMatrixDescriptor>> {
        match self {
            Self::Cell(cell) => crate::function::argument::canonical_matrix_descriptor(cell),
            Self::Absent | Self::MatrixAllSelection => Ok(None),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpecializationInvocation {
    inputs: Box<[SpecializationInput]>,
}

impl SpecializationInvocation {
    pub fn new(inputs: Box<[SpecializationInput]>) -> Self {
        Self { inputs }
    }

    pub fn from_cells(inputs: Box<[ValueCell]>) -> Self {
        Self::new(
            inputs
                .into_vec()
                .into_iter()
                .map(SpecializationInput::Cell)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        )
    }

    pub fn inputs(&self) -> &[SpecializationInput] {
        &self.inputs
    }

    pub fn input(&self, index: usize) -> Option<&SpecializationInput> {
        self.inputs.get(index)
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }
}

/// Explicit, invocation-local facilities used while selecting a concrete
/// function implementation. No global schema or catalog state is consulted.
pub struct SpecializationContext<'a> {
    schemas: Rc<SchemaTable>,
    catalog: Option<&'a FunctionCatalog>,
}

impl<'a> SpecializationContext<'a> {
    pub fn new(schemas: Rc<SchemaTable>) -> Self {
        Self {
            schemas,
            catalog: None,
        }
    }

    pub fn with_catalog(schemas: Rc<SchemaTable>, catalog: &'a FunctionCatalog) -> Self {
        Self {
            schemas,
            catalog: Some(catalog),
        }
    }

    pub fn for_invocation(
        invocation: &SpecializationInvocation,
        catalog: Option<&'a FunctionCatalog>,
    ) -> MResult<Self> {
        let mut builder = SchemaTableBuilder::new();
        for input in invocation.inputs() {
            let SpecializationInput::Cell(cell) = input else {
                continue;
            };
            let schemas = cell.schema_table();
            for entry in schemas.entries() {
                builder.insert(entry.schema().clone())?;
            }
        }
        let schemas = if builder.is_empty() {
            empty_schema_table()
        } else {
            Rc::new(builder.finish()?.table)
        };
        Ok(Self { schemas, catalog })
    }

    pub fn schemas(&self) -> &SchemaTable {
        self.schemas.as_ref()
    }

    pub fn catalog(&self) -> Option<&FunctionCatalog> {
        self.catalog
    }

    pub fn schema(&self, key: SchemaKey) -> MResult<&Schema> {
        self.schemas
            .find_by_key(key)
            .and_then(|id| self.schemas.get(id))
            .ok_or_else(|| {
                MechError::new(SpecializationUnknownSchema { key }, None).with_compiler_loc()
            })
    }

    pub fn resolve_shape(
        &self,
        schema: SchemaKey,
        parameter_values: Box<[u64]>,
    ) -> MResult<ShapeInstance> {
        Ok(self.schema(schema)?.instantiate_shape(parameter_values)?)
    }

    pub fn resolve_dimension(
        &self,
        shape: &ShapeInstance,
        dimension: &DimensionExpr,
    ) -> MResult<u64> {
        Ok(shape.resolve_dimension(dimension)?)
    }

    pub fn typed_cell<T>(
        &self,
        reference: Ref<T>,
        schema: SchemaKey,
        shape: ShapeInstance,
    ) -> MResult<ValueCell>
    where
        T: crate::CanonicalCellBacking,
    {
        let schema = self.schemas.find_by_key(schema).ok_or_else(|| {
            MechError::new(SpecializationUnknownSchema { key: schema }, None).with_compiler_loc()
        })?;
        ValueCell::from_ref(reference, schema, shape, self.schemas.clone())
    }

    pub fn value_cell(&self, value: Value) -> MResult<ValueCell> {
        ValueCell::from_runtime_value(value, self.schemas.clone())
    }

    /// Selects and binds one exact runtime factory from canonical input and
    /// output representations.
    ///
    /// This is the shared source/runtime seam for operation families whose
    /// concrete factories are already registered in the catalog. The caller
    /// derives output representation and extent from operation semantics;
    /// this method never projects a cell through the legacy universal value.
    pub fn bind_runtime_factory(
        &self,
        name_prefix: &str,
        output_representation: FunctionValueRepresentation,
        output_dimensions: Option<(usize, usize)>,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    factory_prefix: name_prefix.into(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let input_representations = inputs
            .iter()
            .map(|input| {
                input.representation().ok_or_else(|| {
                    control_input_error("non-value", "runtime factory representation")
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        let mut candidates = catalog.runtime_entries().filter(|entry| {
            entry.name.starts_with(name_prefix)
                && entry.signature().output == output_representation
                && runtime_inputs_match(entry.signature().inputs, input_representations.as_slice())
        });
        let entry = candidates.next().ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    factory_prefix: name_prefix.into(),
                    output: output_representation,
                    inputs: input_representations.clone().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if let Some(second) = candidates.next() {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    factory_prefix: name_prefix.into(),
                    first: entry.name.clone(),
                    second: second.name.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let output =
            ValueCell::default_for_representation(output_representation, output_dimensions)?;
        let input_cells = inputs
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let invocation =
            invocation_for_runtime_inputs(entry.signature().inputs, output, input_cells)?;
        Ok(SpecializedFunction::new(entry.bind_invocation(invocation)?))
    }

    /// Selects a runtime factory by canonical inputs and lets the registered
    /// signature determine the exact output storage. Fixed output storage is
    /// preferred over a dynamic fallback when both represent the same logical
    /// extent (for example row-vector by column-vector multiplication).
    pub fn bind_runtime_factory_derived_output(
        &self,
        name_prefix: &str,
        output_dimensions: Option<(usize, usize)>,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    factory_prefix: name_prefix.into(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let input_representations = inputs
            .iter()
            .map(|input| {
                input.representation().ok_or_else(|| {
                    control_input_error("non-value", "runtime factory representation")
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        let mut candidates = catalog
            .runtime_entries()
            .filter(|entry| {
                entry.name.starts_with(name_prefix)
                    && runtime_inputs_match(
                        entry.signature().inputs,
                        input_representations.as_slice(),
                    )
            })
            .filter_map(|entry| {
                let dimensions = output_dimensions
                    .or_else(|| inferred_output_dimensions(entry.signature().output, inputs));
                ValueCell::default_for_representation(entry.signature().output, dimensions)
                    .ok()
                    .map(|output| (runtime_output_rank(entry.signature().output), entry, output))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(rank, entry, _)| (*rank, entry.id));
        let Some((best_rank, entry, output)) = candidates.into_iter().next() else {
            return Err(MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    factory_prefix: name_prefix.into(),
                    output: FunctionValueRepresentation::AnyValue,
                    inputs: input_representations.into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc());
        };
        let competing = catalog.runtime_entries().find(|candidate| {
            candidate.id != entry.id
                && candidate.name.starts_with(name_prefix)
                && runtime_inputs_match(
                    candidate.signature().inputs,
                    input_representations.as_slice(),
                )
                && runtime_output_rank(candidate.signature().output) == best_rank
                && ValueCell::default_for_representation(
                    candidate.signature().output,
                    output_dimensions.or_else(|| {
                        inferred_output_dimensions(candidate.signature().output, inputs)
                    }),
                )
                .is_ok()
        });
        if let Some(second) = competing {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    factory_prefix: name_prefix.into(),
                    first: entry.name.clone(),
                    second: second.name.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let input_cells = inputs
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let invocation =
            invocation_for_runtime_inputs(entry.signature().inputs, output, input_cells)?;
        Ok(SpecializedFunction::new(entry.bind_invocation(invocation)?))
    }

    /// Selects a registered runtime factory while retaining an existing
    /// canonical output cell as the authoritative read-modify-write target.
    ///
    /// Assignment specialization uses this path so the output is never
    /// projected into, or reconstructed from, the legacy universal value.
    pub fn bind_runtime_factory_existing_output(
        &self,
        name_prefix: &str,
        output: &SpecializationInput,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    factory_prefix: name_prefix.into(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let output_representation = output.representation().ok_or_else(|| {
            control_input_error("non-value", "runtime factory output representation")
        })?;
        let input_representations = inputs
            .iter()
            .map(|input| {
                input.representation().ok_or_else(|| {
                    control_input_error("non-value", "runtime factory representation")
                })
            })
            .collect::<MResult<Vec<_>>>()?;
        let mut candidates = catalog.runtime_entries().filter(|entry| {
            entry.name.starts_with(name_prefix)
                && entry.signature().output == output_representation
                && runtime_inputs_match(entry.signature().inputs, input_representations.as_slice())
        });
        let entry = candidates.next().ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    factory_prefix: name_prefix.into(),
                    output: output_representation,
                    inputs: input_representations.clone().into_boxed_slice(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if let Some(second) = candidates.next() {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    factory_prefix: name_prefix.into(),
                    first: entry.name.clone(),
                    second: second.name.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let output = output.cell()?.clone();
        let input_cells = inputs
            .iter()
            .map(|input| input.cell().cloned())
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let invocation =
            invocation_for_runtime_inputs(entry.signature().inputs, output, input_cells)?;
        Ok(SpecializedFunction::new(entry.bind_invocation(invocation)?))
    }
}

fn inferred_output_dimensions(
    representation: FunctionValueRepresentation,
    _inputs: &[&SpecializationInput],
) -> Option<(usize, usize)> {
    let FunctionValueRepresentation::Matrix { storage, .. } = representation else {
        return None;
    };
    let exact = match storage {
        crate::FunctionMatrixStoragePattern::Exact(storage) => storage,
        crate::FunctionMatrixStoragePattern::AnyStorage => return None,
    };
    match exact {
        crate::FunctionMatrixRepresentation::Matrix1 => return Some((1, 1)),
        crate::FunctionMatrixRepresentation::Matrix2 => return Some((2, 2)),
        crate::FunctionMatrixRepresentation::Matrix3 => return Some((3, 3)),
        crate::FunctionMatrixRepresentation::Matrix4 => return Some((4, 4)),
        crate::FunctionMatrixRepresentation::Matrix2x3 => return Some((2, 3)),
        crate::FunctionMatrixRepresentation::Matrix3x2 => return Some((3, 2)),
        crate::FunctionMatrixRepresentation::RowVector2 => return Some((1, 2)),
        crate::FunctionMatrixRepresentation::RowVector3 => return Some((1, 3)),
        crate::FunctionMatrixRepresentation::RowVector4 => return Some((1, 4)),
        crate::FunctionMatrixRepresentation::Vector2 => return Some((2, 1)),
        crate::FunctionMatrixRepresentation::Vector3 => return Some((3, 1)),
        crate::FunctionMatrixRepresentation::Vector4 => return Some((4, 1)),
        crate::FunctionMatrixRepresentation::RowVectorD
        | crate::FunctionMatrixRepresentation::VectorD
        | crate::FunctionMatrixRepresentation::MatrixD => {}
    }
    #[cfg(feature = "matrix")]
    {
        let descriptors = _inputs
            .iter()
            .filter_map(|input| input.matrix_descriptor().ok().flatten())
            .collect::<Vec<_>>();
        if descriptors.is_empty() {
            return None;
        }
        return match exact {
            crate::FunctionMatrixRepresentation::RowVectorD => descriptors
                .iter()
                .map(|value| value.cols)
                .max()
                .map(|cols| (1, cols)),
            crate::FunctionMatrixRepresentation::VectorD => descriptors
                .iter()
                .map(|value| value.rows)
                .max()
                .map(|rows| (rows, 1)),
            crate::FunctionMatrixRepresentation::MatrixD => Some((
                descriptors.iter().map(|value| value.rows).max()?,
                descriptors.iter().map(|value| value.cols).max()?,
            )),
            _ => unreachable!("fixed matrix outputs return above"),
        };
    }
    #[cfg(not(feature = "matrix"))]
    None
}

fn runtime_output_rank(representation: FunctionValueRepresentation) -> u8 {
    match representation {
        FunctionValueRepresentation::Matrix {
            storage:
                crate::FunctionMatrixStoragePattern::Exact(
                    crate::FunctionMatrixRepresentation::RowVectorD
                    | crate::FunctionMatrixRepresentation::VectorD
                    | crate::FunctionMatrixRepresentation::MatrixD,
                ),
            ..
        } => 1,
        _ => 0,
    }
}

fn runtime_inputs_match(
    signature: RuntimeFunctionInputs,
    inputs: &[FunctionValueRepresentation],
) -> bool {
    match (signature, inputs) {
        (RuntimeFunctionInputs::Nullary, []) => true,
        (RuntimeFunctionInputs::Unary(expected), [actual]) => expected.matches(*actual),
        (RuntimeFunctionInputs::Binary(first, second), [actual_first, actual_second]) => {
            first.matches(*actual_first) && second.matches(*actual_second)
        }
        (
            RuntimeFunctionInputs::Ternary(first, second, third),
            [actual_first, actual_second, actual_third],
        ) => {
            first.matches(*actual_first)
                && second.matches(*actual_second)
                && third.matches(*actual_third)
        }
        (
            RuntimeFunctionInputs::Quaternary(first, second, third, fourth),
            [actual_first, actual_second, actual_third, actual_fourth],
        ) => {
            first.matches(*actual_first)
                && second.matches(*actual_second)
                && third.matches(*actual_third)
                && fourth.matches(*actual_fourth)
        }
        (RuntimeFunctionInputs::Variadic { element }, inputs) => {
            inputs.iter().all(|actual| element.matches(*actual))
        }
        _ => false,
    }
}

fn invocation_for_runtime_inputs(
    signature: RuntimeFunctionInputs,
    output: ValueCell,
    inputs: Box<[ValueCell]>,
) -> MResult<FunctionInvocation> {
    let inputs = inputs.into_vec();
    Ok(match (signature, inputs.as_slice()) {
        (RuntimeFunctionInputs::Nullary, []) => FunctionInvocation::nullary(output),
        (RuntimeFunctionInputs::Unary(_), [input]) => {
            FunctionInvocation::unary(output, input.clone())
        }
        (RuntimeFunctionInputs::Binary(_, _), [first, second]) => {
            FunctionInvocation::binary(output, first.clone(), second.clone())
        }
        (RuntimeFunctionInputs::Ternary(_, _, _), [first, second, third]) => {
            FunctionInvocation::ternary(output, first.clone(), second.clone(), third.clone())
        }
        (RuntimeFunctionInputs::Quaternary(_, _, _, _), [first, second, third, fourth]) => {
            FunctionInvocation::quaternary(
                output,
                first.clone(),
                second.clone(),
                third.clone(),
                fourth.clone(),
            )
        }
        (RuntimeFunctionInputs::Variadic { .. }, inputs) => {
            FunctionInvocation::variadic(output, inputs.to_vec().into_boxed_slice())
        }
        (signature, inputs) => {
            return Err(MechError::new(
                crate::IncorrectNumberOfArguments {
                    expected: match signature {
                        RuntimeFunctionInputs::Nullary => 0,
                        RuntimeFunctionInputs::Unary(_) => 1,
                        RuntimeFunctionInputs::Binary(_, _) => 2,
                        RuntimeFunctionInputs::Ternary(_, _, _) => 3,
                        RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
                        RuntimeFunctionInputs::Variadic { .. } => 0,
                    },
                    found: inputs.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
    })
}

pub struct SpecializedFunction {
    instance: FunctionInstance,
}

impl SpecializedFunction {
    pub fn new(instance: FunctionInstance) -> Self {
        Self { instance }
    }

    pub fn instance(&self) -> &FunctionInstance {
        &self.instance
    }

    pub fn output(&self) -> &ValueCell {
        self.instance.output()
    }

    pub fn into_instance(self) -> FunctionInstance {
        self.instance
    }

    /// Binds a canonical output and canonical source inputs directly to a
    /// runtime factory while preserving the factory's declared arity.
    pub fn bind_factory<F>(output: ValueCell, inputs: Box<[ValueCell]>) -> MResult<Self>
    where
        F: MechFunctionFactory,
    {
        let invocation = match (F::SIGNATURE.inputs, inputs.into_vec().as_slice()) {
            (RuntimeFunctionInputs::Nullary, []) => FunctionInvocation::nullary(output),
            (RuntimeFunctionInputs::Unary(_), [input]) => {
                FunctionInvocation::unary(output, input.clone())
            }
            (RuntimeFunctionInputs::Binary(_, _), [first, second]) => {
                FunctionInvocation::binary(output, first.clone(), second.clone())
            }
            (RuntimeFunctionInputs::Ternary(_, _, _), [first, second, third]) => {
                FunctionInvocation::ternary(output, first.clone(), second.clone(), third.clone())
            }
            (RuntimeFunctionInputs::Quaternary(_, _, _, _), [first, second, third, fourth]) => {
                FunctionInvocation::quaternary(
                    output,
                    first.clone(),
                    second.clone(),
                    third.clone(),
                    fourth.clone(),
                )
            }
            (RuntimeFunctionInputs::Variadic { .. }, inputs) => {
                FunctionInvocation::variadic(output, inputs.to_vec().into_boxed_slice())
            }
            (signature, inputs) => {
                let expected = match signature {
                    RuntimeFunctionInputs::Nullary => 0,
                    RuntimeFunctionInputs::Unary(_) => 1,
                    RuntimeFunctionInputs::Binary(_, _) => 2,
                    RuntimeFunctionInputs::Ternary(_, _, _) => 3,
                    RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
                    RuntimeFunctionInputs::Variadic { .. } => inputs.len(),
                };
                return Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected,
                        found: inputs.len(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        let implementation = F::new_invocation(invocation.clone())?;
        Ok(Self::new(FunctionInstance::new(implementation, invocation)))
    }

    pub fn with_semantic_operation(self, operation: impl Into<Box<str>>) -> Self {
        Self::new(self.instance.with_semantic_operation(operation))
    }

    #[doc(hidden)]
    pub fn into_legacy_implementation(self) -> Box<dyn crate::MechFunction> {
        self.instance.into_implementation()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationInputAbsent {
    pub control: String,
    pub requested: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationUnknownSchema {
    pub key: SchemaKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeCatalogUnavailable {
    pub factory_prefix: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryUnavailable {
    pub factory_prefix: String,
    pub output: FunctionValueRepresentation,
    pub inputs: Box<[FunctionValueRepresentation]>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryAmbiguous {
    pub factory_prefix: String,
    pub first: String,
    pub second: String,
}

impl MechErrorKind for SpecializationUnknownSchema {
    fn name(&self) -> &str {
        "SpecializationUnknownSchema"
    }

    fn message(&self) -> String {
        format!(
            "source specialization referenced unknown schema {:?}",
            self.key,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeCatalogUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeCatalogUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "canonical source specialization for {} requires the runtime factory catalog",
            self.factory_prefix,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "no canonical runtime factory matching {} {:?} -> {:?} is registered",
            self.factory_prefix, self.inputs, self.output,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryAmbiguous {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryAmbiguous"
    }

    fn message(&self) -> String {
        format!(
            "canonical runtime factory selection for {} is ambiguous between {:?} and {:?}",
            self.factory_prefix, self.first, self.second,
        )
    }
}

impl MechErrorKind for SpecializationInputAbsent {
    fn name(&self) -> &str {
        "SpecializationInputAbsent"
    }

    fn message(&self) -> String {
        format!(
            "source specialization requested {} from {} control input",
            self.requested, self.control,
        )
    }
}

fn control_input_error(control: &'static str, requested: &'static str) -> MechError {
    MechError::new(
        SpecializationInputAbsent {
            control: String::from(control),
            requested: String::from(requested),
        },
        None,
    )
    .with_compiler_loc()
}

fn empty_schema_table() -> Rc<SchemaTable> {
    Rc::new(
        SchemaTableBuilder::new()
            .finish()
            .expect("an empty schema table is valid")
            .table,
    )
}

#[doc(hidden)]
#[macro_export]
macro_rules! __mech_for_each_canonical_binop_factory_group {
    (
        $callback:path,
        $context:tt,
        $lib:ident,
        $scalar:ty,
        $scalar_name:literal,
        $scalar_token:ident;
        $cfg:meta;
        $($suffix:ident),+ $(,)?
    ) => {
        $(
            #[cfg($cfg)]
            $callback!($context, $lib, $suffix, $scalar, $scalar_name, $scalar_token);
        )+
    };
}

/// Enumerates the exact concrete binary factory surface for canonical source
/// specialization. This traversal deliberately lives outside the legacy
/// adapter so source execution and native registration share the same type
/// grid without requiring a legacy value projection.
#[macro_export]
macro_rules! for_each_canonical_binop_factory {
    ($callback:path, $context:tt, $lib:ident, $scalar:ty, $scalar_name:literal, $scalar_token:ident) => {
        $callback!($context, $lib, SS, $scalar, $scalar_name, $scalar_token);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix1"; SM1, M1S, M1M1);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2"; SM2, M2S, M2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3"; SM3, M3S, M3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix4"; SM4, M4S, M4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix2x3"; SM2x3, M2x3S, M2x3M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrix3x2"; SM3x2, M3x2S, M3x2M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "matrixd"; SMD, MDS, MDMD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector2"; SR2, R2S, R2R2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector3"; SR3, R3S, R3R3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vector4"; SR4, R4S, R4R4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "row_vectord"; SRD, RDS, RDRD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector2"; SV2, V2S, V2V2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector3"; SV3, V3S, V3V3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vector4"; SV4, V4S, V4V4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; feature = "vectord"; SVD, VDS, VDVD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "vector2"); M2V2, V2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "vector3"); M3V3, V3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "vector4"); M4V4, V4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "vector2"); M2x3V2, V2M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "vector3"); M3x2V3, V3M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vectord"); MDVD, VDMD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector2"); MDV2, V2MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector3"); MDV3, V3MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "vector4"); MDV4, V4MD);

        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2", feature = "row_vector2"); M2R2, R2M2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3", feature = "row_vector3"); M3R3, R3M3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix4", feature = "row_vector4"); M4R4, R4M4);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix2x3", feature = "row_vector3"); M2x3R3, R3M2x3);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrix3x2", feature = "row_vector2"); M3x2R2, R2M3x2);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vectord"); MDRD, RDMD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector2"); MDR2, R2MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector3"); MDR3, R3MD);
        $crate::__mech_for_each_canonical_binop_factory_group!($callback, $context, $lib, $scalar, $scalar_name, $scalar_token; all(feature = "matrixd", feature = "row_vector4"); MDR4, R4MD);
    };
}

#[cfg(all(test, feature = "f64"))]
mod tests {
    use super::*;
    use crate::{
        FunctionSpecializer, LegacyValue, MechFunction, MechFunctionImpl,
        specialization_invocation_from_legacy,
    };
    use std::sync::Arc;

    #[cfg(feature = "semantic-compiler")]
    use crate::{BytecodeCompilerContext, MechFunctionCompiler, Register};

    struct EchoFunction {
        output: Ref<f64>,
    }

    impl MechFunctionImpl for EchoFunction {
        fn solve_result(&self) -> MResult<()> {
            Ok(())
        }

        fn primary_output_state_port(&self) -> Option<crate::FunctionStatePort<'_>> {
            Some(crate::FunctionStatePort::from_ref(&self.output))
        }

        fn to_string(&self) -> String {
            String::from("EchoFunction")
        }
    }

    #[cfg(feature = "semantic-compiler")]
    impl MechFunctionCompiler for EchoFunction {
        fn compile(&self, _context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
            Ok(0)
        }
    }

    struct EchoSpecializer;

    impl FunctionSpecializer for EchoSpecializer {
        fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            let [LegacyValue::F64(output)] = arguments else {
                panic!("test specialization receives one f64 input")
            };
            Ok(Box::new(EchoFunction {
                output: output.clone(),
            }))
        }
    }

    #[test]
    fn inputs_preserve_exact_cells_and_keep_absence_out_of_runtime_values() {
        let source = Ref::new(7.5);
        let cell = ValueCell::from_inferred_ref(source.clone(), None).unwrap();
        let input = SpecializationInput::Cell(cell.clone());

        assert!(input.try_ref::<f64>().unwrap().same_handle(&source));
        assert_eq!(input.schema_key(), Some(cell.schema_key()));
        assert_eq!(
            input.closed_schema_body().unwrap(),
            Some(cell.closed_schema_body().unwrap())
        );
        assert_eq!(input.shape(), Some(cell.shape().clone()));
        assert_eq!(
            input.representation(),
            Some(FunctionValueRepresentation::F64)
        );
        assert!(matches!(
            input.snapshot().unwrap().data(),
            crate::ValueData::F64(_)
        ));

        let absent = SpecializationInput::Absent;
        assert!(absent.is_absent());
        assert_eq!(absent.schema_key(), None);
        assert_eq!(absent.closed_schema_body().unwrap(), None);
        assert_eq!(absent.shape(), None);
        assert!(absent.snapshot().is_err());
    }

    #[cfg(all(feature = "matrix", feature = "matrix2"))]
    #[test]
    fn matrix_inputs_retain_the_exact_wrapper_and_inner_handle() {
        let source = Ref::new(crate::Matrix2::new(1.0, 2.0, 3.0, 4.0));
        let cell = ValueCell::from_inferred_ref(source.clone(), Some((2, 2))).unwrap();
        let input = SpecializationInput::Cell(cell);

        let Matrix::Matrix2(actual) = input.try_matrix::<f64>(0).unwrap() else {
            panic!("the exact Matrix2 representation must be retained")
        };
        assert!(actual.same_handle(&source));

        let scalar = SpecializationInput::Cell(ValueCell::from_exact(1.0_f64).unwrap());
        let error = scalar.try_matrix::<f64>(2).unwrap_err();
        assert_eq!(
            error
                .kind_as::<crate::FunctionArgumentTypeMismatch>()
                .unwrap()
                .role,
            FunctionArgumentRole::Input(2)
        );
    }

    #[test]
    fn context_constructs_typed_and_canonical_cells_in_its_schema_table() {
        let source = Ref::new(3.0);
        let input_cell = ValueCell::from_inferred_ref(source, None).unwrap();
        let invocation =
            SpecializationInvocation::from_cells(vec![input_cell.clone()].into_boxed_slice());
        let context = SpecializationContext::for_invocation(&invocation, None).unwrap();
        let replacement = Ref::new(9.0);
        let typed = context
            .typed_cell(
                replacement.clone(),
                input_cell.schema_key(),
                input_cell.shape().clone(),
            )
            .unwrap();
        assert!(typed.try_ref::<f64>().unwrap().same_handle(&replacement));

        let snapshot = input_cell.snapshot().unwrap();
        let canonical = context.value_cell(snapshot).unwrap();
        assert_eq!(canonical.schema_key(), input_cell.schema_key());
        assert_eq!(
            canonical.shape().parameter_values(),
            input_cell.shape().parameter_values()
        );
    }

    #[cfg(feature = "bool")]
    #[test]
    fn invocation_context_merges_independent_schema_tables_and_rebinds_values() {
        let scalar = ValueCell::from_exact(3.0_f64).unwrap();
        let flag = ValueCell::from_exact(true).unwrap();
        let invocation = SpecializationInvocation::from_cells(
            vec![scalar.clone(), flag.clone()].into_boxed_slice(),
        );
        let context = SpecializationContext::for_invocation(&invocation, None).unwrap();

        assert_eq!(scalar.schema(), flag.schema());
        assert_eq!(context.schemas().len(), 2);
        assert!(context.schemas().find_by_key(scalar.schema_key()).is_some());
        assert!(context.schemas().find_by_key(flag.schema_key()).is_some());
        assert_eq!(
            context.schema(scalar.schema_key()).unwrap().body(),
            &SchemaBody::FloatingPoint(crate::FloatWidth::W64)
        );
        assert_eq!(
            context.schema(flag.schema_key()).unwrap().body(),
            &SchemaBody::Bool
        );

        let rebound = context.value_cell(flag.snapshot().unwrap()).unwrap();
        assert_eq!(rebound.schema_key(), flag.schema_key());
        assert_eq!(*rebound.try_ref::<bool>().unwrap().borrow(), true);

        let reversed = SpecializationInvocation::from_cells(
            vec![flag.clone(), scalar.clone()].into_boxed_slice(),
        );
        let reversed = SpecializationContext::for_invocation(&reversed, None).unwrap();
        assert_eq!(
            reversed.schema(scalar.schema_key()).unwrap().body(),
            &SchemaBody::FloatingPoint(crate::FloatWidth::W64)
        );
        assert_eq!(
            reversed.schema(flag.schema_key()).unwrap().body(),
            &SchemaBody::Bool
        );
    }

    #[test]
    fn legacy_adapter_binds_the_specialized_output_without_rediscovery_by_callers() {
        let source = Ref::new(11.0);
        let invocation =
            specialization_invocation_from_legacy(&[LegacyValue::F64(source.clone())]).unwrap();
        let mut context = SpecializationContext::for_invocation(&invocation, None).unwrap();
        let specializer = crate::canonical_function_specializer(Arc::new(EchoSpecializer));
        let specialized = specializer
            .specialize_invocation(&invocation, &mut context)
            .unwrap();

        assert!(
            specialized
                .output()
                .try_ref::<f64>()
                .unwrap()
                .same_handle(&source)
        );
        assert_eq!(specialized.instance().inputs().len(), 1);
        assert!(specialized.instance().inputs()[0].same_cell(specialized.output()));
    }

    #[test]
    fn legacy_mutable_specialization_inputs_retain_the_inner_typed_cell() {
        let source = Ref::new(11.0);
        let wrapper = Ref::new(LegacyValue::F64(source.clone()));
        let invocation = specialization_invocation_from_legacy(&[
            LegacyValue::MutableReference(wrapper),
            LegacyValue::F64(Ref::new(2.0)),
        ])
        .unwrap();

        let sink = invocation.input(0).unwrap().cell().unwrap();
        assert_eq!(sink.representation(), FunctionValueRepresentation::F64);
        assert!(sink.try_ref::<f64>().unwrap().same_handle(&source));
    }

    #[test]
    fn legacy_all_selection_is_not_materialized_as_a_runtime_specialization_input() {
        let error = specialization_invocation_from_legacy(&[LegacyValue::IndexAll]).unwrap_err();
        assert!(error.simple_message().contains("matrix-all-selection"));
    }

    #[test]
    fn option_absence_is_a_canonical_cell_not_source_absence_or_unit() {
        let invocation = specialization_invocation_from_legacy(&[LegacyValue::EmptyKind(
            crate::ValueKind::Option(Box::new(crate::ValueKind::F64)),
        )])
        .unwrap();
        let input = invocation.input(0).unwrap();
        assert!(!input.is_absent());
        assert!(matches!(
            input.snapshot().unwrap().data(),
            crate::ValueData::Option(None)
        ));

        let unit = ValueCell::unit().snapshot().unwrap();
        assert!(matches!(unit.data(), crate::ValueData::Tuple(values) if values.is_empty()));
        assert_ne!(input.snapshot().unwrap().schema_key(), unit.schema_key());
    }
}

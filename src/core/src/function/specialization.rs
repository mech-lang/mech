#[cfg(feature = "no_std")]
use alloc::{boxed::Box, rc::Rc, string::String, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, rc::Rc, string::String, vec::Vec};

use crate::{
    ConversionPlan, DimensionExpr, FunctionCatalog, FunctionInstance, FunctionInvocation,
    FunctionPortBacking, FunctionValueRepresentation, MResult, MechError, MechErrorKind,
    MechFunctionFactory, Ref, ResolvedType, RuntimeFunctionInputs, Schema, SchemaBody, SchemaKey,
    SchemaTable, SchemaTableBuilder, ShapeInstance, Value, ValueCell,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedCall {
    pub overload_id: u32,
    pub original_inputs: Box<[ResolvedType]>,
    pub converted_inputs: Box<[ResolvedType]>,
    pub input_conversions: Box<[ConversionPlan]>,
    pub outputs: Box<[ResolvedType]>,
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
    semantic_operation: Option<String>,
    resolved_call: Option<ResolvedCall>,
}

impl<'a> SpecializationContext<'a> {
    pub fn new(schemas: Rc<SchemaTable>) -> Self {
        Self {
            schemas,
            catalog: None,
            semantic_operation: None,
            resolved_call: None,
        }
    }

    pub fn with_catalog(schemas: Rc<SchemaTable>, catalog: &'a FunctionCatalog) -> Self {
        Self {
            schemas,
            catalog: Some(catalog),
            semantic_operation: None,
            resolved_call: None,
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
        Ok(Self {
            schemas,
            catalog,
            semantic_operation: None,
            resolved_call: None,
        })
    }

    pub fn for_resolved_invocation(
        invocation: &SpecializationInvocation,
        catalog: Option<&'a FunctionCatalog>,
        semantic_operation: impl Into<String>,
        resolved_call: ResolvedCall,
    ) -> MResult<Self> {
        let mut context = Self::for_invocation(invocation, catalog)?;
        context.semantic_operation = Some(semantic_operation.into());
        context.resolved_call = Some(resolved_call);
        Ok(context)
    }

    pub fn schemas(&self) -> &SchemaTable {
        self.schemas.as_ref()
    }

    pub fn catalog(&self) -> Option<&FunctionCatalog> {
        self.catalog
    }

    pub fn resolved_call(&self) -> MResult<&ResolvedCall> {
        self.resolved_call.as_ref().ok_or_else(|| {
            MechError::new(
                SpecializationSemanticCallUnavailable {
                    semantic_operation: self
                        .semantic_operation
                        .clone()
                        .unwrap_or_else(|| "named operation".into()),
                },
                None,
            )
            .with_compiler_loc()
        })
    }

    pub fn resolved_input(&self, index: usize) -> MResult<&ResolvedType> {
        self.resolved_call()?
            .converted_inputs
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "input", index))
    }

    pub fn resolved_output(&self, index: usize) -> MResult<&ResolvedType> {
        self.resolved_call()?
            .outputs
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "output", index))
    }

    pub fn input_conversion(&self, index: usize) -> MResult<&ConversionPlan> {
        self.resolved_call()?
            .input_conversions
            .get(index)
            .ok_or_else(|| resolved_call_index_error(self, "input conversion", index))
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

    fn semantic_binding_inputs(&self) -> MResult<Box<[String]>> {
        Ok(self
            .resolved_call()?
            .converted_inputs
            .iter()
            .map(ResolvedType::semantic_name)
            .collect::<Vec<_>>()
            .into_boxed_slice())
    }

    /// Selects and binds one exact runtime factory from canonical input and
    /// output representations.
    ///
    /// This is the shared source/runtime seam for operation families whose
    /// concrete factories are already registered in the catalog. The caller
    /// derives output representation and extent from operation semantics;
    /// this method never projects a cell through an erased universal value.
    pub fn bind_runtime_factory(
        &self,
        name_prefix: &str,
        output_representation: FunctionValueRepresentation,
        output_dimensions: Option<(usize, usize)>,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let resolved_output = self.resolved_output(0)?;
        let semantic_operation = self
            .semantic_operation
            .clone()
            .unwrap_or_else(|| name_prefix.into());
        let semantic_output = resolved_output.semantic_name();
        let semantic_inputs = self.semantic_binding_inputs()?;
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
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
                && representation_supports_resolved_type(entry.signature().output, resolved_output)
                && runtime_inputs_match(entry.signature().inputs, input_representations.as_slice())
        });
        let entry = candidates.next().ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if candidates.next().is_some() {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc());
        }
        let output =
            ValueCell::default_for_representation(output_representation, output_dimensions)?
                .with_resolved_output_type(resolved_output)?;
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
        let resolved_output = self.resolved_output(0)?;
        let semantic_operation = self
            .semantic_operation
            .clone()
            .unwrap_or_else(|| name_prefix.into());
        let semantic_output = resolved_output.semantic_name();
        let semantic_inputs = self.semantic_binding_inputs()?;
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
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
                    && representation_supports_resolved_type(
                        entry.signature().output,
                        resolved_output,
                    )
                    && runtime_inputs_match(
                        entry.signature().inputs,
                        input_representations.as_slice(),
                    )
            })
            .map(|entry| (runtime_output_rank(entry.signature().output), entry))
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(rank, entry)| (*rank, entry.id));
        let Some((best_rank, entry)) = candidates.into_iter().next() else {
            return Err(MechError::new(
                SpecializationRuntimeFactoryUnavailable {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc());
        };
        let competing = catalog.runtime_entries().find(|candidate| {
            candidate.id != entry.id
                && candidate.name.starts_with(name_prefix)
                && representation_supports_resolved_type(
                    candidate.signature().output,
                    resolved_output,
                )
                && runtime_inputs_match(
                    candidate.signature().inputs,
                    input_representations.as_slice(),
                )
                && runtime_output_rank(candidate.signature().output) == best_rank
        });
        if competing.is_some() {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc());
        }
        let dimensions = output_dimensions
            .or_else(|| inferred_output_dimensions(entry.signature().output, inputs));
        let output = ValueCell::default_for_representation(entry.signature().output, dimensions)?
            .with_resolved_output_type(resolved_output)?;
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
    /// projected into, or reconstructed from, an erased universal value.
    pub fn bind_runtime_factory_existing_output(
        &self,
        name_prefix: &str,
        output: &SpecializationInput,
        inputs: &[&SpecializationInput],
    ) -> MResult<SpecializedFunction> {
        let resolved_output = self.resolved_output(0)?;
        let semantic_operation = self
            .semantic_operation
            .clone()
            .unwrap_or_else(|| name_prefix.into());
        let semantic_output = resolved_output.semantic_name();
        let semantic_inputs = self.semantic_binding_inputs()?;
        let catalog = self.catalog.ok_or_else(|| {
            MechError::new(
                SpecializationRuntimeCatalogUnavailable {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let output_representation = output.representation().ok_or_else(|| {
            control_input_error("non-value", "runtime factory output representation")
        })?;
        let live_output = output.cell()?.resolved_type()?;
        if !crate::exact_type_equal(&live_output, resolved_output) {
            return Err(MechError::from(crate::TypeResolutionError::incompatible(
                self.semantic_operation
                    .clone()
                    .unwrap_or_else(|| name_prefix.into()),
                crate::TypeConstraintFailure::OutputTypeMismatch {
                    expected: resolved_output.semantic_name(),
                    actual: live_output.semantic_name(),
                },
            )));
        }
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
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
                },
                None,
            )
            .with_compiler_loc()
        })?;
        if candidates.next().is_some() {
            return Err(MechError::new(
                SpecializationRuntimeFactoryAmbiguous {
                    semantic_operation: semantic_operation.clone(),
                    semantic_inputs: semantic_inputs.clone(),
                    semantic_output: semantic_output.clone(),
                    execution_profile: "direct runtime",
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

fn representation_supports_resolved_type(
    representation: FunctionValueRepresentation,
    resolved: &ResolvedType,
) -> bool {
    use crate::{BuiltinScalarKind as Scalar, FunctionMatrixElement as Element, KindExpr};
    use FunctionValueRepresentation as Representation;

    match (representation, resolved.kind()) {
        (Representation::AnyValue, _) | (_, KindExpr::Wildcard) => true,
        (Representation::U8, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U8),
        (Representation::U16, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::U16)
        }
        (Representation::U32, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::U32)
        }
        (Representation::U64, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::U64)
        }
        (Representation::U128, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::U128)
        }
        (Representation::I8, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I8),
        (Representation::I16, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::I16)
        }
        (Representation::I32, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::I32)
        }
        (Representation::I64, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::I64)
        }
        (Representation::I128, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::I128)
        }
        (Representation::F32, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::F32)
        }
        (Representation::F64, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::F64)
        }
        (Representation::C64, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::C64)
        }
        (Representation::R64, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::R64)
        }
        (Representation::String, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::String)
        }
        (Representation::Bool, KindExpr::Named(id)) => {
            Scalar::from_kind_id(*id) == Some(Scalar::Bool)
        }
        (Representation::Id, KindExpr::Id) | (Representation::Index, KindExpr::Index) => true,
        (Representation::Atom, KindExpr::Atom(_)) | (Representation::Enum, KindExpr::Enum(_)) => {
            true
        }
        (Representation::Record, KindExpr::Record(_))
        | (Representation::Map, KindExpr::Map { .. })
        | (Representation::Set, KindExpr::Set { .. })
        | (Representation::Table, KindExpr::Table { .. })
        | (Representation::Tuple, KindExpr::Tuple(_))
        | (Representation::Kind, KindExpr::TypeOf(_)) => true,
        (
            Representation::Matrix { element, .. },
            KindExpr::Matrix {
                element: resolved_element,
                ..
            },
        ) => match (element, resolved_element.as_ref()) {
            (Element::Value, _) => true,
            (Element::Index, KindExpr::Index) => true,
            (Element::Bool, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::Bool),
            (Element::String, KindExpr::Named(id)) => {
                Scalar::from_kind_id(*id) == Some(Scalar::String)
            }
            (Element::U8, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U8),
            (Element::U16, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U16),
            (Element::U32, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U32),
            (Element::U64, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U64),
            (Element::U128, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::U128),
            (Element::I8, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I8),
            (Element::I16, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I16),
            (Element::I32, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I32),
            (Element::I64, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I64),
            (Element::I128, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::I128),
            (Element::F32, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::F32),
            (Element::F64, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::F64),
            (Element::C64, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::C64),
            (Element::R64, KindExpr::Named(id)) => Scalar::from_kind_id(*id) == Some(Scalar::R64),
            _ => false,
        },
        (Representation::Empty, KindExpr::Tuple(elements)) => elements.is_empty(),
        _ => false,
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
pub struct SpecializationSemanticCallUnavailable {
    pub semantic_operation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationResolvedCallIndexUnavailable {
    pub semantic_operation: String,
    pub category: &'static str,
    pub index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeCatalogUnavailable {
    pub semantic_operation: String,
    pub semantic_inputs: Box<[String]>,
    pub semantic_output: String,
    pub execution_profile: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryUnavailable {
    pub semantic_operation: String,
    pub semantic_inputs: Box<[String]>,
    pub semantic_output: String,
    pub execution_profile: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpecializationRuntimeFactoryAmbiguous {
    pub semantic_operation: String,
    pub semantic_inputs: Box<[String]>,
    pub semantic_output: String,
    pub execution_profile: &'static str,
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

impl MechErrorKind for SpecializationSemanticCallUnavailable {
    fn name(&self) -> &str {
        "SpecializationSemanticCallUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "named operation {:?} reached physical specialization without a resolved semantic call",
            self.semantic_operation,
        )
    }
}

impl MechErrorKind for SpecializationResolvedCallIndexUnavailable {
    fn name(&self) -> &str {
        "SpecializationResolvedCallIndexUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "named operation {:?} has no resolved {} at index {}",
            self.semantic_operation, self.category, self.index,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeCatalogUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeCatalogUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` with inputs {:?} and output {} cannot bind for the {} execution profile because its runtime catalog is unavailable",
            self.semantic_operation,
            self.semantic_inputs,
            self.semantic_output,
            self.execution_profile,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryUnavailable {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` with inputs {:?} and output {} is unavailable for the {} execution profile",
            self.semantic_operation,
            self.semantic_inputs,
            self.semantic_output,
            self.execution_profile,
        )
    }
}

impl MechErrorKind for SpecializationRuntimeFactoryAmbiguous {
    fn name(&self) -> &str {
        "SpecializationRuntimeFactoryAmbiguous"
    }

    fn message(&self) -> String {
        format!(
            "semantic operation `{}` with inputs {:?} and output {} has more than one binding for the {} execution profile",
            self.semantic_operation,
            self.semantic_inputs,
            self.semantic_output,
            self.execution_profile,
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

fn resolved_call_index_error(
    context: &SpecializationContext<'_>,
    category: &'static str,
    index: usize,
) -> MechError {
    MechError::new(
        SpecializationResolvedCallIndexUnavailable {
            semantic_operation: context
                .semantic_operation
                .clone()
                .unwrap_or_else(|| "named operation".into()),
            category,
            index,
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
/// specialization. Source execution and native registration share this exact
/// type grid without requiring an erased value projection.
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

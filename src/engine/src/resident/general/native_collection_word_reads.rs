//! A selected scalar-word read from a genuine immutable canonical matrix.
//! No selector/result Value, temporary vector, or runtime call plan is built.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::NativeScalarKind as Kind;
use crate::{
    native_artifact::NativeCollectionPrimitive as Primitive,
    resident::budget::{
        NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
    },
};
use mech_core::{
    IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData,
    snapshot::SequenceView,
};

impl NativePreparedLibraryCall {
    /// Exact typed reads reuse the admitted scalar-word boundary. No canonical
    /// scalar owner or unsigned view is invented for Bool/I64/F64 storage.
    pub fn scalar_matrix_element_kind(&self) -> Option<Kind> {
        let inputs = self.template.inputs();
        let matrix = inputs.first()?;
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = matrix.body()
        else {
            return None;
        };
        let kind = Kind::from_schema_body(element)?;
        (dimensions.len() == 2
            && self.template.output().body() == element.as_ref()
            && self.template.output().dimension_parameters().is_empty()
            && matches!(
                (self.template.primitive(), inputs.len()),
                (Primitive::MatrixScalarAccess, 2) | (Primitive::MatrixCellAccess, 3)
            )
            && inputs[1..].iter().all(|selector| {
                selector.body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    && selector.dimension_parameters().is_empty()
            }))
        .then_some(kind)
    }

    /// Selected rank-two transpose for exact plain native scalar storage.
    /// Geometry, finite coexistence, source construction and publication are
    /// separate caller proofs; this method grants no owner or permit.
    pub fn supports_numeric_matrix_transpose(&self) -> bool {
        let [input] = self.template.inputs() else {
            return false;
        };
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = input.body()
        else {
            return false;
        };
        let SchemaBody::Matrix {
            element: output,
            dimensions: output_dimensions,
        } = self.template.output().body()
        else {
            return false;
        };
        self.template.primitive() == Primitive::MatrixTranspose
            && dimensions.len() == 2
            && output_dimensions.len() == 2
            && element == output
            && Kind::from_schema_body(element).is_some()
    }

    /// A selected ordered row reduction. The caller separately proves current
    /// one-row geometry, exact output shape/order and complete finite admission.
    pub fn supports_f64_row_sum(&self) -> bool {
        let f64_matrix = |schema: &mech_core::Schema| {
            matches!(schema.body(),
            SchemaBody::Matrix { element, dimensions } if dimensions.len() == 2
                && element.as_ref() == &SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
        };
        self.template.primitive() == Primitive::SumColumns
            && self.template.inputs().len() == 1
            && f64_matrix(&self.template.inputs()[0])
            && f64_matrix(self.template.output())
    }

    pub fn supports_f64_matrix_element(&self) -> bool {
        self.scalar_matrix_element_kind() == Some(Kind::F64)
    }

    pub fn f64_matrix_element(
        &self,
        source: &Value,
        indices: &[u64],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_f64_matrix_element() || indices.len() + 1 != self.template.inputs().len()
        {
            return Err(InvokeError::InvalidInput);
        }
        self.matrix_element_word(source, indices, Kind::F64, limits, work, coexistence)
    }

    pub fn scalar_matrix_element(
        &self,
        source: &Value,
        indices: &[u64],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let kind = self
            .scalar_matrix_element_kind()
            .ok_or(InvokeError::InvalidInput)?;
        if indices.len() + 1 != self.template.inputs().len() {
            return Err(InvokeError::InvalidInput);
        }
        self.matrix_element_word(source, indices, kind, limits, work, coexistence)
    }

    fn unsigned_matrix_element_width(&self) -> Option<IntegerWidth> {
        let [matrix, selector] = self.template.inputs() else {
            return None;
        };
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = matrix.body()
        else {
            return None;
        };
        let SchemaBody::UnsignedInteger(width) = element.as_ref() else {
            return None;
        };
        if !matches!(
            width,
            IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64
        ) || dimensions.len() != 2
            || self.template.primitive() != Primitive::MatrixScalarAccess
            || selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !selector.dimension_parameters().is_empty()
            || self.template.output().body() != element.as_ref()
            || !self.template.output().dimension_parameters().is_empty()
        {
            return None;
        }
        Some(*width)
    }

    /// Only the privately witnessed maintained linear scalar access qualifies.
    pub fn supports_unsigned_matrix_element(&self) -> bool {
        self.unsigned_matrix_element_width().is_some()
    }

    /// Compatibility with the original exact U64 projection contract.
    pub fn supports_u64_matrix_element(&self) -> bool {
        self.unsigned_matrix_element_width() == Some(IntegerWidth::W64)
    }

    pub fn u64_matrix_element(
        &self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_u64_matrix_element() {
            return Err(InvokeError::InvalidInput);
        }
        self.unsigned_matrix_element(source, index, limits, work, coexistence)
    }

    /// Return the scalar word into already admitted native storage. The caller
    /// supplies its complete current owner/caller census and original visit.
    /// This pure read grants no visit, loop, output-publication, or owner permit.
    pub fn unsigned_matrix_element(
        &self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let width = self
            .unsigned_matrix_element_width()
            .ok_or(InvokeError::InvalidInput)?;
        let kind = match width {
            IntegerWidth::W8 => Kind::U8,
            IntegerWidth::W16 => Kind::U16,
            IntegerWidth::W32 => Kind::U32,
            IntegerWidth::W64 => Kind::U64,
            _ => return Err(InvokeError::InvalidInput),
        };
        self.matrix_element_word(source, &[index], kind, limits, work, coexistence)
    }

    fn matrix_element_word(
        &self,
        source: &Value,
        indices: &[u64],
        kind: Kind,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        let output_bytes = match kind {
            Kind::Bool | Kind::U8 => 1,
            Kind::U16 => 2,
            Kind::U32 => 4,
            Kind::U64 | Kind::I64 | Kind::F64 => 8,
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                // Include the result even though its physical scalar lane already
                // belongs to the native frame. No new canonical owner is invented.
                coexistence.output_elements = coexistence
                    .output_elements
                    .checked_add(1)
                    .ok_or(InvokeError::InvalidInput)?;
                // Logical output limits still describe the exact scalar width;
                // absence of a new allocation does not waive those limits.
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    output_bytes,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(self.lookup_work, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || source.shape().parameter_values().len()
                        != self.template.inputs()[0].dimension_parameters().len()
                {
                    return Err(InvokeError::InvalidInput);
                }
                source
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let memory = work.memory_budget();
                let bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let nodes = source
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                // Call-scoped coexistence includes the actual immutable owner
                // and two already admitted scalar words, not unrelated roots
                // elsewhere in the program's separate shared memory account.
                let context = context.with_added_capacity(
                    bytes
                        .checked_add((indices.len() as u64 + 1) * 8)
                        .ok_or(InvokeError::InvalidInput)?,
                    nodes
                        .checked_add(indices.len() as u64 + 1)
                        .ok_or(InvokeError::InvalidInput)?,
                )?;
                context.check_capacity(0, 0)?;
                context.charge(128, 0)?;
                let ValueData::Matrix(matrix) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let cells = matrix.elements();
                // Reuse the maintained linear scalar selector's one-based check
                // and error, including zero, empty input, and oversized ordinals.
                let ordinal = match indices {
                    [index] => {
                        let ordinal =
                            crate::resident::numeric::available_scalar_index(*index, cells.len())?;
                        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                            source,
                            self.template.inputs()[0].body(),
                        )?;
                        // Linear selection is column-major; canonical cells are row-major.
                        if rows == 1 || columns == 1 {
                            ordinal
                        } else {
                            (ordinal % rows) * columns + ordinal / rows
                        }
                    }
                    [row, column] => {
                        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                            source,
                            self.template.inputs()[0].body(),
                        )?;
                        let row = crate::resident::numeric::available_scalar_index(*row, rows)?;
                        let column =
                            crate::resident::numeric::available_scalar_index(*column, columns)?;
                        context.charge(16, 0)?;
                        row.checked_mul(columns)
                            .and_then(|index| index.checked_add(column))
                            .ok_or(InvokeError::InvalidInput)?
                    }
                    _ => return Err(InvokeError::InvalidInput),
                };
                match (kind, cells) {
                    (Kind::U8, SequenceView::U8(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::U16, SequenceView::U16(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::U32, SequenceView::U32(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::U64, SequenceView::U64(values)) => values
                        .get(ordinal)
                        .copied()
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::F64, SequenceView::F64(values)) => values
                        .get(ordinal)
                        .map(|value| value.bits())
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::I64, SequenceView::I64(values)) => values
                        .get(ordinal)
                        .map(|value| *value as u64)
                        .ok_or(InvokeError::InvalidInput),
                    (Kind::Bool, SequenceView::Bool(values)) => values
                        .get(ordinal)
                        .map(|value| u64::from(*value))
                        .ok_or(InvokeError::InvalidInput),
                    (kind, SequenceView::Values(values)) => match (kind, values.get(ordinal)) {
                        (Kind::U8, Some(ValueData::U8(value))) => Ok(u64::from(*value)),
                        (Kind::U16, Some(ValueData::U16(value))) => Ok(u64::from(*value)),
                        (Kind::U32, Some(ValueData::U32(value))) => Ok(u64::from(*value)),
                        (Kind::U64, Some(ValueData::U64(value))) => Ok(*value),
                        (Kind::I64, Some(ValueData::I64(value))) => Ok(*value as u64),
                        (Kind::Bool, Some(ValueData::Bool(value))) => Ok(u64::from(*value)),
                        (Kind::F64, Some(ValueData::F64(value))) => Ok(value.bits()),
                        _ => Err(InvokeError::InvalidInput),
                    },
                    _ => Err(InvokeError::InvalidInput),
                }
            },
        )?
    }
}

impl NativePreparedLibraryCall {
    /// Fixed logical debit of the maintained typed scalar read. A fused
    /// consumer must also fund its own physical work, current-owner census,
    /// temporary coexistence and fallback before borrowing any native view.
    pub fn scalar_matrix_element_native_work(&self) -> Option<u64> {
        self.scalar_matrix_element_kind()?;
        self.lookup_work
            .checked_add(128)?
            .checked_add(if self.template.inputs().len() == 3 {
                16
            } else {
                0
            })
    }

    /// Borrow canonical numeric storage through this exact selected read.
    /// This grants no visit, work credit, mutable access or publication. The
    /// caller retains the current owner for the entire synchronous native loan.
    /// The established unsigned buffer view keeps its narrower contract.
    pub fn scalar_matrix_element_native_view(
        &self,
        source: &Value,
        memory: &mech_core::ManagedMemoryBudget,
    ) -> Option<[u64; 4]> {
        self.scalar_matrix_element_kind()?;
        self.scalar_matrix_native_view(source, memory)
    }

    /// Current exact scalar storage for a selected read or replacement factory.
    /// This is an immutable synchronous view; it grants no execution, mutation,
    /// ownership handoff or publication authority. Unsigned growth/length uses
    /// the maintained narrower view and its own entry contract.
    pub fn scalar_matrix_native_view(
        &self,
        source: &Value,
        memory: &mech_core::ManagedMemoryBudget,
    ) -> Option<[u64; 4]> {
        let kind = self.scalar_matrix_element_kind().or_else(|| {
            #[cfg(feature = "sequence_buffers")]
            {
                self.scalar_matrix_replace_kind()
            }
            #[cfg(not(feature = "sequence_buffers"))]
            {
                None
            }
        });
        let Some(kind) = kind else {
            return self.unsigned_matrix_native_view(source, memory);
        };
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return None;
        };
        if source.schema() != contract.inputs[0].schema
            || source.schema_key() != self.input_keys[0]
            || source.shape().parameter_values().len()
                != self.template.inputs()[0].dimension_parameters().len()
        {
            return None;
        }
        source.validate_against(&self.schemas).ok()?;
        source.memory_budget_retained_bytes(memory)?;
        source.memory_budget_retained_nodes(memory)?;
        let ValueData::Matrix(matrix) = source.data() else {
            return None;
        };
        let cells = matrix.elements();
        let pointer = match (kind, cells) {
            (Kind::Bool, SequenceView::Bool(xs)) => xs.as_ptr() as u64,
            (Kind::U8, SequenceView::U8(xs)) => xs.as_ptr() as u64,
            (Kind::U16, SequenceView::U16(xs)) => xs.as_ptr() as u64,
            (Kind::U32, SequenceView::U32(xs)) => xs.as_ptr() as u64,
            (Kind::U64, SequenceView::U64(xs)) => xs.as_ptr() as u64,
            (Kind::I64, SequenceView::I64(xs)) => xs.as_ptr() as u64,
            (Kind::F64, SequenceView::F64(xs)) => xs.as_ptr() as u64,
            _ => return None,
        };
        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
            source,
            self.template.inputs()[0].body(),
        )
        .ok()?;
        Some([pointer, cells.len() as u64, rows as u64, columns as u64])
    }

    /// Derive a borrowed packed view only from this original selected read/length
    /// factory and an immutable value admitted to the caller's SAME account.
    /// No execution permit is granted. The synchronous caller must retain the
    /// value, forbid mutation/callbacks while loading, and discard the view on
    /// every invocation exit. Nonpacked storage conservatively keeps its route.
    pub fn unsigned_matrix_native_view(
        &self,
        source: &Value,
        memory: &mech_core::ManagedMemoryBudget,
    ) -> Option<[u64; 4]> {
        if !(self.supports_unsigned_matrix_element()
            || {
                #[cfg(feature = "pure_text")]
                {
                    self.supports_unsigned_matrix_length()
                }
                #[cfg(not(feature = "pure_text"))]
                {
                    false
                }
            }
            || {
                #[cfg(feature = "sequence_buffers")]
                {
                    self.supports_unsigned_matrix_replace()
                        || self.supports_unsigned_matrix_append()
                        || self.supports_unsigned_matrix_append_le_bytes()
                }
                #[cfg(not(feature = "sequence_buffers"))]
                {
                    false
                }
            })
        {
            return None;
        }
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return None;
        };
        if source.schema() != contract.inputs[0].schema
            || source.schema_key() != self.input_keys[0]
            || source.shape().parameter_values().len()
                != self.template.inputs()[0].dimension_parameters().len()
        {
            return None;
        }
        source.validate_against(&self.schemas).ok()?;
        source.memory_budget_retained_bytes(memory)?;
        source.memory_budget_retained_nodes(memory)?;
        let ValueData::Matrix(matrix) = source.data() else {
            return None;
        };
        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
            source,
            self.template.inputs()[0].body(),
        )
        .ok()?;
        let cells = matrix.elements();
        let address = match cells {
            SequenceView::U8(values) => values.as_ptr() as u64,
            SequenceView::U16(values) => values.as_ptr() as u64,
            SequenceView::U32(values) => values.as_ptr() as u64,
            SequenceView::U64(values) => values.as_ptr() as u64,
            _ => return None,
        };
        Some([address, cells.len() as u64, rows as u64, columns as u64])
    }
}

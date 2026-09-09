use crate::intrinsics::*;
use nalgebra::{
    Dim, IsContiguous, Scalar,
    base::{Matrix as naMatrix, Storage, StorageMut},
};
use std::fmt::Debug;
use std::marker::PhantomData;

macro_rules! optional_operation_contract {
    () => {
        None
    };
    ($contract:path) => {
        Some(&*$contract)
    };
}
use std::sync::LazyLock;

fn assignment_source_out_of_bounds(required: usize, actual: usize) -> MechError {
    function_shape_contract_violation(
        "assign_slice",
        format!(
            "reactive assignment selector requires source offset {required}, but the source has {actual} elements"
        ),
    )
}

fn require_assignment_source_index(actual: usize, index: usize) -> MResult<()> {
    if index >= actual {
        return Err(assignment_source_out_of_bounds(index, actual));
    }
    Ok(())
}

fn require_assignment_selector_len(axis: &str, actual: usize, expected: usize) -> MResult<()> {
    if actual != expected {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment {axis} selector has length {actual}, but the sink requires {expected}"
            ),
        ));
    }
    Ok(())
}

fn require_assignment_index(axis: &str, index: usize, extent: usize) -> MResult<()> {
    if index == 0 || index > extent {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment {axis} index {index} is outside the one-based sink extent 1..={extent}"
            ),
        ));
    }
    Ok(())
}

fn require_assignment_source_layout(
    source_rows: usize,
    source_columns: usize,
    required_rows: usize,
    required_columns: usize,
    broadcast_rows: bool,
    broadcast_columns: bool,
) -> MResult<()> {
    let rows_valid = source_rows >= required_rows || (broadcast_rows && source_rows == 1);
    let columns_valid =
        source_columns >= required_columns || (broadcast_columns && source_columns == 1);
    if !rows_valid || !columns_valid {
        return Err(function_shape_contract_violation(
            "assign_slice",
            format!(
                "reactive assignment selector requires source layout {required_rows}x{required_columns}{}{}, but the source is {source_rows}x{source_columns}",
                if broadcast_rows {
                    " or one broadcast row"
                } else {
                    ""
                },
                if broadcast_columns {
                    " or one broadcast column"
                } else {
                    ""
                },
            ),
        ));
    }
    Ok(())
}

static PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::SingleElement,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::RectangularRegion,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ManagedMatrixAssignmentMode {
    Linear,
    Rows,
    Columns,
    WholeIf,
}

macro_rules! managed_assignment_mode {
    (assign_1d_scalar) => {
        ManagedMatrixAssignmentMode::Linear
    };
    (assign_1d_scalar_b) => {
        ManagedMatrixAssignmentMode::WholeIf
    };
    (set_1d_range) => {
        ManagedMatrixAssignmentMode::Linear
    };
    (set_1d_range_b) => {
        ManagedMatrixAssignmentMode::Linear
    };
    (set_1d_range_vec) => {
        ManagedMatrixAssignmentMode::Linear
    };
    (set_1d_range_vec_b) => {
        ManagedMatrixAssignmentMode::Linear
    };
    (assign_1d_scalar_vb) => {
        ManagedMatrixAssignmentMode::WholeIf
    };
    (assign_2d_all_range) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_all_range_b) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_all_range_v) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_all_range_vb) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_all_vector) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_all_scalar) => {
        ManagedMatrixAssignmentMode::Columns
    };
    (assign_2d_range_all) => {
        ManagedMatrixAssignmentMode::Rows
    };
    (assign_2d_range_all_b) => {
        ManagedMatrixAssignmentMode::Rows
    };
    (assign_2d_range_all_v) => {
        ManagedMatrixAssignmentMode::Rows
    };
    (assign_2d_range_all_vb) => {
        ManagedMatrixAssignmentMode::Rows
    };
    (assign_2d_scalar_all_vector) => {
        ManagedMatrixAssignmentMode::Rows
    };
    (assign_2d_scalar_all_scalar) => {
        ManagedMatrixAssignmentMode::Rows
    };
}

trait ManagedAssignmentSelectorElement: mech_core::ManagedElement + FunctionPortBacking {
    const LOGICAL: bool;

    fn ordinal(self, upper: usize, axis: &str) -> MResult<usize>;
    fn selected(self) -> bool;
}

impl ManagedAssignmentSelectorElement for usize {
    const LOGICAL: bool = false;

    fn ordinal(self, upper: usize, axis: &str) -> MResult<usize> {
        require_assignment_index(axis, self, upper)?;
        Ok(self - 1)
    }

    fn selected(self) -> bool {
        true
    }
}

#[cfg(feature = "bool")]
impl ManagedAssignmentSelectorElement for bool {
    const LOGICAL: bool = true;

    fn ordinal(self, _upper: usize, _axis: &str) -> MResult<usize> {
        Err(function_shape_contract_violation(
            "assign_slice",
            "logical selector cannot be converted to a positional index",
        ))
    }

    fn selected(self) -> bool {
        self
    }
}

trait ManagedAssignmentSelectorBacking {
    type Element: ManagedAssignmentSelectorElement;

    fn validate(port: FunctionInputPort<'_>, semantic_input: usize) -> MResult<()> {
        let _ = port.try_managed_element_at::<Self::Element>(semantic_input)?;
        Ok(())
    }
}

impl ManagedAssignmentSelectorBacking for usize {
    type Element = usize;
}

impl<R: Dim, C: Dim, S: Storage<usize, R, C>> ManagedAssignmentSelectorBacking
    for naMatrix<usize, R, C, S>
{
    type Element = usize;
}

#[cfg(feature = "bool")]
impl ManagedAssignmentSelectorBacking for bool {
    type Element = bool;
}

#[cfg(feature = "bool")]
impl<R: Dim, C: Dim, S: Storage<bool, R, C>> ManagedAssignmentSelectorBacking
    for naMatrix<bool, R, C, S>
{
    type Element = bool;
}

fn validate_assignment_selector<S: ManagedAssignmentSelectorElement>(
    selector: &mech_core::ManagedValueView<'_, S>,
    upper: usize,
    axis: &str,
) -> MResult<usize> {
    if S::LOGICAL {
        require_assignment_selector_len(axis, selector.len(), upper)?;
        return Ok((0..selector.len())
            .filter(|index| {
                selector
                    .get_column_major(*index)
                    .expect("validated selector geometry")
                    .selected()
            })
            .count());
    }
    for index in 0..selector.len() {
        selector
            .get_column_major(index)
            .expect("validated selector geometry")
            .ordinal(upper, axis)?;
    }
    Ok(selector.len())
}

fn assignment_selector_position<S: ManagedAssignmentSelectorElement>(
    selector: &mech_core::ManagedValueView<'_, S>,
    selected_ordinal: usize,
    upper: usize,
    axis: &str,
) -> MResult<usize> {
    if !S::LOGICAL {
        return selector
            .get_column_major(selected_ordinal)
            .ok_or_else(|| {
                function_shape_contract_violation(
                    "assign_slice",
                    format!("{axis} selector ordinal {selected_ordinal} is unavailable"),
                )
            })?
            .ordinal(upper, axis);
    }
    let mut ordinal = 0usize;
    for index in 0..selector.len() {
        if selector
            .get_column_major(index)
            .expect("validated selector geometry")
            .selected()
        {
            if ordinal == selected_ordinal {
                return Ok(index);
            }
            ordinal += 1;
        }
    }
    Err(function_shape_contract_violation(
        "assign_slice",
        format!("{axis} logical selector has no selected ordinal {selected_ordinal}"),
    ))
}

fn execute_fixed_selection_assignment<T, S>(
    sink: mech_core::ManagedValueView<'_, T>,
    source: mech_core::ManagedValueView<'_, T>,
    selector: mech_core::ManagedValueView<'_, S>,
    output: &mut mech_core::ManagedValueViewMut<'_, T>,
    source_is_scalar: bool,
    mode: ManagedMatrixAssignmentMode,
) -> MResult<()>
where
    T: mech_core::ManagedElement,
    S: ManagedAssignmentSelectorElement,
{
    if sink.rows() != output.rows()
        || sink.columns() != output.columns()
        || sink.len() != output.len()
    {
        return Err(function_shape_contract_violation(
            "assign_slice",
            "assignment stage geometry does not match the published sink",
        ));
    }
    let (upper, axis) = match mode {
        ManagedMatrixAssignmentMode::Linear => (sink.len(), "linear"),
        ManagedMatrixAssignmentMode::Rows => (sink.rows(), "row"),
        ManagedMatrixAssignmentMode::Columns => (sink.columns(), "column"),
        ManagedMatrixAssignmentMode::WholeIf => (1, "linear"),
    };
    let selected = if mode == ManagedMatrixAssignmentMode::WholeIf {
        if !S::LOGICAL || selector.len() != 1 {
            return Err(function_shape_contract_violation(
                "assign_slice",
                "whole-value conditional assignment requires one logical selector",
            ));
        }
        usize::from(
            selector
                .get_column_major(0)
                .expect("validated scalar selector geometry")
                .selected(),
        )
    } else {
        validate_assignment_selector(&selector, upper, axis)?
    };

    // Prove every source read before the first stage write. Logical selectors
    // retain physical source alignment; positional selectors consume source
    // lanes in selector order.
    if !source_is_scalar {
        match mode {
            ManagedMatrixAssignmentMode::Linear => {
                if S::LOGICAL {
                    for ordinal in 0..selected {
                        let position =
                            assignment_selector_position(&selector, ordinal, upper, axis)?;
                        require_assignment_source_index(source.len(), position)?;
                    }
                } else if selected != 0 {
                    require_assignment_source_index(source.len(), selected - 1)?;
                }
            }
            ManagedMatrixAssignmentMode::Rows => {
                if source.columns() < sink.columns() {
                    return require_assignment_source_layout(
                        source.rows(),
                        source.columns(),
                        selected,
                        sink.columns(),
                        !S::LOGICAL,
                        false,
                    );
                }
                if S::LOGICAL {
                    for ordinal in 0..selected {
                        let row = assignment_selector_position(&selector, ordinal, upper, axis)?;
                        if source.rows() <= row {
                            return require_assignment_source_layout(
                                source.rows(),
                                source.columns(),
                                row + 1,
                                sink.columns(),
                                false,
                                false,
                            );
                        }
                    }
                } else {
                    require_assignment_source_layout(
                        source.rows(),
                        source.columns(),
                        selected,
                        sink.columns(),
                        true,
                        false,
                    )?;
                }
            }
            ManagedMatrixAssignmentMode::Columns => {
                require_assignment_source_layout(
                    source.rows(),
                    source.columns(),
                    sink.rows(),
                    selected,
                    false,
                    true,
                )?;
            }
            ManagedMatrixAssignmentMode::WholeIf => {
                if selected != 0 && source.len() < sink.len() {
                    require_assignment_source_index(source.len(), sink.len() - 1)?;
                }
            }
        }
    }

    output.try_fill_column_major(|index| {
        sink.get_column_major(index).ok_or_else(|| {
            function_shape_contract_violation(
                "assign_slice",
                "published assignment sink geometry is inconsistent",
            )
        })
    })?;

    for ordinal in 0..selected {
        let position = assignment_selector_position(&selector, ordinal, upper, axis)?;
        match mode {
            ManagedMatrixAssignmentMode::Linear => {
                let source_index = if source_is_scalar {
                    0
                } else if S::LOGICAL {
                    position
                } else {
                    ordinal
                };
                let value = source
                    .get_column_major(source_index)
                    .ok_or_else(|| assignment_source_out_of_bounds(source_index, source.len()))?;
                output.try_set_column_major(position, value)?;
            }
            ManagedMatrixAssignmentMode::Rows => {
                for column in 0..sink.columns() {
                    let source_row = if source_is_scalar {
                        0
                    } else if S::LOGICAL {
                        position
                    } else if source.rows() == 1 {
                        0
                    } else {
                        ordinal
                    };
                    let source_column = if source_is_scalar || source.columns() == 1 {
                        0
                    } else {
                        column
                    };
                    let value = source.get(source_row, source_column).ok_or_else(|| {
                        assignment_source_out_of_bounds(
                            source_column
                                .saturating_mul(source.rows())
                                .saturating_add(source_row),
                            source.len(),
                        )
                    })?;
                    output.try_set_column_major(column * sink.rows() + position, value)?;
                }
            }
            ManagedMatrixAssignmentMode::Columns => {
                for row in 0..sink.rows() {
                    let source_column = if source_is_scalar || source.columns() == 1 {
                        0
                    } else {
                        ordinal
                    };
                    let source_row = if source_is_scalar { 0 } else { row };
                    let value = source.get(source_row, source_column).ok_or_else(|| {
                        assignment_source_out_of_bounds(
                            source_column
                                .saturating_mul(source.rows())
                                .saturating_add(source_row),
                            source.len(),
                        )
                    })?;
                    output.try_set_column_major(position * sink.rows() + row, value)?;
                }
            }
            ManagedMatrixAssignmentMode::WholeIf => {
                for index in 0..sink.len() {
                    let value = source
                        .get_column_major(index)
                        .ok_or_else(|| assignment_source_out_of_bounds(index, source.len()))?;
                    output.try_set_column_major(index, value)?;
                }
            }
        }
    }
    Ok(())
}

fn execute_fixed_element_assignment<T>(
    sink: mech_core::ManagedValueView<'_, T>,
    source: mech_core::ManagedValueView<'_, T>,
    rows: mech_core::ManagedValueView<'_, usize>,
    columns: mech_core::ManagedValueView<'_, usize>,
    output: &mut mech_core::ManagedValueViewMut<'_, T>,
) -> MResult<()>
where
    T: mech_core::ManagedElement,
{
    if rows.len() != 1 || columns.len() != 1 || source.len() != 1 {
        return Err(function_shape_contract_violation(
            "assign_slice",
            "single-element assignment requires scalar source, row, and column inputs",
        ));
    }
    if sink.rows() != output.rows()
        || sink.columns() != output.columns()
        || sink.len() != output.len()
    {
        return Err(function_shape_contract_violation(
            "assign_slice",
            "single-element assignment stage geometry does not match the sink",
        ));
    }
    let row = rows
        .get_column_major(0)
        .expect("validated row selector")
        .ordinal(sink.rows(), "row")?;
    let column = columns
        .get_column_major(0)
        .expect("validated column selector")
        .ordinal(sink.columns(), "column")?;
    let value = source
        .get_column_major(0)
        .expect("validated scalar assignment source");
    output.try_fill_column_major(|index| {
        sink.get_column_major(index).ok_or_else(|| {
            function_shape_contract_violation(
                "assign_slice",
                "published assignment sink geometry is inconsistent",
            )
        })
    })?;
    output.try_set_column_major(column * sink.rows() + row, value)
}

fn execute_fixed_rectangle_assignment<T, R, C>(
    sink: mech_core::ManagedValueView<'_, T>,
    source: mech_core::ManagedValueView<'_, T>,
    rows: mech_core::ManagedValueView<'_, R>,
    columns: mech_core::ManagedValueView<'_, C>,
    output: &mut mech_core::ManagedValueViewMut<'_, T>,
    source_is_scalar: bool,
) -> MResult<()>
where
    T: mech_core::ManagedElement,
    R: ManagedAssignmentSelectorElement,
    C: ManagedAssignmentSelectorElement,
{
    if sink.rows() != output.rows()
        || sink.columns() != output.columns()
        || sink.len() != output.len()
    {
        return Err(function_shape_contract_violation(
            "assign_slice",
            "rectangle assignment stage geometry does not match the published sink",
        ));
    }
    let selected_rows = validate_assignment_selector(&rows, sink.rows(), "row")?;
    let selected_columns = validate_assignment_selector(&columns, sink.columns(), "column")?;
    let selected_len = selected_rows.checked_mul(selected_columns).ok_or_else(|| {
        function_shape_contract_violation(
            "assign_slice",
            "rectangle assignment source cardinality overflowed usize",
        )
    })?;
    if source_is_scalar {
        if source.len() != 1 {
            return Err(function_shape_contract_violation(
                "assign_slice",
                "rectangle scalar assignment source is not scalar",
            ));
        }
    } else if rows.len() == 1 && columns.len() != 1 {
        let last = if C::LOGICAL {
            (0..selected_columns)
                .map(|ordinal| {
                    assignment_selector_position(&columns, ordinal, sink.columns(), "column")
                })
                .collect::<MResult<Vec<_>>>()?
                .into_iter()
                .max()
        } else {
            selected_columns.checked_sub(1)
        };
        if let Some(last) = last {
            require_assignment_source_index(source.len(), last)?;
        }
    } else if columns.len() == 1 && rows.len() != 1 {
        let last = if R::LOGICAL {
            (0..selected_rows)
                .map(|ordinal| assignment_selector_position(&rows, ordinal, sink.rows(), "row"))
                .collect::<MResult<Vec<_>>>()?
                .into_iter()
                .max()
        } else {
            selected_rows.checked_sub(1)
        };
        if let Some(last) = last {
            require_assignment_source_index(source.len(), last)?;
        }
    } else if R::LOGICAL || C::LOGICAL {
        for row_ordinal in 0..selected_rows {
            let row = assignment_selector_position(&rows, row_ordinal, sink.rows(), "row")?;
            for column_ordinal in 0..selected_columns {
                let column = assignment_selector_position(
                    &columns,
                    column_ordinal,
                    sink.columns(),
                    "column",
                )?;
                require_assignment_source_index(
                    source.len(),
                    column
                        .checked_mul(sink.rows())
                        .and_then(|offset| offset.checked_add(row))
                        .ok_or_else(|| {
                            function_shape_contract_violation(
                                "assign_slice",
                                "rectangle assignment source offset overflowed usize",
                            )
                        })?,
                )?;
            }
        }
    } else if selected_len != 0 {
        require_assignment_source_index(source.len(), selected_len - 1)?;
    }

    output.try_fill_column_major(|index| {
        sink.get_column_major(index).ok_or_else(|| {
            function_shape_contract_violation(
                "assign_slice",
                "published assignment sink geometry is inconsistent",
            )
        })
    })?;
    for row_ordinal in 0..selected_rows {
        let row = assignment_selector_position(&rows, row_ordinal, sink.rows(), "row")?;
        for column_ordinal in 0..selected_columns {
            let column =
                assignment_selector_position(&columns, column_ordinal, sink.columns(), "column")?;
            let source_index = if source_is_scalar {
                0
            } else if rows.len() == 1 && columns.len() != 1 {
                if C::LOGICAL { column } else { column_ordinal }
            } else if columns.len() == 1 && rows.len() != 1 {
                if R::LOGICAL { row } else { row_ordinal }
            } else if R::LOGICAL || C::LOGICAL {
                column * sink.rows() + row
            } else {
                row_ordinal * selected_columns + column_ordinal
            };
            let value = source
                .get_column_major(source_index)
                .ok_or_else(|| assignment_source_out_of_bounds(source_index, source.len()))?;
            output.try_set_column_major(column * sink.rows() + row, value)?;
        }
    }
    Ok(())
}

trait ManagedAssignmentElement:
    Debug
    + Clone
    + Sync
    + Send
    + PartialEq
    + 'static
    + ConstElem
    + FunctionRuntimeType
    + CanonicalMatrixElementBacking
{
    const MEMORY_CLASS: mech_core::ImplementationMemoryClass;

    fn validate_input(port: FunctionInputPort<'_>, semantic_input: usize) -> MResult<()>;
    fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()>;
    fn planned_selection_output_footprint(
        _sink: &ValueCell,
        _source: &ValueCell,
        _selector: &ValueCell,
        _mode: ManagedMatrixAssignmentMode,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn planned_element_output_footprint(
        _sink: &ValueCell,
        _source: &ValueCell,
        _row: &ValueCell,
        _column: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn planned_rectangle_output_footprint(
        _sink: &ValueCell,
        _source: &ValueCell,
        _rows: &ValueCell,
        _columns: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn planned_whole_output_footprint(
        _sink: &ValueCell,
        _source: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        Ok(None)
    }
    fn solve_selection<S: ManagedAssignmentSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        selector: &ValueCell,
        source_is_scalar: bool,
        mode: ManagedMatrixAssignmentMode,
    ) -> MResult<()>;
    fn solve_element(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        row: &ValueCell,
        column: &ValueCell,
    ) -> MResult<()>;
    fn solve_rectangle<R: ManagedAssignmentSelectorElement, C: ManagedAssignmentSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        rows: &ValueCell,
        columns: &ValueCell,
        source_is_scalar: bool,
    ) -> MResult<()>;
    fn solve_whole(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        source_is_scalar: bool,
    ) -> MResult<()>;
}

macro_rules! impl_managed_fixed_assignment_element {
    ($($type:ty),+ $(,)?) => {$(
        impl ManagedAssignmentElement for $type {
            const MEMORY_CLASS: mech_core::ImplementationMemoryClass =
                mech_core::ImplementationMemoryClass::NoAdditionalScratch;

            fn validate_input(port: FunctionInputPort<'_>, semantic_input: usize) -> MResult<()> {
                let _ = port.try_managed_element_at::<Self>(semantic_input)?;
                Ok(())
            }

            fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()> {
                let _ = port.try_managed_element::<Self>()?;
                let _ = port.try_managed_element_base_input::<Self>(0)?;
                Ok(())
            }

            fn solve_selection<S: ManagedAssignmentSelectorElement>(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                sink: &ValueCell,
                source: &ValueCell,
                selector: &ValueCell,
                source_is_scalar: bool,
                mode: ManagedMatrixAssignmentMode,
            ) -> MResult<()> {
                frame.with_assignment_selection_views::<Self, S, _>(
                    sink,
                    source,
                    selector,
                    |sink, source, selector, output| {
                        execute_fixed_selection_assignment(
                            sink,
                            source,
                            selector,
                            output,
                            source_is_scalar,
                            mode,
                        )
                    },
                )
            }

            fn solve_element(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                sink: &ValueCell,
                source: &ValueCell,
                row: &ValueCell,
                column: &ValueCell,
            ) -> MResult<()> {
                frame.with_assignment_rectangle_views::<Self, usize, usize, _>(
                    sink,
                    source,
                    row,
                    column,
                    execute_fixed_element_assignment,
                )
            }

            fn solve_rectangle<R: ManagedAssignmentSelectorElement, C: ManagedAssignmentSelectorElement>(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                sink: &ValueCell,
                source: &ValueCell,
                rows: &ValueCell,
                columns: &ValueCell,
                source_is_scalar: bool,
            ) -> MResult<()> {
                frame.with_assignment_rectangle_views::<Self, R, C, _>(
                    sink,
                    source,
                    rows,
                    columns,
                    |sink, source, rows, columns, output| {
                        execute_fixed_rectangle_assignment(
                            sink,
                            source,
                            rows,
                            columns,
                            output,
                            source_is_scalar,
                        )
                    },
                )
            }

            fn solve_whole(
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                sink: &ValueCell,
                source: &ValueCell,
                source_is_scalar: bool,
            ) -> MResult<()> {
                frame.with_assignment_whole_views::<Self, _>(
                    sink,
                    source,
                    |_sink, source, output| {
                        if !source_is_scalar && source.len() != output.len() {
                            return Err(function_shape_contract_violation(
                                "assign_slice",
                                "whole-value assignment source and sink extents disagree",
                            ));
                        }
                        if source_is_scalar && source.len() != 1 {
                            return Err(function_shape_contract_violation(
                                "assign_slice",
                                "whole-value scalar assignment source is not scalar",
                            ));
                        }
                        output.try_fill_column_major(|index| {
                            source
                                .get_column_major(if source_is_scalar { 0 } else { index })
                                .ok_or_else(|| {
                                    assignment_source_out_of_bounds(index, source.len())
                                })
                        })
                    },
                )
            }
        }
    )+};
}

#[cfg(feature = "u8")]
impl_managed_fixed_assignment_element!(u8);
#[cfg(feature = "u16")]
impl_managed_fixed_assignment_element!(u16);
#[cfg(feature = "u32")]
impl_managed_fixed_assignment_element!(u32);
#[cfg(feature = "u64")]
impl_managed_fixed_assignment_element!(u64);
#[cfg(feature = "u128")]
impl_managed_fixed_assignment_element!(u128);
#[cfg(feature = "i8")]
impl_managed_fixed_assignment_element!(i8);
#[cfg(feature = "i16")]
impl_managed_fixed_assignment_element!(i16);
#[cfg(feature = "i32")]
impl_managed_fixed_assignment_element!(i32);
#[cfg(feature = "i64")]
impl_managed_fixed_assignment_element!(i64);
#[cfg(feature = "i128")]
impl_managed_fixed_assignment_element!(i128);
#[cfg(feature = "f32")]
impl_managed_fixed_assignment_element!(f32);
#[cfg(feature = "f64")]
impl_managed_fixed_assignment_element!(f64);
impl_managed_fixed_assignment_element!(usize);
#[cfg(feature = "bool")]
impl_managed_fixed_assignment_element!(bool);
#[cfg(feature = "complex")]
impl_managed_fixed_assignment_element!(C64);
#[cfg(feature = "rational")]
impl_managed_fixed_assignment_element!(R64);

#[cfg(feature = "string")]
impl ManagedAssignmentElement for String {
    const MEMORY_CLASS: mech_core::ImplementationMemoryClass =
        mech_core::ImplementationMemoryClass::CanonicalFinalize;

    fn validate_input(port: FunctionInputPort<'_>, semantic_input: usize) -> MResult<()> {
        let _ = port.try_managed_element_at::<Self>(semantic_input)?;
        Ok(())
    }

    fn validate_output(port: FunctionOutputPort<'_>) -> MResult<()> {
        let _ = port.try_managed_element::<Self>()?;
        let _ = port.try_managed_element_base_input::<Self>(0)?;
        Ok(())
    }

    fn planned_selection_output_footprint(
        sink: &ValueCell,
        source: &ValueCell,
        selector: &ValueCell,
        mode: ManagedMatrixAssignmentMode,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        return string_selection_assignment(sink, source, selector, mode)
            .prospective_output_footprint()
            .map(Some);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (sink, source, selector, mode);
            Ok(None)
        }
    }

    fn planned_element_output_footprint(
        sink: &ValueCell,
        source: &ValueCell,
        row: &ValueCell,
        column: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        return string_element_assignment(sink, source, row, column)
            .prospective_output_footprint()
            .map(Some);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (sink, source, row, column);
            Ok(None)
        }
    }

    fn planned_rectangle_output_footprint(
        sink: &ValueCell,
        source: &ValueCell,
        rows: &ValueCell,
        columns: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        return string_rectangle_assignment(sink, source, rows, columns)
            .prospective_output_footprint()
            .map(Some);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (sink, source, rows, columns);
            Ok(None)
        }
    }

    fn planned_whole_output_footprint(
        sink: &ValueCell,
        source: &ValueCell,
    ) -> MResult<Option<CurrentMemoryFootprint>> {
        #[cfg(feature = "semantic-compiler")]
        return string_whole_assignment(sink, source)
            .prospective_output_footprint()
            .map(Some);
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (sink, source);
            Ok(None)
        }
    }

    fn solve_selection<S: ManagedAssignmentSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        selector: &ValueCell,
        _source_is_scalar: bool,
        mode: ManagedMatrixAssignmentMode,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        {
            return string_selection_assignment(sink, source, selector, mode).stage_managed(frame);
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, sink, source, selector, mode);
            Err(function_shape_contract_violation(
                "assign_slice",
                "String matrix assignment requires canonical runtime support",
            ))
        }
    }

    fn solve_element(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        row: &ValueCell,
        column: &ValueCell,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        {
            return string_element_assignment(sink, source, row, column).stage_managed(frame);
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, sink, source, row, column);
            Err(function_shape_contract_violation(
                "assign_slice",
                "String element assignment requires canonical runtime support",
            ))
        }
    }

    fn solve_rectangle<R: ManagedAssignmentSelectorElement, C: ManagedAssignmentSelectorElement>(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        rows: &ValueCell,
        columns: &ValueCell,
        _source_is_scalar: bool,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        {
            return string_rectangle_assignment(sink, source, rows, columns).stage_managed(frame);
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, sink, source, rows, columns);
            Err(function_shape_contract_violation(
                "assign_slice",
                "String rectangle assignment requires canonical runtime support",
            ))
        }
    }

    fn solve_whole(
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        sink: &ValueCell,
        source: &ValueCell,
        _source_is_scalar: bool,
    ) -> MResult<()> {
        #[cfg(feature = "semantic-compiler")]
        {
            return string_whole_assignment(sink, source).stage_managed(frame);
        }
        #[cfg(not(feature = "semantic-compiler"))]
        {
            let _ = (frame, sink, source);
            Err(function_shape_contract_violation(
                "assign_slice",
                "String whole-value assignment requires canonical runtime support",
            ))
        }
    }
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_selection_assignment(
    sink: &ValueCell,
    source: &ValueCell,
    selector: &ValueCell,
    mode: ManagedMatrixAssignmentMode,
) -> super::AssignCanonicalSelection {
    let (selectors, selection_kind) = match mode {
        ManagedMatrixAssignmentMode::Linear => (
            vec![
                crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                    selector.clone(),
                ),
            ],
            super::CanonicalAssignmentSelectionKind::Linear,
        ),
        ManagedMatrixAssignmentMode::Rows => (
            vec![
                crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                    selector.clone(),
                ),
                crate::intrinsics::canonical_access::CanonicalAccessSelector::All,
            ],
            super::CanonicalAssignmentSelectionKind::Rows,
        ),
        ManagedMatrixAssignmentMode::Columns => (
            vec![
                crate::intrinsics::canonical_access::CanonicalAccessSelector::All,
                crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(
                    selector.clone(),
                ),
            ],
            super::CanonicalAssignmentSelectionKind::Columns,
        ),
        ManagedMatrixAssignmentMode::WholeIf => (
            vec![crate::intrinsics::canonical_access::CanonicalAccessSelector::All],
            super::CanonicalAssignmentSelectionKind::WholeValue,
        ),
    };
    super::AssignCanonicalSelection {
        sink: sink.clone(),
        source: source.clone(),
        selectors,
        selection_kind,
    }
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_element_assignment(
    sink: &ValueCell,
    source: &ValueCell,
    row: &ValueCell,
    column: &ValueCell,
) -> super::AssignCanonicalSelection {
    super::AssignCanonicalSelection {
        sink: sink.clone(),
        source: source.clone(),
        selectors: vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(row.clone()),
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(column.clone()),
        ],
        selection_kind: super::CanonicalAssignmentSelectionKind::SingleElement,
    }
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_rectangle_assignment(
    sink: &ValueCell,
    source: &ValueCell,
    rows: &ValueCell,
    columns: &ValueCell,
) -> super::AssignCanonicalSelection {
    super::AssignCanonicalSelection {
        sink: sink.clone(),
        source: source.clone(),
        selectors: vec![
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(rows.clone()),
            crate::intrinsics::canonical_access::CanonicalAccessSelector::Cell(columns.clone()),
        ],
        selection_kind: super::CanonicalAssignmentSelectionKind::Rectangular,
    }
}

#[cfg(all(feature = "string", feature = "semantic-compiler"))]
fn string_whole_assignment(
    sink: &ValueCell,
    source: &ValueCell,
) -> super::AssignCanonicalSelection {
    super::AssignCanonicalSelection {
        sink: sink.clone(),
        source: source.clone(),
        selectors: vec![crate::intrinsics::canonical_access::CanonicalAccessSelector::All],
        selection_kind: super::CanonicalAssignmentSelectionKind::WholeValue,
    }
}

// Assign -----------------------------------------------------------------

#[macro_export]
macro_rules! impl_set_all_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            source: ValueCell,
            ixes: ValueCell,
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, IxVec)>,
        }
        impl<T, R1, C1, S1: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem
                + Debug
                + AsRef<[$ix]>
                + FunctionPortBacking
                + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: arg2.value().cell().clone(),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R1, C1, S1, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: Scalar + ManagedAssignmentElement,
            IxVec: AsRef<[$ix]> + Debug + ManagedAssignmentSelectorBacking,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
        {
            fn planned_output_footprints(
                &self,
            ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_selection_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    managed_assignment_mode!($op),
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_selection::<<IxVec as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    true,
                    managed_assignment_mode!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, IxVec>
        where
            T: ManagedAssignmentElement + CompileConst,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ixes = compile_value_cell_register(&self.ixes, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, sink, source, ixes);
                Ok(sink)
            }
        }
    };
}

// Assignment keeps its legacy runtime IDs, but the implementations retain
// only logical cells. This local definition deliberately shadows the older
// repository-wide matrix macro, whose concrete `Ref<Matrix<..>>` capture is
// not a valid R6 execution path.
macro_rules! impl_all_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:path)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            source: ValueCell,
            ixes: ValueCell,
            sink: ValueCell,
            marker: PhantomData<fn() -> (T, MatA, MatB, IxVec)>,
        }

        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAssignmentElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec: ConstElem
                + Debug
                + AsRef<[$ix]>
                + FunctionPortBacking
                + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <IxVec as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (sink, source, ixes) = invocation.expect_binary()?;
                T::validate_output(sink)?;
                T::validate_input(source, 1)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(ixes, 2)?;
                Ok(Box::new(Self {
                    sink: sink.value().cell().clone(),
                    source: source.value().cell().clone(),
                    ixes: ixes.value().cell().clone(),
                    marker: PhantomData,
                }))
            }
        }

        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAssignmentElement,
            IxVec: AsRef<[$ix]> + Debug + ManagedAssignmentSelectorBacking,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_footprints(
                &self,
            ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_selection_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    managed_assignment_mode!($op),
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_selection::<<IxVec as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    false,
                    managed_assignment_mode!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }

            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }

            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }

            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }

            fn to_string(&self) -> String {
                format!("{self:#?}")
            }
        }

        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAssignmentElement + CompileConst,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ixes = compile_value_cell_register(&self.ixes, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, sink, source, ixes);
                Ok(sink)
            }
        }
    };
}

// x[1] = 1 ------------------------------------------------------------------

#[macro_export]
macro_rules! assign_1d_scalar {
    ($source:expr, $ix:expr, $sink:expr) => {
        require_assignment_index("linear", $ix, ($sink).len())?;
        ($sink)[$ix - 1] = ($source).clone();
    };
}

#[macro_export]
macro_rules! assign_1d_scalar_b {
    ($source:expr, $ix:expr, $sink:expr) => {
        if $ix {
            for ix in 0..$sink.len() {
                $sink[ix] = $source.clone();
            }
        }
    };
}

#[macro_export]
macro_rules! assign_1d_scalar_vb {
    ($source:expr, $ix:expr, $sink:expr) => {
        if *$ix {
            if !($sink).is_empty() {
                require_assignment_source_index(($source).len(), ($sink).len() - 1)?;
            }
            for ix in 0..$sink.len() {
                $sink[ix] = $source[ix].clone();
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_fxn_s {
    ($struct_name:ident, $op:ident, $ix:ty $(, $semantic_contract:ident)?) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA> {
            source: ValueCell,
            ixes: ValueCell,
            sink: ValueCell,
            marker: PhantomData<fn() -> (T, MatA)>,
        }
        impl<T, R, C, S: 'static> MechFunctionFactory for $struct_name<T, naMatrix<T, R, C, S>>
        where
            T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + FunctionStateBacking,
            $ix: FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <$ix as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: arg2.value().cell().clone(),
                    marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S> MechFunctionImpl for $struct_name<T, naMatrix<T, R, C, S>>
        where
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + ManagedAssignmentElement,
            $ix: ManagedAssignmentSelectorBacking,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn planned_output_footprints(
                &self,
            ) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_selection_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    managed_assignment_mode!($op),
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_selection::<<$ix as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    true,
                    managed_assignment_mode!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                optional_operation_contract!($($semantic_contract)?)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S> MechFunctionCompiler for $struct_name<T, naMatrix<T, R, C, S>>
        where
            T: ManagedAssignmentElement + CompileConst,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ixes = compile_value_cell_register(&self.ixes, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, sink, source, ixes);
                Ok(sink)
            }
        }
    };
}

impl_assign_fxn_s!(
    Assign1DS,
    assign_1d_scalar,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_assign_fxn_s!(
    Assign1DB,
    assign_1d_scalar_b,
    bool,
    PURE_MATRIX_WHOLE_ASSIGNMENT_CONTRACT
);
impl_assign_scalar_fxn_v!(Assign1DVB, assign_1d_scalar_vb, bool);

// x[1..3] = 1 ----------------------------------------------------------------

impl_set_all_fxn_s!(
    Assign1DRS,
    set_1d_range,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Assign1DRB,
    set_1d_range_b,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Assign1DRV,
    set_1d_range_vec,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Assign1DRVB,
    set_1d_range_vec_b,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);

// x[:] = 1 ------------------------------------------------------------------

#[derive(Debug)]
pub struct Set1DAS<T, Sink> {
    source: ValueCell,
    sink: ValueCell,
    marker: PhantomData<fn() -> (T, Sink)>,
}
impl<T, R, C, S> MechFunctionFactory for Set1DAS<T, naMatrix<T, R, C, S>>
where
    T: ManagedAssignmentElement + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Debug + IsContiguous + 'static,
    naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R, C, S>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::unary(
        <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        T::MEMORY_CLASS
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_FULL_ASSIGNMENT_CONTRACT)
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg1) = invocation.expect_unary()?;
        T::validate_output(out)?;
        T::validate_input(arg1, 1)?;
        Ok(Box::new(Self {
            sink: out.value().cell().clone(),
            source: arg1.value().cell().clone(),
            marker: PhantomData,
        }))
    }
}
impl<T, R, C, S> MechFunctionImpl for Set1DAS<T, naMatrix<T, R, C, S>>
where
    T: ManagedAssignmentElement,
    naMatrix<T, R, C, S>: FunctionStateBacking,
    R: Dim,
    C: Dim,
    S: StorageMut<T, R, C> + Debug + IsContiguous,
{
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(T::planned_whole_output_footprint(&self.sink, &self.source)?
            .map(|footprint| vec![footprint].into_boxed_slice()))
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        T::solve_whole(frame, &self.sink, &self.source, true)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.sink))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_FULL_ASSIGNMENT_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, R, C, S> MechFunctionCompiler for Set1DAS<T, naMatrix<T, R, C, S>>
where
    T: ManagedAssignmentElement + CompileConst,
    naMatrix<T, R, C, S>: CompileConst + ConstElem,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Set1DAS<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<naMatrix<T, R, C, S>>()
        );
        let sink = compile_value_cell_register(&self.sink, ctx)?;
        let source = compile_value_cell_register(&self.source, ctx)?;
        let function = ctx.function_id(&name)?;
        ctx.emit_unop(function, sink, source);
        Ok(sink)
    }
}

#[derive(Debug)]
pub struct Assign2DSSS<T, MatA> {
    source: ValueCell,
    ixes: (ValueCell, ValueCell),
    sink: ValueCell,
    marker: PhantomData<fn() -> (T, MatA)>,
}
impl<T, R1, C1, S1: 'static> MechFunctionFactory for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
    #[cfg(feature = "semantic-compiler")]
    T: CompileConst,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
    naMatrix<T, R1, C1, S1>: ConstElem + FunctionStateBacking,
    #[cfg(feature = "semantic-compiler")]
    naMatrix<T, R1, C1, S1>: CompileConst,
{
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
        <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
        T::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
        <usize as FunctionRuntimeType>::REPRESENTATION,
    );

    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        T::MEMORY_CLASS
    }

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
        T::validate_output(out)?;
        T::validate_input(arg1, 1)?;
        let _ = arg2.try_managed_at::<usize>(2)?;
        let _ = arg3.try_managed_at::<usize>(3)?;
        Ok(Box::new(Self {
            sink: out.value().cell().clone(),
            source: arg1.value().cell().clone(),
            ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
            marker: PhantomData,
        }))
    }

    fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT)
    }
}
impl<T, R1, C1, S1> MechFunctionImpl for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    naMatrix<T, R1, C1, S1>: FunctionStateBacking,
    T: Scalar + ManagedAssignmentElement,
    R1: Dim,
    C1: Dim,
    S1: StorageMut<T, R1, C1> + Clone + Debug,
{
    fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
        Ok(T::planned_element_output_footprint(
            &self.sink,
            &self.source,
            &self.ixes.0,
            &self.ixes.1,
        )?
        .map(|footprint| vec![footprint].into_boxed_slice()))
    }

    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        T::solve_element(frame, &self.sink, &self.source, &self.ixes.0, &self.ixes.1)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }
    fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
        Some(FunctionStatePort::from_cell(&self.sink))
    }
    fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
        Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
    }
    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&PURE_MATRIX_ELEMENT_ASSIGNMENT_CONTRACT)
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T, R1, C1, S1> MechFunctionCompiler for Assign2DSSS<T, naMatrix<T, R1, C1, S1>>
where
    T: ManagedAssignmentElement + CompileConst,
    naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!(
            "Assign2DSSS<{}{}>",
            <T as FunctionRuntimeType>::REPRESENTATION,
            function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>()
        );
        let sink = compile_value_cell_register(&self.sink, ctx)?;
        let source = compile_value_cell_register(&self.source, ctx)?;
        let row = compile_value_cell_register(&self.ixes.0, ctx)?;
        let column = compile_value_cell_register(&self.ixes.1, ctx)?;
        let function = ctx.function_id(&name)?;
        ctx.emit_ternop(function, sink, source, row, column);
        Ok(sink)
    }
}

#[macro_export]
macro_rules! impl_assign_scalar_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB> {
            source: ValueCell,
            ixes: ValueCell,
            sink: ValueCell,
            marker: PhantomData<fn() -> (T, MatA, MatB)>,
        }
        impl<T, R1: 'static, C1: 'static, S1: 'static, R2: 'static, C2: 'static, S2: 'static>
            MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
        where
            T: ManagedAssignmentElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            $ix: FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <$ix as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2) = invocation.expect_binary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <$ix as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: arg2.value().cell().clone(),
                    marker: PhantomData,
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
        where
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: ManagedAssignmentElement,
            $ix: ManagedAssignmentSelectorBacking,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_selection_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    managed_assignment_mode!($op),
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_selection::<<$ix as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes,
                    false,
                    managed_assignment_mode!($op),
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>>
        where
            T: ManagedAssignmentElement + CompileConst,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ixes = compile_value_cell_register(&self.ixes, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_binop(function, sink, source, ixes);
                Ok(sink)
            }
        }
    };
}

impl_assign_fxn_s!(Assign2DASS, assign_2d_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DASV, assign_2d_all_vector, usize);

impl_assign_fxn_s!(Assign2DSAS, assign_2d_scalar_all_scalar, usize);
impl_assign_scalar_fxn_v!(Assign2DSAV, assign_2d_scalar_all_vector, usize);

#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_s {
    ($struct_name:ident, $op:tt, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, IxVec)>,
        }
        impl<T, R, C, S: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                <usize as ManagedAssignmentSelectorBacking>::validate(arg3, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec> MechFunctionImpl for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + ManagedAssignmentElement,
            IxVec: ManagedAssignmentSelectorBacking + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<<IxVec as ManagedAssignmentSelectorBacking>::Element, usize>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    true,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ix1 = compile_value_cell_register(&self.ixes.0, ctx)?;
                let ix2 = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_range_scalar_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, MatB, IxVec)>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAssignmentElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                <usize as ManagedAssignmentSelectorBacking>::validate(arg3, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: ManagedAssignmentElement,
            IxVec: ManagedAssignmentSelectorBacking + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<<IxVec as ManagedAssignmentSelectorBacking>::Element, usize>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    false,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ix1 = compile_value_cell_register(&self.ixes.0, ctx)?;
                let ix2 = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

impl_assign_range_scalar_fxn_s!(Assign2DSSMD, assign_2d_range_scalar, usize);

impl_assign_range_scalar_fxn_s!(Assign2DRSS, assign_2d_range_scalar, usize);
impl_assign_range_scalar_fxn_s!(Assign2DRSB, assign_2d_range_scalar_b, bool);
impl_assign_range_scalar_fxn_v!(Assign2DRSV, assign_2d_range_scalar_v, usize);
impl_assign_range_scalar_fxn_v!(Assign2DRSVB, assign_2d_range_scalar_vb, bool);

#[macro_export]
macro_rules! impl_assign_scalar_range_fxn_s {
    ($struct_name:ident, $op:tt, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, IxVec)>,
        }
        impl<T, R, C, S: 'static, IxVec: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }

            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <usize as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(arg3, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec> MechFunctionImpl for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + ManagedAssignmentElement,
            IxVec: ManagedAssignmentSelectorBacking + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<usize, <IxVec as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    true,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ix1 = compile_value_cell_register(&self.ixes.0, ctx)?;
                let ix2 = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_assign_scalar_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, MatB, IxVec)>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: ManagedAssignmentElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                <usize as FunctionRuntimeType>::REPRESENTATION,
                IxVec::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <usize as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                <IxVec as ManagedAssignmentSelectorBacking>::validate(arg3, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
            T: ManagedAssignmentElement,
            IxVec: ManagedAssignmentSelectorBacking + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<usize, <IxVec as ManagedAssignmentSelectorBacking>::Element>(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    false,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ix1 = compile_value_cell_register(&self.ixes.0, ctx)?;
                let ix2 = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

impl_assign_scalar_range_fxn_s!(Assign2DSRS, assign_2d_scalar_range, usize);
impl_assign_scalar_range_fxn_s!(Assign2DSRB, assign_2d_scalar_range_b, bool);
impl_assign_scalar_range_fxn_v!(Assign2DSRV, assign_2d_scalar_range_v, usize);
impl_assign_scalar_range_fxn_v!(Assign2DSRVB, assign_2d_scalar_range_vb, bool);

#[macro_export]
macro_rules! impl_assign_range_range_fxn_s {
    ($struct_name:ident, $op:tt, $ix1:ty, $ix2:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, IxVec1, IxVec2> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, IxVec1, IxVec2)>,
        }
        impl<T, R, C, S: 'static, IxVec1: 'static, IxVec2: 'static> MechFunctionFactory
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            T: Scalar + ManagedAssignmentElement + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst + CanonicalMatrixElementBacking,
            IxVec1: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            IxVec2: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
            naMatrix<T, R, C, S>: ConstElem + Debug + FunctionStateBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R, C, S>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R, C, S> as FunctionRuntimeType>::REPRESENTATION,
                T::REPRESENTATION,
                IxVec1::REPRESENTATION,
                IxVec2::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }

            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, arg1, arg2, arg3) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(arg1, 1)?;
                <IxVec1 as ManagedAssignmentSelectorBacking>::validate(arg2, 2)?;
                <IxVec2 as ManagedAssignmentSelectorBacking>::validate(arg3, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: arg1.value().cell().clone(),
                    ixes: (arg2.value().cell().clone(), arg3.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R, C, S, IxVec1, IxVec2> MechFunctionImpl
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            naMatrix<T, R, C, S>: FunctionStateBacking,
            T: Scalar + ManagedAssignmentElement,
            IxVec1: ManagedAssignmentSelectorBacking + Debug,
            IxVec2: ManagedAssignmentSelectorBacking + Debug,
            R: Dim,
            C: Dim,
            S: StorageMut<T, R, C> + Clone + Debug,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<
                    <IxVec1 as ManagedAssignmentSelectorBacking>::Element,
                    <IxVec2 as ManagedAssignmentSelectorBacking>::Element,
                >(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    true,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R, C, S, IxVec1, IxVec2> MechFunctionCompiler
            for $struct_name<T, na::Matrix<T, R, C, S>, IxVec1, IxVec2>
        where
            T: CompileConst + ConstElem + FunctionRuntimeType + CanonicalMatrixElementBacking,
            IxVec1: CompileConst + ConstElem,
            IxVec2: CompileConst + ConstElem,
            naMatrix<T, R, C, S>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R, C, S>>(),
                    function_matrix_storage_name::<IxVec1>(),
                    function_matrix_storage_name::<IxVec2>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let ix1 = compile_value_cell_register(&self.ixes.0, ctx)?;
                let ix2 = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, ix1, ix2);
                Ok(sink)
            }
        }
    };
}

macro_rules! impl_range_range_fxn_v {
    ($struct_name:ident, $op:ident, $ix1:ty, $ix2:ty) => {
        #[derive(Debug)]
        pub struct $struct_name<T, MatA, MatB, IxVec1, IxVec2> {
            source: ValueCell,
            ixes: (ValueCell, ValueCell),
            sink: ValueCell,
            _marker: PhantomData<fn() -> (T, MatA, MatB, IxVec1, IxVec2)>,
        }
        impl<
            T,
            R1: 'static,
            C1: 'static,
            S1: 'static,
            R2: 'static,
            C2: 'static,
            S2: 'static,
            IxVec1: 'static,
            IxVec2: 'static,
        > MechFunctionFactory
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: ManagedAssignmentElement,
            #[cfg(feature = "semantic-compiler")]
            T: CompileConst,
            IxVec1: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            IxVec2: ConstElem + Debug + FunctionPortBacking + ManagedAssignmentSelectorBacking,
            #[cfg(feature = "semantic-compiler")]
            IxVec1: CompileConst,
            #[cfg(feature = "semantic-compiler")]
            IxVec2: CompileConst,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: ConstElem + Debug + FunctionStateBacking,
            naMatrix<T, R2, C2, S2>: ConstElem + Debug + FunctionPortBacking,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R1, C1, S1>: CompileConst,
            #[cfg(feature = "semantic-compiler")]
            naMatrix<T, R2, C2, S2>: CompileConst,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::ternary(
                <naMatrix<T, R1, C1, S1> as FunctionRuntimeType>::REPRESENTATION,
                <naMatrix<T, R2, C2, S2> as FunctionRuntimeType>::REPRESENTATION,
                IxVec1::REPRESENTATION,
                IxVec2::REPRESENTATION,
            );

            fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
                T::MEMORY_CLASS
            }
            fn declared_operation_contract() -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
                let (out, source, rows, columns) = invocation.expect_ternary()?;
                T::validate_output(out)?;
                T::validate_input(source, 1)?;
                <IxVec1 as ManagedAssignmentSelectorBacking>::validate(rows, 2)?;
                <IxVec2 as ManagedAssignmentSelectorBacking>::validate(columns, 3)?;
                Ok(Box::new(Self {
                    sink: out.value().cell().clone(),
                    source: source.value().cell().clone(),
                    ixes: (rows.value().cell().clone(), columns.value().cell().clone()),
                    _marker: PhantomData,
                }))
            }
        }
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionImpl
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: ManagedAssignmentElement,
            IxVec1: ManagedAssignmentSelectorBacking + Debug,
            IxVec2: ManagedAssignmentSelectorBacking + Debug,
            R1: Dim,
            C1: Dim,
            S1: StorageMut<T, R1, C1> + Clone + Debug,
            R2: Dim,
            C2: Dim,
            S2: Storage<T, R2, C2> + Clone + Debug,
            naMatrix<T, R1, C1, S1>: FunctionStateBacking,
        {
            fn planned_output_footprints(&self) -> MResult<Option<Box<[CurrentMemoryFootprint]>>> {
                Ok(T::planned_rectangle_output_footprint(
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                )?
                .map(|footprint| vec![footprint].into_boxed_slice()))
            }

            fn solve_managed(
                &self,
                frame: &mut mech_core::KernelMemoryFrame<'_>,
                _services: &mut dyn mech_core::MechExecutionServices,
            ) -> MResult<mech_core::ReactiveSolveStatus> {
                T::solve_rectangle::<
                    <IxVec1 as ManagedAssignmentSelectorBacking>::Element,
                    <IxVec2 as ManagedAssignmentSelectorBacking>::Element,
                >(
                    frame,
                    &self.sink,
                    &self.source,
                    &self.ixes.0,
                    &self.ixes.1,
                    false,
                )?;
                Ok(mech_core::ReactiveSolveStatus::Changed)
            }
            fn primary_output_state_port(&self) -> Option<FunctionStatePort<'_>> {
                Some(FunctionStatePort::from_cell(&self.sink))
            }
            fn transaction_state_ports(&self) -> MResult<Option<Vec<FunctionStatePort<'_>>>> {
                Ok(Some(vec![FunctionStatePort::from_cell(&self.sink)]))
            }
            fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
                Some(&PURE_MATRIX_RECTANGLE_ASSIGNMENT_CONTRACT)
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T, R1, C1, S1, R2, C2, S2, IxVec1, IxVec2> MechFunctionCompiler
            for $struct_name<T, naMatrix<T, R1, C1, S1>, naMatrix<T, R2, C2, S2>, IxVec1, IxVec2>
        where
            T: ManagedAssignmentElement + CompileConst,
            IxVec1: CompileConst + ConstElem,
            IxVec2: CompileConst + ConstElem,
            naMatrix<T, R1, C1, S1>: CompileConst + ConstElem,
            naMatrix<T, R2, C2, S2>: CompileConst + ConstElem,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!(
                    "{}<{}{}{}{}>",
                    stringify!($struct_name),
                    <T as FunctionRuntimeType>::REPRESENTATION,
                    function_matrix_storage_name::<naMatrix<T, R1, C1, S1>>(),
                    function_matrix_storage_name::<naMatrix<T, R2, C2, S2>>(),
                    function_matrix_storage_name::<IxVec1>()
                );
                let sink = compile_value_cell_register(&self.sink, ctx)?;
                let source = compile_value_cell_register(&self.source, ctx)?;
                let rows = compile_value_cell_register(&self.ixes.0, ctx)?;
                let columns = compile_value_cell_register(&self.ixes.1, ctx)?;
                let function = ctx.function_id(&name)?;
                ctx.emit_ternop(function, sink, source, rows, columns);
                Ok(sink)
            }
        }
    };
}

impl_assign_range_range_fxn_s!(Assign2DRRS, assign_2d_range_range, usize, usize);
impl_range_range_fxn_v!(Assign2DRRV, assign_2d_range_range_v, usize, usize);

impl_assign_range_range_fxn_s!(Assign2DRRBB, assign_2d_range_range_b, bool, bool);
impl_range_range_fxn_v!(Assign2DRRVBB, assign_2d_range_range_vb, bool, bool);

impl_assign_range_range_fxn_s!(Assign2DRRBU, assign_2d_range_range_bu, bool, usize);
impl_range_range_fxn_v!(Assign2DRRVBU, assign_2d_range_range_vbu, bool, usize);

impl_assign_range_range_fxn_s!(Assign2DRRUB, assign_2d_range_range_ub, usize, bool);
impl_range_range_fxn_v!(Assign2DRRVUB, assign_2d_range_range_vub, usize, bool);

// x[:,1..3] = 1 ------------------------------------------------------------------

impl_all_fxn_v!(
    Set2DARV,
    assign_2d_all_range_v,
    usize,
    PURE_MATRIX_AXIS_ONE_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Set2DARS,
    assign_2d_all_range,
    usize,
    PURE_MATRIX_AXIS_ONE_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Set2DARB,
    assign_2d_all_range_b,
    bool,
    PURE_MATRIX_AXIS_ONE_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Set2DARVB,
    assign_2d_all_range_vb,
    bool,
    PURE_MATRIX_AXIS_ONE_ASSIGNMENT_CONTRACT
);

// x[1..3,:] = 1 ------------------------------------------------------------------

impl_all_fxn_v!(
    Set2DRAV,
    assign_2d_range_all_v,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Set2DRAS,
    assign_2d_range_all,
    usize,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_set_all_fxn_s!(
    Set2DRAB,
    assign_2d_range_all_b,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);
impl_all_fxn_v!(
    Set2DRAVB,
    assign_2d_range_all_vb,
    bool,
    PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT
);

static PURE_MATRIX_AXIS_ZERO_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 0 },
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static PURE_MATRIX_AXIS_ONE_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::IndexedAxis { axis: 1 },
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static PURE_MATRIX_FULL_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::WholeValue,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

static PURE_MATRIX_WHOLE_ASSIGNMENT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input: 0,
                regions: RegionPolicy::WholeValue,
            },
            alias: AliasPolicy::MayAlias { input: 0 },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    });

#[cfg(all(
    test,
    feature = "matrixd",
    feature = "vectord",
    feature = "logical_indexing",
    feature = "u8"
))]
mod tests {
    use super::*;
    use mech_core::FunctionInvocation;
    use nalgebra::{DMatrix, DVector};

    fn managed<F: MechFunctionFactory>(invocation: FunctionInvocation) -> SpecializedFunction {
        crate::test_support::managed_factory_instance::<F>(
            invocation,
            "test/matrix-selection-assignment",
        )
        .unwrap()
    }

    fn u8_elements(value: &ValueCell) -> Vec<u8> {
        value
            .matrix_elements()
            .unwrap()
            .expect("u8 matrix elements")
            .iter()
            .map(|element| match element.snapshot().unwrap().data() {
                ValueData::U8(value) => *value,
                other => panic!("expected u8 matrix element, found {other:?}"),
            })
            .collect()
    }

    #[cfg(feature = "string")]
    fn string_elements(value: &ValueCell) -> Vec<String> {
        value
            .matrix_elements()
            .unwrap()
            .expect("String matrix elements")
            .iter()
            .map(|element| match element.snapshot().unwrap().data() {
                ValueData::String(value) => value.to_string(),
                other => panic!("expected String matrix element, found {other:?}"),
            })
            .collect()
    }

    fn replace_exact<T: CanonicalCellBacking>(cell: &ValueCell, value: T) {
        cell.replace(&ValueCell::from_exact(value).unwrap().snapshot().unwrap())
            .unwrap();
    }

    #[test]
    fn column_assignment_routes_each_rectangular_source_column() {
        let source =
            ValueCell::from_exact(DMatrix::from_row_slice(2, 3, &[1_u8, 2, 3, 4, 5, 6])).unwrap();
        let columns = ValueCell::from_exact(DVector::from_vec(vec![1_usize, 2, 3])).unwrap();
        let sink = ValueCell::from_exact(DMatrix::<u8>::zeros(2, 3)).unwrap();
        let function = managed::<Set2DARV<u8, DMatrix<u8>, DMatrix<u8>, DVector<usize>>>(
            FunctionInvocation::binary(sink.clone(), source, columns),
        );

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&sink), vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn empty_in_place_assignment_keeps_logical_undo_authority() {
        let source = ValueCell::from_exact(DVector::<u8>::zeros(0)).unwrap();
        let mask = ValueCell::from_exact(DVector::<bool>::from_vec(Vec::new())).unwrap();
        let sink = ValueCell::from_exact(DVector::<u8>::zeros(0)).unwrap();
        let function = managed::<Assign1DRVB<u8, DVector<u8>, DVector<u8>, DVector<bool>>>(
            FunctionInvocation::binary(sink.clone(), source, mask),
        );

        function.instance().solve_result().unwrap();

        assert!(u8_elements(&sink).is_empty());
    }

    #[cfg(feature = "string")]
    #[test]
    fn typed_string_index_assignment_uses_complete_candidate_admission() {
        let source = ValueCell::from_exact(DVector::from_vec(vec![
            "long replacement".to_owned(),
            "last write wins".to_owned(),
        ]))
        .unwrap();
        let indices = ValueCell::from_exact(DVector::from_vec(vec![2_usize, 2])).unwrap();
        let sink = ValueCell::from_exact(DVector::from_vec(vec![
            "unchanged".to_owned(),
            "old".to_owned(),
            "also unchanged".to_owned(),
        ]))
        .unwrap();
        let function = managed::<
            Assign1DRV<String, DVector<String>, DVector<String>, DVector<usize>>,
        >(FunctionInvocation::binary(sink.clone(), source, indices));

        function.instance().solve_result().unwrap();
        assert_eq!(
            string_elements(&sink),
            vec![
                "unchanged".to_owned(),
                "last write wins".to_owned(),
                "also unchanged".to_owned(),
            ]
        );
    }

    #[test]
    fn reactive_sparse_mask_is_revalidated_before_any_write() {
        let source = ValueCell::from_exact(DVector::from_vec(vec![9_u8])).unwrap();
        let mask = ValueCell::from_exact(DVector::from_vec(vec![true, false, false])).unwrap();
        let sink = ValueCell::from_exact(DVector::<u8>::zeros(3)).unwrap();
        let function = managed::<Assign1DRVB<u8, DVector<u8>, DVector<u8>, DVector<bool>>>(
            FunctionInvocation::binary(sink.clone(), source, mask.clone()),
        );

        function.instance().solve_result().unwrap();
        assert_eq!(u8_elements(&sink), vec![9, 0, 0]);

        replace_exact(&mask, DVector::from_vec(vec![false, true, false]));
        let error = function.instance().solve_result().unwrap_err();
        assert!(error.kind_message().contains("source offset 1"));
        assert_eq!(u8_elements(&sink), vec![9, 0, 0]);
    }

    #[test]
    fn reactive_mask_extents_are_revalidated_before_any_matrix_write() {
        let source = ValueCell::from_exact(DMatrix::from_element(2, 4, 7_u8)).unwrap();
        let columns = ValueCell::from_exact(DVector::from_vec(vec![true, false, false])).unwrap();
        let sink = ValueCell::from_exact(DMatrix::<u8>::zeros(2, 3)).unwrap();
        let function = managed::<Set2DARVB<u8, DMatrix<u8>, DMatrix<u8>, DVector<bool>>>(
            FunctionInvocation::binary(sink.clone(), source, columns.clone()),
        );

        function.instance().solve_result().unwrap();
        let before = u8_elements(&sink);
        replace_exact(&columns, DVector::from_vec(vec![true, false, false, true]));
        let error = function.instance().solve_result().unwrap_err();
        assert!(
            error
                .kind_message()
                .contains("column selector has length 4")
        );
        assert_eq!(u8_elements(&sink), before);

        let source = ValueCell::from_exact(DMatrix::from_element(3, 3, 8_u8)).unwrap();
        let rows = ValueCell::from_exact(DVector::from_vec(vec![true, false])).unwrap();
        let sink = ValueCell::from_exact(DMatrix::<u8>::zeros(2, 3)).unwrap();
        let function = managed::<Set2DRAVB<u8, DMatrix<u8>, DMatrix<u8>, DVector<bool>>>(
            FunctionInvocation::binary(sink.clone(), source, rows.clone()),
        );

        function.instance().solve_result().unwrap();
        let before = u8_elements(&sink);
        replace_exact(&rows, DVector::from_vec(vec![true, false, true]));
        let error = function.instance().solve_result().unwrap_err();
        assert!(error.kind_message().contains("row selector has length 3"));
        assert_eq!(u8_elements(&sink), before);
    }

    #[test]
    fn reactive_numeric_indices_are_revalidated_before_any_matrix_write() {
        let source_cell = ValueCell::from_exact(9_u8).unwrap();
        let row_cell = ValueCell::from_exact(1_usize).unwrap();
        let column_cell = ValueCell::from_exact(1_usize).unwrap();
        let sink = ValueCell::from_exact(DMatrix::<u8>::zeros(2, 3)).unwrap();
        let function = managed::<Assign2DSSS<u8, DMatrix<u8>>>(FunctionInvocation::ternary(
            sink.clone(),
            source_cell,
            row_cell.clone(),
            column_cell.clone(),
        ));

        function.instance().solve_result().unwrap();
        let before = u8_elements(&sink);
        row_cell
            .replace(&ValueCell::from_exact(3_usize).unwrap().snapshot().unwrap())
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(u8_elements(&sink), before);
        row_cell
            .replace(&ValueCell::from_exact(1_usize).unwrap().snapshot().unwrap())
            .unwrap();
        column_cell
            .replace(&ValueCell::from_exact(4_usize).unwrap().snapshot().unwrap())
            .unwrap();
        assert!(function.instance().solve_result().is_err());
        assert_eq!(u8_elements(&sink), before);
    }
}

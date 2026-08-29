//! Compatibility construction of canonical function invocations.

use crate::{
    CanonicalFunctionSpecializer, FunctionArgumentRole, FunctionArgumentTypeMismatch,
    FunctionInstance, FunctionInvocation, FunctionInvocationLayout, FunctionMatrixDescriptor,
    FunctionMatrixElement, FunctionMatrixStoragePattern, FunctionOutputSchemaRule,
    FunctionValueRepresentation, GenericError, GuardFunctionSafety, LegacyValue, MResult,
    MechError, MechErrorKind, MechFunction, MechFunctionFactory, Ref, SpecializationContext,
    SpecializationInput, SpecializationInvocation, SpecializedFunction, ToValue, ValueCell,
    ValueData, ValueKind,
};
use core::any::Any;

#[cfg(feature = "no_std")]
use alloc::rc::Rc;
#[cfg(feature = "no_std")]
use core::cell;
#[cfg(not(feature = "no_std"))]
use std::{cell, rc::Rc};

#[cfg(feature = "set")]
use crate::{RuntimeFunctionInputs, SchemaBody, ValueDataDraft};

#[cfg(feature = "matrix")]
use crate::function::argument::matrix_descriptor;

/// Compatibility source specializer implemented by pre-cutover callers.
#[doc(hidden)]
pub trait FunctionSpecializer: Send + Sync {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>>;

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

fn legacy_state_ref<T>(backing: &dyn Any) -> Option<LegacyValue>
where
    T: 'static,
    Ref<T>: ToValue,
{
    backing.downcast_ref::<Ref<T>>().map(ToValue::to_value)
}

fn legacy_state_port_value(port: crate::FunctionStatePort<'_>) -> MResult<LegacyValue> {
    let backing = port.backing_any();
    if let Some(cell) = backing.downcast_ref::<ValueCell>() {
        return LegacyValue::from_canonical_value(&cell.snapshot()?);
    }

    macro_rules! scalar {
        ($type:ty, $feature:literal) => {
            #[cfg(feature = $feature)]
            if let Some(value) = legacy_state_ref::<$type>(backing) {
                return Ok(value);
            }
        };
    }
    scalar!(u8, "u8");
    scalar!(u16, "u16");
    scalar!(u32, "u32");
    scalar!(u64, "u64");
    scalar!(u128, "u128");
    scalar!(i8, "i8");
    scalar!(i16, "i16");
    scalar!(i32, "i32");
    scalar!(i64, "i64");
    scalar!(i128, "i128");
    scalar!(f32, "f32");
    scalar!(f64, "f64");
    scalar!(bool, "bool");
    scalar!(String, "string");
    scalar!(crate::C64, "complex");
    scalar!(crate::R64, "rational");
    if let Some(value) = legacy_state_ref::<usize>(backing) {
        return Ok(value);
    }

    #[cfg(feature = "set")]
    if let Some(value) = legacy_state_ref::<crate::MechSet>(backing) {
        return Ok(value);
    }

    macro_rules! matrices {
        ($element:ty, $feature:literal) => {
            #[cfg(all(feature = "matrix", feature = $feature))]
            {
                macro_rules! matrix {
                    ($type:ident, $matrix_feature:literal) => {
                        #[cfg(feature = $matrix_feature)]
                        if let Some(value) = legacy_state_ref::<crate::$type<$element>>(backing) {
                            return Ok(value);
                        }
                    };
                }
                matrix!(Matrix1, "matrix1");
                matrix!(Matrix2, "matrix2");
                matrix!(Matrix3, "matrix3");
                matrix!(Matrix4, "matrix4");
                matrix!(Matrix2x3, "matrix2x3");
                matrix!(Matrix3x2, "matrix3x2");
                matrix!(RowVector2, "row_vector2");
                matrix!(RowVector3, "row_vector3");
                matrix!(RowVector4, "row_vector4");
                matrix!(RowDVector, "row_vectord");
                matrix!(Vector2, "vector2");
                matrix!(Vector3, "vector3");
                matrix!(Vector4, "vector4");
                matrix!(DVector, "vectord");
                matrix!(DMatrix, "matrixd");
            }
        };
    }
    matrices!(u8, "u8");
    matrices!(u16, "u16");
    matrices!(u32, "u32");
    matrices!(u64, "u64");
    matrices!(u128, "u128");
    matrices!(i8, "i8");
    matrices!(i16, "i16");
    matrices!(i32, "i32");
    matrices!(i64, "i64");
    matrices!(i128, "i128");
    matrices!(f32, "f32");
    matrices!(f64, "f64");
    matrices!(bool, "bool");
    matrices!(String, "string");
    matrices!(crate::C64, "complex");
    matrices!(crate::R64, "rational");
    matrices!(usize, "matrix");

    Err(MechError::new(
        GenericError {
            msg: format!(
                "legacy adapter cannot project function state representation {:?}",
                port.representation(),
            ),
        },
        None,
    )
    .with_compiler_loc())
}

/// Projects a function output only inside the explicit legacy adapter.
pub fn legacy_function_output(function: &dyn MechFunction) -> MResult<LegacyValue> {
    if let Some(output) = function.reactive_output_value_cells().into_iter().next() {
        return LegacyValue::from_canonical_value(&output.snapshot()?);
    }
    let output = function.primary_output_state_port().ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!(
                    "legacy function {:?} does not expose a canonical primary output",
                    function.to_string(),
                ),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    legacy_state_port_value(output)
}

fn legacy_registered_instance(
    function: Box<dyn MechFunction>,
    arguments: &[LegacyValue],
) -> MResult<FunctionInstance> {
    let output = function
        .reactive_output_value_cells()
        .into_iter()
        .next()
        .map(Ok)
        .unwrap_or_else(|| {
            legacy_function_output(function.as_ref()).map(value_cell_from_legacy_function_value)
        })?;
    let inputs = arguments
        .iter()
        .cloned()
        .map(value_cell_from_legacy_function_value)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    Ok(FunctionInstance::new(
        function,
        FunctionInvocation::variadic(output, inputs),
    ))
}

/// Compatibility registration retained only for legacy tests and adapters.
pub trait LegacyReactivePlanRegistration {
    fn register(
        &mut self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<crate::ReactiveNodeId>;
}

impl LegacyReactivePlanRegistration for crate::ReactivePlan {
    fn register(
        &mut self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<crate::ReactiveNodeId> {
        self.register_instance_with_activation(
            legacy_registered_instance(function, arguments)?,
            None,
        )
    }
}

/// Compatibility registration retained only for legacy tests and adapters.
pub trait LegacyPlanRegistration {
    fn register_function(
        &self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<crate::ReactiveNodeId>;
}

impl LegacyPlanRegistration for crate::Plan {
    fn register_function(
        &self,
        function: Box<dyn MechFunction>,
        arguments: &[LegacyValue],
    ) -> MResult<crate::ReactiveNodeId> {
        self.register_instance(legacy_registered_instance(function, arguments)?)
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind1 {
    pub arg: ValueKind,
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind1 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind1"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kind for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind2 {
    pub arg: (ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind2 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind2"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind3 {
    pub arg: (ValueKind, ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind3 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind3"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKind4 {
    pub arg: (ValueKind, ValueKind, ValueKind, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKind4 {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind4"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKindVarg {
    pub arg: Vec<ValueKind>,
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentKindVarg {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKindVarg"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxes {
    pub arg: (ValueKind, Vec<ValueKind>, ValueKind),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentIxes {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentIxes"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentIxesMono {
    pub arg: (ValueKind, Vec<ValueKind>),
    pub fxn_name: String,
}
impl MechErrorKind for UnhandledFunctionArgumentIxesMono {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentIxesMono"
    }
    fn message(&self) -> String {
        format!(
            "Unhandled function argument kinds for function '{}': arg = {:?}",
            self.fxn_name, self.arg
        )
    }
}

fn exact_function_matrix(
    _value: &LegacyValue,
) -> Option<(FunctionMatrixElement, FunctionMatrixDescriptor)> {
    #[cfg(feature = "matrix")]
    {
        let matrix = _value.exact_matrix_any()?;
        macro_rules! exact_matrix {
            ($feature:literal, $type:ty, $element:ident) => {
                #[cfg(feature = $feature)]
                if let Some(matrix) = matrix.downcast_ref::<crate::matrix::Matrix<$type>>() {
                    return Some((FunctionMatrixElement::$element, matrix_descriptor(matrix)));
                }
            };
        }
        exact_matrix!("bool", bool, Bool);
        exact_matrix!("u8", u8, U8);
        exact_matrix!("u16", u16, U16);
        exact_matrix!("u32", u32, U32);
        exact_matrix!("u64", u64, U64);
        exact_matrix!("u128", u128, U128);
        exact_matrix!("i8", i8, I8);
        exact_matrix!("i16", i16, I16);
        exact_matrix!("i32", i32, I32);
        exact_matrix!("i64", i64, I64);
        exact_matrix!("i128", i128, I128);
        exact_matrix!("f32", f32, F32);
        exact_matrix!("f64", f64, F64);
        exact_matrix!("string", String, String);
        exact_matrix!("rational", crate::R64, R64);
        exact_matrix!("complex", crate::C64, C64);
        if let Some(matrix) = matrix.downcast_ref::<crate::matrix::Matrix<usize>>() {
            return Some((FunctionMatrixElement::Index, matrix_descriptor(matrix)));
        }
        if let Some(matrix) = matrix.downcast_ref::<crate::matrix::Matrix<LegacyValue>>() {
            return Some((FunctionMatrixElement::Value, matrix_descriptor(matrix)));
        }
    }
    None
}

fn non_ref_function_representation(value: &LegacyValue) -> FunctionValueRepresentation {
    if value.is_legacy_empty() {
        return FunctionValueRepresentation::Empty;
    }
    if value.is_legacy_index_all() {
        return FunctionValueRepresentation::Index;
    }
    match value.to_canonical_value() {
        Ok(snapshot) => {
            if let ValueData::Id(_) = snapshot.data() {
                return FunctionValueRepresentation::Id;
            }
            if let ValueData::Type(_) = snapshot.data() {
                return FunctionValueRepresentation::Kind;
            }
            if let ValueData::Option(option) = snapshot.data()
                && option.is_none()
            {
                return FunctionValueRepresentation::Empty;
            }
            FunctionValueRepresentation::AnyValue
        }
        Err(error) => {
            let legacy = error.kind_as::<super::LegacySnapshotError>();
            if matches!(
                legacy,
                Some(
                    super::LegacySnapshotError::LegacyEmptyNotSnapshot
                        | super::LegacySnapshotError::InvalidTypedEmptySchema
                )
            ) {
                return FunctionValueRepresentation::Empty;
            }
            if matches!(
                legacy,
                Some(super::LegacySnapshotError::LegacySelectionValueRequiresC3)
            ) {
                return FunctionValueRepresentation::Index;
            }
            FunctionValueRepresentation::AnyValue
        }
    }
}

impl LegacyValue {
    pub fn function_matrix_descriptor(
        &self,
        role: FunctionArgumentRole,
    ) -> MResult<Option<FunctionMatrixDescriptor>> {
        if let Some((_, descriptor)) = exact_function_matrix(self) {
            return Ok(Some(descriptor));
        }
        if matches!(
            FunctionValueRepresentation::from_value(self),
            FunctionValueRepresentation::AnyValue | FunctionValueRepresentation::MutableValueCell
        ) {
            return Err(MechError::new(
                FunctionArgumentTypeMismatch {
                    role,
                    expected: "an unwrapped scalar, nonmatrix, or exact matrix backing".to_string(),
                    found: self.exact_runtime_representation_name(),
                },
                None,
            )
            .with_compiler_loc());
        }
        Ok(None)
    }
}

/// Extracts only an exact `Ref<T>` compatibility backing representation.
///
/// This deliberately performs no scalar conversion, matrix reshaping, or
/// unwrapping of typed and mutable-reference values.
pub fn require_function_ref<T: 'static>(
    value: &LegacyValue,
    role: FunctionArgumentRole,
) -> MResult<Ref<T>> {
    value
        .exact_ref_any()
        .and_then(|backing| backing.downcast_ref::<Ref<T>>())
        .cloned()
        .ok_or_else(|| {
            MechError::new(
                FunctionArgumentTypeMismatch {
                    role,
                    expected: core::any::type_name::<Ref<T>>().to_string(),
                    found: value.exact_runtime_representation_name(),
                },
                None,
            )
            .with_compiler_loc()
        })
}

struct LegacyCellStorage {
    reference: Ref<LegacyValue>,
    representation: FunctionValueRepresentation,
}

#[cfg(feature = "matrix")]
fn replace_legacy_matrix_backing<T: Clone + 'static>(
    target: &crate::matrix::Matrix<T>,
    replacement: &crate::matrix::Matrix<T>,
) -> bool {
    macro_rules! replace {
        ($backing:ty) => {{
            if let (Some(left), Some(right)) = (
                target.exact_ref_any().downcast_ref::<Ref<$backing>>(),
                replacement.exact_ref_any().downcast_ref::<Ref<$backing>>(),
            ) {
                *left.borrow_mut() = right.borrow().clone();
                return true;
            }
        }};
    }
    #[cfg(feature = "matrix1")]
    replace!(crate::Matrix1<T>);
    #[cfg(feature = "matrix2")]
    replace!(crate::Matrix2<T>);
    #[cfg(feature = "matrix2x3")]
    replace!(crate::Matrix2x3<T>);
    #[cfg(feature = "matrix3x2")]
    replace!(crate::Matrix3x2<T>);
    #[cfg(feature = "matrix3")]
    replace!(crate::Matrix3<T>);
    #[cfg(feature = "matrix4")]
    replace!(crate::Matrix4<T>);
    #[cfg(feature = "vector2")]
    replace!(crate::Vector2<T>);
    #[cfg(feature = "vector3")]
    replace!(crate::Vector3<T>);
    #[cfg(feature = "vector4")]
    replace!(crate::Vector4<T>);
    #[cfg(feature = "row_vector2")]
    replace!(crate::RowVector2<T>);
    #[cfg(feature = "row_vector3")]
    replace!(crate::RowVector3<T>);
    #[cfg(feature = "row_vector4")]
    replace!(crate::RowVector4<T>);
    #[cfg(feature = "vectord")]
    replace!(crate::DVector<T>);
    #[cfg(feature = "row_vectord")]
    replace!(crate::RowDVector<T>);
    #[cfg(feature = "matrixd")]
    replace!(crate::DMatrix<T>);
    false
}

#[cfg(feature = "matrix")]
fn replace_legacy_value_matrix(
    target: &crate::matrix::Matrix<LegacyValue>,
    replacement: &crate::matrix::Matrix<LegacyValue>,
) -> bool {
    if target.rows() == replacement.rows() && target.cols() == replacement.cols() {
        for index in 1..=target.rows().saturating_mul(target.cols()) {
            let mut target_value = target.index1d(index);
            replace_legacy_value_preserving_handles(&mut target_value, &replacement.index1d(index));
            target.set_index1d(index - 1, target_value);
        }
        return true;
    }

    #[cfg(any(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
    let values = replacement.as_vec();
    #[cfg(feature = "vectord")]
    if let crate::matrix::Matrix::DVector(target) = target
        && replacement.cols() == 1
    {
        *target.borrow_mut() = crate::DVector::from_vec(values);
        return true;
    }
    #[cfg(feature = "row_vectord")]
    if let crate::matrix::Matrix::RowDVector(target) = target
        && replacement.rows() == 1
    {
        *target.borrow_mut() = crate::RowDVector::from_vec(values);
        return true;
    }
    #[cfg(feature = "matrixd")]
    if let crate::matrix::Matrix::DMatrix(target) = target {
        *target.borrow_mut() =
            crate::DMatrix::from_vec(replacement.rows(), replacement.cols(), values);
        return true;
    }
    false
}

fn replace_legacy_value_preserving_handles(target: &mut LegacyValue, replacement: &LegacyValue) {
    if let LegacyValue::Typed(target_inner, _) = target {
        let replacement_inner = match replacement {
            LegacyValue::Typed(replacement_inner, _) => replacement_inner.as_ref(),
            replacement => replacement,
        };
        replace_legacy_value_preserving_handles(target_inner, replacement_inner);
        return;
    }
    if let LegacyValue::Typed(replacement_inner, _) = replacement {
        replace_legacy_value_preserving_handles(target, replacement_inner);
        return;
    }

    fn exact_refs<T: 'static>(
        target: &LegacyValue,
        replacement: &LegacyValue,
    ) -> Option<(Ref<T>, Ref<T>)> {
        Some((
            target.exact_ref_any()?.downcast_ref::<Ref<T>>()?.clone(),
            replacement
                .exact_ref_any()?
                .downcast_ref::<Ref<T>>()?
                .clone(),
        ))
    }
    macro_rules! replace_exact {
        ($feature:literal, $type:ty) => {
            #[cfg(feature = $feature)]
            if let Some((left, right)) = exact_refs::<$type>(target, replacement) {
                *left.borrow_mut() = right.borrow().clone();
                return;
            }
        };
    }
    replace_exact!("u8", u8);
    replace_exact!("u16", u16);
    replace_exact!("u32", u32);
    replace_exact!("u64", u64);
    replace_exact!("u128", u128);
    replace_exact!("i8", i8);
    replace_exact!("i16", i16);
    replace_exact!("i32", i32);
    replace_exact!("i64", i64);
    replace_exact!("i128", i128);
    replace_exact!("f32", f32);
    replace_exact!("f64", f64);
    replace_exact!("string", String);
    replace_exact!("bool", bool);
    replace_exact!("complex", crate::C64);
    replace_exact!("rational", crate::R64);
    replace_exact!("atom", crate::MechAtom);
    replace_exact!("enum", crate::MechEnum);
    replace_exact!("set", crate::MechSet);
    if let Some((left, right)) = exact_refs::<usize>(target, replacement) {
        *left.borrow_mut() = *right.borrow();
        return;
    }

    #[cfg(feature = "record")]
    if let Some((left, right)) = exact_refs::<crate::MechRecord>(target, replacement) {
        let mut left = left.borrow_mut();
        let right = right.borrow();
        for (id, replacement) in &right.data {
            if let Some(target) = left.data.get_mut(id) {
                replace_legacy_value_preserving_handles(target, replacement);
            }
        }
        left.cols = right.cols;
        left.kinds = right.kinds.clone();
        left.field_names = right.field_names.clone();
        return;
    }
    #[cfg(feature = "tuple")]
    if let Some((left, right)) = exact_refs::<crate::MechTuple>(target, replacement) {
        let mut left = left.borrow_mut();
        let right = right.borrow();
        if left.elements.len() == right.elements.len() {
            for (target, replacement) in left.elements.iter_mut().zip(&right.elements) {
                replace_legacy_value_preserving_handles(target, replacement);
            }
        } else {
            *left = right.clone();
        }
        return;
    }
    #[cfg(feature = "map")]
    if let Some((left, right)) = exact_refs::<crate::MechMap>(target, replacement) {
        let mut left = left.borrow_mut();
        let right = right.borrow();
        if left.map.len() == right.map.len()
            && right.map.keys().all(|key| left.map.contains_key(key))
        {
            for (key, replacement) in &right.map {
                replace_legacy_value_preserving_handles(
                    left.map.get_mut(key).expect("validated map key"),
                    replacement,
                );
            }
            left.key_kind = right.key_kind.clone();
            left.value_kind = right.value_kind.clone();
            left.num_elements = right.num_elements;
        } else {
            *left = right.clone();
        }
        return;
    }
    #[cfg(feature = "table")]
    if let Some((left, right)) = exact_refs::<crate::MechTable>(target, replacement) {
        let mut left = left.borrow_mut();
        let right = right.borrow();
        if left.data.len() == right.data.len()
            && right.data.keys().all(|key| left.data.contains_key(key))
        {
            for (id, (kind, replacement)) in &right.data {
                let (target_kind, target) = left.data.get_mut(id).expect("validated table column");
                *target_kind = kind.clone();
                if !replace_legacy_value_matrix(target, replacement) {
                    *target = replacement.clone();
                }
            }
            left.rows = right.rows;
            left.cols = right.cols;
            left.col_names = right.col_names.clone();
        } else {
            *left = right.clone();
        }
        return;
    }

    #[cfg(feature = "matrix")]
    if let (Some(left), Some(right)) = (target.exact_matrix_any(), replacement.exact_matrix_any()) {
        macro_rules! replace_matrix {
            ($feature:literal, $type:ty) => {
                #[cfg(feature = $feature)]
                if let (Some(left), Some(right)) = (
                    left.downcast_ref::<crate::matrix::Matrix<$type>>(),
                    right.downcast_ref::<crate::matrix::Matrix<$type>>(),
                ) && replace_legacy_matrix_backing(left, right)
                {
                    return;
                }
            };
        }
        replace_matrix!("bool", bool);
        replace_matrix!("u8", u8);
        replace_matrix!("u16", u16);
        replace_matrix!("u32", u32);
        replace_matrix!("u64", u64);
        replace_matrix!("u128", u128);
        replace_matrix!("i8", i8);
        replace_matrix!("i16", i16);
        replace_matrix!("i32", i32);
        replace_matrix!("i64", i64);
        replace_matrix!("i128", i128);
        replace_matrix!("f32", f32);
        replace_matrix!("f64", f64);
        replace_matrix!("string", String);
        replace_matrix!("rational", crate::R64);
        replace_matrix!("complex", crate::C64);
        if let (Some(left), Some(right)) = (
            left.downcast_ref::<crate::matrix::Matrix<usize>>(),
            right.downcast_ref::<crate::matrix::Matrix<usize>>(),
        ) && replace_legacy_matrix_backing(left, right)
        {
            return;
        }
        if let (Some(left), Some(right)) = (
            left.downcast_ref::<crate::matrix::Matrix<LegacyValue>>(),
            right.downcast_ref::<crate::matrix::Matrix<LegacyValue>>(),
        ) && replace_legacy_value_matrix(left, right)
        {
            return;
        }
    }

    *target = replacement.clone();
}

fn preflight_structured_legacy_replacement(
    target: &LegacyValue,
    replacement: &LegacyValue,
    expected: &crate::Value,
) -> MResult<()> {
    let structured = matches!(
        FunctionValueRepresentation::from_value(target),
        FunctionValueRepresentation::Record
            | FunctionValueRepresentation::Map
            | FunctionValueRepresentation::Table
            | FunctionValueRepresentation::Tuple
            | FunctionValueRepresentation::Matrix {
                element: FunctionMatrixElement::Value,
                ..
            }
    );
    if !structured {
        return Ok(());
    }

    let mut candidate = target.try_deep_snapshot()?;
    replace_legacy_value_preserving_handles(&mut candidate, replacement);
    let actual = candidate.to_canonical_value()?;
    let snapshot_error = |error: crate::SnapshotValueError| {
        MechError::new(
            GenericError {
                msg: format!("{error:?}"),
            },
            None,
        )
        .with_compiler_loc()
    };
    let actual_data = actual.canonical_data_draft().map_err(snapshot_error)?;
    let expected_data = expected.canonical_data_draft().map_err(snapshot_error)?;
    if actual.schema_key() != expected.schema_key()
        || actual.shape() != expected.shape()
        || actual_data != expected_data
    {
        return Err(MechError::new(
            GenericError {
                msg: "stable structured update cannot preserve aliased child cell".into(),
            },
            None,
        )
        .with_compiler_loc());
    }
    Ok(())
}

impl crate::cell_binding::ErasedCellStorage for LegacyCellStorage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation(&self, _schema: &crate::SchemaBody) -> FunctionValueRepresentation {
        self.representation
    }

    fn snapshot(
        &self,
        _schema: crate::SchemaId,
        _shape: &crate::ShapeInstance,
        _schemas: &crate::SchemaTable,
    ) -> MResult<crate::Value> {
        let value = self
            .reference
            .try_borrow()
            .map_err(|_| crate::cell_binding::borrow_conflict(crate::CellAccess::Snapshot))?;
        match value.to_canonical_value() {
            Ok(value) => Ok(value),
            #[cfg(feature = "set")]
            Err(_) if self.representation == FunctionValueRepresentation::Set => {
                crate::cell_binding::finalize_draft(
                    _schema,
                    _shape,
                    _schemas,
                    ValueDataDraft::Set(Vec::new().into_boxed_slice()),
                )
            }
            Err(error) => Err(error),
        }
    }

    fn replace(&self, value: &crate::Value) -> MResult<()> {
        #[cfg(feature = "matrix")]
        let replacement = if let FunctionValueRepresentation::Matrix { element, storage } =
            self.representation
        {
            let canonical = ValueCell::from_snapshot(value.clone())?;
            let descriptor = crate::function::argument::canonical_matrix_descriptor(&canonical)?
                .ok_or_else(|| {
                    MechError::new(
                        LegacySpecializationProjectionUnsupported {
                            representation: self.representation,
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
            let exact = ValueCell::default_for_representation(
                self.representation,
                Some((descriptor.rows, descriptor.cols)),
            )?;
            // The compatibility representation may select a fixed or dynamic
            // nalgebra backing whose storage schema differs from the canonical
            // snapshot's source table. The owning `ValueCell` already checked
            // semantic schema and extent evolution before dispatching here;
            // populate only the fresh projection backing at this boundary.
            exact.binding.storage.replace(value)?;
            legacy_matrix_specialization_input(&exact, element, storage, 0)?
        } else {
            LegacyValue::from_canonical_value(value)?
        };
        #[cfg(not(feature = "matrix"))]
        let replacement = LegacyValue::from_canonical_value(value)?;
        let mut target = self
            .reference
            .try_borrow_mut()
            .map_err(|_| crate::cell_binding::borrow_conflict(crate::CellAccess::Replace))?;
        preflight_structured_legacy_replacement(&target, &replacement, value)?;
        replace_legacy_value_preserving_handles(&mut target, &replacement);
        Ok(())
    }

    fn preflight_replace(&self) -> MResult<()> {
        self.reference
            .try_borrow_mut()
            .map(|_| ())
            .map_err(|_| crate::cell_binding::borrow_conflict(crate::CellAccess::Replace))
    }

    fn detached_clone(&self) -> MResult<Rc<dyn crate::cell_binding::ErasedCellStorage>> {
        let value = self
            .reference
            .try_borrow()
            .map_err(|_| crate::cell_binding::borrow_conflict(crate::CellAccess::Snapshot))?
            .try_deep_snapshot()?;
        Ok(Rc::new(Self {
            reference: Ref::new(value),
            representation: self.representation,
        }))
    }

    fn same_storage(&self, other: &dyn crate::cell_binding::ErasedCellStorage) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| self.reference.same_handle(&other.reference))
    }

    fn borrow_state(&self) -> crate::cell_binding::CellBorrowState {
        if self.reference.try_borrow().is_ok() {
            crate::cell_binding::CellBorrowState::Available
        } else {
            crate::cell_binding::CellBorrowState::Borrowed
        }
    }

    fn logical_cell_id(&self) -> crate::CanonicalCellId {
        self.reference.reactive_cell_id()
    }
}

impl ValueCell {
    #[doc(hidden)]
    pub fn new(value: LegacyValue) -> Self {
        Self::from_legacy_ref(Ref::new(value))
    }

    #[doc(hidden)]
    pub fn from_legacy_ref(reference: Ref<LegacyValue>) -> Self {
        let identity = reference.reactive_cell_id();
        Self::from_legacy_ref_with_identity(reference, identity)
    }

    fn from_legacy_ref_with_identity(
        reference: Ref<LegacyValue>,
        identity: crate::CanonicalCellId,
    ) -> Self {
        let representation = FunctionValueRepresentation::from_value(&reference.borrow());
        let canonical = reference
            .try_borrow()
            .ok()
            .and_then(|value| value.to_canonical_value().ok());
        let (schema, schema_key, shape, schemas) = if let Some(value) = canonical {
            let schemas = Rc::new(
                (*value
                    .schemas()
                    .expect("canonicalized compatibility value retains schemas"))
                .clone(),
            );
            (
                value.schema(),
                value.schema_key(),
                value.shape().clone(),
                schemas,
            )
        } else {
            let (schema, shape, schemas) = crate::cell_binding::compatibility_unit_schema();
            let schema_key = schemas
                .entry(schema)
                .expect("compatibility unit schema exists")
                .key();
            (schema, schema_key, shape, schemas)
        };
        Self {
            binding: crate::cell_binding::CellBinding {
                identity,
                schema,
                schema_key,
                shape: Rc::new(cell::RefCell::new(shape)),
                schemas,
                storage: Rc::new(LegacyCellStorage {
                    reference,
                    representation,
                }),
                compiler_children: None,
            },
        }
    }

    #[doc(hidden)]
    pub fn legacy_ref(&self) -> Ref<LegacyValue> {
        self.legacy_ref_compat()
            .expect("legacy compatibility requires a legacy-backed value cell")
    }

    pub(crate) fn legacy_ref_compat(&self) -> Option<Ref<LegacyValue>> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .map(|storage| storage.reference.clone())
    }

    #[doc(hidden)]
    pub fn borrow(&self) -> cell::Ref<'_, LegacyValue> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .borrow()
    }

    #[doc(hidden)]
    pub fn borrow_mut(&self) -> cell::RefMut<'_, LegacyValue> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .borrow_mut()
    }

    #[doc(hidden)]
    pub fn try_borrow(&self) -> Result<cell::Ref<'_, LegacyValue>, cell::BorrowError> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .try_borrow()
    }

    #[doc(hidden)]
    pub fn try_borrow_mut(&self) -> Result<cell::RefMut<'_, LegacyValue>, cell::BorrowMutError> {
        self.binding
            .storage
            .as_any()
            .downcast_ref::<LegacyCellStorage>()
            .expect("legacy compatibility requires a legacy-backed value cell")
            .reference
            .try_borrow_mut()
    }
}

#[derive(Clone, Debug)]
pub enum FunctionArgs {
    Nullary(LegacyValue),
    Unary(LegacyValue, LegacyValue),
    Binary(LegacyValue, LegacyValue, LegacyValue),
    Ternary(LegacyValue, LegacyValue, LegacyValue, LegacyValue),
    Quaternary(
        LegacyValue,
        LegacyValue,
        LegacyValue,
        LegacyValue,
        LegacyValue,
    ),
    Variadic(LegacyValue, Vec<LegacyValue>),
}

impl FunctionArgs {
    #[cfg(test)]
    pub(crate) fn normalize_for_signature(
        self,
        signature: crate::RuntimeFunctionSignature,
    ) -> Self {
        if !matches!(
            signature.inputs,
            crate::RuntimeFunctionInputs::Variadic { .. }
        ) {
            return self;
        }
        match self {
            Self::Nullary(output) => Self::Variadic(output, Vec::new()),
            Self::Unary(output, a) => Self::Variadic(output, vec![a]),
            Self::Binary(output, a, b) => Self::Variadic(output, vec![a, b]),
            Self::Ternary(output, a, b, c) => Self::Variadic(output, vec![a, b, c]),
            Self::Quaternary(output, a, b, c, d) => Self::Variadic(output, vec![a, b, c, d]),
            args @ Self::Variadic(_, _) => args,
        }
    }

    pub fn output_value(&self) -> &LegacyValue {
        match self {
            Self::Nullary(output)
            | Self::Unary(output, _)
            | Self::Binary(output, _, _)
            | Self::Ternary(output, _, _, _)
            | Self::Quaternary(output, _, _, _, _)
            | Self::Variadic(output, _) => output,
        }
    }

    pub fn input_value(&self, index: usize) -> Option<&LegacyValue> {
        match self {
            Self::Nullary(_) => None,
            Self::Unary(_, a) => [a].get(index).copied(),
            Self::Binary(_, a, b) => [a, b].get(index).copied(),
            Self::Ternary(_, a, b, c) => [a, b, c].get(index).copied(),
            Self::Quaternary(_, a, b, c, d) => [a, b, c, d].get(index).copied(),
            Self::Variadic(_, arguments) => arguments.get(index),
        }
    }

    pub fn input_count(&self) -> usize {
        self.len()
    }

    pub fn validate_contract(&self, contract: crate::RuntimeFunctionContract) -> MResult<()> {
        if contract.output_alias == crate::RuntimeOutputAliasPolicy::DisallowInputAlias {
            let output_roots = self.output_value().reactive_root_cell_ids();
            for index in 0..self.input_count() {
                let Some(input) = self.input_value(index) else {
                    continue;
                };
                for cell in input.reactive_root_cell_ids() {
                    if output_roots.contains(&cell) {
                        return Err(MechError::new(
                            crate::FunctionArgumentAliasViolation { input: index, cell },
                            None,
                        )
                        .with_compiler_loc());
                    }
                }
            }
        }
        (contract.validate_shapes)(self)
    }

    pub fn validate_signature(&self, signature: crate::RuntimeFunctionSignature) -> MResult<()> {
        use crate::RuntimeFunctionInputs;
        let arity_kind_matches = matches!(
            (self, signature.inputs),
            (Self::Nullary(_), RuntimeFunctionInputs::Nullary)
                | (Self::Unary(_, _), RuntimeFunctionInputs::Unary(_))
                | (Self::Binary(_, _, _), RuntimeFunctionInputs::Binary(_, _))
                | (
                    Self::Ternary(_, _, _, _),
                    RuntimeFunctionInputs::Ternary(_, _, _)
                )
                | (
                    Self::Quaternary(_, _, _, _, _),
                    RuntimeFunctionInputs::Quaternary(_, _, _, _)
                )
                | (Self::Variadic(_, _), RuntimeFunctionInputs::Variadic { .. })
        );
        let expected_inputs: Vec<FunctionValueRepresentation> = match signature.inputs {
            RuntimeFunctionInputs::Nullary => Vec::new(),
            RuntimeFunctionInputs::Unary(argument) => vec![argument],
            RuntimeFunctionInputs::Binary(lhs, rhs) => vec![lhs, rhs],
            RuntimeFunctionInputs::Ternary(first, second, third) => vec![first, second, third],
            RuntimeFunctionInputs::Quaternary(first, second, third, fourth) => {
                vec![first, second, third, fourth]
            }
            RuntimeFunctionInputs::Variadic { element } => vec![element; self.input_count()],
        };
        if !arity_kind_matches || expected_inputs.len() != self.input_count() {
            return Err(MechError::new(
                crate::IncorrectNumberOfArguments {
                    expected: expected_inputs.len(),
                    found: self.input_count(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let found_output = FunctionValueRepresentation::from_value(self.output_value());
        if !signature.output.matches(found_output) {
            return Err(crate::signature_violation(
                crate::FunctionArgumentRole::Output,
                signature.output,
                self.output_value(),
            ));
        }
        for (index, expected) in expected_inputs.into_iter().enumerate() {
            let input = self.input_value(index).expect("validated function arity");
            let found = FunctionValueRepresentation::from_value(input);
            if !expected.matches(found) {
                return Err(crate::signature_violation(
                    crate::FunctionArgumentRole::Input(index),
                    expected,
                    input,
                ));
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Nullary(_) => 0,
            Self::Unary(_, _) => 1,
            Self::Binary(_, _, _) => 2,
            Self::Ternary(_, _, _, _) => 3,
            Self::Quaternary(_, _, _, _, _) => 4,
            Self::Variadic(_, args) => args.len(),
        }
    }

    pub fn input_values(&self) -> Vec<LegacyValue> {
        match self {
            Self::Nullary(_) => Vec::new(),
            Self::Unary(_, a) => vec![a.clone()],
            Self::Binary(_, a, b) => vec![a.clone(), b.clone()],
            Self::Ternary(_, a, b, c) => vec![a.clone(), b.clone(), c.clone()],
            Self::Quaternary(_, a, b, c, d) => {
                vec![a.clone(), b.clone(), c.clone(), d.clone()]
            }
            Self::Variadic(_, arguments) => arguments.clone(),
        }
    }
}

impl crate::FunctionRuntimeType for LegacyValue {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::AnyValue;
}

impl FunctionValueRepresentation {
    pub fn from_value(value: &LegacyValue) -> Self {
        if let Some((element, descriptor)) = exact_function_matrix(value) {
            return Self::Matrix {
                element,
                storage: FunctionMatrixStoragePattern::Exact(descriptor.representation),
            };
        }
        let Some(backing) = value.exact_ref_any() else {
            return non_ref_function_representation(value);
        };
        macro_rules! exact_ref {
            ($feature:literal, $type:ty, $representation:ident) => {
                #[cfg(feature = $feature)]
                if backing.is::<Ref<$type>>() {
                    return Self::$representation;
                }
            };
        }
        exact_ref!("u8", u8, U8);
        exact_ref!("u16", u16, U16);
        exact_ref!("u32", u32, U32);
        exact_ref!("u64", u64, U64);
        exact_ref!("u128", u128, U128);
        exact_ref!("i8", i8, I8);
        exact_ref!("i16", i16, I16);
        exact_ref!("i32", i32, I32);
        exact_ref!("i64", i64, I64);
        exact_ref!("i128", i128, I128);
        exact_ref!("f32", f32, F32);
        exact_ref!("f64", f64, F64);
        exact_ref!("string", String, String);
        exact_ref!("bool", bool, Bool);
        exact_ref!("complex", crate::C64, C64);
        exact_ref!("rational", crate::R64, R64);
        exact_ref!("atom", crate::MechAtom, Atom);
        exact_ref!("enum", crate::MechEnum, Enum);
        exact_ref!("record", crate::MechRecord, Record);
        exact_ref!("map", crate::MechMap, Map);
        exact_ref!("set", crate::MechSet, Set);
        exact_ref!("table", crate::MechTable, Table);
        exact_ref!("tuple", crate::MechTuple, Tuple);
        if backing.is::<Ref<usize>>() {
            return Self::Index;
        }
        if backing.is::<Ref<LegacyValue>>() {
            return Self::MutableValueCell;
        }
        non_ref_function_representation(value)
    }
}

pub(crate) fn signature_violation(
    role: crate::FunctionArgumentRole,
    expected: FunctionValueRepresentation,
    value: &LegacyValue,
) -> MechError {
    MechError::new(
        crate::FunctionSignatureViolation {
            role,
            expected,
            found: FunctionValueRepresentation::from_value(value),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(feature = "matrix")]
use crate::matrix::Matrix;

#[cfg(feature = "no_std")]
use alloc::{string::String, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

#[cfg(feature = "no_std")]
use alloc::sync::Arc;
#[cfg(not(feature = "no_std"))]
use std::sync::Arc;

pub fn construct_compatibility_function<F>(args: FunctionArgs) -> MResult<Box<dyn MechFunction>>
where
    F: MechFunctionFactory,
{
    if F::OUTPUT_SCHEMA_RULE == FunctionOutputSchemaRule::Declared {
        return F::new_invocation(function_invocation_from_legacy(args));
    }
    construct_dynamic_set_compatibility_function::<F>(args)
}

#[cfg(feature = "set")]
fn construct_dynamic_set_compatibility_function<F>(
    args: FunctionArgs,
) -> MResult<Box<dyn MechFunction>>
where
    F: MechFunctionFactory,
{
    let sink = require_function_ref::<crate::MechSet>(
        args.output_value(),
        crate::FunctionArgumentRole::Output,
    )?;
    let converted = function_invocation_from_legacy(args);
    let inputs = converted.input_cells().to_vec();
    let element = match F::OUTPUT_SCHEMA_RULE {
        FunctionOutputSchemaRule::Declared => unreachable!(),
        FunctionOutputSchemaRule::DynamicSetLikeInput(index) => {
            set_element_schema(inputs.get(index).ok_or_else(|| {
                MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected: index + 1,
                        found: inputs.len(),
                    },
                    None,
                )
                .with_compiler_loc()
            })?)?
        }
        FunctionOutputSchemaRule::DynamicSetCartesianProduct => {
            if inputs.len() != 2 {
                return Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected: 2,
                        found: inputs.len(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            SchemaBody::Tuple(
                vec![
                    set_element_schema(&inputs[0])?,
                    set_element_schema(&inputs[1])?,
                ]
                .into_boxed_slice(),
            )
        }
        FunctionOutputSchemaRule::DynamicSetPowerset => {
            if inputs.len() != 1 {
                return Err(MechError::new(
                    crate::IncorrectNumberOfArguments {
                        expected: 1,
                        found: inputs.len(),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            SchemaBody::Set {
                element: Box::new(set_element_schema(&inputs[0])?),
                cardinality: crate::CardinalitySpec::Dynamic { upper_bound: None },
            }
        }
    };
    let output = ValueCell::empty_dynamic_set(element)?;
    let invocation = compatibility_invocation(F::SIGNATURE.inputs, output.clone(), inputs)?;
    let implementation = F::new_invocation(invocation)?;
    Ok(Box::new(DynamicSetCompatibilitySink {
        implementation,
        output,
        sink,
    }))
}

#[cfg(not(feature = "set"))]
fn construct_dynamic_set_compatibility_function<F>(
    _args: FunctionArgs,
) -> MResult<Box<dyn MechFunction>>
where
    F: MechFunctionFactory,
{
    unreachable!("dynamic set compatibility requires the set feature")
}

#[cfg(feature = "set")]
fn set_element_schema(cell: &ValueCell) -> MResult<SchemaBody> {
    let body = cell.closed_schema_body()?;
    let SchemaBody::Set { element, .. } = body else {
        return Err(MechError::new(
            crate::FunctionArgumentTypeMismatch {
                role: crate::FunctionArgumentRole::Input(0),
                expected: "canonical Set value".into(),
                found: format!("{:?}", cell.representation()),
            },
            None,
        )
        .with_compiler_loc());
    };
    Ok(*element)
}

#[cfg(feature = "set")]
fn compatibility_invocation(
    inputs: RuntimeFunctionInputs,
    output: ValueCell,
    values: Vec<ValueCell>,
) -> MResult<FunctionInvocation> {
    let found = values.len();
    let invocation = match (inputs, values.as_slice()) {
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
        (RuntimeFunctionInputs::Variadic { .. }, values) => {
            FunctionInvocation::variadic(output, values.to_vec().into_boxed_slice())
        }
        (inputs, _) => {
            let expected = match inputs {
                RuntimeFunctionInputs::Nullary => 0,
                RuntimeFunctionInputs::Unary(_) => 1,
                RuntimeFunctionInputs::Binary(_, _) => 2,
                RuntimeFunctionInputs::Ternary(_, _, _) => 3,
                RuntimeFunctionInputs::Quaternary(_, _, _, _) => 4,
                RuntimeFunctionInputs::Variadic { .. } => found,
            };
            return Err(MechError::new(
                crate::IncorrectNumberOfArguments { expected, found },
                None,
            )
            .with_compiler_loc());
        }
    };
    Ok(invocation)
}

#[cfg(feature = "set")]
struct DynamicSetCompatibilitySink {
    implementation: Box<dyn MechFunction>,
    output: ValueCell,
    sink: crate::Ref<crate::MechSet>,
}

#[cfg(feature = "set")]
struct DynamicSetCompatibilityState {
    target: crate::Ref<crate::MechSet>,
    before: crate::Value,
    after: Option<crate::Value>,
}

#[cfg(feature = "set")]
impl DynamicSetCompatibilityState {
    fn new(target: crate::Ref<crate::MechSet>) -> MResult<Self> {
        let before = target.to_value().to_canonical_value()?;
        Ok(Self {
            target,
            before,
            after: None,
        })
    }

    fn materialize(value: &crate::Value) -> MResult<crate::MechSet> {
        let legacy = LegacyValue::from_canonical_value(value)?;
        let value =
            require_function_ref::<crate::MechSet>(&legacy, crate::FunctionArgumentRole::Output)?;
        let value = value.try_borrow().map_err(|_| {
            MechError::new(
                crate::ValueStateBorrowConflict {
                    phase: "materialize compatibility checkpoint",
                    type_name: core::any::type_name::<crate::MechSet>(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        Ok(value.clone())
    }

    fn preflight_restore(&self, value: &crate::Value, phase: &'static str) -> MResult<()> {
        self.target.try_borrow_mut().map(|_| ()).map_err(|_| {
            MechError::new(
                crate::ValueStateBorrowConflict {
                    phase,
                    type_name: core::any::type_name::<crate::MechSet>(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        Self::materialize(value).map(|_| ())
    }

    fn apply_restore(&self, value: &crate::Value) {
        *self.target.borrow_mut() =
            Self::materialize(value).expect("compatibility set restore was preflighted");
    }
}

#[cfg(feature = "set")]
impl crate::CustomValueStateEntry for DynamicSetCompatibilityState {
    fn capture_after(&mut self) -> MResult<()> {
        self.after = Some(self.target.to_value().to_canonical_value()?);
        Ok(())
    }

    fn preflight_restore_before(&self) -> MResult<()> {
        self.preflight_restore(&self.before, "restore-before")
    }

    fn preflight_restore_after(&self) -> MResult<()> {
        self.after.as_ref().map_or(Ok(()), |after| {
            self.preflight_restore(after, "restore-after")
        })
    }

    fn apply_restore_before(&self) {
        self.apply_restore(&self.before);
    }

    fn apply_restore_after(&self) {
        if let Some(after) = &self.after {
            self.apply_restore(after);
        }
    }
}

#[cfg(feature = "set")]
impl DynamicSetCompatibilitySink {
    fn project_output(&self) -> MResult<()> {
        let legacy = LegacyValue::from_canonical_value(&self.output.snapshot()?)?;
        let projected =
            require_function_ref::<crate::MechSet>(&legacy, crate::FunctionArgumentRole::Output)?;
        *self.sink.borrow_mut() = projected.borrow().clone();
        Ok(())
    }
}

#[cfg(feature = "set")]
impl crate::MechFunctionImpl for DynamicSetCompatibilitySink {
    fn solve_result(&self) -> MResult<()> {
        self.implementation.solve_result()?;
        self.project_output()
    }

    fn solve_result_with(&self, services: &mut dyn crate::MechExecutionServices) -> MResult<()> {
        self.implementation.solve_result_with(services)?;
        self.project_output()
    }

    fn primary_output_state_port(&self) -> Option<crate::FunctionStatePort<'_>> {
        Some(crate::FunctionStatePort::from_cell(&self.output))
    }

    fn capture_retained_state(
        &self,
        journal: &mut crate::function::state::FunctionCheckpoint,
    ) -> MResult<()> {
        journal.capture_custom_entry(Box::new(DynamicSetCompatibilityState::new(
            self.sink.clone(),
        )?))
    }

    fn semantic_operation_contract(&self) -> Option<&'static crate::OperationContractDeclaration> {
        self.implementation.semantic_operation_contract()
    }

    fn semantic_operation_name(&self) -> Option<&str> {
        self.implementation.semantic_operation_name()
    }

    fn to_string(&self) -> String {
        self.implementation.to_string()
    }
}

#[cfg(all(feature = "set", feature = "semantic-compiler"))]
impl crate::MechFunctionCompiler for DynamicSetCompatibilitySink {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        self.implementation.compiler_owned_value_cells()
    }

    fn reserve_bytecode_registers(
        &self,
        context: &mut dyn crate::BytecodeCompilerContext,
    ) -> MResult<()> {
        self.implementation.reserve_bytecode_registers(context)
    }

    fn compile(
        &self,
        context: &mut dyn crate::BytecodeCompilerContext,
    ) -> MResult<crate::Register> {
        self.implementation.compile(context)
    }
}

struct LegacyFunctionSpecializerAdapter {
    inner: Arc<dyn FunctionSpecializer>,
}

impl CanonicalFunctionSpecializer for LegacyFunctionSpecializerAdapter {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        specialize_legacy_function(self.inner.as_ref(), invocation, context)
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        self.inner.guard_safety()
    }
}

pub fn canonical_function_specializer(
    specializer: Arc<dyn FunctionSpecializer>,
) -> Arc<dyn CanonicalFunctionSpecializer> {
    Arc::new(LegacyFunctionSpecializerAdapter { inner: specializer })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacySpecializationControlUnsupported {
    pub control: &'static str,
}

impl MechErrorKind for LegacySpecializationControlUnsupported {
    fn name(&self) -> &str {
        "LegacySpecializationControlUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "legacy source control `{}` has no canonical specialization input",
            self.control,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LegacySpecializationProjectionUnsupported {
    pub representation: FunctionValueRepresentation,
}

impl MechErrorKind for LegacySpecializationProjectionUnsupported {
    fn name(&self) -> &str {
        "LegacySpecializationProjectionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "canonical specialization input {:?} cannot be projected into the legacy adapter",
            self.representation,
        )
    }
}

impl From<FunctionArgs> for FunctionInvocation {
    fn from(args: FunctionArgs) -> Self {
        function_invocation_from_legacy(args)
    }
}

pub fn function_invocation_from_legacy(args: FunctionArgs) -> FunctionInvocation {
    let (layout, output, inputs) = match args {
        FunctionArgs::Nullary(output) => (FunctionInvocationLayout::Nullary, output, Vec::new()),
        FunctionArgs::Unary(output, input) => {
            (FunctionInvocationLayout::Unary, output, vec![input])
        }
        FunctionArgs::Binary(output, first, second) => (
            FunctionInvocationLayout::Binary,
            output,
            vec![first, second],
        ),
        FunctionArgs::Ternary(output, first, second, third) => (
            FunctionInvocationLayout::Ternary,
            output,
            vec![first, second, third],
        ),
        FunctionArgs::Quaternary(output, first, second, third, fourth) => (
            FunctionInvocationLayout::Quaternary,
            output,
            vec![first, second, third, fourth],
        ),
        FunctionArgs::Variadic(output, inputs) => {
            (FunctionInvocationLayout::Variadic, output, inputs)
        }
    };
    FunctionInvocation::from_cells(
        layout,
        value_cell_from_legacy_function_value(output),
        inputs
            .into_iter()
            .map(value_cell_from_legacy_function_value)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
    .expect("legacy function arguments retain a valid invocation layout")
}

/// Builds a canonical factory invocation from compatibility arguments.
///
/// Exact scalar and matrix handles remain shared with the caller. Universal
/// aggregate inputs become immutable canonical snapshots, so normal factory
/// implementations never receive a compatibility-backed cell.
pub fn try_function_invocation_from_legacy(
    args: FunctionArgs,
    signature: crate::RuntimeFunctionSignature,
) -> MResult<FunctionInvocation> {
    let (layout, output, inputs) = match args {
        FunctionArgs::Nullary(output) => (FunctionInvocationLayout::Nullary, output, Vec::new()),
        FunctionArgs::Unary(output, input) => {
            (FunctionInvocationLayout::Unary, output, vec![input])
        }
        FunctionArgs::Binary(output, first, second) => (
            FunctionInvocationLayout::Binary,
            output,
            vec![first, second],
        ),
        FunctionArgs::Ternary(output, first, second, third) => (
            FunctionInvocationLayout::Ternary,
            output,
            vec![first, second, third],
        ),
        FunctionArgs::Quaternary(output, first, second, third, fourth) => (
            FunctionInvocationLayout::Quaternary,
            output,
            vec![first, second, third, fourth],
        ),
        FunctionArgs::Variadic(output, inputs) => {
            (FunctionInvocationLayout::Variadic, output, inputs)
        }
    };
    let inputs = inputs
        .into_iter()
        .map(canonical_factory_cell)
        .collect::<MResult<Vec<_>>>()?
        .into_boxed_slice();
    // Compatibility outputs retain their original typed or aggregate handle;
    // the cell binding itself provides canonical snapshot and replacement.
    let mut output = value_cell_from_legacy_function_value(output);
    if output.snapshot().is_err()
        && signature.output == FunctionValueRepresentation::Set
        && let Some(template) = inputs
            .iter()
            .find(|input| input.representation() == FunctionValueRepresentation::Set)
    {
        output = output.with_schema_from(template)?;
    }
    FunctionInvocation::from_cells(layout, output, inputs)
}

fn canonical_factory_cell(value: LegacyValue) -> MResult<ValueCell> {
    let representation = FunctionValueRepresentation::from_value(&value);
    if matches!(
        representation,
        FunctionValueRepresentation::MutableValueCell | FunctionValueRepresentation::AnyValue
    ) {
        return Ok(value_cell_from_legacy_function_value(value));
    }
    if matches!(
        representation,
        FunctionValueRepresentation::Atom
            | FunctionValueRepresentation::Enum
            | FunctionValueRepresentation::Map
            | FunctionValueRepresentation::Record
            | FunctionValueRepresentation::Set
            | FunctionValueRepresentation::Table
            | FunctionValueRepresentation::Tuple
    ) {
        return ValueCell::from_snapshot(value.to_canonical_value()?);
    }
    #[cfg(feature = "matrix")]
    if let FunctionValueRepresentation::Matrix {
        element: FunctionMatrixElement::Value,
        ..
    } = representation
    {
        return ValueCell::from_snapshot(value.to_canonical_value()?);
    }
    Ok(value_cell_from_legacy_function_value(value))
}

pub fn specialization_invocation_from_legacy(
    arguments: &[LegacyValue],
) -> MResult<SpecializationInvocation> {
    let inputs = arguments
        .iter()
        .cloned()
        .map(|value| {
            if let LegacyValue::MutableReference(reference) = value {
                let value = reference.try_borrow().map_err(|_| {
                    MechError::new(
                        crate::ValueCellBorrowConflict {
                            access: crate::CellAccess::Snapshot,
                        },
                        None,
                    )
                    .with_compiler_loc()
                })?;
                return Ok(SpecializationInput::Cell(
                    value_cell_from_legacy_function_value(value.clone()),
                ));
            }
            let representation = FunctionValueRepresentation::from_value(&value);
            if representation == FunctionValueRepresentation::Empty {
                return match value.to_canonical_value() {
                    Ok(value) => Ok(SpecializationInput::Cell(ValueCell::from_snapshot(value)?)),
                    Err(error)
                        if matches!(
                            error.kind_as::<super::LegacySnapshotError>(),
                            Some(super::LegacySnapshotError::LegacyEmptyNotSnapshot)
                        ) =>
                    {
                        Ok(SpecializationInput::Absent)
                    }
                    Err(error) => Err(error),
                };
            }
            if representation == FunctionValueRepresentation::Index
                && value.exact_ref_any().is_none()
            {
                return Err(MechError::new(
                    LegacySpecializationControlUnsupported {
                        control: "matrix-all-selection",
                    },
                    None,
                )
                .with_compiler_loc());
            }
            Ok(SpecializationInput::Cell(
                value_cell_from_legacy_function_value(value),
            ))
        })
        .collect::<MResult<Vec<_>>>()?;
    Ok(SpecializationInvocation::new(inputs.into_boxed_slice()))
}

pub fn specialize_legacy_function<S>(
    specializer: &S,
    invocation: &SpecializationInvocation,
    _context: &mut SpecializationContext<'_>,
) -> MResult<SpecializedFunction>
where
    S: FunctionSpecializer + ?Sized,
{
    let arguments = invocation
        .inputs()
        .iter()
        .enumerate()
        .map(|(index, input)| legacy_specialization_input(input, index))
        .collect::<MResult<Vec<_>>>()?;
    let implementation = specializer.specialize(&arguments)?;
    bind_legacy_specialized_function(implementation, invocation)
}

pub fn bind_legacy_specialized_function(
    implementation: Box<dyn MechFunction>,
    invocation: &SpecializationInvocation,
) -> MResult<SpecializedFunction> {
    let output =
        value_cell_from_legacy_function_value(legacy_function_output(implementation.as_ref())?);
    let inputs = invocation
        .inputs()
        .iter()
        .filter_map(|input| match input {
            SpecializationInput::Cell(cell) => Some(cell.clone()),
            SpecializationInput::Absent | SpecializationInput::MatrixAllSelection => None,
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let bound = FunctionInvocation::variadic(output, inputs);
    Ok(SpecializedFunction::new(FunctionInstance::new(
        implementation,
        bound,
    )))
}

pub fn legacy_function_value_from_cell(cell: &ValueCell) -> MResult<LegacyValue> {
    legacy_specialization_input(&SpecializationInput::Cell(cell.clone()), 0)
}

fn legacy_specialization_input(input: &SpecializationInput, _index: usize) -> MResult<LegacyValue> {
    let cell = match input {
        SpecializationInput::Cell(cell) => cell,
        SpecializationInput::Absent => return Ok(LegacyValue::legacy_absent_control()),
        SpecializationInput::MatrixAllSelection => {
            return Ok(LegacyValue::legacy_matrix_all_control());
        }
    };
    if let Some(reference) = cell.legacy_ref_compat() {
        return reference
            .try_borrow()
            .map(|value| value.clone())
            .map_err(|_| {
                MechError::new(
                    crate::ValueCellBorrowConflict {
                        access: crate::CellAccess::Snapshot,
                    },
                    None,
                )
                .with_compiler_loc()
            });
    }
    match cell.representation() {
        #[cfg(feature = "u8")]
        FunctionValueRepresentation::U8 => Ok(cell.try_ref::<u8>()?.to_value()),
        #[cfg(not(feature = "u8"))]
        FunctionValueRepresentation::U8 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "u16")]
        FunctionValueRepresentation::U16 => Ok(cell.try_ref::<u16>()?.to_value()),
        #[cfg(not(feature = "u16"))]
        FunctionValueRepresentation::U16 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "u32")]
        FunctionValueRepresentation::U32 => Ok(cell.try_ref::<u32>()?.to_value()),
        #[cfg(not(feature = "u32"))]
        FunctionValueRepresentation::U32 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "u64")]
        FunctionValueRepresentation::U64 => Ok(cell.try_ref::<u64>()?.to_value()),
        #[cfg(not(feature = "u64"))]
        FunctionValueRepresentation::U64 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "u128")]
        FunctionValueRepresentation::U128 => Ok(cell.try_ref::<u128>()?.to_value()),
        #[cfg(not(feature = "u128"))]
        FunctionValueRepresentation::U128 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "i8")]
        FunctionValueRepresentation::I8 => Ok(cell.try_ref::<i8>()?.to_value()),
        #[cfg(not(feature = "i8"))]
        FunctionValueRepresentation::I8 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "i16")]
        FunctionValueRepresentation::I16 => Ok(cell.try_ref::<i16>()?.to_value()),
        #[cfg(not(feature = "i16"))]
        FunctionValueRepresentation::I16 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "i32")]
        FunctionValueRepresentation::I32 => Ok(cell.try_ref::<i32>()?.to_value()),
        #[cfg(not(feature = "i32"))]
        FunctionValueRepresentation::I32 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "i64")]
        FunctionValueRepresentation::I64 => Ok(cell.try_ref::<i64>()?.to_value()),
        #[cfg(not(feature = "i64"))]
        FunctionValueRepresentation::I64 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "i128")]
        FunctionValueRepresentation::I128 => Ok(cell.try_ref::<i128>()?.to_value()),
        #[cfg(not(feature = "i128"))]
        FunctionValueRepresentation::I128 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "f32")]
        FunctionValueRepresentation::F32 => Ok(cell.try_ref::<f32>()?.to_value()),
        #[cfg(not(feature = "f32"))]
        FunctionValueRepresentation::F32 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "f64")]
        FunctionValueRepresentation::F64 => Ok(cell.try_ref::<f64>()?.to_value()),
        #[cfg(not(feature = "f64"))]
        FunctionValueRepresentation::F64 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "string")]
        FunctionValueRepresentation::String => Ok(cell.try_ref::<String>()?.to_value()),
        #[cfg(not(feature = "string"))]
        FunctionValueRepresentation::String => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "bool")]
        FunctionValueRepresentation::Bool => Ok(cell.try_ref::<bool>()?.to_value()),
        #[cfg(not(feature = "bool"))]
        FunctionValueRepresentation::Bool => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "complex")]
        FunctionValueRepresentation::C64 => Ok(cell.try_ref::<crate::C64>()?.to_value()),
        #[cfg(not(feature = "complex"))]
        FunctionValueRepresentation::C64 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "rational")]
        FunctionValueRepresentation::R64 => Ok(cell.try_ref::<crate::R64>()?.to_value()),
        #[cfg(not(feature = "rational"))]
        FunctionValueRepresentation::R64 => LegacyValue::from_canonical_value(&cell.snapshot()?),
        FunctionValueRepresentation::Index => Ok(cell.try_ref::<usize>()?.to_value()),
        FunctionValueRepresentation::Id => LegacyValue::from_canonical_value(&cell.snapshot()?),
        #[cfg(feature = "matrix")]
        FunctionValueRepresentation::Matrix { element, storage } => {
            legacy_matrix_specialization_input(cell, element, storage, _index)
        }
        #[cfg(not(feature = "matrix"))]
        FunctionValueRepresentation::Matrix { .. } => {
            LegacyValue::from_canonical_value(&cell.snapshot()?)
        }
        FunctionValueRepresentation::Empty
        | FunctionValueRepresentation::Atom
        | FunctionValueRepresentation::Enum
        | FunctionValueRepresentation::Record
        | FunctionValueRepresentation::Map
        | FunctionValueRepresentation::Set
        | FunctionValueRepresentation::Table
        | FunctionValueRepresentation::Tuple
        | FunctionValueRepresentation::Kind
        | FunctionValueRepresentation::MutableValueCell
        | FunctionValueRepresentation::AnyValue => {
            LegacyValue::from_canonical_value(&cell.snapshot()?)
        }
    }
}

#[cfg(feature = "matrix")]
fn legacy_matrix_specialization_input(
    cell: &ValueCell,
    element: FunctionMatrixElement,
    storage: FunctionMatrixStoragePattern,
    index: usize,
) -> MResult<LegacyValue> {
    macro_rules! project {
        ($type:ty, $variant:ident) => {
            crate::function::argument::matrix_from_cell::<$type>(
                cell,
                FunctionArgumentRole::Input(index),
            )
            .map(LegacyValue::$variant)
        };
    }
    let unsupported = |element| {
        Err(MechError::new(
            LegacySpecializationProjectionUnsupported {
                representation: FunctionValueRepresentation::Matrix { element, storage },
            },
            None,
        )
        .with_compiler_loc())
    };
    match element {
        FunctionMatrixElement::Index => project!(usize, MatrixIndex),
        #[cfg(feature = "bool")]
        FunctionMatrixElement::Bool => project!(bool, MatrixBool),
        #[cfg(not(feature = "bool"))]
        FunctionMatrixElement::Bool => unsupported(FunctionMatrixElement::Bool),
        #[cfg(feature = "string")]
        FunctionMatrixElement::String => project!(String, MatrixString),
        #[cfg(not(feature = "string"))]
        FunctionMatrixElement::String => unsupported(FunctionMatrixElement::String),
        #[cfg(feature = "u8")]
        FunctionMatrixElement::U8 => project!(u8, MatrixU8),
        #[cfg(not(feature = "u8"))]
        FunctionMatrixElement::U8 => unsupported(FunctionMatrixElement::U8),
        #[cfg(feature = "u16")]
        FunctionMatrixElement::U16 => project!(u16, MatrixU16),
        #[cfg(not(feature = "u16"))]
        FunctionMatrixElement::U16 => unsupported(FunctionMatrixElement::U16),
        #[cfg(feature = "u32")]
        FunctionMatrixElement::U32 => project!(u32, MatrixU32),
        #[cfg(not(feature = "u32"))]
        FunctionMatrixElement::U32 => unsupported(FunctionMatrixElement::U32),
        #[cfg(feature = "u64")]
        FunctionMatrixElement::U64 => project!(u64, MatrixU64),
        #[cfg(not(feature = "u64"))]
        FunctionMatrixElement::U64 => unsupported(FunctionMatrixElement::U64),
        #[cfg(feature = "u128")]
        FunctionMatrixElement::U128 => project!(u128, MatrixU128),
        #[cfg(not(feature = "u128"))]
        FunctionMatrixElement::U128 => unsupported(FunctionMatrixElement::U128),
        #[cfg(feature = "i8")]
        FunctionMatrixElement::I8 => project!(i8, MatrixI8),
        #[cfg(not(feature = "i8"))]
        FunctionMatrixElement::I8 => unsupported(FunctionMatrixElement::I8),
        #[cfg(feature = "i16")]
        FunctionMatrixElement::I16 => project!(i16, MatrixI16),
        #[cfg(not(feature = "i16"))]
        FunctionMatrixElement::I16 => unsupported(FunctionMatrixElement::I16),
        #[cfg(feature = "i32")]
        FunctionMatrixElement::I32 => project!(i32, MatrixI32),
        #[cfg(not(feature = "i32"))]
        FunctionMatrixElement::I32 => unsupported(FunctionMatrixElement::I32),
        #[cfg(feature = "i64")]
        FunctionMatrixElement::I64 => project!(i64, MatrixI64),
        #[cfg(not(feature = "i64"))]
        FunctionMatrixElement::I64 => unsupported(FunctionMatrixElement::I64),
        #[cfg(feature = "i128")]
        FunctionMatrixElement::I128 => project!(i128, MatrixI128),
        #[cfg(not(feature = "i128"))]
        FunctionMatrixElement::I128 => unsupported(FunctionMatrixElement::I128),
        #[cfg(feature = "f32")]
        FunctionMatrixElement::F32 => project!(f32, MatrixF32),
        #[cfg(not(feature = "f32"))]
        FunctionMatrixElement::F32 => unsupported(FunctionMatrixElement::F32),
        #[cfg(feature = "f64")]
        FunctionMatrixElement::F64 => project!(f64, MatrixF64),
        #[cfg(not(feature = "f64"))]
        FunctionMatrixElement::F64 => unsupported(FunctionMatrixElement::F64),
        #[cfg(feature = "complex")]
        FunctionMatrixElement::C64 => project!(crate::C64, MatrixC64),
        #[cfg(not(feature = "complex"))]
        FunctionMatrixElement::C64 => unsupported(FunctionMatrixElement::C64),
        #[cfg(feature = "rational")]
        FunctionMatrixElement::R64 => project!(crate::R64, MatrixR64),
        #[cfg(not(feature = "rational"))]
        FunctionMatrixElement::R64 => unsupported(FunctionMatrixElement::R64),
        FunctionMatrixElement::Value => unsupported(FunctionMatrixElement::Value),
    }
}

#[cfg(feature = "matrix")]
macro_rules! exact_matrix_cell {
    ($matrix:expr) => {{
        match $matrix {
            #[cfg(feature = "matrix1")]
            Matrix::Matrix1(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix2")]
            Matrix::Matrix2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix3")]
            Matrix::Matrix3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix4")]
            Matrix::Matrix4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix2x3")]
            Matrix::Matrix2x3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrix3x2")]
            Matrix::Matrix3x2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector2")]
            Matrix::RowVector2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector3")]
            Matrix::RowVector3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vector4")]
            Matrix::RowVector4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector2")]
            Matrix::Vector2(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector3")]
            Matrix::Vector3(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vector4")]
            Matrix::Vector4(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "row_vectord")]
            Matrix::RowDVector(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "vectord")]
            Matrix::DVector(reference) => inferred_matrix_reference_cell!(reference),
            #[cfg(feature = "matrixd")]
            Matrix::DMatrix(reference) => inferred_matrix_reference_cell!(reference),
        }
    }};
}

#[cfg(feature = "matrix")]
macro_rules! inferred_matrix_reference_cell {
    ($reference:expr) => {{
        let reference = $reference;
        let extents = {
            let matrix = reference.borrow();
            (matrix.nrows(), matrix.ncols())
        };
        ValueCell::from_inferred_ref(reference, Some(extents))
            .expect("exact legacy matrix has a canonical cell representation")
    }};
}

#[doc(hidden)]
pub fn value_cell_from_legacy_function_value(value: LegacyValue) -> ValueCell {
    if let Some(backing) = value.exact_ref_any() {
        macro_rules! exact_ref_cell {
            ($feature:literal, $type:ty) => {
                #[cfg(feature = $feature)]
                if let Some(reference) = backing.downcast_ref::<Ref<$type>>() {
                    return inferred_cell(reference.clone());
                }
            };
        }
        exact_ref_cell!("u8", u8);
        exact_ref_cell!("u16", u16);
        exact_ref_cell!("u32", u32);
        exact_ref_cell!("u64", u64);
        exact_ref_cell!("u128", u128);
        exact_ref_cell!("i8", i8);
        exact_ref_cell!("i16", i16);
        exact_ref_cell!("i32", i32);
        exact_ref_cell!("i64", i64);
        exact_ref_cell!("i128", i128);
        exact_ref_cell!("f32", f32);
        exact_ref_cell!("f64", f64);
        exact_ref_cell!("string", String);
        exact_ref_cell!("bool", bool);
        exact_ref_cell!("complex", crate::C64);
        exact_ref_cell!("rational", crate::R64);
        if let Some(reference) = backing.downcast_ref::<Ref<usize>>() {
            return inferred_cell(reference.clone());
        }
    }
    #[cfg(feature = "matrix")]
    if let Some(matrix) = value.exact_matrix_any() {
        macro_rules! exact_matrix_value_cell {
            ($feature:literal, $type:ty) => {
                #[cfg(feature = $feature)]
                if let Some(matrix) = matrix.downcast_ref::<Matrix<$type>>() {
                    return exact_matrix_cell!(matrix.clone());
                }
            };
        }
        exact_matrix_value_cell!("bool", bool);
        exact_matrix_value_cell!("u8", u8);
        exact_matrix_value_cell!("u16", u16);
        exact_matrix_value_cell!("u32", u32);
        exact_matrix_value_cell!("u64", u64);
        exact_matrix_value_cell!("u128", u128);
        exact_matrix_value_cell!("i8", i8);
        exact_matrix_value_cell!("i16", i16);
        exact_matrix_value_cell!("i32", i32);
        exact_matrix_value_cell!("i64", i64);
        exact_matrix_value_cell!("i128", i128);
        exact_matrix_value_cell!("f32", f32);
        exact_matrix_value_cell!("f64", f64);
        exact_matrix_value_cell!("string", String);
        exact_matrix_value_cell!("rational", crate::R64);
        exact_matrix_value_cell!("complex", crate::C64);
        if let Some(matrix) = matrix.downcast_ref::<Matrix<usize>>() {
            return exact_matrix_cell!(matrix.clone());
        }
    }
    let representation = FunctionValueRepresentation::from_value(&value);
    if matches!(
        representation,
        FunctionValueRepresentation::Atom
            | FunctionValueRepresentation::Enum
            | FunctionValueRepresentation::Map
            | FunctionValueRepresentation::Record
            | FunctionValueRepresentation::Set
            | FunctionValueRepresentation::Table
            | FunctionValueRepresentation::Tuple
    ) && let Some(identity) = value.reactive_root_cell_ids().into_iter().next()
    {
        return ValueCell::from_legacy_ref_with_identity(Ref::new(value), identity);
    }
    ValueCell::new(value)
}

/// Builds a legacy compatibility view over the exact backing retained by a
/// canonical value cell.
pub fn legacy_value_from_cell_compat(cell: &ValueCell) -> MResult<LegacyValue> {
    if let Some(reference) = cell.legacy_ref_compat() {
        return reference
            .try_borrow()
            .map(|value| value.clone())
            .map_err(|_| {
                MechError::new(
                    crate::ValueCellBorrowConflict {
                        access: crate::CellAccess::Snapshot,
                    },
                    None,
                )
                .with_compiler_loc()
            });
    }

    macro_rules! scalar {
        ($feature:literal, $type:ty, $variant:ident) => {
            #[cfg(feature = $feature)]
            if let Ok(reference) = cell.try_ref::<$type>() {
                return Ok(LegacyValue::$variant(reference));
            }
        };
    }
    scalar!("u8", u8, U8);
    scalar!("u16", u16, U16);
    scalar!("u32", u32, U32);
    scalar!("u64", u64, U64);
    scalar!("u128", u128, U128);
    scalar!("i8", i8, I8);
    scalar!("i16", i16, I16);
    scalar!("i32", i32, I32);
    scalar!("i64", i64, I64);
    scalar!("i128", i128, I128);
    scalar!("f32", f32, F32);
    scalar!("f64", f64, F64);
    scalar!("bool", bool, Bool);
    scalar!("string", String, String);
    scalar!("rational", crate::R64, R64);
    scalar!("complex", crate::C64, C64);
    if let Ok(reference) = cell.try_ref::<usize>() {
        return Ok(reference.to_value());
    }

    macro_rules! matrix {
        ($feature:literal, $type:ty, $variant:ident) => {
            #[cfg(all(feature = "matrix", feature = $feature))]
            if let Some(matrix) = legacy_matrix_from_cell::<$type>(cell) {
                return Ok(LegacyValue::$variant(matrix));
            }
        };
    }
    matrix!("u8", u8, MatrixU8);
    matrix!("u16", u16, MatrixU16);
    matrix!("u32", u32, MatrixU32);
    matrix!("u64", u64, MatrixU64);
    matrix!("u128", u128, MatrixU128);
    matrix!("i8", i8, MatrixI8);
    matrix!("i16", i16, MatrixI16);
    matrix!("i32", i32, MatrixI32);
    matrix!("i64", i64, MatrixI64);
    matrix!("i128", i128, MatrixI128);
    matrix!("f32", f32, MatrixF32);
    matrix!("f64", f64, MatrixF64);
    matrix!("bool", bool, MatrixBool);
    matrix!("string", String, MatrixString);
    matrix!("rational", crate::R64, MatrixR64);
    matrix!("complex", crate::C64, MatrixC64);
    #[cfg(feature = "matrix")]
    if let Some(matrix) = legacy_matrix_from_cell::<usize>(cell) {
        return Ok(matrix.to_value());
    }

    LegacyValue::from_canonical_value(&cell.snapshot()?)
}

#[cfg(feature = "matrix")]
fn legacy_matrix_from_cell<T: 'static>(cell: &ValueCell) -> Option<crate::matrix::Matrix<T>> {
    macro_rules! backing {
        ($feature:literal, $variant:ident, $type:ident) => {
            #[cfg(feature = $feature)]
            if let Ok(reference) = cell.try_ref::<crate::$type<T>>() {
                return Some(crate::matrix::Matrix::$variant(reference));
            }
        };
    }
    backing!("matrix1", Matrix1, Matrix1);
    backing!("matrix2", Matrix2, Matrix2);
    backing!("matrix3", Matrix3, Matrix3);
    backing!("matrix4", Matrix4, Matrix4);
    backing!("matrix2x3", Matrix2x3, Matrix2x3);
    backing!("matrix3x2", Matrix3x2, Matrix3x2);
    backing!("row_vector2", RowVector2, RowVector2);
    backing!("row_vector3", RowVector3, RowVector3);
    backing!("row_vector4", RowVector4, RowVector4);
    backing!("row_vectord", RowDVector, RowDVector);
    backing!("vector2", Vector2, Vector2);
    backing!("vector3", Vector3, Vector3);
    backing!("vector4", Vector4, Vector4);
    backing!("vectord", DVector, DVector);
    backing!("matrixd", DMatrix, DMatrix);
    None
}

fn inferred_cell<T>(reference: crate::Ref<T>) -> ValueCell
where
    T: crate::CanonicalCellBacking,
{
    ValueCell::from_inferred_ref(reference, None)
        .expect("exact legacy function value has a canonical cell representation")
}

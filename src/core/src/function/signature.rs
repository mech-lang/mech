#[cfg(feature = "no_std")]
use alloc::{collections::BTreeSet, string::String};
#[cfg(not(feature = "no_std"))]
use std::{collections::BTreeSet, string::String};

#[cfg(feature = "program")]
use crate::RuntimeType;
#[cfg(feature = "matrix")]
use crate::structures::Matrix as MechMatrix;
use crate::{
    FunctionArgumentRole, FunctionMatrixRepresentation, LegacyValue, MechError, MechErrorKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionMatrixElement {
    Index,
    Bool,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    C64,
    R64,
    Value,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionMatrixStoragePattern {
    Exact(FunctionMatrixRepresentation),
    AnyStorage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FunctionValueRepresentation {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    C64,
    R64,
    String,
    Bool,
    Id,
    Index,
    Empty,
    Matrix {
        element: FunctionMatrixElement,
        storage: FunctionMatrixStoragePattern,
    },
    Atom,
    Enum,
    Record,
    Map,
    Set,
    Table,
    Tuple,
    Kind,
    MutableValueCell,
    AnyValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RuntimeFunctionInputs {
    Nullary,
    Unary(FunctionValueRepresentation),
    Binary(FunctionValueRepresentation, FunctionValueRepresentation),
    Ternary(
        FunctionValueRepresentation,
        FunctionValueRepresentation,
        FunctionValueRepresentation,
    ),
    Quaternary(
        FunctionValueRepresentation,
        FunctionValueRepresentation,
        FunctionValueRepresentation,
        FunctionValueRepresentation,
    ),
    Variadic {
        element: FunctionValueRepresentation,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeFunctionSignature {
    pub output: FunctionValueRepresentation,
    pub inputs: RuntimeFunctionInputs,
}

impl RuntimeFunctionSignature {
    pub const fn nullary(output: FunctionValueRepresentation) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Nullary,
        }
    }

    pub const fn unary(
        output: FunctionValueRepresentation,
        argument: FunctionValueRepresentation,
    ) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Unary(argument),
        }
    }

    pub const fn binary(
        output: FunctionValueRepresentation,
        lhs: FunctionValueRepresentation,
        rhs: FunctionValueRepresentation,
    ) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Binary(lhs, rhs),
        }
    }

    pub const fn ternary(
        output: FunctionValueRepresentation,
        first: FunctionValueRepresentation,
        second: FunctionValueRepresentation,
        third: FunctionValueRepresentation,
    ) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Ternary(first, second, third),
        }
    }

    pub const fn quaternary(
        output: FunctionValueRepresentation,
        first: FunctionValueRepresentation,
        second: FunctionValueRepresentation,
        third: FunctionValueRepresentation,
        fourth: FunctionValueRepresentation,
    ) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Quaternary(first, second, third, fourth),
        }
    }

    pub const fn variadic(
        output: FunctionValueRepresentation,
        element: FunctionValueRepresentation,
    ) -> Self {
        Self {
            output,
            inputs: RuntimeFunctionInputs::Variadic { element },
        }
    }

    pub fn required_native_features(self) -> BTreeSet<NativeValueFeature> {
        let mut features = BTreeSet::new();
        collect_representation_features(self.output, &mut features);
        match self.inputs {
            RuntimeFunctionInputs::Nullary => {}
            RuntimeFunctionInputs::Unary(argument) => {
                collect_representation_features(argument, &mut features);
            }
            RuntimeFunctionInputs::Binary(lhs, rhs) => {
                collect_representation_features(lhs, &mut features);
                collect_representation_features(rhs, &mut features);
            }
            RuntimeFunctionInputs::Ternary(first, second, third) => {
                collect_representation_features(first, &mut features);
                collect_representation_features(second, &mut features);
                collect_representation_features(third, &mut features);
            }
            RuntimeFunctionInputs::Quaternary(first, second, third, fourth) => {
                collect_representation_features(first, &mut features);
                collect_representation_features(second, &mut features);
                collect_representation_features(third, &mut features);
                collect_representation_features(fourth, &mut features);
            }
            RuntimeFunctionInputs::Variadic { element } => {
                collect_representation_features(element, &mut features);
            }
        }
        features
    }
}

pub trait FunctionRuntimeType {
    const REPRESENTATION: FunctionValueRepresentation;
}

macro_rules! scalar_runtime_type {
    ($type:ty, $representation:ident, $feature:literal) => {
        #[cfg(feature = $feature)]
        impl FunctionRuntimeType for $type {
            const REPRESENTATION: FunctionValueRepresentation =
                FunctionValueRepresentation::$representation;
        }
    };
}

scalar_runtime_type!(u8, U8, "u8");
scalar_runtime_type!(u16, U16, "u16");
scalar_runtime_type!(u32, U32, "u32");
scalar_runtime_type!(u64, U64, "u64");
scalar_runtime_type!(u128, U128, "u128");
scalar_runtime_type!(i8, I8, "i8");
scalar_runtime_type!(i16, I16, "i16");
scalar_runtime_type!(i32, I32, "i32");
scalar_runtime_type!(i64, I64, "i64");
scalar_runtime_type!(i128, I128, "i128");
scalar_runtime_type!(f32, F32, "f32");
scalar_runtime_type!(f64, F64, "f64");
scalar_runtime_type!(bool, Bool, "bool");
scalar_runtime_type!(String, String, "string");

impl FunctionRuntimeType for usize {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Index;
}

#[cfg(feature = "complex")]
impl FunctionRuntimeType for crate::C64 {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::C64;
}

#[cfg(feature = "rational")]
impl FunctionRuntimeType for crate::R64 {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::R64;
}

impl FunctionRuntimeType for LegacyValue {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::AnyValue;
}

#[cfg(feature = "atom")]
impl FunctionRuntimeType for crate::MechAtom {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Atom;
}

#[cfg(feature = "enum")]
impl FunctionRuntimeType for crate::MechEnum {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Enum;
}

#[cfg(feature = "record")]
impl FunctionRuntimeType for crate::MechRecord {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Record;
}

#[cfg(feature = "map")]
impl FunctionRuntimeType for crate::MechMap {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Map;
}

#[cfg(feature = "set")]
impl FunctionRuntimeType for crate::MechSet {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Set;
}

#[cfg(feature = "table")]
impl FunctionRuntimeType for crate::MechTable {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Table;
}

#[cfg(feature = "tuple")]
impl FunctionRuntimeType for crate::MechTuple {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Tuple;
}

#[cfg(feature = "matrix")]
impl<T: FunctionRuntimeType> FunctionRuntimeType for MechMatrix<T> {
    const REPRESENTATION: FunctionValueRepresentation = FunctionValueRepresentation::Matrix {
        element: matrix_element_for_representation(T::REPRESENTATION),
        storage: FunctionMatrixStoragePattern::AnyStorage,
    };
}

macro_rules! exact_matrix_runtime_type {
    ($type:ident, $feature:literal, $representation:ident) => {
        #[cfg(feature = $feature)]
        impl<T: FunctionRuntimeType> FunctionRuntimeType for crate::$type<T> {
            const REPRESENTATION: FunctionValueRepresentation =
                FunctionValueRepresentation::Matrix {
                    element: matrix_element_for_representation(T::REPRESENTATION),
                    storage: FunctionMatrixStoragePattern::Exact(
                        FunctionMatrixRepresentation::$representation,
                    ),
                };
        }
    };
}

exact_matrix_runtime_type!(Matrix1, "matrix1", Matrix1);
exact_matrix_runtime_type!(Matrix2, "matrix2", Matrix2);
exact_matrix_runtime_type!(Matrix3, "matrix3", Matrix3);
exact_matrix_runtime_type!(Matrix4, "matrix4", Matrix4);
exact_matrix_runtime_type!(Matrix2x3, "matrix2x3", Matrix2x3);
exact_matrix_runtime_type!(Matrix3x2, "matrix3x2", Matrix3x2);
exact_matrix_runtime_type!(RowVector2, "row_vector2", RowVector2);
exact_matrix_runtime_type!(RowVector3, "row_vector3", RowVector3);
exact_matrix_runtime_type!(RowVector4, "row_vector4", RowVector4);
exact_matrix_runtime_type!(RowDVector, "row_vectord", RowVectorD);
exact_matrix_runtime_type!(Vector2, "vector2", Vector2);
exact_matrix_runtime_type!(Vector3, "vector3", Vector3);
exact_matrix_runtime_type!(Vector4, "vector4", Vector4);
exact_matrix_runtime_type!(DVector, "vectord", VectorD);
exact_matrix_runtime_type!(DMatrix, "matrixd", MatrixD);

#[cfg(feature = "matrix")]
const fn matrix_element_for_representation(
    representation: FunctionValueRepresentation,
) -> FunctionMatrixElement {
    match representation {
        FunctionValueRepresentation::Index => FunctionMatrixElement::Index,
        FunctionValueRepresentation::Bool => FunctionMatrixElement::Bool,
        FunctionValueRepresentation::String => FunctionMatrixElement::String,
        FunctionValueRepresentation::U8 => FunctionMatrixElement::U8,
        FunctionValueRepresentation::U16 => FunctionMatrixElement::U16,
        FunctionValueRepresentation::U32 => FunctionMatrixElement::U32,
        FunctionValueRepresentation::U64 => FunctionMatrixElement::U64,
        FunctionValueRepresentation::U128 => FunctionMatrixElement::U128,
        FunctionValueRepresentation::I8 => FunctionMatrixElement::I8,
        FunctionValueRepresentation::I16 => FunctionMatrixElement::I16,
        FunctionValueRepresentation::I32 => FunctionMatrixElement::I32,
        FunctionValueRepresentation::I64 => FunctionMatrixElement::I64,
        FunctionValueRepresentation::I128 => FunctionMatrixElement::I128,
        FunctionValueRepresentation::F32 => FunctionMatrixElement::F32,
        FunctionValueRepresentation::F64 => FunctionMatrixElement::F64,
        FunctionValueRepresentation::C64 => FunctionMatrixElement::C64,
        FunctionValueRepresentation::R64 => FunctionMatrixElement::R64,
        FunctionValueRepresentation::AnyValue => FunctionMatrixElement::Value,
        _ => panic!("unsupported runtime matrix element representation"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NativeValueFeature {
    Bool,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    C64,
    R64,
    Matrix,
    Matrix1,
    Matrix2,
    Matrix3,
    Matrix4,
    Matrix2x3,
    Matrix3x2,
    RowVector2,
    RowVector3,
    RowVector4,
    RowVectorD,
    Vector2,
    Vector3,
    Vector4,
    VectorD,
    MatrixD,
    Atom,
    Enum,
    Record,
    Map,
    Set,
    Table,
    Tuple,
    KindAnnotation,
}

impl NativeValueFeature {
    pub const fn cargo_feature(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::String => "string",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::I128 => "i128",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::C64 => "c64",
            Self::R64 => "r64",
            Self::Matrix => "matrix",
            Self::Matrix1 => "matrix1",
            Self::Matrix2 => "matrix2",
            Self::Matrix3 => "matrix3",
            Self::Matrix4 => "matrix4",
            Self::Matrix2x3 => "matrix2x3",
            Self::Matrix3x2 => "matrix3x2",
            Self::RowVector2 => "row_vector2",
            Self::RowVector3 => "row_vector3",
            Self::RowVector4 => "row_vector4",
            Self::RowVectorD => "row_vectord",
            Self::Vector2 => "vector2",
            Self::Vector3 => "vector3",
            Self::Vector4 => "vector4",
            Self::VectorD => "vectord",
            Self::MatrixD => "matrixd",
            Self::Atom => "atom",
            Self::Enum => "enum",
            Self::Record => "record",
            Self::Map => "map",
            Self::Set => "set",
            Self::Table => "table",
            Self::Tuple => "tuple",
            Self::KindAnnotation => "kind_annotation",
        }
    }

    pub fn from_cargo_feature(feature: &str) -> Option<Self> {
        ALL_NATIVE_VALUE_FEATURES
            .iter()
            .copied()
            .find(|candidate| candidate.cargo_feature() == feature)
    }
}

pub const ALL_NATIVE_VALUE_FEATURES: &[NativeValueFeature] = &[
    NativeValueFeature::Bool,
    NativeValueFeature::String,
    NativeValueFeature::U8,
    NativeValueFeature::U16,
    NativeValueFeature::U32,
    NativeValueFeature::U64,
    NativeValueFeature::U128,
    NativeValueFeature::I8,
    NativeValueFeature::I16,
    NativeValueFeature::I32,
    NativeValueFeature::I64,
    NativeValueFeature::I128,
    NativeValueFeature::F32,
    NativeValueFeature::F64,
    NativeValueFeature::C64,
    NativeValueFeature::R64,
    NativeValueFeature::Matrix,
    NativeValueFeature::Matrix1,
    NativeValueFeature::Matrix2,
    NativeValueFeature::Matrix3,
    NativeValueFeature::Matrix4,
    NativeValueFeature::Matrix2x3,
    NativeValueFeature::Matrix3x2,
    NativeValueFeature::RowVector2,
    NativeValueFeature::RowVector3,
    NativeValueFeature::RowVector4,
    NativeValueFeature::RowVectorD,
    NativeValueFeature::Vector2,
    NativeValueFeature::Vector3,
    NativeValueFeature::Vector4,
    NativeValueFeature::VectorD,
    NativeValueFeature::MatrixD,
    NativeValueFeature::Atom,
    NativeValueFeature::Enum,
    NativeValueFeature::Record,
    NativeValueFeature::Map,
    NativeValueFeature::Set,
    NativeValueFeature::Table,
    NativeValueFeature::Tuple,
    NativeValueFeature::KindAnnotation,
];

fn collect_representation_features(
    representation: FunctionValueRepresentation,
    features: &mut BTreeSet<NativeValueFeature>,
) {
    use FunctionMatrixRepresentation as Storage;
    use FunctionMatrixStoragePattern as Pattern;
    use FunctionValueRepresentation as Representation;
    use NativeValueFeature as Feature;

    match representation {
        Representation::Bool => {
            features.insert(Feature::Bool);
        }
        Representation::String => {
            features.insert(Feature::String);
        }
        Representation::U8 => {
            features.insert(Feature::U8);
        }
        Representation::U16 => {
            features.insert(Feature::U16);
        }
        Representation::U32 => {
            features.insert(Feature::U32);
        }
        Representation::U64 => {
            features.insert(Feature::U64);
        }
        Representation::U128 => {
            features.insert(Feature::U128);
        }
        Representation::I8 => {
            features.insert(Feature::I8);
        }
        Representation::I16 => {
            features.insert(Feature::I16);
        }
        Representation::I32 => {
            features.insert(Feature::I32);
        }
        Representation::I64 => {
            features.insert(Feature::I64);
        }
        Representation::I128 => {
            features.insert(Feature::I128);
        }
        Representation::F32 => {
            features.insert(Feature::F32);
        }
        Representation::F64 => {
            features.insert(Feature::F64);
        }
        Representation::C64 => {
            features.insert(Feature::C64);
        }
        Representation::R64 => {
            features.insert(Feature::R64);
        }
        Representation::Matrix { element, storage } => {
            collect_matrix_element_feature(element, features);
            features.insert(match storage {
                Pattern::AnyStorage => Feature::Matrix,
                Pattern::Exact(Storage::Matrix1) => Feature::Matrix1,
                Pattern::Exact(Storage::Matrix2) => Feature::Matrix2,
                Pattern::Exact(Storage::Matrix3) => Feature::Matrix3,
                Pattern::Exact(Storage::Matrix4) => Feature::Matrix4,
                Pattern::Exact(Storage::Matrix2x3) => Feature::Matrix2x3,
                Pattern::Exact(Storage::Matrix3x2) => Feature::Matrix3x2,
                Pattern::Exact(Storage::RowVector2) => Feature::RowVector2,
                Pattern::Exact(Storage::RowVector3) => Feature::RowVector3,
                Pattern::Exact(Storage::RowVector4) => Feature::RowVector4,
                Pattern::Exact(Storage::Vector2) => Feature::Vector2,
                Pattern::Exact(Storage::Vector3) => Feature::Vector3,
                Pattern::Exact(Storage::Vector4) => Feature::Vector4,
                Pattern::Exact(Storage::RowVectorD) => Feature::RowVectorD,
                Pattern::Exact(Storage::VectorD) => Feature::VectorD,
                Pattern::Exact(Storage::MatrixD) => Feature::MatrixD,
            });
        }
        Representation::Atom => {
            features.insert(Feature::Atom);
        }
        Representation::Enum => {
            features.insert(Feature::Enum);
        }
        Representation::Record => {
            features.insert(Feature::Record);
        }
        Representation::Map => {
            features.insert(Feature::Map);
        }
        Representation::Set => {
            features.insert(Feature::Set);
        }
        Representation::Table => {
            features.insert(Feature::Table);
        }
        Representation::Tuple => {
            features.insert(Feature::Tuple);
        }
        Representation::Kind => {
            features.insert(Feature::KindAnnotation);
        }
        Representation::Id
        | Representation::Index
        | Representation::Empty
        | Representation::MutableValueCell
        | Representation::AnyValue => {}
    }
}

fn collect_matrix_element_feature(
    element: FunctionMatrixElement,
    features: &mut BTreeSet<NativeValueFeature>,
) {
    use FunctionMatrixElement as Element;
    use NativeValueFeature as Feature;
    let feature = match element {
        Element::Bool => Some(Feature::Bool),
        Element::String => Some(Feature::String),
        Element::U8 => Some(Feature::U8),
        Element::U16 => Some(Feature::U16),
        Element::U32 => Some(Feature::U32),
        Element::U64 => Some(Feature::U64),
        Element::U128 => Some(Feature::U128),
        Element::I8 => Some(Feature::I8),
        Element::I16 => Some(Feature::I16),
        Element::I32 => Some(Feature::I32),
        Element::I64 => Some(Feature::I64),
        Element::I128 => Some(Feature::I128),
        Element::F32 => Some(Feature::F32),
        Element::F64 => Some(Feature::F64),
        Element::C64 => Some(Feature::C64),
        Element::R64 => Some(Feature::R64),
        Element::Index | Element::Value => None,
    };
    if let Some(feature) = feature {
        features.insert(feature);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionSignatureViolation {
    pub role: FunctionArgumentRole,
    pub expected: FunctionValueRepresentation,
    pub found: FunctionValueRepresentation,
}

impl MechErrorKind for FunctionSignatureViolation {
    fn name(&self) -> &str {
        "FunctionSignatureViolation"
    }

    fn message(&self) -> String {
        format!(
            "function argument {:?} requires {:?}, found {:?}",
            self.role, self.expected, self.found,
        )
    }
}

impl FunctionValueRepresentation {
    pub fn from_value(value: &LegacyValue) -> Self {
        #[cfg(feature = "matrix")]
        let matrix = |element, representation| Self::Matrix {
            element,
            storage: FunctionMatrixStoragePattern::Exact(representation),
        };
        match value {
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => Self::U8,
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => Self::U16,
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => Self::U32,
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => Self::U64,
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => Self::U128,
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => Self::I8,
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => Self::I16,
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => Self::I32,
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => Self::I64,
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => Self::I128,
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => Self::F32,
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => Self::F64,
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(_) => Self::String,
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => Self::Bool,
            #[cfg(feature = "complex")]
            LegacyValue::C64(_) => Self::C64,
            #[cfg(feature = "rational")]
            LegacyValue::R64(_) => Self::R64,
            #[cfg(feature = "atom")]
            LegacyValue::Atom(_) => Self::Atom,
            #[cfg(feature = "enum")]
            LegacyValue::Enum(_) => Self::Enum,
            #[cfg(feature = "record")]
            LegacyValue::Record(_) => Self::Record,
            #[cfg(feature = "map")]
            LegacyValue::Map(_) => Self::Map,
            #[cfg(feature = "set")]
            LegacyValue::Set(_) => Self::Set,
            #[cfg(feature = "table")]
            LegacyValue::Table(_) => Self::Table,
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(_) => Self::Tuple,
            LegacyValue::Id(_) => Self::Id,
            LegacyValue::Index(_) | LegacyValue::IndexAll => Self::Index,
            LegacyValue::MutableReference(_) => Self::MutableValueCell,
            LegacyValue::Kind(_) => Self::Kind,
            LegacyValue::Empty | LegacyValue::EmptyKind(_) => Self::Empty,
            LegacyValue::Typed(_, _) => Self::AnyValue,
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(value) => matrix(
                FunctionMatrixElement::Index,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(value) => matrix(
                FunctionMatrixElement::Bool,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(value) => matrix(
                FunctionMatrixElement::U8,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(value) => matrix(
                FunctionMatrixElement::U16,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(value) => matrix(
                FunctionMatrixElement::U32,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(value) => matrix(
                FunctionMatrixElement::U64,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(value) => matrix(
                FunctionMatrixElement::U128,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(value) => matrix(
                FunctionMatrixElement::I8,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(value) => matrix(
                FunctionMatrixElement::I16,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(value) => matrix(
                FunctionMatrixElement::I32,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(value) => matrix(
                FunctionMatrixElement::I64,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(value) => matrix(
                FunctionMatrixElement::I128,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(value) => matrix(
                FunctionMatrixElement::F32,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(value) => matrix(
                FunctionMatrixElement::F64,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(value) => matrix(
                FunctionMatrixElement::String,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(value) => matrix(
                FunctionMatrixElement::R64,
                matrix_storage_representation(value),
            ),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(value) => matrix(
                FunctionMatrixElement::C64,
                matrix_storage_representation(value),
            ),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(value) => matrix(
                FunctionMatrixElement::Value,
                matrix_storage_representation(value),
            ),
        }
    }

    pub fn matches(self, found: Self) -> bool {
        match (self, found) {
            (Self::AnyValue, _) => true,
            (
                Self::Matrix {
                    element: expected_element,
                    storage: FunctionMatrixStoragePattern::AnyStorage,
                },
                Self::Matrix {
                    element: found_element,
                    ..
                },
            ) => expected_element == found_element,
            _ => self == found,
        }
    }
}

#[cfg(feature = "matrix")]
fn matrix_storage_representation<T>(matrix: &MechMatrix<T>) -> FunctionMatrixRepresentation {
    match matrix {
        #[cfg(feature = "matrix1")]
        MechMatrix::Matrix1(_) => FunctionMatrixRepresentation::Matrix1,
        #[cfg(feature = "matrix2")]
        MechMatrix::Matrix2(_) => FunctionMatrixRepresentation::Matrix2,
        #[cfg(feature = "matrix3")]
        MechMatrix::Matrix3(_) => FunctionMatrixRepresentation::Matrix3,
        #[cfg(feature = "matrix4")]
        MechMatrix::Matrix4(_) => FunctionMatrixRepresentation::Matrix4,
        #[cfg(feature = "matrix2x3")]
        MechMatrix::Matrix2x3(_) => FunctionMatrixRepresentation::Matrix2x3,
        #[cfg(feature = "matrix3x2")]
        MechMatrix::Matrix3x2(_) => FunctionMatrixRepresentation::Matrix3x2,
        #[cfg(feature = "row_vector2")]
        MechMatrix::RowVector2(_) => FunctionMatrixRepresentation::RowVector2,
        #[cfg(feature = "row_vector3")]
        MechMatrix::RowVector3(_) => FunctionMatrixRepresentation::RowVector3,
        #[cfg(feature = "row_vector4")]
        MechMatrix::RowVector4(_) => FunctionMatrixRepresentation::RowVector4,
        #[cfg(feature = "vector2")]
        MechMatrix::Vector2(_) => FunctionMatrixRepresentation::Vector2,
        #[cfg(feature = "vector3")]
        MechMatrix::Vector3(_) => FunctionMatrixRepresentation::Vector3,
        #[cfg(feature = "vector4")]
        MechMatrix::Vector4(_) => FunctionMatrixRepresentation::Vector4,
        #[cfg(feature = "row_vectord")]
        MechMatrix::RowDVector(_) => FunctionMatrixRepresentation::RowVectorD,
        #[cfg(feature = "vectord")]
        MechMatrix::DVector(_) => FunctionMatrixRepresentation::VectorD,
        #[cfg(feature = "matrixd")]
        MechMatrix::DMatrix(_) => FunctionMatrixRepresentation::MatrixD,
    }
}

#[cfg(feature = "program")]
pub fn native_features_for_runtime_type(
    runtime_type: &RuntimeType,
    features: &mut BTreeSet<NativeValueFeature>,
) {
    use crate::MatrixStorage;
    match runtime_type {
        RuntimeType::Empty
        | RuntimeType::Any
        | RuntimeType::None
        | RuntimeType::Id
        | RuntimeType::Index => {}
        RuntimeType::Bool => {
            features.insert(NativeValueFeature::Bool);
        }
        RuntimeType::String => {
            features.insert(NativeValueFeature::String);
        }
        RuntimeType::U8 => {
            features.insert(NativeValueFeature::U8);
        }
        RuntimeType::U16 => {
            features.insert(NativeValueFeature::U16);
        }
        RuntimeType::U32 => {
            features.insert(NativeValueFeature::U32);
        }
        RuntimeType::U64 => {
            features.insert(NativeValueFeature::U64);
        }
        RuntimeType::U128 => {
            features.insert(NativeValueFeature::U128);
        }
        RuntimeType::I8 => {
            features.insert(NativeValueFeature::I8);
        }
        RuntimeType::I16 => {
            features.insert(NativeValueFeature::I16);
        }
        RuntimeType::I32 => {
            features.insert(NativeValueFeature::I32);
        }
        RuntimeType::I64 => {
            features.insert(NativeValueFeature::I64);
        }
        RuntimeType::I128 => {
            features.insert(NativeValueFeature::I128);
        }
        RuntimeType::F32 => {
            features.insert(NativeValueFeature::F32);
        }
        RuntimeType::F64 => {
            features.insert(NativeValueFeature::F64);
        }
        RuntimeType::C64 => {
            features.insert(NativeValueFeature::C64);
        }
        RuntimeType::R64 => {
            features.insert(NativeValueFeature::R64);
        }
        RuntimeType::Matrix {
            element, storage, ..
        } => {
            features.insert(match storage {
                MatrixStorage::Matrix1 => NativeValueFeature::Matrix1,
                MatrixStorage::Matrix2 => NativeValueFeature::Matrix2,
                MatrixStorage::Matrix3 => NativeValueFeature::Matrix3,
                MatrixStorage::Matrix4 => NativeValueFeature::Matrix4,
                MatrixStorage::Matrix2x3 => NativeValueFeature::Matrix2x3,
                MatrixStorage::Matrix3x2 => NativeValueFeature::Matrix3x2,
                MatrixStorage::RowVector2 => NativeValueFeature::RowVector2,
                MatrixStorage::RowVector3 => NativeValueFeature::RowVector3,
                MatrixStorage::RowVector4 => NativeValueFeature::RowVector4,
                MatrixStorage::Vector2 => NativeValueFeature::Vector2,
                MatrixStorage::Vector3 => NativeValueFeature::Vector3,
                MatrixStorage::Vector4 => NativeValueFeature::Vector4,
                MatrixStorage::RowVectorD => NativeValueFeature::RowVectorD,
                MatrixStorage::VectorD => NativeValueFeature::VectorD,
                MatrixStorage::MatrixD => NativeValueFeature::MatrixD,
            });
            native_features_for_runtime_type(element, features);
        }
        RuntimeType::Record(fields) => {
            features.insert(NativeValueFeature::Record);
            for (_, child) in fields {
                native_features_for_runtime_type(child, features);
            }
        }
        RuntimeType::Map { key, value } => {
            features.insert(NativeValueFeature::Map);
            native_features_for_runtime_type(key, features);
            native_features_for_runtime_type(value, features);
        }
        RuntimeType::Set { element, .. } => {
            features.insert(NativeValueFeature::Set);
            native_features_for_runtime_type(element, features);
        }
        RuntimeType::Table { columns, .. } => {
            features.insert(NativeValueFeature::Table);
            for (_, child) in columns {
                native_features_for_runtime_type(child, features);
            }
        }
        RuntimeType::Tuple(children) => {
            features.insert(NativeValueFeature::Tuple);
            for child in children {
                native_features_for_runtime_type(child, features);
            }
        }
        RuntimeType::Atom { .. } => {
            features.insert(NativeValueFeature::Atom);
        }
        RuntimeType::Enum { .. } => {
            features.insert(NativeValueFeature::Enum);
        }
        RuntimeType::Reference(child) | RuntimeType::Option(child) => {
            native_features_for_runtime_type(child, features);
        }
        RuntimeType::Kind(_) => {
            features.insert(NativeValueFeature::KindAnnotation);
        }
    }
}

pub(crate) fn signature_violation(
    role: FunctionArgumentRole,
    expected: FunctionValueRepresentation,
    value: &LegacyValue,
) -> MechError {
    MechError::new(
        FunctionSignatureViolation {
            role,
            expected,
            found: FunctionValueRepresentation::from_value(value),
        },
        None,
    )
    .with_compiler_loc()
}

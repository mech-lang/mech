#[cfg(feature = "matrix")]
use crate::matrix::Matrix;
#[cfg(feature = "matrix")]
#[cfg(feature = "complex")]
use crate::types::complex_numbers::C64;
use crate::*;
#[cfg(any(feature = "functions", feature = "matrix"))]
use core::any::Any;
#[cfg(feature = "no_std")]
use core::hash::BuildHasherDefault;
#[cfg(feature = "no_std")]
use core::mem;
#[cfg(feature = "no_std")]
use fxhash::FxHasher;
#[cfg(feature = "no_std")]
use hashbrown::HashSet as HashBrownSet;
#[cfg(not(feature = "no_std"))]
use std::collections::HashSet;
#[cfg(not(feature = "no_std"))]
use std::mem;
#[cfg(feature = "no_std")]
type HashSet<T> = HashBrownSet<T, BuildHasherDefault<FxHasher>>;

#[cfg(feature = "vectord")]
use nalgebra::DVector;

macro_rules! impl_as_type {
    ($target_type:ty) => {
        paste! {
          pub fn [<as_ $target_type>](&self) -> MResult<Ref<$target_type>> {
            match self {
              #[cfg(feature = "u8")]
              LegacyValue::U8(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "u16")]
              LegacyValue::U16(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "u32")]
              LegacyValue::U32(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "u64")]
              LegacyValue::U64(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "u128")]
              LegacyValue::U128(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "i8")]
              LegacyValue::I8(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "i16")]
              LegacyValue::I16(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "i32")]
              LegacyValue::I32(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "i64")]
              LegacyValue::I64(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "i128")]
              LegacyValue::I128(v) => Ok(Ref::new(*v.borrow() as $target_type)),
              #[cfg(feature = "f32")]
              LegacyValue::F32(v) => Ok(Ref::new((*v.borrow()) as $target_type)),
              #[cfg(feature = "f64")]
              LegacyValue::F64(v) => Ok(Ref::new((*v.borrow()) as $target_type)),
              LegacyValue::Id(v) => Ok(Ref::new(*v as $target_type)),
              LegacyValue::Typed(value, _) => value.[<as_ $target_type>](),
              LegacyValue::MutableReference(val) => val.borrow().[<as_ $target_type>](),
              _ => Err(
                MechError::new(
                  CannotConvertToTypeError { target_type: stringify!($target_type) },
                  None
                ).with_compiler_loc()
              ),
            }
          }
        }
    };
}

// Value Kind
// ----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ValueKind {
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
    Any,
    None,
    Matrix(Box<ValueKind>, Vec<usize>),
    Enum(u64, String),
    Record(Vec<(String, ValueKind)>),
    Map(Box<ValueKind>, Box<ValueKind>),
    Atom(u64, String),
    Table(Vec<(String, ValueKind)>, usize),
    Tuple(Vec<ValueKind>),
    Reference(Box<ValueKind>),
    Set(Box<ValueKind>, Option<usize>),
    Option(Box<ValueKind>),
    Kind(Box<ValueKind>),
}

impl Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueKind::R64 => write!(f, "r64"),
            ValueKind::C64 => write!(f, "c64"),
            ValueKind::U8 => write!(f, "u8"),
            ValueKind::U16 => write!(f, "u16"),
            ValueKind::U32 => write!(f, "u32"),
            ValueKind::U64 => write!(f, "u64"),
            ValueKind::U128 => write!(f, "u128"),
            ValueKind::I8 => write!(f, "i8"),
            ValueKind::I16 => write!(f, "i16"),
            ValueKind::I32 => write!(f, "i32"),
            ValueKind::I64 => write!(f, "i64"),
            ValueKind::I128 => write!(f, "i128"),
            ValueKind::F32 => write!(f, "f32"),
            ValueKind::F64 => write!(f, "f64"),
            ValueKind::String => write!(f, "string"),
            ValueKind::Bool => write!(f, "bool"),
            ValueKind::Matrix(x, s) => write!(
                f,
                "[{}]:{}",
                x,
                s.iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ValueKind::Set(x, el) => write!(
                f,
                "{{{}}}{}",
                x,
                el.map_or("".to_string(), |e| format!(":{}", e))
            ),
            ValueKind::Map(x, y) => write!(f, "{{{}:{}}}", x, y),
            ValueKind::Record(x) => write!(
                f,
                "{{{}}}",
                x.iter()
                    .map(|(i, k)| format!("{}<{}>", i.to_string(), k))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
            ValueKind::Table(x, y) => {
                let size_str = if y > &0 {
                    format!(":{}", y)
                } else {
                    "".to_string()
                };
                write!(
                    f,
                    "|{}|{}",
                    x.iter()
                        .map(|(i, k)| format!("{}<{}>", i.to_string(), k))
                        .collect::<Vec<String>>()
                        .join(" "),
                    size_str
                )
            }
            ValueKind::Tuple(x) => write!(
                f,
                "({})",
                x.iter()
                    .map(|x| format!("{}", x))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ValueKind::Id => write!(f, "id"),
            ValueKind::Index => write!(f, "ix"),
            ValueKind::Reference(x) => write!(f, "{}", x),
            ValueKind::Enum(_, name) => write!(f, ":{}", name),
            ValueKind::Atom(_, name) => write!(f, ":{}", name),
            ValueKind::Empty => write!(f, "_"),
            ValueKind::Any => write!(f, "*"),
            ValueKind::None => write!(f, "none"),
            ValueKind::Option(x) => write!(f, "{}?", x),
            ValueKind::Kind(x) => write!(f, "<{}>", x),
        }
    }
}

pub fn escape_html_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

impl ValueKind {
    fn format_with_budget(&self, budget: &mut InlineFormatBudget) -> String {
        match self {
            ValueKind::Matrix(element, shape) => format!(
                "[{}]:{}",
                element.format_with_budget(budget),
                shape
                    .iter()
                    .map(|dimension| dimension.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            ValueKind::Set(element, size) => format!(
                "{{{}}}{}",
                element.format_with_budget(budget),
                size.map_or(String::new(), |size| format!(":{size}")),
            ),
            ValueKind::Map(key, value) => format!(
                "{{{}:{}}}",
                key.format_with_budget(budget),
                value.format_with_budget(budget),
            ),
            ValueKind::Record(fields) => {
                let mut rendered = Vec::new();
                for (name, kind) in fields {
                    if !budget.consume() {
                        rendered.push("…<*>".to_string());
                        break;
                    }
                    rendered.push(format!("{}<{}>", name, kind.format_with_budget(budget)));
                }
                format!("{{{}}}", rendered.join(" "))
            }
            ValueKind::Table(columns, rows) => {
                let mut rendered = Vec::new();
                for (name, kind) in columns {
                    if !budget.consume() {
                        rendered.push("…<*>".to_string());
                        break;
                    }
                    rendered.push(format!("{}<{}>", name, kind.format_with_budget(budget)));
                }
                let size = (*rows > 0).then(|| format!(":{rows}")).unwrap_or_default();
                format!("|{}|{size}", rendered.join(" "))
            }
            ValueKind::Tuple(elements) => {
                let mut rendered = Vec::new();
                for kind in elements {
                    if !budget.consume() {
                        rendered.push("…".to_string());
                        break;
                    }
                    rendered.push(kind.format_with_budget(budget));
                }
                format!("({})", rendered.join(","))
            }
            ValueKind::Reference(inner) => inner.format_with_budget(budget),
            ValueKind::Option(inner) => format!("{}?", inner.format_with_budget(budget)),
            ValueKind::Kind(inner) => format!("<{}>", inner.format_with_budget(budget)),
            scalar => scalar.to_string(),
        }
    }

    /// Formats a structural kind without allowing record, table, or tuple
    /// schemas to grow beyond the shared interactive element budget.
    pub fn format_with_element_limit(&self, limit: usize) -> String {
        self.format_with_budget(&mut InlineFormatBudget::bounded(limit))
    }

    pub fn collection_kind(&self) -> Option<ValueKind> {
        match self {
            ValueKind::Matrix(x, _) => Some(*x.clone()),
            ValueKind::Set(x, _) => Some(*x.clone()),
            _ => None,
        }
    }

    pub fn deref_kind(&self) -> ValueKind {
        match self {
            ValueKind::Reference(x) => *x.clone(),
            _ => self.clone(),
        }
    }

    pub fn is_convertible_to(&self, other: &ValueKind) -> bool {
        match (self, other) {
            // Unsigned widening
            (ValueKind::U8, ValueKind::U16)
            | (ValueKind::U8, ValueKind::U32)
            | (ValueKind::U8, ValueKind::U64)
            | (ValueKind::U8, ValueKind::U128)
            | (ValueKind::U16, ValueKind::U32)
            | (ValueKind::U16, ValueKind::U64)
            | (ValueKind::U16, ValueKind::U128)
            | (ValueKind::U32, ValueKind::U64)
            | (ValueKind::U32, ValueKind::U128)
            | (ValueKind::U64, ValueKind::U128) => true,

            // Signed widening
            (ValueKind::I8, ValueKind::I16)
            | (ValueKind::I8, ValueKind::I32)
            | (ValueKind::I8, ValueKind::I64)
            | (ValueKind::I8, ValueKind::I128)
            | (ValueKind::I16, ValueKind::I32)
            | (ValueKind::I16, ValueKind::I64)
            | (ValueKind::I16, ValueKind::I128)
            | (ValueKind::I32, ValueKind::I64)
            | (ValueKind::I32, ValueKind::I128)
            | (ValueKind::I64, ValueKind::I128) => true,

            // Unsigned -> signed widening
            (ValueKind::U8, ValueKind::I16)
            | (ValueKind::U8, ValueKind::I32)
            | (ValueKind::U8, ValueKind::I64)
            | (ValueKind::U8, ValueKind::I128)
            | (ValueKind::U16, ValueKind::I32)
            | (ValueKind::U16, ValueKind::I64)
            | (ValueKind::U16, ValueKind::I128)
            | (ValueKind::U32, ValueKind::I64)
            | (ValueKind::U32, ValueKind::I128)
            | (ValueKind::U64, ValueKind::I128) => true,

            // Signed -> unsigned widening (runtime safety not enforced here)
            (ValueKind::I8, ValueKind::U16)
            | (ValueKind::I8, ValueKind::U32)
            | (ValueKind::I8, ValueKind::U64)
            | (ValueKind::I8, ValueKind::U128)
            | (ValueKind::I16, ValueKind::U32)
            | (ValueKind::I16, ValueKind::U64)
            | (ValueKind::I16, ValueKind::U128)
            | (ValueKind::I32, ValueKind::U64)
            | (ValueKind::I32, ValueKind::U128)
            | (ValueKind::I64, ValueKind::U128) => true,

            // Integer -> float
            (ValueKind::U8, ValueKind::F32)
            | (ValueKind::U8, ValueKind::F64)
            | (ValueKind::U16, ValueKind::F32)
            | (ValueKind::U16, ValueKind::F64)
            | (ValueKind::U32, ValueKind::F32)
            | (ValueKind::U32, ValueKind::F64)
            | (ValueKind::U64, ValueKind::F32)
            | (ValueKind::U64, ValueKind::F64)
            | (ValueKind::U128, ValueKind::F32)
            | (ValueKind::U128, ValueKind::F64)
            | (ValueKind::I8, ValueKind::F32)
            | (ValueKind::I8, ValueKind::F64)
            | (ValueKind::I16, ValueKind::F32)
            | (ValueKind::I16, ValueKind::F64)
            | (ValueKind::I32, ValueKind::F32)
            | (ValueKind::I32, ValueKind::F64)
            | (ValueKind::I64, ValueKind::F32)
            | (ValueKind::I64, ValueKind::F64)
            | (ValueKind::I128, ValueKind::F32)
            | (ValueKind::I128, ValueKind::F64) => true,

            // Float widening + narrowing
            (ValueKind::F32, ValueKind::F64) | (ValueKind::F64, ValueKind::F32) => true,

            // Float -> integer (allowed, but lossy)
            (ValueKind::F32, ValueKind::I8)
            | (ValueKind::F32, ValueKind::I16)
            | (ValueKind::F32, ValueKind::I32)
            | (ValueKind::F32, ValueKind::I64)
            | (ValueKind::F32, ValueKind::I128)
            | (ValueKind::F32, ValueKind::U8)
            | (ValueKind::F32, ValueKind::U16)
            | (ValueKind::F32, ValueKind::U32)
            | (ValueKind::F32, ValueKind::U64)
            | (ValueKind::F32, ValueKind::U128)
            | (ValueKind::F64, ValueKind::I8)
            | (ValueKind::F64, ValueKind::I16)
            | (ValueKind::F64, ValueKind::I32)
            | (ValueKind::F64, ValueKind::I64)
            | (ValueKind::F64, ValueKind::I128)
            | (ValueKind::F64, ValueKind::U8)
            | (ValueKind::F64, ValueKind::U16)
            | (ValueKind::F64, ValueKind::U32)
            | (ValueKind::F64, ValueKind::U64)
            | (ValueKind::F64, ValueKind::U128) => true,

            // Index conversions (both ways)
            (ValueKind::Index, ValueKind::U8)
            | (ValueKind::Index, ValueKind::U16)
            | (ValueKind::Index, ValueKind::U32)
            | (ValueKind::Index, ValueKind::U64)
            | (ValueKind::Index, ValueKind::U128)
            | (ValueKind::Index, ValueKind::I8)
            | (ValueKind::Index, ValueKind::I16)
            | (ValueKind::Index, ValueKind::I32)
            | (ValueKind::Index, ValueKind::I64)
            | (ValueKind::Index, ValueKind::I128)
            | (ValueKind::Index, ValueKind::F32)
            | (ValueKind::Index, ValueKind::F64)
            | (ValueKind::U8, ValueKind::Index)
            | (ValueKind::U16, ValueKind::Index)
            | (ValueKind::U32, ValueKind::Index)
            | (ValueKind::U64, ValueKind::Index)
            | (ValueKind::U128, ValueKind::Index)
            | (ValueKind::I8, ValueKind::Index)
            | (ValueKind::I16, ValueKind::Index)
            | (ValueKind::I32, ValueKind::Index)
            | (ValueKind::I64, ValueKind::Index)
            | (ValueKind::I128, ValueKind::Index) => true,

            // Matrix: element type convertible and shape matches.
            // An empty target shape (`[]`) is treated as a wildcard shape.
            (ValueKind::Matrix(a, _ashape), ValueKind::Matrix(b, bshape))
                if bshape.is_empty() && a.as_ref().is_convertible_to(b.as_ref()) =>
            {
                true
            }
            (ValueKind::Matrix(a, ashape), ValueKind::Matrix(b, bshape))
                if ashape.into_iter().product::<usize>()
                    == bshape.into_iter().product::<usize>()
                    && a.as_ref().is_convertible_to(b.as_ref()) =>
            {
                true
            }

            // Option conversions
            (x, ValueKind::Option(b)) if x.is_convertible_to(b.as_ref()) => true,
            (ValueKind::Empty, ValueKind::Option(_)) => true,
            (ValueKind::Option(a), ValueKind::Option(b))
                if a.as_ref().is_convertible_to(b.as_ref()) =>
            {
                true
            }

            // Reference conversions
            (ValueKind::Reference(a), ValueKind::Reference(b))
                if a.as_ref().is_convertible_to(b.as_ref()) =>
            {
                true
            }

            // Tuple conversions (element-wise)
            (ValueKind::Tuple(a), ValueKind::Tuple(b))
                if a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(x, y)| x.is_convertible_to(y)) =>
            {
                true
            }

            // Set conversions
            (ValueKind::Set(a, _), ValueKind::Set(b, _))
                if a.as_ref().is_convertible_to(b.as_ref()) =>
            {
                true
            }

            // Map conversions
            (ValueKind::Map(ak, av), ValueKind::Map(bk, bv))
                if ak.as_ref().is_convertible_to(bk.as_ref())
                    && av.as_ref().is_convertible_to(bv.as_ref()) =>
            {
                true
            }

            // Table conversions: allow source to have extra columns
            (ValueKind::Table(acols, _), ValueKind::Table(bcols, _))
                if bcols.iter().all(|(bk, bv)| {
                    acols
                        .iter()
                        .any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
                }) =>
            {
                true
            }

            // Record conversions: allow source to have extra fields
            (ValueKind::Record(afields), ValueKind::Record(bfields))
                if bfields.iter().all(|(bk, bv)| {
                    afields
                        .iter()
                        .any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
                }) =>
            {
                true
            }

            // Direct match
            _ => self == other,
        }
    }

    pub fn is_compatible(k1: ValueKind, k2: ValueKind) -> bool {
        match k1 {
            ValueKind::Reference(x) => ValueKind::is_compatible(*x, k2),
            ValueKind::Matrix(x, _) => *x == k2,
            x => x == k2,
        }
    }

    pub fn align(&self) -> usize {
        // pointer alignment (platform word size)
        let ptr_align = mem::align_of::<usize>();

        match self {
            // unsigned integers
            ValueKind::U8 => 1,
            ValueKind::U16 => 2,
            ValueKind::U32 => 4,
            ValueKind::U64 => 8,
            ValueKind::U128 => 16,
            // signed integers
            ValueKind::I8 => 1,
            ValueKind::I16 => 2,
            ValueKind::I32 => 4,
            ValueKind::I64 => 8,
            ValueKind::I128 => 16,
            // floats
            ValueKind::F32 => 4,
            ValueKind::F64 => 8,
            // complex / rational (assume composed of f64 parts)
            ValueKind::C64 => 8,
            ValueKind::R64 => 8,
            // small simple payloads
            ValueKind::Bool => 1,
            ValueKind::String => ptr_align, // String = (ptr, len, cap)
            ValueKind::Id | ValueKind::Index => 8,
            ValueKind::Empty => 1,
            ValueKind::Any => ptr_align,
            ValueKind::None => 1,
            // compound types
            ValueKind::Matrix(elem_ty, _) => {
                // flat element storage
                elem_ty.align()
            }
            // inline enum / atom payloads
            ValueKind::Enum(_, _) => 8, // u64 + String => max(8, 8)
            ValueKind::Atom(_, _) => 8,
            ValueKind::Record(fields) => {
                // record alignment = max field alignment
                fields.iter().map(|(_, ty)| ty.align()).max().unwrap_or(1)
            }
            // pointer-backed containers
            ValueKind::Map(_, _) => ptr_align,
            ValueKind::Table(cols, _) => cols
                .iter()
                .map(|(_, ty)| ty.align())
                .max()
                .unwrap_or(ptr_align),
            ValueKind::Tuple(elems) => elems.iter().map(|ty| ty.align()).max().unwrap_or(1),
            ValueKind::Reference(_) => ptr_align,
            ValueKind::Set(elem, _) => {
                // if Set is implemented inline; otherwise use ptr_align
                elem.align()
            }
            ValueKind::Option(inner) => inner.align(),
            ValueKind::Kind(inner) => inner.align(),
        }
    }
}

pub trait AsNaKind {
    fn as_na_kind() -> String;
}

#[cfg(feature = "matrix")]
macro_rules! impl_as_na_kind {
    ($type:ty, $kind:expr) => {
        impl<T> AsNaKind for $type {
            fn as_na_kind() -> String {
                $kind.to_string()
            }
        }
    };
}

#[cfg(feature = "row_vector2")]
impl_as_na_kind!(RowVector2<T>, "RowVector2");
#[cfg(feature = "row_vector3")]
impl_as_na_kind!(RowVector3<T>, "RowVector3");
#[cfg(feature = "row_vector4")]
impl_as_na_kind!(RowVector4<T>, "RowVector4");
#[cfg(feature = "row_vectord")]
impl_as_na_kind!(RowDVector<T>, "RowDVector");
#[cfg(feature = "vector2")]
impl_as_na_kind!(Vector2<T>, "Vector2");
#[cfg(feature = "vector3")]
impl_as_na_kind!(Vector3<T>, "Vector3");
#[cfg(feature = "vector4")]
impl_as_na_kind!(Vector4<T>, "Vector4");
#[cfg(feature = "vectord")]
impl_as_na_kind!(DVector<T>, "DVector");
#[cfg(feature = "matrix1")]
impl_as_na_kind!(Matrix1<T>, "Matrix1");
#[cfg(feature = "matrix2")]
impl_as_na_kind!(Matrix2<T>, "Matrix2");
#[cfg(feature = "matrix3")]
impl_as_na_kind!(Matrix3<T>, "Matrix3");
#[cfg(feature = "matrix4")]
impl_as_na_kind!(Matrix4<T>, "Matrix4");
#[cfg(feature = "matrix2x3")]
impl_as_na_kind!(Matrix2x3<T>, "Matrix2x3");
#[cfg(feature = "matrix3x2")]
impl_as_na_kind!(Matrix3x2<T>, "Matrix3x2");
#[cfg(feature = "matrixd")]
impl_as_na_kind!(DMatrix<T>, "DMatrix");

pub trait AsValueKind {
    fn as_value_kind() -> ValueKind;
}

macro_rules! impl_as_value_kind {
    ($type:ty, $value_kind:expr) => {
        impl AsValueKind for $type {
            fn as_value_kind() -> ValueKind {
                $value_kind
            }
        }
    };
}

impl_as_value_kind!(usize, ValueKind::Index);

#[cfg(feature = "i8")]
impl_as_value_kind!(i8, ValueKind::I8);
#[cfg(feature = "i16")]
impl_as_value_kind!(i16, ValueKind::I16);
#[cfg(feature = "i32")]
impl_as_value_kind!(i32, ValueKind::I32);
#[cfg(feature = "i64")]
impl_as_value_kind!(i64, ValueKind::I64);
#[cfg(feature = "i128")]
impl_as_value_kind!(i128, ValueKind::I128);
#[cfg(feature = "u8")]
impl_as_value_kind!(u8, ValueKind::U8);
#[cfg(feature = "u16")]
impl_as_value_kind!(u16, ValueKind::U16);
#[cfg(feature = "u32")]
impl_as_value_kind!(u32, ValueKind::U32);
#[cfg(feature = "u64")]
impl_as_value_kind!(u64, ValueKind::U64);
#[cfg(feature = "u128")]
impl_as_value_kind!(u128, ValueKind::U128);
#[cfg(feature = "f32")]
impl_as_value_kind!(f32, ValueKind::F32);
#[cfg(feature = "f64")]
impl_as_value_kind!(f64, ValueKind::F64);
#[cfg(any(feature = "bool", feature = "variable_define"))]
impl_as_value_kind!(bool, ValueKind::Bool);
#[cfg(any(feature = "string", feature = "variable_define"))]
impl_as_value_kind!(String, ValueKind::String);
#[cfg(feature = "rational")]
impl_as_value_kind!(R64, ValueKind::R64);
#[cfg(feature = "complex")]
impl_as_value_kind!(C64, ValueKind::C64);

#[cfg(feature = "matrix")]
macro_rules! impl_as_value_kind_for_matrix {
    ($type:ty, $dims:expr) => {
        impl<T: AsValueKind> AsValueKind for $type {
            fn as_value_kind() -> ValueKind {
                ValueKind::Matrix(Box::new(T::as_value_kind()), $dims)
            }
        }
    };
}

#[cfg(feature = "row_vectord")]
impl_as_value_kind_for_matrix!(RowDVector<T>, vec![1, 0]);
#[cfg(feature = "row_vector2")]
impl_as_value_kind_for_matrix!(RowVector2<T>, vec![1, 2]);
#[cfg(feature = "row_vector3")]
impl_as_value_kind_for_matrix!(RowVector3<T>, vec![1, 3]);
#[cfg(feature = "row_vector4")]
impl_as_value_kind_for_matrix!(RowVector4<T>, vec![1, 4]);
#[cfg(feature = "vectord")]
impl_as_value_kind_for_matrix!(DVector<T>, vec![0, 1]);
#[cfg(feature = "vector2")]
impl_as_value_kind_for_matrix!(Vector2<T>, vec![2, 1]);
#[cfg(feature = "vector3")]
impl_as_value_kind_for_matrix!(Vector3<T>, vec![3, 1]);
#[cfg(feature = "vector4")]
impl_as_value_kind_for_matrix!(Vector4<T>, vec![4, 1]);
#[cfg(feature = "matrix1")]
impl_as_value_kind_for_matrix!(Matrix1<T>, vec![1, 1]);
#[cfg(feature = "matrix2")]
impl_as_value_kind_for_matrix!(Matrix2<T>, vec![2, 2]);
#[cfg(feature = "matrix3")]
impl_as_value_kind_for_matrix!(Matrix3<T>, vec![3, 3]);
#[cfg(feature = "matrix4")]
impl_as_value_kind_for_matrix!(Matrix4<T>, vec![4, 4]);
#[cfg(feature = "matrix2x3")]
impl_as_value_kind_for_matrix!(Matrix2x3<T>, vec![2, 3]);
#[cfg(feature = "matrix3x2")]
impl_as_value_kind_for_matrix!(Matrix3x2<T>, vec![3, 2]);
#[cfg(feature = "matrixd")]
impl_as_value_kind_for_matrix!(DMatrix<T>, vec![0, 0]);

impl AsValueKind for LegacyValue {
    fn as_value_kind() -> ValueKind {
        ValueKind::Any
    }
}

// Value
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum LegacyValue {
    #[cfg(feature = "u8")]
    U8(Ref<u8>),
    #[cfg(feature = "u16")]
    U16(Ref<u16>),
    #[cfg(feature = "u32")]
    U32(Ref<u32>),
    #[cfg(feature = "u64")]
    U64(Ref<u64>),
    #[cfg(feature = "u128")]
    U128(Ref<u128>),
    #[cfg(feature = "i8")]
    I8(Ref<i8>),
    #[cfg(feature = "i16")]
    I16(Ref<i16>),
    #[cfg(feature = "i32")]
    I32(Ref<i32>),
    #[cfg(feature = "i64")]
    I64(Ref<i64>),
    #[cfg(feature = "i128")]
    I128(Ref<i128>),
    #[cfg(feature = "f32")]
    F32(Ref<f32>),
    #[cfg(feature = "f64")]
    F64(Ref<f64>),
    #[cfg(any(feature = "string", feature = "variable_define"))]
    String(Ref<String>),
    #[cfg(any(feature = "bool", feature = "variable_define"))]
    Bool(Ref<bool>),
    #[cfg(feature = "atom")]
    Atom(Ref<MechAtom>),
    #[cfg(feature = "matrix")]
    MatrixIndex(Matrix<usize>),
    #[cfg(all(feature = "matrix", feature = "bool"))]
    MatrixBool(Matrix<bool>),
    #[cfg(all(feature = "matrix", feature = "u8"))]
    MatrixU8(Matrix<u8>),
    #[cfg(all(feature = "matrix", feature = "u16"))]
    MatrixU16(Matrix<u16>),
    #[cfg(all(feature = "matrix", feature = "u32"))]
    MatrixU32(Matrix<u32>),
    #[cfg(all(feature = "matrix", feature = "u64"))]
    MatrixU64(Matrix<u64>),
    #[cfg(all(feature = "matrix", feature = "u128"))]
    MatrixU128(Matrix<u128>),
    #[cfg(all(feature = "matrix", feature = "i8"))]
    MatrixI8(Matrix<i8>),
    #[cfg(all(feature = "matrix", feature = "i16"))]
    MatrixI16(Matrix<i16>),
    #[cfg(all(feature = "matrix", feature = "i32"))]
    MatrixI32(Matrix<i32>),
    #[cfg(all(feature = "matrix", feature = "i64"))]
    MatrixI64(Matrix<i64>),
    #[cfg(all(feature = "matrix", feature = "i128"))]
    MatrixI128(Matrix<i128>),
    #[cfg(all(feature = "matrix", feature = "f32"))]
    MatrixF32(Matrix<f32>),
    #[cfg(all(feature = "matrix", feature = "f64"))]
    MatrixF64(Matrix<f64>),
    #[cfg(all(feature = "matrix", feature = "string"))]
    MatrixString(Matrix<String>),
    #[cfg(all(feature = "matrix", feature = "rational"))]
    MatrixR64(Matrix<R64>),
    #[cfg(all(feature = "matrix", feature = "complex"))]
    MatrixC64(Matrix<C64>),
    #[cfg(feature = "matrix")]
    MatrixValue(Matrix<LegacyValue>),
    #[cfg(feature = "complex")]
    C64(Ref<C64>),
    #[cfg(feature = "rational")]
    R64(Ref<R64>),
    #[cfg(feature = "set")]
    Set(Ref<MechSet>),
    #[cfg(feature = "map")]
    Map(Ref<MechMap>),
    #[cfg(feature = "record")]
    Record(Ref<MechRecord>),
    #[cfg(feature = "table")]
    Table(Ref<MechTable>),
    #[cfg(feature = "tuple")]
    Tuple(Ref<MechTuple>),
    #[cfg(feature = "enum")]
    Enum(Ref<MechEnum>),
    Id(u64),
    Index(Ref<usize>),
    MutableReference(MutableReference),
    Typed(Box<LegacyValue>, ValueKind),
    Kind(ValueKind),
    IndexAll,
    EmptyKind(ValueKind),
    Empty,
}

/// Shared traversal budget for portable inline projections. A single budget
/// follows nested values so a small outer container cannot hide an unbounded
/// table, map, tuple, or matrix inside one of its entries.
#[derive(Clone, Copy, Debug)]
struct InlineFormatBudget {
    remaining: Option<usize>,
    elided: bool,
}

impl InlineFormatBudget {
    fn unlimited() -> Self {
        Self {
            remaining: None,
            elided: false,
        }
    }

    fn bounded(limit: usize) -> Self {
        Self {
            remaining: Some(limit),
            elided: false,
        }
    }

    fn consume(&mut self) -> bool {
        match &mut self.remaining {
            Some(0) => {
                self.elided = true;
                false
            }
            Some(remaining) => {
                *remaining -= 1;
                true
            }
            None => true,
        }
    }
}

impl Eq for LegacyValue {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// A process-local identity used by the reactive scheduler.
///
/// This is not a durable cell identity and must not be persisted as transaction
/// history. Durable history requires a stable logical cell ID that is
/// independent of the current [`Ref`] backing representation.
pub struct ReactiveCellId(u64);

impl ReactiveCellId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for LegacyValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[cfg(feature = "pretty_print")]
        return fmt::Display::fmt(&self.pretty_print(), f);
        #[cfg(not(feature = "pretty_print"))]
        f.write_str(&self.format_canonical_inline())
    }
}

impl Hash for LegacyValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            #[cfg(feature = "rational")]
            LegacyValue::R64(x) => x.borrow().hash(state),
            #[cfg(feature = "u8")]
            LegacyValue::U8(x) => x.borrow().hash(state),
            #[cfg(feature = "u16")]
            LegacyValue::U16(x) => x.borrow().hash(state),
            #[cfg(feature = "u32")]
            LegacyValue::U32(x) => x.borrow().hash(state),
            #[cfg(feature = "u64")]
            LegacyValue::U64(x) => x.borrow().hash(state),
            #[cfg(feature = "u128")]
            LegacyValue::U128(x) => x.borrow().hash(state),
            #[cfg(feature = "i8")]
            LegacyValue::I8(x) => x.borrow().hash(state),
            #[cfg(feature = "i16")]
            LegacyValue::I16(x) => x.borrow().hash(state),
            #[cfg(feature = "i32")]
            LegacyValue::I32(x) => x.borrow().hash(state),
            #[cfg(feature = "i64")]
            LegacyValue::I64(x) => x.borrow().hash(state),
            #[cfg(feature = "i128")]
            LegacyValue::I128(x) => x.borrow().hash(state),
            #[cfg(feature = "f32")]
            LegacyValue::F32(x) => x.borrow().to_bits().hash(state),
            #[cfg(feature = "f64")]
            LegacyValue::F64(x) => x.borrow().to_bits().hash(state),
            #[cfg(feature = "complex")]
            LegacyValue::C64(x) => x.borrow().hash(state),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(x) => x.borrow().hash(state),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(x) => x.borrow().hash(state),
            #[cfg(feature = "set")]
            LegacyValue::Set(x) => x.borrow().hash(state),
            #[cfg(feature = "map")]
            LegacyValue::Map(x) => x.borrow().hash(state),
            #[cfg(feature = "table")]
            LegacyValue::Table(x) => x.borrow().hash(state),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(x) => x.borrow().hash(state),
            #[cfg(feature = "record")]
            LegacyValue::Record(x) => x.borrow().hash(state),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(x) => x.borrow().hash(state),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(x) => x.borrow().hash(state),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(x) => x.hash(state),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(x) => {
                core::mem::discriminant(x).hash(state);
                x.shape().hash(state);
                for value in x.as_vec() {
                    value.to_bits().hash(state);
                }
            }
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(x) => {
                core::mem::discriminant(x).hash(state);
                x.shape().hash(state);
                for value in x.as_vec() {
                    value.to_bits().hash(state);
                }
            }
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(x) => x.hash(state),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(x) => x.hash(state),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(x) => x.hash(state),
            LegacyValue::Id(x) => x.hash(state),
            LegacyValue::Kind(x) => x.hash(state),
            LegacyValue::Typed(v, k) => {
                v.hash(state);
                k.hash(state);
            }
            LegacyValue::Index(x) => x.borrow().hash(state),
            LegacyValue::MutableReference(x) => x.borrow().hash(state),
            LegacyValue::EmptyKind(k) => k.hash(state),
            LegacyValue::Empty | LegacyValue::IndexAll => core::mem::discriminant(self).hash(state),
        }
    }
}
impl LegacyValue {
    pub fn reactive_root_cell_ids(&self) -> Vec<ReactiveCellId> {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "complex")]
            LegacyValue::C64(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "rational")]
            LegacyValue::R64(v) => vec![ReactiveCellId::new(v.id())],
            LegacyValue::Index(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "atom")]
            LegacyValue::Atom(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "enum")]
            LegacyValue::Enum(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "set")]
            LegacyValue::Set(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "map")]
            LegacyValue::Map(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "record")]
            LegacyValue::Record(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "table")]
            LegacyValue::Table(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(v) => vec![ReactiveCellId::new(v.id())],
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(v) => vec![ReactiveCellId::new(v.addr() as u64)],
            LegacyValue::MutableReference(v) => vec![ReactiveCellId::new(v.id())],
            LegacyValue::Typed(value, _) => value.reactive_root_cell_ids(),
            LegacyValue::Id(_)
            | LegacyValue::Kind(_)
            | LegacyValue::IndexAll
            | LegacyValue::EmptyKind(_)
            | LegacyValue::Empty => Vec::new(),
        }
    }

    pub fn reactive_cell_ids(&self) -> Vec<ReactiveCellId> {
        let mut ids = Vec::new();
        let mut seen = HashSet::default();

        self.collect_reactive_cell_ids(&mut ids, &mut seen);

        ids
    }

    /// Returns the cells that carry a value's logical reactive state.
    ///
    /// Mutable-reference wrappers provide stable storage identity, but the
    /// value inside the wrapper remains the dependency observed by source
    /// execution. External bytecode nodes use this view so stable register
    /// storage does not become an extra reactive input.
    pub fn logical_reactive_cell_ids(&self) -> Vec<ReactiveCellId> {
        fn collect(
            value: &LegacyValue,
            ids: &mut Vec<ReactiveCellId>,
            seen_references: &mut HashSet<ReactiveCellId>,
        ) {
            match value {
                LegacyValue::MutableReference(reference) => {
                    let cell = ReactiveCellId::new(reference.id());
                    if !seen_references.insert(cell) {
                        if !ids.contains(&cell) {
                            ids.push(cell);
                        }
                        return;
                    }
                    collect(&reference.borrow(), ids, seen_references);
                }
                LegacyValue::Typed(value, _) => collect(value, ids, seen_references),
                _ => {
                    for cell in value.reactive_cell_ids() {
                        if !ids.contains(&cell) {
                            ids.push(cell);
                        }
                    }
                }
            }
        }

        let mut ids = Vec::new();
        let mut seen_references = HashSet::default();
        collect(self, &mut ids, &mut seen_references);
        ids
    }

    fn push_reactive_cell_id(
        ids: &mut Vec<ReactiveCellId>,
        seen: &mut HashSet<ReactiveCellId>,
        id: u64,
    ) -> bool {
        let cell = ReactiveCellId::new(id);

        if seen.insert(cell) {
            ids.push(cell);
            true
        } else {
            false
        }
    }

    fn collect_reactive_cell_ids(
        &self,
        ids: &mut Vec<ReactiveCellId>,
        seen: &mut HashSet<ReactiveCellId>,
    ) {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "complex")]
            LegacyValue::C64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "rational")]
            LegacyValue::R64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            LegacyValue::Index(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "atom")]
            LegacyValue::Atom(v) => {
                Self::push_reactive_cell_id(ids, seen, v.id());
            }
            #[cfg(feature = "enum")]
            LegacyValue::Enum(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let enum_brrw = v.borrow();
                    for (_, payload) in &enum_brrw.variants {
                        if let Some(payload) = payload {
                            payload.collect_reactive_cell_ids(ids, seen);
                        }
                    }
                }
            }
            #[cfg(feature = "set")]
            LegacyValue::Set(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let set_brrw = v.borrow();
                    for value in &set_brrw.set {
                        value.collect_reactive_cell_ids(ids, seen);
                    }
                }
            }
            #[cfg(feature = "map")]
            LegacyValue::Map(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let map_brrw = v.borrow();
                    for (key, value) in &map_brrw.map {
                        key.collect_reactive_cell_ids(ids, seen);
                        value.collect_reactive_cell_ids(ids, seen);
                    }
                }
            }
            #[cfg(feature = "record")]
            LegacyValue::Record(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let record_brrw = v.borrow();
                    for value in record_brrw.data.values() {
                        value.collect_reactive_cell_ids(ids, seen);
                    }
                }
            }
            #[cfg(feature = "table")]
            LegacyValue::Table(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let table_brrw = v.borrow();
                    for (_, column) in table_brrw.data.values() {
                        if Self::push_reactive_cell_id(ids, seen, column.addr() as u64) {
                            for value in column.as_vec().iter() {
                                value.collect_reactive_cell_ids(ids, seen);
                            }
                        }
                    }
                }
            }
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    let tuple_brrw = v.borrow();
                    for value in &tuple_brrw.elements {
                        value.collect_reactive_cell_ids(ids, seen);
                    }
                }
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(v) => {
                Self::push_reactive_cell_id(ids, seen, v.addr() as u64);
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.addr() as u64) {
                    for value in v.as_vec().iter() {
                        value.collect_reactive_cell_ids(ids, seen);
                    }
                }
            }
            LegacyValue::MutableReference(v) => {
                if Self::push_reactive_cell_id(ids, seen, v.id()) {
                    v.borrow().collect_reactive_cell_ids(ids, seen);
                }
            }
            LegacyValue::Typed(value, _) => value.collect_reactive_cell_ids(ids, seen),
            LegacyValue::Id(_)
            | LegacyValue::Kind(_)
            | LegacyValue::IndexAll
            | LegacyValue::EmptyKind(_)
            | LegacyValue::Empty => {}
        }
    }
}

pub fn legacy_ref_reactive_cell_ids(value: &Ref<LegacyValue>) -> Vec<ReactiveCellId> {
    let mut ids = Vec::new();
    let mut seen = HashSet::default();

    LegacyValue::push_reactive_cell_id(&mut ids, &mut seen, value.id());

    value
        .borrow()
        .collect_reactive_cell_ids(&mut ids, &mut seen);

    ids
}
impl LegacyValue {
    /// Creates a detached copy of an acyclic value graph.
    ///
    /// Every reachable reference-backed cell is detached. Repeated source handles
    /// remain shared within the detached graph. Atom and enum dictionaries are
    /// detached. Acyclic mutable-reference chains are value-transparent.
    ///
    /// Cyclic graphs return `ValueSnapshotCycleUnsupported` before the detached
    /// clone phase begins.
    pub fn try_deep_snapshot(&self) -> MResult<LegacyValue> {
        crate::value_snapshot::try_deep_snapshot(self)
    }

    #[cfg(feature = "matrix")]
    fn infer_matrix_value_kind(matrix: &Matrix<LegacyValue>) -> ValueKind {
        let mut base_kind: Option<ValueKind> = None;
        let mut saw_empty = false;

        for value in matrix.as_vec().iter() {
            match value {
                LegacyValue::Empty | LegacyValue::EmptyKind(_) => {
                    saw_empty = true;
                }
                _ => {
                    let kind = value.kind();
                    let (normalized_kind, normalized_empty) = match kind {
                        ValueKind::Option(inner) => ((*inner).clone(), true),
                        other => (other, false),
                    };
                    saw_empty |= normalized_empty;
                    match &base_kind {
                        None => base_kind = Some(normalized_kind),
                        Some(existing) if *existing == normalized_kind => {}
                        Some(_) => return ValueKind::Any,
                    }
                }
            }
        }

        match (base_kind, saw_empty) {
            (Some(kind), true) => ValueKind::Option(Box::new(kind)),
            (Some(kind), false) => kind,
            (None, true) => ValueKind::Option(Box::new(ValueKind::Any)),
            (None, false) => ValueKind::Any,
        }
    }

    /// Returns the exact `Ref<_>` stored by this value, if it has one.
    #[cfg(feature = "functions")]
    pub(crate) fn exact_ref_any(&self) -> Option<&dyn Any> {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(r) => Some(r),
            #[cfg(feature = "u16")]
            LegacyValue::U16(r) => Some(r),
            #[cfg(feature = "u32")]
            LegacyValue::U32(r) => Some(r),
            #[cfg(feature = "u64")]
            LegacyValue::U64(r) => Some(r),
            #[cfg(feature = "u128")]
            LegacyValue::U128(r) => Some(r),
            #[cfg(feature = "i8")]
            LegacyValue::I8(r) => Some(r),
            #[cfg(feature = "i16")]
            LegacyValue::I16(r) => Some(r),
            #[cfg(feature = "i32")]
            LegacyValue::I32(r) => Some(r),
            #[cfg(feature = "i64")]
            LegacyValue::I64(r) => Some(r),
            #[cfg(feature = "i128")]
            LegacyValue::I128(r) => Some(r),
            #[cfg(feature = "f32")]
            LegacyValue::F32(r) => Some(r),
            #[cfg(feature = "f64")]
            LegacyValue::F64(r) => Some(r),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(r) => Some(r),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(r) => Some(r),
            #[cfg(feature = "rational")]
            LegacyValue::R64(r) => Some(r),
            #[cfg(feature = "complex")]
            LegacyValue::C64(r) => Some(r),
            #[cfg(all(feature = "f64", feature = "matrix"))]
            LegacyValue::MatrixF64(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "f32", feature = "matrix"))]
            LegacyValue::MatrixF32(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "i8", feature = "matrix"))]
            LegacyValue::MatrixI8(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "i16", feature = "matrix"))]
            LegacyValue::MatrixI16(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "i32", feature = "matrix"))]
            LegacyValue::MatrixI32(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "i64", feature = "matrix"))]
            LegacyValue::MatrixI64(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "i128", feature = "matrix"))]
            LegacyValue::MatrixI128(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "u8", feature = "matrix"))]
            LegacyValue::MatrixU8(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "u16", feature = "matrix"))]
            LegacyValue::MatrixU16(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "u32", feature = "matrix"))]
            LegacyValue::MatrixU32(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "u64", feature = "matrix"))]
            LegacyValue::MatrixU64(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "u128", feature = "matrix"))]
            LegacyValue::MatrixU128(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "bool", feature = "matrix"))]
            LegacyValue::MatrixBool(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "string", feature = "matrix"))]
            LegacyValue::MatrixString(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "rational", feature = "matrix"))]
            LegacyValue::MatrixR64(r) => Some(r.exact_ref_any()),
            #[cfg(all(feature = "complex", feature = "matrix"))]
            LegacyValue::MatrixC64(r) => Some(r.exact_ref_any()),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(r) => Some(r.exact_ref_any()),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(r) => Some(r.exact_ref_any()),
            LegacyValue::Index(r) => Some(r),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(r) => Some(r),
            #[cfg(feature = "set")]
            LegacyValue::Set(r) => Some(r),
            #[cfg(feature = "table")]
            LegacyValue::Table(r) => Some(r),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(r) => Some(r),
            #[cfg(feature = "record")]
            LegacyValue::Record(r) => Some(r),
            #[cfg(feature = "map")]
            LegacyValue::Map(r) => Some(r),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(r) => Some(r),
            LegacyValue::MutableReference(r) => Some(r),
            LegacyValue::Id(_)
            | LegacyValue::Typed(_, _)
            | LegacyValue::Kind(_)
            | LegacyValue::IndexAll
            | LegacyValue::EmptyKind(_)
            | LegacyValue::Empty => None,
        }
    }

    #[cfg(feature = "matrix")]
    pub fn exact_matrix_any(&self) -> Option<&dyn Any> {
        match self {
            LegacyValue::MatrixIndex(matrix) => Some(matrix),
            #[cfg(feature = "bool")]
            LegacyValue::MatrixBool(matrix) => Some(matrix),
            #[cfg(feature = "u8")]
            LegacyValue::MatrixU8(matrix) => Some(matrix),
            #[cfg(feature = "u16")]
            LegacyValue::MatrixU16(matrix) => Some(matrix),
            #[cfg(feature = "u32")]
            LegacyValue::MatrixU32(matrix) => Some(matrix),
            #[cfg(feature = "u64")]
            LegacyValue::MatrixU64(matrix) => Some(matrix),
            #[cfg(feature = "u128")]
            LegacyValue::MatrixU128(matrix) => Some(matrix),
            #[cfg(feature = "i8")]
            LegacyValue::MatrixI8(matrix) => Some(matrix),
            #[cfg(feature = "i16")]
            LegacyValue::MatrixI16(matrix) => Some(matrix),
            #[cfg(feature = "i32")]
            LegacyValue::MatrixI32(matrix) => Some(matrix),
            #[cfg(feature = "i64")]
            LegacyValue::MatrixI64(matrix) => Some(matrix),
            #[cfg(feature = "i128")]
            LegacyValue::MatrixI128(matrix) => Some(matrix),
            #[cfg(feature = "f32")]
            LegacyValue::MatrixF32(matrix) => Some(matrix),
            #[cfg(feature = "f64")]
            LegacyValue::MatrixF64(matrix) => Some(matrix),
            #[cfg(feature = "string")]
            LegacyValue::MatrixString(matrix) => Some(matrix),
            #[cfg(feature = "rational")]
            LegacyValue::MatrixR64(matrix) => Some(matrix),
            #[cfg(feature = "complex")]
            LegacyValue::MatrixC64(matrix) => Some(matrix),
            LegacyValue::MatrixValue(matrix) => Some(matrix),
            _ => None,
        }
    }

    #[cfg(all(feature = "matrix", feature = "functions"))]
    pub fn try_function_matrix<T: Clone + 'static>(
        &self,
        role: FunctionArgumentRole,
    ) -> MResult<Matrix<T>> {
        self.exact_matrix_any()
            .and_then(|matrix| matrix.downcast_ref::<Matrix<T>>())
            .cloned()
            .ok_or_else(|| {
                MechError::new(
                    FunctionArgumentTypeMismatch {
                        role,
                        expected: core::any::type_name::<Matrix<T>>().to_string(),
                        found: self.exact_runtime_representation_name(),
                    },
                    None,
                )
                .with_compiler_loc()
            })
    }

    #[cfg(all(feature = "matrix", feature = "functions"))]
    pub fn try_function_copyable_matrix<T: 'static>(
        &self,
        role: FunctionArgumentRole,
    ) -> MResult<Box<dyn CopyMat<T>>>
    where
        T: Clone + AsValueKind,
        #[cfg(feature = "semantic-compiler")]
        T: CompileConst + ConstElem + Debug + PartialEq,
    {
        self.exact_matrix_any()
            .and_then(|matrix| matrix.downcast_ref::<Matrix<T>>())
            .map(Matrix::get_copyable_matrix)
            .ok_or_else(|| {
                MechError::new(
                    FunctionArgumentTypeMismatch {
                        role,
                        expected: core::any::type_name::<Matrix<T>>().to_string(),
                        found: self.exact_runtime_representation_name(),
                    },
                    None,
                )
                .with_compiler_loc()
            })
    }

    #[cfg(feature = "functions")]
    pub fn try_function_ref<T: 'static>(&self, role: FunctionArgumentRole) -> MResult<Ref<T>> {
        require_function_ref(self, role)
    }

    pub fn exact_runtime_representation_name(&self) -> String {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => core::any::type_name::<Ref<u8>>().to_string(),
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => core::any::type_name::<Ref<u16>>().to_string(),
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => core::any::type_name::<Ref<u32>>().to_string(),
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => core::any::type_name::<Ref<u64>>().to_string(),
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => core::any::type_name::<Ref<u128>>().to_string(),
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => core::any::type_name::<Ref<i8>>().to_string(),
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => core::any::type_name::<Ref<i16>>().to_string(),
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => core::any::type_name::<Ref<i32>>().to_string(),
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => core::any::type_name::<Ref<i64>>().to_string(),
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => core::any::type_name::<Ref<i128>>().to_string(),
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => core::any::type_name::<Ref<f32>>().to_string(),
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => core::any::type_name::<Ref<f64>>().to_string(),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(_) => core::any::type_name::<Ref<String>>().to_string(),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => core::any::type_name::<Ref<bool>>().to_string(),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(_) => core::any::type_name::<Ref<MechAtom>>().to_string(),
            #[cfg(feature = "complex")]
            LegacyValue::C64(_) => core::any::type_name::<Ref<C64>>().to_string(),
            #[cfg(feature = "rational")]
            LegacyValue::R64(_) => core::any::type_name::<Ref<R64>>().to_string(),
            #[cfg(feature = "set")]
            LegacyValue::Set(_) => core::any::type_name::<Ref<MechSet>>().to_string(),
            #[cfg(feature = "map")]
            LegacyValue::Map(_) => core::any::type_name::<Ref<MechMap>>().to_string(),
            #[cfg(feature = "record")]
            LegacyValue::Record(_) => core::any::type_name::<Ref<MechRecord>>().to_string(),
            #[cfg(feature = "table")]
            LegacyValue::Table(_) => core::any::type_name::<Ref<MechTable>>().to_string(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(_) => core::any::type_name::<Ref<MechTuple>>().to_string(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(_) => core::any::type_name::<Ref<MechEnum>>().to_string(),
            LegacyValue::Index(_) => core::any::type_name::<Ref<usize>>().to_string(),
            LegacyValue::MutableReference(_) => {
                core::any::type_name::<Ref<LegacyValue>>().to_string()
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(matrix) => matrix.exact_runtime_representation_name(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(matrix) => matrix.exact_runtime_representation_name(),
            LegacyValue::Id(_) => "u64 (direct ID value)".to_string(),
            LegacyValue::Typed(_, _) => "Typed".to_string(),
            LegacyValue::Kind(_) => "Kind".to_string(),
            LegacyValue::IndexAll => "IndexAll".to_string(),
            LegacyValue::EmptyKind(_) => "EmptyKind".to_string(),
            LegacyValue::Empty => "Empty".to_string(),
        }
    }

    pub fn addr(&self) -> usize {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => v.addr(),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => v.addr(),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => v.addr(),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => v.addr(),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => v.addr(),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => v.addr(),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => v.addr(),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => v.addr(),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => v.addr(),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => v.addr(),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => v.addr(),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => v.addr(),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(v) => v.addr(),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(v) => v.addr(),
            #[cfg(feature = "complex")]
            LegacyValue::C64(v) => v.addr(),
            #[cfg(feature = "rational")]
            LegacyValue::R64(v) => v.addr(),
            #[cfg(feature = "record")]
            LegacyValue::Record(v) => v.addr(),
            #[cfg(feature = "table")]
            LegacyValue::Table(v) => v.addr(),
            #[cfg(feature = "map")]
            LegacyValue::Map(v) => v.addr(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(v) => v.addr(),
            #[cfg(feature = "set")]
            LegacyValue::Set(v) => v.addr(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(v) => v.addr(),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(v) => v.addr(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(v) => v.addr(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(v) => v.addr(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(v) => v.addr(),
            LegacyValue::Index(v) => v.addr(),
            LegacyValue::MutableReference(v) => v.addr(),
            _ => todo!(),
        }
    }

    pub fn convert_to(&self, other: &ValueKind) -> Option<LegacyValue> {
        if self.kind() == *other {
            return Some(self.clone());
        }

        if !self.kind().is_convertible_to(other) {
            return None;
        }

        match (self, other) {
            (LegacyValue::Typed(value, _), target_kind) => value.convert_to(target_kind),
            (LegacyValue::Empty, ValueKind::Option(_)) => Some(LegacyValue::Empty),
            (LegacyValue::EmptyKind(_), ValueKind::Option(_)) => Some(LegacyValue::Empty),
            (value, ValueKind::Option(inner)) => value.convert_to(inner.as_ref()),
            (value, ValueKind::Matrix(_, target_shape))
                if target_shape.is_empty() && matches!(value.kind(), ValueKind::Matrix(_, _)) =>
            {
                Some(value.clone())
            }
            // ==== Unsigned widening and narrowing ====
            #[cfg(all(feature = "u8", feature = "u16"))]
            (LegacyValue::U8(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "u8", feature = "u32"))]
            (LegacyValue::U8(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "u8", feature = "u64"))]
            (LegacyValue::U8(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "u8", feature = "u128"))]
            (LegacyValue::U8(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "u8", feature = "i16"))]
            (LegacyValue::U8(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "u8", feature = "i32"))]
            (LegacyValue::U8(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "u8", feature = "i64"))]
            (LegacyValue::U8(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "u8", feature = "i128"))]
            (LegacyValue::U8(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "u8", feature = "f32"))]
            (LegacyValue::U8(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "u8", feature = "f64"))]
            (LegacyValue::U8(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "u16", feature = "u8"))]
            (LegacyValue::U16(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "u16", feature = "u32"))]
            (LegacyValue::U16(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "u16", feature = "u64"))]
            (LegacyValue::U16(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "u16", feature = "u128"))]
            (LegacyValue::U16(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "u16", feature = "i8"))]
            (LegacyValue::U16(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "u16", feature = "i32"))]
            (LegacyValue::U16(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "u16", feature = "i64"))]
            (LegacyValue::U16(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "u16", feature = "i128"))]
            (LegacyValue::U16(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "u16", feature = "f32"))]
            (LegacyValue::U16(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "u16", feature = "f64"))]
            (LegacyValue::U16(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "u32", feature = "u8"))]
            (LegacyValue::U32(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "u32", feature = "u16"))]
            (LegacyValue::U32(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "u32", feature = "u64"))]
            (LegacyValue::U32(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "u32", feature = "u128"))]
            (LegacyValue::U32(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "u32", feature = "i8"))]
            (LegacyValue::U32(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "u32", feature = "i16"))]
            (LegacyValue::U32(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "u32", feature = "i64"))]
            (LegacyValue::U32(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "u32", feature = "i128"))]
            (LegacyValue::U32(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "u32", feature = "f32"))]
            (LegacyValue::U32(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "u32", feature = "f64"))]
            (LegacyValue::U32(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "u64", feature = "u8"))]
            (LegacyValue::U64(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "u64", feature = "u16"))]
            (LegacyValue::U64(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "u64", feature = "u32"))]
            (LegacyValue::U64(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "u64", feature = "u128"))]
            (LegacyValue::U64(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "u64", feature = "i8"))]
            (LegacyValue::U64(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "u64", feature = "i16"))]
            (LegacyValue::U64(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "u64", feature = "i32"))]
            (LegacyValue::U64(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "u64", feature = "i128"))]
            (LegacyValue::U64(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "u64", feature = "f32"))]
            (LegacyValue::U64(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "u64", feature = "f64"))]
            (LegacyValue::U64(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "u128", feature = "u8"))]
            (LegacyValue::U128(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "u128", feature = "u16"))]
            (LegacyValue::U128(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "u128", feature = "u32"))]
            (LegacyValue::U128(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "u128", feature = "u64"))]
            (LegacyValue::U128(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "u128", feature = "i8"))]
            (LegacyValue::U128(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "u128", feature = "i16"))]
            (LegacyValue::U128(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "u128", feature = "i32"))]
            (LegacyValue::U128(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "u128", feature = "i64"))]
            (LegacyValue::U128(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "u128", feature = "f32"))]
            (LegacyValue::U128(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "u128", feature = "f64"))]
            (LegacyValue::U128(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            // ==== Signed widening and narrowing ====
            #[cfg(all(feature = "i8", feature = "i16"))]
            (LegacyValue::I8(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "i8", feature = "i32"))]
            (LegacyValue::I8(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "i8", feature = "i64"))]
            (LegacyValue::I8(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "i8", feature = "i128"))]
            (LegacyValue::I8(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "i8", feature = "u16"))]
            (LegacyValue::I8(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "i8", feature = "u32"))]
            (LegacyValue::I8(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "i8", feature = "u64"))]
            (LegacyValue::I8(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "i8", feature = "u128"))]
            (LegacyValue::I8(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "i8", feature = "f32"))]
            (LegacyValue::I8(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "i8", feature = "f64"))]
            (LegacyValue::I8(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "i16", feature = "i8"))]
            (LegacyValue::I16(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "i16", feature = "i32"))]
            (LegacyValue::I16(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "i16", feature = "i64"))]
            (LegacyValue::I16(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "i16", feature = "i128"))]
            (LegacyValue::I16(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "i16", feature = "u8"))]
            (LegacyValue::I16(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "i16", feature = "u32"))]
            (LegacyValue::I16(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "i16", feature = "u64"))]
            (LegacyValue::I16(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "i16", feature = "u128"))]
            (LegacyValue::I16(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "i16", feature = "f32"))]
            (LegacyValue::I16(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "i16", feature = "f64"))]
            (LegacyValue::I16(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "i32", feature = "i8"))]
            (LegacyValue::I32(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "i32", feature = "i16"))]
            (LegacyValue::I32(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "i32", feature = "i64"))]
            (LegacyValue::I32(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "i32", feature = "i128"))]
            (LegacyValue::I32(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "i32", feature = "u8"))]
            (LegacyValue::I32(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "i32", feature = "u16"))]
            (LegacyValue::I32(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "i32", feature = "u64"))]
            (LegacyValue::I32(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "i32", feature = "u128"))]
            (LegacyValue::I32(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "i32", feature = "f32"))]
            (LegacyValue::I32(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "i32", feature = "f64"))]
            (LegacyValue::I32(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "i64", feature = "i8"))]
            (LegacyValue::I64(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "i64", feature = "i16"))]
            (LegacyValue::I64(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "i64", feature = "i32"))]
            (LegacyValue::I64(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "i64", feature = "i128"))]
            (LegacyValue::I64(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new((*v.borrow()) as i128)))
            }
            #[cfg(all(feature = "i64", feature = "u8"))]
            (LegacyValue::I64(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "i64", feature = "u16"))]
            (LegacyValue::I64(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "i64", feature = "u32"))]
            (LegacyValue::I64(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "i64", feature = "u128"))]
            (LegacyValue::I64(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new((*v.borrow()) as u128)))
            }
            #[cfg(all(feature = "i64", feature = "f32"))]
            (LegacyValue::I64(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "i64", feature = "f64"))]
            (LegacyValue::I64(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            #[cfg(all(feature = "i128", feature = "i8"))]
            (LegacyValue::I128(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new((*v.borrow()) as i8)))
            }
            #[cfg(all(feature = "i128", feature = "i16"))]
            (LegacyValue::I128(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new((*v.borrow()) as i16)))
            }
            #[cfg(all(feature = "i128", feature = "i32"))]
            (LegacyValue::I128(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new((*v.borrow()) as i32)))
            }
            #[cfg(all(feature = "i128", feature = "i64"))]
            (LegacyValue::I128(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new((*v.borrow()) as i64)))
            }
            #[cfg(all(feature = "i128", feature = "u8"))]
            (LegacyValue::I128(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new((*v.borrow()) as u8)))
            }
            #[cfg(all(feature = "i128", feature = "u16"))]
            (LegacyValue::I128(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new((*v.borrow()) as u16)))
            }
            #[cfg(all(feature = "i128", feature = "u32"))]
            (LegacyValue::I128(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new((*v.borrow()) as u32)))
            }
            #[cfg(all(feature = "i128", feature = "u64"))]
            (LegacyValue::I128(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new((*v.borrow()) as u64)))
            }
            #[cfg(all(feature = "i128", feature = "f32"))]
            (LegacyValue::I128(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }
            #[cfg(all(feature = "i128", feature = "f64"))]
            (LegacyValue::I128(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }

            // ==== Float widening and narrowing ====
            #[cfg(all(feature = "f32", feature = "f64"))]
            (LegacyValue::F32(v), ValueKind::F64) => {
                Some(LegacyValue::F64(Ref::new((*v.borrow()) as f64)))
            }
            #[cfg(all(feature = "f32", feature = "f64"))]
            (LegacyValue::F64(v), ValueKind::F32) => {
                Some(LegacyValue::F32(Ref::new((*v.borrow()) as f32)))
            }

            // ==== Float to integer conversions (truncate) ====
            #[cfg(all(feature = "f32", feature = "i8"))]
            (LegacyValue::F32(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new(*v.borrow() as i8)))
            }
            #[cfg(all(feature = "f32", feature = "i16"))]
            (LegacyValue::F32(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new(*v.borrow() as i16)))
            }
            #[cfg(all(feature = "f32", feature = "i32"))]
            (LegacyValue::F32(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new(*v.borrow() as i32)))
            }
            #[cfg(all(feature = "f32", feature = "i64"))]
            (LegacyValue::F32(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new(*v.borrow() as i64)))
            }
            #[cfg(all(feature = "f32", feature = "i128"))]
            (LegacyValue::F32(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new(*v.borrow() as i128)))
            }
            #[cfg(all(feature = "f32", feature = "u8"))]
            (LegacyValue::F32(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new(*v.borrow() as u8)))
            }
            #[cfg(all(feature = "f32", feature = "u16"))]
            (LegacyValue::F32(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new(*v.borrow() as u16)))
            }
            #[cfg(all(feature = "f32", feature = "u32"))]
            (LegacyValue::F32(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new(*v.borrow() as u32)))
            }
            #[cfg(all(feature = "f32", feature = "u64"))]
            (LegacyValue::F32(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new(*v.borrow() as u64)))
            }
            #[cfg(all(feature = "f32", feature = "u128"))]
            (LegacyValue::F32(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new(*v.borrow() as u128)))
            }
            #[cfg(all(feature = "f64", feature = "i8"))]
            (LegacyValue::F64(v), ValueKind::I8) => {
                Some(LegacyValue::I8(Ref::new(*v.borrow() as i8)))
            }
            #[cfg(all(feature = "f64", feature = "i16"))]
            (LegacyValue::F64(v), ValueKind::I16) => {
                Some(LegacyValue::I16(Ref::new(*v.borrow() as i16)))
            }
            #[cfg(all(feature = "f64", feature = "i32"))]
            (LegacyValue::F64(v), ValueKind::I32) => {
                Some(LegacyValue::I32(Ref::new(*v.borrow() as i32)))
            }
            #[cfg(all(feature = "f64", feature = "i64"))]
            (LegacyValue::F64(v), ValueKind::I64) => {
                Some(LegacyValue::I64(Ref::new(*v.borrow() as i64)))
            }
            #[cfg(all(feature = "f64", feature = "i128"))]
            (LegacyValue::F64(v), ValueKind::I128) => {
                Some(LegacyValue::I128(Ref::new(*v.borrow() as i128)))
            }
            #[cfg(all(feature = "f64", feature = "u8"))]
            (LegacyValue::F64(v), ValueKind::U8) => {
                Some(LegacyValue::U8(Ref::new(*v.borrow() as u8)))
            }
            #[cfg(all(feature = "f64", feature = "u16"))]
            (LegacyValue::F64(v), ValueKind::U16) => {
                Some(LegacyValue::U16(Ref::new(*v.borrow() as u16)))
            }
            #[cfg(all(feature = "f64", feature = "u32"))]
            (LegacyValue::F64(v), ValueKind::U32) => {
                Some(LegacyValue::U32(Ref::new(*v.borrow() as u32)))
            }
            #[cfg(all(feature = "f64", feature = "u64"))]
            (LegacyValue::F64(v), ValueKind::U64) => {
                Some(LegacyValue::U64(Ref::new(*v.borrow() as u64)))
            }
            #[cfg(all(feature = "f64", feature = "u128"))]
            (LegacyValue::F64(v), ValueKind::U128) => {
                Some(LegacyValue::U128(Ref::new(*v.borrow() as u128)))
            }

            /*
            // ==== INDEX conversions ====
            (LegacyValue::Index(i), U32) => Some(LegacyValue::U32(Ref::new((*i.borrow()) as u32))),
            (LegacyValue::U32(v), Index) => Some(LegacyValue::Index(Ref::new((*v.borrow()) as usize))),


            // ==== MATRIX conversions (element-wise) ====
            (LegacyValue::MatrixU8(m), MatrixU16) => Some(LegacyValue::MatrixU16(m.map(|x| *x as u16))),
            (LegacyValue::MatrixI32(m), MatrixF64) => Some(LegacyValue::MatrixF64(m.map(|x| (*x) as f64))),
            // You can expand other matrix conversions similarly...

            // ==== COMPLEX TYPES (stubs) ====
            (LegacyValue::Set(set), Set(_)) => Some(LegacyValue::Set(set.clone())), // TODO: element-wise convert
            (LegacyValue::Map(map), Map(_)) => Some(LegacyValue::Map(map.clone())), // TODO: key/value convert
            (LegacyValue::Record(r), Record(_)) => Some(LegacyValue::Record(r.clone())), // TODO: field convert
            (LegacyValue::Table(t), Table(_)) => Some(LegacyValue::Table(t.clone())), // TODO: column convert

            // ==== ENUM, KIND ====
            (LegacyValue::Enum(e), Enum(_)) => Some(LegacyValue::Enum(e.clone())),
            (LegacyValue::Kind(k), Kind(_)) => Some(LegacyValue::Kind(k.clone())),

            // ==== SPECIAL CASES ====
            (LegacyValue::IndexAll, IndexAll) => Some(LegacyValue::IndexAll),
            (LegacyValue::Empty, Empty) => Some(LegacyValue::Empty),
            */
            // ==== FALLBACK ====
            _ => None,
        }
    }

    pub fn size_of(&self) -> usize {
        match self {
            #[cfg(feature = "rational")]
            LegacyValue::R64(_) => 16,
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => 1,
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => 2,
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => 4,
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => 8,
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => 16,
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => 1,
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => 2,
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => 4,
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => 8,
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => 16,
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => 4,
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => 8,
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => 1,
            #[cfg(feature = "complex")]
            LegacyValue::C64(_) => 16,
            #[cfg(all(feature = "matrix"))]
            LegacyValue::MatrixIndex(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(x) => x.size_of(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(x) => x.size_of(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(x) => x.size_of(),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(x) => x.borrow().len(),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(_) => 8,
            #[cfg(feature = "set")]
            LegacyValue::Set(x) => x.borrow().size_of(),
            #[cfg(feature = "map")]
            LegacyValue::Map(x) => x.borrow().size_of(),
            #[cfg(feature = "table")]
            LegacyValue::Table(x) => x.borrow().size_of(),
            #[cfg(feature = "record")]
            LegacyValue::Record(x) => x.borrow().size_of(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(x) => x.borrow().size_of(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(x) => x.borrow().size_of(),
            LegacyValue::MutableReference(x) => x.borrow().size_of(),
            LegacyValue::Id(_) => 8,
            LegacyValue::Index(_) => 8,
            LegacyValue::Kind(_) => 0, // Kind is not a value, so it has no size
            LegacyValue::Typed(value, _) => value.size_of(),
            LegacyValue::EmptyKind(_) => 0,
            LegacyValue::Empty => 0,
            LegacyValue::IndexAll => 0, // IndexAll is a special value, so it has no size
        }
    }

    #[cfg(feature = "pretty_print")]
    pub fn to_html(&self) -> String {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "u16")]
            LegacyValue::U16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "u32")]
            LegacyValue::U32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "u64")]
            LegacyValue::U64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "u128")]
            LegacyValue::U128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "i8")]
            LegacyValue::I8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "i16")]
            LegacyValue::I16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "i32")]
            LegacyValue::I32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "i64")]
            LegacyValue::I64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "i128")]
            LegacyValue::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "f32")]
            LegacyValue::F32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(feature = "f64")]
            LegacyValue::F64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(s) => format!(
                "<span class='mech-string'>\"{}\"</span>",
                escape_html_text(&s.borrow())
            ),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(b) => format!("<span class='mech-boolean'>{}</span>", b.borrow()),
            #[cfg(feature = "complex")]
            LegacyValue::C64(c) => c.borrow().to_html(),
            #[cfg(feature = "rational")]
            LegacyValue::R64(r) => r.borrow().to_html(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(m) => m.to_html(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(m) => m.to_html(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(m) => m.to_html(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(m) => m.to_html(),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(a) => a.borrow().to_html(),
            #[cfg(feature = "set")]
            LegacyValue::Set(s) => s.borrow().to_html(),
            #[cfg(feature = "map")]
            LegacyValue::Map(m) => m.borrow().to_html(),
            #[cfg(feature = "table")]
            LegacyValue::Table(t) => t.borrow().to_html(),
            #[cfg(feature = "record")]
            LegacyValue::Record(r) => r.borrow().to_html(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(t) => t.borrow().to_html(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(e) => e.borrow().to_html(),
            LegacyValue::Empty | LegacyValue::EmptyKind(_) => {
                "<span class='mech-empty'>_</span>".to_string()
            }
            LegacyValue::MutableReference(m) => {
                let inner = m.borrow();
                format!("<span class='mech-reference'>{}</span>", inner.to_html())
            }
            LegacyValue::Typed(value, _) => value.to_html(),
            _ => "???".to_string(),
        }
    }

    fn format_kind_with_budget(&self, budget: &mut InlineFormatBudget) -> String {
        match self {
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(matrix) => format!(
                "[*]:{}",
                matrix
                    .shape()
                    .iter()
                    .map(|dimension| dimension.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            #[cfg(feature = "table")]
            LegacyValue::Table(table) => {
                let table = table.borrow();
                let mut columns = Vec::new();
                for (key, (kind, _)) in &table.data {
                    if !budget.consume() {
                        columns.push("…<*>".to_string());
                        break;
                    }
                    let name = table
                        .col_names
                        .get(key)
                        .cloned()
                        .unwrap_or_else(|| key.to_string());
                    columns.push(format!("{}<{}>", name, kind.format_with_budget(budget)));
                }
                let size = (table.rows > 0)
                    .then(|| format!(":{}", table.rows))
                    .unwrap_or_default();
                format!("|{}|{size}", columns.join(" "))
            }
            #[cfg(feature = "record")]
            LegacyValue::Record(record) => {
                let record = record.borrow();
                let mut fields = Vec::new();
                for (key, value) in &record.data {
                    if !budget.consume() {
                        fields.push("…<*>".to_string());
                        break;
                    }
                    let name = record
                        .field_names
                        .get(key)
                        .cloned()
                        .unwrap_or_else(|| key.to_string());
                    fields.push(format!(
                        "{}<{}>",
                        name,
                        value.format_kind_with_budget(budget)
                    ));
                }
                format!("{{{}}}", fields.join(" "))
            }
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(tuple) => {
                let tuple = tuple.borrow();
                let mut elements = Vec::new();
                for value in &tuple.elements {
                    if !budget.consume() {
                        elements.push("…".to_string());
                        break;
                    }
                    elements.push(value.format_kind_with_budget(budget));
                }
                format!("({})", elements.join(","))
            }
            #[cfg(feature = "set")]
            LegacyValue::Set(set) => {
                let set = set.borrow();
                format!(
                    "{{{}}}{}",
                    set.kind.format_with_budget(budget),
                    set.max_elements
                        .map_or(String::new(), |size| format!(":{size}")),
                )
            }
            #[cfg(feature = "map")]
            LegacyValue::Map(map) => {
                let map = map.borrow();
                format!(
                    "{{{}:{}}}",
                    map.key_kind.format_with_budget(budget),
                    map.value_kind.format_with_budget(budget),
                )
            }
            #[cfg(feature = "enum")]
            LegacyValue::Enum(enm) => {
                let enm = enm.borrow();
                let names = enm.names.borrow();
                if let [(variant_id, payload)] = enm.variants.as_slice() {
                    let variant_name = names
                        .get(variant_id)
                        .map(|name| name.rsplit('/').next().unwrap_or(name).to_string())
                        .unwrap_or_else(|| variant_id.to_string());
                    if let Some(value) = payload
                        && !matches!(value, LegacyValue::Kind(_))
                    {
                        return format!(
                            ":{}({})",
                            variant_name,
                            value.format_kind_with_budget(budget),
                        );
                    }
                    return format!(":{variant_name}");
                }
                let name = names
                    .get(&enm.id)
                    .cloned()
                    .unwrap_or_else(|| enm.id.to_string());
                format!(":{name}")
            }
            LegacyValue::MutableReference(reference) => {
                reference.borrow().format_kind_with_budget(budget)
            }
            LegacyValue::Typed(_, kind) => kind.format_with_budget(budget),
            LegacyValue::EmptyKind(kind) | LegacyValue::Kind(kind) => {
                kind.format_with_budget(budget)
            }
            value => value.kind().format_with_budget(budget),
        }
    }

    /// Formats the kind shown beside a browser or terminal value without
    /// reconstructing an unbounded aggregate schema from the retained value.
    pub fn format_kind_with_element_limit(&self, limit: usize) -> String {
        self.format_kind_with_budget(&mut InlineFormatBudget::bounded(limit))
    }

    /// Render rich HTML without allowing an aggregate projection to traverse
    /// more elements than the portable REPL budget. Small values retain their
    /// structured renderer; elided values use their bounded canonical form so
    /// nested containers and schema-only tables cannot bypass the limit.
    #[cfg(feature = "pretty_print")]
    pub fn to_html_with_element_limit(&self, limit: usize) -> String {
        let mut budget = InlineFormatBudget::bounded(limit);
        let canonical = self.format_canonical_inline_with_budget(&mut budget);
        if budget.elided {
            format!(
                "<pre class=\"mech-value-preview mech-value-elided\">{}</pre>",
                escape_html_text(&canonical),
            )
        } else {
            self.to_html()
        }
    }

    /// Formats this value as a single-line, language-valid Mech value.
    ///
    /// String escaping and recursive container formatting belong here so
    /// every host publishes the same canonical representation.
    pub fn format_canonical_inline(&self) -> String {
        self.format_canonical_inline_with_budget(&mut InlineFormatBudget::unlimited())
    }

    fn format_canonical_inline_with_budget(&self, budget: &mut InlineFormatBudget) -> String {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(n) => format!("{}", n.borrow()),
            #[cfg(feature = "u16")]
            LegacyValue::U16(n) => format!("{}", n.borrow()),
            #[cfg(feature = "u32")]
            LegacyValue::U32(n) => format!("{}", n.borrow()),
            #[cfg(feature = "u64")]
            LegacyValue::U64(n) => format!("{}", n.borrow()),
            #[cfg(feature = "u128")]
            LegacyValue::U128(n) => format!("{}", n.borrow()),
            #[cfg(feature = "i8")]
            LegacyValue::I8(n) => format!("{}", n.borrow()),
            #[cfg(feature = "i16")]
            LegacyValue::I16(n) => format!("{}", n.borrow()),
            #[cfg(feature = "i32")]
            LegacyValue::I32(n) => format!("{}", n.borrow()),
            #[cfg(feature = "i64")]
            LegacyValue::I64(n) => format!("{}", n.borrow()),
            #[cfg(feature = "i128")]
            LegacyValue::I128(n) => format!("{}", n.borrow()),
            #[cfg(feature = "f32")]
            LegacyValue::F32(n) => format!("{}", n.borrow()),
            #[cfg(feature = "f64")]
            LegacyValue::F64(n) => format!("{}", n.borrow()),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(s) => Self::format_string_inline(s.borrow().as_str()),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(b) => format!("{}", b.borrow()),
            #[cfg(feature = "complex")]
            LegacyValue::C64(c) => format!("{}", c.borrow()),
            #[cfg(feature = "rational")]
            LegacyValue::R64(r) => format!("{}", r.borrow()),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(m) => {
                Self::format_matrix_inline_with(m, budget, |value, _| {
                    Self::format_string_inline(&value)
                })
            }
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(m) => Self::format_matrix_inline(m, budget),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(m) => Self::format_matrix_inline(m, budget),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(m) => {
                Self::format_matrix_inline_with(m, budget, |value, budget| {
                    value.format_canonical_inline_with_budget(budget)
                })
            }
            #[cfg(feature = "atom")]
            LegacyValue::Atom(a) => format!("{}", a.borrow()),
            #[cfg(feature = "set")]
            LegacyValue::Set(s) => {
                let set = s.borrow();
                let mut vals = Vec::new();
                let mut elided = false;
                for value in &set.set {
                    if !budget.consume() {
                        elided = true;
                        break;
                    }
                    vals.push(value.format_canonical_inline_with_budget(budget));
                }
                if elided {
                    vals.push("…".to_string());
                }
                format!("{{{}}}", vals.join(", "))
            }
            #[cfg(feature = "map")]
            LegacyValue::Map(m) => {
                let map = m.borrow();
                let mut vals = Vec::new();
                let mut elided = false;
                for (key, value) in &map.map {
                    if !budget.consume() {
                        elided = true;
                        break;
                    }
                    vals.push(format!(
                        "{}: {}",
                        key.format_canonical_inline_with_budget(budget),
                        value.format_canonical_inline_with_budget(budget)
                    ));
                }
                if elided {
                    vals.push("…".to_string());
                }
                format!("{{{}}}", vals.join(", "))
            }
            #[cfg(feature = "record")]
            LegacyValue::Record(r) => {
                let record = r.borrow();
                let mut vals = Vec::new();
                let mut elided = false;
                for (key, value) in &record.data {
                    if !budget.consume() {
                        elided = true;
                        break;
                    }
                    let name = record
                        .field_names
                        .get(key)
                        .cloned()
                        .unwrap_or_else(|| format!("{}", key));
                    vals.push(format!(
                        "{}: {}",
                        name,
                        value.format_canonical_inline_with_budget(budget)
                    ));
                }
                if elided {
                    vals.push("…".to_string());
                }
                format!("{{{}}}", vals.join(", "))
            }
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(t) => {
                let tuple = t.borrow();
                let mut vals = Vec::new();
                let mut elided = false;
                for value in &tuple.elements {
                    if !budget.consume() {
                        elided = true;
                        break;
                    }
                    vals.push(value.format_canonical_inline_with_budget(budget));
                }
                if elided {
                    vals.push("…".to_string());
                }
                format!("({})", vals.join(", "))
            }
            #[cfg(feature = "enum")]
            LegacyValue::Enum(e) => {
                let enm = e.borrow();
                // A multi-variant MechEnum is the retained nominal definition,
                // not one active variant. Project that descriptor through the
                // language's kind-literal syntax instead of inventing an enum
                // value grammar that the parser does not support.
                if enm.variants.len() != 1 {
                    return format!("<:{}>", enm.name());
                }
                let dict = enm.names.borrow();
                let (variant_id, payload) = &enm.variants[0];
                let variant_name = dict
                    .get(variant_id)
                    .map(|name| name.rsplit('/').next().unwrap_or(name).to_string())
                    .unwrap_or_else(|| format!("{}", variant_id));
                match payload {
                    Some(value) => {
                        let payload = if budget.consume() {
                            value.format_canonical_inline_with_budget(budget)
                        } else {
                            "…".to_string()
                        };
                        format!(":{}({})", variant_name, payload)
                    }
                    None => format!(":{}", variant_name),
                }
            }
            #[cfg(feature = "table")]
            LegacyValue::Table(t) => {
                let table = t.borrow();
                // Mech has no zero-row table value literal. Preserve its
                // schema as a valid kind literal instead of emitting a header
                // that the table grammar cannot terminate.
                if table.data.is_empty() {
                    return "<*>".to_string();
                }
                if table.rows == 0 {
                    let mut columns = Vec::new();
                    let mut elided = false;
                    for (key, (kind, _)) in &table.data {
                        if !budget.consume() {
                            elided = true;
                            break;
                        }
                        let name = table
                            .col_names
                            .get(key)
                            .cloned()
                            .unwrap_or_else(|| format!("{}", key));
                        columns.push(format!("{}<{}>", name, kind));
                    }
                    if elided {
                        columns.push("…<*>".to_string());
                    }
                    return format!("<|{}|>", columns.join(" "));
                }
                let mut visible_columns = Vec::new();
                let mut columns_elided = false;
                for column in &table.data {
                    if !budget.consume() {
                        columns_elided = true;
                        break;
                    }
                    visible_columns.push(column);
                }
                let mut headers = visible_columns
                    .iter()
                    .map(|(key, (kind, _))| {
                        let name = table
                            .col_names
                            .get(key)
                            .cloned()
                            .unwrap_or_else(|| format!("{}", key));
                        format!("{}<{}>", name, kind)
                    })
                    .collect::<Vec<_>>();
                if columns_elided {
                    headers.push("…<*>".to_string());
                }
                let headers = headers.join(" ");
                let mut rows = Vec::new();
                let mut values_elided = false;
                if visible_columns.is_empty() {
                    // A zero budget still needs one structural elision marker;
                    // emitting an empty row for every retained row would make
                    // the supposedly bounded projection scale with the table.
                    rows.push("…".to_string());
                } else {
                    'rows: for row in 0..table.rows {
                        let mut cells = Vec::new();
                        for (_, (_, column)) in &visible_columns {
                            if !budget.consume() {
                                values_elided = true;
                                if !cells.is_empty() {
                                    cells.push("…".to_string());
                                    rows.push(cells.join(" "));
                                }
                                break 'rows;
                            }
                            cells.push(
                                column
                                    .index2d(row + 1, 1)
                                    .format_canonical_table_cell_inline(budget),
                            );
                        }
                        rows.push(cells.join(" "));
                    }
                    if values_elided || rows.len() < table.rows {
                        rows.push("…".to_string());
                    }
                }
                format!("|{}| {} |", headers, rows.join(" | "))
            }
            LegacyValue::Id(x) => format!("{}", humanize(x)),
            LegacyValue::Index(x) => format!("{}", x.borrow()),
            LegacyValue::Kind(k) => format!("<{}>", k),
            LegacyValue::Typed(value, _) => value.format_canonical_inline_with_budget(budget),
            LegacyValue::MutableReference(m) => {
                m.borrow().format_canonical_inline_with_budget(budget)
            }
            LegacyValue::IndexAll => ":".to_string(),
            LegacyValue::EmptyKind(_) => "_".to_string(),
            LegacyValue::Empty => "_".to_string(),
        }
    }

    /// Formats a canonical REPL projection while bounding aggregate traversal.
    /// The accepted runtime value remains complete; only its interactive
    /// presentation is elided, including nested aggregates of every kind.
    pub fn format_canonical_inline_with_element_limit(&self, limit: usize) -> String {
        self.format_canonical_inline_with_budget(&mut InlineFormatBudget::bounded(limit))
    }

    /// Formats one cell for the inline-table grammar. A table literal uses
    /// the same bar token as its enclosing row terminator, so a directly
    /// nested table must be parenthesized to give both the parser and preview
    /// scanner an unambiguous structural boundary. Inspecting the canonical
    /// prefix also covers typed and referenced table values transparently.
    #[cfg(feature = "table")]
    fn format_canonical_table_cell_inline(&self, budget: &mut InlineFormatBudget) -> String {
        let canonical = self.format_canonical_inline_with_budget(budget);
        if canonical.starts_with('|') {
            format!("({canonical})")
        } else {
            canonical
        }
    }

    /// Formats a bounded preview without splitting encoded string characters
    /// or discarding delimiters that were opened before the elision point.
    pub fn format_preview_inline(&self, limit: usize) -> String {
        Self::preview_canonical_inline(
            &self.format_canonical_inline_with_element_limit(limit),
            limit,
        )
    }

    /// Compatibility alias for callers that require the complete canonical
    /// inline representation. New preview surfaces should use
    /// [`Self::format_preview_inline`] explicitly.
    pub fn format_value_inline(&self) -> String {
        self.format_canonical_inline()
    }

    fn preview_canonical_inline(canonical: &str, limit: usize) -> String {
        #[derive(Clone)]
        struct Checkpoint {
            bytes: usize,
            chars: usize,
            closers: Vec<char>,
        }

        let canonical_chars = canonical.chars().count();
        if canonical_chars <= limit {
            return canonical.to_string();
        }
        if limit == 0 {
            return String::new();
        }

        let mut prefix = String::new();
        let mut prefix_chars = 0;
        let mut closers = Vec::new();
        let mut table_closing_bytes = Vec::new();
        let mut in_string = false;
        let mut byte = 0;
        let mut checkpoint: Option<Checkpoint> = None;

        while byte < canonical.len() {
            let character = canonical[byte..]
                .chars()
                .next()
                .expect("byte offset remains on a character boundary");
            let mut next_closers = closers.clone();
            let mut next_table_closing_bytes = table_closing_bytes.clone();
            let mut next_in_string = in_string;
            let mut safe_cut = false;

            let end = if in_string {
                match character {
                    '"' => {
                        if next_closers.last() == Some(&'"') {
                            next_closers.pop();
                        }
                        next_in_string = false;
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '\\' => {
                        safe_cut = true;
                        Self::inline_escape_end(canonical, byte)
                    }
                    _ => {
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                }
            } else {
                match character {
                    '"' => {
                        next_closers.push('"');
                        next_in_string = true;
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '[' => {
                        next_closers.push(']');
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '{' => {
                        next_closers.push('}');
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '(' => {
                        next_closers.push(')');
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '<' => {
                        next_closers.push('>');
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    ']' | '}' | ')' | '>' => {
                        if next_closers.last() == Some(&character) {
                            next_closers.pop();
                        }
                        safe_cut = true;
                        byte + character.len_utf8()
                    }
                    '|' if next_closers.last() != Some(&'|') => {
                        let after = byte + character.len_utf8();
                        let surrounding_closer = next_closers.last().copied();
                        next_closers.push('|');
                        next_table_closing_bytes.push(Self::inline_table_closing_bar(
                            canonical,
                            after,
                            surrounding_closer,
                        ));
                        safe_cut = true;
                        after
                    }
                    '|' if next_closers.last() == Some(&'|') => {
                        let after = byte + character.len_utf8();
                        if next_table_closing_bytes.last().copied().flatten() == Some(byte) {
                            next_closers.pop();
                            next_table_closing_bytes.pop();
                        }
                        safe_cut = true;
                        after
                    }
                    character
                        if character.is_whitespace() || matches!(character, ',' | ';' | '|') =>
                    {
                        byte + character.len_utf8()
                    }
                    _ => {
                        let mut end = byte;
                        for (offset, character) in canonical[byte..].char_indices() {
                            if offset > 0
                                && (character.is_whitespace()
                                    || matches!(
                                        character,
                                        '"' | '['
                                            | ']'
                                            | '{'
                                            | '}'
                                            | '('
                                            | ')'
                                            | '<'
                                            | '>'
                                            | ','
                                            | ';'
                                            | '|'
                                    ))
                            {
                                break;
                            }
                            end = byte + offset + character.len_utf8();
                        }
                        safe_cut = !canonical[byte..end].ends_with(':');
                        end
                    }
                }
            };

            let token = &canonical[byte..end];
            let token_chars = token.chars().count();
            let next_chars = prefix_chars + token_chars;
            if next_chars + 1 + next_closers.len() > limit {
                break;
            }

            prefix.push_str(token);
            prefix_chars = next_chars;
            closers = next_closers;
            table_closing_bytes = next_table_closing_bytes;
            in_string = next_in_string;
            byte = end;

            if safe_cut {
                checkpoint = Some(Checkpoint {
                    bytes: prefix.len(),
                    chars: prefix_chars,
                    closers: closers.clone(),
                });
            }
        }

        let Some(checkpoint) = checkpoint else {
            return "…".chars().take(limit).collect();
        };

        let mut preview = prefix[..checkpoint.bytes].to_string();
        preview.push('…');
        for closer in checkpoint.closers.iter().rev() {
            preview.push(*closer);
        }
        debug_assert!(checkpoint.chars + 1 + checkpoint.closers.len() <= limit);
        preview
    }

    /// Locates the final bar before the active table's enclosing value ends.
    /// Bars inside strings or nested delimiters belong to nested values. This
    /// lookahead runs once per table opener, rather than once per table row.
    fn inline_table_closing_bar(
        value: &str,
        start: usize,
        surrounding_closer: Option<char>,
    ) -> Option<usize> {
        let mut closers = Vec::new();
        let mut in_string = false;
        let mut byte = start;
        let mut closing_bar = None;

        while byte < value.len() {
            let character = value[byte..]
                .chars()
                .next()
                .expect("byte offset remains on a character boundary");

            if in_string {
                match character {
                    '"' => in_string = false,
                    '\\' => {
                        byte = Self::inline_escape_end(value, byte);
                        continue;
                    }
                    _ => {}
                }
                byte += character.len_utf8();
                continue;
            }

            match character {
                '"' => in_string = true,
                '[' => closers.push(']'),
                '{' => closers.push('}'),
                '(' => closers.push(')'),
                '<' => closers.push('>'),
                ']' | '}' | ')' | '>' if closers.last() == Some(&character) => {
                    closers.pop();
                }
                character if closers.is_empty() && surrounding_closer == Some(character) => {
                    return closing_bar;
                }
                '|' if closers.is_empty() => closing_bar = Some(byte),
                ',' | ';' if closers.is_empty() => return closing_bar,
                _ => {}
            }
            byte += character.len_utf8();
        }

        closing_bar
    }

    fn inline_escape_end(value: &str, start: usize) -> usize {
        let mut characters = value[start..].char_indices();
        let Some((_, '\\')) = characters.next() else {
            return start;
        };
        let Some((offset, escaped)) = characters.next() else {
            return value.len();
        };
        let mut end = start + offset + escaped.len_utf8();

        if escaped == 'u' && value[end..].starts_with('{') {
            end += '{'.len_utf8();
            for character in value[end..].chars() {
                end += character.len_utf8();
                if character == '}' {
                    break;
                }
            }
        } else if escaped == 'x' {
            for character in value[end..].chars().take(2) {
                end += character.len_utf8();
            }
        }
        end
    }

    #[cfg(feature = "matrix")]
    fn format_matrix_inline<T>(matrix: &Matrix<T>, budget: &mut InlineFormatBudget) -> String
    where
        T: Clone + std::fmt::Display + std::fmt::Debug + PartialEq + 'static,
    {
        Self::format_matrix_inline_with(matrix, budget, |value, _| value.to_string())
    }

    #[cfg(feature = "matrix")]
    fn format_matrix_inline_with<T, F>(
        matrix: &Matrix<T>,
        budget: &mut InlineFormatBudget,
        mut format_element: F,
    ) -> String
    where
        T: Clone + std::fmt::Debug + PartialEq + 'static,
        F: FnMut(T, &mut InlineFormatBudget) -> String,
    {
        let shape = matrix.shape();
        let rows = shape[0];
        let cols = shape[1];
        let total = rows.saturating_mul(cols);
        let mut rendered = String::from("[");
        let mut visible = 0;
        for index in 0..total {
            if !budget.consume() {
                break;
            }
            let row = index / cols;
            let column = index % cols;
            if index > 0 {
                rendered.push_str(if column == 0 { "; " } else { " " });
            }
            rendered.push_str(&format_element(matrix.index2d(row + 1, column + 1), budget));
            visible += 1;
        }
        if visible < total {
            if visible > 0 {
                rendered.push(' ');
            }
            rendered.push('…');
        }
        rendered.push(']');
        rendered
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    fn format_string_inline(value: &str) -> String {
        let mut encoded = String::with_capacity(value.len() + 2);
        encoded.push('"');
        for character in value.chars() {
            match character {
                '\\' => encoded.push_str("\\\\"),
                '"' => encoded.push_str("\\\""),
                '\n' => encoded.push_str("\\n"),
                '\r' => encoded.push_str("\\r"),
                '\t' => encoded.push_str("\\t"),
                '\u{2028}' | '\u{2029}' => encoded.extend(character.escape_default()),
                character if character.is_control() => encoded.extend(character.escape_default()),
                character => encoded.push(character),
            }
        }
        encoded.push('"');
        encoded
    }

    pub fn shape(&self) -> Vec<usize> {
        match self {
            #[cfg(feature = "rational")]
            LegacyValue::R64(_) => vec![1, 1],
            #[cfg(feature = "complex")]
            LegacyValue::C64(_) => vec![1, 1],
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => vec![1, 1],
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => vec![1, 1],
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => vec![1, 1],
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => vec![1, 1],
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => vec![1, 1],
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => vec![1, 1],
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => vec![1, 1],
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => vec![1, 1],
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => vec![1, 1],
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => vec![1, 1],
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => vec![1, 1],
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => vec![1, 1],
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(_) => vec![1, 1],
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => vec![1, 1],
            #[cfg(feature = "atom")]
            LegacyValue::Atom(_) => vec![1, 1],
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(x) => x.shape(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(x) => x.shape(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(x) => x.shape(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(_) => vec![1, 1],
            #[cfg(feature = "table")]
            LegacyValue::Table(x) => x.borrow().shape(),
            #[cfg(feature = "set")]
            LegacyValue::Set(x) => vec![1, x.borrow().set.len()],
            #[cfg(feature = "map")]
            LegacyValue::Map(x) => vec![1, x.borrow().map.len()],
            #[cfg(feature = "record")]
            LegacyValue::Record(x) => x.borrow().shape(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(x) => vec![1, x.borrow().size()],
            LegacyValue::Index(_) => vec![1, 1],
            LegacyValue::MutableReference(x) => x.borrow().shape(),
            LegacyValue::Typed(x, _) => x.shape(),
            LegacyValue::Empty | LegacyValue::EmptyKind(_) => vec![1, 1],
            LegacyValue::IndexAll => vec![0, 0],
            LegacyValue::Kind(_) => vec![0, 0],
            LegacyValue::Id(_) => vec![0, 0],
        }
    }

    pub fn deref_kind(&self) -> ValueKind {
        match self {
            LegacyValue::MutableReference(x) => x.borrow().kind(),
            x => x.kind(),
        }
    }

    pub fn kind(&self) -> ValueKind {
        match self {
            #[cfg(feature = "complex")]
            LegacyValue::C64(_) => ValueKind::C64,
            #[cfg(feature = "rational")]
            LegacyValue::R64(_) => ValueKind::R64,
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => ValueKind::U8,
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => ValueKind::U16,
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => ValueKind::U32,
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => ValueKind::U64,
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => ValueKind::U128,
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => ValueKind::I8,
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => ValueKind::I16,
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => ValueKind::I32,
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => ValueKind::I64,
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => ValueKind::I128,
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => ValueKind::F32,
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => ValueKind::F64,
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(_) => ValueKind::String,
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => ValueKind::Bool,
            #[cfg(feature = "atom")]
            LegacyValue::Atom(x) => ValueKind::Atom(x.borrow().id(), x.borrow().name().clone()),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(x) => {
                ValueKind::Matrix(Box::new(Self::infer_matrix_value_kind(x)), x.shape())
            }
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(x) => ValueKind::Matrix(Box::new(ValueKind::Index), x.shape()),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(x) => ValueKind::Matrix(Box::new(ValueKind::Bool), x.shape()),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(x) => ValueKind::Matrix(Box::new(ValueKind::U8), x.shape()),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(x) => ValueKind::Matrix(Box::new(ValueKind::U16), x.shape()),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(x) => ValueKind::Matrix(Box::new(ValueKind::U32), x.shape()),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(x) => ValueKind::Matrix(Box::new(ValueKind::U64), x.shape()),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(x) => ValueKind::Matrix(Box::new(ValueKind::U128), x.shape()),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(x) => ValueKind::Matrix(Box::new(ValueKind::I8), x.shape()),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(x) => ValueKind::Matrix(Box::new(ValueKind::I16), x.shape()),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(x) => ValueKind::Matrix(Box::new(ValueKind::I32), x.shape()),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(x) => ValueKind::Matrix(Box::new(ValueKind::I64), x.shape()),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(x) => ValueKind::Matrix(Box::new(ValueKind::I128), x.shape()),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(x) => ValueKind::Matrix(Box::new(ValueKind::F32), x.shape()),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(x) => ValueKind::Matrix(Box::new(ValueKind::F64), x.shape()),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(x) => {
                ValueKind::Matrix(Box::new(ValueKind::String), x.shape())
            }
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(x) => ValueKind::Matrix(Box::new(ValueKind::R64), x.shape()),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(x) => ValueKind::Matrix(Box::new(ValueKind::C64), x.shape()),
            #[cfg(feature = "table")]
            LegacyValue::Table(x) => x.borrow().kind(),
            #[cfg(feature = "set")]
            LegacyValue::Set(x) => x.borrow().kind(),
            #[cfg(feature = "map")]
            LegacyValue::Map(x) => x.borrow().kind(),
            #[cfg(feature = "record")]
            LegacyValue::Record(x) => x.borrow().kind(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(x) => x.borrow().kind(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(x) => x.borrow().kind(),
            LegacyValue::MutableReference(x) => ValueKind::Reference(Box::new(x.borrow().kind())),
            LegacyValue::Typed(_, kind) => kind.clone(),
            LegacyValue::EmptyKind(k) => k.clone(),
            LegacyValue::Empty => ValueKind::Empty,
            LegacyValue::IndexAll => ValueKind::Empty,
            LegacyValue::Id(_) => ValueKind::Id,
            LegacyValue::Index(_) => ValueKind::Index,
            LegacyValue::Kind(x) => x.clone(),
        }
    }

    #[cfg(feature = "matrix")]
    pub fn is_matrix(&self) -> bool {
        match self {
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(_) => true,
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(_) => true,
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(_) => true,
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(_) => true,
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(_) => true,
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(_) => true,
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(_) => true,
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(_) => true,
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(_) => true,
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(_) => true,
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(_) => true,
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(_) => true,
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(_) => true,
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(_) => true,
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(_) => true,
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(_) => true,
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(_) => true,
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(_) => true,
            _ => false,
        }
    }

    pub fn is_scalar(&self) -> bool {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(_) => true,
            #[cfg(feature = "u16")]
            LegacyValue::U16(_) => true,
            #[cfg(feature = "u32")]
            LegacyValue::U32(_) => true,
            #[cfg(feature = "u64")]
            LegacyValue::U64(_) => true,
            #[cfg(feature = "u128")]
            LegacyValue::U128(_) => true,
            #[cfg(feature = "i8")]
            LegacyValue::I8(_) => true,
            #[cfg(feature = "i16")]
            LegacyValue::I16(_) => true,
            #[cfg(feature = "i32")]
            LegacyValue::I32(_) => true,
            #[cfg(feature = "i64")]
            LegacyValue::I64(_) => true,
            #[cfg(feature = "i128")]
            LegacyValue::I128(_) => true,
            #[cfg(feature = "f32")]
            LegacyValue::F32(_) => true,
            #[cfg(feature = "f64")]
            LegacyValue::F64(_) => true,
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(_) => true,
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(_) => true,
            #[cfg(feature = "atom")]
            LegacyValue::Atom(_) => true,
            LegacyValue::Index(_) => true,
            _ => false,
        }
    }

    #[cfg(any(feature = "bool", feature = "variable_define"))]
    pub fn as_bool(&self) -> MResult<Ref<bool>> {
        if let LegacyValue::Bool(v) = self {
            Ok(v.clone())
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_bool()
        } else {
            Err(MechError::new(UnhandledFunctionArgumentKindError, None).with_compiler_loc())
        }
    }

    impl_as_type!(i8);
    impl_as_type!(i16);
    impl_as_type!(i32);
    impl_as_type!(i64);
    impl_as_type!(i128);
    impl_as_type!(u8);
    impl_as_type!(u16);
    impl_as_type!(u32);
    impl_as_type!(u64);
    impl_as_type!(u128);

    pub fn is_string(&self) -> bool {
        match self {
            #[cfg(feature = "string")]
            LegacyValue::String(_) => true,
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(_) => true,
            LegacyValue::MutableReference(val) => val.borrow().is_string(),
            _ => false,
        }
    }

    #[cfg(any(feature = "string", feature = "variable_define"))]
    pub fn as_string(&self) -> MResult<Ref<String>> {
        match self {
            LegacyValue::String(v) => Ok(v.clone()),
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok(Ref::new(format!("{}", v.borrow()))),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok(Ref::new(format!("{}", v.borrow()))),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(v) => Ok(Ref::new(format!("{}", v.borrow()))),
            #[cfg(feature = "rational")]
            LegacyValue::R64(v) => Ok(Ref::new(v.borrow().to_string())),
            #[cfg(feature = "complex")]
            LegacyValue::C64(v) => Ok(Ref::new(v.borrow().to_string())),
            LegacyValue::MutableReference(val) => val.borrow().as_string(),
            _ => Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "string",
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    #[cfg(feature = "r64")]
    pub fn as_r64(&self) -> MResult<Ref<R64>> {
        match self {
            LegacyValue::R64(v) => Ok(v.clone()),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
            LegacyValue::MutableReference(val) => val.borrow().as_r64(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "r64" }, None)
                    .with_compiler_loc(),
            ),
        }
    }

    #[cfg(feature = "c64")]
    pub fn as_c64(&self) -> MResult<Ref<C64>> {
        match self {
            LegacyValue::C64(v) => Ok(v.clone()),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok(Ref::new(C64::new(*v.borrow(), 0.0))),
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
            LegacyValue::MutableReference(val) => val.borrow().as_c64(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "c64" }, None)
                    .with_compiler_loc(),
            ),
        }
    }

    #[cfg(feature = "f32")]
    pub fn as_f32(&self) -> MResult<Ref<f32>> {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(Ref::new(*v.borrow() as f32)),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(Ref::new(*v.borrow() as f32)),
            LegacyValue::F32(v) => Ok(v.clone()),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok(Ref::new((*v.borrow()) as f32)),
            LegacyValue::MutableReference(val) => val.borrow().as_f32(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "f32" }, None)
                    .with_compiler_loc(),
            ),
        }
    }

    #[cfg(feature = "f64")]
    pub fn as_f64(&self) -> MResult<Ref<f64>> {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(Ref::new(*v.borrow() as f64)),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok(Ref::new((*v.borrow()) as f64)),
            LegacyValue::F64(v) => Ok(v.clone()),
            LegacyValue::MutableReference(val) => val.borrow().as_f64(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "f64" }, None)
                    .with_compiler_loc(),
            ),
        }
    }

    #[cfg(all(feature = "matrix", feature = "bool"))]
    pub fn as_vecbool(&self) -> MResult<Vec<bool>> {
        if let LegacyValue::MatrixBool(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::Bool(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecbool()
        } else {
            Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "bool",
                },
                None,
            )
            .with_compiler_loc())
        }
    }
    #[cfg(all(feature = "matrix", feature = "f64"))]
    pub fn as_vecf64(&self) -> MResult<Vec<f64>> {
        if let LegacyValue::MatrixF64(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::F64(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecf64()
        } else if let Ok(v) = self.as_f64() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "f64" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "f32"))]
    pub fn as_vecf32(&self) -> MResult<Vec<f32>> {
        if let LegacyValue::MatrixF32(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::F32(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecf32()
        } else if let Ok(v) = self.as_f32() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "f32" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "u8"))]
    pub fn as_vecu8(&self) -> MResult<Vec<u8>> {
        if let LegacyValue::MatrixU8(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::U8(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecu8()
        } else if let Ok(v) = self.as_u8() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "u8" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "u16"))]
    pub fn as_vecu16(&self) -> MResult<Vec<u16>> {
        if let LegacyValue::MatrixU16(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::U16(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecu16()
        } else if let Ok(v) = self.as_u16() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "u16" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "u32"))]
    pub fn as_vecu32(&self) -> MResult<Vec<u32>> {
        if let LegacyValue::MatrixU32(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::U32(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecu32()
        } else if let Ok(v) = self.as_u32() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "u32" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "u64"))]
    pub fn as_vecu64(&self) -> MResult<Vec<u64>> {
        if let LegacyValue::MatrixU64(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::U64(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecu64()
        } else if let Ok(v) = self.as_u64() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "u64" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "u128"))]
    pub fn as_vecu128(&self) -> MResult<Vec<u128>> {
        if let LegacyValue::MatrixU128(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::U128(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecu128()
        } else if let Ok(v) = self.as_u128() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "u128",
                },
                None,
            )
            .with_compiler_loc())
        }
    }
    #[cfg(all(feature = "matrix", feature = "i8"))]
    pub fn as_veci8(&self) -> MResult<Vec<i8>> {
        if let LegacyValue::MatrixI8(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::I8(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_veci8()
        } else if let Ok(v) = self.as_i8() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "i8" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "i16"))]
    pub fn as_veci16(&self) -> MResult<Vec<i16>> {
        if let LegacyValue::MatrixI16(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::I16(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_veci16()
        } else if let Ok(v) = self.as_i16() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "i16" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "i32"))]
    pub fn as_veci32(&self) -> MResult<Vec<i32>> {
        if let LegacyValue::MatrixI32(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::I32(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_veci32()
        } else if let Ok(v) = self.as_i32() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "i32" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "i64"))]
    pub fn as_veci64(&self) -> MResult<Vec<i64>> {
        if let LegacyValue::MatrixI64(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::I64(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_veci64()
        } else if let Ok(v) = self.as_i64() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "i64" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "i128"))]
    pub fn as_veci128(&self) -> MResult<Vec<i128>> {
        if let LegacyValue::MatrixI128(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::I128(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_veci128()
        } else if let Ok(v) = self.as_i128() {
            Ok(vec![v.borrow().clone()])
        } else {
            Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "i128",
                },
                None,
            )
            .with_compiler_loc())
        }
    }
    #[cfg(all(feature = "matrix", feature = "string"))]
    pub fn as_vecstring(&self) -> MResult<Vec<String>> {
        if let LegacyValue::MatrixString(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::String(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecstring()
        } else {
            Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "string",
                },
                None,
            )
            .with_compiler_loc())
        }
    }
    #[cfg(all(feature = "matrix", feature = "r64"))]
    pub fn as_vecr64(&self) -> MResult<Vec<R64>> {
        if let LegacyValue::MatrixR64(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::R64(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecr64()
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "r64" }, None)
                    .with_compiler_loc(),
            )
        }
    }
    #[cfg(all(feature = "matrix", feature = "c64"))]
    pub fn as_vecc64(&self) -> MResult<Vec<C64>> {
        if let LegacyValue::MatrixC64(v) = self {
            Ok(v.as_vec())
        } else if let LegacyValue::C64(v) = self {
            Ok(vec![v.borrow().clone()])
        } else if let LegacyValue::MutableReference(val) = self {
            val.borrow().as_vecc64()
        } else {
            Err(
                MechError::new(CannotConvertToTypeError { target_type: "c64" }, None)
                    .with_compiler_loc(),
            )
        }
    }

    pub fn as_vecusize(&self) -> MResult<Vec<usize>> {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(vec![*v.borrow() as usize]),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok(vec![(*v.borrow()) as usize]),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok(vec![(*v.borrow()) as usize]),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(v) => Ok(v.as_vec()),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| (*x) as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| (*x) as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(v) => Ok(v
                .as_vec()
                .iter()
                .map(|x| *x as usize)
                .collect::<Vec<usize>>()),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(_) => Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "[usize]",
                },
                None,
            )
            .with_compiler_loc()),
            #[cfg(feature = "bool")]
            LegacyValue::Bool(_) => Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "[usize]",
                },
                None,
            )
            .with_compiler_loc()),
            LegacyValue::MutableReference(x) => x.borrow().as_vecusize(),
            _ => Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "[usize]",
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    fn as_one_based_usize(&self) -> Option<usize> {
        #[cfg(any(feature = "f32", feature = "f64"))]
        fn positive_float(value: f64) -> Option<usize> {
            if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
                return None;
            }
            usize::try_from(value as u128).ok()
        }

        let index = match self {
            LegacyValue::Index(value) => *value.borrow(),
            #[cfg(feature = "u8")]
            LegacyValue::U8(value) => usize::from(*value.borrow()),
            #[cfg(feature = "u16")]
            LegacyValue::U16(value) => usize::from(*value.borrow()),
            #[cfg(feature = "u32")]
            LegacyValue::U32(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "u64")]
            LegacyValue::U64(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "u128")]
            LegacyValue::U128(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "i8")]
            LegacyValue::I8(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "i16")]
            LegacyValue::I16(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "i32")]
            LegacyValue::I32(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "i64")]
            LegacyValue::I64(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "i128")]
            LegacyValue::I128(value) => usize::try_from(*value.borrow()).ok()?,
            #[cfg(feature = "f32")]
            LegacyValue::F32(value) => positive_float(f64::from(*value.borrow()))?,
            #[cfg(feature = "f64")]
            LegacyValue::F64(value) => positive_float(*value.borrow())?,
            LegacyValue::MutableReference(value) => value.borrow().as_one_based_usize()?,
            _ => return None,
        };
        (index > 0).then_some(index)
    }

    #[cfg(feature = "matrix")]
    fn as_one_based_index_matrix(&self) -> Option<Vec<usize>> {
        #[cfg(any(feature = "f32", feature = "f64"))]
        fn positive_float(value: f64) -> Option<usize> {
            if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
                return None;
            }
            usize::try_from(value as u128).ok()
        }

        #[cfg(any(
            feature = "u8",
            feature = "u16",
            feature = "u32",
            feature = "u64",
            feature = "u128"
        ))]
        macro_rules! unsigned_matrix {
            ($variant:ident) => {
                if let LegacyValue::$variant(value) = self {
                    return value
                        .as_vec()
                        .into_iter()
                        .map(|value| usize::try_from(value).ok().filter(|value| *value > 0))
                        .collect();
                }
            };
        }
        #[cfg(any(
            feature = "i8",
            feature = "i16",
            feature = "i32",
            feature = "i64",
            feature = "i128"
        ))]
        macro_rules! signed_matrix {
            ($variant:ident) => {
                if let LegacyValue::$variant(value) = self {
                    return value
                        .as_vec()
                        .into_iter()
                        .map(|value| usize::try_from(value).ok().filter(|value| *value > 0))
                        .collect();
                }
            };
        }
        #[cfg(any(feature = "f32", feature = "f64"))]
        macro_rules! float_matrix {
            ($variant:ident) => {
                if let LegacyValue::$variant(value) = self {
                    return value
                        .as_vec()
                        .into_iter()
                        .map(|value| positive_float(f64::from(value)))
                        .collect();
                }
            };
        }

        if let LegacyValue::MatrixIndex(value) = self {
            return value
                .as_vec()
                .into_iter()
                .map(|value| (value > 0).then_some(value))
                .collect();
        }
        #[cfg(feature = "u8")]
        unsigned_matrix!(MatrixU8);
        #[cfg(feature = "u16")]
        unsigned_matrix!(MatrixU16);
        #[cfg(feature = "u32")]
        unsigned_matrix!(MatrixU32);
        #[cfg(feature = "u64")]
        unsigned_matrix!(MatrixU64);
        #[cfg(feature = "u128")]
        unsigned_matrix!(MatrixU128);
        #[cfg(feature = "i8")]
        signed_matrix!(MatrixI8);
        #[cfg(feature = "i16")]
        signed_matrix!(MatrixI16);
        #[cfg(feature = "i32")]
        signed_matrix!(MatrixI32);
        #[cfg(feature = "i64")]
        signed_matrix!(MatrixI64);
        #[cfg(feature = "i128")]
        signed_matrix!(MatrixI128);
        #[cfg(feature = "f32")]
        float_matrix!(MatrixF32);
        #[cfg(feature = "f64")]
        float_matrix!(MatrixF64);
        if let LegacyValue::MutableReference(value) = self {
            return value.borrow().as_one_based_index_matrix();
        }
        None
    }

    pub fn as_index(&self) -> MResult<LegacyValue> {
        if let Some(index) = self.as_one_based_usize() {
            return Ok(LegacyValue::Index(Ref::new(index)));
        }
        #[cfg(feature = "matrix")]
        if let Some(indexes) = self.as_one_based_index_matrix() {
            let shape = self.shape();
            return Ok(LegacyValue::MatrixIndex(usize::to_matrix(
                indexes,
                shape[0] * shape[1],
                1,
            )));
        }
        #[cfg(all(feature = "matrix", feature = "bool"))]
        if let Ok(values) = self.as_vecbool() {
            let shape = self.shape();
            let out = match (shape[0], shape[1]) {
                (1, 1) => LegacyValue::Bool(Ref::new(values[0])),
                #[cfg(feature = "vectord")]
                _ => LegacyValue::MatrixBool(Matrix::DVector(Ref::new(DVector::from_vec(values)))),
                #[cfg(not(feature = "vectord"))]
                _ => todo!(),
            };
            return Ok(out);
        }
        #[cfg(feature = "bool")]
        if let Ok(value) = self.as_bool() {
            return Ok(LegacyValue::Bool(value));
        }
        Err(MechError::new(
            CannotConvertToTypeError {
                target_type: "ix (1..=max)",
            },
            None,
        )
        .with_compiler_loc())
    }

    pub fn as_usize(&self) -> MResult<usize> {
        match self {
            LegacyValue::Index(v) => Ok(*v.borrow()),
            #[cfg(feature = "u8")]
            LegacyValue::U8(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "u16")]
            LegacyValue::U16(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "u32")]
            LegacyValue::U32(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "u64")]
            LegacyValue::U64(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "u128")]
            LegacyValue::U128(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "i8")]
            LegacyValue::I8(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "i16")]
            LegacyValue::I16(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "i32")]
            LegacyValue::I32(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "i64")]
            LegacyValue::I64(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "i128")]
            LegacyValue::I128(v) => Ok(*v.borrow() as usize),
            #[cfg(feature = "f32")]
            LegacyValue::F32(v) => Ok((*v.borrow()) as usize),
            #[cfg(feature = "f64")]
            LegacyValue::F64(v) => Ok((*v.borrow()) as usize),
            LegacyValue::MutableReference(v) => v.borrow().as_usize(),
            _ => Err(MechError::new(
                CannotConvertToTypeError {
                    target_type: "usize",
                },
                None,
            )
            .with_compiler_loc()),
        }
    }

    #[cfg(feature = "u8")]
    pub fn expect_u8(&self) -> MResult<Ref<u8>> {
        match self {
            LegacyValue::U8(v) => Ok(v.clone()),
            LegacyValue::MutableReference(v) => v.borrow().expect_u8(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "u8" }, None)
                    .with_compiler_loc(),
            ),
        }
    }

    #[cfg(feature = "f64")]
    pub fn expect_f64(&self) -> MResult<Ref<f64>> {
        match self {
            LegacyValue::F64(v) => Ok(v.clone()),
            LegacyValue::MutableReference(v) => v.borrow().expect_f64(),
            _ => Err(
                MechError::new(CannotConvertToTypeError { target_type: "f64" }, None)
                    .with_compiler_loc(),
            ),
        }
    }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for LegacyValue {
    fn pretty_print(&self) -> String {
        match self {
            #[cfg(feature = "u8")]
            LegacyValue::U8(x) => format!("{}", x.borrow()),
            #[cfg(feature = "u16")]
            LegacyValue::U16(x) => format!("{}", x.borrow()),
            #[cfg(feature = "u32")]
            LegacyValue::U32(x) => format!("{}", x.borrow()),
            #[cfg(feature = "u64")]
            LegacyValue::U64(x) => format!("{}", x.borrow()),
            #[cfg(feature = "u128")]
            LegacyValue::U128(x) => format!("{}", x.borrow()),
            #[cfg(feature = "i8")]
            LegacyValue::I8(x) => format!("{}", x.borrow()),
            #[cfg(feature = "i16")]
            LegacyValue::I16(x) => format!("{}", x.borrow()),
            #[cfg(feature = "i32")]
            LegacyValue::I32(x) => format!("{}", x.borrow()),
            #[cfg(feature = "i64")]
            LegacyValue::I64(x) => format!("{}", x.borrow()),
            #[cfg(feature = "i128")]
            LegacyValue::I128(x) => format!("{}", x.borrow()),
            #[cfg(feature = "f32")]
            LegacyValue::F32(x) => format!("{}", x.borrow()),
            #[cfg(feature = "f64")]
            LegacyValue::F64(x) => format!("{}", x.borrow()),
            #[cfg(any(feature = "bool", feature = "variable_define"))]
            LegacyValue::Bool(x) => format!("{}", x.borrow()),
            #[cfg(feature = "complex")]
            LegacyValue::C64(x) => x.borrow().pretty_print(),
            #[cfg(feature = "rational")]
            LegacyValue::R64(x) => format!("{}", x.borrow().pretty_print()),
            #[cfg(feature = "atom")]
            LegacyValue::Atom(x) => format!("{}", x.borrow()),
            #[cfg(feature = "set")]
            LegacyValue::Set(x) => x.borrow().pretty_print(),
            #[cfg(feature = "map")]
            LegacyValue::Map(x) => x.borrow().pretty_print(),
            #[cfg(any(feature = "string", feature = "variable_define"))]
            LegacyValue::String(x) => Self::format_string_inline(x.borrow().as_str()),
            #[cfg(feature = "table")]
            LegacyValue::Table(x) => x.borrow().pretty_print(),
            #[cfg(feature = "tuple")]
            LegacyValue::Tuple(x) => x.borrow().pretty_print(),
            #[cfg(feature = "record")]
            LegacyValue::Record(x) => x.borrow().pretty_print(),
            #[cfg(feature = "enum")]
            LegacyValue::Enum(x) => x.borrow().pretty_print(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixIndex(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "bool"))]
            LegacyValue::MatrixBool(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "u8"))]
            LegacyValue::MatrixU8(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "u16"))]
            LegacyValue::MatrixU16(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "u32"))]
            LegacyValue::MatrixU32(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "u64"))]
            LegacyValue::MatrixU64(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "u128"))]
            LegacyValue::MatrixU128(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "i8"))]
            LegacyValue::MatrixI8(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "i16"))]
            LegacyValue::MatrixI16(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "i32"))]
            LegacyValue::MatrixI32(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "i64"))]
            LegacyValue::MatrixI64(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "i128"))]
            LegacyValue::MatrixI128(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "f32"))]
            LegacyValue::MatrixF32(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "f64"))]
            LegacyValue::MatrixF64(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "string"))]
            LegacyValue::MatrixString(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "rational"))]
            LegacyValue::MatrixR64(x) => x.pretty_print(),
            #[cfg(all(feature = "matrix", feature = "complex"))]
            LegacyValue::MatrixC64(x) => x.pretty_print(),
            #[cfg(feature = "matrix")]
            LegacyValue::MatrixValue(x) => x.pretty_print(),
            LegacyValue::Index(x) => format!("{}", x.borrow()),
            LegacyValue::MutableReference(x) => x.borrow().pretty_print(),
            LegacyValue::Typed(x, _) => x.pretty_print(),
            LegacyValue::Empty | LegacyValue::EmptyKind(_) => "_".to_string(),
            LegacyValue::IndexAll => ":".to_string(),
            LegacyValue::Id(x) => format!("{}", humanize(x)),
            LegacyValue::Kind(x) => format!("<{}>", x),
        }
    }
}

pub trait ToIndex {
    fn to_index(&self) -> LegacyValue;
}

#[cfg(feature = "matrix")]
impl ToIndex for Ref<Vec<i64>> {
    fn to_index(&self) -> LegacyValue {
        (*self.borrow())
            .iter()
            .map(|x| *x as usize)
            .collect::<Vec<usize>>()
            .to_value()
    }
}

pub trait ToValue {
    fn to_value(&self) -> LegacyValue;
}

#[cfg(feature = "matrix")]
impl ToValue for Vec<usize> {
    fn to_value(&self) -> LegacyValue {
        match self.len() {
            1 => LegacyValue::Index(Ref::new(self[0].clone())),
            #[cfg(feature = "vector2")]
            2 => {
                LegacyValue::MatrixIndex(Matrix::Vector2(Ref::new(Vector2::from_vec(self.clone()))))
            }
            #[cfg(feature = "vector3")]
            3 => {
                LegacyValue::MatrixIndex(Matrix::Vector3(Ref::new(Vector3::from_vec(self.clone()))))
            }
            #[cfg(feature = "vector4")]
            4 => {
                LegacyValue::MatrixIndex(Matrix::Vector4(Ref::new(Vector4::from_vec(self.clone()))))
            }
            #[cfg(feature = "vectord")]
            _ => {
                LegacyValue::MatrixIndex(Matrix::DVector(Ref::new(DVector::from_vec(self.clone()))))
            }
            #[cfg(not(feature = "vectord"))]
            _ => todo!(),
        }
    }
}

impl ToValue for Ref<usize> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Index(self.clone())
    }
}
#[cfg(feature = "u8")]
impl ToValue for Ref<u8> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::U8(self.clone())
    }
}
#[cfg(feature = "u16")]
impl ToValue for Ref<u16> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::U16(self.clone())
    }
}
#[cfg(feature = "u32")]
impl ToValue for Ref<u32> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::U32(self.clone())
    }
}
#[cfg(feature = "u64")]
impl ToValue for Ref<u64> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::U64(self.clone())
    }
}
#[cfg(feature = "u128")]
impl ToValue for Ref<u128> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::U128(self.clone())
    }
}
#[cfg(feature = "i8")]
impl ToValue for Ref<i8> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::I8(self.clone())
    }
}
#[cfg(feature = "i16")]
impl ToValue for Ref<i16> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::I16(self.clone())
    }
}
#[cfg(feature = "i32")]
impl ToValue for Ref<i32> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::I32(self.clone())
    }
}
#[cfg(feature = "i64")]
impl ToValue for Ref<i64> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::I64(self.clone())
    }
}
#[cfg(feature = "i128")]
impl ToValue for Ref<i128> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::I128(self.clone())
    }
}
#[cfg(feature = "f32")]
impl ToValue for Ref<f32> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::F32(self.clone())
    }
}
#[cfg(feature = "f64")]
impl ToValue for Ref<f64> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::F64(self.clone())
    }
}
#[cfg(any(feature = "bool", feature = "variable_define"))]
impl ToValue for Ref<bool> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Bool(self.clone())
    }
}
#[cfg(any(feature = "string", feature = "variable_define"))]
impl ToValue for Ref<String> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::String(self.clone())
    }
}
#[cfg(feature = "rational")]
impl ToValue for Ref<R64> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::R64(self.clone())
    }
}
#[cfg(feature = "complex")]
impl ToValue for Ref<C64> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::C64(self.clone())
    }
}
#[cfg(feature = "atom")]
impl ToValue for Ref<MechAtom> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Atom(self.clone())
    }
}
#[cfg(feature = "enum")]
impl ToValue for Ref<MechEnum> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Enum(self.clone())
    }
}

impl ToValue for Ref<LegacyValue> {
    fn to_value(&self) -> LegacyValue {
        (*self.borrow()).clone()
    }
}

#[cfg(feature = "u8")]
impl From<u8> for LegacyValue {
    fn from(val: u8) -> Self {
        LegacyValue::U8(Ref::new(val))
    }
}

#[cfg(feature = "u16")]
impl From<u16> for LegacyValue {
    fn from(val: u16) -> Self {
        LegacyValue::U16(Ref::new(val))
    }
}

#[cfg(feature = "u32")]
impl From<u32> for LegacyValue {
    fn from(val: u32) -> Self {
        LegacyValue::U32(Ref::new(val))
    }
}

#[cfg(feature = "u64")]
impl From<u64> for LegacyValue {
    fn from(val: u64) -> Self {
        LegacyValue::U64(Ref::new(val))
    }
}

#[cfg(feature = "u128")]
impl From<u128> for LegacyValue {
    fn from(val: u128) -> Self {
        LegacyValue::U128(Ref::new(val))
    }
}

#[cfg(feature = "i8")]
impl From<i8> for LegacyValue {
    fn from(val: i8) -> Self {
        LegacyValue::I8(Ref::new(val))
    }
}

#[cfg(feature = "i16")]
impl From<i16> for LegacyValue {
    fn from(val: i16) -> Self {
        LegacyValue::I16(Ref::new(val))
    }
}

#[cfg(feature = "i32")]
impl From<i32> for LegacyValue {
    fn from(val: i32) -> Self {
        LegacyValue::I32(Ref::new(val))
    }
}

#[cfg(feature = "i64")]
impl From<i64> for LegacyValue {
    fn from(val: i64) -> Self {
        LegacyValue::I64(Ref::new(val))
    }
}

#[cfg(feature = "i128")]
impl From<i128> for LegacyValue {
    fn from(val: i128) -> Self {
        LegacyValue::I128(Ref::new(val))
    }
}

#[cfg(feature = "f32")]
impl From<f32> for LegacyValue {
    fn from(val: f32) -> Self {
        LegacyValue::F32(Ref::new(val))
    }
}

#[cfg(feature = "f64")]
impl From<f64> for LegacyValue {
    fn from(val: f64) -> Self {
        LegacyValue::F64(Ref::new(val))
    }
}

#[cfg(any(feature = "bool", feature = "variable_define"))]
impl From<bool> for LegacyValue {
    fn from(val: bool) -> Self {
        LegacyValue::Bool(Ref::new(val))
    }
}

#[cfg(any(feature = "string", feature = "variable_define"))]
impl From<String> for LegacyValue {
    fn from(val: String) -> Self {
        LegacyValue::String(Ref::new(val))
    }
}

#[cfg(feature = "rational")]
impl From<R64> for LegacyValue {
    fn from(val: R64) -> Self {
        LegacyValue::R64(Ref::new(val))
    }
}

pub trait ToUsize {
    fn to_usize(&self) -> usize;
}

macro_rules! impl_unsigned_to_usize_for {
    ($t:ty) => {
        impl ToUsize for $t {
            fn to_usize(&self) -> usize {
                *self as usize
            }
        }
    };
}

#[cfg(any(
    feature = "i8",
    feature = "i16",
    feature = "i32",
    feature = "i64",
    feature = "i128",
    feature = "f32",
    feature = "f64"
))]
macro_rules! impl_signed_to_usize_for {
    ($t:ty) => {
        impl ToUsize for $t {
            fn to_usize(&self) -> usize {
                if *self < 0 as $t {
                    panic!("Cannot convert negative number to usize");
                }
                *self as usize
            }
        }
    };
}

#[cfg(feature = "u8")]
impl_unsigned_to_usize_for!(u8);
#[cfg(feature = "u16")]
impl_unsigned_to_usize_for!(u16);
#[cfg(feature = "u32")]
impl_unsigned_to_usize_for!(u32);
#[cfg(feature = "u64")]
impl_unsigned_to_usize_for!(u64);
#[cfg(feature = "u128")]
impl_unsigned_to_usize_for!(u128);
impl_unsigned_to_usize_for!(usize);

#[cfg(feature = "i8")]
impl_signed_to_usize_for!(i8);
#[cfg(feature = "i16")]
impl_signed_to_usize_for!(i16);
#[cfg(feature = "i32")]
impl_signed_to_usize_for!(i32);
#[cfg(feature = "i64")]
impl_signed_to_usize_for!(i64);
#[cfg(feature = "i128")]
impl_signed_to_usize_for!(i128);

#[cfg(feature = "f64")]
impl_signed_to_usize_for!(f64);
#[cfg(feature = "f32")]
impl_signed_to_usize_for!(f32);

#[cfg(feature = "table")]
impl ToValue for Ref<MechTable> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Table(self.clone())
    }
}

#[cfg(feature = "set")]
impl ToValue for Ref<MechSet> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Set(self.clone())
    }
}

#[cfg(feature = "map")]
impl ToValue for Ref<MechMap> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Map(self.clone())
    }
}

#[cfg(feature = "tuple")]
impl ToValue for Ref<MechTuple> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Tuple(self.clone())
    }
}

#[cfg(feature = "record")]
impl ToValue for Ref<MechRecord> {
    fn to_value(&self) -> LegacyValue {
        LegacyValue::Record(self.clone())
    }
}

#[cfg(test)]
mod reactive_cell_tests {
    use super::*;
    #[cfg(any(feature = "map", feature = "record", feature = "table"))]
    use indexmap::IndexMap;
    #[cfg(feature = "set")]
    use indexmap::IndexSet;

    fn cell_ids(ids: &[u64]) -> Vec<ReactiveCellId> {
        ids.iter().copied().map(ReactiveCellId::new).collect()
    }

    #[cfg(feature = "f64")]
    #[test]
    fn index_conversion_is_exact_and_one_based() {
        assert!(matches!(
            LegacyValue::F64(Ref::new(1.0)).as_index().unwrap(),
            LegacyValue::Index(value) if *value.borrow() == 1
        ));
        for invalid in [0.0, -1.0, 1.5] {
            assert!(LegacyValue::F64(Ref::new(invalid)).as_index().is_err());
        }
        assert!(LegacyValue::Index(Ref::new(0)).as_index().is_err());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn scalar_reactive_cell_identity_is_stable() {
        let scalar = Ref::new(1.0);
        let value = LegacyValue::F64(scalar.clone());

        let first = value.reactive_cell_ids();
        let second = value.reactive_cell_ids();

        assert_eq!(first, second);
        assert_eq!(first, cell_ids(&[scalar.id()]));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn scalar_display_is_distribution_neutral() {
        assert_eq!(LegacyValue::F64(Ref::new(3.0)).to_string(), "3");
    }

    #[cfg(feature = "string")]
    #[test]
    fn canonical_inline_strings_escape_recursively_across_value_containers() {
        let string = LegacyValue::String(Ref::new(
            "a\"b\\c\nnext\r\t\u{2028}line\u{2029}paragraph".to_string(),
        ));
        let expected = "\"a\\\"b\\\\c\\nnext\\r\\t\\u{2028}line\\u{2029}paragraph\"";
        assert_eq!(string.format_canonical_inline(), expected);

        #[cfg(feature = "tuple")]
        {
            let tuple = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![string.clone()])));
            assert_eq!(tuple.format_canonical_inline(), format!("({expected})"));
        }

        #[cfg(feature = "set")]
        {
            let set = LegacyValue::Set(Ref::new(MechSet::from_vec(vec![string.clone()])));
            assert_eq!(set.format_canonical_inline(), format!("{{{expected}}}"));
        }

        #[cfg(feature = "map")]
        {
            let key = LegacyValue::String(Ref::new("key".to_string()));
            let map = LegacyValue::Map(Ref::new(MechMap::from_vec(vec![(key, string.clone())])));
            assert_eq!(
                map.format_canonical_inline(),
                format!("{{\"key\": {expected}}}")
            );
        }

        #[cfg(feature = "record")]
        {
            let record =
                LegacyValue::Record(Ref::new(MechRecord::new(vec![("message", string.clone())])));
            assert_eq!(
                record.format_canonical_inline(),
                format!("{{message: {expected}}}")
            );
        }

        #[cfg(feature = "enum")]
        {
            let variant_id = hash_str("message/value");
            let enum_id = hash_str("message");
            let mut names = Dictionary::new();
            names.insert(enum_id, "message".to_string());
            names.insert(variant_id, "message/value".to_string());
            let enm = LegacyValue::Enum(Ref::new(MechEnum {
                id: enum_id,
                variants: vec![(variant_id, Some(string.clone()))],
                names: Ref::new(names),
            }));
            assert_eq!(enm.format_canonical_inline(), format!(":value({expected})"));

            let mut names = Dictionary::new();
            names.insert(enum_id, "message".to_string());
            names.insert(hash_str("message/ready"), "message/ready".to_string());
            names.insert(hash_str("message/error"), "message/error".to_string());
            let definition = LegacyValue::Enum(Ref::new(MechEnum {
                id: enum_id,
                variants: vec![
                    (hash_str("message/ready"), None),
                    (
                        hash_str("message/error"),
                        Some(LegacyValue::Kind(ValueKind::String)),
                    ),
                ],
                names: Ref::new(names),
            }));
            assert_eq!(definition.format_canonical_inline(), "<:message>");
        }
    }

    #[cfg(all(feature = "string", feature = "table", feature = "matrixd"))]
    #[test]
    fn canonical_inline_tables_terminate_every_row() {
        let column_id = hash_str("message");
        let column = Matrix::from_vec(
            vec![
                LegacyValue::String(Ref::new("first".to_string())),
                LegacyValue::String(Ref::new("second".to_string())),
            ],
            2,
            1,
        );
        let table = LegacyValue::Table(Ref::new(MechTable::from_parts(
            2,
            1,
            vec![(column_id, ValueKind::String, column)],
            vec![(column_id, "message".to_string())],
        )));
        assert_eq!(
            table.format_canonical_inline(),
            "|message<string>| \"first\" | \"second\" |"
        );
    }

    #[cfg(all(feature = "string", feature = "table", feature = "matrixd"))]
    #[test]
    fn zero_row_table_previews_close_nested_kind_and_table_delimiters() {
        let mut columns = Vec::new();
        let mut names = Vec::new();
        for index in 0..10 {
            let name = format!("column{index}");
            let column_id = hash_str(&name);
            columns.push((
                column_id,
                ValueKind::String,
                Matrix::from_vec(Vec::<LegacyValue>::new(), 0, 1),
            ));
            names.push((column_id, name));
        }
        let table = LegacyValue::Table(Ref::new(MechTable::from_parts(
            0,
            columns.len(),
            columns,
            names,
        )));

        let canonical = table.format_canonical_inline();
        assert!(canonical.starts_with("<|"), "{canonical}");
        assert!(canonical.ends_with("|>"), "{canonical}");
        assert_eq!(
            table.format_canonical_inline_with_element_limit(2),
            "<|column0<string> column1<string> …<*>|>",
        );
        assert_eq!(
            table.format_kind_with_element_limit(2),
            "|column0<string> column1<string> …<*>|",
        );
        #[cfg(feature = "pretty_print")]
        {
            let html = table.to_html_with_element_limit(2);
            assert!(html.contains("mech-value-elided"), "{html}");
            assert!(html.contains("column0&lt;string&gt;"), "{html}");
            assert!(!html.contains("column9"), "{html}");
        }
        let preview = table.format_preview_inline(96);
        assert!(preview.starts_with("<|"), "{preview}");
        assert!(preview.ends_with("|>"), "{preview}");
        assert!(preview.contains('…'), "{preview}");
        assert!(preview.chars().count() <= 96, "{preview}");
    }

    #[cfg(all(feature = "string", feature = "table", feature = "matrixd"))]
    #[test]
    fn table_valued_cells_have_unambiguous_canonical_and_preview_boundaries() {
        let inner_column_id = hash_str("message");
        let inner = LegacyValue::Table(Ref::new(MechTable::from_parts(
            1,
            1,
            vec![(
                inner_column_id,
                ValueKind::String,
                Matrix::from_vec(vec![LegacyValue::String(Ref::new("x".repeat(100)))], 1, 1),
            )],
            vec![(inner_column_id, "message".to_string())],
        )));
        let outer_column_id = hash_str("nested");
        let outer = LegacyValue::Table(Ref::new(MechTable::from_parts(
            1,
            1,
            vec![(
                outer_column_id,
                inner.kind(),
                Matrix::from_vec(vec![inner], 1, 1),
            )],
            vec![(outer_column_id, "nested".to_string())],
        )));

        let canonical = outer.format_canonical_inline();
        assert!(
            canonical.starts_with("|nested<|message<string>|:1>| (|message<string>| \""),
            "{canonical}"
        );
        assert!(canonical.ends_with("\" |) |"), "{canonical}");

        let preview = outer.format_preview_inline(64);
        assert!(preview.starts_with("|nested<"), "{preview}");
        assert!(preview.ends_with("\"|)|"), "{preview}");
        assert!(preview.contains('…'), "{preview}");
        assert!(preview.chars().count() <= 64, "{preview}");
    }

    #[cfg(all(feature = "string", feature = "matrixd"))]
    #[test]
    fn inline_string_matrices_quote_and_escape_every_element() {
        let matrix = Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
            1,
            2,
            &["a\"b".to_string(), "c\\d\nnext".to_string()],
        )));
        assert_eq!(
            LegacyValue::MatrixString(matrix).format_canonical_inline(),
            "[\"a\\\"b\" \"c\\\\d\\nnext\"]"
        );
    }

    #[cfg(all(feature = "string", feature = "matrixd"))]
    #[test]
    fn inline_value_matrices_use_recursive_value_formatting() {
        let matrix = Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
            1,
            2,
            &[
                LegacyValue::String(Ref::new("a\"b".to_string())),
                LegacyValue::String(Ref::new("c\\d".to_string())),
            ],
        )));
        assert_eq!(
            LegacyValue::MatrixValue(matrix).format_canonical_inline(),
            "[\"a\\\"b\" \"c\\\\d\"]"
        );
    }

    #[cfg(feature = "string")]
    #[test]
    fn bounded_inline_previews_preserve_quotes_escapes_and_unicode_boundaries() {
        let escaped = LegacyValue::String(Ref::new("aa\"bbbb".to_string()));
        assert_eq!(escaped.format_preview_inline(5), "\"aa…\"");
        assert_eq!(escaped.format_preview_inline(7), "\"aa\\\"…\"");

        let unicode = LegacyValue::String(Ref::new("αβ\"γδεζη".to_string()));
        let preview = unicode.format_preview_inline(8);
        assert_eq!(preview, "\"αβ\\\"γ…\"");
        assert_eq!(preview.chars().count(), 8);
        assert!(!preview.ends_with("\\…\""), "split escape: {preview}");

        let separators =
            LegacyValue::String(Ref::new("left\u{2028}middle\u{2029}right".to_string()));
        assert_eq!(
            separators.format_preview_inline(usize::MAX),
            "\"left\\u{2028}middle\\u{2029}right\""
        );

        assert_eq!(unicode.format_preview_inline(0), "");
        assert_eq!(
            unicode.format_preview_inline(usize::MAX),
            unicode.format_canonical_inline()
        );
    }

    #[cfg(all(feature = "string", feature = "f64", feature = "matrixd"))]
    #[test]
    fn bounded_inline_previews_close_nested_string_and_matrix_delimiters() {
        let strings = Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
            1,
            2,
            &["αβ\"γδεζη".to_string(), "tail".to_string()],
        )));
        let preview = LegacyValue::MatrixString(strings).format_preview_inline(12);
        assert_eq!(preview, "[\"αβ\\\"γδε…\"]");
        assert_eq!(preview.chars().count(), 12);

        let numbers = Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
            1,
            8,
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        )));
        let preview = LegacyValue::MatrixF64(numbers).format_preview_inline(12);
        assert!(preview.starts_with('['), "{preview}");
        assert!(preview.ends_with(']'), "{preview}");
        assert!(preview.contains('…'), "{preview}");
        assert!(preview.chars().count() <= 12, "{preview}");

        for canonical in [
            "\"text\"",
            "[1 2]",
            "(1, 2)",
            "{1, 2}",
            "|x<f64>| 1 |",
            "<:message>",
            "<|x<f64>|>",
        ] {
            assert_eq!(LegacyValue::preview_canonical_inline(canonical, 1), "…");
            assert_eq!(LegacyValue::preview_canonical_inline(canonical, 2), "…");
        }

        let table = LegacyValue::preview_canonical_inline("|x<f64>| 1 2 3 4 5 6 |", 12);
        assert!(table.starts_with('|'), "{table}");
        assert!(table.ends_with('|'), "{table}");
        assert!(table.contains('…'), "{table}");
        assert!(table.chars().count() <= 12, "{table}");

        let table_with_string_bar = LegacyValue::preview_canonical_inline(
            "|message<string>| \"left|right\" | \"tail\" |",
            28,
        );
        assert!(
            table_with_string_bar.starts_with('|'),
            "{table_with_string_bar}"
        );
        assert!(
            table_with_string_bar.ends_with('|'),
            "{table_with_string_bar}"
        );
        assert!(
            table_with_string_bar.contains('…'),
            "{table_with_string_bar}"
        );
        assert!(table_with_string_bar.chars().count() <= 28);

        let enum_kind = LegacyValue::preview_canonical_inline(
            "<:a_very_long_retained_nominal_enum_definition>",
            16,
        );
        assert!(enum_kind.starts_with('<'), "{enum_kind}");
        assert!(enum_kind.ends_with('>'), "{enum_kind}");
        assert!(enum_kind.contains('…'), "{enum_kind}");

        let matrix_kind = LegacyValue::preview_canonical_inline("<[string]:1000,1000>", 12);
        assert!(matrix_kind.starts_with("<["), "{matrix_kind}");
        assert!(matrix_kind.ends_with('>'), "{matrix_kind}");
        assert!(matrix_kind.contains('…'), "{matrix_kind}");

        let nested_kind = LegacyValue::preview_canonical_inline(
            "(<|column0<string> column1<string> column2<string>|>, 1)",
            28,
        );
        assert!(nested_kind.starts_with("(<|"), "{nested_kind}");
        assert!(nested_kind.ends_with("|>)"), "{nested_kind}");
        assert!(nested_kind.contains('…'), "{nested_kind}");
        assert!(nested_kind.chars().count() <= 28, "{nested_kind}");
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn matrix_display_preserves_row_major_value_order() {
        let matrix = Matrix::DMatrix(Ref::new(na::DMatrix::from_row_slice(
            2,
            2,
            &[1.0, 2.0, 3.0, 4.0],
        )));
        assert_eq!(
            LegacyValue::MatrixF64(matrix).format_canonical_inline(),
            "[1 2; 3 4]"
        );
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn bounded_matrix_formatting_crosses_transparent_value_wrappers() {
        let matrix = LegacyValue::MatrixF64(Matrix::DMatrix(Ref::new(
            na::DMatrix::from_row_slice(2, 3, &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        )));
        let typed = LegacyValue::Typed(
            Box::new(matrix),
            ValueKind::Matrix(Box::new(ValueKind::F64), vec![2, 3]),
        );
        let referenced = LegacyValue::MutableReference(Ref::new(typed));

        assert_eq!(
            referenced.format_canonical_inline_with_element_limit(3),
            "[1 2 3 …]"
        );
        assert_eq!(referenced.format_canonical_inline(), "[1 2 3; 4 5 6]");
    }

    #[cfg(all(feature = "enum", feature = "tuple", feature = "f64"))]
    #[test]
    fn bounded_kind_formatting_traverses_enum_and_reference_wrappers_in_place() {
        let payload = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(
            (0..32).map(|_| LegacyValue::F64(Ref::new(0.0))).collect(),
        )));
        let enum_id = hash_str("message");
        let variant_id = hash_str("message/value");
        let mut names = Dictionary::new();
        names.insert(enum_id, "message".to_string());
        names.insert(variant_id, "message/value".to_string());
        let value =
            LegacyValue::MutableReference(Ref::new(LegacyValue::Enum(Ref::new(MechEnum {
                id: enum_id,
                variants: vec![(variant_id, Some(payload))],
                names: Ref::new(names),
            }))));

        assert_eq!(
            value.format_kind_with_element_limit(2),
            ":value((f64,f64,…))",
        );
    }

    #[cfg(all(
        feature = "f64",
        feature = "string",
        feature = "tuple",
        feature = "set",
        feature = "map",
        feature = "record",
        feature = "enum",
        feature = "table",
        feature = "matrixd"
    ))]
    #[test]
    fn bounded_inline_formatting_caps_every_aggregate_family() {
        let numbers = (1..=5)
            .map(|number| LegacyValue::F64(Ref::new(number as f64)))
            .collect::<Vec<_>>();

        let tuple = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(numbers.clone())));
        assert_eq!(
            tuple.format_canonical_inline_with_element_limit(2),
            "(1, 2, …)"
        );

        let set = LegacyValue::Set(Ref::new(MechSet::from_vec(numbers.clone())));
        assert_eq!(
            set.format_canonical_inline_with_element_limit(2),
            "{1, 2, …}"
        );

        let map = LegacyValue::Map(Ref::new(MechMap::from_vec(
            numbers
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    (
                        LegacyValue::String(Ref::new(format!("key{index}"))),
                        value.clone(),
                    )
                })
                .collect(),
        )));
        assert_eq!(
            map.format_canonical_inline_with_element_limit(2),
            "{\"key0\": 1, \"key1\": 2, …}"
        );

        let record = LegacyValue::Record(Ref::new(MechRecord::new(vec![
            ("one", numbers[0].clone()),
            ("two", numbers[1].clone()),
            ("three", numbers[2].clone()),
        ])));
        assert_eq!(
            record.format_canonical_inline_with_element_limit(2),
            "{one: 1, two: 2, …}"
        );

        let enum_id = hash_str("answer");
        let variant_id = hash_str("answer/value");
        let mut names = Dictionary::new();
        names.insert(enum_id, "answer".to_string());
        names.insert(variant_id, "answer/value".to_string());
        let enm = LegacyValue::Enum(Ref::new(MechEnum {
            id: enum_id,
            variants: vec![(variant_id, Some(tuple))],
            names: Ref::new(names),
        }));
        assert_eq!(
            enm.format_canonical_inline_with_element_limit(0),
            ":value(…)"
        );

        let column_id = hash_str("message");
        let column = Matrix::from_vec(
            vec![
                LegacyValue::String(Ref::new("first".to_string())),
                LegacyValue::String(Ref::new("second".to_string())),
            ],
            2,
            1,
        );
        let table = LegacyValue::Table(Ref::new(MechTable::from_parts(
            2,
            1,
            vec![(column_id, ValueKind::String, column)],
            vec![(column_id, "message".to_string())],
        )));
        assert_eq!(
            table.format_canonical_inline_with_element_limit(2),
            "|message<string>| \"first\" | … |"
        );
        assert_eq!(
            table.format_canonical_inline_with_element_limit(0),
            "|…<*>| … |"
        );
    }

    #[cfg(all(feature = "f64", feature = "matrixd"))]
    #[test]
    fn matrix_value_addr_uses_backing_matrix_identity() {
        let matrix = Matrix::DMatrix(Ref::new(na::DMatrix::from_element(2, 2, 1.0)));

        assert_eq!(LegacyValue::MatrixF64(matrix.clone()).addr(), matrix.addr());
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn deep_snapshot_preserves_dynamic_matrix_storage() {
        for (rows, cols) in [(2, 2), (1, 5), (5, 1)] {
            let source = Ref::new(na::DMatrix::from_vec(
                rows,
                cols,
                (0..rows * cols).map(|value| value as f64).collect(),
            ));
            let value = LegacyValue::MatrixF64(Matrix::DMatrix(source.clone()));

            let snapshot = value.try_deep_snapshot().expect("acyclic matrix fixture");
            let LegacyValue::MatrixF64(Matrix::DMatrix(snapshot)) = snapshot else {
                panic!("snapshot changed the dynamic {rows}x{cols} matrix storage class");
            };

            assert_eq!(*snapshot.borrow(), *source.borrow());
            assert_ne!(snapshot.as_ptr(), source.as_ptr());
        }
    }

    #[cfg(all(feature = "f64", feature = "matrix2", feature = "matrixd"))]
    #[test]
    fn deep_snapshot_preserves_value_matrix_storage() {
        let live = Ref::new(1.0);
        let source = Matrix::DMatrix(Ref::new(na::DMatrix::from_vec(
            2,
            2,
            vec![
                LegacyValue::F64(live.clone()),
                LegacyValue::F64(Ref::new(2.0)),
                LegacyValue::F64(Ref::new(3.0)),
                LegacyValue::F64(Ref::new(4.0)),
            ],
        )));

        let snapshot = LegacyValue::MatrixValue(source)
            .try_deep_snapshot()
            .expect("acyclic value-matrix fixture");
        let LegacyValue::MatrixValue(Matrix::DMatrix(snapshot)) = snapshot else {
            panic!("snapshot changed the dynamic value-matrix storage class");
        };
        let snapshot = snapshot.borrow();
        let LegacyValue::F64(first) = &snapshot[0] else {
            panic!("expected scalar matrix element");
        };

        assert_eq!(*first.borrow(), 1.0);
        assert_ne!(first.as_ptr(), live.as_ptr());
    }

    #[cfg(feature = "f64")]
    #[test]
    fn typed_value_reuses_inner_cell_identity() {
        let scalar = Ref::new(1.0);
        let value = LegacyValue::Typed(Box::new(LegacyValue::F64(scalar.clone())), ValueKind::F64);

        assert_eq!(value.reactive_cell_ids(), cell_ids(&[scalar.id()]));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn mutable_reference_includes_outer_and_inner_cells() {
        let scalar = Ref::new(1.0);
        let outer = Ref::new(LegacyValue::F64(scalar.clone()));
        let value = LegacyValue::MutableReference(outer.clone());

        assert_eq!(
            value.reactive_cell_ids(),
            cell_ids(&[outer.id(), scalar.id()])
        );
    }

    #[cfg(all(feature = "set", feature = "f64"))]
    #[test]
    fn reactive_root_cells_exclude_nested_container_cells() {
        let first = Ref::new(1.0);
        let second = Ref::new(2.0);
        let mut members = IndexSet::new();
        members.insert(LegacyValue::F64(first.clone()));
        members.insert(LegacyValue::F64(second.clone()));
        let set = Ref::new(MechSet {
            kind: ValueKind::F64,
            max_elements: Some(2),
            num_elements: 2,
            set: members,
        });
        let value = LegacyValue::Set(set.clone());

        assert_eq!(value.reactive_root_cell_ids(), cell_ids(&[set.id()]));
        assert_eq!(
            value.reactive_cell_ids(),
            cell_ids(&[set.id(), first.id(), second.id()])
        );
    }

    #[cfg(feature = "f64")]
    #[test]
    fn mutable_reference_root_cell_is_outer_only() {
        let scalar = Ref::new(1.0);
        let outer = Ref::new(LegacyValue::F64(scalar.clone()));
        let value = LegacyValue::MutableReference(outer.clone());

        assert_eq!(value.reactive_root_cell_ids(), cell_ids(&[outer.id()]));
        assert_eq!(
            value.reactive_cell_ids(),
            cell_ids(&[outer.id(), scalar.id()])
        );
        assert_eq!(value.logical_reactive_cell_ids(), cell_ids(&[scalar.id()]));
    }

    #[test]
    fn logical_reactive_cells_terminate_reference_cycles() {
        let reference = Ref::new(LegacyValue::Empty);
        *reference.borrow_mut() = LegacyValue::MutableReference(reference.clone());
        let value = LegacyValue::MutableReference(reference.clone());

        assert_eq!(
            value.logical_reactive_cell_ids(),
            cell_ids(&[reference.id()])
        );
    }

    #[cfg(all(feature = "table", feature = "matrix", feature = "f64"))]
    #[test]
    fn table_reactive_cells_include_columns_and_nested_values() {
        let a = Ref::new(1.0);
        let b = Ref::new(2.0);
        let c = Ref::new(3.0);
        let d = Ref::new(4.0);
        let first_column = Matrix::from_vec(
            vec![LegacyValue::F64(a.clone()), LegacyValue::F64(b.clone())],
            2,
            1,
        );
        let second_column = Matrix::from_vec(
            vec![LegacyValue::F64(c.clone()), LegacyValue::F64(d.clone())],
            2,
            1,
        );
        let first_column_id = first_column.addr() as u64;
        let second_column_id = second_column.addr() as u64;
        let mut data = IndexMap::new();
        data.insert(hash_str("first"), (ValueKind::F64, first_column));
        data.insert(hash_str("second"), (ValueKind::F64, second_column));
        let table = Ref::new(MechTable {
            rows: 2,
            cols: 2,
            data,
            col_names: HashMap::new(),
        });

        assert_eq!(
            LegacyValue::Table(table.clone()).reactive_cell_ids(),
            cell_ids(&[
                table.id(),
                first_column_id,
                a.id(),
                b.id(),
                second_column_id,
                c.id(),
                d.id()
            ]),
        );
    }

    #[cfg(all(feature = "record", feature = "tuple", feature = "f64"))]
    #[test]
    fn record_and_tuple_recurse_into_nested_values() {
        let a = Ref::new(1.0);
        let b = Ref::new(2.0);
        let tuple = Ref::new(MechTuple {
            elements: vec![
                Box::new(LegacyValue::F64(a.clone())),
                Box::new(LegacyValue::F64(b.clone())),
            ],
        });
        let mut data = IndexMap::new();
        data.insert(hash_str("tuple"), LegacyValue::Tuple(tuple.clone()));
        let record = Ref::new(MechRecord {
            cols: 1,
            kinds: vec![ValueKind::Tuple(vec![ValueKind::F64, ValueKind::F64])],
            data,
            field_names: HashMap::new(),
        });

        assert_eq!(
            LegacyValue::Record(record.clone()).reactive_cell_ids(),
            cell_ids(&[record.id(), tuple.id(), a.id(), b.id()])
        );
    }

    #[cfg(all(feature = "record", feature = "tuple", feature = "f64"))]
    #[test]
    fn deep_snapshot_detaches_nested_record_and_tuple_cells() {
        let live = Ref::new(1.0);
        let value = LegacyValue::Record(Ref::new(MechRecord::new(vec![(
            "position",
            LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
                LegacyValue::F64(live.clone()),
                LegacyValue::F64(Ref::new(2.0)),
            ]))),
        )])));

        let snapshot = value.try_deep_snapshot().expect("acyclic fixture");
        *live.borrow_mut() = 9.0;

        let LegacyValue::Record(snapshot) = snapshot else {
            panic!("expected record snapshot");
        };
        let position = {
            let snapshot = snapshot.borrow();
            let LegacyValue::Tuple(position) = snapshot.data.get(&hash_str("position")).unwrap()
            else {
                panic!("expected tuple field");
            };
            position.clone()
        };
        let position = position.borrow();
        let LegacyValue::F64(x) = position.elements[0].as_ref() else {
            panic!("expected scalar tuple element");
        };
        assert_eq!(*x.borrow(), 1.0);
        assert_ne!(x.as_ptr(), live.as_ptr());
    }

    #[cfg(all(feature = "map", feature = "set", feature = "f64"))]
    #[test]
    fn map_and_set_recurse_in_container_order() {
        let key1 = Ref::new(1.0);
        let value1 = Ref::new(2.0);
        let key2 = Ref::new(3.0);
        let value2 = Ref::new(4.0);
        let mut map_data = IndexMap::new();
        map_data.insert(
            LegacyValue::F64(key1.clone()),
            LegacyValue::F64(value1.clone()),
        );
        map_data.insert(
            LegacyValue::F64(key2.clone()),
            LegacyValue::F64(value2.clone()),
        );
        let map = Ref::new(MechMap {
            key_kind: ValueKind::F64,
            value_kind: ValueKind::F64,
            num_elements: 2,
            map: map_data,
        });
        assert_eq!(
            LegacyValue::Map(map.clone()).reactive_cell_ids(),
            cell_ids(&[map.id(), key1.id(), value1.id(), key2.id(), value2.id()])
        );

        let set1 = Ref::new(5.0);
        let set2 = Ref::new(6.0);
        let mut set_data = IndexSet::new();
        set_data.insert(LegacyValue::F64(set1.clone()));
        set_data.insert(LegacyValue::F64(set2.clone()));
        let set = Ref::new(MechSet {
            kind: ValueKind::F64,
            max_elements: Some(2),
            num_elements: 2,
            set: set_data,
        });
        assert_eq!(
            LegacyValue::Set(set.clone()).reactive_cell_ids(),
            cell_ids(&[set.id(), set1.id(), set2.id()])
        );
    }

    #[cfg(all(feature = "enum", feature = "f64"))]
    #[test]
    fn enum_recurse_excludes_dictionary() {
        let payload = Ref::new(1.0);
        let dictionary = Ref::new(Dictionary::new());
        let dictionary_id = dictionary.id();
        let enum_value = Ref::new(MechEnum {
            id: hash_str("example"),
            variants: vec![(hash_str("payload"), Some(LegacyValue::F64(payload.clone())))],
            names: dictionary,
        });

        let ids = LegacyValue::Enum(enum_value.clone()).reactive_cell_ids();
        assert_eq!(ids, cell_ids(&[enum_value.id(), payload.id()]));
        assert!(!ids.contains(&ReactiveCellId::new(dictionary_id)));
    }

    #[cfg(feature = "f64")]
    #[test]
    fn legacy_ref_helper_includes_program_cell() {
        let inner = Ref::new(1.0);
        let cell = Ref::new(LegacyValue::F64(inner.clone()));

        assert_eq!(
            legacy_ref_reactive_cell_ids(&cell),
            cell_ids(&[cell.id(), inner.id()])
        );
    }
}

// Errors

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKindError;

impl MechErrorKind for UnhandledFunctionArgumentKindError {
    fn name(&self) -> &str {
        "UnhandledFunctionArgumentKind"
    }
    fn message(&self) -> String {
        "Value kind is not valid for this function.".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct CannotConvertToTypeError {
    pub target_type: &'static str,
}

impl MechErrorKind for CannotConvertToTypeError {
    fn name(&self) -> &str {
        "CannotConvertToType"
    }
    fn message(&self) -> String {
        format!("Cannot convert to {}", self.target_type)
    }
}

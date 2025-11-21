use crate::*;
use crate::nodes::Matrix as Mat;
#[cfg(feature = "matrix")]
use crate::matrix::Matrix;
#[cfg(feature = "complex")]
use crate::types::complex_numbers::C64;
#[cfg(feature = "rational")]
use num_rational::Rational64;

#[cfg(feature = "no_std")]
use core::mem;
#[cfg(not(feature = "no_std"))]
use std::mem;

#[cfg(feature = "matrix")]
use nalgebra::DVector;

macro_rules! impl_as_type {
  ($target_type:ty) => {
    paste!{
      pub fn [<as_ $target_type>](&self) -> MResult<Ref<$target_type>> {
        match self {
          #[cfg(feature = "u8")]
          Value::U8(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "u16")]
          Value::U16(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "u32")]
          Value::U32(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "u64")]
          Value::U64(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "u128")]
          Value::U128(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "i8")]
          Value::I8(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "i16")]
          Value::I16(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "i32")]
          Value::I32(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "i64")]
          Value::I64(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "i128")]
          Value::I128(v) => Ok(Ref::new(*v.borrow() as $target_type)),
          #[cfg(feature = "f32")]
          Value::F32(v) => Ok(Ref::new((*v.borrow()) as $target_type)),
          #[cfg(feature = "f64")]
          Value::F64(v) => Ok(Ref::new((*v.borrow()) as $target_type)),
          Value::Id(v) => Ok(Ref::new(*v as $target_type)),
          Value::MutableReference(val) => val.borrow().[<as_ $target_type>](),
          _ => Err(
            MechError2::new(
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
  U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64, C64, R64,
  String, Bool, Id, Index, Empty, Any, 
  Matrix(Box<ValueKind>,Vec<usize>),  Enum(u64),                  Record(Vec<(String,ValueKind)>),
  Map(Box<ValueKind>,Box<ValueKind>), Atom(u64),                  Table(Vec<(String,ValueKind)>, usize), 
  Tuple(Vec<ValueKind>),              Reference(Box<ValueKind>),  Set(Box<ValueKind>, Option<usize>), 
  Option(Box<ValueKind>),
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
      ValueKind::Matrix(x,s) => write!(f, "[{}]:{}", x, s.iter().map(|s| s.to_string()).collect::<Vec<String>>().join(",")),
      ValueKind::Enum(x) => write!(f, "{}",x),
      ValueKind::Set(x,el) => write!(f, "{{{}}}{}", x, el.map_or("".to_string(), |e| format!(":{}", e))),
      ValueKind::Map(x,y) => write!(f, "{{{}:{}}}",x,y),
      ValueKind::Record(x) => write!(f, "{{{}}}",x.iter().map(|(i,k)| format!("{}<{}>",i.to_string(),k)).collect::<Vec<String>>().join(" ")),
      ValueKind::Table(x,y) => {
        let size_str = if y > &0 { format!(":{}", y) } else { "".to_string() };
        write!(f, "|{}|{}",x.iter().map(|(i,k)| format!("{}<{}>",i.to_string(),k)).collect::<Vec<String>>().join(" "),size_str)
      }
      ValueKind::Tuple(x) => write!(f, "({})",x.iter().map(|x| format!("{}",x)).collect::<Vec<String>>().join(",")),
      ValueKind::Id => write!(f, "id"),
      ValueKind::Index => write!(f, "ix"),
      ValueKind::Reference(x) => write!(f, "{}",x),
      ValueKind::Atom(x) => write!(f, "`{}",x),
      ValueKind::Empty => write!(f, "_"),
      ValueKind::Any => write!(f, "*"),
      ValueKind::Option(x) => write!(f, "{}?", x),
    }
  }
}

impl ValueKind {

  #[cfg(feature = "compiler")]
  pub fn to_feature_kind(&self) -> FeatureKind {
    match self {
      #[cfg(feature = "i8")]
      ValueKind::I8 => FeatureKind::I8,
      #[cfg(feature = "i16")]
      ValueKind::I16 => FeatureKind::I16,
      #[cfg(feature = "i32")]
      ValueKind::I32 => FeatureKind::I32,
      #[cfg(feature = "i64")]
      ValueKind::I64 => FeatureKind::I64,
      #[cfg(feature = "i128")]
      ValueKind::I128 => FeatureKind::I128,
      #[cfg(feature = "u8")] 
      ValueKind::U8 => FeatureKind::U8,
      #[cfg(feature = "u16")]
      ValueKind::U16 => FeatureKind::U16,
      #[cfg(feature = "u32")]
      ValueKind::U32 => FeatureKind::U32,
      #[cfg(feature = "u64")]
      ValueKind::U64 => FeatureKind::U64,
      #[cfg(feature = "u128")]
      ValueKind::U128 => FeatureKind::U128,
      #[cfg(feature = "f32")]
      ValueKind::F32 => FeatureKind::F32, 
      #[cfg(feature = "f64")]
      ValueKind::F64 => FeatureKind::F64,
      #[cfg(any(feature = "string", feature = "variable_define"))]
      ValueKind::String => FeatureKind::String,
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      ValueKind::Bool => FeatureKind::Bool,
      #[cfg(feature = "table")]
      ValueKind::Table(_,_) => FeatureKind::Table,
      #[cfg(feature = "set")]
      ValueKind::Set(_,_) => FeatureKind::Set,
      #[cfg(feature = "map")]
      ValueKind::Map(_,_) => FeatureKind::Map,
      #[cfg(feature = "record")]
      ValueKind::Record(_) => FeatureKind::Record,
      #[cfg(feature = "tuple")] 
      ValueKind::Tuple(_) => FeatureKind::Tuple,
      #[cfg(feature = "enum")]
      ValueKind::Enum(_) => FeatureKind::Enum,
      #[cfg(feature = "matrix")]
      ValueKind::Matrix(_,shape) => {
        match shape[..] {
          #[cfg(feature = "matrix1")]
          [1,1] => FeatureKind::Matrix1,
          #[cfg(feature = "matrix2")]
          [2,2] => FeatureKind::Matrix2,
          #[cfg(feature = "matrix3")]
          [3,3] => FeatureKind::Matrix3,
          #[cfg(feature = "matrix4")]
          [4,4] => FeatureKind::Matrix4,
          #[cfg(feature = "matrix2x3")]
          [2,3] => FeatureKind::Matrix2x3,
          #[cfg(feature = "matrix3x2")]
          [3,2] => FeatureKind::Matrix3x2,
          #[cfg(feature = "row_vector2")]
          [1,2] => FeatureKind::RowVector2,
          #[cfg(feature = "row_vector3")]
          [1,3] => FeatureKind::RowVector3,
          #[cfg(feature = "row_vector4")]
          [1,4] => FeatureKind::RowVector4,
          #[cfg(feature = "vector2")]
          [2,1] => FeatureKind::Vector2,
          #[cfg(feature = "vector3")]
          [3,1] => FeatureKind::Vector3,
          #[cfg(feature = "vector4")]
          [4,1] => FeatureKind::Vector4,
          #[cfg(feature = "row_vectord")]
          [1,n] => FeatureKind::RowVectorD,
          #[cfg(feature = "vectord")]
          [n,1] => FeatureKind::VectorD,
          #[cfg(feature = "matrixd")]
          [n,m] => FeatureKind::MatrixD,
          _ => panic!("Unsupported matrix shape for feature kind: {}", self),
        }
      }
      #[cfg(feature = "complex")]
      ValueKind::C64 => FeatureKind::C64,
      #[cfg(feature = "rational")]
      ValueKind::R64 => FeatureKind::R64,
      ValueKind::Atom(_) => FeatureKind::Atom,
      ValueKind::Index => FeatureKind::Index,
      _ => panic!("Unsupported feature kind for value kind: {}", self),
    }
  }

  pub fn collection_kind(&self) -> Option<ValueKind> {
    match self {
      ValueKind::Matrix(x,_) => Some(*x.clone()),
      ValueKind::Set(x,_) => Some(*x.clone()),
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
    use ValueKind::*;
    match (self, other) {
      // Unsigned widening
      (U8, U16) | (U8, U32) | (U8, U64) | (U8, U128) |
      (U16, U32) | (U16, U64) | (U16, U128) |
      (U32, U64) | (U32, U128) |
      (U64, U128) => true,

      // Signed widening
      (I8, I16) | (I8, I32) | (I8, I64) | (I8, I128) |
      (I16, I32) | (I16, I64) | (I16, I128) |
      (I32, I64) | (I32, I128) |
      (I64, I128) => true,

      // Unsigned -> signed widening
      (U8, I16) | (U8, I32) | (U8, I64) | (U8, I128) |
      (U16, I32) | (U16, I64) | (U16, I128) |
      (U32, I64) | (U32, I128) |
      (U64, I128) => true,

      // Signed -> unsigned widening (runtime safety not enforced here)
      (I8, U16) | (I8, U32) | (I8, U64) | (I8, U128) |
      (I16, U32) | (I16, U64) | (I16, U128) |
      (I32, U64) | (I32, U128) |
      (I64, U128) => true,

      // Integer -> float
      (U8, F32) | (U8, F64) |
      (U16, F32) | (U16, F64) |
      (U32, F32) | (U32, F64) |
      (U64, F32) | (U64, F64) |
      (U128, F32) | (U128, F64) |
      (I8, F32) | (I8, F64) |
      (I16, F32) | (I16, F64) |
      (I32, F32) | (I32, F64) |
      (I64, F32) | (I64, F64) |
      (I128, F32) | (I128, F64) => true,

      // Float widening + narrowing
      (F32, F64) | (F64, F32) => true,

      // Float -> integer (allowed, but lossy)
      (F32, I8) | (F32, I16) | (F32, I32) | (F32, I64) | (F32, I128) |
      (F32, U8) | (F32, U16) | (F32, U32) | (F32, U64) | (F32, U128) |
      (F64, I8) | (F64, I16) | (F64, I32) | (F64, I64) | (F64, I128) |
      (F64, U8) | (F64, U16) | (F64, U32) | (F64, U64) | (F64, U128) => true,

      // Index conversions (both ways)
      (Index, U8) | (Index, U16) | (Index, U32) | (Index, U64) | (Index, U128) |
      (Index, I8) | (Index, I16) | (Index, I32) | (Index, I64) | (Index, I128) |
      (Index, F32) | (Index, F64) |
      (U8, Index) | (U16, Index) | (U32, Index) | (U64, Index) | (U128, Index) |
      (I8, Index) | (I16, Index) | (I32, Index) | (I64, Index) | (I128, Index) => true,

      // Matrix: element type convertible and shape matches
      (Matrix(box a, ashape), Matrix(box b, bshape)) if ashape.into_iter().product::<usize>() == bshape.into_iter().product::<usize>() && a.is_convertible_to(b) => true,

      // Option conversions
      (Option(box a), Option(box b)) if a.is_convertible_to(b) => true,

      // Reference conversions
      (Reference(box a), Reference(box b)) if a.is_convertible_to(b) => true,

      // Tuple conversions (element-wise)
      (Tuple(a), Tuple(b)) if a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.is_convertible_to(y)) => true,

      // Set conversions
      (Set(box a, _), Set(box b, _)) if a.is_convertible_to(b) => true,

      // Map conversions
      (Map(box ak, box av), Map(box bk, box bv)) if ak.is_convertible_to(bk) && av.is_convertible_to(bv) => true,

      // Table conversions: allow source to have extra columns
      (Table(acols, _), Table(bcols, _)) if bcols.iter().all(|(bk, bv)| 
        acols.iter().any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
      ) => true,

      // Record conversions: allow source to have extra fields
      (Record(afields), Record(bfields)) if bfields.iter().all(|(bk, bv)| 
        afields.iter().any(|(ak, av)| ak == bk && av.is_convertible_to(bv))
      ) => true,

      // Direct match
      _ => self == other,
    }
  }

  pub fn is_compatible(k1: ValueKind, k2: ValueKind) -> bool {
    match k1 {
      ValueKind::Reference(x) => {
        ValueKind::is_compatible(*x,k2)
      }
      ValueKind::Matrix(x,_) => {
        *x == k2
      }
      x => x == k2,
    }
  }

  pub fn align(&self) -> usize {
    // pointer alignment (platform word size) for pointer-like kinds
    let ptr_align = mem::align_of::<usize>();

    match self {
      // unsigned integers
      ValueKind::U8   => 1,
      ValueKind::U16  => 2,
      ValueKind::U32  => 4,
      ValueKind::U64  => 8,
      ValueKind::U128 => 16,

      // signed integers
      ValueKind::I8   => 1,
      ValueKind::I16  => 2,
      ValueKind::I32  => 4,
      ValueKind::I64  => 8,
      ValueKind::I128 => 16,

      // floats
      ValueKind::F32  => 4,
      ValueKind::F64  => 8,

      // complex / rational (assume composed of f64 parts)
      ValueKind::C64 => 8,
      ValueKind::R64 => 8,

      // small simple payloads
      ValueKind::Bool => 1,
      ValueKind::String => 1, // strings are length+bytes; bytes are packed
      ValueKind::Id | ValueKind::Index => 8, // id/index -> likely machine word (u64)
      ValueKind::Empty => 1,
      ValueKind::Any => ptr_align,

      // compound types
      ValueKind::Matrix(elem_ty, _dims) => {
        // matrix alignment = alignment of element type
        elem_ty.align()
      }

      ValueKind::Enum(_space) => 8, // enum tag stored in u64
      ValueKind::Atom(_id) => 8,

      ValueKind::Record(fields) => {
        // record alignment = max alignment of fields (or 1 if empty)
        fields.iter()
            .map(|(_, ty)| ty.align())
            .max()
            .unwrap_or(1)
      }

      ValueKind::Map(_, _) => ptr_align,   // typically pointer-based representation
      ValueKind::Table(cols, _pk) => {
        // table: use max column alignment or pointer-align if empty
        cols.iter()
            .map(|(_, ty)| ty.align())
            .max()
            .unwrap_or(ptr_align)
      }

      ValueKind::Tuple(elems) => {
        elems.iter().map(|ty| ty.align()).max().unwrap_or(1)
      }

      ValueKind::Reference(inner) => ptr_align, // references are pointers at runtime

      ValueKind::Set(elem, _) => {
        // set alignment equals element alignment (or ptr_align fallback)
        match elem.as_ref() {
          v => v.align()
        }
      }
      ValueKind::Option(inner) => inner.align(),
    }
  }
}

pub trait AsNaKind {
  fn as_na_kind() -> String;
}

macro_rules! impl_as_na_kind {
  ($type:ty, $kind:expr) => {
    impl<T> AsNaKind for $type {
      fn as_na_kind() -> String { $kind.to_string() }
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
      fn as_value_kind() -> ValueKind { $value_kind }
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

impl AsValueKind for Value {
  fn as_value_kind() -> ValueKind {
    ValueKind::Any
  }
}


// Value
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
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
  MatrixValue(Matrix<Value>),
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
  Kind(ValueKind),
  IndexAll,
  Empty
}

impl Eq for Value {}

impl fmt::Display for Value {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if cfg!(feature = "pretty_print") {
      #[cfg(feature = "pretty_print")]
      return fmt::Display::fmt(&self.pretty_print(), f);
      fmt::Display::fmt(&"".to_string(), f) // kind of a hack to assuage the compiler
    } else {
      write!(f, "{:?}", self)
    }
  }
}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      #[cfg(feature = "rational")]
      Value::R64(x) => x.borrow().hash(state),
      #[cfg(feature = "u8")]
      Value::U8(x)   => x.borrow().hash(state),
      #[cfg(feature = "u16")]
      Value::U16(x)  => x.borrow().hash(state),
      #[cfg(feature = "u32")]
      Value::U32(x)  => x.borrow().hash(state),
      #[cfg(feature = "u64")]
      Value::U64(x)  => x.borrow().hash(state),
      #[cfg(feature = "u128")]
      Value::U128(x) => x.borrow().hash(state),
      #[cfg(feature = "i8")]
      Value::I8(x)   => x.borrow().hash(state),
      #[cfg(feature = "i16")]
      Value::I16(x)  => x.borrow().hash(state),
      #[cfg(feature = "i32")]
      Value::I32(x)  => x.borrow().hash(state),
      #[cfg(feature = "i64")]
      Value::I64(x)  => x.borrow().hash(state),
      #[cfg(feature = "i128")]
      Value::I128(x) => x.borrow().hash(state),
      #[cfg(feature = "f32")]
      Value::F32(x)  => x.borrow().to_bits().hash(state),
      #[cfg(feature = "f64")]
      Value::F64(x)  => x.borrow().to_bits().hash(state),
      #[cfg(feature = "complex")]
      Value::C64(x) => x.borrow().hash(state),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(x) => x.borrow().hash(state),
      #[cfg(feature = "atom")]
      Value::Atom(x) => x.borrow().hash(state),
      #[cfg(feature = "set")]
      Value::Set(x)  => x.borrow().hash(state),
      #[cfg(feature = "map")]
      Value::Map(x)  => x.borrow().hash(state),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().hash(state),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.borrow().hash(state),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().hash(state),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.borrow().hash(state),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(x) => x.borrow().hash(state),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.hash(state),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x)   => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x)   => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x)  => todo!(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x)  => todo!(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.hash(state),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x)  => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => x.hash(state),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => x.hash(state),
      Value::Id(x)   => x.hash(state),
      Value::Kind(x) => x.hash(state),
      Value::Index(x)=> x.borrow().hash(state),
      Value::MutableReference(x) => x.borrow().hash(state),
      Value::Empty => Value::Empty.hash(state),
      Value::IndexAll => Value::IndexAll.hash(state),
    }
  }
}
impl Value {

  #[cfg(feature = "matrix")]
  pub unsafe fn get_copyable_matrix_unchecked<T>(&self) -> Box<dyn CopyMat<T>> 
  where T: AsValueKind + 'static
  {
    match (T::as_value_kind(), self) {
      #[cfg(all(feature = "matrix", feature = "bool"))]
      (ValueKind::Bool, Value::MatrixBool(m)) => {
        let b: Box<dyn CopyMat<bool>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<bool>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "u8"))]
      (ValueKind::U8, Value::MatrixU8(m)) => {
        let b: Box<dyn CopyMat<u8>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<u8>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "u16"))]
      (ValueKind::U16, Value::MatrixU16(m)) => {
        let b: Box<dyn CopyMat<u16>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<u16>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "u32"))]
      (ValueKind::U32, Value::MatrixU32(m)) => {
        let b: Box<dyn CopyMat<u32>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<u32>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "u64"))]
      (ValueKind::U64, Value::MatrixU64(m)) => {
        let b: Box<dyn CopyMat<u64>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<u64>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "u128"))]
      (ValueKind::U128, Value::MatrixU128(m)) => {
        let b: Box<dyn CopyMat<u128>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<u128>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "i8"))]
      (ValueKind::I8, Value::MatrixI8(m)) => {
        let b: Box<dyn CopyMat<i8>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<i8>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "i16"))]
      (ValueKind::I16, Value::MatrixI16(m)) => {
        let b: Box<dyn CopyMat<i16>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<i16>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "i32"))]
      (ValueKind::I32, Value::MatrixI32(m)) => {
        let b: Box<dyn CopyMat<i32>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<i32>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "i64"))]
      (ValueKind::I64, Value::MatrixI64(m)) => {
        let b: Box<dyn CopyMat<i64>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<i64>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "i128"))]
      (ValueKind::I128, Value::MatrixI128(m)) => {
        let b: Box<dyn CopyMat<i128>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<i128>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "f32"))]
      (ValueKind::F32, Value::MatrixF32(m)) => {
        let b: Box<dyn CopyMat<f32>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<f32>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "f64"))]
      (ValueKind::F64, Value::MatrixF64(m)) => {
        let b: Box<dyn CopyMat<f64>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<f64>>, Box<dyn CopyMat<T>>>(b)
      }
      #[cfg(all(feature = "matrix", feature = "string"))]
      (ValueKind::String, Value::MatrixString(m)) => {
        let b: Box<dyn CopyMat<String>> = m.get_copyable_matrix();
        std::mem::transmute::<Box<dyn CopyMat<String>>, Box<dyn CopyMat<T>>>(b)
      }
      _ => panic!("Unsupported type for get_copyable_matrix_unchecked"),
    }
  }

  pub unsafe fn as_unchecked<T>(&self) -> &Ref<T> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(r) => &*(r as *const Ref<u8> as *const Ref<T>),
      #[cfg(feature = "u16")]
      Value::U16(r) => &*(r as *const Ref<u16> as *const Ref<T>),
      #[cfg(feature = "u32")]
      Value::U32(r) => &*(r as *const Ref<u32> as *const Ref<T>),
      #[cfg(feature = "u64")]
      Value::U64(r) => &*(r as *const Ref<u64> as *const Ref<T>),
      #[cfg(feature = "u128")]
      Value::U128(r) => &*(r as *const Ref<u128> as *const Ref<T>),
      #[cfg(feature = "i8")]
      Value::I8(r) => &*(r as *const Ref<i8> as *const Ref<T>),
      #[cfg(feature = "i16")]
      Value::I16(r) => &*(r as *const Ref<i16> as *const Ref<T>),
      #[cfg(feature = "i32")]
      Value::I32(r) => &*(r as *const Ref<i32> as *const Ref<T>),
      #[cfg(feature = "i64")]
      Value::I64(r) => &*(r as *const Ref<i64> as *const Ref<T>),
      #[cfg(feature = "i128")]
      Value::I128(r) => &*(r as *const Ref<i128> as *const Ref<T>),
      #[cfg(feature = "f32")]
      Value::F32(r) => &*(r as *const Ref<f32> as *const Ref<T>),
      #[cfg(feature = "f64")]
      Value::F64(r) => &*(r as *const Ref<f64> as *const Ref<T>),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(r) => &*(r as *const Ref<String> as *const Ref<T>),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(r) => &*(r as *const Ref<bool> as *const Ref<T>),
      #[cfg(feature = "rational")]
      Value::R64(r) => &*(r as *const Ref<R64> as *const Ref<T>),
      #[cfg(feature = "complex")]
      Value::C64(r) => &*(r as *const Ref<C64> as *const Ref<T>),
      #[cfg(all(feature = "f64", feature = "matrix"))]
      Value::MatrixF64(r) => r.as_unchecked(),
      #[cfg(all(feature = "f32", feature = "matrix"))]
      Value::MatrixF32(r) => r.as_unchecked(),
      #[cfg(all(feature = "i8", feature = "matrix"))]
      Value::MatrixI8(r) => r.as_unchecked(),
      #[cfg(all(feature = "i16", feature = "matrix"))]
      Value::MatrixI16(r) => r.as_unchecked(),
      #[cfg(all(feature = "i32", feature = "matrix"))]
      Value::MatrixI32(r) => r.as_unchecked(),
      #[cfg(all(feature = "i64", feature = "matrix"))]
      Value::MatrixI64(r) => r.as_unchecked(),
      #[cfg(all(feature = "i128", feature = "matrix"))]
      Value::MatrixI128(r) => r.as_unchecked(),
      #[cfg(all(feature = "u8", feature = "matrix"))]
      Value::MatrixU8(r) => r.as_unchecked(),
      #[cfg(all(feature = "u16", feature = "matrix"))]
      Value::MatrixU16(r) => r.as_unchecked(),
      #[cfg(all(feature = "u32", feature = "matrix"))]
      Value::MatrixU32(r) => r.as_unchecked(),
      #[cfg(all(feature = "u64", feature = "matrix"))]
      Value::MatrixU64(r) => r.as_unchecked(),
      #[cfg(all(feature = "u128", feature = "matrix"))]
      Value::MatrixU128(r) => r.as_unchecked(),
      #[cfg(all(feature = "bool", feature = "matrix"))]
      Value::MatrixBool(r) => r.as_unchecked(),
      #[cfg(all(feature = "string", feature = "matrix"))]
      Value::MatrixString(r) => r.as_unchecked(),
      #[cfg(all(feature = "rational", feature = "matrix"))]
      Value::MatrixR64(r) => r.as_unchecked(),
      #[cfg(all(feature = "complex", feature = "matrix"))]
      Value::MatrixC64(r) => r.as_unchecked(),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(r) => r.as_unchecked(),
      Value::Index(r) => &*(r as *const Ref<usize> as *const Ref<T>),
      #[cfg(feature = "enum")]
      Value::Enum(r) => &*(r as *const Ref<MechEnum> as *const Ref<T>),
      #[cfg(feature = "set")]
      Value::Set(r) => &*(r as *const Ref<MechSet> as *const Ref<T>),
      #[cfg(feature = "table")]
      Value::Table(r) => &*(r as *const Ref<MechTable> as *const Ref<T>),
      x => panic!("Unsupported type for as_unchecked: {:?}.", x),
    }
  }

  pub fn addr(&self) -> usize {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => v.addr(),
      #[cfg(feature = "u16")]
      Value::U16(v) => v.addr(),
      #[cfg(feature = "u32")]
      Value::U32(v) => v.addr(),
      #[cfg(feature = "u64")]
      Value::U64(v) => v.addr(),
      #[cfg(feature = "u128")]
      Value::U128(v) => v.addr(),
      #[cfg(feature = "i8")]
      Value::I8(v) => v.addr(),
      #[cfg(feature = "i16")]
      Value::I16(v) => v.addr(),
      #[cfg(feature = "i32")]
      Value::I32(v) => v.addr(),
      #[cfg(feature = "i64")]
      Value::I64(v) => v.addr(),
      #[cfg(feature = "i128")]
      Value::I128(v) => v.addr(),
      #[cfg(feature = "f32")]
      Value::F32(v) => v.addr(),
      #[cfg(feature = "f64")]
      Value::F64(v) => v.addr(),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(v) => v.addr(),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(v) => v.addr(),
      #[cfg(feature = "complex")]
      Value::C64(v) => v.addr(),
      #[cfg(feature = "rational")]
      Value::R64(v) => v.addr(),
      #[cfg(feature = "record")]
      Value::Record(v) => v.addr(),
      #[cfg(feature = "table")]
      Value::Table(v) => v.addr(),
      #[cfg(feature = "map")]
      Value::Map(v) => v.addr(),
      #[cfg(feature = "tuple")]
      Value::Tuple(v) => v.addr(),
      #[cfg(feature = "set")]
      Value::Set(v) => v.addr(),
      #[cfg(feature = "enum")]
      Value::Enum(v) => v.addr(),
      #[cfg(feature = "atom")]
      Value::Atom(v) => v.addr(),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(v) => v.addr(),
      Value::Index(v) => v.addr(),
      Value::MutableReference(v) => v.addr(),
      _ => todo!(),
    }
  }

  pub fn convert_to(&self, other: &ValueKind) -> Option<Value> {

    if self.kind() == *other {
        return Some(self.clone());
    }

    if !self.kind().is_convertible_to(other) {
        return None;
    }

    match (self, other) {
    // ==== Unsigned widening and narrowing ====
    #[cfg(all(feature = "u8", feature = "u16"))]
    (Value::U8(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "u8", feature = "u32"))]
    (Value::U8(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "u8", feature = "u64"))]
    (Value::U8(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "u8", feature = "u128"))]
    (Value::U8(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "u8", feature = "i16"))]
    (Value::U8(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "u8", feature = "i32"))]
    (Value::U8(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "u8", feature = "i64"))]
    (Value::U8(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "u8", feature = "i128"))]
    (Value::U8(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "u8", feature = "f32"))]
    (Value::U8(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "u8", feature = "f64"))]
    (Value::U8(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "u16", feature = "u8"))]
    (Value::U16(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "u16", feature = "u32"))]
    (Value::U16(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "u16", feature = "u64"))]
    (Value::U16(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "u16", feature = "u128"))]
    (Value::U16(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "u16", feature = "i8"))]
    (Value::U16(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "u16", feature = "i32"))]
    (Value::U16(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "u16", feature = "i64"))]
    (Value::U16(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "u16", feature = "i128"))]
    (Value::U16(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "u16", feature = "f32"))]
    (Value::U16(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "u16", feature = "f64"))]
    (Value::U16(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "u32", feature = "u8"))]
    (Value::U32(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "u32", feature = "u16"))]
    (Value::U32(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "u32", feature = "u64"))]
    (Value::U32(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "u32", feature = "u128"))]
    (Value::U32(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "u32", feature = "i8"))]
    (Value::U32(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "u32", feature = "i16"))]
    (Value::U32(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "u32", feature = "i64"))]
    (Value::U32(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "u32", feature = "i128"))]
    (Value::U32(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "u32", feature = "f32"))]
    (Value::U32(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "u32", feature = "f64"))]
    (Value::U32(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "u64", feature = "u8"))]
    (Value::U64(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "u64", feature = "u16"))]
    (Value::U64(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "u64", feature = "u32"))]
    (Value::U64(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "u64", feature = "u128"))]
    (Value::U64(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "u64", feature = "i8"))]
    (Value::U64(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "u64", feature = "i16"))]
    (Value::U64(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "u64", feature = "i32"))]
    (Value::U64(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "u64", feature = "i128"))]
    (Value::U64(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "u64", feature = "f32"))]
    (Value::U64(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "u64", feature = "f64"))]
    (Value::U64(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "u128", feature = "u8"))]
    (Value::U128(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "u128", feature = "u16"))]
    (Value::U128(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "u128", feature = "u32"))]
    (Value::U128(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "u128", feature = "u64"))]
    (Value::U128(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "u128", feature = "i8"))]
    (Value::U128(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "u128", feature = "i16"))]
    (Value::U128(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "u128", feature = "i32"))]
    (Value::U128(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "u128", feature = "i64"))]
    (Value::U128(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "u128", feature = "f32"))]
    (Value::U128(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "u128", feature = "f64"))]
    (Value::U128(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    // ==== Signed widening and narrowing ====
    #[cfg(all(feature = "i8", feature = "i16"))]
    (Value::I8(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "i8", feature = "i32"))]
    (Value::I8(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "i8", feature = "i64"))]
    (Value::I8(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "i8", feature = "i128"))]
    (Value::I8(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "i8", feature = "u16"))]
    (Value::I8(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "i8", feature = "u32"))]
    (Value::I8(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "i8", feature = "u64"))]
    (Value::I8(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "i8", feature = "u128"))]
    (Value::I8(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "i8", feature = "f32"))]
    (Value::I8(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "i8", feature = "f64"))]
    (Value::I8(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "i16", feature = "i8"))]
    (Value::I16(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "i16", feature = "i32"))]
    (Value::I16(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "i16", feature = "i64"))]
    (Value::I16(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "i16", feature = "i128"))]
    (Value::I16(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "i16", feature = "u8"))]
    (Value::I16(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "i16", feature = "u32"))]
    (Value::I16(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "i16", feature = "u64"))]
    (Value::I16(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "i16", feature = "u128"))]
    (Value::I16(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "i16", feature = "f32"))]
    (Value::I16(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "i16", feature = "f64"))]
    (Value::I16(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "i32", feature = "i8"))]
    (Value::I32(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "i32", feature = "i16"))]
    (Value::I32(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "i32", feature = "i64"))]
    (Value::I32(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "i32", feature = "i128"))]
    (Value::I32(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "i32", feature = "u8"))]
    (Value::I32(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "i32", feature = "u16"))]
    (Value::I32(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "i32", feature = "u64"))]
    (Value::I32(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "i32", feature = "u128"))]
    (Value::I32(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "i32", feature = "f32"))]
    (Value::I32(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "i32", feature = "f64"))]
    (Value::I32(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "i64", feature = "i8"))]
    (Value::I64(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "i64", feature = "i16"))]
    (Value::I64(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "i64", feature = "i32"))]
    (Value::I64(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "i64", feature = "i128"))]
    (Value::I64(v), ValueKind::I128) => Some(Value::I128(Ref::new((*v.borrow()) as i128))),
    #[cfg(all(feature = "i64", feature = "u8"))]
    (Value::I64(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "i64", feature = "u16"))]
    (Value::I64(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "i64", feature = "u32"))]
    (Value::I64(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "i64", feature = "u128"))]
    (Value::I64(v), ValueKind::U128) => Some(Value::U128(Ref::new((*v.borrow()) as u128))),
    #[cfg(all(feature = "i64", feature = "f32"))]
    (Value::I64(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "i64", feature = "f64"))]
    (Value::I64(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    #[cfg(all(feature = "i128", feature = "i8"))]
    (Value::I128(v), ValueKind::I8) => Some(Value::I8(Ref::new((*v.borrow()) as i8))),
    #[cfg(all(feature = "i128", feature = "i16"))]
    (Value::I128(v), ValueKind::I16) => Some(Value::I16(Ref::new((*v.borrow()) as i16))),
    #[cfg(all(feature = "i128", feature = "i32"))]
    (Value::I128(v), ValueKind::I32) => Some(Value::I32(Ref::new((*v.borrow()) as i32))),
    #[cfg(all(feature = "i128", feature = "i64"))]
    (Value::I128(v), ValueKind::I64) => Some(Value::I64(Ref::new((*v.borrow()) as i64))),
    #[cfg(all(feature = "i128", feature = "u8"))]
    (Value::I128(v), ValueKind::U8) => Some(Value::U8(Ref::new((*v.borrow()) as u8))),
    #[cfg(all(feature = "i128", feature = "u16"))]
    (Value::I128(v), ValueKind::U16) => Some(Value::U16(Ref::new((*v.borrow()) as u16))),
    #[cfg(all(feature = "i128", feature = "u32"))]
    (Value::I128(v), ValueKind::U32) => Some(Value::U32(Ref::new((*v.borrow()) as u32))),
    #[cfg(all(feature = "i128", feature = "u64"))]
    (Value::I128(v), ValueKind::U64) => Some(Value::U64(Ref::new((*v.borrow()) as u64))),
    #[cfg(all(feature = "i128", feature = "f32"))]
    (Value::I128(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),
    #[cfg(all(feature = "i128", feature = "f64"))]
    (Value::I128(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),

    // ==== Float widening and narrowing ====
    #[cfg(all(feature = "f32", feature = "f64"))]
    (Value::F32(v), ValueKind::F64) => Some(Value::F64(Ref::new((*v.borrow()) as f64))),
    #[cfg(all(feature = "f32", feature = "f64"))]
    (Value::F64(v), ValueKind::F32) => Some(Value::F32(Ref::new((*v.borrow()) as f32))),

    // ==== Float to integer conversions (truncate) ====
    #[cfg(all(feature = "f32", feature = "i8"))]
    (Value::F32(v), ValueKind::I8) => Some(Value::I8(Ref::new(*v.borrow() as i8))),
    #[cfg(all(feature = "f32", feature = "i16"))]
    (Value::F32(v), ValueKind::I16) => Some(Value::I16(Ref::new(*v.borrow() as i16))),
    #[cfg(all(feature = "f32", feature = "i32"))]
    (Value::F32(v), ValueKind::I32) => Some(Value::I32(Ref::new(*v.borrow() as i32))),
    #[cfg(all(feature = "f32", feature = "i64"))]
    (Value::F32(v), ValueKind::I64) => Some(Value::I64(Ref::new(*v.borrow() as i64))),
    #[cfg(all(feature = "f32", feature = "i128"))]
    (Value::F32(v), ValueKind::I128) => Some(Value::I128(Ref::new(*v.borrow() as i128))),
    #[cfg(all(feature = "f32", feature = "u8"))]
    (Value::F32(v), ValueKind::U8) => Some(Value::U8(Ref::new(*v.borrow() as u8))),
    #[cfg(all(feature = "f32", feature = "u16"))]
    (Value::F32(v), ValueKind::U16) => Some(Value::U16(Ref::new(*v.borrow() as u16))),
    #[cfg(all(feature = "f32", feature = "u32"))]
    (Value::F32(v), ValueKind::U32) => Some(Value::U32(Ref::new(*v.borrow() as u32))),
    #[cfg(all(feature = "f32", feature = "u64"))]
    (Value::F32(v), ValueKind::U64) => Some(Value::U64(Ref::new(*v.borrow() as u64))),
    #[cfg(all(feature = "f32", feature = "u128"))]
    (Value::F32(v), ValueKind::U128) => Some(Value::U128(Ref::new(*v.borrow() as u128))),
    #[cfg(all(feature = "f64", feature = "i8"))]
    (Value::F64(v), ValueKind::I8) => Some(Value::I8(Ref::new(*v.borrow() as i8))),
    #[cfg(all(feature = "f64", feature = "i16"))]
    (Value::F64(v), ValueKind::I16) => Some(Value::I16(Ref::new(*v.borrow() as i16))),
    #[cfg(all(feature = "f64", feature = "i32"))]
    (Value::F64(v), ValueKind::I32) => Some(Value::I32(Ref::new(*v.borrow() as i32))),
    #[cfg(all(feature = "f64", feature = "i64"))]
    (Value::F64(v), ValueKind::I64) => Some(Value::I64(Ref::new(*v.borrow() as i64))),
    #[cfg(all(feature = "f64", feature = "i128"))]
    (Value::F64(v), ValueKind::I128) => Some(Value::I128(Ref::new(*v.borrow() as i128))),
    #[cfg(all(feature = "f64", feature = "u8"))]
    (Value::F64(v), ValueKind::U8) => Some(Value::U8(Ref::new(*v.borrow() as u8))),
    #[cfg(all(feature = "f64", feature = "u16"))]
    (Value::F64(v), ValueKind::U16) => Some(Value::U16(Ref::new(*v.borrow() as u16))),
    #[cfg(all(feature = "f64", feature = "u32"))]
    (Value::F64(v), ValueKind::U32) => Some(Value::U32(Ref::new(*v.borrow() as u32))),
    #[cfg(all(feature = "f64", feature = "u64"))]
    (Value::F64(v), ValueKind::U64) => Some(Value::U64(Ref::new(*v.borrow() as u64))),
    #[cfg(all(feature = "f64", feature = "u128"))]
    (Value::F64(v), ValueKind::U128) => Some(Value::U128(Ref::new(*v.borrow() as u128))),

      /*
      // ==== INDEX conversions ====
      (Value::Index(i), U32) => Some(Value::U32(Ref::new((*i.borrow()) as u32))),
      (Value::U32(v), Index) => Some(Value::Index(Ref::new((*v.borrow()) as usize))),


      // ==== MATRIX conversions (element-wise) ====
      (Value::MatrixU8(m), MatrixU16) => Some(Value::MatrixU16(m.map(|x| *x as u16))),
      (Value::MatrixI32(m), MatrixF64) => Some(Value::MatrixF64(m.map(|x| (*x) as f64))),
      // You can expand other matrix conversions similarly...

      // ==== COMPLEX TYPES (stubs) ====
      (Value::Set(set), Set(_)) => Some(Value::Set(set.clone())), // TODO: element-wise convert
      (Value::Map(map), Map(_)) => Some(Value::Map(map.clone())), // TODO: key/value convert
      (Value::Record(r), Record(_)) => Some(Value::Record(r.clone())), // TODO: field convert
      (Value::Table(t), Table(_)) => Some(Value::Table(t.clone())), // TODO: column convert

      // ==== ENUM, KIND ====
      (Value::Enum(e), Enum(_)) => Some(Value::Enum(e.clone())),
      (Value::Kind(k), Kind(_)) => Some(Value::Kind(k.clone())),

      // ==== SPECIAL CASES ====
      (Value::IndexAll, IndexAll) => Some(Value::IndexAll),
      (Value::Empty, Empty) => Some(Value::Empty),
      */
      // ==== FALLBACK ====
      _ => None,
    }
  }

  pub fn size_of(&self) -> usize {
    match self {
      #[cfg(feature = "rational")]
      Value::R64(x) => 16,
      #[cfg(feature = "u8")]
      Value::U8(x) => 1,
      #[cfg(feature = "u16")]
      Value::U16(x) => 2,
      #[cfg(feature = "u32")]
      Value::U32(x) => 4,
      #[cfg(feature = "u64")]
      Value::U64(x) => 8,
      #[cfg(feature = "u128")]
      Value::U128(x) => 16,
      #[cfg(feature = "i8")]
      Value::I8(x) => 1,
      #[cfg(feature = "i16")]
      Value::I16(x) => 2,
      #[cfg(feature = "i32")]
      Value::I32(x) => 4,
      #[cfg(feature = "i64")]
      Value::I64(x) => 8,
      #[cfg(feature = "i128")]
      Value::I128(x) => 16,
      #[cfg(feature = "f32")]
      Value::F32(x) => 4,
      #[cfg(feature = "f64")]
      Value::F64(x) => 8,
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(x) => 1,
      #[cfg(feature = "complex")]
      Value::C64(x) => 16,
      #[cfg(all(feature = "matrix"))]
      Value::MatrixIndex(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x)   => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x)   => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x)  => x.size_of(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x)  => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => x.size_of(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => x.size_of(),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(x) => x.borrow().len(),
      #[cfg(feature = "atom")]
      Value::Atom(x) => 8,
      #[cfg(feature = "set")]
      Value::Set(x) => x.borrow().size_of(),
      #[cfg(feature = "map")]
      Value::Map(x) => x.borrow().size_of(),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().size_of(),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().size_of(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.borrow().size_of(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.borrow().size_of(),
      Value::MutableReference(x) => x.borrow().size_of(),
      Value::Id(_) => 8,
      Value::Index(x) => 8,
      Value::Kind(_) => 0, // Kind is not a value, so it has no size
      Value::Empty => 0,
      Value::IndexAll => 0, // IndexAll is a special value, so it has no size
    }
  }

  #[cfg(feature = "pretty_print")]
  pub fn to_html(&self) -> String {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u16")]
      Value::U16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u32")]
      Value::U32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "u64")]
      Value::U64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i8")]
      Value::I8(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i128")]
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i16")]
      Value::I16(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i32")]
      Value::I32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i64")]
      Value::I64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "i128")]
      Value::I128(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "f32")]
      Value::F32(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(feature = "f64")]
      Value::F64(n) => format!("<span class='mech-number'>{}</span>", n.borrow()),
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(s) => format!("<span class='mech-string'>\"{}\"</span>", s.borrow()),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(b) => format!("<span class='mech-boolean'>{}</span>", b.borrow()),
      #[cfg(feature = "complex")]
      Value::C64(c) => c.borrow().to_html(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(m) => m.to_html(),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(m) => m.to_html(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(m) => m.to_html(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(m) => m.to_html(),
      #[cfg(feature = "atom")]
      Value::Atom(a) => format!("<span class=\"mech-atom\"><span class=\"mech-atom-grave\">`</span><span class=\"mech-atom-name\">{}</span></span>",a.borrow()),
      #[cfg(feature = "set")]
      Value::Set(s) => s.borrow().to_html(),
      #[cfg(feature = "map")]
      Value::Map(m) => m.borrow().to_html(),
      #[cfg(feature = "table")]
      Value::Table(t) => t.borrow().to_html(),
      #[cfg(feature = "record")]
      Value::Record(r) => r.borrow().to_html(),
      #[cfg(feature = "tuple")]
      Value::Tuple(t) => t.borrow().to_html(),
      #[cfg(feature = "enum")]
      Value::Enum(e) => e.borrow().to_html(),
      Value::MutableReference(m) => {
        let inner = m.borrow();
        format!("<span class='mech-reference'>{}</span>", inner.to_html())
      },
      _ => "???".to_string(),
    }
  }

  pub fn shape(&self) -> Vec<usize> {
    match self {
      #[cfg(feature = "rational")]
      Value::R64(x) => vec![1,1],
      #[cfg(feature = "complex")]
      Value::C64(x) => vec![1,1],
      #[cfg(feature = "u8")]
      Value::U8(x) => vec![1,1],
      #[cfg(feature = "u16")]
      Value::U16(x) => vec![1,1],
      #[cfg(feature = "u32")]
      Value::U32(x) => vec![1,1],
      #[cfg(feature = "u64")]
      Value::U64(x) => vec![1,1],
      #[cfg(feature = "u128")]
      Value::U128(x) => vec![1,1],
      #[cfg(feature = "i8")]
      Value::I8(x) => vec![1,1],
      #[cfg(feature = "i16")]
      Value::I16(x) => vec![1,1],
      #[cfg(feature = "i32")]
      Value::I32(x) => vec![1,1],
      #[cfg(feature = "i64")]
      Value::I64(x) => vec![1,1],
      #[cfg(feature = "i128")]
      Value::I128(x) => vec![1,1],
      #[cfg(feature = "f32")]
      Value::F32(x) => vec![1,1],
      #[cfg(feature = "f64")]
      Value::F64(x) => vec![1,1],
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(x) => vec![1,1],
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(x) => vec![1,1],
      #[cfg(feature = "atom")]
      Value::Atom(x) => vec![1,1],
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.shape(),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => x.shape(),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => x.shape(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => vec![1,1],
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().shape(),
      #[cfg(feature = "set")]
      Value::Set(x) => vec![1,x.borrow().set.len()],
      #[cfg(feature = "map")]
      Value::Map(x) => vec![1,x.borrow().map.len()],
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().shape(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => vec![1,x.borrow().size()],
      Value::Index(x) => vec![1,1],
      Value::MutableReference(x) => x.borrow().shape(),
      Value::Empty => vec![0,0],
      Value::IndexAll => vec![0,0],
      Value::Kind(_) => vec![0,0],
      Value::Id(x) => vec![0,0],
    }
  }

  pub fn deref_kind(&self) -> ValueKind {
    match self {
      Value::MutableReference(x) => x.borrow().kind(),
      x => x.kind(),
    }
  }

  pub fn kind(&self) -> ValueKind {
    match self {
      #[cfg(feature = "complex")]
      Value::C64(_) => ValueKind::C64,
      #[cfg(feature = "rational")]
      Value::R64(_) => ValueKind::R64,
      #[cfg(feature = "u8")]
      Value::U8(_) => ValueKind::U8,
      #[cfg(feature = "u16")]
      Value::U16(_) => ValueKind::U16,
      #[cfg(feature = "u32")]
      Value::U32(_) => ValueKind::U32,
      #[cfg(feature = "u64")]
      Value::U64(_) => ValueKind::U64,
      #[cfg(feature = "u128")]
      Value::U128(_) => ValueKind::U128,
      #[cfg(feature = "i8")]
      Value::I8(_) => ValueKind::I8,
      #[cfg(feature = "i16")]
      Value::I16(_) => ValueKind::I16,
      #[cfg(feature = "i32")]
      Value::I32(_) => ValueKind::I32,
      #[cfg(feature = "i64")]
      Value::I64(_) => ValueKind::I64,
      #[cfg(feature = "i128")]
      Value::I128(_) => ValueKind::I128,
      #[cfg(feature = "f32")]
      Value::F32(_) => ValueKind::F32,
      #[cfg(feature = "f64")]
      Value::F64(_) => ValueKind::F64,
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(_) => ValueKind::String,
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(_) => ValueKind::Bool,
      #[cfg(feature = "atom")]
      Value::Atom(x) => ValueKind::Atom((x.borrow().0)),
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x) => ValueKind::Matrix(Box::new(ValueKind::Any),x.shape()),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => ValueKind::Matrix(Box::new(ValueKind::Index),x.shape()),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => ValueKind::Matrix(Box::new(ValueKind::Bool), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x) => ValueKind::Matrix(Box::new(ValueKind::U8), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x) => ValueKind::Matrix(Box::new(ValueKind::U16), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x) => ValueKind::Matrix(Box::new(ValueKind::U32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x) => ValueKind::Matrix(Box::new(ValueKind::U64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => ValueKind::Matrix(Box::new(ValueKind::U128), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x) => ValueKind::Matrix(Box::new(ValueKind::I8), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x) => ValueKind::Matrix(Box::new(ValueKind::I16), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x) => ValueKind::Matrix(Box::new(ValueKind::I32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x) => ValueKind::Matrix(Box::new(ValueKind::I64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => ValueKind::Matrix(Box::new(ValueKind::I128), x.shape()),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x) => ValueKind::Matrix(Box::new(ValueKind::F32), x.shape()),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => ValueKind::Matrix(Box::new(ValueKind::F64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => ValueKind::Matrix(Box::new(ValueKind::String), x.shape()),
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => ValueKind::Matrix(Box::new(ValueKind::R64), x.shape()),
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => ValueKind::Matrix(Box::new(ValueKind::C64), x.shape()),
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().kind(),
      #[cfg(feature = "set")]
      Value::Set(x) => x.borrow().kind(),
      #[cfg(feature = "map")]
      Value::Map(x) => x.borrow().kind(),
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().kind(),
      #[cfg(feature = "tuple")]
      Value::Tuple(x) => x.borrow().kind(),
      #[cfg(feature = "enum")]
      Value::Enum(x) => x.borrow().kind(),
      Value::MutableReference(x) => ValueKind::Reference(Box::new(x.borrow().kind())),
      Value::Empty => ValueKind::Empty,
      Value::IndexAll => ValueKind::Empty,
      Value::Id(x) => ValueKind::Id,
      Value::Index(x) => ValueKind::Index,
      Value::Kind(x) => x.clone(),
    }
  }

  #[cfg(feature = "matrix")]
  pub fn is_matrix(&self) -> bool {
    match self {
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(_) => true,
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(_) => true,
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(_) => true,
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(_) => true,
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(_) => true,
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(_) => true,
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(_) => true,
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(_) => true,
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(_) => true,
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(_) => true,
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(_) => true,
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(_) => true,
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(_) => true,
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(_) => true,
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(_) => true,
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(_) => true,
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(_) => true,
      #[cfg(feature = "matrix")]
      Value::MatrixValue(_) => true,
      _ => false,
    }
  }

  pub fn is_scalar(&self) -> bool {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(_) => true,
      #[cfg(feature = "u16")]
      Value::U16(_) => true,
      #[cfg(feature = "u32")]
      Value::U32(_) => true,
      #[cfg(feature = "u64")]
      Value::U64(_) => true,
      #[cfg(feature = "u128")]
      Value::U128(_) => true,
      #[cfg(feature = "i8")]
      Value::I8(_) => true,
      #[cfg(feature = "i16")]
      Value::I16(_) => true,
      #[cfg(feature = "i32")]
      Value::I32(_) => true,
      #[cfg(feature = "i64")]
      Value::I64(_) => true,
      #[cfg(feature = "i128")]
      Value::I128(_) => true,
      #[cfg(feature = "f32")]
      Value::F32(_) => true,
      #[cfg(feature = "f64")]
      Value::F64(_) => true,
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(_) => true,
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(_) => true,
      #[cfg(feature = "atom")]
      Value::Atom(_) => true,
      Value::Index(_) => true,
      _ => false,
    }
  }

  #[cfg(any(feature = "bool", feature = "variable_define"))]
  pub fn as_bool(&self) -> MResult<Ref<bool>> {
    if let Value::Bool(v) = self {
      Ok(v.clone())
    } else if let Value::MutableReference(val) = self {
      val.borrow().as_bool()
    } else {
      Err(MechError2::new(
        UnhandledFunctionArgumentKindError,
        None
      ).with_compiler_loc())
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

  #[cfg(any(feature = "string", feature = "variable_define"))]
  pub fn as_string(&self) -> MResult<Ref<String>> {
    match self {
      Value::String(v) => Ok(v.clone()),
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok(Ref::new(format!("{}", v.borrow()))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok(Ref::new(format!("{}", v.borrow()))),
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(v) => Ok(Ref::new(format!("{}", v.borrow()))),
      #[cfg(feature = "rational")]
      Value::R64(v) => Ok(Ref::new(v.borrow().to_string())),
      #[cfg(feature = "complex")]
      Value::C64(v) => Ok(Ref::new(v.borrow().to_string())),
      Value::MutableReference(val) => val.borrow().as_string(),
      _ => Err(
        MechError2::new(
          CannotConvertToTypeError { 
            target_type: "string",
          },
          None
        ).with_compiler_loc()
      ),
    }
  }

  #[cfg(feature = "r64")]
  pub fn as_r64(&self) -> MResult<Ref<R64>> {
    match self {
      Value::R64(v) => Ok(v.clone()),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(Ref::new(R64::new(*v.borrow() as i64, 1))),
      Value::MutableReference(val) => val.borrow().as_r64(),
      _ => Err(
        MechError2::new(
          CannotConvertToTypeError { 
            target_type: "r64",
          },
          None
        ).with_compiler_loc()
      ),
    }
  }

  #[cfg(feature = "c64")]
  pub fn as_c64(&self) -> MResult<Ref<C64>> {
    match self {
      Value::C64(v) => Ok(v.clone()),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok(Ref::new(C64::new(*v.borrow(), 0.0))),
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(Ref::new(C64::new(*v.borrow() as f64, 0.0))),
      Value::MutableReference(val) => val.borrow().as_c64(),
      _ => Err(
        MechError2::new(
          CannotConvertToTypeError { 
            target_type: "c64",
          },
          None
        ).with_compiler_loc()
      ),
    }
  }

  #[cfg(feature = "f32")]
  pub fn as_f32(&self) -> MResult<Ref<f32>> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(Ref::new(*v.borrow() as f32)),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(Ref::new(*v.borrow() as f32)),
      Value::F32(v) => Ok(v.clone()),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok(Ref::new((*v.borrow()) as f32)),
      Value::MutableReference(val) => val.borrow().as_f32(),
      _ => Err(
        MechError2::new(
          CannotConvertToTypeError { 
            target_type: "f32",
          },
          None
        ).with_compiler_loc()
      ),
    }
  }

  #[cfg(feature = "f64")]
  pub fn as_f64(&self) -> MResult<Ref<f64>> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(Ref::new(*v.borrow() as f64)),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok(Ref::new((*v.borrow()) as f64)),
      Value::F64(v) => Ok(v.clone()),
      Value::MutableReference(val) => val.borrow().as_f64(),
      _ => Err(
        MechError2::new(
          CannotConvertToTypeError { 
            target_type: "f64",
          },
          None
        ).with_compiler_loc()
      ),
    }
  }

  #[cfg(all(feature = "matrix", feature = "bool"))] pub fn as_vecbool(&self) -> MResult<Vec<bool>> { if let Value::MatrixBool(v) = self { Ok(v.as_vec()) } else if let Value::Bool(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecbool() } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "bool" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "f64"))] pub fn as_vecf64(&self) -> MResult<Vec<f64>> { if let Value::MatrixF64(v) = self { Ok(v.as_vec()) } else if let Value::F64(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf64() } else if let Ok(v) = self.as_f64() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "f64" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "f32"))] pub fn as_vecf32(&self) -> MResult<Vec<f32>> { if let Value::MatrixF32(v) = self { Ok(v.as_vec()) } else if let Value::F32(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecf32() } else if let Ok(v) = self.as_f32() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "f32" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "u8"))] pub fn as_vecu8(&self) -> MResult<Vec<u8>> { if let Value::MatrixU8(v) = self { Ok(v.as_vec()) } else if let Value::U8(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu8() } else if let Ok(v) = self.as_u8() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "u8" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "u16"))] pub fn as_vecu16(&self) -> MResult<Vec<u16>> { if let Value::MatrixU16(v) = self { Ok(v.as_vec()) } else if let Value::U16(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu16() } else if let Ok(v) = self.as_u16() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "u16" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "u32"))] pub fn as_vecu32(&self) -> MResult<Vec<u32>> { if let Value::MatrixU32(v) = self { Ok(v.as_vec()) } else if let Value::U32(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu32() } else if let Ok(v) = self.as_u32() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "u32" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "u64"))] pub fn as_vecu64(&self) -> MResult<Vec<u64>> { if let Value::MatrixU64(v) = self { Ok(v.as_vec()) } else if let Value::U64(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu64() } else if let Ok(v) = self.as_u64() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "u64" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "u128"))] pub fn as_vecu128(&self) -> MResult<Vec<u128>> { if let Value::MatrixU128(v) = self { Ok(v.as_vec()) } else if let Value::U128(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecu128() } else if let Ok(v) = self.as_u128() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "u128" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "i8"))] pub fn as_veci8(&self) -> MResult<Vec<i8>> { if let Value::MatrixI8(v) = self { Ok(v.as_vec()) } else if let Value::I8(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci8() } else if let Ok(v) = self.as_i8() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "i8" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "i16"))] pub fn as_veci16(&self) -> MResult<Vec<i16>> { if let Value::MatrixI16(v) = self { Ok(v.as_vec()) } else if let Value::I16(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci16() } else if let Ok(v) = self.as_i16() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "i16" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "i32"))] pub fn as_veci32(&self) -> MResult<Vec<i32>> { if let Value::MatrixI32(v) = self { Ok(v.as_vec()) } else if let Value::I32(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci32() } else if let Ok(v) = self.as_i32() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "i32" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "i64"))] pub fn as_veci64(&self) -> MResult<Vec<i64>> { if let Value::MatrixI64(v) = self { Ok(v.as_vec()) } else if let Value::I64(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci64() } else if let Ok(v) = self.as_i64() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "i64" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "i128"))] pub fn as_veci128(&self) -> MResult<Vec<i128>> { if let Value::MatrixI128(v) = self { Ok(v.as_vec()) } else if let Value::I128(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_veci128() } else if let Ok(v) = self.as_i128() { Ok(vec![v.borrow().clone()]) } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "i128" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "string"))] pub fn as_vecstring(&self) -> MResult<Vec<String>> { if let Value::MatrixString(v) = self { Ok(v.as_vec()) } else if let Value::String(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecstring() } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "string" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "r64"))] pub fn as_vecr64(&self) -> MResult<Vec<R64>> { if let Value::MatrixR64(v) = self { Ok(v.as_vec()) } else if let Value::R64(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecr64() } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "r64" }, None).with_compiler_loc()) } }
  #[cfg(all(feature = "matrix", feature = "c64"))] pub fn as_vecc64(&self) -> MResult<Vec<C64>> { if let Value::MatrixC64(v) = self { Ok(v.as_vec()) } else if let Value::C64(v) = self { Ok(vec![v.borrow().clone()]) } else if let Value::MutableReference(val) = self { val.borrow().as_vecc64() } else { Err(MechError2::new(CannotConvertToTypeError { target_type: "c64" }, None).with_compiler_loc()) } }

  pub fn as_vecusize(&self) -> MResult<Vec<usize>> {
    match self {
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(vec![*v.borrow() as usize]),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok(vec![(*v.borrow()) as usize]),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok(vec![(*v.borrow()) as usize]),
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(v) => Ok(v.as_vec()),
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(v) => Ok(v.as_vec().iter().map(|x| (*x) as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(v) => Ok(v.as_vec().iter().map(|x| (*x) as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "u16"))]  
      Value::MatrixU16(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(v) => Ok(v.as_vec().iter().map(|x| *x as usize).collect::<Vec<usize>>()),
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(_) =>
        Err(MechError2::new(
          CannotConvertToTypeError { target_type: "[usize]" },
          None
        ).with_compiler_loc()),
      #[cfg(any(feature = "bool", feature = "[usize]"))]
      Value::Bool(_) =>
        Err(MechError2::new(
          CannotConvertToTypeError { target_type: "[usize]" },
          None
        ).with_compiler_loc()),
      Value::MutableReference(x) => x.borrow().as_vecusize(),
      _ =>
        Err(MechError2::new(
          CannotConvertToTypeError { target_type: "[usize]" },
          None
        ).with_compiler_loc()),
    }
  }


    pub fn as_index(&self) -> MResult<Value> {
    match self.as_usize() {      
      Ok(ix) => Ok(Value::Index(Ref::new(ix))),
      #[cfg(feature = "matrix")]
      Err(_) => match self.as_vecusize() {
        #[cfg(feature = "matrix")]
        Ok(x) => {
          let shape = self.shape();
          let out = Value::MatrixIndex(usize::to_matrix(x, shape[0] * shape[1],1 ));
          Ok(out)
        },
        #[cfg(all(feature = "matrix", feature = "bool"))]
        Err(_) => match self.as_vecbool() {
          Ok(x) => {
            let shape = self.shape();
            let out = match (shape[0], shape[1]) {
              (1,1) => Value::Bool(Ref::new(x[0])),
              #[cfg(all(feature = "vectord", feature = "bool"))]
              (1,n) => Value::MatrixBool(Matrix::DVector(Ref::new(DVector::from_vec(x)))),
              #[cfg(all(feature = "vectord", feature = "bool"))]
              (m,1) => Value::MatrixBool(Matrix::DVector(Ref::new(DVector::from_vec(x)))),
              #[cfg(all(feature = "vectord", feature = "bool"))]
              (m,n) => Value::MatrixBool(Matrix::DVector(Ref::new(DVector::from_vec(x)))),
              _ => todo!(),
            };
            Ok(out)
          }
          Err(_) => match self.as_bool() {
            Ok(x) => Ok(Value::Bool(x)),
            Err(_) => Err(MechError2::new(
              CannotConvertToTypeError { target_type: "ix" },
              None
            ).with_compiler_loc()),
          }
        }
        x => Err(MechError2::new(
          CannotConvertToTypeError { target_type: "ix" },
          None
        ).with_compiler_loc()),
      }
      _ => todo!(),
    }
  }

  pub fn as_usize(&self) -> MResult<usize> {
    match self {      
      Value::Index(v) => Ok(*v.borrow()),
      #[cfg(feature = "u8")]
      Value::U8(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "u16")]
      Value::U16(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "u32")]
      Value::U32(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "u64")]
      Value::U64(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "u128")]
      Value::U128(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "i8")]
      Value::I8(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "i16")]
      Value::I16(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "i32")]
      Value::I32(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "i64")]
      Value::I64(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "i128")]
      Value::I128(v) => Ok(*v.borrow() as usize),
      #[cfg(feature = "f32")]
      Value::F32(v) => Ok((*v.borrow()) as usize),
      #[cfg(feature = "f64")]
      Value::F64(v) => Ok((*v.borrow()) as usize),
      Value::MutableReference(v) => v.borrow().as_usize(),
      _ =>
        Err(
          MechError2::new(
            CannotConvertToTypeError { target_type: "usize" },
            None
          ).with_compiler_loc()
        ),
    }
  }

  #[cfg(feature = "u8")]
  pub fn expect_u8(&self) -> MResult<Ref<u8>> {
    match self {
      Value::U8(v) => Ok(v.clone()),
      Value::MutableReference(v) => v.borrow().expect_u8(),
      _ =>
        Err(
          MechError2::new(
            CannotConvertToTypeError { target_type: "u8" },
            None
          ).with_compiler_loc()
        ),
    }
  }

  #[cfg(feature = "f64")]
  pub fn expect_f64(&self) -> MResult<Ref<f64>> {
    match self {
      Value::F64(v) => Ok(v.clone()),
      Value::MutableReference(v) => v.borrow().expect_f64(),
      _ =>
        Err(
          MechError2::new(
            CannotConvertToTypeError { target_type: "f64" },
            None
          ).with_compiler_loc()
        ),
    }
  }

}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Value {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();
    match self {
      #[cfg(feature = "u8")]
      Value::U8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u16")]
      Value::U16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u32")]
      Value::U32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u64")]
      Value::U64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "u128")]
      Value::U128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i8")]
      Value::I8(x)   => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i16")]
      Value::I16(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i32")]
      Value::I32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i64")]
      Value::I64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "i128")]
      Value::I128(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "f32")]
      Value::F32(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "f64")]
      Value::F64(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(any(feature = "bool", feature = "variable_define"))]
      Value::Bool(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "complex")]
      Value::C64(x) => {builder.push_record(vec![x.borrow().pretty_print()]);},
      #[cfg(feature = "rational")]
      Value::R64(x) => {builder.push_record(vec![format!("{}",x.borrow().pretty_print())]);},
      #[cfg(feature = "atom")]
      Value::Atom(x) => {builder.push_record(vec![format!("{}",x.borrow())]);},
      #[cfg(feature = "set")]
      Value::Set(x)  => {return x.borrow().pretty_print();}
      #[cfg(feature = "map")]
      Value::Map(x)  => {return x.borrow().pretty_print();}
      #[cfg(any(feature = "string", feature = "variable_define"))]
      Value::String(x) => {return format!("\"{}\"",x.borrow().clone());},
      #[cfg(feature = "table")]
      Value::Table(x)  => {return x.borrow().pretty_print();},
      #[cfg(feature = "tuple")]
      Value::Tuple(x)  => {return x.borrow().pretty_print();},
      #[cfg(feature = "record")]
      Value::Record(x) => {return x.borrow().pretty_print();},
      #[cfg(feature = "enum")]
      Value::Enum(x) => {return x.borrow().pretty_print();},
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x)   => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x)   => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "any"))]
      Value::MatrixValue(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x)  => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => {return x.pretty_print();},
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => {return x.pretty_print();},
      Value::Index(x)  => {builder.push_record(vec![format!("{}",x.borrow())]);},
      Value::MutableReference(x) => {return x.borrow().pretty_print();},
      Value::Empty => builder.push_record(vec!["_"]),
      Value::IndexAll => builder.push_record(vec![":"]),
      Value::Id(x) => builder.push_record(vec![format!("{}",humanize(x))]),
      Value::Kind(x) => builder.push_record(vec![format!("{}",x)]),
      x => {
        todo!("{x:#?}");
      },
    };
    let value_style = Style::empty()
      .top(' ')
      .left(' ')
      .right(' ')
      .bottom(' ')
      .vertical(' ')
      .intersection_bottom(' ')
      .corner_top_left(' ')
      .corner_top_right(' ')
      .corner_bottom_left(' ')
      .corner_bottom_right(' ');
    let mut table = builder.build();
    table.with(value_style);
    format!("{table}")
  }
}


pub trait ToIndex {
  fn to_index(&self) -> Value;
}

#[cfg(feature = "matrix")]
impl ToIndex for Ref<Vec<i64>> { fn to_index(&self) -> Value { (*self.borrow()).iter().map(|x| *x as usize).collect::<Vec<usize>>().to_value() } }

pub trait ToValue {
  fn to_value(&self) -> Value;
}

#[cfg(feature = "matrix")]
impl ToValue for Vec<usize> {
  fn to_value(&self) -> Value {
    match self.len() {
      1 => Value::Index(Ref::new(self[0].clone())),
      #[cfg(feature = "vector2")]
      2 => Value::MatrixIndex(Matrix::Vector2(Ref::new(Vector2::from_vec(self.clone())))),
      #[cfg(feature = "vector3")]
      3 => Value::MatrixIndex(Matrix::Vector3(Ref::new(Vector3::from_vec(self.clone())))),
      #[cfg(feature = "vector4")]
      4 => Value::MatrixIndex(Matrix::Vector4(Ref::new(Vector4::from_vec(self.clone())))),
      #[cfg(feature = "vectord")]
      n => Value::MatrixIndex(Matrix::DVector(Ref::new(DVector::from_vec(self.clone())))),
      _ => todo!(),
    }
  }
}

impl ToValue for Ref<usize>  { fn to_value(&self) -> Value { Value::Index(self.clone())  } }
#[cfg(feature = "u8")]
impl ToValue for Ref<u8>     { fn to_value(&self) -> Value { Value::U8(self.clone())     } }
#[cfg(feature = "u16")]
impl ToValue for Ref<u16>    { fn to_value(&self) -> Value { Value::U16(self.clone())    } }
#[cfg(feature = "u32")]
impl ToValue for Ref<u32>    { fn to_value(&self) -> Value { Value::U32(self.clone())    } }
#[cfg(feature = "u64")]
impl ToValue for Ref<u64>    { fn to_value(&self) -> Value { Value::U64(self.clone())    } }
#[cfg(feature = "u128")]
impl ToValue for Ref<u128>   { fn to_value(&self) -> Value { Value::U128(self.clone())   } }
#[cfg(feature = "i8")]
impl ToValue for Ref<i8>     { fn to_value(&self) -> Value { Value::I8(self.clone())     } }
#[cfg(feature = "i16")]
impl ToValue for Ref<i16>    { fn to_value(&self) -> Value { Value::I16(self.clone())    } }
#[cfg(feature = "i32")]
impl ToValue for Ref<i32>    { fn to_value(&self) -> Value { Value::I32(self.clone())    } }
#[cfg(feature = "i64")]
impl ToValue for Ref<i64>    { fn to_value(&self) -> Value { Value::I64(self.clone())    } }
#[cfg(feature = "i128")]
impl ToValue for Ref<i128>   { fn to_value(&self) -> Value { Value::I128(self.clone())   } }
#[cfg(feature = "f32")]
impl ToValue for Ref<f32>    { fn to_value(&self) -> Value { Value::F32(self.clone())    } }
#[cfg(feature = "f64")]
impl ToValue for Ref<f64>    { fn to_value(&self) -> Value { Value::F64(self.clone())    } }
#[cfg(any(feature = "bool", feature = "variable_define"))]
impl ToValue for Ref<bool>   { fn to_value(&self) -> Value { Value::Bool(self.clone())   } }
#[cfg(any(feature = "string", feature = "variable_define"))]
impl ToValue for Ref<String> { fn to_value(&self) -> Value { Value::String(self.clone()) } }
#[cfg(feature = "rational")]
impl ToValue for Ref<R64> { fn to_value(&self) -> Value { Value::R64(self.clone()) } }
#[cfg(feature = "complex")]
impl ToValue for Ref<C64> { fn to_value(&self) -> Value { Value::C64(self.clone()) } }

impl ToValue for Ref<Value> { fn to_value(&self) -> Value { (*self.borrow()).clone() } }

#[cfg(feature = "u8")]
impl From<u8> for Value {
  fn from(val: u8) -> Self {
    Value::U8(Ref::new(val))
  }
}

#[cfg(feature = "u16")]
impl From<u16> for Value {
  fn from(val: u16) -> Self {
    Value::U16(Ref::new(val))
  }
}

#[cfg(feature = "u32")]
impl From<u32> for Value {
  fn from(val: u32) -> Self {
    Value::U32(Ref::new(val))
  }
}

#[cfg(feature = "u64")]
impl From<u64> for Value {
  fn from(val: u64) -> Self {
    Value::U64(Ref::new(val))
  }
}

#[cfg(feature = "u128")]
impl From<u128> for Value {
  fn from(val: u128) -> Self {
    Value::U128(Ref::new(val))
  }
}

#[cfg(feature = "i8")]
impl From<i8> for Value {
  fn from(val: i8) -> Self {
    Value::I8(Ref::new(val))
  }
}

#[cfg(feature = "i16")]
impl From<i16> for Value {
  fn from(val: i16) -> Self {
    Value::I16(Ref::new(val))
  }
}

#[cfg(feature = "i32")]
impl From<i32> for Value {
  fn from(val: i32) -> Self {
    Value::I32(Ref::new(val))
  }
}

#[cfg(feature = "i64")]
impl From<i64> for Value {
  fn from(val: i64) -> Self {
    Value::I64(Ref::new(val))
  }
}

#[cfg(feature = "i128")]
impl From<i128> for Value {
  fn from(val: i128) -> Self {
    Value::I128(Ref::new(val))
  }
}

#[cfg(any(feature = "bool", feature = "variable_define"))]
impl From<bool> for Value {
  fn from(val: bool) -> Self {
    Value::Bool(Ref::new(val))
  }
}

#[cfg(any(feature = "string", feature = "variable_define"))]
impl From<String> for Value {
  fn from(val: String) -> Self {
    Value::String(Ref::new(val))
  }
}

#[cfg(feature = "rational")]
impl From<R64> for Value {
  fn from(val: R64) -> Self {
    Value::R64(Ref::new(val))
  }
}


pub trait ToUsize {
  fn to_usize(&self) -> usize;
}

macro_rules! impl_to_usize_for {
  ($t:ty) => {
    impl ToUsize for $t {
      fn to_usize(&self) -> usize {
        #[allow(unused_comparisons)]
        if *self < 0 as $t {
          panic!("Cannot convert negative number to usize");
        }
        *self as usize
      }
    }
  };
}

#[cfg(feature = "u8")]
impl_to_usize_for!(u8);
#[cfg(feature = "u16")]
impl_to_usize_for!(u16);
#[cfg(feature = "u32")]
impl_to_usize_for!(u32);
#[cfg(feature = "u64")]
impl_to_usize_for!(u64);
#[cfg(feature = "u128")]
impl_to_usize_for!(u128);
impl_to_usize_for!(usize);

#[cfg(feature = "i8")]
impl_to_usize_for!(i8);
#[cfg(feature = "i16")]
impl_to_usize_for!(i16);
#[cfg(feature = "i32")]
impl_to_usize_for!(i32);
#[cfg(feature = "i64")]
impl_to_usize_for!(i64);
#[cfg(feature = "i128")]
impl_to_usize_for!(i128);

#[cfg(feature = "f64")]
impl_to_usize_for!(f64);
#[cfg(feature = "f32")]
impl_to_usize_for!(f32);

#[cfg(feature = "table")]
impl ToValue for Ref<MechTable> {
  fn to_value(&self) -> Value {
    Value::Table(self.clone())
  }
}

#[cfg(feature = "set")]
impl ToValue for Ref<MechSet> {
  fn to_value(&self) -> Value {
    Value::Set(self.clone())
  }
}

#[cfg(feature = "map")]
impl ToValue for Ref<MechMap> {
  fn to_value(&self) -> Value {
    Value::Map(self.clone())
  }
}

#[cfg(feature = "tuple")]
impl ToValue for Ref<MechTuple> {
  fn to_value(&self) -> Value {
    Value::Tuple(self.clone())
  }
}

#[cfg(feature = "record")]
impl ToValue for Ref<MechRecord> {
  fn to_value(&self) -> Value {
    Value::Record(self.clone())
  }
}

// Errors

#[derive(Debug, Clone)]
pub struct UnhandledFunctionArgumentKindError;

impl MechErrorKind2 for UnhandledFunctionArgumentKindError {
  fn name(&self) -> &str { "UnhandledFunctionArgumentKind" }
  fn message(&self) -> String {
    "Value kind is not valid for this function.".to_string()
  }
}

#[derive(Debug, Clone)]
pub struct CannotConvertToTypeError {
  pub target_type: &'static str,
}

impl MechErrorKind2 for CannotConvertToTypeError {
  fn name(&self) -> &str { "CannotConvertToType" }
  fn message(&self) -> String {
    format!("Cannot convert to {}", self.target_type)
  }
}
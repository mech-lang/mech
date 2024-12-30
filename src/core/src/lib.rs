#![feature(get_mut_unchecked)]
#![feature(concat_idents)]
#![allow(warnings)]
#![feature(iter_intersperse)]
#![feature(extract_if)]
#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
#![feature(step_trait)]

#[cfg(feature="no-std")] #[macro_use] extern crate alloc;
#[cfg(not(feature = "no-std"))] extern crate core;
#[macro_use]
extern crate lazy_static;
extern crate nalgebra as na;
extern crate tabled;
extern crate libm;

#[cfg(feature = "no-std")]
extern crate alloc;
extern crate core as rust_core;
extern crate hashbrown;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate num_traits;
extern crate ed25519_dalek;
extern crate rand;
extern crate getrandom;

extern crate seahash;
extern crate indexmap;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use num_traits::*;
use std::ops::*;
use std::mem;

pub mod value;
pub mod matrix;
pub mod types;
pub mod functions;
pub mod kind;
pub mod error;
pub mod nodes;

pub use self::value::*;
pub use self::matrix::*;
pub use self::types::*;
pub use self::functions::*;
pub use self::kind::*;
pub use self::error::*;
pub use self::nodes::*;

pub fn hash_chars(input: &Vec<char>) -> u64 {
  seahash::hash(input.iter().map(|s| String::from(*s)).collect::<String>().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_bytes(input: &Vec<u8>) -> u64 {
  seahash::hash(input) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_str(input: &str) -> u64 {
  seahash::hash(input.to_string().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn humanize(hash: &u64) -> String {
  let bytes: [u8; 8] = hash.to_be_bytes();
  let mut string = "".to_string();
  let mut ix = 0;
  for byte in bytes.iter() {
    if ix % 2 == 0 {
      ix += 1;
      continue;
    }
    string.push_str(&WORDLIST[*byte as usize]);
    if ix < 7 {
      string.push_str("-");
    }
    ix += 1;
  }
  string
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MechSourceCode {
  String(String),
}

pub const WORDLIST: &[&str;256] = &[
  "nil", "ama", "ine", "ska", "pha", "gel", "art", 
  "ona", "sas", "ist", "aus", "pen", "ust", "umn",
  "ado", "con", "loo", "man", "eer", "lin", "ium",
  "ack", "som", "lue", "ird", "avo", "dog", "ger",
  "ter", "nia", "bon", "nal", "ina", "pet", "cat",
  "ing", "lie", "ken", "fee", "ola", "old", "rad",
  "met", "cut", "azy", "cup", "ota", "dec", "del",
  "elt", "iet", "don", "ble", "ear", "rth", "eas", 
  "war", "eig", "tee", "ele", "emm", "ene", "qua",
  "tst", "fan", "fif", "fil", "fin", "fis", "fiv", 
  "flo", "for", "foo", "fou", "fot", "fox", "fre",
  "fri", "fru", "gee", "gia", "glu", "fol", "gre", 
  "ham", "hap", "har", "haw", "hel", "hig", "hot", 
  "hyd", "ida", "ill", "ind", "ini", "ink", "iwa",
  "and", "ite", "jer", "jig", "joh", "jul", "uly", 
  "kan", "ket", "kil", "kin", "kit", "lac", "lak", 
  "lem", "ard", "lim", "lio", "lit", "lon", "lou",
  "low", "mag", "nes", "mai", "gam", "arc", "mar",
  "mao", "mas", "may", "mex", "mic", "mik", "ril",
  "min", "mir", "mis", "mio", "mob", "moc", "ech",
  "moe", "tan", "oon", "ain", "mup", "sic", "neb",
  "une", "net", "nev", "nin", "een", "nit", "nor",
  "nov", "nut", "oct", "ohi", "okl", "one", "ora",
  "ges", "ore", "osc", "ove", "oxy", "pap", "par", 
  "pey", "pip", "piz", "plu", "pot", "pri", "pur",
  "que", "uqi", "qui", "red", "riv", "rob", "roi", 
  "rug", "sad", "sal", "sat", "sep", "sev", "eve",
  "sha", "sie", "sin", "sik", "six", "sit", "sky", 
  "soc", "sod", "sol", "sot", "tir", "ker", "spr",
  "sta", "ste", "mam", "mer", "swe", "tab", "tag", 
  "see", "nis", "tex", "thi", "the", "tim", "tri",
  "twe", "ent", "two", "unc", "ess", "uni", "ura", 
  "veg", "ven", "ver", "vic", "vid", "vio", "vir",
  "was", "est", "whi", "hit", "iam", "win", "his",
  "wis", "olf", "wyo", "ray", "ank", "yel", "zeb",
  "ulu", "fix", "gry", "hol", "jup", "lam", "pas",
  "rom", "sne", "ten", "uta"];

// ============================================================================
// The Standard Library!
// ============================================================================

#[macro_export]
macro_rules! impl_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
      #[derive(Debug)]
      struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
      }
      impl<T> MechFunction for $struct_name<T>
      where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Zero + One,
      Ref<$out_type>: ToValue
      {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      }};}

#[macro_export]
macro_rules! impl_bool_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let lhs_ptr = self.lhs.as_ptr();
        let rhs_ptr = self.rhs.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

#[macro_export]  
macro_rules! impl_bool_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}

#[macro_export]  
macro_rules! impl_urop {
  ($struct_name:ident, $arg_type:ty, $out_type:ty, $op:ident) => {
    #[derive(Debug)]
    struct $struct_name {
      arg: Ref<$arg_type>,
      out: Ref<$out_type>,
    }
    impl MechFunction for $struct_name {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }};}  

#[macro_export]
macro_rules! impl_fxns {
  ($lib:ident, $in:ident, $out:ident, $op:ident) => {
    paste!{
      // Scalar
      $op!([<$lib SS>], $in, $in, $out, [<$lib:lower _op>]);
      // Scalar Matrix
      #[cfg(feature = "Matrix1")]
      $op!([<$lib SM1>], $in, Matrix1<$in>, Matrix1<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Matrix2")]
      $op!([<$lib SM2>], $in, Matrix2<$in>, Matrix2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Matrix3")]
      $op!([<$lib SM3>], $in, Matrix3<$in>, Matrix3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Matrix4")]
      $op!([<$lib SM4>], $in, Matrix4<$in>, Matrix4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Matrix2x3")]
      $op!([<$lib SM2x3>], $in, Matrix2x3<$in>, Matrix2x3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Matrix3x2")]
      $op!([<$lib SM3x2>], $in, Matrix3x2<$in>, Matrix3x2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Row
      #[cfg(feature = "RowVector2")]
      $op!([<$lib SR2>], $in, RowVector2<$in>, RowVector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "RowVector3")]
      $op!([<$lib SR3>], $in, RowVector3<$in>, RowVector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "RowVector4")]
      $op!([<$lib SR4>], $in, RowVector4<$in>, RowVector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "RowVectorD")]
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Scalar Vector
      #[cfg(feature = "Vector2")]
      $op!([<$lib SV2>], $in, Vector2<$in>, Vector2<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib SV3>], $in, Vector3<$in>, Vector3<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "Vector4")]
      $op!([<$lib SV4>], $in, Vector4<$in>, Vector4<$out>,[<$lib:lower _scalar_rhs_op>]);
      #[cfg(feature = "VectorD")]
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>]);
      // Matrix Scalar
      #[cfg(feature = "Matrix1")]
      $op!([<$lib M1S>], Matrix1<$in>, $in, Matrix1<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Matrix2")]
      $op!([<$lib M2S>], Matrix2<$in>, $in, Matrix2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Matrix3")]
      $op!([<$lib M3S>], Matrix3<$in>, $in, Matrix3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Matrix4")]
      $op!([<$lib M4S>], Matrix4<$in>, $in, Matrix4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Matrix2x3")]
      $op!([<$lib M2x3S>], Matrix2x3<$in>, $in, Matrix2x3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Matrix3x2")]
      $op!([<$lib M3x2S>], Matrix3x2<$in>, $in, Matrix3x2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Row Scalar
      #[cfg(feature = "RowVector2")]
      $op!([<$lib R2S>], RowVector2<$in>, $in, RowVector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "RowVector3")]
      $op!([<$lib R3S>], RowVector3<$in>, $in, RowVector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "RowVector4")]
      $op!([<$lib R4S>], RowVector4<$in>, $in, RowVector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "RowVectorD")]
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Vector Scalar
      #[cfg(feature = "Vector2")]
      $op!([<$lib V2S>], Vector2<$in>, $in, Vector2<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib V3S>], Vector3<$in>, $in, Vector3<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "Vector4")]
      $op!([<$lib V4S>], Vector4<$in>, $in, Vector4<$out>,[<$lib:lower _scalar_lhs_op>]);
      #[cfg(feature = "VectorD")]
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>]);
      // Matrix Matrix
      #[cfg(feature = "Matrix1")]
      $op!([<$lib M1M1>], Matrix1<$in>, Matrix1<$in>, Matrix1<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Matrix2")]
      $op!([<$lib M2M2>], Matrix2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Matrix3")]
      $op!([<$lib M3M3>], Matrix3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Matrix4")]
      $op!([<$lib M4M4>], Matrix4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Matrix2x3")]
      $op!([<$lib M2x3M2x3>], Matrix2x3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Matrix3x2")]
      $op!([<$lib M3x2M3x2>], Matrix3x2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>]);
      // Matrix Vector
      #[cfg(feature = "Matrix2")]
      $op!([<$lib M2V2>], Matrix2<$in>, Vector2<$in>, Matrix2<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "Matrix3")]
      $op!([<$lib M3V3>], Matrix3<$in>, Vector3<$in>, Matrix3<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "Matrix4")]
      $op!([<$lib M4V4>], Matrix4<$in>, Vector4<$in>, Matrix4<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "Matrix2x3")]
      $op!([<$lib M2x3V2>], Matrix2x3<$in>, Vector2<$in>, Matrix2x3<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "Matrix3x2")]
      $op!([<$lib M3x2V3>], Matrix3x2<$in>, Vector3<$in>, Matrix3x2<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDVD>], DMatrix<$in>, DVector<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDV2>], DMatrix<$in>, Vector2<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDV3>], DMatrix<$in>, Vector3<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDV4>], DMatrix<$in>, Vector4<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>]);
      // Vector Matrix
      #[cfg(feature = "Vector2")]
      $op!([<$lib V2M2>], Vector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib V3M3>], Vector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector4")]
      $op!([<$lib V4M4>], Vector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector2")]
      $op!([<$lib V2M2x3>], Vector2<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib V3M3x2>], Vector3<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "VectorD")]
      $op!([<$lib VDMD>], DVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector2")]
      $op!([<$lib V2MD>], Vector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib V3MD>], Vector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      #[cfg(feature = "Vector4")]
      $op!([<$lib V4MD>], Vector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>]);
      // Matrix Row
      #[cfg(feature = "Matrix2")]
      $op!([<$lib M2R2>], Matrix2<$in>, RowVector2<$in>, Matrix2<$out>, [<$lib:lower _mat_row_op>]); 
      #[cfg(feature = "Matrix3")]
      $op!([<$lib M3R3>], Matrix3<$in>, RowVector3<$in>, Matrix3<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "Matrix4")]
      $op!([<$lib M4R4>], Matrix4<$in>, RowVector4<$in>, Matrix4<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "Matrix2x3")]
      $op!([<$lib M2x3R3>], Matrix2x3<$in>, RowVector3<$in>, Matrix2x3<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "Matrix3x2")]
      $op!([<$lib M3x2R2>], Matrix3x2<$in>, RowVector2<$in>, Matrix3x2<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDRD>], DMatrix<$in>, RowDVector<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDR2>], DMatrix<$in>, RowVector2<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDR3>], DMatrix<$in>, RowVector3<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]);
      #[cfg(feature = "MatrixD")]
      $op!([<$lib MDR4>], DMatrix<$in>, RowVector4<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>]); 
      // Row Matrix
      #[cfg(feature = "RowVector2")]
      $op!([<$lib R2M2>], RowVector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _row_mat_op>]); 
      #[cfg(feature = "RowVector3")]
      $op!([<$lib R3M3>], RowVector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector4")]
      $op!([<$lib R4M4>], RowVector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector3")]
      $op!([<$lib R3M2x3>], RowVector3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector2")]
      $op!([<$lib R2M3x2>], RowVector2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVectorD")]
      $op!([<$lib RDMD>], RowDVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector2")]
      $op!([<$lib R2MD>], RowVector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector3")]
      $op!([<$lib R3MD>], RowVector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      #[cfg(feature = "RowVector4")]
      $op!([<$lib R4MD>], RowVector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>]);
      // Row Row
      #[cfg(feature = "RowVector2")]
      $op!([<$lib R2R2>], RowVector2<$in>, RowVector2<$in>, RowVector2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "RowVector3")]
      $op!([<$lib R3R3>], RowVector3<$in>, RowVector3<$in>, RowVector3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "RowVector4")]
      $op!([<$lib R4R4>], RowVector4<$in>, RowVector4<$in>, RowVector4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "RowVectorD")]
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>]);
      // Vector Vector
      #[cfg(feature = "Vector2")]
      $op!([<$lib V2V2>], Vector2<$in>, Vector2<$in>, Vector2<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Vector3")]
      $op!([<$lib V3V3>], Vector3<$in>, Vector3<$in>, Vector3<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "Vector4")]
      $op!([<$lib V4V4>], Vector4<$in>, Vector4<$in>, Vector4<$out>, [<$lib:lower _vec_op>]);
      #[cfg(feature = "VectorD")]
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>]);
    }
  }}

#[macro_export]
macro_rules! impl_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $rhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            (Value::$lhs_type(lhs), Value::$rhs_type(rhs)) => Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref($default) })),
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib SM1>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib SM2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib SM3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib SM4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},   
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib SR2>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib SR3>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib SR4>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: new_ref(RowDVector::from_element(rhs.borrow().len(),$default))})),
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib SV2>]{lhs, rhs, out: new_ref(Vector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib SV3>]{lhs, rhs, out: new_ref(Vector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new([<$lib SV4>]{lhs, rhs, out: new_ref(Vector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$lhs_type(lhs), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: new_ref(DVector::from_element(rhs.borrow().len(),$default))})),
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M1S>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2S>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3S>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M4S>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R2S>]{lhs, rhs, out: new_ref(RowVector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R3S>]{lhs, rhs, out: new_ref(RowVector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R4S>]{lhs, rhs, out: new_ref(RowVector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V2S>]{lhs, rhs, out: new_ref(Vector2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V3S>]{lhs, rhs, out: new_ref(Vector3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V4S>]{lhs, rhs, out: new_ref(Vector4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(lhs)), Value::$matrix_kind(Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),  
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
            // Row Row
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(lhs)), Value::$matrix_kind(Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: new_ref(RowDVector::from_element(lhs.borrow().len(),$default))})),
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(lhs)), Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: new_ref(DVector::from_element(lhs.borrow().len(),$default))})),
            // Matrix Vector     
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)),Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2V2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)),Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3V3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),Value::$matrix_kind(Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2x3V2>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)),Value::$matrix_kind(Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3x2V3>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)),Value::$matrix_kind(Matrix::Vector4(rhs))) => Ok(Box::new([<$lib M4V4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            // Matrix Row     
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(lhs)),Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M2R2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(lhs)),Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M3R3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(lhs)),Value::$matrix_kind(Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M2x3R3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(lhs)),Value::$matrix_kind(Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M3x2R2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(lhs)),Value::$matrix_kind(Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib M4R4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(lhs)),Value::$matrix_kind(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              let rhs_shape = rhs.shape();
              match (rows,cols,rhs_shape[0],rhs_shape[1]) {
                // matching rows
                (n,_,m,1) if n == m => (),
                // matching cols
                (_,n,1,m) if n == m => (),
                // mismatching dimensions
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
              match rhs {
                Matrix::Vector2(rhs) => Ok(Box::new([<$lib MDV2>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::Vector3(rhs) => Ok(Box::new([<$lib MDV3>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::Vector4(rhs) => Ok(Box::new([<$lib MDV4>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::DVector(rhs) => Ok(Box::new([<$lib MDVD>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector2(rhs) => Ok(Box::new([<$lib MDR2>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector3(rhs) => Ok(Box::new([<$lib MDR3>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector4(rhs) => Ok(Box::new([<$lib MDR4>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowDVector(rhs) => Ok(Box::new([<$lib MDRD>]{lhs: lhs.clone(), rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib V2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib V3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Vector2(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib V2M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Vector3(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib V3M3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),                     
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Vector4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib V4M4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),                     
            // Row Matrix     
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib R2M2>]{lhs, rhs, out: new_ref(Matrix2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib R3M3>]{lhs, rhs, out: new_ref(Matrix3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(lhs)), Value::$matrix_kind(Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib R3M2x3>]{lhs, rhs, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(lhs)), Value::$matrix_kind(Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib R2M3x2>]{lhs, rhs, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(lhs)), Value::$matrix_kind(Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib R4M4>]{lhs, rhs, out: new_ref(Matrix4::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(lhs),Value::$matrix_kind(Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              let lhs_shape = lhs.shape();
              match (lhs_shape[0],lhs_shape[1],rows,cols) {
                // matching rows
                (m,1,n,_) if n == m => (),
                // matching cols
                (1,m,_,n) if n == m => (),
                // mismatching dimensions
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
              match lhs {
                Matrix::Vector2(lhs) => Ok(Box::new([<$lib V2MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::Vector3(lhs) => Ok(Box::new([<$lib V3MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::Vector4(lhs) => Ok(Box::new([<$lib V4MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::DVector(lhs) => Ok(Box::new([<$lib VDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector2(lhs) => Ok(Box::new([<$lib R2MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector3(lhs) => Ok(Box::new([<$lib R3MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowVector4(lhs) => Ok(Box::new([<$lib R4MD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                Matrix::RowDVector(lhs) => Ok(Box::new([<$lib RDMD>]{lhs, rhs, out: new_ref(DMatrix::from_element(rows,cols,$default))})),
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
            }
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_urnop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident => $($matrix_kind:ident, $target_type:ident, $default:expr, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: new_ref($default) })),
            #[cfg(all(feature = $value_string, feature = "Matrix1"))]
            (Value::$matrix_kind(Matrix::Matrix1(arg))) => Ok(Box::new([<$lib M1>]{arg, out: new_ref(Matrix1::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2"))]
            (Value::$matrix_kind(Matrix::Matrix2(arg))) => Ok(Box::new([<$lib M2>]{arg, out: new_ref(Matrix2::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix3"))]
            (Value::$matrix_kind(Matrix::Matrix3(arg))) => Ok(Box::new([<$lib M3>]{arg, out: new_ref(Matrix3::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix4"))]
            (Value::$matrix_kind(Matrix::Matrix4(arg))) => Ok(Box::new([<$lib M4>]{arg, out: new_ref(Matrix4::from_element($default))})),
            #[cfg(all(feature = $value_string, feature = "Matrix2x3"))]
            (Value::$matrix_kind(Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib M2x3>]{arg, out: new_ref(Matrix2x3::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "Matrix3x2"))]
            (Value::$matrix_kind(Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib M3x2>]{arg, out: new_ref(Matrix3x2::from_element($default))})),         
            #[cfg(all(feature = $value_string, feature = "RowVector2"))]
            (Value::$matrix_kind(Matrix::RowVector2(arg))) => Ok(Box::new([<$lib R2>]{arg: arg.clone(), out: new_ref(RowVector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector3"))]
            (Value::$matrix_kind(Matrix::RowVector3(arg))) => Ok(Box::new([<$lib R3>]{arg: arg.clone(), out: new_ref(RowVector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVector4"))]
            (Value::$matrix_kind(Matrix::RowVector4(arg))) => Ok(Box::new([<$lib R4>]{arg: arg.clone(), out: new_ref(RowVector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "RowVectorD"))]
            (Value::$matrix_kind(Matrix::RowDVector(arg))) => Ok(Box::new([<$lib RD>]{arg: arg.clone(), out: new_ref(RowDVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "Vector2"))]
            (Value::$matrix_kind(Matrix::Vector2(arg))) => Ok(Box::new([<$lib V2>]{arg: arg.clone(), out: new_ref(Vector2::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector3"))]
            (Value::$matrix_kind(Matrix::Vector3(arg))) => Ok(Box::new([<$lib V3>]{arg: arg.clone(), out: new_ref(Vector3::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "Vector4"))]
            (Value::$matrix_kind(Matrix::Vector4(arg))) => Ok(Box::new([<$lib V4>]{arg: arg.clone(), out: new_ref(Vector4::from_element($default)) })),
            #[cfg(all(feature = $value_string, feature = "VectorD"))]
            (Value::$matrix_kind(Matrix::DVector(arg))) => Ok(Box::new([<$lib VD>]{arg: arg.clone(), out: new_ref(DVector::from_element(arg.borrow().len(),$default))})),
            #[cfg(all(feature = $value_string, feature = "MatrixD"))]
            (Value::$matrix_kind(Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib MD>]{arg, out: new_ref(DMatrix::from_element(rows,cols,$default))}))},
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_binop_fxn {
  ($fxn_name:ident, $gen_fxn:ident) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let lhs_value = arguments[0].clone();
        let rhs_value = arguments[1].clone();
        match $gen_fxn(lhs_value.clone(), rhs_value.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (lhs_value,rhs_value) {
              (Value::MutableReference(lhs),Value::MutableReference(rhs)) => {$gen_fxn(lhs.borrow().clone(), rhs.borrow().clone())}
              (lhs_value,Value::MutableReference(rhs)) => { $gen_fxn(lhs_value.clone(), rhs.borrow().clone())}
              (Value::MutableReference(lhs),rhs_value) => { $gen_fxn(lhs.borrow().clone(), rhs_value.clone()) }
              x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_urnop_fxn {
  ($fxn_name:ident, $gen_fxn:ident) => {
    pub struct $fxn_name {}
    impl NativeFunctionCompiler for $fxn_name {
      fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
          return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
        }
        let input = arguments[0].clone();
        match $gen_fxn(input.clone()) {
          Ok(fxn) => Ok(fxn),
          Err(_) => {
            match (input) {
              (Value::MutableReference(input)) => {$gen_fxn(input.borrow().clone())}
              x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

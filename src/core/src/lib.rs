#![feature(get_mut_unchecked)]
#![allow(warnings)]
#![feature(iter_intersperse)]
#![feature(extract_if)]
#![cfg_attr(feature = "no-std", no_std)]
#![cfg_attr(feature = "no-std", alloc)]
#![allow(dead_code)]
#![feature(step_trait)]
#![feature(box_patterns)]

extern crate core as rust_core;
extern crate seahash;

#[cfg(feature="no-std")] #[macro_use] 
extern crate alloc;
#[cfg(not(feature = "no-std"))] 
extern crate core;
#[cfg(feature = "matrix")]  
extern crate nalgebra as na;
#[cfg(feature = "pretty_print")]
extern crate tabled;
#[cfg(feature = "no-std")]
extern crate alloc;
#[cfg(feature = "serde")] #[macro_use] 
extern crate serde_derive;
#[cfg(feature = "serde")]
extern crate serde;
#[cfg(any(feature = "math_exp", feature = "f64"))]
extern crate num_traits;
//extern crate ed25519_dalek;
//extern crate rand;

use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};
use std::collections::HashMap;
use std::io::{self, Write, Read};
use paste::paste;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::ops::*;
use std::mem;
use std::hash::{Hash, Hasher};
use std::convert::TryInto;
#[cfg(any(feature = "math_exp", feature = "f64"))]
use num_traits::*;
#[cfg(feature = "rational")]
use num_rational::Rational64;
#[cfg(feature = "vector3")]
use nalgebra::Vector3;
#[cfg(feature = "vectord")]
use nalgebra::DVector;
#[cfg(feature = "vector2")]
use nalgebra::Vector2;
#[cfg(feature = "vector4")]
use nalgebra::Vector4;
#[cfg(feature = "row_vectord")]
use nalgebra::RowDVector;
#[cfg(feature = "matrix1")]
use nalgebra::Matrix1;
#[cfg(feature = "matrix3")]
use nalgebra::Matrix3;
#[cfg(feature = "matrix4")]
use nalgebra::Matrix4;
#[cfg(feature = "row_vector3")]
use nalgebra::RowVector3;
#[cfg(feature = "row_vector4")]
use nalgebra::RowVector4;
#[cfg(feature = "row_vector2")]
use nalgebra::RowVector2;
#[cfg(feature = "matrixd")]
use nalgebra::DMatrix;
#[cfg(feature = "matrix2x3")]
use nalgebra::Matrix2x3;
#[cfg(feature = "matrix3x2")]
use nalgebra::Matrix3x2;
#[cfg(feature = "matrix2")]
use nalgebra::Matrix2;

#[cfg(feature = "pretty_print")]
use tabled::{
  builder::Builder,
  settings::{object::Rows,Panel, Span, Alignment, Modify, Style},
  Tabled,
};

pub mod value;
#[cfg(feature = "matrix")]
pub mod matrix;
pub mod types;
pub mod functions;
pub mod kind;
pub mod error;
pub mod nodes;
pub mod structures;
pub mod compiler;

pub use self::value::*;
#[cfg(feature = "matrix")]
pub use self::matrix::*;
pub use self::types::*;
pub use self::functions::*;
pub use self::kind::*;
pub use self::error::*;
pub use self::nodes::*;
pub use self::structures::*;
pub use self::compiler::*;

pub fn hash_chars(input: &Vec<char>) -> u64 {
  seahash::hash(input.iter().map(|s| String::from(*s)).collect::<String>().as_bytes()) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_bytes(input: &Vec<u8>) -> u64 {
  seahash::hash(input) & 0x00FFFFFFFFFFFFFF
}

pub fn hash_str(input: &str) -> u64 {
  seahash::hash(input.to_string().as_bytes()) & 0x00FFFFFFFFFFFFFF
}


pub fn emojify_bytes(bytes: &[u8]) -> String {
  let parts: Vec<&str> = bytes
    .iter()
    .enumerate()
    .filter_map(|(ix, &b)| if ix % 2 == 1 { Some(EMOJILIST[b as usize]) } else { None })
    .collect();
  parts.join("")
}

pub fn humanize_bytes(bytes: &[u8]) -> String {
  let parts: Vec<&str> = bytes
    .iter()
    .enumerate()
    .filter_map(|(ix, &b)| if ix % 2 == 1 { Some(WORDLIST[b as usize]) } else { None })
    .collect();
  parts.join("-")
}

pub fn emojify<T>(num: &T) -> String
where
    T: std::fmt::Display + Copy + TryInto<u128>,
    <T as TryInto<u128>>::Error: std::fmt::Debug,
{
    match (*num).try_into() {
        Ok(v) => {
            let bytes = v.to_be_bytes();
            let first_non_zero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
            let trimmed = &bytes[first_non_zero..];
            emojify_bytes(trimmed)
        }
        Err(_) => format!("{}", num),
    }
}

pub fn humanize<T>(num: &T) -> String
where
    T: std::fmt::Display + Copy + TryInto<u128>,
    <T as TryInto<u128>>::Error: std::fmt::Debug,
{
    match (*num).try_into() {
        Ok(v) => {
            let bytes = v.to_be_bytes();
            let first_non_zero = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len() - 1);
            let trimmed = &bytes[first_non_zero..];
            humanize_bytes(trimmed)
        }
        Err(_) => format!("{}", num),
    }
}


#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MechSourceCode {
    String(String),
    Tree(Program),
    Html(String),
    ByteCode(ParsedProgram),
    Program(Vec<MechSourceCode>),
}

impl MechSourceCode {

  pub fn to_string(&self) -> String {
    match self {
      MechSourceCode::ByteCode(bc) => format!("{:#?}", bc),
      MechSourceCode::String(s) => s.clone(),
      MechSourceCode::Tree(p) => todo!("Print the tree!"),
      MechSourceCode::Html(h) => h.clone(),
      MechSourceCode::Program(v) => v.iter().map(|c| c.to_string()).collect::<Vec<String>>().join("\n"),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexedString {
  pub data: Vec<char>,
  pub index_map: Vec<Vec<usize>>,
  pub rows: usize,
  pub cols: usize,
}

impl IndexedString {
  
  fn new(input: &str) -> Self {
      let mut data = Vec::new();
      let mut index_map = Vec::new();
      let mut current_row = 0;
      let mut current_col = 0;
      index_map.push(Vec::new());
      for c in input.chars() {
        data.push(c);
        if c == '\n' {
          index_map.push(Vec::new());
          current_row += 1;
          current_col = 0;
        } else {
          index_map[current_row].push(data.len() - 1);
          current_col += 1;
        }
      }
      let rows = index_map.len();
      let cols = if rows > 0 { index_map[0].len() } else { 0 };
      IndexedString {
          data,
          index_map,
          rows,
          cols,
      }
  }

  fn to_string(&self) -> String {
    self.data.iter().collect()
  }

  fn get(&self, row: usize, col: usize) -> Option<char> {
    if row < self.rows {
      let rowz = &self.index_map[row];
      if col < rowz.len() {
        let index = self.index_map[row][col];
        Some(self.data[index])
      } else {
        None
      }
    } else {
      None
    }
  }

  fn set(&mut self, row: usize, col: usize, new_char: char) -> Result<(), String> {
    if row < self.rows {
      let row_indices = &mut self.index_map[row];
      if col < row_indices.len() {
        let index = row_indices[col];
        self.data[index] = new_char;
        Ok(())
      } else {
        Err("Column index out of bounds".to_string())
      }
    } else {
      Err("Row index out of bounds".to_string())
    }
  }
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

pub const EMOJILIST: &[&str; 256] = &[
  "🐵","🐶","🐺","🦊","🦝","🐱","🐈","🐈","🦁","🐷","🐮","🦬","🐯","🐴","🫎","🦄","🦓","🦙","🦒","🐘","🦣","🦏","🦛","🐫","🐏","🐭","🐰","🐿️","🦫","🦔","🦇","🐻","🐨","🐼","🦥","🦦","🦨","🦘","🦡","🦃","🐔","🐦","🐧","🕊️","🦅","🦆","🐦‍🔥","🦉","🦤","🦩","🦚","🦜","🐸","🐊","🐢","🦎","🐍","🐲","🦖","🐳","🐬","🦭","🐠","🦈","🐙","🪼","🦀","🦞","🦐","🦑","🐌","🦋","🐛","🐝","🪲","🐞","🦗","🕸️","🪰","🪱","🦠","👻","👽","🐶","🐮","🐚","🪸","🪶","🦧","🪿","🦢","🤖",
  "🌹","🌳","🌴","🌵","🍀","🍁","🍄","🌛","🌞","🪐","⭐","⛅","🌧️","🌨️","🌈","❄️","☃️","☄️","🔥","🌻",
  "🍇","🍉","🍊","🍋","🍋‍🟩","🍌","🍍","🥭","🍎","🍐","🍓","🥝","🍅","🫒","🥥","🥔","🥕","🌽","🌶️","🫑","🥒","🥦","🧄","🧅","🫛","🍦","🍧","🍩","🍪","🍰","🧁","🥧","🍫","🍭","🍞","🥨","🥯","🧇","🍟","🍿","🧃",
  "🎤","🎧","📻","🎷","🪗","🎸","🎹","🎺","🎻","🪕","🥁","🪇","📷","🧳","🌡️","🧸","🧶","🔎","🕯️","💡","🔦","🔒","🗝️","🪚","🔧","🪛","🔩","⚙️","⚖️","🧰","🧲","🪜","🔬","📡","🧷","🧹","🧺","🪣","🧼","🧽","🧯","🛒",  
  "⏰","🛟","🛩️","🚁","🛰️","🚀","🛸","⚓","🚂","🚑","🚒","🚕","🚗","🚚","🚜","🏎️","🏍️","🛵","🦼","🚲","🛹","🛼","🛞","📰","📦","📫","✏️","🖊️","🖌️","🖍️","📌","📏","✂️","🗑️","🏆","⚾","🏀","🎾","🎳","⛳","⛸️","🤿","🛷","🎯","🪁","🧩","🪅","🎨","🧭","🏔️","🏝️","⛲","⛺","🎠","🛝","🧵","💈","🎪","🛎️","💎","⛵"
];


// ============================================================================
// The Standard Library!
// ============================================================================

#[macro_export]
macro_rules! impl_binop {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:ident) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      lhs: Ref<$arg1_type>,
      rhs: Ref<$arg2_type>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd + CompileConst +
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
        let out_ptr = self.out.as_mut_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
    }
    fn out(&self) -> Value { self.out.to_value() }
          fn to_string(&self) -> String { format!("{:#?}", self) }
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
      // allocate three registers as an array
      let mut registers = [0,0,0];

      // Compile lhs
      let lhs_addr = self.lhs.addr();
      let lhs_reg = ctx.alloc_register_for_ptr(lhs_addr);
      let lhs_borrow = self.lhs.borrow();
      let lhs_const_id = lhs_borrow.compile_const(ctx).unwrap();
      ctx.emit_const_load(lhs_reg, lhs_const_id);
      registers[0] = lhs_reg;

      // Compile rhs
      let rhs_addr = self.rhs.addr();
      let rhs_reg = ctx.alloc_register_for_ptr(rhs_addr);
      let rhs_borrow = self.rhs.borrow();
      let rhs_const_id = rhs_borrow.compile_const(ctx).unwrap();
      ctx.emit_const_load(rhs_reg, rhs_const_id);
      registers[1] = rhs_reg;

      // Compile out
      let out_addr = self.out.addr();
      let out_reg = ctx.alloc_register_for_ptr(out_addr);
      let out_borrow = self.out.borrow();
      let out_const_id = out_borrow.compile_const(ctx).unwrap();
      ctx.emit_const_load(out_reg, out_const_id);
      registers[2] = out_reg;

      ctx.features.insert(FeatureFlag::Builtin(FeatureKind::$feature_flag));

      // Emit the operation
      ctx.emit_binop(
        hash_str(stringify!($struct_name)),
        registers[0],
        registers[1],
        registers[2],
      );

      Ok(registers[2])
    }
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
        let out_ptr = self.out.as_mut_ptr();
        $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
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
      T: Debug + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let arg_ptr = self.arg.as_ptr();
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
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
        let out_ptr = self.out.as_mut_ptr();
        $op!(arg_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }};} 

#[macro_export]
macro_rules! impl_fxns {
  ($lib:ident, $in:ident, $out:ident, $op:ident) => {
    paste!{
      // Scalar
      $op!([<$lib SS>], $in, $in, $out, [<$lib:lower _op>], $lib);
      // Scalar Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib SM1>], $in, Matrix1<$in>, Matrix1<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrix2")]
      $op!([<$lib SM2>], $in, Matrix2<$in>, Matrix2<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrix3")]
      $op!([<$lib SM3>], $in, Matrix3<$in>, Matrix3<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrix4")]
      $op!([<$lib SM4>], $in, Matrix4<$in>, Matrix4<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib SM2x3>], $in, Matrix2x3<$in>, Matrix2x3<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib SM3x2>], $in, Matrix3x2<$in>, Matrix3x2<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib SMD>], $in, DMatrix<$in>, DMatrix<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      // Scalar Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib SR2>], $in, RowVector2<$in>, RowVector2<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib SR3>], $in, RowVector3<$in>, RowVector3<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib SR4>], $in, RowVector4<$in>, RowVector4<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib SRD>], $in, RowDVector<$in>, RowDVector<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      // Scalar Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib SV2>], $in, Vector2<$in>, Vector2<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib SV3>], $in, Vector3<$in>, Vector3<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "vector4")]
      $op!([<$lib SV4>], $in, Vector4<$in>, Vector4<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      #[cfg(feature = "vectord")]
      $op!([<$lib SVD>], $in, DVector<$in>, DVector<$out>,[<$lib:lower _scalar_rhs_op>], $lib);
      // Matrix Scalar
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1S>], Matrix1<$in>, $in, Matrix1<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2S>], Matrix2<$in>, $in, Matrix2<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3S>], Matrix3<$in>, $in, Matrix3<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4S>], Matrix4<$in>, $in, Matrix4<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3S>], Matrix2x3<$in>, $in, Matrix2x3<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2S>], Matrix3x2<$in>, $in, Matrix3x2<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDS>], DMatrix<$in>, $in, DMatrix<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      // Row Scalar
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2S>], RowVector2<$in>, $in, RowVector2<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3S>], RowVector3<$in>, $in, RowVector3<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4S>], RowVector4<$in>, $in, RowVector4<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDS>], RowDVector<$in>, $in, RowDVector<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      // Vector Scalar
      #[cfg(feature = "vector2")]
      $op!([<$lib V2S>], Vector2<$in>, $in, Vector2<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3S>], Vector3<$in>, $in, Vector3<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4S>], Vector4<$in>, $in, Vector4<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      #[cfg(feature = "vectord")]
      $op!([<$lib VDS>], DVector<$in>, $in, DVector<$out>,[<$lib:lower _scalar_lhs_op>], $lib);
      // Matrix Matrix
      #[cfg(feature = "matrix1")]
      $op!([<$lib M1M1>], Matrix1<$in>, Matrix1<$in>, Matrix1<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2M2>], Matrix2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3M3>], Matrix3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4M4>], Matrix4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3M2x3>], Matrix2x3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2M3x2>], Matrix3x2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDMD>], DMatrix<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_op>], $lib);
      // Matrix Vector
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2V2>], Matrix2<$in>, Vector2<$in>, Matrix2<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3V3>], Matrix3<$in>, Vector3<$in>, Matrix3<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4V4>], Matrix4<$in>, Vector4<$in>, Matrix4<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3V2>], Matrix2x3<$in>, Vector2<$in>, Matrix2x3<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2V3>], Matrix3x2<$in>, Vector3<$in>, Matrix3x2<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDVD>], DMatrix<$in>, DVector<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDV2>], DMatrix<$in>, Vector2<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDV3>], DMatrix<$in>, Vector3<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDV4>], DMatrix<$in>, Vector4<$in>, DMatrix<$out>, [<$lib:lower _mat_vec_op>], $lib);
      // Vector Matrix
      #[cfg(feature = "vector2")]
      $op!([<$lib V2M2>], Vector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3M3>], Vector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4M4>], Vector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector2")]
      $op!([<$lib V2M2x3>], Vector2<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3M3x2>], Vector3<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vectord")]
      $op!([<$lib VDMD>], DVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector2")]
      $op!([<$lib V2MD>], Vector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3MD>], Vector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], $lib);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4MD>], Vector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _vec_mat_op>], $lib);
      // Matrix Row
      #[cfg(feature = "matrix2")]
      $op!([<$lib M2R2>], Matrix2<$in>, RowVector2<$in>, Matrix2<$out>, [<$lib:lower _mat_row_op>], $lib); 
      #[cfg(feature = "matrix3")]
      $op!([<$lib M3R3>], Matrix3<$in>, RowVector3<$in>, Matrix3<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrix4")]
      $op!([<$lib M4R4>], Matrix4<$in>, RowVector4<$in>, Matrix4<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrix2x3")]
      $op!([<$lib M2x3R3>], Matrix2x3<$in>, RowVector3<$in>, Matrix2x3<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrix3x2")]
      $op!([<$lib M3x2R2>], Matrix3x2<$in>, RowVector2<$in>, Matrix3x2<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDRD>], DMatrix<$in>, RowDVector<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDR2>], DMatrix<$in>, RowVector2<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDR3>], DMatrix<$in>, RowVector3<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], $lib);
      #[cfg(feature = "matrixd")]
      $op!([<$lib MDR4>], DMatrix<$in>, RowVector4<$in>, DMatrix<$out>, [<$lib:lower _mat_row_op>], $lib); 
      // Row Matrix
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2M2>], RowVector2<$in>, Matrix2<$in>, Matrix2<$out>, [<$lib:lower _row_mat_op>], $lib); 
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3M3>], RowVector3<$in>, Matrix3<$in>, Matrix3<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4M4>], RowVector4<$in>, Matrix4<$in>, Matrix4<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3M2x3>], RowVector3<$in>, Matrix2x3<$in>, Matrix2x3<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2M3x2>], RowVector2<$in>, Matrix3x2<$in>, Matrix3x2<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDMD>], RowDVector<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2MD>], RowVector2<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3MD>], RowVector3<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], $lib);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4MD>], RowVector4<$in>, DMatrix<$in>, DMatrix<$out>, [<$lib:lower _row_mat_op>], $lib);
      // Row Row
      #[cfg(feature = "row_vector2")]
      $op!([<$lib R2R2>], RowVector2<$in>, RowVector2<$in>, RowVector2<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "row_vector3")]
      $op!([<$lib R3R3>], RowVector3<$in>, RowVector3<$in>, RowVector3<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "row_vector4")]
      $op!([<$lib R4R4>], RowVector4<$in>, RowVector4<$in>, RowVector4<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "row_vectord")]
      $op!([<$lib RDRD>], RowDVector<$in>, RowDVector<$in>, RowDVector<$out>, [<$lib:lower _vec_op>], $lib);
      // Vector Vector
      #[cfg(feature = "vector2")]
      $op!([<$lib V2V2>], Vector2<$in>, Vector2<$in>, Vector2<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "vector3")]
      $op!([<$lib V3V3>], Vector3<$in>, Vector3<$in>, Vector3<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "vector4")]
      $op!([<$lib V4V4>], Vector4<$in>, Vector4<$in>, Vector4<$out>, [<$lib:lower _vec_op>], $lib);
      #[cfg(feature = "vectord")]
      $op!([<$lib VDVD>], DVector<$in>, DVector<$in>, DVector<$out>, [<$lib:lower _vec_op>], $lib);
    }
  }}

#[macro_export]
macro_rules! impl_binop_match_arms {
  ($lib:ident, $arg:expr, $($lhs_type:ident, $($target_type:ident, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            // Scalar Scalar
            #[cfg(all(feature = $value_string))]
            (Value::$lhs_type(lhs), Value::$lhs_type(rhs)) => Ok(Box::new([<$lib SS>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new($target_type::default()) })),
            // Scalar Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib SM1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib SM2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib SM3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib SM4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib SM2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib SM3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {rhs.borrow().shape()};
              Ok(Box::new([<$lib SMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },   
            // Scalar Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib SR2>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib SR3>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib SR4>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib SRD>]{lhs, rhs: rhs.clone(), out: Ref::new(RowDVector::from_element(rhs.borrow().len(),$target_type::default()))})),
            // Scalar Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib SV2>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib SV3>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib SV4>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::$lhs_type(lhs), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => Ok(Box::new([<$lib SVD>]{lhs, rhs: rhs.clone(), out: Ref::new(DVector::from_element(rhs.borrow().len(),$target_type::default()))})),
            // Matrix Scalar
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M1S>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2S>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3S>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M4S>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M2x3S>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib M3x2S>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),Value::$lhs_type(rhs)) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Scalar
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R2S>]{lhs, rhs, out: Ref::new(RowVector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R3S>]{lhs, rhs, out: Ref::new(RowVector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib R4S>]{lhs, rhs, out: Ref::new(RowVector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib RDS>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Vector Scalar
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V2S>]{lhs, rhs, out: Ref::new(Vector2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V3S>]{lhs, rhs, out: Ref::new(Vector3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib V4S>]{lhs, rhs, out: Ref::new(Vector4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)),Value::$lhs_type(rhs)) => Ok(Box::new([<$lib VDS>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Matrix Matrix
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix1(rhs))) => Ok(Box::new([<$lib M1M1>]{lhs, rhs, out: Ref::new(Matrix1::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib M2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib M3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib M4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib M2x3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),  
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib M3x2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)), Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
              let (rows,cols) = {lhs.borrow().shape()};
              Ok(Box::new([<$lib MDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))}))
            },
            // Row Row
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib R2R2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib R3R3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib R4R4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(RowVector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::RowDVector(rhs))) => Ok(Box::new([<$lib RDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(RowDVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Vector Vector
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib V2V2>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector2::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib V3V3>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector3::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib V4V4>]{lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(Vector4::from_element($target_type::default())) })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(lhs)), Value::[<Matrix $lhs_type>](Matrix::DVector(rhs))) => Ok(Box::new([<$lib VDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DVector::from_element(lhs.borrow().len(),$target_type::default()))})),
            // Matrix Vector     
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2V2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3V3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector2(rhs))) => Ok(Box::new([<$lib M2x3V2>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector3(rhs))) => Ok(Box::new([<$lib M3x2V3>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::Vector4(rhs))) => Ok(Box::new([<$lib M4V4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M2R2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M3R3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector3(rhs))) => Ok(Box::new([<$lib M2x3R3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector2(rhs))) => Ok(Box::new([<$lib M3x2R2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(lhs)),Value::[<Matrix $lhs_type>](Matrix::RowVector4(rhs))) => Ok(Box::new([<$lib M4R4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(lhs)),Value::[<Matrix $lhs_type>](rhs)) => {
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
                #[cfg(feature = "vector2")]
                Matrix::Vector2(rhs) => Ok(Box::new([<$lib MDV2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector3")]
                Matrix::Vector3(rhs) => Ok(Box::new([<$lib MDV3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector4")]
                Matrix::Vector4(rhs) => Ok(Box::new([<$lib MDV4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vectord")]
                Matrix::DVector(rhs) => Ok(Box::new([<$lib MDVD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(rhs) => Ok(Box::new([<$lib MDR2>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(reature = "row_vector3")]
                Matrix::RowVector3(rhs) => Ok(Box::new([<$lib MDR3>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(rhs) => Ok(Box::new([<$lib MDR4>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(rhs) => Ok(Box::new([<$lib MDRD>]{lhs: lhs.clone(), rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                _ => {return Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::DimensionMismatch(vec![]) });},
              }
            },
            // Vector Matrix
            #[cfg(all(feature = $value_string, feature = "matrix2", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib V2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib V3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib V2M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib V3M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),                     
            #[cfg(all(feature = $value_string, feature = "matrix4", feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib V4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),                     
            // Row Matrix     
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2(rhs))) => Ok(Box::new([<$lib R2M2>]{lhs, rhs, out: Ref::new(Matrix2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3(rhs))) => Ok(Box::new([<$lib R3M3>]{lhs, rhs, out: Ref::new(Matrix3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(rhs))) => Ok(Box::new([<$lib R3M2x3>]{lhs, rhs, out: Ref::new(Matrix2x3::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(rhs))) => Ok(Box::new([<$lib R2M3x2>]{lhs, rhs, out: Ref::new(Matrix3x2::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(lhs)), Value::[<Matrix $lhs_type>](Matrix::Matrix4(rhs))) => Ok(Box::new([<$lib R4M4>]{lhs, rhs, out: Ref::new(Matrix4::from_element($target_type::default()))})),         
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](lhs),Value::[<Matrix $lhs_type>](Matrix::DMatrix(rhs))) => {
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
                #[cfg(feature = "vector2")]
                Matrix::Vector2(lhs) => Ok(Box::new([<$lib V2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector3")]
                Matrix::Vector3(lhs) => Ok(Box::new([<$lib V3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vector4")]
                Matrix::Vector4(lhs) => Ok(Box::new([<$lib V4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "vectord")]
                Matrix::DVector(lhs) => Ok(Box::new([<$lib VDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector2")]
                Matrix::RowVector2(lhs) => Ok(Box::new([<$lib R2MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector3")]
                Matrix::RowVector3(lhs) => Ok(Box::new([<$lib R3MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vector4")]
                Matrix::RowVector4(lhs) => Ok(Box::new([<$lib R4MD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
                #[cfg(feature = "row_vectord")]
                Matrix::RowDVector(lhs) => Ok(Box::new([<$lib RDMD>]{lhs, rhs, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default()))})),
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
  ($lib:tt, $arg:tt, $($lhs_type:tt, $($target_type:tt, $value_string:tt),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            #[cfg(feature = $value_string)]
            (Value::$lhs_type(arg)) => Ok(Box::new([<$lib S>]{arg: arg.clone(), out: Ref::new($target_type::default()), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix1"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix1(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix1::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix4(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix2x3(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix2x3::from_element($target_type::default())), _marker: PhantomData::default() })),         
            #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Matrix3x2(arg))) => Ok(Box::new([<$lib V>]{arg, out: Ref::new(Matrix3x2::from_element($target_type::default())), _marker: PhantomData::default() })),         
            #[cfg(all(feature = $value_string, feature = "row_vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowVector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowVector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "row_vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::RowDVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(RowDVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector2"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector2(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector2::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector3"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector3(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector3::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vector4"))]
            (Value::[<Matrix $lhs_type>](Matrix::Vector4(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(Vector4::from_element($target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "vectord"))]
            (Value::[<Matrix $lhs_type>](Matrix::DVector(arg))) => Ok(Box::new([<$lib V>]{arg: arg.clone(), out: Ref::new(DVector::from_element(arg.borrow().len(),$target_type::default())), _marker: PhantomData::default() })),
            #[cfg(all(feature = $value_string, feature = "matrixd"))]
            (Value::[<Matrix $lhs_type>](Matrix::DMatrix(arg))) => {
              let (rows,cols) = {arg.borrow().shape()};
              Ok(Box::new([<$lib V>]{arg, out: Ref::new(DMatrix::from_element(rows,cols,$target_type::default())), _marker: PhantomData::default() }))},
          )+
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

#[macro_export]
macro_rules! impl_mech_binop_fxn {
  ($fxn_name:ident, $gen_fxn:tt) => {
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
  ($fxn_name:ident, $gen_fxn:tt) => {
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
              x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:#?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
            }
          }
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_all_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident, $value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Vector Scalar
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          // Vector Vector
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name V>]{ sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Matrix Scalar Bool
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name SB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        
          // Matrix Vector Bool
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix4(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix2x3(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixBool(Matrix::DVector(ix)),Value::IndexAll], Value::[<Matrix $value_kind>](Matrix::Matrix3x2(source))) if ix.borrow().len() == source.borrow().nrows() && sink.borrow().ncols() == source.borrow().ncols() => Ok(Box::new([<$fxn_name VB>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => {
          println!("{:#?}", x);
          Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind })
        }
      }
    }
  }
}

#[macro_export]
macro_rules! impl_set_range_match_arms {
  ($fxn_name:ident, $arg:expr, $($value_kind:ident,$value_string:tt);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          // Set vector
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector4(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector3(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::RowVector2(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector4(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector3(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::Vector2(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Set Matrix
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::[<Matrix $value_kind>](Matrix::DMatrix(source))) => Ok(Box::new([<$fxn_name V>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          
          // Set scalar
          #[cfg(all(feature = $value_string, feature = "row_vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),            
          #[cfg(all(feature = $value_string, feature = "row_vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)), [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),   [Value::MatrixIndex(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name S>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),

          // Bool
          #[cfg(all(feature = $value_string, feature = "row_vector4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vector2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowVector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vector2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Vector2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix4", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix4(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix1", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix1(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix2x3", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix2x3(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "matrix3x2", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::Matrix3x2(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),            
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DMatrix(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "vectord", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::DVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
          #[cfg(all(feature = $value_string, feature = "row_vectord", feature = "bool"))]
          (Value::[<Matrix $value_kind>](Matrix::RowDVector(sink)),[Value::MatrixBool(Matrix::DVector(ix))], Value::$value_kind(source)) => Ok(Box::new([<$fxn_name B>] { sink: sink.clone(), ixes: ix.clone(), source: source.clone(), _marker: PhantomData::default() })),
        )+
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

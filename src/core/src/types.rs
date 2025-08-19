use crate::value::*;
use crate::functions::*;
use crate::*;
use crate::nodes::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::ops::*;
use std::iter::Step;
use paste::paste;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
#[cfg(feature = "math_exp")]
use num_traits::Pow;
#[cfg(any(feature = "f64", feature = "f32"))]
use num_traits::{Zero, One};
#[cfg(feature = "math_exp")]
use libm::{pow,powf};
#[cfg(feature = "complex")]
use nalgebra::Complex;

pub struct Ref<T>(pub Rc<RefCell<T>>);

impl<T: Debug> Debug for Ref<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let addr = self.0.as_ptr() as usize;
    let emojified = emojify(&(addr as u64));
    write!(f, "@{}: {:#?}", emojified, self.borrow())
  }
}

impl<T> Clone for Ref<T> {
  fn clone(&self) -> Self {
    Ref(self.0.clone())
  }
}

impl<T> Ref<T> {
  pub fn new(item: T) -> Self { Ref(Rc::new(RefCell::new(item))) }
  pub fn as_ptr(&self) -> *const T { self.0.as_ptr() }
  pub fn as_mut_ptr(&self) -> *mut T { self.0.as_ptr() as *mut T }
  pub fn borrow(&self) -> std::cell::Ref<'_, T> { self.0.borrow() }
  pub fn borrow_mut(&self) -> std::cell::RefMut<'_, T> { self.0.borrow_mut() }
  pub fn addr(&self) -> usize { Rc::as_ptr(&self.0) as *const () as usize }
  pub fn id(&self) -> u64 { Rc::as_ptr(&self.0) as *const () as u64 }
}

impl<T: PartialEq> PartialEq for Ref<T> {
  fn eq(&self, other: &Self) -> bool {
    *self.borrow() == *other.borrow()
  }
}
impl<T: PartialEq> Eq for Ref<T> {}

pub struct Plan(pub Ref<Vec<Box<dyn MechFunction>>>);

impl Clone for Plan {
  fn clone(&self) -> Self { Plan(self.0.clone()) }
}

impl fmt::Debug for Plan {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for p in &(*self.0.borrow()) {
      writeln!(f, "{}", p.to_string())?;
    }
    Ok(())
  }
}

impl Plan {
  pub fn new() -> Self { Plan(Ref::new(vec![])) }
  pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow() }
  pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow_mut() }
  pub fn add_function(&self, func: Box<dyn MechFunction>) { self.0.borrow_mut().push(func); }
  pub fn get_functions(&self) -> std::cell::Ref<'_, Vec<Box<dyn MechFunction>>> { self.0.borrow() }
  pub fn len(&self) -> usize { self.0.borrow().len() }
  pub fn is_empty(&self) -> bool { self.0.borrow().is_empty() }
}

#[cfg(feature = "pretty_print")]
impl PrettyPrint for Plan {
  fn pretty_print(&self) -> String {
    let mut builder = Builder::default();

    let mut row = vec![];
    let plan_brrw = self.0.borrow();
    if self.is_empty() {
      builder.push_record(vec!["".to_string()]);
    } else {
      for (ix, fxn) in plan_brrw.iter().enumerate() {
        let plan_str = format!("{}. {}\n", ix + 1, fxn.to_string());
        row.push(plan_str.clone());
        if row.len() == 4 {
          builder.push_record(row.clone());
          row.clear();
        }
      }
    }
    if row.is_empty() == false {
      builder.push_record(row.clone());
    }
    let mut table = builder.build();
    table.with(Style::modern_rounded())
        .with(Panel::header("📋 Plan"));
    format!("{table}")
  }
}

pub type FunctionsRef = Ref<Functions>;
pub type MutableReference = Ref<Value>;
pub type SymbolTableRef= Ref<SymbolTable>;
pub type ValRef = Ref<Value>;
use std::num::FpCategory;

//pub type Ref<T> = Rc<RefCell<T>>;
//pub fn Ref::new<T>(item: T) -> Rc<RefCell<T>> {
//  Rc::new(RefCell::new(item))
//}

pub type MResult<T> = Result<T,MechError>;

#[cfg(feature = "f64")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Copy, PartialOrd)]
pub struct F64(pub f64);

#[cfg(feature = "f64")]
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}

#[cfg(feature = "f64")]
impl fmt::Debug for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f64")]
impl fmt::Display for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f64")]
impl From<F64> for String {
  fn from(f: F64) -> Self {
      f.to_string()
  }
}

#[cfg(feature = "f64")]
impl From<F64> for usize {
  fn from(value: F64) -> Self {
    value.0 as usize
  }
}

#[cfg(feature = "f64")]
impl Eq for F64 {}

#[cfg(feature = "f64")]
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[cfg(all(feature = "f64", feature = "math_exp"))]
impl Pow<F64> for F64 {
  type Output = F64;
  fn pow(self, rhs: F64) -> Self::Output {
    F64(self.0.powf(rhs.0))
  }
}

#[cfg(feature = "f64")]
impl Add for F64 {
  type Output = F64;
  fn add(self, other: F64) -> F64 {
    F64(self.0 + other.0)
  }
}

#[cfg(feature = "f64")]
impl AddAssign for F64 {
  fn add_assign(&mut self, other: F64) {
    self.0 += other.0;
  }
}
#[cfg(feature = "f64")]
impl Sub for F64 {
  type Output = F64;
  fn sub(self, other: F64) -> F64 {
    F64(self.0 - other.0)
  }
}
#[cfg(feature = "f64")]
impl SubAssign for F64 {
  fn sub_assign(&mut self, other: F64) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "f64")]
impl Mul for F64 {
  type Output = F64;
  fn mul(self, other: F64) -> F64 {
    F64(self.0 * other.0)
  }
}

#[cfg(feature = "f64")]
impl MulAssign for F64 {
  fn mul_assign(&mut self, other: F64) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "f64")]
impl Div for F64 {
  type Output = F64;
  fn div(self, other: F64) -> F64 {
    F64(self.0 / other.0)
  }
}

#[cfg(feature = "f64")]
impl DivAssign for F64 {
  fn div_assign(&mut self, other: F64) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "f64")]
impl Rem for F64 {
  type Output = F64;
  fn rem(self, other: F64) -> F64 {
    F64(self.0 % other.0)
  }
}

#[cfg(feature = "f64")]
impl RemAssign for F64 {
  fn rem_assign(&mut self, other: F64) {
    self.0 = self.0 % other.0;
  }
}

#[cfg(feature = "f64")]
impl Default for F64 {
  fn default() -> Self {
    F64(0.0)
  }
}

#[cfg(feature = "f64")]
impl Zero for F64 {
  fn zero() -> Self {
    F64(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

#[cfg(feature = "f64")]
impl One for F64 {
  fn one() -> Self {
    F64(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}

#[cfg(feature = "f64")]
impl Neg for F64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F64(-self.0)
  }
}

#[cfg(feature = "f64")]
impl Step for F64 {
  fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
    if start.0 > end.0 {
      return (0, None);
    }
    let diff = end.0 - start.0;
    // Handle special floating-point cases
    match diff.classify() {
      FpCategory::Normal | FpCategory::Zero => {
        if diff.fract() == 0.0 {
          let steps = diff as usize;
          (steps, Some(steps))
        } else {
          (usize::MAX, None)
        }
      }
      _ => (usize::MAX, None),
    }
  }

  fn forward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F64(start.0 + count as f64)) 
  }

  fn backward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F64(start.0 - count as f64)) 
  }

  fn forward(start: Self, count: usize) -> Self {
    F64(start.0 + count as f64) 
  }

  fn backward(start: Self, count: usize) -> Self {
    F64(start.0 - count as f64)
  }
}

#[cfg(feature = "f64")]
impl From<F64> for Value {
  fn from(val: F64) -> Self {
    Value::F64(Ref::new(val))
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg(feature = "f32")]
#[derive(PartialEq, Clone, Copy, PartialOrd)]
pub struct F32(pub f32);

#[cfg(feature = "f32")]
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
  }
}

macro_rules! impl_into {
  ($from:ty => $($to:ty),*) => {
    $(impl Into<$to> for $from {
      fn into(self) -> $to {
        self.0 as $to
      }
    })*
  };
}

macro_rules! impl_into_float {
  ($from:ty => $($to:ty),*) => {
    paste!{
      $(impl Into<[<$to:upper>]> for $from {
        fn into(self) -> [<$to:upper>] {
          [<$to:upper>]::new(self as $to)
        }
      })*
    }
  };
}

#[cfg(all(feature = "f32", feature = "u8"))]
impl_into_float!(u8 => f32);
#[cfg(all(feature = "f64", feature = "u8"))]
impl_into_float!(u8 => f64);
#[cfg(all(feature = "f32", feature = "u16"))]
impl_into_float!(u16 => f32);
#[cfg(all(feature = "f64", feature = "u16"))]
impl_into_float!(u16 => f64);
#[cfg(all(feature = "f32", feature = "u32"))]
impl_into_float!(u32 => f32);
#[cfg(all(feature = "f64", feature = "u32"))]
impl_into_float!(u32 => f64);
#[cfg(all(feature = "f32", feature = "u64"))]
impl_into_float!(u64 => f32);
#[cfg(all(feature = "f64", feature = "u64"))]
impl_into_float!(u64 => f64);
#[cfg(all(feature = "f32", feature = "u128"))]
impl_into_float!(u128 => f32);
#[cfg(all(feature = "f64", feature = "u128"))]
impl_into_float!(u128 => f64);
#[cfg(all(feature = "f32", feature = "i8"))]
impl_into_float!(i8 => f32);
#[cfg(all(feature = "f64", feature = "i8"))]
impl_into_float!(i8 => f64);
#[cfg(all(feature = "f32", feature = "i16"))]
impl_into_float!(i16 => f32);
#[cfg(all(feature = "f64", feature = "i16"))]
impl_into_float!(i16 => f64);
#[cfg(all(feature = "f32", feature = "i32"))]
impl_into_float!(i32 => f32);
#[cfg(all(feature = "f64", feature = "i32"))]
impl_into_float!(i32 => f64);
#[cfg(all(feature = "f32", feature = "i64"))]
impl_into_float!(i64 => f32);
#[cfg(all(feature = "f64", feature = "i64"))]
impl_into_float!(i64 => f64);
#[cfg(all(feature = "f32", feature = "i128"))]
impl_into_float!(i128 => f32);
#[cfg(all(feature = "f64", feature = "i128"))]
impl_into_float!(i128 => f64);

#[cfg(feature = "f64")]
impl_into!(F64 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
#[cfg(feature = "f32")]
impl_into!(F32 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

#[cfg(feature = "f32")]
impl fmt::Display for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f32")]
impl fmt::Debug for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f32")]
impl From<F32> for String {
  fn from(f: F32) -> Self {
      f.to_string()
  }
}

#[cfg(feature = "f32")]
impl From<F32> for usize {
  fn from(value: F32) -> Self {
    value.0 as usize
  }
}

#[cfg(all(feature = "f32", feature = "math_exp"))]
impl Pow<F32> for F32 {
  type Output = F32;
  fn pow(self, rhs: F32) -> Self::Output {
    F32(self.0.pow(rhs.0))
  }
}

#[cfg(feature = "f32")]
impl Eq for F32 {}

#[cfg(feature = "f32")]
impl Hash for F32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[cfg(feature = "f32")]
impl Add for F32 {
  type Output = F32;
  fn add(self, other: F32) -> F32 {
    F32(self.0 + other.0)
  }
}

#[cfg(feature = "f32")]
impl AddAssign for F32 {
  fn add_assign(&mut self, other: F32) {
    self.0 += other.0;
  }
}

#[cfg(feature = "f32")]
impl Rem for F32 {
  type Output = F32;
  fn rem(self, other: F32) -> F32 {
    F32(self.0 % other.0)
  }
}

#[cfg(feature = "f32")]
impl RemAssign for F32 {
  fn rem_assign(&mut self, other: F32) {
    self.0 = self.0 % other.0;
  }
}

#[cfg(feature = "f32")]
impl Zero for F32 {
  fn zero() -> Self {
    F32(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

#[cfg(feature = "f32")]
impl One for F32 {
  fn one() -> Self {
    F32(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}

#[cfg(feature = "f32")]
impl Sub for F32 {
  type Output = F32;
  fn sub(self, other: F32) -> F32 {
    F32(self.0 - other.0)
  }
}

#[cfg(feature = "f32")]
impl SubAssign for F32 {
  fn sub_assign(&mut self, other: F32) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "f32")]
impl Mul for F32 {
  type Output = F32;
  fn mul(self, other: F32) -> F32 {
    F32(self.0 * other.0)
  }
}

#[cfg(feature = "f32")]
impl MulAssign for F32 {
  fn mul_assign(&mut self, other: F32) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "f32")]
impl Div for F32 {
  type Output = F32;
  fn div(self, other: F32) -> F32 {
    F32(self.0 / other.0)
  }
}

#[cfg(feature = "f32")]
impl DivAssign for F32 {
  fn div_assign(&mut self, other: F32) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "f32")]
impl Neg for F32 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F32(-self.0)
  }
}

#[cfg(feature = "f32")]
impl Step for F32 {

  fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
    if start.0 > end.0 {
      return (0, None);
    }
    let diff = end.0 - start.0;
    // Handle special floating-point cases
    match diff.classify() {
      FpCategory::Normal | FpCategory::Zero => {
        if diff.fract() == 0.0 {
          let steps = diff as usize;
          (steps, Some(steps))
        } else {
          (usize::MAX, None)
        }
      }
      _ => (usize::MAX, None),
    }
  }
  
  fn forward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F32(start.0 + count as f32)) 
  }

  fn backward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F32(start.0 - count as f32)) 
  }

  fn forward(start: Self, count: usize) -> Self {
    F32(start.0 + count as f32) 
  }

  fn backward(start: Self, count: usize) -> Self {
    F32(start.0 - count as f32) 
  }
}

#[cfg(feature = "f32")]
impl From<F32> for Value {
  fn from(val: F32) -> Self {
    Value::F32(Ref::new(val))
  }
}

#[cfg(feature = "f32")]
impl Default for F32 {
  fn default() -> Self {
    F32(0.0)
  }
}

// Complex Numbers

#[cfg(feature = "complex")]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ComplexNumber(pub Complex<f64>);

#[cfg(feature = "complex")]
impl fmt::Display for ComplexNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

#[cfg(feature = "complex")]
impl PartialOrd for ComplexNumber {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.0.norm().partial_cmp(&other.0.norm()).unwrap())
  }
}

#[cfg(feature = "complex")]
impl Default for ComplexNumber {
  fn default() -> Self {
    ComplexNumber(Complex::new(0.0, 0.0))
  }
}

#[cfg(feature = "complex")]
impl Eq for ComplexNumber {}

#[cfg(feature = "complex")]
impl Hash for ComplexNumber {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.re.to_bits().hash(state);
    self.0.im.to_bits().hash(state);
  }
}

#[cfg(feature = "complex")]
impl PrettyPrint for ComplexNumber {
  fn pretty_print(&self) -> String {
    if self.0.re == 0.0 {
      return format!("{}i", self.0.im);
    } else if self.0.im >= 0.0 {
      format!("{}+{}i", self.0.re, self.0.im)
    } else {
      format!("{}{}i", self.0.re, self.0.im)
    }
  }
}

#[cfg(feature = "complex")]
impl ComplexNumber {
  pub fn new(real: f64, imag: f64) -> ComplexNumber {
    ComplexNumber(Complex::new(real, imag))
  }

  pub fn to_html(&self) -> String {
    let pretty = self.pretty_print();
    format!("<span class='mech-complex-number'>{}</span>", pretty)
  }

}

#[cfg(feature = "complex")]
#[cfg(feature = "complex")]
impl Add for ComplexNumber {
  type Output = ComplexNumber;
  fn add(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 + other.0)
  }
}

#[cfg(feature = "complex")]
impl Mul for ComplexNumber {
  type Output = ComplexNumber;
  fn mul(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 * other.0)
  }
}

#[cfg(feature = "complex")]
impl Sub for ComplexNumber {
  type Output = ComplexNumber;
  fn sub(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 - other.0)
  }
}

#[cfg(feature = "complex")]
impl Div for ComplexNumber {
  type Output = ComplexNumber;
  fn div(self, other: ComplexNumber) -> ComplexNumber {
    ComplexNumber(self.0 / other.0)
  }
}

#[cfg(feature = "complex")]
impl AddAssign for ComplexNumber {
  fn add_assign(&mut self, other: ComplexNumber) {
    self.0 += other.0;
  }
}

#[cfg(feature = "complex")]
impl SubAssign for ComplexNumber {
  fn sub_assign(&mut self, other: ComplexNumber) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "complex")]
impl MulAssign for ComplexNumber {
  fn mul_assign(&mut self, other: ComplexNumber) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "complex")]
impl DivAssign for ComplexNumber {
  fn div_assign(&mut self, other: ComplexNumber) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "complex")]
impl Zero for ComplexNumber {
  fn zero() -> Self {
    ComplexNumber(Complex::new(0.0, 0.0))
  }
  fn is_zero(&self) -> bool {
    self.0.re == 0.0 && self.0.im == 0.0
  }
}

#[cfg(feature = "complex")]
impl One for ComplexNumber {
  fn one() -> Self {
    ComplexNumber(Complex::new(1.0, 0.0))
  }
  fn is_one(&self) -> bool {
    self.0 == Complex::new(1.0, 0.0)
  }
}

#[cfg(feature = "complex")]
impl Neg for ComplexNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    ComplexNumber(-self.0)
  }
}

// Rational Numbers

#[cfg(feature = "rational")]
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd)]
pub struct RationalNumber(pub Rational64);

#[cfg(feature = "rational")]
impl RationalNumber {
  pub fn new(numer: i64, denom: i64) -> RationalNumber {
    RationalNumber(Rational64::new(numer, denom))
  }

  pub fn from_f64(f: f64) -> Option<RationalNumber> {
    match Rational64::from_f64(f) {
      Some(r) => Some(RationalNumber(r)),
      None => None,
    }
  }

  pub fn to_f64(&self) -> Option<f64> {
    match self.0.to_f64() {
      Some(val) => Some(val),
      None => None,
    }
  }

  pub fn numer(&self) -> &i64 {
    self.0.numer()
  }

  pub fn denom(&self) -> &i64 {
    self.0.denom()
  }
}

#[cfg(feature = "rational")]
impl std::fmt::Display for RationalNumber {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.pretty_print())
  }
}

#[cfg(feature = "rational")]
impl Default for RationalNumber {
  fn default() -> Self {
    RationalNumber(Rational64::default())
  }
}

#[cfg(feature = "rational")]
impl PrettyPrint for RationalNumber {
  fn pretty_print(&self) -> String {
    format!("{}/{}", self.numer(), self.denom())
  }
}

#[cfg(feature = "rational")]
impl Mul<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn mul(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 * other.0)
  }
}

#[cfg(feature = "rational")]
impl One for RationalNumber {
  fn one() -> Self {
    RationalNumber(Rational64::one())
  }
  fn is_one(&self) -> bool {
    self.0.is_one()
  }
}

#[cfg(feature = "rational")]
impl Add<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn add(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 + other.0)
  }
}

#[cfg(feature = "rational")]
impl AddAssign<RationalNumber> for RationalNumber {
  fn add_assign(&mut self, other: RationalNumber) {
    self.0 += other.0;
  }
}

#[cfg(feature = "rational")]
impl Sub<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn sub(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 - other.0)
  }
}

#[cfg(feature = "rational")]
impl Div<RationalNumber> for RationalNumber {
  type Output = RationalNumber;
  fn div(self, other: RationalNumber) -> RationalNumber {
    RationalNumber(self.0 / other.0)
  }
}

#[cfg(feature = "rational")]
impl DivAssign<RationalNumber> for RationalNumber {
  fn div_assign(&mut self, other: RationalNumber) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "rational")]
impl SubAssign<RationalNumber> for RationalNumber {
  fn sub_assign(&mut self, other: RationalNumber) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "rational")]
impl MulAssign<RationalNumber> for RationalNumber {
  fn mul_assign(&mut self, other: RationalNumber) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "rational")]
impl Zero for RationalNumber {
  fn zero() -> Self {
    RationalNumber(Rational64::zero())
  }
  fn is_zero(&self) -> bool {
    self.0.is_zero()
  }
}

#[cfg(feature = "rational")]
impl Neg for RationalNumber {
  type Output = Self;
  fn neg(self) -> Self::Output {
    RationalNumber(-self.0)
  }
}

#[cfg(all(feature = "rational", feature = "f64"))]
impl From<RationalNumber> for F64 {
  fn from(r: RationalNumber) -> Self {
    F64::new(r.0.to_f64().unwrap())
  }
}

#[cfg(all(feature = "rational", feature = "f64"))]
impl From<F64> for RationalNumber {
  fn from(f: F64) -> Self {
    RationalNumber(Rational64::from_f64(f.0).unwrap())
  }
}

#[cfg(all(feature = "f64", feature = "f32"))]
impl From<F32> for F64 {
  fn from(value: F32) -> Self {
    F64::new(value.0 as f64)
  }
}

#[cfg(all(feature = "f64", feature = "f32"))]
impl From<F64> for F32 {
  fn from(value: F64) -> Self {
    F32::new(value.0 as f32)
  }
}

#[cfg(feature = "f32")]
impl ToUsize for F32 {
  fn to_usize(&self) -> usize {
    self.0 as usize
  }
}

#[cfg(feature = "f64")]
impl ToUsize for F64 {
  fn to_usize(&self) -> usize {
    self.0 as usize
  }
}

#[cfg(feature = "rational")]
impl ToUsize for RationalNumber {
  fn to_usize(&self) -> usize {
    self.0.to_integer() as usize
  }
}

#[cfg(feature = "complex")]
impl ToUsize for ComplexNumber {
  fn to_usize(&self) -> usize {
    self.0.norm() as usize
  }
}

#[cfg(feature = "complex")]
impl ToValue for ComplexNumber {
  fn to_value(&self) -> Value {
    Value::ComplexNumber(Ref::new(*self))
  }
}

#[cfg(feature = "rational")]
impl ToValue for RationalNumber {
  fn to_value(&self) -> Value {
    Value::RationalNumber(Ref::new(*self))
  }
}

pub trait PrettyPrint {
  fn pretty_print(&self) -> String;
}

impl PrettyPrint for String {
  fn pretty_print(&self) -> String {
      format!("\"{}\"", self)
  }
}

macro_rules! impl_pretty_print {
  ($t:ty) => {
    #[cfg(feature = "pretty_print")]
    impl PrettyPrint for $t {
      fn pretty_print(&self) -> String {
        format!("{}", self)
      }
    }
  };
}

#[cfg(feature = "bool")]impl_pretty_print!(bool);
#[cfg(feature = "i8")]  impl_pretty_print!(i8);
#[cfg(feature = "i16")] impl_pretty_print!(i16);
#[cfg(feature = "i32")] impl_pretty_print!(i32);
#[cfg(feature = "i64")] impl_pretty_print!(i64);
#[cfg(feature = "i128")]impl_pretty_print!(i128);
#[cfg(feature = "u8")]  impl_pretty_print!(u8);
#[cfg(feature = "u16")] impl_pretty_print!(u16);
#[cfg(feature = "u32")] impl_pretty_print!(u32);
#[cfg(feature = "u64")] impl_pretty_print!(u64);
#[cfg(feature = "u128")]impl_pretty_print!(u128);
#[cfg(feature = "f32")] impl_pretty_print!(F32);
#[cfg(feature = "f64")] impl_pretty_print!(F64);
impl_pretty_print!(usize);
use crate::*;
use crate::value::*;
use crate::nodes::*;

#[cfg(feature = "no_std")]
use core::ops::*;
#[cfg(not(feature = "no_std"))]
use std::ops::*;

#[cfg(feature = "no_std")]
use core::cell::RefCell;
#[cfg(not(feature = "no_std"))]
use std::cell::RefCell;

#[cfg(feature = "no_std")]
use alloc::rc::Rc; 
#[cfg(not(feature = "no_std"))]
use std::rc::Rc;

#[cfg(feature = "no_std")]
use core::num::FpCategory;
#[cfg(not(feature = "no_std"))]
use std::num::FpCategory;

#[cfg(not(feature = "no_std"))]
use std::iter::Step;

use paste::paste;
#[cfg(feature = "math_exp")]
use num_traits::Pow;
#[cfg(any(feature = "f64", feature = "f32", feature = "complex", feature = "rational"))]
use num_traits::{Zero, One};
#[cfg(feature = "math_exp")]
use libm::{pow,powf};


#[cfg(feature = "complex")]
pub mod complex_numbers;
#[cfg(feature = "rational")]
pub mod rational_numbers;
#[cfg(feature = "floats")]
pub mod floats;
#[cfg(feature = "atom")]
pub mod atom;

#[cfg(feature = "complex")]
pub use self::complex_numbers::*;
#[cfg(feature = "rational")]
pub use self::rational_numbers::*;
#[cfg(feature = "floats")]
pub use self::floats::*;
#[cfg(feature = "atom")]
pub use self::atom::*;

// Ref
// ----------------------------------------------------------------------------

pub struct Ref<T>(pub Rc<RefCell<T>>);

impl<T: Debug> Debug for Ref<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let addr = self.0.as_ptr() as usize;
    let emojified = emojify(&(addr as u16));
    write!(f, "@{}: {:#?}", emojified, self.borrow())
  }
}

impl<T> Clone for Ref<T> {
  fn clone(&self) -> Self {
    Ref(self.0.clone())
  }
}

#[cfg(feature = "no_std")]
use core::cell;
#[cfg(not(feature = "no_std"))]
use std::cell;

impl<T> Ref<T> {
  pub fn new(item: T) -> Self { Ref(Rc::new(RefCell::new(item))) }
  pub fn as_ptr(&self) -> *const T { self.0.as_ptr() }
  pub fn as_mut_ptr(&self) -> *mut T { self.0.as_ptr() as *mut T }
  pub fn borrow(&self) -> cell::Ref<'_, T> { self.0.borrow() }
  pub fn borrow_mut(&self) -> cell::RefMut<'_, T> { self.0.borrow_mut() }
  pub fn addr(&self) -> usize { Rc::as_ptr(&self.0) as *const () as usize }
  pub fn id(&self) -> u64 { Rc::as_ptr(&self.0) as *const () as u64 }
}

impl<T: PartialEq> PartialEq for Ref<T> {
  fn eq(&self, other: &Self) -> bool {
    *self.borrow() == *other.borrow()
  }
}
impl<T: PartialEq> Eq for Ref<T> {}

pub type MutableReference = Ref<Value>;
pub type ValRef = Ref<Value>;

pub type MResult<T> = Result<T,MechError2>;

// Pretty Print
// ----------------------------------------------------------------------------

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


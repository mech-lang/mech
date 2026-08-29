use crate::legacy_value::*;
use crate::*;

#[cfg(feature = "no_std")]
use core::cell::RefCell;
#[cfg(not(feature = "no_std"))]
use std::cell::RefCell;

#[cfg(feature = "no_std")]
use alloc::rc::Rc;
#[cfg(not(feature = "no_std"))]
use std::rc::Rc;

#[cfg(feature = "atom")]
pub mod atom;
#[cfg(feature = "complex")]
pub mod complex_numbers;
#[cfg(feature = "rational")]
pub mod rational_numbers;

#[cfg(feature = "atom")]
pub use self::atom::*;
#[cfg(feature = "complex")]
pub use self::complex_numbers::*;
#[cfg(feature = "rational")]
pub use self::rational_numbers::*;

// Ref
// ----------------------------------------------------------------------------

/// An opaque shared handle to a Mech value payload.
///
/// Callers use this API rather than depending on the current backing store so
/// the representation can change without leaking into checkpoint or runtime
/// coordination code.
pub struct Ref<T>(Rc<RefCell<T>>);

impl<T: Debug> Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let addr = self.0.as_ptr() as usize;
        write!(f, "@0x{addr:016x}: {:#?}", self.borrow())
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
    pub fn new(item: T) -> Self {
        Ref(Rc::new(RefCell::new(item)))
    }
    pub fn as_ptr(&self) -> *const T {
        self.0.as_ptr()
    }
    pub fn as_mut_ptr(&self) -> *mut T {
        self.0.as_ptr() as *mut T
    }
    pub fn borrow(&self) -> cell::Ref<'_, T> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> cell::RefMut<'_, T> {
        self.0.borrow_mut()
    }
    pub fn try_borrow(&self) -> Result<cell::Ref<'_, T>, cell::BorrowError> {
        self.0.try_borrow()
    }
    pub fn try_borrow_mut(&self) -> Result<cell::RefMut<'_, T>, cell::BorrowMutError> {
        self.0.try_borrow_mut()
    }
    pub fn same_handle(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub fn addr(&self) -> usize {
        Rc::as_ptr(&self.0) as *const () as usize
    }
    pub fn id(&self) -> u64 {
        Rc::as_ptr(&self.0) as *const () as u64
    }
}

impl<T: PartialEq> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        *self.borrow() == *other.borrow()
    }
}
impl<T: PartialEq> Eq for Ref<T> {}

pub type MutableReference = Ref<LegacyValue>;

pub type MResult<T> = Result<T, MechError>;

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

#[cfg(feature = "bool")]
impl_pretty_print!(bool);
#[cfg(feature = "i8")]
impl_pretty_print!(i8);
#[cfg(feature = "i16")]
impl_pretty_print!(i16);
#[cfg(feature = "i32")]
impl_pretty_print!(i32);
#[cfg(feature = "i64")]
impl_pretty_print!(i64);
#[cfg(feature = "i128")]
impl_pretty_print!(i128);
#[cfg(feature = "u8")]
impl_pretty_print!(u8);
#[cfg(feature = "u16")]
impl_pretty_print!(u16);
#[cfg(feature = "u32")]
impl_pretty_print!(u32);
#[cfg(feature = "u64")]
impl_pretty_print!(u64);
#[cfg(feature = "u128")]
impl_pretty_print!(u128);
#[cfg(feature = "f32")]
impl_pretty_print!(f32);
#[cfg(feature = "f64")]
impl_pretty_print!(f64);
impl_pretty_print!(usize);

#[cfg(all(test, feature = "functions"))]
mod value_cell_tests {
    use super::*;

    fn index_cell(value: usize) -> ValueCell {
        ValueCell::from_exact(value).unwrap()
    }

    #[test]
    fn cloned_cells_preserve_identity_and_share_mutation() {
        let cell = index_cell(1);
        let clone = cell.clone();

        assert!(cell.same_cell(&clone));
        *clone.try_ref::<usize>().unwrap().borrow_mut() = 2;
        assert_eq!(*cell.try_ref::<usize>().unwrap().borrow(), 2);
    }

    #[test]
    fn equal_payloads_do_not_imply_cell_identity() {
        let left = index_cell(1);
        let right = index_cell(1);

        assert!(!left.same_cell(&right));
    }

    #[test]
    fn fallible_borrows_report_conflicts() {
        let cell = index_cell(1);
        let reference = cell.try_ref::<usize>().unwrap();
        {
            let _write = reference.borrow_mut();
            assert!(reference.try_borrow().is_err());
        }
        {
            let _read = reference.borrow();
            assert!(reference.try_borrow_mut().is_err());
        }
    }

    #[test]
    fn debug_output_is_address_free_even_during_borrow_conflicts() {
        let cell = index_cell(1);
        let available = format!("{cell:?}");
        assert!(available.starts_with("ValueCell {"));
        assert!(!available.contains("0x"));

        let reference = cell.try_ref::<usize>().unwrap();
        let _write = reference.borrow_mut();
        let borrowed = format!("{cell:?}");
        assert!(borrowed.contains("Borrowed"));
        assert!(!borrowed.contains("0x"));
    }
}

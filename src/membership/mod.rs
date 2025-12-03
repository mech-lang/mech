// membership module (element of / not element of)
#[macro_use]

#[cfg(feature = "element_of")]
pub mod element_of;
#[cfg(feature = "not_element_of")]
pub mod not_element_of;

#[cfg(feature = "element_of")]
pub use self::element_of::*;
#[cfg(feature = "not_element_of")]
pub use self::not_element_of::*;
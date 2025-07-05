pub mod table;
pub mod matrix;
pub mod logic;
pub mod compare;
pub mod math;

#[cfg(feature = "stdlib")]
pub mod math_update;
#[cfg(feature = "stdlib")]
pub mod stats;
#[cfg(feature = "stdlib")]
pub mod set;
pub(crate) mod math;
pub(crate) mod operation;

#[cfg(all(feature = "resident-artifact", feature = "compiler"))]
pub(crate) mod catalog;
#[cfg(all(feature = "resident-artifact", feature = "compiler"))]
pub(crate) mod closure;

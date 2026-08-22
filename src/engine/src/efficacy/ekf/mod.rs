pub(crate) mod math;
pub(crate) mod operation;

#[cfg(feature = "ekf")]
pub(crate) mod catalog;
#[cfg(all(feature = "resident-artifact", feature = "semantic-compiler"))]
pub(crate) mod closure;

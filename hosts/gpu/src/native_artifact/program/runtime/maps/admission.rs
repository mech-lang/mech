//! Shared map errors, checked arithmetic and ordered finite admission.
use mech_core::{MemoryRuntimeError, ResidentKernelError};

#[derive(Clone, Copy, Debug)]
pub(in crate::native_artifact) struct Limits {
    pub entries: u64,
    pub retained_bytes: u64,
    pub temporary_bytes: u64,
}
#[derive(Debug)]
pub(in crate::native_artifact) enum Error {
    Memory(MemoryRuntimeError),
    Work(ResidentKernelError),
    Limit {
        required: u64,
        limit: u64,
        field: &'static str,
    },
    Overflow,
    Allocation,
    Publication(&'static str),
    Binding(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(error) => write!(out, "owned map memory: {error:?}"),
            Self::Work(error) => write!(out, "owned map work: {error:?}"),
            Self::Limit {
                required,
                limit,
                field,
            } => write!(out, "owned map {field}: {required} exceeds {limit}"),
            Self::Overflow => out.write_str("owned map size arithmetic overflow"),
            Self::Allocation => out.write_str("owned map allocation failed"),
            Self::Publication(reason) => write!(out, "owned map publication: {reason}"),
            Self::Binding(reason) => write!(out, "owned map binding: {reason}"),
        }
    }
}

impl std::error::Error for Error {}
impl From<MemoryRuntimeError> for Error {
    fn from(error: MemoryRuntimeError) -> Self {
        Self::Memory(error)
    }
}
impl From<ResidentKernelError> for Error {
    fn from(error: ResidentKernelError) -> Self {
        Self::Work(error)
    }
}
pub(super) fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}
pub(super) fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}
pub(super) fn check(required: u64, limit: u64, field: &'static str) -> Result<(), Error> {
    if required > limit {
        Err(Error::Limit {
            required,
            limit,
            field,
        })
    } else {
        Ok(())
    }
}

pub(super) fn finite(
    limits: mech_core::MemoryBudgetLimits,
    cap: impl Fn(Option<u64>) -> Result<u64, Error>,
) -> Result<(), Error> {
    for limit in [
        limits.max_output_elements,
        limits.max_output_bytes,
        limits.max_temporary_bytes,
        limits.max_cloned_bytes,
        limits.max_retained_nodes,
        limits.max_compute_work,
        limits.max_comparison_work,
    ] {
        cap(limit)?;
    }
    Ok(())
}

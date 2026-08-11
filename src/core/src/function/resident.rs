use crate::{ResolvedOperationContract, SchemaKey};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResidentValueKind {
    Bool,
    Index,
    F64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResidentShape {
    pub rows: u32,
    pub columns: u32,
}

impl ResidentShape {
    pub const SCALAR: Self = Self {
        rows: 1,
        columns: 1,
    };

    pub fn len(self) -> Option<usize> {
        usize::try_from(self.rows)
            .ok()?
            .checked_mul(usize::try_from(self.columns).ok()?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentPortLayout {
    pub schema: SchemaKey,
    pub kind: ResidentValueKind,
    pub shape: ResidentShape,
}

pub struct ResidentKernelBindRequest<'a> {
    pub contract: &'a ResolvedOperationContract,
    pub inputs: &'a [ResidentPortLayout],
    pub output: ResidentPortLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentKernelBindError {
    UnsupportedContract,
    UnsupportedLayout,
    InvalidParameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentKernelError {
    InvalidInput,
    InvalidOutput,
    InvalidShape,
    Arithmetic,
    IndexOutOfRange { index: u64, upper_bound: u64 },
    IncompleteOutput,
}

#[derive(Clone, Copy, Debug)]
pub enum ResidentValueRef<'a> {
    Bool(&'a [u8]),
    Index(&'a [u64]),
    F64(&'a [f64]),
}

impl ResidentValueRef<'_> {
    pub const fn kind(&self) -> ResidentValueKind {
        match self {
            Self::Bool(_) => ResidentValueKind::Bool,
            Self::Index(_) => ResidentValueKind::Index,
            Self::F64(_) => ResidentValueKind::F64,
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Bool(values) => values.len(),
            Self::Index(values) => values.len(),
            Self::F64(values) => values.len(),
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub enum ResidentValueMut<'a> {
    Bool(&'a mut [u8]),
    Index(&'a mut [u64]),
    F64(&'a mut [f64]),
}

impl ResidentValueMut<'_> {
    pub const fn kind(&self) -> ResidentValueKind {
        match self {
            Self::Bool(_) => ResidentValueKind::Bool,
            Self::Index(_) => ResidentValueKind::Index,
            Self::F64(_) => ResidentValueKind::F64,
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Bool(values) => values.len(),
            Self::Index(values) => values.len(),
            Self::F64(values) => values.len(),
        }
    }
}

pub type ResidentKernelExecutor = for<'a> fn(
    kernel: &BoundResidentKernel,
    inputs: &[ResidentValueRef<'a>],
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;

#[derive(Clone)]
pub struct BoundResidentKernel {
    executor: ResidentKernelExecutor,
    parameters: Box<[u64]>,
}

impl core::fmt::Debug for BoundResidentKernel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("BoundResidentKernel")
            .field("executor", &"<prebound>")
            .field("parameters", &self.parameters)
            .finish()
    }
}

impl BoundResidentKernel {
    pub fn new(executor: ResidentKernelExecutor, parameters: Box<[u64]>) -> Self {
        Self {
            executor,
            parameters,
        }
    }

    pub fn parameters(&self) -> &[u64] {
        &self.parameters
    }

    pub fn execute<'a>(
        &self,
        inputs: &[ResidentValueRef<'a>],
        output: ResidentValueMut<'a>,
    ) -> Result<bool, ResidentKernelError> {
        (self.executor)(self, inputs, output)
    }
}

pub type ResidentKernelFactory =
    fn(&ResidentKernelBindRequest<'_>) -> Result<BoundResidentKernel, ResidentKernelBindError>;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResidentOperationKey {
    pub module_path: Box<[String]>,
    pub operation_name: String,
}

impl ResidentOperationKey {
    pub fn new(module_path: Box<[String]>, operation_name: String) -> Option<Self> {
        if module_path.is_empty()
            || module_path.iter().any(String::is_empty)
            || operation_name.is_empty()
        {
            return None;
        }
        Some(Self {
            module_path,
            operation_name,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ResidentKernelFactoryEntry {
    pub key: ResidentOperationKey,
    pub factory: ResidentKernelFactory,
}

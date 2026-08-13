use crate::{ResolvedOperationContract, SchemaId, SchemaKey};

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
    pub schema_id: SchemaId,
    pub schema_key: SchemaKey,
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

/// Allocation-free view over the prevalidated input locations of one bound
/// resident node. Implementations may borrow disjoint regions from the same
/// typed arena while its output region is mutably borrowed.
pub trait ResidentKernelInputs {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<ResidentValueRef<'_>>;

    fn f64(&self, index: usize) -> Option<&[f64]> {
        let ResidentValueRef::F64(values) = self.get(index)? else {
            return None;
        };
        Some(values)
    }

    fn f64_1(&self) -> Option<[&[f64]; 1]> {
        Some([self.f64(0)?])
    }

    fn f64_2(&self) -> Option<[&[f64]; 2]> {
        Some([self.f64(0)?, self.f64(1)?])
    }

    fn f64_3(&self) -> Option<[&[f64]; 3]> {
        Some([self.f64(0)?, self.f64(1)?, self.f64(2)?])
    }

    fn f64_4(&self) -> Option<[&[f64]; 4]> {
        Some([self.f64(0)?, self.f64(1)?, self.f64(2)?, self.f64(3)?])
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub type ResidentKernelExecutor = for<'a> fn(
    kernel: &BoundResidentKernel,
    inputs: &'a dyn ResidentKernelInputs,
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;

pub type ResidentKernelF64Executor1 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64Executor2 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64Executor3 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    third: &'a [f64],
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64Executor4 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    third: &'a [f64],
    fourth: &'a [f64],
    output: ResidentValueMut<'a>,
) -> Result<bool, ResidentKernelError>;

pub type ResidentKernelF64OutputExecutor1 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    output: &'a mut [f64],
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64OutputExecutor2 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    output: &'a mut [f64],
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64OutputExecutor3 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    third: &'a [f64],
    output: &'a mut [f64],
) -> Result<bool, ResidentKernelError>;
pub type ResidentKernelF64OutputExecutor4 = for<'a> fn(
    kernel: &BoundResidentKernel,
    first: &'a [f64],
    second: &'a [f64],
    third: &'a [f64],
    fourth: &'a [f64],
    output: &'a mut [f64],
) -> Result<bool, ResidentKernelError>;

#[derive(Clone, Copy)]
enum ResidentKernelDispatch {
    Dynamic(ResidentKernelExecutor),
    F64_1(ResidentKernelF64Executor1),
    F64_2(ResidentKernelF64Executor2),
    F64_3(ResidentKernelF64Executor3),
    F64_4(ResidentKernelF64Executor4),
    F64Output1(ResidentKernelF64OutputExecutor1),
    F64Output2(ResidentKernelF64OutputExecutor2),
    F64Output3(ResidentKernelF64OutputExecutor3),
    F64Output4(ResidentKernelF64OutputExecutor4),
}

#[derive(Clone)]
pub struct BoundResidentKernel {
    dispatch: ResidentKernelDispatch,
    parameters: Box<[u64]>,
}

impl core::fmt::Debug for BoundResidentKernel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("BoundResidentKernel")
            .field("dispatch", &"<prebound>")
            .field("parameters", &self.parameters)
            .finish()
    }
}

impl BoundResidentKernel {
    pub fn new(executor: ResidentKernelExecutor, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::Dynamic(executor),
            parameters,
        }
    }

    pub fn new_f64_1(executor: ResidentKernelF64Executor1, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_1(executor),
            parameters,
        }
    }

    pub fn new_f64_2(executor: ResidentKernelF64Executor2, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_2(executor),
            parameters,
        }
    }

    pub fn new_f64_3(executor: ResidentKernelF64Executor3, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_3(executor),
            parameters,
        }
    }

    pub fn new_f64_4(executor: ResidentKernelF64Executor4, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_4(executor),
            parameters,
        }
    }

    pub fn new_f64_output_1(
        executor: ResidentKernelF64OutputExecutor1,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output1(executor),
            parameters,
        }
    }

    pub fn new_f64_output_2(
        executor: ResidentKernelF64OutputExecutor2,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output2(executor),
            parameters,
        }
    }

    pub fn new_f64_output_3(
        executor: ResidentKernelF64OutputExecutor3,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output3(executor),
            parameters,
        }
    }

    pub fn new_f64_output_4(
        executor: ResidentKernelF64OutputExecutor4,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output4(executor),
            parameters,
        }
    }

    pub const fn has_direct_f64_output(&self) -> bool {
        matches!(
            self.dispatch,
            ResidentKernelDispatch::F64Output1(_)
                | ResidentKernelDispatch::F64Output2(_)
                | ResidentKernelDispatch::F64Output3(_)
                | ResidentKernelDispatch::F64Output4(_)
        )
    }

    pub fn parameters(&self) -> &[u64] {
        &self.parameters
    }

    #[inline(always)]
    pub fn execute<'a, I>(
        &self,
        inputs: &'a I,
        output: ResidentValueMut<'a>,
    ) -> Result<bool, ResidentKernelError>
    where
        I: ResidentKernelInputs,
    {
        match self.dispatch {
            ResidentKernelDispatch::Dynamic(executor) => executor(self, inputs, output),
            ResidentKernelDispatch::F64_1(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64_2(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64_3(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64_4(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(3).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64Output1(executor) => {
                let ResidentValueMut::F64(output) = output else {
                    return Err(ResidentKernelError::InvalidOutput);
                };
                executor(
                    self,
                    inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                    output,
                )
            }
            ResidentKernelDispatch::F64Output2(executor) => {
                let ResidentValueMut::F64(output) = output else {
                    return Err(ResidentKernelError::InvalidOutput);
                };
                executor(
                    self,
                    inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                    output,
                )
            }
            ResidentKernelDispatch::F64Output3(executor) => {
                let ResidentValueMut::F64(output) = output else {
                    return Err(ResidentKernelError::InvalidOutput);
                };
                executor(
                    self,
                    inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                    output,
                )
            }
            ResidentKernelDispatch::F64Output4(executor) => {
                let ResidentValueMut::F64(output) = output else {
                    return Err(ResidentKernelError::InvalidOutput);
                };
                executor(
                    self,
                    inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                    inputs.f64(3).ok_or(ResidentKernelError::InvalidInput)?,
                    output,
                )
            }
        }
    }

    #[inline(always)]
    pub fn execute_f64_output<'a, I>(
        &self,
        inputs: &'a I,
        output: &'a mut [f64],
    ) -> Result<bool, ResidentKernelError>
    where
        I: ResidentKernelInputs,
    {
        match self.dispatch {
            ResidentKernelDispatch::F64Output1(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64Output2(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64Output3(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            ResidentKernelDispatch::F64Output4(executor) => executor(
                self,
                inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(2).ok_or(ResidentKernelError::InvalidInput)?,
                inputs.f64(3).ok_or(ResidentKernelError::InvalidInput)?,
                output,
            ),
            _ => Err(ResidentKernelError::InvalidOutput),
        }
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

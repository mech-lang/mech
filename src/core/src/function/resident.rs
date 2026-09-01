use core::any::Any;

use crate::{ResolvedOperationContract, SchemaId, SchemaKey, SchemaTable, ShapeInstance, Value};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String, sync::Arc};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResidentValueKind {
    Bool,
    Index,
    F64,
    String,
    /// Canonical immutable value storage for schemas that do not have a
    /// specialized dense resident lane.
    Snapshot,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ResidentShape {
    pub rows: u32,
    pub columns: u32,
}

/// Selector identity that is proven immutable by the artifact source. This is
/// deliberately narrower than `Value`: binders only need an ordinal or a
/// field identifier to close heterogeneous aggregate access.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResidentResolvedSelector {
    Ordinal(usize),
    Id(u64),
}

/// Canonical semantic selection chosen by specialization. Backends consume
/// this identity directly; they must not reconstruct it from coincidental
/// input and output dimensions.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResolvedSelectionMode {
    Whole,
    LinearScalar,
    LinearGather,
    Rows,
    Columns,
    Rectangle,
    Field { ordinal: u32 },
    TableColumn { ordinal: u32 },
    MapKey,
}

/// Canonical mapping from assignment source positions to selected outputs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u64)]
pub enum ResolvedSourceRouting {
    ScalarBroadcast = 0,
    Positional = 1,
    CompactSelectionOrder = 2,
}

impl ResolvedSourceRouting {
    pub const fn from_parameter(value: u64) -> Option<Self> {
        Some(match value {
            0 => Self::ScalarBroadcast,
            1 => Self::Positional,
            2 => Self::CompactSelectionOrder,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResolvedRangeMode {
    Exclusive,
    ExclusiveIncrement,
    Inclusive,
    InclusiveIncrement,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResolvedReductionMode {
    Rows,
    Columns,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentPortLayout {
    pub schema_id: SchemaId,
    pub schema_key: SchemaKey,
    pub kind: ResidentValueKind,
    pub shape: ResidentShape,
    /// Fully resolved semantic shape for self-describing dynamic values.
    pub shape_instance: ShapeInstance,
    /// Present only when the input is an immutable artifact constant whose
    /// selector identity can be embedded in the execution plan.
    pub resolved_selector: Option<ResidentResolvedSelector>,
}

pub struct ResidentKernelBindRequest<'a> {
    pub contract: &'a ResolvedOperationContract,
    pub schemas: &'a SchemaTable,
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
    String(&'a [String]),
    Snapshot(&'a [Option<Value>]),
}

impl ResidentValueRef<'_> {
    pub const fn kind(&self) -> ResidentValueKind {
        match self {
            Self::Bool(_) => ResidentValueKind::Bool,
            Self::Index(_) => ResidentValueKind::Index,
            Self::F64(_) => ResidentValueKind::F64,
            Self::String(_) => ResidentValueKind::String,
            Self::Snapshot(_) => ResidentValueKind::Snapshot,
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Bool(values) => values.len(),
            Self::Index(values) => values.len(),
            Self::F64(values) => values.len(),
            Self::String(values) => values.len(),
            Self::Snapshot(values) => values.len(),
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
    String(&'a mut [String]),
    Snapshot(&'a mut [Option<Value>]),
}

impl ResidentValueMut<'_> {
    pub const fn kind(&self) -> ResidentValueKind {
        match self {
            Self::Bool(_) => ResidentValueKind::Bool,
            Self::Index(_) => ResidentValueKind::Index,
            Self::F64(_) => ResidentValueKind::F64,
            Self::String(_) => ResidentValueKind::String,
            Self::Snapshot(_) => ResidentValueKind::Snapshot,
        }
    }

    pub const fn len(&self) -> usize {
        match self {
            Self::Bool(values) => values.len(),
            Self::Index(values) => values.len(),
            Self::F64(values) => values.len(),
            Self::String(values) => values.len(),
            Self::Snapshot(values) => values.len(),
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
    snapshot_output: Option<ResidentSnapshotOutput>,
    snapshot_schemas: Option<SchemaTable>,
    retained_state: Option<Arc<dyn Any + Send + Sync>>,
}

#[derive(Clone, Debug)]
pub struct ResidentSnapshotOutput {
    pub schema: SchemaId,
    pub schema_key: SchemaKey,
    pub shape: ShapeInstance,
    pub exact_cardinality: Option<usize>,
    pub maximum_cardinality: Option<usize>,
}

impl core::fmt::Debug for BoundResidentKernel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("BoundResidentKernel")
            .field("dispatch", &"<prebound>")
            .field("parameters", &self.parameters)
            .field("snapshot_output", &self.snapshot_output)
            .field("snapshot_schemas", &self.snapshot_schemas.is_some())
            .field("retained_state", &self.retained_state.is_some())
            .finish()
    }
}

impl BoundResidentKernel {
    pub fn new(executor: ResidentKernelExecutor, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::Dynamic(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_1(executor: ResidentKernelF64Executor1, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_1(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_2(executor: ResidentKernelF64Executor2, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_2(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_3(executor: ResidentKernelF64Executor3, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_3(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_4(executor: ResidentKernelF64Executor4, parameters: Box<[u64]>) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64_4(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_output_1(
        executor: ResidentKernelF64OutputExecutor1,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output1(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_output_2(
        executor: ResidentKernelF64OutputExecutor2,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output2(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_output_3(
        executor: ResidentKernelF64OutputExecutor3,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output3(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn new_f64_output_4(
        executor: ResidentKernelF64OutputExecutor4,
        parameters: Box<[u64]>,
    ) -> Self {
        Self {
            dispatch: ResidentKernelDispatch::F64Output4(executor),
            parameters,
            snapshot_output: None,
            snapshot_schemas: None,
            retained_state: None,
        }
    }

    pub fn with_snapshot_output(mut self, output: ResidentSnapshotOutput) -> Self {
        self.snapshot_output = Some(output);
        self
    }

    pub fn snapshot_output(&self) -> Option<&ResidentSnapshotOutput> {
        self.snapshot_output.as_ref()
    }

    pub fn with_snapshot_schemas(mut self, schemas: SchemaTable) -> Self {
        self.snapshot_schemas = Some(schemas);
        self
    }

    pub fn snapshot_schemas(&self) -> Option<&SchemaTable> {
        self.snapshot_schemas.as_ref()
    }

    /// Retains operation-owned immutable state for the lifetime of an
    /// activated kernel. Dynamic-module kernels use this to keep the loaded
    /// library and validated ABI function together without a process-global
    /// registry.
    pub fn with_retained_state<T>(mut self, state: Arc<T>) -> Self
    where
        T: Any + Send + Sync,
    {
        self.retained_state = Some(state);
        self
    }

    pub fn retained_state<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.retained_state.as_deref()?.downcast_ref()
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

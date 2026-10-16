//! Checked invocation boundary for the private nonprinting native scalar ABI.
use super::abi::{
    ABI_VERSION, NO_NODE, NativeAbiDescriptor, NativeEntryFrame as Frame, STATUS_ABI,
    STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK, primitive_work,
};
use super::memory::{NativeWordStorage, transport_bytes};
use mech_core::{ManagedMemoryBudget, MemoryRuntimeError, NodeId};
use mech_engine::native_artifact::{NativeScalarKind, NativeScalarValue};
use std::ffi::c_void;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeExecutionLimits {
    pub max_work: u64,
    pub max_transport_bytes: u64,
    /// Bounds the generated entry frame, excluding the Rust callback stack.
    pub max_native_stack_bytes: u64,
}
impl Default for NativeExecutionLimits {
    fn default() -> Self {
        Self {
            max_work: 1_000_000,
            max_transport_bytes: 1024 * 1024,
            max_native_stack_bytes: 64 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeExecutionPhase {
    Setup,
    Node(NodeId),
    Completion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeExecutionError {
    InvalidInputs {
        expected: usize,
        actual: usize,
    },
    InvalidOutputs {
        expected: usize,
        actual: usize,
    },
    InputType {
        input: usize,
        expected: NativeScalarKind,
    },
    InvalidOutputBool {
        output: usize,
        value: u64,
    },
    TransportLimit {
        required: u64,
        limit: u64,
    },
    WorkLimit {
        phase: NativeExecutionPhase,
        required: u64,
        limit: u64,
    },
    StackLimit {
        required: u64,
        limit: u64,
    },
    CallbackSequence {
        expected: u64,
        actual: u64,
    },
    Arithmetic {
        node: NodeId,
    },
    InvalidStatus {
        status: u64,
    },
    Descriptor {
        reason: &'static str,
    },
    Memory {
        error: MemoryRuntimeError,
    },
}
impl From<MemoryRuntimeError> for NativeExecutionError {
    fn from(error: MemoryRuntimeError) -> Self {
        Self::Memory { error }
    }
}
impl std::fmt::Display for NativeExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "canonical native execution: {self:?}")
    }
}
impl std::error::Error for NativeExecutionError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeExecutionReport {
    /// Conservative logical work units, not hardware instructions or time.
    pub work: u64,
    pub transport_bytes: u64,
    pub completed_nodes: usize,
}

fn as_u64(count: usize) -> Result<u64, NativeExecutionError> {
    u64::try_from(count).map_err(|_| NativeExecutionError::Descriptor {
        reason: "native ABI count exceeds u64",
    })
}
fn setup_work(words: u64, inputs: u64) -> Result<u64, NativeExecutionError> {
    // Header/descriptor checks, initialized transport words, and both input
    // kind-validation and encoding passes are charged before they occur.
    words
        .checked_mul(2)
        .and_then(|work| work.checked_add(inputs.checked_mul(8)?))
        .and_then(|work| work.checked_add(64))
        .ok_or(NativeExecutionError::Descriptor {
            reason: "native setup work exceeds u64",
        })
}
fn completion_work(outputs: u64) -> Result<u64, NativeExecutionError> {
    // Native final stores plus the complete validation and infallible typed
    // copy-out passes are funded before the first result word is written.
    outputs
        .checked_mul(8)
        .and_then(|work| work.checked_add(32))
        .ok_or(NativeExecutionError::Descriptor {
            reason: "native completion work exceeds u64",
        })
}

struct ExecutionContext<'a> {
    descriptor: &'a NativeAbiDescriptor,
    limit: u64,
    work: u64,
    next_ordinal: u64,
    completion: u64,
    error: Option<NativeExecutionError>,
}
impl ExecutionContext<'_> {
    fn charge(
        &mut self,
        phase: NativeExecutionPhase,
        amount: u64,
    ) -> Result<(), NativeExecutionError> {
        let required = self
            .work
            .checked_add(amount)
            .ok_or(NativeExecutionError::WorkLimit {
                phase,
                required: u64::MAX,
                limit: self.limit,
            })?;
        if required > self.limit {
            return Err(NativeExecutionError::WorkLimit {
                phase,
                required,
                limit: self.limit,
            });
        }
        self.work = required;
        Ok(())
    }

    fn admit(&mut self, ordinal: u64) -> Result<(), NativeExecutionError> {
        if ordinal != self.next_ordinal || ordinal > self.completion {
            return Err(NativeExecutionError::CallbackSequence {
                expected: self.next_ordinal,
                actual: ordinal,
            });
        }
        if ordinal == self.completion {
            self.charge(
                NativeExecutionPhase::Completion,
                completion_work(as_u64(self.descriptor.outputs().len())?)?,
            )?;
        } else {
            let index = usize::try_from(ordinal).map_err(|_| NativeExecutionError::Descriptor {
                reason: "native node ordinal exceeds usize",
            })?;
            let (node, op) =
                self.descriptor
                    .lets()
                    .get(index)
                    .ok_or(NativeExecutionError::Descriptor {
                        reason: "native callback node is absent",
                    })?;
            self.charge(NativeExecutionPhase::Node(*node), primitive_work(*op))?;
        }
        self.next_ordinal =
            self.next_ordinal
                .checked_add(1)
                .ok_or(NativeExecutionError::Descriptor {
                    reason: "native callback cursor exceeds u64",
                })?;
        Ok(())
    }
}

/// The frame and callback are private, synchronous, and generated together.
/// No Rust panic or allocation is permitted on this native callback path.
unsafe extern "C" fn admit_next(context: *mut c_void, ordinal: u64) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: execute installs this exact stack context and retains exclusive
    // access for the whole synchronous native call. It cannot escape the ABI.
    let context = unsafe { &mut *context.cast::<ExecutionContext<'_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    match context.admit(ordinal) {
        Ok(()) => STATUS_OK,
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}

/// Safe callers reach this only through a compiled artifact bound to the
/// immutable descriptor and this module's private native entry signature.
pub(super) fn execute<F>(
    descriptor: &NativeAbiDescriptor,
    entry: F,
    inputs: &[NativeScalarValue],
    outputs: &mut [NativeScalarValue],
    limits: NativeExecutionLimits,
    budget: Option<&ManagedMemoryBudget>,
) -> Result<NativeExecutionReport, NativeExecutionError>
where
    F: FnOnce(*mut Frame) -> u64,
{
    if inputs.len() != descriptor.inputs().len() {
        return Err(NativeExecutionError::InvalidInputs {
            expected: descriptor.inputs().len(),
            actual: inputs.len(),
        });
    }
    if outputs.len() != descriptor.outputs().len() {
        return Err(NativeExecutionError::InvalidOutputs {
            expected: descriptor.outputs().len(),
            actual: outputs.len(),
        });
    }
    if descriptor.stack_bytes() > limits.max_native_stack_bytes {
        return Err(NativeExecutionError::StackLimit {
            required: descriptor.stack_bytes(),
            limit: limits.max_native_stack_bytes,
        });
    }
    let words = inputs
        .len()
        .checked_add(descriptor.lets().len())
        .and_then(|count| count.checked_add(outputs.len()))
        .ok_or(NativeExecutionError::Descriptor {
            reason: "native transport word count exceeds usize",
        })?;
    let bytes = transport_bytes(words)?;
    if bytes > limits.max_transport_bytes {
        return Err(NativeExecutionError::TransportLimit {
            required: bytes,
            limit: limits.max_transport_bytes,
        });
    }
    let input_count = as_u64(inputs.len())?;
    let local_count = as_u64(descriptor.lets().len())?;
    let output_count = as_u64(outputs.len())?;
    let mut context = ExecutionContext {
        descriptor,
        limit: limits.max_work,
        work: 0,
        next_ordinal: 0,
        completion: local_count,
        error: None,
    };
    context.charge(
        NativeExecutionPhase::Setup,
        setup_work(as_u64(words)?, input_count)?,
    )?;
    let mut storage = NativeWordStorage::new(words, limits.max_transport_bytes, budget)?;
    for (index, (value, ty)) in inputs.iter().zip(descriptor.inputs()).enumerate() {
        if !matches!(
            (value, ty.kind()),
            (NativeScalarValue::Bool(_), NativeScalarKind::Bool)
                | (NativeScalarValue::U64(_), NativeScalarKind::U64)
        ) {
            return Err(NativeExecutionError::InputType {
                input: index,
                expected: ty.kind(),
            });
        }
    }
    let (input_words, rest) = storage.values_mut().split_at_mut(inputs.len());
    let (local_words, output_words) = rest.split_at_mut(descriptor.lets().len());
    for (word, input) in input_words.iter_mut().zip(inputs) {
        *word = match input {
            NativeScalarValue::Bool(value) => u64::from(*value),
            NativeScalarValue::U64(value) => *value,
        };
    }
    let input_pointer = input_words.as_ptr();
    let local_pointer = local_words.as_mut_ptr();
    let output_pointer = output_words.as_mut_ptr();
    let context_pointer = (&mut context as *mut ExecutionContext<'_>).cast::<c_void>();
    let mut frame = Frame {
        version: ABI_VERSION,
        input_count,
        local_count,
        output_count,
        inputs: input_pointer,
        locals: local_pointer,
        outputs: output_pointer,
        context: context_pointer,
        admit_next,
        error_node: NO_NODE,
    };
    // SAFETY: the code/descriptor association is constructor-private. All word
    // pointers are disjoint, initialized, bounds-checked managed transport,
    // and retained with this context until the synchronous call returns.
    let status = entry(&mut frame);
    if let Some(error) = context.error.take() {
        return Err(error);
    }
    if frame.version != ABI_VERSION
        || frame.input_count != input_count
        || frame.local_count != local_count
        || frame.output_count != output_count
        || frame.inputs != input_pointer
        || frame.locals != local_pointer
        || frame.outputs != output_pointer
        || frame.context != context_pointer
        || frame.admit_next as usize != admit_next as *const () as usize
    {
        return Err(NativeExecutionError::Descriptor {
            reason: "native entry changed immutable ABI metadata",
        });
    }
    match status {
        STATUS_OK => {
            let expected = local_count
                .checked_add(1)
                .ok_or(NativeExecutionError::Descriptor {
                    reason: "native completion ordinal exceeds u64",
                })?;
            if context.next_ordinal != expected {
                return Err(NativeExecutionError::CallbackSequence {
                    expected,
                    actual: context.next_ordinal,
                });
            }
            if frame.error_node != NO_NODE {
                return Err(NativeExecutionError::Descriptor {
                    reason: "successful native entry retained an error origin",
                });
            }
        }
        STATUS_ARITHMETIC => {
            let index = context
                .next_ordinal
                .checked_sub(1)
                .and_then(|ordinal| usize::try_from(ordinal).ok())
                .and_then(|index| descriptor.lets().get(index));
            let Some((node, _)) = index else {
                return Err(NativeExecutionError::Descriptor {
                    reason: "arithmetic failure has no admitted node",
                });
            };
            if frame.error_node != u64::from(node.get()) {
                return Err(NativeExecutionError::Descriptor {
                    reason: "arithmetic failure origin does not match admitted node",
                });
            }
            return Err(NativeExecutionError::Arithmetic { node: *node });
        }
        STATUS_ABI => {
            return Err(NativeExecutionError::Descriptor {
                reason: "native entry rejected its ABI frame",
            });
        }
        _ => return Err(NativeExecutionError::InvalidStatus { status }),
    }
    // Complete private validation precedes every caller-visible write. Once
    // this pass succeeds, the typed commit below cannot fail or allocate.
    for (index, (word, ty)) in output_words.iter().zip(descriptor.outputs()).enumerate() {
        if ty.kind() == NativeScalarKind::Bool && *word > 1 {
            return Err(NativeExecutionError::InvalidOutputBool {
                output: index,
                value: *word,
            });
        }
    }
    for ((output, word), ty) in outputs
        .iter_mut()
        .zip(output_words.iter())
        .zip(descriptor.outputs())
    {
        *output = match ty.kind() {
            NativeScalarKind::Bool => NativeScalarValue::Bool(*word != 0),
            NativeScalarKind::U64 => NativeScalarValue::U64(*word),
        };
    }
    Ok(NativeExecutionReport {
        work: context.work,
        transport_bytes: bytes,
        completed_nodes: descriptor.lets().len(),
    })
}

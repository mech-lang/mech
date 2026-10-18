//! Safe managed invocation for a compiled CFG. Callbacks admit visits only;
//! they do not evaluate a condition, pattern, operation, or program bytecode.
use super::abi::{
    NO_NODE, NativeEntryFrame as Frame, STATUS_ABI, STATUS_ADMISSION, STATUS_ARITHMETIC, STATUS_OK,
};
use super::control_abi::{NativeMatchDescriptor, STATUS_REJECT, VERSION};
use super::memory::{NativeWordStorage, transport_bytes};
use super::{NativeExecutionError, NativeExecutionLimits};
use mech_core::ManagedMemoryBudget;
use mech_engine::native_artifact::{NativeMatchOrigin, NativeScalarKind, NativeScalarValue};
use std::ffi::c_void;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeMatchExecutionError {
    Base(NativeExecutionError),
    Work {
        origin: Option<NativeMatchOrigin>,
        required: u64,
        limit: u64,
    },
    Visit {
        previous: Option<u32>,
        actual: u64,
    },
    Arithmetic {
        origin: NativeMatchOrigin,
    },
    Unmatched {
        origin: NativeMatchOrigin,
    },
}
impl From<NativeExecutionError> for NativeMatchExecutionError {
    fn from(value: NativeExecutionError) -> Self {
        Self::Base(value)
    }
}
impl std::fmt::Display for NativeMatchExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "canonical native Match execution: {self:?}")
    }
}
impl std::error::Error for NativeMatchExecutionError {}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeMatchExecutionReport {
    pub work: u64,
    pub transport_bytes: u64,
    pub visits: u64,
}
fn invalid(reason: &'static str) -> NativeMatchExecutionError {
    NativeExecutionError::Descriptor { reason }.into()
}
fn add(a: u64, b: u64) -> Result<u64, NativeMatchExecutionError> {
    a.checked_add(b)
        .ok_or_else(|| invalid("Match work overflow"))
}
fn mul(a: u64, b: u64) -> Result<u64, NativeMatchExecutionError> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("Match work overflow"))
}

struct Context<'a> {
    descriptor: &'a NativeMatchDescriptor,
    previous: Option<u32>,
    completed: bool,
    work: u64,
    limit: u64,
    visits: u64,
    error: Option<NativeMatchExecutionError>,
}
impl Context<'_> {
    fn charge(
        &mut self,
        origin: Option<NativeMatchOrigin>,
        work: u64,
    ) -> Result<(), NativeMatchExecutionError> {
        let required = self
            .work
            .checked_add(work)
            .ok_or(NativeMatchExecutionError::Work {
                origin,
                required: u64::MAX,
                limit: self.limit,
            })?;
        if required > self.limit {
            return Err(NativeMatchExecutionError::Work {
                origin,
                required,
                limit: self.limit,
            });
        }
        self.work = required;
        Ok(())
    }
    fn visit(&mut self, raw: u64) -> Result<(), NativeMatchExecutionError> {
        let id = u32::try_from(raw).ok();
        let allowed = !self.completed
            && match self.previous {
                None => id == Some(self.descriptor.entry),
                Some(previous) => {
                    id.is_some()
                        && self.descriptor.visits[previous as usize]
                            .successors
                            .contains(&id)
                }
            };
        if !allowed {
            return Err(NativeMatchExecutionError::Visit {
                previous: self.previous,
                actual: raw,
            });
        }
        let id = id.ok_or_else(|| invalid("Match visit id overflow"))?;
        let site = self
            .descriptor
            .visits
            .get(id as usize)
            .ok_or_else(|| invalid("Match visit outside descriptor"))?;
        let origin = site.origin;
        let work = site.work;
        let complete = site.complete;
        self.charge(Some(origin), work)?;
        self.previous = Some(id);
        self.completed = complete;
        self.visits = add(self.visits, 1)?;
        Ok(())
    }
}
unsafe extern "C" fn admit(context: *mut c_void, id: u64) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: only execute creates this callback context, for this synchronous
    // invocation; generated code cannot retain or replace it.
    let context = unsafe { &mut *context.cast::<Context<'_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    match context.visit(id) {
        Ok(()) => STATUS_OK,
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}

pub(super) fn execute<F: FnOnce(*mut Frame) -> u64>(
    descriptor: &NativeMatchDescriptor,
    entry: F,
    inputs: &[NativeScalarValue],
    outputs: &mut [NativeScalarValue],
    limits: NativeExecutionLimits,
    budget: Option<&ManagedMemoryBudget>,
) -> Result<NativeMatchExecutionReport, NativeMatchExecutionError> {
    if inputs.len() != descriptor.inputs.len() {
        return Err(NativeExecutionError::InvalidInputs {
            expected: descriptor.inputs.len(),
            actual: inputs.len(),
        }
        .into());
    }
    if outputs.len() != descriptor.outputs.len() {
        return Err(NativeExecutionError::InvalidOutputs {
            expected: descriptor.outputs.len(),
            actual: outputs.len(),
        }
        .into());
    }
    if descriptor.stack_bytes > limits.max_native_stack_bytes {
        return Err(NativeExecutionError::StackLimit {
            required: descriptor.stack_bytes,
            limit: limits.max_native_stack_bytes,
        }
        .into());
    }
    let words = inputs
        .len()
        .checked_add(descriptor.value_count)
        .and_then(|n| n.checked_add(outputs.len()))
        .ok_or_else(|| invalid("Match transport count overflow"))?;
    let bytes = transport_bytes(words)?;
    if bytes > limits.max_transport_bytes {
        return Err(NativeExecutionError::TransportLimit {
            required: bytes,
            limit: limits.max_transport_bytes,
        }
        .into());
    }
    let mut context = Context {
        descriptor,
        previous: None,
        completed: false,
        work: 0,
        limit: limits.max_work,
        visits: 0,
        error: None,
    };
    // Setup funds managed zeroing, input type/encoding and generated input and
    // constant copies, plus fixed header validation, before any such work.
    let setup = add(
        128,
        add(
            mul(words as u64, 4)?,
            add(
                mul(inputs.len() as u64, 16)?,
                mul(descriptor.constant_count as u64, 8)?,
            )?,
        )?,
    )?;
    context.charge(None, setup)?;
    let mut storage = NativeWordStorage::new(words, limits.max_transport_bytes, budget)?;
    for (index, (value, ty)) in inputs.iter().zip(&descriptor.inputs).enumerate() {
        if !matches!(
            (value, ty.kind()),
            (NativeScalarValue::Bool(_), NativeScalarKind::Bool)
                | (NativeScalarValue::U64(_), NativeScalarKind::U64)
                | (NativeScalarValue::I64(_), NativeScalarKind::I64)
                | (NativeScalarValue::F64(_), NativeScalarKind::F64)
        ) {
            return Err(NativeExecutionError::InputType {
                input: index,
                expected: ty.kind(),
            }
            .into());
        }
    }
    let (input_words, rest) = storage.values_mut().split_at_mut(inputs.len());
    let (values, output_words) = rest.split_at_mut(descriptor.value_count);
    for (word, value) in input_words.iter_mut().zip(inputs) {
        *word = match value {
            NativeScalarValue::Bool(value) => u64::from(*value),
            NativeScalarValue::U64(value) => *value,
            NativeScalarValue::I64(value) => *value as u64,
            NativeScalarValue::F64(value) => value.bits(),
        };
    }
    let input_pointer = input_words.as_ptr();
    let local_pointer = values.as_mut_ptr();
    let output_pointer = output_words.as_mut_ptr();
    let context_pointer = (&mut context as *mut Context<'_>).cast::<c_void>();
    let mut frame = Frame {
        version: VERSION,
        input_count: inputs.len() as u64,
        local_count: descriptor.value_count as u64,
        output_count: outputs.len() as u64,
        inputs: input_pointer,
        locals: local_pointer,
        outputs: output_pointer,
        context: context_pointer,
        admit_next: admit,
        error_node: NO_NODE,
    };
    let status = entry(&mut frame);
    if let Some(error) = context.error.take() {
        return Err(error);
    }
    if frame.version != VERSION
        || frame.input_count != inputs.len() as u64
        || frame.local_count != descriptor.value_count as u64
        || frame.output_count != outputs.len() as u64
        || frame.inputs != input_pointer
        || frame.locals != local_pointer
        || frame.outputs != output_pointer
        || frame.context != context_pointer
        || frame.admit_next as usize != admit as *const () as usize
    {
        return Err(invalid("Match entry changed immutable frame"));
    }
    match status {
        STATUS_OK => {
            if !context.completed || frame.error_node != NO_NODE {
                return Err(invalid("Match entry omitted atomic completion"));
            }
        }
        STATUS_ARITHMETIC | STATUS_REJECT => {
            let id = context
                .previous
                .ok_or_else(|| invalid("Match failure before admitted visit"))?;
            let site = &descriptor.visits[id as usize];
            if frame.error_node != id as u64 {
                return Err(invalid("Match failure origin mismatch"));
            }
            if status == STATUS_ARITHMETIC {
                if !site.arithmetic {
                    return Err(invalid("Match arithmetic outside primitive"));
                }
                return Err(NativeMatchExecutionError::Arithmetic {
                    origin: site.origin,
                });
            }
            return Err(NativeMatchExecutionError::Unmatched {
                origin: site.origin,
            });
        }
        _ => return Err(NativeExecutionError::InvalidStatus { status }.into()),
    }
    for (index, (&word, ty)) in output_words.iter().zip(&descriptor.outputs).enumerate() {
        if ty.kind() == NativeScalarKind::Bool && word > 1 {
            return Err(NativeExecutionError::InvalidOutputBool {
                output: index,
                value: word,
            }
            .into());
        }
    }
    // All validation is complete. This final pass cannot fail or allocate.
    for ((target, &word), ty) in outputs
        .iter_mut()
        .zip(output_words.iter())
        .zip(&descriptor.outputs)
    {
        *target = match ty.kind() {
            NativeScalarKind::Bool => NativeScalarValue::Bool(word != 0),
            NativeScalarKind::U64 => NativeScalarValue::U64(word),
            NativeScalarKind::I64 => NativeScalarValue::I64(word as i64),
            NativeScalarKind::F64 => {
                NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_bits(word))
            }
        };
    }
    Ok(NativeMatchExecutionReport {
        work: context.work,
        transport_bytes: bytes,
        visits: context.visits,
    })
}

#[cfg(test)]
mod tests {
    use super::super::JitMatchArtifact;
    use super::*;
    use mech_engine::CanonicalSourceFrontend;
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    fn descriptor() -> NativeMatchDescriptor {
        let parsed = parse_canonical_document(
            TextSnapshot::new(
                DocumentId(0x4d564953),
                Revision(1),
                "(flag<bool>? | true => value<u64> + 1u64 | false => 0u64)\n",
            )
            .unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        let catalog = mech_stdlib::source_catalog();
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let plan =
            mech_engine::native_artifact::lower_scalar_match_artifact(&artifact, &catalog).unwrap();
        JitMatchArtifact::compile(&plan)
            .unwrap()
            .descriptor()
            .clone()
    }
    #[test]
    fn invalid_cfg_visit_is_rejected_without_publication() {
        let descriptor = descriptor();
        let inputs = descriptor
            .inputs()
            .iter()
            .map(|ty| match ty.kind() {
                NativeScalarKind::Bool => NativeScalarValue::Bool(true),
                NativeScalarKind::U64 => NativeScalarValue::U64(8),
                NativeScalarKind::I64 => NativeScalarValue::I64(8),
                NativeScalarKind::F64 => {
                    NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_f64(8.0))
                }
            })
            .collect::<Vec<_>>();
        let mut outputs = vec![NativeScalarValue::U64(77); descriptor.outputs().len()];
        let error = execute(
            &descriptor,
            |frame| {
                // SAFETY: the injected entry uses only the live callback and
                // context supplied by execute; it never accesses a word arena.
                unsafe { ((*frame).admit_next)((*frame).context, u64::MAX) }
            },
            &inputs,
            &mut outputs,
            NativeExecutionLimits::default(),
            None,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            NativeMatchExecutionError::Visit {
                previous: None,
                actual: u64::MAX
            }
        ));
        assert!(
            outputs
                .iter()
                .all(|value| *value == NativeScalarValue::U64(77))
        );
        let mut context = Context {
            descriptor: &descriptor,
            previous: None,
            completed: false,
            work: 0,
            limit: u64::MAX,
            visits: 0,
            error: None,
        };
        context.visit(descriptor.entry as u64).unwrap();
        let before = context.work;
        assert!(matches!(
            context.visit(descriptor.entry as u64),
            Err(NativeMatchExecutionError::Visit { .. })
        ));
        assert_eq!(context.work, before);
        assert_eq!(context.visits, 1);
    }
    #[test]
    fn setup_and_memory_denial_precede_injected_entry_and_preserve_outputs() {
        let descriptor = descriptor();
        let inputs = descriptor
            .inputs()
            .iter()
            .map(|ty| match ty.kind() {
                NativeScalarKind::Bool => NativeScalarValue::Bool(true),
                NativeScalarKind::U64 => NativeScalarValue::U64(8),
                NativeScalarKind::I64 => NativeScalarValue::I64(8),
                NativeScalarKind::F64 => {
                    NativeScalarValue::F64(mech_core::snapshot::F64Bits::from_f64(8.0))
                }
            })
            .collect::<Vec<_>>();
        for (limits, budget) in [
            (
                NativeExecutionLimits {
                    max_work: 0,
                    ..NativeExecutionLimits::default()
                },
                None,
            ),
            (
                NativeExecutionLimits::default(),
                Some(ManagedMemoryBudget::new(0)),
            ),
        ] {
            let entered = std::cell::Cell::new(false);
            let mut outputs = vec![NativeScalarValue::U64(77); descriptor.outputs().len()];
            assert!(
                execute(
                    &descriptor,
                    |_| {
                        entered.set(true);
                        STATUS_OK
                    },
                    &inputs,
                    &mut outputs,
                    limits,
                    budget.as_ref()
                )
                .is_err()
            );
            assert!(!entered.get());
            assert!(
                outputs
                    .iter()
                    .all(|value| *value == NativeScalarValue::U64(77))
            );
        }
    }
}

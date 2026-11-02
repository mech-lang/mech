//! Synchronous private ABI callbacks. No mutable lane borrow crosses native execution.
use super::*;
#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime) fn test_corrupt_prefix_checkpoint(
    frame: *mut Frame,
    field: u8,
) {
    // SAFETY: only the synchronous owning Prepared test driver calls this,
    // after the retained checked JIT returned. Descriptor offsets name its
    // exact admitted lanes; no poisoned word is subsequently executed.
    unsafe {
        let frame = &mut *frame;
        assert_eq!(
            frame.phase,
            crate::native_artifact::program::protocol::abi::PHASE_PREFIX
        );
        let context = &*frame.base.context.cast::<Invocation<'_, '_, '_>>();
        let descriptor = context.descriptor;
        let region = descriptor.metadata().prefix.as_ref().unwrap();
        let borrowed = &descriptor.metadata().borrowed_strings.regions[region.borrowed];
        match field {
            0..=2 => {
                let word = match field {
                    0 => borrowed.ready,
                    1 => borrowed.end - 1,
                    _ => {
                        descriptor.metadata().closed_tails.regions[descriptor
                            .metadata()
                            .closed_tails
                            .sites[region.tail.header() as usize]
                            .unwrap()]
                        .words
                            + 1
                    }
                };
                assert!(word < frame.base.local_count as usize);
                *frame.base.locals.add(word) = 1;
            }
            3 => {
                assert!(frame.argument_capacity > 0);
                *frame.arguments = 1;
            }
            4 => frame.resume_site ^= 1,
            5 => frame.available_lines ^= 1,
            6 => frame.input_eof ^= 1,
            _ => panic!("unknown checkpoint corruption"),
        }
    }
}
pub(in crate::native_artifact::program::runtime) unsafe extern "C" fn admit(
    context: *mut c_void,
    site: u64,
) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: execute retains this invocation context and disjoint backing for
    // the synchronous private entry; no pointer escapes the call.
    let context = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    admit_current(context, site)
}
pub(in crate::native_artifact::program::runtime) unsafe extern "C" fn helper(
    context: *mut c_void,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    if context.is_null() {
        return STATUS_ABI;
    }
    // SAFETY: the private entry receives only execute's retained context.
    let context = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
    if context.error.is_some() {
        return STATUS_ADMISSION;
    }
    let operation = match GlobalHelper::decode(operation) {
        GlobalHelper::AdmitMove => {
            let status = admit_current(context, site);
            if status != STATUS_OK {
                return status;
            }
            GlobalHelper::Move.raw()
        }
        GlobalHelper::AdmitStructuralObservation => {
            let status = admit_current(context, site);
            if status != STATUS_OK {
                return status;
            }
            GlobalHelper::StructuralObservation.raw()
        }
        GlobalHelper::AdmitNumericSegmentBegin => {
            let outcome = admission_word(context, site);
            let scalar_ready = match &outcome {
                Ok(Some(word)) => *word,
                _ => 0,
            };
            let status = finish_callback(context, outcome);
            if status != STATUS_OK {
                return status;
            }
            // The original native capacity guard fails after admission but
            // before helper-pointer validation or any numeric state change.
            if first == 0 {
                return STATUS_OK;
            }
            let outcome = if result.is_null() || result != context.result {
                Err(invalid("helper result pointer mismatch"))
            } else if first != 1 {
                Err(invalid("numeric admission capacity word is not Boolean"))
            } else {
                context
                    .helper(
                        site,
                        GlobalHelper::NumericSegmentBegin.raw(),
                        [0, second, third],
                    )
                    .map(|word| word.map(|selected| selected | (scalar_ready << 1)))
            };
            return finish_callback(context, outcome);
        }
        _ => operation,
    };
    let outcome = if result.is_null() || result != context.result {
        Err(invalid("helper result pointer mismatch"))
    } else {
        context.helper(site, operation, [first, second, third])
    };
    finish_callback(context, outcome)
}

#[inline]
fn admit_current(context: &mut Invocation<'_, '_, '_>, site: u64) -> u64 {
    // Finish the complete original admission, including its optional scalar
    // decision word, before validating any helper pointer or operand. A fused
    // callback grants no permission beyond the same current visit.
    let outcome = admission_word(context, site);
    finish_callback(context, outcome)
}

#[inline(always)]
fn admission_word(
    context: &mut Invocation<'_, '_, '_>,
    site: u64,
) -> Result<Option<u64>, NativeProgramExecutionError> {
    context
        .visit(site)
        .map(|()| context.scalar_admission_word(site))
}

#[inline]
fn finish_callback(
    context: &mut Invocation<'_, '_, '_>,
    outcome: Result<Option<u64>, NativeProgramExecutionError>,
) -> u64 {
    match outcome {
        Ok(word) => {
            if let Some(word) = word {
                let result = context.result;
                if result.is_null() {
                    context.error = Some(invalid("missing private callback result lane"));
                    return STATUS_ABI;
                }
                // SAFETY: the owning session registers this exact one-word
                // result lane, disjoint from control and Value arenas. Helper
                // calls additionally check the caller's pointer equality.
                unsafe {
                    *result = word;
                }
            }
            STATUS_OK
        }
        Err(error) => {
            context.error = Some(error);
            STATUS_ADMISSION
        }
    }
}

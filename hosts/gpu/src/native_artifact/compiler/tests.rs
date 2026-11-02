use crate::native_artifact::compiler::*;
use core::ffi::c_void;
use mech_engine::CanonicalSourceFrontend;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};
use std::sync::atomic::{AtomicU64, Ordering};
fn bool_jit() -> JitArtifact {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x434e554e), Revision(1), "!flag<bool>\n").unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let artifact = CanonicalSourceFrontend
        .compile_document_with_catalog(&syntax, mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let plan = mech_engine::native_artifact::lower_pure_native_artifact(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    JitArtifact::compile(&plan).unwrap()
}
unsafe extern "C" fn count_admission(context: *mut c_void, _: u64) -> u64 {
    // SAFETY: this private test retains the exact AtomicU64 through call.
    unsafe { &*context.cast::<AtomicU64>() }.fetch_add(1, Ordering::SeqCst);
    STATUS_OK
}
#[test]
fn raw_private_abi_rejects_noncanonical_bool_and_bad_header_before_callbacks_or_stores() {
    let jit = bool_jit();
    let calls = AtomicU64::new(0);
    let mut locals = vec![0; jit.descriptor().lets().len()];
    let mut outputs = vec![0xbeef; jit.descriptor().outputs().len()];
    for word in [2, u64::MAX] {
        let input = [word];
        let mut frame = NativeEntryFrame {
            version: ABI_VERSION,
            input_count: 1,
            local_count: locals.len() as u64,
            output_count: outputs.len() as u64,
            inputs: input.as_ptr(),
            locals: locals.as_mut_ptr(),
            outputs: outputs.as_mut_ptr(),
            context: (&calls as *const AtomicU64).cast_mut().cast(),
            admit_next: count_admission,
            error_node: NO_NODE,
        };
        // SAFETY: private test supplies exact buffers/counts; invalid Bool
        // bits are intentionally rejected by the generated ABI guard.
        assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(outputs.iter().all(|v| *v == 0xbeef));
    }
    let input = [1];
    for bad_version in [true, false] {
        let mut frame = NativeEntryFrame {
            version: if bad_version {
                ABI_VERSION + 1
            } else {
                ABI_VERSION
            },
            input_count: if bad_version { 1 } else { 0 },
            local_count: locals.len() as u64,
            output_count: outputs.len() as u64,
            inputs: input.as_ptr(),
            locals: locals.as_mut_ptr(),
            outputs: outputs.as_mut_ptr(),
            context: (&calls as *const AtomicU64).cast_mut().cast(),
            admit_next: count_admission,
            error_node: NO_NODE,
        };
        // SAFETY: allocations still cover the true signature exactly.
        assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(outputs.iter().all(|v| *v == 0xbeef));
    }
}
static ENTRY_CALLS: AtomicU64 = AtomicU64::new(0);
unsafe extern "C" fn never_admitted_entry(_: *mut NativeEntryFrame) -> u64 {
    ENTRY_CALLS.fetch_add(1, Ordering::SeqCst);
    STATUS_OK
}
#[test]
fn denied_setup_never_invokes_an_entry_or_changes_outputs() {
    let jit = bool_jit();
    let mut output = vec![NativeScalarValue::U64(99); jit.descriptor().outputs().len()];
    let before = output.clone();
    ENTRY_CALLS.store(0, Ordering::SeqCst);
    // The real source-derived descriptor is retained. Only an internal
    // entry-counter fixture is substituted to prove setup denies first.
    let error = crate::native_artifact::runtime::execute(
        jit.descriptor(),
        |frame| unsafe { never_admitted_entry(frame) },
        &[NativeScalarValue::Bool(true)],
        &mut output,
        NativeExecutionLimits {
            max_work: 0,
            ..NativeExecutionLimits::default()
        },
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        NativeExecutionError::WorkLimit {
            phase: crate::native_artifact::runtime::NativeExecutionPhase::Setup,
            ..
        }
    ));
    assert_eq!(ENTRY_CALLS.load(Ordering::SeqCst), 0);
    assert_eq!(output, before);
}
struct CompletionDenial {
    calls: u64,
    stop: u64,
}
unsafe extern "C" fn deny_completion(context: *mut c_void, ordinal: u64) -> u64 {
    // SAFETY: this private test retains the exact fixed context through call.
    let context = unsafe { &mut *context.cast::<CompletionDenial>() };
    context.calls += 1;
    if ordinal == context.stop {
        STATUS_ADMISSION
    } else {
        STATUS_OK
    }
}
#[test]
fn generated_completion_admission_precedes_every_private_output_store() {
    let jit = bool_jit();
    let input = [1_u64];
    let mut locals = vec![77; jit.descriptor().lets().len()];
    let mut outputs = vec![0xbeef; jit.descriptor().outputs().len()];
    let mut denial = CompletionDenial {
        calls: 0,
        stop: jit.descriptor().lets().len() as u64,
    };
    let mut frame = NativeEntryFrame {
        version: ABI_VERSION,
        input_count: 1,
        local_count: locals.len() as u64,
        output_count: outputs.len() as u64,
        inputs: input.as_ptr(),
        locals: locals.as_mut_ptr(),
        outputs: outputs.as_mut_ptr(),
        context: (&mut denial as *mut CompletionDenial).cast(),
        admit_next: deny_completion,
        error_node: NO_NODE,
    };
    // SAFETY: this test supplies exact private buffers and finite callback;
    // only its completion phase intentionally denies admission.
    assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ADMISSION);
    assert_eq!(denial.calls, jit.descriptor().lets().len() as u64 + 1);
    assert_eq!(locals, [0]);
    assert!(outputs.iter().all(|word| *word == 0xbeef));
}

//! Exact narrow transport bounds at the raw entry and safe publication boundary.
use super::{
    ABI_VERSION, JitArtifact, NO_NODE, NativeEntryFrame, NativeExecutionError,
    NativeExecutionLimits, NativeScalarKind, NativeScalarValue, STATUS_ABI, STATUS_OK,
};
use std::{
    ffi::c_void,
    sync::atomic::{AtomicU64, Ordering},
};

fn identity(width: u32) -> JitArtifact {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    let source = format!("value := input<u{width}>\nvalue\n");
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4e415252), Revision(0), source.as_str()).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let syntax = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let artifact = mech_engine::CanonicalSourceFrontend
        .compile_interactive_document_with_catalog(&syntax, mech_stdlib::source_catalog())
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

unsafe extern "C" fn count(context: *mut c_void, _: u64) -> u64 {
    // SAFETY: this synchronous private test pins the exact AtomicU64 owner.
    unsafe { &*context.cast::<AtomicU64>() }.fetch_add(1, Ordering::SeqCst);
    STATUS_OK
}

#[test]
fn narrow_raw_entries_reject_excess_bits_before_admission_or_stores() {
    for width in [8, 16, 32] {
        let jit = identity(width);
        let maximum = (1u64 << width) - 1;
        let calls = AtomicU64::new(0);
        let mut locals = vec![0; jit.descriptor().lets().len()];
        let mut output = vec![0xbeef; jit.descriptor().outputs().len()];
        for word in [maximum + 1, u64::MAX] {
            let input = [word];
            let mut frame = NativeEntryFrame {
                version: ABI_VERSION,
                input_count: 1,
                local_count: locals.len() as u64,
                output_count: output.len() as u64,
                inputs: input.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: output.as_mut_ptr(),
                context: (&calls as *const AtomicU64).cast_mut().cast(),
                admit_next: count,
                error_node: NO_NODE,
            };
            // SAFETY: real checked code, exact disjoint buffers and counts;
            // only this input word intentionally exceeds its declared width.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            assert!(output.iter().all(|word| *word == 0xbeef));
        }
        let input = [maximum];
        let mut frame = NativeEntryFrame {
            version: ABI_VERSION,
            input_count: 1,
            local_count: locals.len() as u64,
            output_count: output.len() as u64,
            inputs: input.as_ptr(),
            locals: locals.as_mut_ptr(),
            outputs: output.as_mut_ptr(),
            context: (&calls as *const AtomicU64).cast_mut().cast(),
            admit_next: count,
            error_node: NO_NODE,
        };
        // SAFETY: the same exact private buffers now contain a valid payload.
        assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_OK);
        assert!(calls.load(Ordering::SeqCst) > 0);
        assert!(output.iter().all(|word| *word == maximum));
    }
}

#[test]
fn narrow_output_validation_rejects_a_late_bad_word_without_partial_publication() {
    for (width, kind) in [
        (8, NativeScalarKind::U8),
        (16, NativeScalarKind::U16),
        (32, NativeScalarKind::U32),
    ] {
        let jit = identity(width);
        assert!(jit.descriptor().lets().is_empty());
        let output_count = jit.descriptor().outputs().len();
        assert!(output_count >= 2);
        let bad_output = output_count - 1;
        let input = [NativeScalarValue::from_bits(kind, 7).unwrap()];
        let bad = 1u64 << width;
        let sentinel = vec![NativeScalarValue::U64(0xbeef); output_count];
        let mut output = sentinel.clone();
        let error = super::super::runtime::execute(
            jit.descriptor(),
            |frame| {
                // SAFETY: execute owns these exact private output lanes. This test
                // simulates malformed machine output after the genuine completion
                // admission, while leaving every header and count unchanged.
                unsafe {
                    let frame = &mut *frame;
                    let status = (frame.admit_next)(frame.context, 0);
                    if status != STATUS_OK {
                        return status;
                    }
                    for index in 0..output_count {
                        *frame.outputs.add(index) = 7;
                    }
                    *frame.outputs.add(bad_output) = bad;
                    STATUS_OK
                }
            },
            &input,
            &mut output,
            NativeExecutionLimits::default(),
            None,
        )
        .unwrap_err();
        assert_eq!(
            error,
            NativeExecutionError::InvalidOutputUnsigned {
                output: bad_output,
                kind,
                value: bad
            }
        );
        assert_eq!(output, sentinel);
        jit.evaluate_into(&input, &mut output, NativeExecutionLimits::default(), None)
            .unwrap();
        assert_eq!(output, vec![input[0]; output_count]);
    }
}

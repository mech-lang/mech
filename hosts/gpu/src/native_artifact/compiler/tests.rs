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
fn unsigned_native_checks_match_full_width_arithmetic_and_keep_admission_atomic() {
    let edges = [
        0,
        1,
        2,
        3,
        255,
        256,
        65535,
        65536,
        (1u64 << 32) - 1,
        1u64 << 32,
        (1u64 << 32) + 1,
        u64::MAX / 2,
        u64::MAX / 2 + 1,
        u64::MAX - 1,
        u64::MAX,
    ];
    let mut pairs = edges
        .iter()
        .flat_map(|&a| edges.iter().map(move |&b| (a, b)))
        .collect::<Vec<_>>();
    let mut seed = 0x9e3779b97f4a7c15u64;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let a = seed;
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        pairs.push((a, seed));
    }
    let catalog = mech_stdlib::source_catalog();
    for operator in ["+", "-", "*"] {
        let text = format!("left<u64> {operator} right<u64>\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x554e5349474e4544), Revision(1), text.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean());
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog(
                &DocumentSyntax::cast(parsed.syntax()).unwrap(),
                catalog.clone(),
            )
            .unwrap()
            .compile_artifact()
            .unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(
            &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for artifact in [artifact, decoded] {
            let plan =
                mech_engine::native_artifact::lower_pure_native_artifact(&artifact, &catalog)
                    .unwrap();
            let jit = JitArtifact::compile(&plan).unwrap();
            assert_eq!(jit.descriptor().inputs().len(), 2);
            assert_eq!(jit.descriptor().outputs().len(), 1);
            let calls = AtomicU64::new(0);
            let mut locals = vec![0; jit.descriptor().lets().len()];
            let mut outputs = [0xbeefu64];
            for &(a, c) in &pairs {
                let expected = match operator {
                    "+" => a.checked_add(c),
                    "-" => a.checked_sub(c),
                    _ => a.checked_mul(c),
                };
                outputs[0] = 0xbeef;
                calls.store(0, Ordering::SeqCst);
                let inputs = [a, c];
                let mut frame = NativeEntryFrame {
                    version: ABI_VERSION,
                    input_count: 2,
                    local_count: locals.len() as u64,
                    output_count: 1,
                    inputs: inputs.as_ptr(),
                    locals: locals.as_mut_ptr(),
                    outputs: outputs.as_mut_ptr(),
                    context: (&calls as *const AtomicU64).cast_mut().cast(),
                    admit_next: count_admission,
                    error_node: NO_NODE,
                };
                // SAFETY: exact source-derived signature and live private buffers;
                // every U64 bit pattern is canonical at this ABI.
                let status = unsafe { (jit.entry)(&mut frame) };
                if let Some(value) = expected {
                    assert_eq!(status, STATUS_OK, "{a} {operator} {c}");
                    assert_eq!(outputs[0], value, "{a} {operator} {c}");
                    assert_eq!(calls.load(Ordering::SeqCst), locals.len() as u64 + 1);
                } else {
                    assert_eq!(status, STATUS_ARITHMETIC, "{a} {operator} {c}");
                    assert_eq!(outputs[0], 0xbeef);
                    assert_ne!(frame.error_node, NO_NODE);
                    assert!((1..=locals.len() as u64).contains(&calls.load(Ordering::SeqCst)));
                }
            }
            let inputs = [u64::MAX, u64::MAX];
            let mut denial = CompletionDenial { calls: 0, stop: 0 };
            outputs[0] = 0xbeef;
            let mut frame = NativeEntryFrame {
                version: ABI_VERSION,
                input_count: 2,
                local_count: locals.len() as u64,
                output_count: 1,
                inputs: inputs.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: outputs.as_mut_ptr(),
                context: (&mut denial as *mut CompletionDenial).cast(),
                admit_next: deny_completion,
                error_node: NO_NODE,
            };
            // SAFETY: exact buffers/signature, with the first source admission denied.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ADMISSION);
            assert_eq!(denial.calls, 1);
            assert_eq!(outputs[0], 0xbeef);
        }
    }
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

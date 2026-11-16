//! Exact narrow transport bounds at the raw entry and safe publication boundary.
use super::{
    ABI_VERSION, JitArtifact, NO_NODE, NativeEntryFrame, NativeExecutionError,
    NativeExecutionLimits, NativeScalarKind, NativeScalarValue, STATUS_ABI, STATUS_ADMISSION,
    STATUS_ARITHMETIC, STATUS_OK,
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

unsafe extern "C" fn deny(context: *mut c_void, _: u64) -> u64 {
    // SAFETY: the caller pins this exact atomic through the synchronous entry.
    unsafe { &*context.cast::<AtomicU64>() }.fetch_add(1, Ordering::SeqCst);
    STATUS_ADMISSION
}

#[test]
fn checked_u32_native_arithmetic_preserves_edges_errors_admission_and_recovery() {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    let edges = [
        0,
        1,
        2,
        3,
        255,
        256,
        65535,
        65536,
        u32::MAX / 2,
        u32::MAX / 2 + 1,
        u32::MAX - 1,
        u32::MAX,
    ];
    let mut pairs = edges
        .iter()
        .flat_map(|&a| edges.iter().map(move |&b| (a, b)))
        .collect::<Vec<_>>();
    let mut seed = 0x71e432a9u32;
    for _ in 0..4096 {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let left = seed;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        pairs.push((left, seed));
        // Include many successful products as well as full-width failures.
        pairs.push((left & 0xffff, seed & 0xffff));
    }
    let catalog = mech_stdlib::source_catalog();
    for operator in ["*", "/", "%"] {
        let text = format!("left<u32> {operator} right<u32>\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5533324152495448), Revision(0), text.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let artifact = mech_engine::CanonicalSourceFrontend
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
            let mut outputs = [u64::MAX];
            for &(left, right) in &pairs {
                let expected = match operator {
                    "*" => left.checked_mul(right),
                    "/" => left.checked_div(right),
                    _ => left.checked_rem(right),
                };
                let inputs = [u64::from(left), u64::from(right)];
                outputs[0] = u64::MAX;
                calls.store(0, Ordering::SeqCst);
                let mut frame = NativeEntryFrame {
                    version: ABI_VERSION,
                    input_count: 2,
                    local_count: locals.len() as u64,
                    output_count: 1,
                    inputs: inputs.as_ptr(),
                    locals: locals.as_mut_ptr(),
                    outputs: outputs.as_mut_ptr(),
                    context: (&calls as *const AtomicU64).cast_mut().cast(),
                    admit_next: count,
                    error_node: NO_NODE,
                };
                // SAFETY: real source-derived code, canonical U32 inputs and
                // exact disjoint scratch/output extents for this private ABI.
                let status = unsafe { (jit.entry)(&mut frame) };
                if let Some(value) = expected {
                    assert_eq!(status, STATUS_OK, "{left} {operator} {right}");
                    assert_eq!(outputs[0], u64::from(value));
                } else {
                    assert_eq!(status, STATUS_ARITHMETIC, "{left} {operator} {right}");
                    assert_eq!(outputs[0], u64::MAX);
                    assert_ne!(frame.error_node, NO_NODE);
                }
            }
            for inputs in [[u64::from(u32::MAX) + 1, 0], [1, u64::MAX]] {
                outputs[0] = u64::MAX;
                calls.store(0, Ordering::SeqCst);
                let mut frame = NativeEntryFrame {
                    version: ABI_VERSION,
                    input_count: 2,
                    local_count: locals.len() as u64,
                    output_count: 1,
                    inputs: inputs.as_ptr(),
                    locals: locals.as_mut_ptr(),
                    outputs: outputs.as_mut_ptr(),
                    context: (&calls as *const AtomicU64).cast_mut().cast(),
                    admit_next: count,
                    error_node: NO_NODE,
                };
                // SAFETY: allocations are exact; only an input payload is
                // deliberately malformed and must be rejected before admission.
                assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
                assert_eq!(calls.load(Ordering::SeqCst), 0);
                assert_eq!(outputs[0], u64::MAX);
            }
            let inputs = [u64::from(u32::MAX), 0];
            let mut frame = NativeEntryFrame {
                version: ABI_VERSION,
                input_count: 2,
                local_count: locals.len() as u64,
                output_count: 1,
                inputs: inputs.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: outputs.as_mut_ptr(),
                context: (&calls as *const AtomicU64).cast_mut().cast(),
                admit_next: deny,
                error_node: NO_NODE,
            };
            // SAFETY: exact valid buffers; refusal must precede arithmetic and
            // publication, including division/remainder with a zero divisor.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ADMISSION);
            assert_eq!(outputs[0], u64::MAX);
            let input = [
                NativeScalarValue::from_bits(NativeScalarKind::U32, 42).unwrap(),
                NativeScalarValue::from_bits(NativeScalarKind::U32, 6).unwrap(),
            ];
            let mut result = [NativeScalarValue::U64(99)];
            jit.evaluate_into(&input, &mut result, NativeExecutionLimits::default(), None)
                .unwrap();
            let expected = match operator {
                "*" => 252,
                "/" => 7,
                _ => 0,
            };
            assert_eq!(
                result[0],
                NativeScalarValue::from_bits(NativeScalarKind::U32, expected).unwrap()
            );
        }
    }
}

#[test]
fn checked_u16_native_arithmetic_preserves_edges_raw_admission_and_recovery() {
    use mech_syntax::document::{
        AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
        parse_canonical_document,
    };
    let edges = [
        0,
        1,
        2,
        3,
        255,
        256,
        32767,
        32768,
        u16::MAX / 2,
        u16::MAX / 2 + 1,
        u16::MAX - 1,
        u16::MAX,
    ];
    let mut pairs = edges
        .iter()
        .flat_map(|&a| edges.iter().map(move |&b| (a, b)))
        .collect::<Vec<_>>();
    let mut seed = 0x32a9u16;
    for _ in 0..4096 {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        let left = seed;
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        pairs.push((left, seed));
        // Include many successful products as well as full-width failures.
        pairs.push((left & 0xff, seed & 0xff));
    }
    let catalog = mech_stdlib::source_catalog();
    for operator in ["+", "-", "*", "/", "%"] {
        let text = format!("left<u16> {operator} right<u16>\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x5533324152495448), Revision(0), text.as_str()).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
        let artifact = mech_engine::CanonicalSourceFrontend
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
            let mut outputs = [u64::MAX];
            for &(left, right) in &pairs {
                let expected = match operator {
                    "+" => left.checked_add(right),
                    "-" => left.checked_sub(right),
                    "*" => left.checked_mul(right),
                    "/" => left.checked_div(right),
                    _ => left.checked_rem(right),
                };
                let inputs = [u64::from(left), u64::from(right)];
                outputs[0] = u64::MAX;
                calls.store(0, Ordering::SeqCst);
                let mut frame = NativeEntryFrame {
                    version: ABI_VERSION,
                    input_count: 2,
                    local_count: locals.len() as u64,
                    output_count: 1,
                    inputs: inputs.as_ptr(),
                    locals: locals.as_mut_ptr(),
                    outputs: outputs.as_mut_ptr(),
                    context: (&calls as *const AtomicU64).cast_mut().cast(),
                    admit_next: count,
                    error_node: NO_NODE,
                };
                // SAFETY: real source-derived code, canonical U16 inputs and
                // exact disjoint scratch/output extents for this private ABI.
                let status = unsafe { (jit.entry)(&mut frame) };
                if let Some(value) = expected {
                    assert_eq!(status, STATUS_OK, "{left} {operator} {right}");
                    assert_eq!(outputs[0], u64::from(value));
                } else {
                    assert_eq!(status, STATUS_ARITHMETIC, "{left} {operator} {right}");
                    assert_eq!(outputs[0], u64::MAX);
                    assert_ne!(frame.error_node, NO_NODE);
                }
            }
            for bad in [65536, 65537, u64::MAX] {
                for inputs in [[bad, 3], [9, bad]] {
                    outputs[0] = u64::MAX;
                    calls.store(0, Ordering::SeqCst);
                    let mut frame = NativeEntryFrame {
                        version: ABI_VERSION,
                        input_count: 2,
                        local_count: locals.len() as u64,
                        output_count: 1,
                        inputs: inputs.as_ptr(),
                        locals: locals.as_mut_ptr(),
                        outputs: outputs.as_mut_ptr(),
                        context: (&calls as *const AtomicU64).cast_mut().cast(),
                        admit_next: count,
                        error_node: NO_NODE,
                    };
                    // SAFETY: allocations are exact; only an input payload is
                    // deliberately malformed and must be rejected before admission.
                    assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ABI);
                    assert_eq!(calls.load(Ordering::SeqCst), 0);
                    assert_eq!(frame.error_node, NO_NODE);
                    assert_eq!(outputs[0], u64::MAX);
                }
            }
            let inputs = [u64::from(u16::MAX), 0];
            let mut frame = NativeEntryFrame {
                version: ABI_VERSION,
                input_count: 2,
                local_count: locals.len() as u64,
                output_count: 1,
                inputs: inputs.as_ptr(),
                locals: locals.as_mut_ptr(),
                outputs: outputs.as_mut_ptr(),
                context: (&calls as *const AtomicU64).cast_mut().cast(),
                admit_next: deny,
                error_node: NO_NODE,
            };
            // SAFETY: exact valid buffers; refusal must precede arithmetic and
            // publication, including division/remainder with a zero divisor.
            assert_eq!(unsafe { (jit.entry)(&mut frame) }, STATUS_ADMISSION);
            assert_eq!(outputs[0], u64::MAX);
            let input = [
                NativeScalarValue::from_bits(NativeScalarKind::U16, 42).unwrap(),
                NativeScalarValue::from_bits(NativeScalarKind::U16, 6).unwrap(),
            ];
            let mut result = [NativeScalarValue::U64(99)];
            jit.evaluate_into(&input, &mut result, NativeExecutionLimits::default(), None)
                .unwrap();
            let expected = match operator {
                "+" => 48,
                "-" => 36,
                "*" => 252,
                "/" => 7,
                _ => 0,
            };
            assert_eq!(
                result[0],
                NativeScalarValue::from_bits(NativeScalarKind::U16, expected).unwrap()
            );
        }
    }
}

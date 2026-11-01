//! Private native-word/String reads versus actual ordinary maintained leaves.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ResourceDemand, ValueData, snapshot::ValueDataDraft as Data};
use mech_engine::native_artifact::{
    NativeCollectionPrimitive as Primitive, NativeIterationAction, NativeLibraryWorkBudget,
    NativeTextLineMatrixChoice, lower_native_program_artifact, prepare_native_program_library_call,
};

#[test]
fn line_length_and_scalar_reads_match_ordinary_values_and_exact_bounds_errors() {
    let source = "+> text\n+> sequence\nlines := text/lines(input<string>)\nn := sequence/length(lines)\nlines[index<u64>]\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        // The source exposes its selected String, so the complete-use proof
        // correctly REFUSES the optimization. These are isolated leaf tests,
        // never permission to skip a real Matrix in that source program.
        assert!(plan.text_line_matrix_regions().is_empty());
        let calls = [
            Primitive::TextLines,
            Primitive::SequenceLength,
            Primitive::MatrixScalarAccess,
        ]
        .map(|wanted| {
            let template = plan
                .blocks()
                .iter()
                .find_map(|block| match block.action() {
                    NativeIterationAction::LibraryCall { template, .. }
                        if template.primitive() == wanted =>
                    {
                        Some(template)
                    }
                    _ => None,
                })
                .unwrap();
            prepare_native_program_library_call(&artifact, template, &catalog).unwrap()
        });
        for text in ["", "\n", "\r", "a\r\né\n🧑‍🚀", "\0\n\n"] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            for index in [0, 1, 2, 3, u64::MAX] {
                let args = inputs(
                    &artifact,
                    &[
                        ("input", Data::String(text.into())),
                        ("index", Data::U64(index)),
                    ],
                    &memory,
                );
                let input = artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == "input"
                    })
                    .unwrap();
                let selector = 1 - input;
                let view = calls[0]
                    .text_line_matrix(
                        &args[input],
                        limits().library,
                        &work,
                        ResourceDemand::default(),
                    )
                    .unwrap();
                let mut canonical = None;
                calls[0]
                    .invoke_current(
                        &[Some(args[input].clone())],
                        &mut canonical,
                        limits().library,
                        &work,
                    )
                    .unwrap();
                let mut count = None;
                calls[1]
                    .invoke_current(&[canonical.clone()], &mut count, limits().library, &work)
                    .unwrap();
                let ValueData::U64(expected) = count.as_ref().unwrap().data() else {
                    panic!("canonical U64")
                };
                assert_eq!(
                    calls[1]
                        .text_line_matrix_length(
                            &view,
                            limits().library,
                            &work,
                            ResourceDemand::default()
                        )
                        .unwrap(),
                    *expected
                );
                let mut selected = None;
                let ordinary = calls[2].invoke_current(
                    &[canonical.clone(), Some(args[selector].clone())],
                    &mut selected,
                    limits().library,
                    &work,
                );
                let borrowed = calls[2].text_line_matrix_element(
                    &view,
                    index,
                    limits().library,
                    &work,
                    ResourceDemand::default(),
                );
                match ordinary {
                    Err(error) => assert!(
                        matches!(borrowed, Err(candidate) if candidate == error),
                        "{text:?} {index}: {error:?}"
                    ),
                    Ok(_) => {
                        let ValueData::String(expected) = selected.as_ref().unwrap().data() else {
                            panic!("canonical String")
                        };
                        assert_eq!(borrowed.unwrap(), &**expected);
                    }
                }
                drop(selected);
                drop(count);
                drop(canonical);
                drop(view);
                drop(args);
            }
            drop(work);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn unicode_fallback_keeps_one_original_call_cap_without_refunding_work() {
    let source =
        "+> text\n+> sequence\nlines := text/lines(input<string>)\nsequence/length(lines)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let region = plan.text_line_matrix_regions().pop().unwrap();
        let NativeIterationAction::LibraryCall { template, .. } =
            plan.blocks()[region.producer() as usize].action()
        else {
            unreachable!()
        };
        let call = prepare_native_program_library_call(&artifact, template, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[("input", Data::String("aé\r\n".repeat(1024)))],
            &memory,
        );
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let choice = call
            .text_line_matrix_or_canonical(
                &args[0],
                limits().library,
                &work,
                ResourceDemand::default(),
            )
            .unwrap();
        assert!(matches!(choice, NativeTextLineMatrixChoice::Canonical(_)));
        let total = work.used().0;
        drop(choice);
        drop(work);
        let mut cap = limits().library;
        cap.max_compute_work = Some(total - 1);
        // Ordinary construction fits this cap by itself. It is only the
        // already-spent representation preference that makes ONE call exceed
        // it. Independent inner scopes would incorrectly accept the fallback.
        let plain_work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let mut ordinary = None;
        call.invoke_current(&[Some(args[0].clone())], &mut ordinary, cap, &plain_work)
            .unwrap();
        assert!(plain_work.used().0 < total - 1);
        drop(ordinary);
        drop(plain_work);
        let limited = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let before = memory.used_bytes();
        let denied =
            call.text_line_matrix_or_canonical(&args[0], cap, &limited, ResourceDemand::default());
        assert!(matches!(
            denied,
            Err(
                mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                    mech_core::ResidentKernelError::ResourceLimit {
                        dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                        ..
                    }
                )
            )
        ));
        assert!(limited.used().0 > 0 && limited.used().0 <= total - 1);
        assert_eq!(memory.used_bytes(), before);
        let spent = limited.used().0;
        let recovered = call
            .text_line_matrix_or_canonical(
                &args[0],
                limits().library,
                &limited,
                ResourceDemand::default(),
            )
            .unwrap();
        assert!(limited.used().0 > spent);
        drop(recovered);
        drop(limited);
        drop(args);
        assert_eq!(memory.used_bytes(), 0);
    }
}

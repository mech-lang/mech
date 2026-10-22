//! Private line geometry, not a canonical Matrix or a second text evaluator.
use super::{artifacts, inputs, limits};
use mech_core::{
    ManagedMemoryBudget, MemoryBudgetDimension, ResourceDemand, ValueData,
    snapshot::{SequenceView, StringMatrixSnapshotConstructor, ValueDataDraft as Data},
};
use mech_engine::native_artifact::{
    NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
    NativePreparedLibraryCall, lower_native_program_artifact, prepare_native_program_library_call,
};
use std::{rc::Rc, sync::Arc};

const SOURCE: &str =
    "+> text\n+> sequence\nlines := text/lines(input<string>)\nsequence/length(lines)\n";

fn call(artifact: &mech_engine::ProgramArtifact) -> NativePreparedLibraryCall {
    let catalog = mech_stdlib::source_catalog();
    let plan = lower_native_program_artifact(artifact, &catalog).unwrap();
    let region = plan.text_line_matrix_regions().pop().unwrap();
    let NativeIterationAction::LibraryCall { template, .. } =
        plan.blocks()[region.producer() as usize].action()
    else {
        panic!("actual text/lines producer")
    };
    prepare_native_program_library_call(artifact, template, &catalog).unwrap()
}

#[test]
fn text_line_matrix_view_matches_canonical_utf8_crlf_empty_shape_and_bytes() {
    for artifact in artifacts(SOURCE).0 {
        let call = call(&artifact);
        for text in [
            "",
            "\n",
            "\r",
            "\r\n",
            "a\nb\n",
            "a\r\n\r\nb\r",
            "é\n🧑‍🚀\r\n終",
            "\0\n\0",
            "a\u{0085}\u{2028}\u{2029}b",
        ] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut args = inputs(&artifact, &[("input", Data::String(text.into()))], &memory);
            let source = args.pop().unwrap();
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let view = call
                .text_line_matrix(&source, limits().library, &work, ResourceDemand::default())
                .unwrap();
            let snapshot_source = view
                .source()
                .expect("snapshot view retains its String source");
            let rows = view.rows().expect("snapshot view retains its row offsets");
            assert!(snapshot_source.shares_frozen_storage(&source));
            assert_eq!(view.is_ascii(), text.is_ascii());
            assert_eq!(view.count(), text.lines().count());
            assert_eq!(rows.len(), view.count());
            assert_eq!(view.program_revision(), artifact.revision());
            assert_eq!(
                view.retained_bytes(&memory).unwrap(),
                source.memory_budget_retained_bytes(&memory).unwrap() + view.metadata_bytes()
            );
            let ValueData::String(owner) = source.data() else {
                panic!("real String owner")
            };
            for (i, expected) in owner.lines().enumerate() {
                assert_eq!(view.line(i), Some(expected));
                assert_eq!(
                    rows[i],
                    [expected.as_ptr() as u64, expected.len() as u64]
                );
                assert_eq!(view.line(i).unwrap().as_ptr(), expected.as_ptr());
            }
            assert_eq!(view.line(view.count()), None);
            let mut ordinary = None;
            call.invoke_current(
                &[Some(source.clone())],
                &mut ordinary,
                limits().library,
                &work,
            )
            .unwrap();
            let ordinary = ordinary.unwrap();
            assert_eq!(view.schema(), ordinary.schema());
            assert_eq!(view.schema_key(), ordinary.schema_key());
            assert_eq!(view.shape(), ordinary.shape());
            let ValueData::Matrix(matrix) = ordinary.data() else {
                panic!("canonical Matrix")
            };
            let SequenceView::String(cells) = matrix.elements() else {
                panic!("canonical String cells")
            };
            assert_eq!(cells.len(), view.count());
            let cells = (0..view.count())
                .map(|i| Box::<str>::from(view.line(i).unwrap()))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            let rebuilt = StringMatrixSnapshotConstructor::bind(
                view.schema(),
                view.shape().clone(),
                Arc::new(artifact.schemas().clone()),
            )
            .unwrap()
            .construct(cells)
            .unwrap();
            assert_eq!(
                rebuilt
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap(),
                ordinary
                    .canonical_snapshot_bytes(artifact.schemas())
                    .unwrap()
            );
            drop(source); // Only the view and ordinary result still own inputs.
            assert_eq!(
                (0..view.count())
                    .map(|i| view.line(i).unwrap())
                    .collect::<Vec<_>>(),
                text.lines().collect::<Vec<_>>()
            );
            drop(view);
            drop(ordinary);
            drop(work);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn text_line_matrix_denials_keep_the_same_account_and_release_all_metadata() {
    for artifact in artifacts(SOURCE).0 {
        let call = call(&artifact);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut args = inputs(
            &artifact,
            &[("input", Data::String("one\ntwo\n".into()))],
            &memory,
        );
        let source = args.pop().unwrap();
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let baseline = memory.used_bytes();
        let mut denied = limits().library;
        denied.max_output_elements = Some(0);
        assert!(matches!(
            call.text_line_matrix(&source, denied, &work, ResourceDemand::default()),
            Err(Error::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::OutputElements,
                    ..
                }
            ))
        ));
        assert_eq!(memory.used_bytes(), baseline);
        let after_denial = work.used();
        assert!(after_denial.0 > 0);
        let hold = memory
            .reserve_capacity(memory.limit_bytes() - memory.used_bytes())
            .unwrap();
        let held = memory.used_bytes();
        assert!(matches!(
            call.text_line_matrix(&source, limits().library, &work, ResourceDemand::default()),
            Err(Error::Memory(_))
        ));
        assert_eq!(memory.used_bytes(), held);
        assert!(work.used().0 > after_denial.0);
        drop(hold);
        let used = work.used();
        let view = call
            .text_line_matrix(&source, limits().library, &work, ResourceDemand::default())
            .unwrap();
        assert_eq!(view.line(0), Some("one"));
        assert!(work.used().0 > used.0);
        let other = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let foreign = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &other).unwrap();
        assert!(matches!(
            call.text_line_matrix(
                &source,
                limits().library,
                &foreign,
                ResourceDemand::default()
            ),
            Err(Error::UnadmittedValue)
        ));
        assert!(matches!(
            view.retained_bytes(&other),
            Err(Error::UnadmittedValue)
        ));
        let starved = NativeLibraryWorkBudget::new(1, 1, &memory).unwrap();
        let before = memory.used_bytes();
        assert!(matches!(
            call.text_line_matrix(
                &source,
                limits().library,
                &starved,
                ResourceDemand::default()
            ),
            Err(Error::Kernel(
                mech_core::ResidentKernelError::ResourceLimit {
                    dimension: MemoryBudgetDimension::ComputeWork,
                    ..
                }
            ))
        ));
        assert_eq!(memory.used_bytes(), before);
        assert_eq!(starved.used(), (0, 0));
        drop(view);
        drop(starved);
        drop(foreign);
        drop(work);
        drop(source);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(other.used_bytes(), 0);
    }
}

#[test]
fn selected_line_matrix_reuses_ordinary_positions_errors_shape_and_join() {
    let source = "+> text\nlines := text/lines(input<string>)\ntext/join(lines[indices<[u64]>], separator<string>)\n";
    for artifact in artifacts(source).0 {
        let catalog = mech_stdlib::source_catalog();
        let plan = lower_native_program_artifact(&artifact, &catalog).unwrap();
        let producer = plan.text_line_matrix_regions().pop().unwrap();
        let join = plan.selected_text_joins().pop().unwrap();
        let calls = [producer.producer(), join.selection(), join.consumer()].map(|site| {
            let NativeIterationAction::LibraryCall { template, .. } =
                plan.blocks()[site as usize].action()
            else {
                unreachable!()
            };
            prepare_native_program_library_call(&artifact, template, &catalog).unwrap()
        });
        for (text, indices, separator) in [
            ("one\ntwo\nthree\n", vec![3, 1, 3], "|"),
            ("é\r\n🧑‍🚀\r\n終", vec![2, 3, 1], "→"),
            ("\n\0\n", vec![2, 1, 2], "\0"),
            ("", vec![], "!"),
            ("a\nb\n", vec![], "!"),
            ("a\nb\n", vec![0], ""),
            ("a\nb\n", vec![3], ""),
        ] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("input", Data::String(text.into())),
                    (
                        "indices",
                        Data::Matrix(indices.iter().copied().map(Data::U64).collect()),
                    ),
                    ("separator", Data::String(separator.into())),
                ],
                &memory,
            );
            let owner = |name: &str| {
                args[artifact
                    .inputs()
                    .iter()
                    .position(|input| {
                        mech_engine::decode_source_input_name(&input.name).unwrap() == name
                    })
                    .unwrap()]
                .clone()
            };
            let input = owner("input");
            let selector = owner("indices");
            let separator = owner("separator");
            let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
            let lines = Rc::new(
                calls[0]
                    .text_line_matrix(&input, limits().library, &work, ResourceDemand::default())
                    .unwrap(),
            );
            let mut canonical = None;
            calls[0]
                .invoke_current(
                    &[Some(input.clone())],
                    &mut canonical,
                    limits().library,
                    &work,
                )
                .unwrap();
            let mut selected = None;
            let ordinary = calls[1].invoke_current(
                &[canonical.clone(), Some(selector.clone())],
                &mut selected,
                limits().library,
                &work,
            );
            let projected = calls[1].selected_text_line_matrix(
                &lines,
                &selector,
                limits().library,
                &work,
                ResourceDemand::default(),
            );
            if let Err(error) = ordinary {
                assert!(
                    matches!(&projected, Err(candidate) if candidate == &error),
                    "{text:?} {indices:?}: {error:?}"
                );
                drop(projected);
                drop(selected);
                drop(canonical);
                drop(lines);
                drop(input);
                drop(selector);
                drop(separator);
                drop(args);
                drop(work);
                assert_eq!(memory.used_bytes(), 0);
                continue;
            }
            let view = projected.unwrap();
            let mut output = None;
            calls[2]
                .invoke_current(
                    &[selected.clone(), Some(separator.clone())],
                    &mut output,
                    limits().library,
                    &work,
                )
                .unwrap();
            let ValueData::String(expected) = output.as_ref().unwrap().data() else {
                panic!("canonical join")
            };
            let ValueData::String(sep) = separator.data() else {
                unreachable!()
            };
            let bytes =
                view.payload_bytes() + sep.len() as u64 * view.count().saturating_sub(1) as u64;
            work.charge(bytes * 16 + view.count() as u64 * 128 + 1024, 0)
                .unwrap();
            let claim = memory.reserve_capacity(bytes).unwrap();
            let mut actual = String::new();
            actual.try_reserve_exact(bytes as usize).unwrap();
            assert_eq!(actual.capacity() as u64, bytes);
            drop(lines);
            drop(input);
            drop(selector);
            drop(args);
            calls[2]
                .selected_matrix_join_append(&view, &separator, &mut actual, &memory)
                .unwrap();
            assert_eq!(actual.as_str(), &**expected);
            drop(actual);
            drop(claim);
            drop(view);
            drop(separator);
            drop(output);
            drop(selected);
            drop(canonical);
            drop(work);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

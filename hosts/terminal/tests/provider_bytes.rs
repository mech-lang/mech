#![cfg(feature = "provider")]

use mech_core::{
    DimensionExpr, IntegerWidth, MResult, MechError, Schema, SchemaBody, SchemaDraft, Value,
    ValueCell, ValueData,
    snapshot::{SequenceView, ValueDataDraft},
};
use mech_runtime::{
    PreparedRuntimeEffect, ResourceSchemaPreflight, RuntimeCapabilityOperation, RuntimeEffectId,
    RuntimeHostFactory, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteCommand, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    TransactionId, value_string,
};
use mech_terminal::{ArgumentSnapshotCliBackend, CliBackend, CliHostFactory, CliResourceProvider};
use std::{cell::Cell, ffi::OsString};

#[derive(Clone, Debug, Default)]
struct Backend {
    input: Vec<u8>,
    reads: Cell<usize>,
    read_error: bool,
    output: Vec<(String, Vec<u8>)>,
    fail_write: bool,
    fail_flush: bool,
}
fn failure(reason: &str) -> MechError {
    MechError::new(
        mech_terminal::CliResourceProviderError {
            resource: "test".into(),
            reason: reason.into(),
        },
        None,
    )
}
impl Backend {
    fn write(&mut self, stream: &str, bytes: &[u8]) -> MResult<()> {
        if self.fail_write {
            return Err(failure("injected byte write failure"));
        }
        self.output.push((stream.into(), bytes.to_vec()));
        if self.fail_flush {
            return Err(failure("injected byte flush failure"));
        }
        Ok(())
    }
}
impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        self.reads.set(self.reads.get() + 1);
        Some(if self.read_error {
            Err(failure("injected raw input failure"))
        } else {
            Ok(self.input.clone())
        })
    }
    fn stdin_text(&self) -> MResult<String> {
        panic!("raw backend must not take a second text snapshot")
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        self.write("stdout", text.as_bytes())
    }
    fn write_stderr(&mut self, text: &str) -> MResult<()> {
        self.write("stderr", text.as_bytes())
    }
    fn write_stdout_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.write("stdout", bytes)
    }
    fn write_stderr_bytes(&mut self, bytes: &[u8]) -> MResult<()> {
        self.write("stderr", bytes)
    }
}
fn read(path: &str) -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: "cli://cli/stdin".into(),
        path: path.into(),
        context_name: "stdin".into(),
    }
}
fn row(rows: u64, columns: u64, bytes: &[u8]) -> Value {
    ValueCell::dynamic_matrix(
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        Box::new([rows, columns]),
        bytes.iter().copied().map(ValueDataDraft::U8).collect(),
    )
    .unwrap()
    .snapshot()
    .unwrap()
}
fn assert_row(value: &Value, expected: &[u8]) {
    let schemas = value.schemas().unwrap();
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schemas.get(value.schema()).unwrap().body()
    else {
        panic!("matrix schema")
    };
    assert_eq!(
        element.as_ref(),
        &SchemaBody::UnsignedInteger(IntegerWidth::W8)
    );
    assert_eq!(dimensions.len(), 2);
    assert_eq!(value.shape().resolve_dimension(&dimensions[0]).unwrap(), 1);
    assert_eq!(
        value.shape().resolve_dimension(&dimensions[1]).unwrap(),
        expected.len() as u64
    );
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("matrix payload")
    };
    let SequenceView::U8(actual) = matrix.elements() else {
        panic!("U8 payload")
    };
    assert_eq!(actual, expected);
}
fn command(stream: &str, path: &str, value: Value) -> RuntimeResourceWriteCommand {
    RuntimeResourceWriteCommand {
        base_uri: format!("cli://cli/{stream}"),
        path: path.into(),
        context_name: stream.into(),
        operation: RuntimeCapabilityOperation::Write,
        value,
        intent: RuntimeResourceWriteIntent::Send,
    }
}
fn effect(
    provider: &CliResourceProvider<impl CliBackend + 'static>,
    stream: &str,
    path: &str,
    value: Value,
) -> Box<dyn mech_runtime::RuntimeAfterCommitEffect> {
    match provider
        .prepare_write(command(stream, path, value).bind_effect(
            RuntimeEffectId {
                transaction: TransactionId::new(1),
                sequence: 0,
            },
            "test:bytes".into(),
        ))
        .unwrap()
    {
        PreparedRuntimeEffect::AfterCommit(effect) => effect,
        other => panic!("expected after-commit effect: {other:?}"),
    }
}
fn schema(body: SchemaBody) -> Schema {
    SchemaDraft {
        body,
        dimension_parameters: Box::new([]),
    }
    .finalize()
    .unwrap()
}

#[test]
fn raw_snapshot_retains_all_bytes_and_text_decode_failure_does_not_poison_bytes() {
    for text_first in [false, true] {
        let input = vec![0, 0x80, 0xff, b'\r', b'\n'];
        let mut provider = CliResourceProvider::new(Backend {
            input: input.clone(),
            ..Default::default()
        });
        if text_first {
            assert!(provider.read(read("text")).is_err());
        }
        assert_row(&provider.read(read("bytes")).unwrap(), &input);
        provider.backend_mut().input = vec![b'x'];
        let first = provider.read(read("text")).unwrap_err().display_message();
        let second = provider.read(read("text")).unwrap_err().display_message();
        assert_eq!(first, second);
        assert!(first.contains("UTF-8"));
        assert_row(&provider.read(read("bytes")).unwrap(), &input);
        assert_eq!(provider.backend().reads.get(), 1);
    }
}
#[test]
fn valid_mixed_views_and_empty_input_share_one_acquisition() {
    for input in [b"".as_slice(), "héllo\r\n".as_bytes()] {
        for text_first in [false, true] {
            let provider = CliResourceProvider::new(Backend {
                input: input.to_vec(),
                ..Default::default()
            });
            for path in if text_first {
                ["text", "bytes", "text", "bytes"]
            } else {
                ["bytes", "text", "bytes", "text"]
            } {
                let value = provider.read(read(path)).unwrap();
                if path == "bytes" {
                    assert_row(&value, input);
                } else {
                    let ValueData::String(text) = value.data() else {
                        panic!("text")
                    };
                    assert_eq!(text.as_bytes(), input);
                }
            }
            assert_eq!(provider.backend().reads.get(), 1);
        }
    }
}
#[test]
fn acquisition_errors_are_cached_across_both_views() {
    let mut provider = CliResourceProvider::new(Backend {
        read_error: true,
        ..Default::default()
    });
    let first = provider.read(read("bytes")).unwrap_err().display_message();
    provider.backend_mut().read_error = false;
    for path in ["text", "bytes", "text"] {
        assert_eq!(
            provider.read(read(path)).unwrap_err().display_message(),
            first
        );
    }
    assert_eq!(provider.backend().reads.get(), 1);
}
#[test]
fn planning_and_rejected_paths_do_not_observe_the_stream() {
    let provider = CliResourceProvider::new(Backend {
        read_error: true,
        ..Default::default()
    });
    let planned = provider.plan_read(read("bytes")).unwrap();
    assert_row(&planned, &[]);
    let schemas = planned.schemas().unwrap();
    let SchemaBody::Matrix { dimensions, .. } = schemas.get(planned.schema()).unwrap().body()
    else {
        panic!()
    };
    assert!(
        matches!(&dimensions[1], DimensionExpr::Parameter(_)),
        "width must remain dynamic after empty planning"
    );
    provider.plan_read(read("text")).unwrap();
    for path in ["Bytes", "bytes/1", "/bytes", " bytes", "bytes ", ""] {
        assert!(provider.plan_read(read(path)).is_err());
        assert!(provider.read(read(path)).is_err());
    }
    let mut wrong_context = read("bytes");
    wrong_context.context_name = "args".into();
    assert!(provider.read(wrong_context).is_err());
    assert!(!provider.observation_requires_input_driver(&read("bytes")));
    assert_eq!(provider.backend().reads.get(), 0);
}
#[test]
fn byte_sends_are_staged_costed_and_delivered_without_suffix_or_transcoding() {
    let provider = CliResourceProvider::new(Backend::default());
    for stream in ["stdout", "stderr"] {
        let pending = effect(&provider, stream, "bytes", row(1, 3, &[0, 0x80, 0xff]));
        assert_eq!(pending.metadata().cost.bytes, 3);
        assert_eq!(pending.metadata().cost.items, 1);
        drop(pending);
        assert!(provider.backend().output.is_empty());
    }
    for stream in ["stdout", "stderr"] {
        effect(&provider, stream, "bytes", row(1, 3, &[0, 0x80, 0xff]))
            .deliver()
            .unwrap();
        effect(&provider, stream, "bytes", row(1, 0, &[]))
            .deliver()
            .unwrap();
        effect(&provider, stream, "line", value_string("é"))
            .deliver()
            .unwrap();
    }
    assert_eq!(
        provider.backend().output,
        vec![
            ("stdout".into(), vec![0, 0x80, 0xff]),
            ("stdout".into(), vec![]),
            ("stdout".into(), "é\n".as_bytes().to_vec()),
            ("stderr".into(), vec![0, 0x80, 0xff]),
            ("stderr".into(), vec![]),
            ("stderr".into(), "é\n".as_bytes().to_vec())
        ]
    );
}
#[test]
fn byte_values_reject_wrong_kind_rank_and_row_shape_before_delivery() {
    let provider = CliResourceProvider::new(Backend::default());
    let invalid = [value_string("\0"), row(2, 1, &[1, 2]), row(0, 2, &[])];
    for value in invalid {
        assert!(
            provider
                .plan_write(command("stdout", "bytes", value.clone()))
                .is_err()
        );
        assert!(
            provider
                .prepare_output(command("stdout", "bytes", value))
                .is_err()
        );
    }
    let invalid_schemas = [
        schema(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W16)),
            dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(2)]),
        }),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: Box::new([DimensionExpr::Constant(2), DimensionExpr::Constant(1)]),
        }),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: Box::new([DimensionExpr::Constant(2)]),
        }),
    ];
    let preflight = || RuntimeResourceWritePreflightRequest {
        base_uri: "cli://stdout".into(),
        path: "bytes".into(),
        context_name: "stdout".into(),
        operation: RuntimeCapabilityOperation::Write,
        intent: RuntimeResourceWriteIntent::Send,
    };
    for invalid in invalid_schemas {
        assert!(
            provider
                .preflight_write_schema(preflight(), &invalid)
                .is_err()
        );
    }
    let valid = schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
        dimensions: Box::new([DimensionExpr::Constant(1), DimensionExpr::Constant(0)]),
    });
    assert_eq!(
        provider
            .preflight_write_schema(preflight(), &valid)
            .unwrap(),
        ResourceSchemaPreflight::Validated
    );
    let dynamic = row(1, 2, &[1, 2]);
    let schemas = dynamic.schemas().unwrap();
    assert_eq!(
        provider
            .preflight_write_schema(preflight(), schemas.get(dynamic.schema()).unwrap())
            .unwrap(),
        ResourceSchemaPreflight::DeferredToRuntime
    );
    assert!(provider.backend().output.is_empty());
    assert_eq!(provider.backend().reads.get(), 0);
}
#[test]
fn raw_backend_write_and_flush_errors_propagate_without_lossy_fallback() {
    for flush in [false, true] {
        let provider = CliResourceProvider::new(Backend {
            fail_write: !flush,
            fail_flush: flush,
            ..Default::default()
        });
        let error = effect(&provider, "stdout", "bytes", row(1, 2, &[0x80, 0xff]))
            .deliver()
            .unwrap_err();
        assert!(error.display_message().contains(if flush {
            "flush failure"
        } else {
            "write failure"
        }));
        // A flush failure can follow a physical write. After-commit effects do
        // not promise to undo bytes already delivered to an external stream.
        assert_eq!(provider.backend().output.len(), usize::from(flush));
    }
}
#[test]
fn argument_snapshot_and_host_factory_delegate_raw_io_and_preserve_instance_scope() {
    let snapshot = ArgumentSnapshotCliBackend::new(
        Backend {
            input: vec![0xff, 0],
            ..Default::default()
        },
        vec![OsString::from("1")],
    );
    assert_eq!(snapshot.stdin_bytes().unwrap().unwrap(), vec![0xff, 0]);
    let factory = CliHostFactory::with_backend(snapshot).unwrap();
    let installation = factory
        .instantiate("term", &mech_runtime::ConfigValue::Map(Default::default()))
        .unwrap();
    let provider = &installation.resource_providers[0];
    let mut req = read("bytes");
    req.base_uri = "cli://term/stdin".into();
    assert_row(&provider.read(req).unwrap(), &[0xff, 0]);
    assert!(provider.read(read("bytes")).is_err());
    assert!(provider.equivalent_base_uri_groups().is_empty());
}
#[test]
fn legacy_text_backend_remains_compatible_and_never_synthesizes_raw_input() {
    #[derive(Debug, Default)]
    struct Legacy {
        reads: Cell<usize>,
    }
    impl CliBackend for Legacy {
        fn env_var(&self, _: &str) -> MResult<Option<String>> {
            Ok(None)
        }
        fn stdin_text(&self) -> MResult<String> {
            self.reads.set(self.reads.get() + 1);
            Ok("legacy é".into())
        }
        fn write_stdout(&mut self, _: &str) -> MResult<()> {
            Ok(())
        }
        fn write_stderr(&mut self, _: &str) -> MResult<()> {
            Ok(())
        }
    }
    let provider = CliResourceProvider::new(Legacy::default());
    assert!(
        provider
            .read(read("bytes"))
            .unwrap_err()
            .display_message()
            .contains("unavailable")
    );
    assert_eq!(provider.backend().reads.get(), 0);
    provider.read(read("text")).unwrap();
    provider.read(read("text")).unwrap();
    assert_eq!(provider.backend().reads.get(), 1);
    assert!(
        effect(&provider, "stdout", "bytes", row(1, 1, &[0xff]))
            .deliver()
            .unwrap_err()
            .display_message()
            .contains("unavailable")
    );
}

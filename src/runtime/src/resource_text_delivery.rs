//! Scoped delivery from the SAME registered provider, after the ordinary
//! resource transaction/capability gate. No resource callback reaches native.
use super::*;
use crate::resource_text_capture::{ConsumerFailure, DeferredCaptureFailure};
use crate::{RuntimeTextCaptureBuffer, RuntimeTextCaptureError, RuntimeTextCaptureSummary};
use mech_core::{SchemaKey, ValueData, Utf8Fragment};
use std::{convert::Infallible, io::Cursor};

fn invalid(reason: &str) -> MechError {
    MechError::new(
        crate::NativeResourceBindingError {
            reason: reason.into(),
        },
        None,
    )
}

impl RuntimeResourceRegistry {
    fn text_capture_provider(
        &self,
        request: &RuntimeResourceReadRequest,
    ) -> MResult<(&RuntimeResourceProviderEntry, String)> {
        let scheme = resource_uri_scheme(&request.base_uri)?.to_string();
        let entry = self
            .provider_entry_for(&scheme, &request.base_uri)
            .ok_or_else(|| {
                MechError::new(
                    RuntimeResourceProviderNotFound {
                        scheme: scheme.clone(),
                        uri: request.base_uri.clone(),
                    },
                    None,
                )
            })?;
        Ok((entry, format!("resource provider `{scheme}`")))
    }

    pub(crate) fn supports_single_use_text_capture(
        &self,
        request: &RuntimeResourceReadRequest,
    ) -> MResult<bool> {
        let (entry, label) = self.text_capture_provider(request)?;
        invoke_extension_value(label, "supports_single_use_text_capture", || {
            entry.provider.supports_single_use_text_capture(request)
        })
    }

    /// The outer error is capture/contract failure; the inner error is the
    /// original native consumer failure, deferred until complete valid capture.
    pub(crate) fn capture_text(
        &self,
        request: RuntimeResourceReadRequest,
        staged: Option<Value>,
        expected: SchemaKey,
        buffer: RuntimeTextCaptureBuffer,
        consumer: &mut dyn FnMut(Utf8Fragment<'_>) -> MResult<()>,
    ) -> MResult<Result<RuntimeTextCaptureSummary, ConsumerFailure<MechError>>> {
        let (entry, label) = self.text_capture_provider(&request)?;
        let finish = |result| match result {
            Ok(summary) => Ok(Ok(summary)),
            Err(DeferredCaptureFailure::Panic(payload)) => Ok(Err(ConsumerFailure::Panic(payload))),
            Err(DeferredCaptureFailure::Error(RuntimeTextCaptureError::Consumer(error))) => {
                Ok(Err(ConsumerFailure::Error(error)))
            }
            Err(DeferredCaptureFailure::Error(error)) => {
                let error: RuntimeTextCaptureError<Infallible> = match error {
                    RuntimeTextCaptureError::Acquisition(error) => {
                        RuntimeTextCaptureError::Acquisition(error)
                    }
                    RuntimeTextCaptureError::InputLimit { limit } => {
                        RuntimeTextCaptureError::InputLimit { limit }
                    }
                    RuntimeTextCaptureError::InvalidUtf8 {
                        valid_up_to,
                        error_len,
                    } => RuntimeTextCaptureError::InvalidUtf8 {
                        valid_up_to,
                        error_len,
                    },
                    RuntimeTextCaptureError::Consumer(_) => unreachable!("consumer handled above"),
                };
                Err(invoke_extension_value(
                    label.clone(),
                    "text_capture_error",
                    || entry.provider.text_capture_error(&request, error),
                )?)
            }
        };
        if let Some(value) = staged {
            let ValueData::String(text) = value.data() else {
                return Err(invalid(
                    "native text observation returned a non-string value",
                ));
            };
            if value.schema_key() != expected {
                return Err(invalid(
                    "staged String schema differs from canonical observation",
                ));
            }
            return finish(buffer.capture_fragments_deferred(Cursor::new(text.as_bytes()), consumer));
        }
        if !invoke_extension_value(label.clone(), "supports_single_use_text_capture", || {
            entry.provider.supports_single_use_text_capture(&request)
        })? {
            return Err(invalid("provider no longer offers dedicated text capture"));
        }
        let mut buffer = Some(buffer);
        let mut result = None;
        let mut calls = 0usize;
        let provider = invoke_extension(label.clone(), "with_single_use_text_reader", || {
            entry
                .provider
                .with_single_use_text_reader(request.clone(), &mut |reader| {
                    calls += 1;
                    let Some(buffer) = buffer.take() else {
                        return Err(invalid("provider replayed a one-shot text reader"));
                    };
                    result = Some(buffer.capture_fragments_deferred(reader, &mut *consumer));
                    // Do not unwind a native consumer panic through provider locks.
                    // Finish the reader loan first; propagate it below only if the
                    // complete capture succeeded. No provider retry is permitted.
                    Ok(())
                })
        });
        let source_failed = matches!(
            result,
            Some(Err(DeferredCaptureFailure::Error(
                RuntimeTextCaptureError::Acquisition(_)
                    | RuntimeTextCaptureError::InputLimit { .. }
            )))
        );
        if source_failed {
            return finish(result.take().unwrap());
        }
        provider?;
        if calls != 1 {
            return Err(invalid("provider must lend exactly one text reader"));
        }
        finish(result.ok_or_else(|| invalid("provider omitted its text capture"))?)
    }
}

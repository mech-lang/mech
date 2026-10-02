//! Versioned retained-source transport used by browser document controllers.

use crate::{GenericError, MResult, MechError};
#[cfg(feature = "no_std")]
use alloc::{string::String, vec::Vec};

pub const BROWSER_DOCUMENT_PAYLOAD_VERSION: u16 = 1;
#[cfg(feature = "serde")]
const BROWSER_DOCUMENT_PAYLOAD_PREFIX: &str = "mech-source-document-v1:";

#[cfg_attr(
    feature = "serde",
    derive(serde_derive::Serialize, serde_derive::Deserialize)
)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserDocumentPayload {
    version: u16,
    root_specifier: String,
    source: String,
    presentation_output_ids: Vec<u64>,
}

impl BrowserDocumentPayload {
    pub fn new(root_specifier: impl Into<String>, source: impl Into<String>) -> MResult<Self> {
        let root_specifier = root_specifier.into();
        if root_specifier.trim().is_empty() {
            return Err(payload_error(
                "browser document root specifier must not be empty",
            ));
        }
        Ok(Self {
            version: BROWSER_DOCUMENT_PAYLOAD_VERSION,
            root_specifier,
            source: source.into(),
            presentation_output_ids: Vec::new(),
        })
    }

    pub fn with_presentation_output_ids(
        mut self,
        presentation_output_ids: impl IntoIterator<Item = u64>,
    ) -> Self {
        self.presentation_output_ids = presentation_output_ids.into_iter().collect();
        self
    }

    pub const fn version(&self) -> u16 {
        self.version
    }

    pub fn root_specifier(&self) -> &str {
        &self.root_specifier
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn presentation_output_ids(&self) -> &[u64] {
        &self.presentation_output_ids
    }

    #[cfg(feature = "serde")]
    pub fn encode(&self) -> MResult<String> {
        crate::nodes::compress_and_encode(self)
            .map(|encoded| format!("{BROWSER_DOCUMENT_PAYLOAD_PREFIX}{encoded}"))
            .map_err(|error| payload_error(format!("failed to encode browser document: {error}")))
    }

    #[cfg(feature = "serde")]
    pub fn decode(encoded: &str) -> MResult<Self> {
        let encoded = encoded
            .strip_prefix(BROWSER_DOCUMENT_PAYLOAD_PREFIX)
            .ok_or_else(|| {
                payload_error("browser document payload is missing its retained-source prefix")
            })?;
        let payload: Self = crate::nodes::decode_and_decompress(encoded).map_err(|error| {
            payload_error(format!("failed to decode browser document: {error}"))
        })?;
        if payload.version != BROWSER_DOCUMENT_PAYLOAD_VERSION {
            return Err(payload_error(format!(
                "unsupported browser document payload version {}; expected {}",
                payload.version, BROWSER_DOCUMENT_PAYLOAD_VERSION,
            )));
        }
        if payload.root_specifier.trim().is_empty() {
            return Err(payload_error(
                "browser document root specifier must not be empty",
            ));
        }
        Ok(payload)
    }
}

fn payload_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

/// Stable semantic address for the canonical document's implicit program result.
/// Console overlays do not replace or renumber this document boundary.
pub fn root_document_program_output_id() -> u64 {
    crate::hash_str("mech/document-program-output/v1")
}

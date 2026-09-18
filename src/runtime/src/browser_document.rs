//! Versioned retained-source transport used by browser document controllers.

use mech_core::{GenericError, MResult, MechError};

pub const BROWSER_DOCUMENT_PAYLOAD_VERSION: u16 = 1;

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
        mech_core::nodes::compress_and_encode(self)
            .map_err(|error| payload_error(format!("failed to encode browser document: {error}")))
    }

    #[cfg(feature = "serde")]
    pub fn decode(encoded: &str) -> MResult<Self> {
        let payload: Self = mech_core::nodes::decode_and_decompress(encoded).map_err(|error| {
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

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;

    #[test]
    fn retained_source_payload_round_trips_exact_bytes() {
        let payload =
            BrowserDocumentPayload::new("docs/main.mec", "  value := 1\r\n-- e\u{301}\r\n")
                .unwrap()
                .with_presentation_output_ids([7, 9]);
        let decoded = BrowserDocumentPayload::decode(&payload.encode().unwrap()).unwrap();
        assert_eq!(decoded, payload);
        assert_eq!(decoded.source(), "  value := 1\r\n-- e\u{301}\r\n");
    }

    #[test]
    fn legacy_tree_payload_is_not_accepted_as_retained_source() {
        let tree = mech_core::nodes::Program {
            title: None,
            body: mech_core::nodes::Body {
                sections: Vec::new(),
            },
        };
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
        assert!(BrowserDocumentPayload::decode(&encoded).is_err());
    }
}

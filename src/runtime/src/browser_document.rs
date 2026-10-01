//! Runtime re-export of the shared compiler-free browser transport.

pub use mech_core::browser_document::*;

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

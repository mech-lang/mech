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
        // Frozen compatibility payload: Brotli/base64 of the retired bincode
        // empty Program (title=None, sections=[]), whose raw bytes are [0, 0].
        // Keep rejection coverage without resurrecting its source AST types.
        const LEGACY_EMPTY_PROGRAM: &str = "iwCAAAAD";
        assert!(BrowserDocumentPayload::decode(LEGACY_EMPTY_PROGRAM).is_err());
    }
}

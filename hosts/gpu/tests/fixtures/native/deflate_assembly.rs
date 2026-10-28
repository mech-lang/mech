//! Maintained compressor assembly shared by qualification and public examples.
pub const COMPONENTS: &str = concat!(
    include_str!("deflate-matchfinder.mec"),
    "\n",
    include_str!("deflate-binary-tree.mec"),
    "\n",
    include_str!("deflate-collector.mec"),
    "\n",
    include_str!("deflate-block-collector.mec"),
    "\n",
    include_str!("deflate-block-heuristics.mec"),
    "\n",
    include_str!("deflate-block-loop.mec"),
    "\n",
    include_str!("deflate-optimal-path.mec"),
    "\n",
    include_str!("deflate-huffman-level12.mec"),
    "\n",
    include_str!("deflate-tally.mec"),
    "\n",
    include_str!("deflate-optimal-costs.mec"),
    "\n",
    include_str!("deflate-header.mec"),
    "\n",
    include_str!("deflate-optimize.mec"),
    "\n",
    include_str!("deflate-literal-choice.mec"),
    "\n",
    include_str!("deflate-static-codes.mec"),
    "\n",
    include_str!("deflate-writer-tables.mec"),
    "\n",
    include_str!("deflate-path-writer.mec"),
    "\n",
    include_str!("deflate-sequence-writer.mec"),
    "\n",
    include_str!("deflate-block-choice.mec"),
    "\n",
    include_str!("deflate-block-header-writer.mec"),
    "\n",
    include_str!("deflate-stored-writer.mec"),
    "\n",
    include_str!("deflate-flush-block.mec"),
    "\n",
    include_str!("deflate-select-path.mec"),
    "\n",
    include_str!("deflate-plan-block.mec"),
    "\n",
    include_str!("deflate-level12-tables.mec"),
    "\n",
    include_str!("deflate-compress-controller.mec"),
    "\n",
);

pub const COMPLETE_ENTRY: &str = "buffer := sequence/with-capacity(0u8, 6633832u64)\nprefixed := sequence/append-sublist(buffer, prefix<[u8]>, 0u64, sequence/length(prefix))\ndeflate-compress-into(input<[u8]>, prefixed)\n";

pub fn complete_source() -> String {
    format!("{COMPONENTS}{COMPLETE_ENTRY}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn assembled_compressor_source_preserves_its_pinned_bytes() {
        use sha2::{Digest, Sha256};
        let source = super::complete_source();
        assert_eq!(source.len(), 160992);
        assert_eq!(
            Sha256::digest(source.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "345324ce551aaa2485dcd0fe4262da96e660d6b31f5f772bff89b034e9b1997f"
        );
    }
}

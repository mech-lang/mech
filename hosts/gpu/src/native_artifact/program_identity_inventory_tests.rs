//! Hash the complete same-plan inventories, independently of capture eligibility.
use super::super::super::{JitProgramArtifact, program_identity};
use super::call_cache_tests::artifacts;

#[test]
fn shared_descriptor_proofs_preserve_fresh_identity_and_complete_inventory() {
    for (name, source, expected_joins, expected_lines, expected_ranges) in [
        ("scalar", "input<u64> + 1u64\n", 0, 0, 0),
        (
            "indices",
            "+> text\nwords := input<[string]>\nindices := positions<[u64]>\ntext/join(words[indices], separator<string>)\n",
            1,
            0,
            0,
        ),
        (
            "line-length",
            "+> text\n+> sequence\nsequence/length(text/lines(input<string>))\n",
            0,
            1,
            0,
        ),
        (
            "line-range",
            "+> text\nrows := text/lines(input<string>)\ntext/join(rows[first<u64>..last<u64>], \"\")\n",
            1,
            1,
            1,
        ),
        (
            "multiple-lines-and-joins",
            "+> text\nleft := text/lines(a<string>)\nright := text/lines(b<string>)\nx := text/join(left[first<u64>..last<u64>], \"\")\ny := text/join(right[first..last], \"|\")\n(x,y)\n",
            2,
            2,
            2,
        ),
    ] {
        let mut original_identity = None;
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let descriptor = jit.descriptor();
            let plan = &descriptor.plan;
            let frontiers = plan.text_line_retention_frontiers();
            let joins = plan.selected_text_joins();
            let lines = plan.text_line_matrix_regions();
            assert_eq!(joins.len(), expected_joins, "{name}");
            assert_eq!(lines.len(), expected_lines, "{name}");
            assert_eq!(
                joins.iter().filter(|join| join.range().is_some()).count(),
                expected_ranges,
                "{name}"
            );
            let fresh = program_identity::typed_ir_sha256(plan, &frontiers, &joins, &lines);
            assert_eq!(*descriptor.typed_ir_sha256(), fresh, "{name}");
            assert_eq!(descriptor.selected_joins.regions.len(), joins.len());
            assert_eq!(descriptor.line_matrices.regions.len(), lines.len());
            if let Some(original) = original_identity {
                assert_eq!(original, fresh, "original/decoded {name}");
            } else {
                original_identity = Some(fresh);
            }
            // Omitting, duplicating or reordering proofs must affect identity
            // even when a prefix layout cannot use all of those proofs.
            if !joins.is_empty() {
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &[], &lines)
                );
                let mut duplicated = joins.clone();
                duplicated.extend(joins.iter().cloned());
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &duplicated, &lines)
                );
            }
            if !lines.is_empty() {
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &[])
                );
                let mut duplicated = lines.clone();
                duplicated.extend(lines.iter().cloned());
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &duplicated)
                );
            }
            if joins.len() > 1 {
                let mut reversed = joins.clone();
                reversed.reverse();
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &reversed, &lines)
                );
            }
            if lines.len() > 1 {
                let mut reversed = lines.clone();
                reversed.reverse();
                assert_ne!(
                    fresh,
                    program_identity::typed_ir_sha256(plan, &frontiers, &joins, &reversed)
                );
            }
        }
    }
}

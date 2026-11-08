//! Complete-use applicability for a private map/keys view. These facts grant no
//! runtime authority: a selected Engine loan must still pin/admit the snapshot.
use super::matrix_map_keys;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as Frame, NativeBufferFrames, NativeCollectionPrimitive as P,
    NativeIterationAction as A, NativeMatchValue, NativeProgramArtifactPlan as Plan,
    NativeProgramReadRole as Read,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct PrivateFrameUse {
    pub pack: usize,
    pub output: NativeMatchValue,
    pub field: usize,
    pub retires_snapshot_handle: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::native_artifact) struct Region {
    pub producer: usize,
    pub snapshot: NativeMatchValue,
    pub output: NativeMatchValue,
    pub lengths: Vec<usize>,
    pub cells: Vec<usize>,
    pub private_frames: Vec<PrivateFrameUse>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(in crate::native_artifact) struct Layout {
    pub regions: Vec<Region>,
}
impl Layout {
    /// The caller must first validate the plan. Every use is freshly derived;
    /// there are no serialized frame paths, pointers or snapshot identities.
    pub fn derive(
        plan: &Plan,
        frames: &NativeBufferFrames,
        cells: &matrix_map_keys::Layout,
    ) -> Result<Self, &'static str> {
        if cells.regions.is_empty() || plan.blocks().len() > 16_384 {
            return Ok(Self::default());
        }
        let facts = plan.facts();
        let mut layout = Self::default();
        for (producer, block) in plan.blocks().iter().enumerate() {
            let A::LibraryCall {
                template,
                operands,
                output,
            } = block.action()
            else {
                continue;
            };
            if template.primitive() != P::StringMapKeys || operands.len() != 1 {
                continue;
            }
            let snapshot = operands[0];
            let [definition] = facts.definition_sites(output.get())? else {
                continue;
            };
            if definition.site() != Some(producer as u32)
                || plan.activation_boundary() == Some(producer as u32)
            {
                continue;
            }
            let mut region = Region {
                producer,
                snapshot,
                output: *output,
                lengths: Vec::new(),
                cells: Vec::new(),
                private_frames: Vec::new(),
            };
            let mut supported = true;
            for role in facts.read_roles(*output)? {
                let Read::Action(site) = role else {
                    supported = false;
                    break;
                };
                let site = *site as usize;
                if plan.activation_boundary() == Some(site as u32) {
                    supported = false;
                    break;
                }
                if cells
                    .producer(site)
                    .is_some_and(|(_, cell)| cell.source == *output)
                {
                    region.cells.push(site);
                    continue;
                }
                if let A::LibraryCall {
                    template, operands, ..
                } = plan.blocks()[site].action()
                    && template.primitive() == P::SequenceLength
                    && operands.as_ref() == [*output]
                {
                    region.lengths.push(site);
                    continue;
                }
                let Some(action) = frames.action(site) else {
                    supported = false;
                    break;
                };
                let Frame::Pack { output: frame } = action.operation() else {
                    supported = false;
                    break;
                };
                let A::LibraryCall { operands, .. } = plan.blocks()[site].action() else {
                    supported = false;
                    break;
                };
                let fields = operands
                    .iter()
                    .enumerate()
                    .filter(|(_, value)| **value == *output)
                    .map(|(ordinal, _)| ordinal)
                    .collect::<Vec<_>>();
                let [field] = fields.as_slice() else {
                    supported = false;
                    break;
                };
                let [definition] = facts.definition_sites(frame.get())? else {
                    supported = false;
                    break;
                };
                if definition.site() != Some(site as u32) {
                    supported = false;
                    break;
                }
                // The only supported frame uses project OTHER fields. The
                // logical borrowed field still participates in pack shape and
                // footprint admission; it is never physically read or escaped.
                for role in facts.read_roles(*frame)? {
                    let Read::Action(reader) = role else {
                        supported = false;
                        break;
                    };
                    if !matches!(frames.action(*reader as usize).map(|a| a.operation()),
                        Some(Frame::Project { source, ordinal, .. }) if source == frame && ordinal != field)
                    {
                        supported = false;
                        break;
                    }
                }
                if !supported {
                    break;
                }
                region.private_frames.push(PrivateFrameUse {
                    pack: site,
                    output: *frame,
                    field: *field,
                    retires_snapshot_handle: action.retire().contains(&snapshot),
                });
            }
            if supported && !region.cells.is_empty() {
                layout.regions.push(region);
            }
        }
        Ok(layout)
    }
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod tests {
    use super::*;
    use crate::native_artifact::program::prepare::matrix_map_keys::tests::{artifact, source};
    fn derive(source: &str) -> Layout {
        let artifact = artifact(source);
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        plan.validate_current_visits().unwrap();
        let (frames, owners) = plan.facts().buffer_frame_owner_layouts().unwrap();
        let cells = matrix_map_keys::Layout::derive(&plan, Some(&owners)).unwrap();
        Layout::derive(&plan, &frames, &cells).unwrap()
    }
    #[test]
    fn borrowed_map_keys_prove_dead_private_frame_field_and_refuse_escapes() {
        let original = source("at + 1u64", "at", "at + 1u64", "histogram");
        let layout = derive(&original);
        assert_eq!(layout.regions.len(), 1, "{layout:?}");
        assert!(!layout.regions[0].private_frames.is_empty(), "{layout:?}");
        // A top-level public tuple can already lower without the separate
        // dynamic-to-exact Map-cardinality return conversion. The otherwise
        // identical nonescaping source must qualify, so rejection below
        // exercises complete uses, not an unrelated eligibility miss.
        let declarations = original.split("\nbuild(").next().unwrap();
        let top = format!(
            "{declarations}\ntags := tags<[string]>\nfirst := sequence/length(tags)\nhistogram := map/histogram(tags)\nkeys := map/keys(histogram)\nend := sequence/length(keys)\nindex := renumber(keys,first,end,histogram)\nindex\n"
        );
        let positive = derive(&top);
        assert_eq!(positive.regions.len(), 1, "{positive:?}");
        let escaping = top.strip_suffix("index\n").unwrap().to_owned() + "(index,keys)\n";
        assert!(derive(&escaping).regions.is_empty());
    }
    #[test]
    #[ignore = "requires retained benchmark source explicitly supplied by caller"]
    fn borrowed_map_keys_actual_source_pins_snapshot_across_original_handle_retirement() {
        let source =
            std::fs::read_to_string(std::env::var("MECH_MATRIX_KEY_BENCH_SOURCE").unwrap())
                .unwrap();
        let layout = derive(&source);
        let region = layout
            .regions
            .iter()
            .find(|r| r.cells.contains(&61))
            .unwrap();
        assert_eq!(
            (region.producer, region.snapshot.get(), region.output.get()),
            (38, 65, 66)
        );
        assert_eq!(region.lengths, [39]);
        assert!(
            region
                .private_frames
                .iter()
                .any(|f| f.pack == 40 && f.field == 0 && f.retires_snapshot_handle)
        );
        println!("{layout:?}");
    }
}
